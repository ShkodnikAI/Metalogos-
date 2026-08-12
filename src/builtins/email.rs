// ── Email builtins (Наряд MLG-4): SMTP + IMAP on pure Rust ────────────
// Грантовая стратегия: no Python, no external CLI, pure Rust only.
// Crate: lettre (SMTP), imap (IMAP), native-tls

use crate::interpreter::Value;

use super::expect_string_arg;

/// Build a Metalogos struct Value from type name, field keys, and values.
fn make_struct(type_name: &str, keys: &[&str], values: &[Value]) -> Value {
    let mut fields = std::collections::HashMap::new();
    for (k, v) in keys.iter().zip(values.iter()) {
        fields.insert(k.to_string(), v.clone());
    }
    Value::Struct {
        type_name: type_name.to_string(),
        fields,
    }
}

/// Build a Metalogos list Value from a slice of Values.
fn make_list(items: &[Value]) -> Value {
    Value::List(items.to_vec())
}

// ── Helper: read SMTP config from environment ───────────────────────────
// SMTP_HOST, SMTP_PORT (default 465 for TLS / 587 for STARTTLS),
// SMTP_USER, SMTP_PASS, SMTP_FROM (default = SMTP_USER)

fn smtp_config() -> Result<(String, u16, String, String, String), String> {
    let host =
        std::env::var("SMTP_HOST").map_err(|_| "smtp_send: SMTP_HOST env not set".to_string())?;
    let port: u16 = std::env::var("SMTP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(465);
    let user =
        std::env::var("SMTP_USER").map_err(|_| "smtp_send: SMTP_USER env not set".to_string())?;
    let pass =
        std::env::var("SMTP_PASS").map_err(|_| "smtp_send: SMTP_PASS env not set".to_string())?;
    let from = std::env::var("SMTP_FROM").unwrap_or_else(|_| user.clone());
    Ok((host, port, user, pass, from))
}

// ── Helper: read IMAP config from environment ───────────────────────────
// IMAP_HOST, IMAP_PORT (default 993), IMAP_USER, IMAP_PASS

fn imap_config() -> Result<(String, u16, String, String), String> {
    let host = std::env::var("IMAP_HOST").map_err(|_| "imap: IMAP_HOST env not set".to_string())?;
    let port: u16 = std::env::var("IMAP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(993);
    let user = std::env::var("IMAP_USER").map_err(|_| "imap: IMAP_USER env not set".to_string())?;
    let pass = std::env::var("IMAP_PASS").map_err(|_| "imap: IMAP_PASS env not set".to_string())?;
    Ok((host, port, user, pass))
}

// ═══════════════════════════════════════════════════════════════════════
// SMTP: smtp_send, smtp_send_html
// ═══════════════════════════════════════════════════════════════════════

/// `smtp_send(to, subject, body [, attachments_json, from_override, reply_to])`
///
/// Send a plain-text email via SMTP (TLS on port 465 or STARTTLS on 587).
///
/// Config via env vars: SMTP_HOST, SMTP_PORT, SMTP_USER, SMTP_PASS, SMTP_FROM.
///
/// # Arguments
/// - `to` (String): recipient email (comma-separated for multiple)
/// - `subject` (String): email subject
/// - `body` (String): plain-text body
/// - `attachments_json` (String, optional): JSON array of file paths to attach
/// - `from_override` (String, optional): override From address
/// - `reply_to` (String, optional): Reply-To address
///
/// # Returns
/// Struct { ok: true } on success, or error string.
pub fn builtin_smtp_send(args: &[Value]) -> Result<Value, String> {
    let to = expect_string_arg("smtp_send", args, 0)?;
    let subject = expect_string_arg("smtp_send", args, 1)?;
    let body = expect_string_arg("smtp_send", args, 2)?;
    let attachments_json = if args.len() > 3 {
        Some(expect_string_arg("smtp_send", args, 3)?)
    } else {
        None
    };
    let from_override = if args.len() > 4 {
        Some(expect_string_arg("smtp_send", args, 4)?)
    } else {
        None
    };
    let reply_to = if args.len() > 5 {
        Some(expect_string_arg("smtp_send", args, 5)?)
    } else {
        None
    };

    let (host, port, user, pass, default_from) = smtp_config()?;
    let from_addr = from_override.unwrap_or(default_from);

    smtp_send_impl(
        &host,
        port,
        &user,
        &pass,
        &from_addr,
        &to,
        &subject,
        &body,
        attachments_json.as_deref(),
        reply_to.as_deref(),
        false, // is_html
    )
}

/// `smtp_send_html(to, subject, html_body [, attachments_json])`
///
/// Send an HTML email via SMTP.
///
/// # Arguments
/// - `to` (String): recipient email
/// - `subject` (String): email subject
/// - `html_body` (String): HTML body
/// - `attachments_json` (String, optional): JSON array of file paths
///
/// # Returns
/// Struct { ok: true } on success.
pub fn builtin_smtp_send_html(args: &[Value]) -> Result<Value, String> {
    let to = expect_string_arg("smtp_send_html", args, 0)?;
    let subject = expect_string_arg("smtp_send_html", args, 1)?;
    let html_body = expect_string_arg("smtp_send_html", args, 2)?;
    let attachments_json = if args.len() > 3 {
        Some(expect_string_arg("smtp_send_html", args, 3)?)
    } else {
        None
    };

    let (host, port, user, pass, from) = smtp_config()?;

    smtp_send_impl(
        &host,
        port,
        &user,
        &pass,
        &from,
        &to,
        &subject,
        &html_body,
        attachments_json.as_deref(),
        None, // reply_to
        true, // is_html
    )
}

/// Core SMTP send implementation using lettre.
#[allow(clippy::too_many_arguments)]
fn smtp_send_impl(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    attachments_json: Option<&str>,
    reply_to: Option<&str>,
    is_html: bool,
) -> Result<Value, String> {
    use lettre::message::header::ContentType;
    use lettre::message::Message;
    use lettre::message::{Mailbox, MultiPart, SinglePart};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::SmtpTransport;
    use lettre::Transport;

    // Build the email message
    let from_mailbox: Mailbox = from
        .parse()
        .map_err(|e| format!("smtp_send: invalid From '{}': {:?}", from, e))?;

    let mut builder = Message::builder()
        .from(from_mailbox)
        .subject(subject.to_string());

    // Add Reply-To if provided
    if let Some(rt) = reply_to {
        let rt_mailbox: Mailbox = rt
            .parse()
            .map_err(|e| format!("smtp_send: invalid Reply-To '{}': {:?}", rt, e))?;
        builder = builder.reply_to(rt_mailbox);
    }

    // Add recipients (comma-separated)
    for addr in to.split(',') {
        let addr = addr.trim();
        if addr.is_empty() {
            continue;
        }
        let mailbox: Mailbox = addr
            .parse()
            .map_err(|e| format!("smtp_send: invalid To '{}': {:?}", addr, e))?;
        builder = builder.to(mailbox);
    }

    // Build body part
    let body_part = if is_html {
        SinglePart::builder()
            .header(ContentType::TEXT_HTML)
            .body(body.to_string())
    } else {
        SinglePart::builder()
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
    };

    // Handle attachments
    let message = if let Some(att_json) = attachments_json {
        let paths: Vec<String> = serde_json::from_str(att_json)
            .map_err(|e| format!("smtp_send: invalid attachments_json: {}", e))?;

        let mut mixed = MultiPart::mixed().singlepart(body_part);

        for path in &paths {
            let data = std::fs::read(path)
                .map_err(|e| format!("smtp_send: cannot read attachment '{}': {}", path, e))?;
            let filename = std::path::Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "attachment".to_string());
            let ct = guess_content_type(&filename);
            let attachment = lettre::message::Attachment::new(filename).body(data, ct);
            mixed = mixed.singlepart(attachment);
        }

        builder
            .multipart(mixed)
            .map_err(|e| format!("smtp_send: failed to build message: {:?}", e))?
    } else {
        builder
            .singlepart(body_part)
            .map_err(|e| format!("smtp_send: failed to build message: {:?}", e))?
    };

    // Create SMTP transport using relay (auto-detects TLS)
    let creds = Credentials::new(user.to_string(), pass.to_string());

    let mailer = SmtpTransport::relay(host)
        .map_err(|e| format!("smtp_send: cannot configure relay for '{}': {:?}", host, e))?
        .port(port)
        .credentials(creds)
        .build();

    // Send
    let result = mailer.send(&message);
    match result {
        Ok(_) => Ok(make_struct("SmtpResult", &["ok"], &[Value::Bool(true)])),
        Err(e) => Err(format!("smtp_send: send failed: {:?}", e)),
    }
}

/// Guess MIME content type from filename extension.
fn guess_content_type(filename: &str) -> lettre::message::header::ContentType {
    use lettre::message::header::ContentType;

    let ext = std::path::Path::new(filename)
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "pdf" => ContentType::parse("application/pdf").unwrap_or(ContentType::TEXT_PLAIN),
        "png" => ContentType::parse("image/png").unwrap_or(ContentType::TEXT_PLAIN),
        "jpg" | "jpeg" => ContentType::parse("image/jpeg").unwrap_or(ContentType::TEXT_PLAIN),
        "gif" => ContentType::parse("image/gif").unwrap_or(ContentType::TEXT_PLAIN),
        "docx" => ContentType::parse(
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        )
        .unwrap_or(ContentType::TEXT_PLAIN),
        "xlsx" => {
            ContentType::parse("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
                .unwrap_or(ContentType::TEXT_PLAIN)
        }
        "zip" => ContentType::parse("application/zip").unwrap_or(ContentType::TEXT_PLAIN),
        "html" | "htm" => ContentType::TEXT_HTML,
        "txt" | "csv" | "log" => ContentType::TEXT_PLAIN,
        _ => ContentType::parse("application/octet-stream").unwrap_or(ContentType::TEXT_PLAIN),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// IMAP: imap_list, imap_read, imap_search, imap_mark_read, imap_move
// ═══════════════════════════════════════════════════════════════════════

/// `imap_list(folder, limit [, since_date])` — list recent emails in a folder.
///
/// # Arguments
/// - `folder` (String): IMAP folder name (e.g. "INBOX", "Sent")
/// - `limit` (Float or String): maximum number of emails to list
/// - `since_date` (String, optional): ISO date "YYYY-MM-DD" to filter from
///
/// # Returns
/// List of structs: [{ uid, from, subject, date, seen, size }]
pub fn builtin_imap_list(args: &[Value]) -> Result<Value, String> {
    let folder = expect_string_arg("imap_list", args, 0)?;
    let limit = match &args[1] {
        Value::Float(f) => *f as usize,
        Value::String(s) => s
            .parse::<usize>()
            .map_err(|e| format!("imap_list: invalid limit '{}': {}", s, e))?,
        other => {
            return Err(format!(
                "imap_list: limit must be number or string, got {:?}",
                other
            ))
        }
    };
    let since_date = if args.len() > 2 {
        Some(expect_string_arg("imap_list", args, 2)?)
    } else {
        None
    };

    let (host, port, user, pass) = imap_config()?;
    imap_list_impl(
        &host,
        port,
        &user,
        &pass,
        &folder,
        limit,
        since_date.as_deref(),
    )
}

/// `imap_read(uid)` — read a full email by UID.
///
/// # Arguments
/// - `uid` (Float or String): email UID
///
/// # Returns
/// Struct { uid, from, to, subject, date, body_text, body_html, attachments, seen }
pub fn builtin_imap_read(args: &[Value]) -> Result<Value, String> {
    let uid = match &args[0] {
        Value::Float(f) => *f as u32,
        Value::String(s) => s
            .parse::<u32>()
            .map_err(|e| format!("imap_read: invalid uid '{}': {}", s, e))?,
        other => {
            return Err(format!(
                "imap_read: uid must be number or string, got {:?}",
                other
            ))
        }
    };

    let (host, port, user, pass) = imap_config()?;
    imap_read_impl(&host, port, &user, &pass, uid)
}

/// `imap_search(query, folder)` — search emails by subject/from/body text.
///
/// # Arguments
/// - `query` (String): search term
/// - `folder` (String): IMAP folder name
///
/// # Returns
/// List of matching structs: [{ uid, from, subject, date }]
pub fn builtin_imap_search(args: &[Value]) -> Result<Value, String> {
    let query = expect_string_arg("imap_search", args, 0)?;
    let folder = expect_string_arg("imap_search", args, 1)?;

    let (host, port, user, pass) = imap_config()?;
    imap_search_impl(&host, port, &user, &pass, &query, &folder)
}

/// `imap_mark_read(uid)` — mark an email as read (Seen flag).
///
/// # Arguments
/// - `uid` (Float or String): email UID
///
/// # Returns
/// Struct { ok: true }
pub fn builtin_imap_mark_read(args: &[Value]) -> Result<Value, String> {
    let uid = match &args[0] {
        Value::Float(f) => *f as u32,
        Value::String(s) => s
            .parse::<u32>()
            .map_err(|e| format!("imap_mark_read: invalid uid '{}': {}", s, e))?,
        other => {
            return Err(format!(
                "imap_mark_read: uid must be number or string, got {:?}",
                other
            ))
        }
    };

    let (host, port, user, pass) = imap_config()?;
    imap_flag_impl(&host, port, &user, &pass, uid, "+FLAGS", "(\\Seen)")
}

/// `imap_move(uid, dest_folder)` — move email to another folder.
///
/// # Arguments
/// - `uid` (Float or String): email UID
/// - `dest_folder` (String): destination folder name
///
/// # Returns
/// Struct { ok: true }
pub fn builtin_imap_move(args: &[Value]) -> Result<Value, String> {
    let uid = match &args[0] {
        Value::Float(f) => *f as u32,
        Value::String(s) => s
            .parse::<u32>()
            .map_err(|e| format!("imap_move: invalid uid '{}': {}", s, e))?,
        other => {
            return Err(format!(
                "imap_move: uid must be number or string, got {:?}",
                other
            ))
        }
    };
    let dest_folder = expect_string_arg("imap_move", args, 1)?;

    let (host, port, user, pass) = imap_config()?;
    imap_move_impl(&host, port, &user, &pass, uid, &dest_folder)
}

// ═══════════════════════════════════════════════════════════════════════
// IMAP implementation (using imap crate v3 alpha with native-tls)
// ═══════════════════════════════════════════════════════════════════════

fn imap_connect(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
) -> Result<imap::Session<Box<dyn imap::ImapConnection>>, String> {
    let client = imap::ClientBuilder::new(host, port)
        .mode(imap::ConnectionMode::AutoTls)
        .tls_kind(imap::TlsKind::Native)
        .connect()
        .map_err(|e| format!("imap: connect to {}:{} failed: {}", host, port, e))?;

    let session = client
        .login(user, pass)
        .map_err(|(e, _)| format!("imap: login failed: {:?}", e))?;

    Ok(session)
}

/// Extract address from imap Address struct as readable string.
fn format_address(addr: &imap_proto::types::Address<'_>) -> String {
    let name = addr
        .name
        .as_ref()
        .map(|v| String::from_utf8_lossy(v).to_string())
        .unwrap_or_default();
    let mailbox = addr
        .mailbox
        .as_ref()
        .map(|v| String::from_utf8_lossy(v).to_string())
        .unwrap_or_default();
    let host_part = addr
        .host
        .as_ref()
        .map(|v| String::from_utf8_lossy(v).to_string())
        .unwrap_or_default();

    if name.is_empty() {
        format!("{}@{}", mailbox, host_part)
    } else {
        format!("{} <{}@{}>", name, mailbox, host_part)
    }
}

/// Extract a header value from raw email headers.
fn extract_header(headers: &str, name: &str) -> String {
    let prefix = format!("{}:", name);
    for line in headers.lines() {
        if line.to_lowercase().starts_with(&prefix.to_lowercase()) {
            return line[prefix.len()..].trim().to_string();
        }
    }
    String::new()
}

/// Check if Seen flag is present.
fn is_seen(flags: &[imap::types::Flag<'_>]) -> bool {
    flags.iter().any(|f| matches!(f, imap::types::Flag::Seen))
}

fn imap_list_impl(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    folder: &str,
    limit: usize,
    since_date: Option<&str>,
) -> Result<Value, String> {
    let mut session = imap_connect(host, port, user, pass)?;

    session
        .select(folder)
        .map_err(|e| format!("imap_list: cannot select folder '{}': {:?}", folder, e))?;

    // Build search criteria
    let criteria = if let Some(date) = since_date {
        format!("SINCE {}", date)
    } else {
        "ALL".to_string()
    };

    let uids = session
        .uid_search(&criteria)
        .map_err(|e| format!("imap_list: search failed: {:?}", e))?;

    // Sort UIDs descending (most recent first) and take limit
    let mut uid_vec: Vec<u32> = uids.iter().copied().collect();
    uid_vec.sort_by(|a, b| b.cmp(a));
    if uid_vec.len() > limit {
        uid_vec.truncate(limit);
    }

    if uid_vec.is_empty() {
        session
            .logout()
            .map_err(|e| format!("imap_list: logout failed: {:?}", e))?;
        return Ok(make_list(&[]));
    }

    // Fetch envelopes for these UIDs
    let uid_set: Vec<String> = uid_vec.iter().map(|u| u.to_string()).collect();
    let uid_str = uid_set.join(",");

    let messages = session
        .uid_fetch(&uid_str, "(ENVELOPE FLAGS RFC822.SIZE)")
        .map_err(|e| format!("imap_list: fetch failed: {:?}", e))?;

    let mut result: Vec<Value> = Vec::new();

    for msg in messages.iter() {
        let uid = msg.uid.unwrap_or(0);
        let seen = is_seen(msg.flags());

        let (from, subject, date) = if let Some(envelope) = msg.envelope() {
            let from_str = envelope
                .from
                .as_ref()
                .and_then(|v| v.first())
                .map(format_address)
                .unwrap_or_default();

            let subject_str = envelope
                .subject
                .as_ref()
                .map(|v| String::from_utf8_lossy(v).to_string())
                .unwrap_or_else(|| "(no subject)".to_string());
            let date_str = envelope
                .date
                .as_ref()
                .map(|v| String::from_utf8_lossy(v).to_string())
                .unwrap_or_default();

            (from_str, subject_str, date_str)
        } else {
            (String::new(), String::new(), String::new())
        };

        let size = msg.size.unwrap_or(0) as f64;

        result.push(make_struct(
            "ImapMessage",
            &["uid", "from", "subject", "date", "seen", "size"],
            &[
                Value::Float(uid as f64),
                Value::String(from),
                Value::String(subject),
                Value::String(date),
                Value::Bool(seen),
                Value::Float(size),
            ],
        ));
    }

    session
        .logout()
        .map_err(|e| format!("imap_list: logout failed: {:?}", e))?;

    Ok(make_list(&result))
}

fn imap_read_impl(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    uid: u32,
) -> Result<Value, String> {
    let mut session = imap_connect(host, port, user, pass)?;

    session
        .select("INBOX")
        .map_err(|e| format!("imap_read: cannot select INBOX: {:?}", e))?;

    let uid_str = uid.to_string();
    let messages = session
        .uid_fetch(&uid_str, "(RFC822 FLAGS)")
        .map_err(|e| format!("imap_read: fetch UID {} failed: {:?}", uid, e))?;

    let msg = messages
        .iter()
        .next()
        .ok_or_else(|| format!("imap_read: UID {} not found", uid))?;

    let seen = is_seen(msg.flags());

    let body_raw = msg
        .body()
        .ok_or_else(|| format!("imap_read: no body for UID {}", uid))?;

    let body_str = String::from_utf8_lossy(body_raw).to_string();

    // Split headers from body
    let (headers, body_text) = if let Some(pos) = body_str.find("\r\n\r\n") {
        (&body_str[..pos], &body_str[pos + 4..])
    } else if let Some(pos) = body_str.find("\n\n") {
        (&body_str[..pos], &body_str[pos + 2..])
    } else {
        ("", body_str.as_str())
    };

    let from = extract_header(headers, "From");
    let to = extract_header(headers, "To");
    let subject = extract_header(headers, "Subject");
    let date = extract_header(headers, "Date");

    // Count attachments (Content-Disposition: attachment)
    let attachment_count = headers.matches("Content-Disposition: attachment").count();

    // Basic MIME: detect HTML part
    let is_html_body = body_text.contains("<html") || body_text.contains("<body");
    let body_html = if is_html_body {
        body_text.to_string()
    } else {
        String::new()
    };
    let body_text_clean = if is_html_body {
        // Strip basic HTML tags for text version
        body_text
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<p>", "\n")
            .replace("</p>", "\n")
            .replace("<li>", "- ")
            .replace("</li>", "\n")
            .chars()
            .filter(|c| *c != '<' && *c != '>')
            .collect::<String>()
    } else {
        body_text.to_string()
    };

    session
        .logout()
        .map_err(|e| format!("imap_read: logout failed: {:?}", e))?;

    Ok(make_struct(
        "ImapEmail",
        &[
            "uid",
            "from",
            "to",
            "subject",
            "date",
            "body_text",
            "body_html",
            "attachments",
            "seen",
        ],
        &[
            Value::Float(uid as f64),
            Value::String(from),
            Value::String(to),
            Value::String(subject),
            Value::String(date),
            Value::String(body_text_clean),
            Value::String(body_html),
            Value::Float(attachment_count as f64),
            Value::Bool(seen),
        ],
    ))
}

fn imap_search_impl(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    query: &str,
    folder: &str,
) -> Result<Value, String> {
    let mut session = imap_connect(host, port, user, pass)?;

    session
        .select(folder)
        .map_err(|e| format!("imap_search: cannot select folder '{}': {:?}", folder, e))?;

    // IMAP SEARCH TEXT matches against subject, from, and body
    let criteria = format!("TEXT \"{}\"", query.replace('"', "\\\""));

    let uids = session
        .uid_search(&criteria)
        .map_err(|e| format!("imap_search: search failed: {:?}", e))?;

    if uids.is_empty() {
        session
            .logout()
            .map_err(|e| format!("imap_search: logout failed: {:?}", e))?;
        return Ok(make_list(&[]));
    }

    // Sort UIDs descending and take up to 50
    let mut uid_vec: Vec<u32> = uids.iter().copied().collect();
    uid_vec.sort_by(|a, b| b.cmp(a));
    uid_vec.truncate(50);

    let uid_set: Vec<String> = uid_vec.iter().map(|u| u.to_string()).collect();
    let uid_str = uid_set.join(",");

    let messages = session
        .uid_fetch(&uid_str, "(ENVELOPE FLAGS)")
        .map_err(|e| format!("imap_search: fetch failed: {:?}", e))?;

    let mut result: Vec<Value> = Vec::new();

    for msg in messages.iter() {
        let uid_val = msg.uid.unwrap_or(0);

        let (from, subject, date) = if let Some(envelope) = msg.envelope() {
            let from_str = envelope
                .from
                .as_ref()
                .and_then(|v| v.first())
                .map(format_address)
                .unwrap_or_default();

            let subject_str = envelope
                .subject
                .as_ref()
                .map(|v| String::from_utf8_lossy(v).to_string())
                .unwrap_or_else(|| "(no subject)".to_string());
            let date_str = envelope
                .date
                .as_ref()
                .map(|v| String::from_utf8_lossy(v).to_string())
                .unwrap_or_default();

            (from_str, subject_str, date_str)
        } else {
            (String::new(), String::new(), String::new())
        };

        result.push(make_struct(
            "ImapMessage",
            &["uid", "from", "subject", "date"],
            &[
                Value::Float(uid_val as f64),
                Value::String(from),
                Value::String(subject),
                Value::String(date),
            ],
        ));
    }

    session
        .logout()
        .map_err(|e| format!("imap_search: logout failed: {:?}", e))?;

    Ok(make_list(&result))
}

fn imap_flag_impl(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    uid: u32,
    action: &str, // "+FLAGS" or "-FLAGS"
    flags: &str,  // e.g. "(\\Seen)"
) -> Result<Value, String> {
    let mut session = imap_connect(host, port, user, pass)?;

    session
        .select("INBOX")
        .map_err(|e| format!("imap_flag: cannot select INBOX: {:?}", e))?;

    let uid_str = uid.to_string();
    let store_cmd = format!("{} {}", action, flags);

    // Use UID STORE to add/remove flags
    let _ = session
        .uid_store(&uid_str, &store_cmd)
        .map_err(|e| format!("imap_flag: store failed for UID {}: {:?}", uid, e))?;

    session
        .logout()
        .map_err(|e| format!("imap_flag: logout failed: {:?}", e))?;

    Ok(make_struct("ImapResult", &["ok"], &[Value::Bool(true)]))
}

fn imap_move_impl(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    uid: u32,
    dest_folder: &str,
) -> Result<Value, String> {
    let mut session = imap_connect(host, port, user, pass)?;

    session
        .select("INBOX")
        .map_err(|e| format!("imap_move: cannot select INBOX: {:?}", e))?;

    let uid_str = uid.to_string();

    // Try MOVE extension first (RFC 6851), fallback to COPY+DELETE
    let move_result = session.uid_mv(&uid_str, dest_folder);
    if move_result.is_err() {
        // Fallback: COPY + STORE \Deleted + EXPUNGE
        session
            .uid_copy(&uid_str, dest_folder)
            .map_err(|e| format!("imap_move: copy to '{}' failed: {:?}", dest_folder, e))?;

        let flag_str = "+FLAGS (\\Deleted)";
        session
            .uid_store(&uid_str, flag_str)
            .map_err(|e| format!("imap_move: mark deleted failed: {:?}", e))?;

        session
            .expunge()
            .map_err(|e| format!("imap_move: expunge failed: {:?}", e))?;
    }

    session
        .logout()
        .map_err(|e| format!("imap_move: logout failed: {:?}", e))?;

    Ok(make_struct("ImapResult", &["ok"], &[Value::Bool(true)]))
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_send_missing_env() {
        // Without SMTP_HOST set, should return error
        std::env::remove_var("SMTP_HOST");
        let result = builtin_smtp_send(&[
            Value::String("test@example.com".to_string()),
            Value::String("Test".to_string()),
            Value::String("Body".to_string()),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("SMTP_HOST"));
    }

    #[test]
    fn test_imap_list_missing_env() {
        std::env::remove_var("IMAP_HOST");
        let result = builtin_imap_list(&[Value::String("INBOX".to_string()), Value::Float(10.0)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("IMAP_HOST"));
    }

    #[test]
    fn test_imap_read_missing_env() {
        std::env::remove_var("IMAP_HOST");
        let result = builtin_imap_read(&[Value::Float(1.0)]);
        assert!(result.is_err());
    }

    #[test]
    fn test_guess_content_type() {
        let pdf_ct = guess_content_type("report.pdf");
        assert!(format!("{:?}", pdf_ct).contains("pdf"));

        let png_ct = guess_content_type("photo.png");
        assert!(format!("{:?}", png_ct).contains("png"));

        let xlsx_ct = guess_content_type("data.xlsx");
        assert!(format!("{:?}", xlsx_ct).contains("spreadsheetml"));
    }

    #[test]
    fn test_extract_header() {
        let headers = "From: alice@example.com\r\nSubject: Test\r\nDate: 2026-08-13\r\n";
        assert_eq!(extract_header(headers, "From"), "alice@example.com");
        assert_eq!(extract_header(headers, "Subject"), "Test");
        assert_eq!(extract_header(headers, "Date"), "2026-08-13");
        assert_eq!(extract_header(headers, "To"), "");
    }

    #[test]
    fn test_is_seen() {
        assert!(is_seen(&[
            imap::types::Flag::Seen,
            imap::types::Flag::Recent
        ]));
        assert!(!is_seen(&[imap::types::Flag::Recent]));
        assert!(!is_seen(&[]));
    }
}
