// ── Наряд MLG-5: Calendar (CalDAV + iCal) ──────────────────────────────
// Pure Rust: CalDAV client over reqwest + iCal parsing/generation.
// CalDAV protocol: RFC 4791, iCalendar: RFC 5545.

use crate::interpreter::Value;

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

// ── CalDAV session store ─────────────────────────────────────────────

/// A CalDAV session: credentials + HTTP client for a server.
struct CalSession {
    url: String,
    user: String,
    pass: String,
    /// Cached calendar-home-set URL (discovered via PROPFIND).
    home_set: Option<String>,
}

static CAL_SESSIONS: Lazy<Mutex<HashMap<String, CalSession>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

// ── Helpers ──────────────────────────────────────────────────────────

/// Extract a string Value or return an error.
fn str_arg(args: &[Value], idx: usize, name: &str) -> Result<String, String> {
    match args.get(idx) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{}: arg {} must be a string", name, idx + 1)),
    }
}

/// Extract an optional string Value (Unit or missing → None).
fn opt_str_arg(args: &[Value], idx: usize) -> Option<String> {
    match args.get(idx) {
        Some(Value::String(s)) => Some(s.clone()),
        Some(Value::Unit) | None => None,
        _ => None,
    }
}

/// Generate a unique session/calendar/event ID.
fn gen_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("cal_{:x}", ts)
}

/// Generate a UUID4-like string for event UIDs.
fn gen_uid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Simple unique ID — not cryptographically random, but sufficient for event UIDs.
    format!(
        "{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}",
        (ts >> 32) as u32,
        (ts >> 16) as u16 & 0xFFF,
        ts as u32 & 0xFFF,
        (ts >> 4) as u32 & 0xFFF,
        ts as u64 & 0xFFFFFFFFFFFF
    )
}

/// Build a reqwest blocking client.
/// (Basic auth is set per-request, not on the client itself.)
fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true) // Self-hosted CalDAV servers often use self-signed
        .build()
        .map_err(|e| format!("cal_connect: failed to build HTTP client: {}", e))
}

/// Execute a PROPFIND request to discover calendar-home-set.
fn propfind_home_set(
    client: &reqwest::blocking::Client,
    url: &str,
    user: &str,
    pass: &str,
) -> Result<String, String> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop>
    <calendar-home-set xmlns="urn:ietf:params:xml:ns:caldav"/>
  </prop>
</propfind>"#;

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap_or(reqwest::Method::GET),
            url,
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml; charset=utf-8")
        .basic_auth(user, Some(pass))
        .body(body)
        .send()
        .map_err(|e| format!("cal_connect PROPFIND: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("cal_connect PROPFIND: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("cal_connect PROPFIND read body: {}", e))?;

    // Extract calendar-home-set href from XML response.
    // Look for <href> inside <calendar-home-set> or fallback to URL + "/calendars/"
    if let Some(pos) = text.find("calendar-home-set") {
        let rest = &text[pos..];
        if let Some(href_start) = rest.find("<href>") {
            let href_end = rest.find("</href>").unwrap_or(rest.len());
            let href = &rest[href_start + 6..href_end];
            return Ok(href.trim().to_string());
        }
    }

    // Fallback: assume /calendars/ path
    Ok(format!("{}/calendars/", url.trim_end_matches('/')))
}

// ── CalDAV builtins ──────────────────────────────────────────────────

/// `cal_connect(url, user, pass)` → session_id
///
/// Connects to a CalDAV server, discovers calendar-home-set via PROPFIND,
/// and stores a session. Returns the session ID for use in subsequent calls.
pub fn builtin_cal_connect(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("cal_connect: expected 3 args (url, user, pass)".to_string());
    }
    let url = str_arg(args, 0, "cal_connect")?;
    let user = str_arg(args, 1, "cal_connect")?;
    let pass = str_arg(args, 2, "cal_connect")?;

    let client = build_client()?;

    // Try PROPFIND to discover calendar-home-set
    let home_set = propfind_home_set(&client, &url, &user, &pass).ok();

    let session_id = gen_id();
    let mut sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_connect: lock: {}", e))?;
    sessions.insert(
        session_id.clone(),
        CalSession {
            url,
            user,
            pass,
            home_set,
        },
    );

    Ok(Value::String(session_id))
}

/// `cal_list(session_id)` → JSON array of calendars
///
/// Lists available calendars from the CalDAV server.
/// Returns a JSON string like: [{"href":"...", "displayname":"Work"}, ...]
pub fn builtin_cal_list(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cal_list: expected 1 arg (session_id)".to_string());
    }
    let session_id = str_arg(args, 0, "cal_list")?;

    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_list: lock: {}", e))?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| format!("cal_list: session '{}' not found", session_id))?;

    let home_url = session.home_set.as_deref().unwrap_or(session.url.as_str());

    let client = build_client()?;

    // PROPFIND Depth:1 on the home set to list calendars
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop>
    <displayname/>
    <calendar-color xmlns="http://apple.com/ns/ical/"/>
    <resourcetype/>
  </prop>
</propfind>"#;

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap_or(reqwest::Method::GET),
            home_url,
        )
        .header("Depth", "1")
        .header("Content-Type", "application/xml; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(body)
        .send()
        .map_err(|e| format!("cal_list: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("cal_list: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("cal_list: read body: {}", e))?;

    // Parse response to extract calendars with their hrefs and displaynames.
    // Simple XML parsing: find <response> blocks.
    let mut calendars = Vec::new();
    let mut pos = 0;
    while let Some(start) = text[pos..].find("<response>") {
        let block_start = pos + start;
        let block_end = text[block_start..]
            .find("</response>")
            .map(|e| block_start + e + 11)
            .unwrap_or(text.len());
        let block = &text[block_start..block_end];

        // Check if this is a calendar (has <calendar/> in resourcetype)
        if block.contains("calendar") {
            let mut href = String::new();
            let mut displayname = String::new();

            if let Some(hs) = block.find("<href>") {
                let he = block[hs + 6..]
                    .find("</href>")
                    .map(|e| hs + 6 + e)
                    .unwrap_or(block.len());
                href = block[hs + 6..he].trim().to_string();
            }
            if let Some(ds) = block.find("<displayname>") {
                let de = block[ds + 13..]
                    .find("</displayname>")
                    .map(|e| ds + 13 + e)
                    .unwrap_or(block.len());
                displayname = block[ds + 13..de].trim().to_string();
            }

            calendars.push(serde_json::json!({
                "href": href,
                "displayname": displayname,
            }));
        }

        pos = block_end;
    }

    Ok(Value::String(
        serde_json::to_string(&calendars).unwrap_or_else(|_| "[]".to_string()),
    ))
}

/// `cal_events(calendar_id, start, end)` → JSON array of events
///
/// Fetches events in the given date range from a CalDAV calendar.
/// Dates in YYYY-MM-DD format. Returns JSON string of event structs.
pub fn builtin_cal_events(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("cal_events: expected 3 args (calendar_id, start, end)".to_string());
    }
    let calendar_id = str_arg(args, 0, "cal_events")?;
    let start = str_arg(args, 1, "cal_events")?;
    let end = str_arg(args, 2, "cal_events")?;

    // calendar_id is actually the calendar href URL from cal_list
    // We need to find credentials from any session
    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_events: lock: {}", e))?;
    let session = sessions.values().next().ok_or_else(|| {
        "cal_events: no CalDAV session found — call cal_connect first".to_string()
    })?;

    let client = build_client()?;

    // CalDAV calendar-query REPORT (RFC 4791 §7.8)
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
    <c:calendar-data/>
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        <c:time-range start="{}T000000Z" end="{}T235959Z"/>
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#,
        start.replace('-', ""),
        end.replace('-', "")
    );

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"REPORT").unwrap_or(reqwest::Method::POST),
            &calendar_id,
        )
        .header("Depth", "1")
        .header("Content-Type", "application/xml; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(body)
        .send()
        .map_err(|e| format!("cal_events: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("cal_events: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("cal_events: read body: {}", e))?;

    // Extract calendar-data from each response
    let mut events = Vec::new();
    let mut pos = 0;
    while let Some(start_pos) = text[pos..].find("<calendar-data") {
        let tag_start = pos + start_pos;
        // Find the closing > of the opening tag
        let content_start = text[tag_start..]
            .find('>')
            .map(|p| tag_start + p + 1)
            .unwrap_or(text.len());
        let content_end = text[content_start..]
            .find("</calendar-data>")
            .map(|p| content_start + p)
            .unwrap_or(text.len());
        let ical_data = text[content_start..content_end].trim();

        // Parse the iCal data into a simple struct
        if let Ok(parsed) = parse_ical_to_value(ical_data) {
            events.push(parsed);
        }

        pos = content_end + 16; // skip past </calendar-data>
    }

    Ok(Value::String(
        serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string()),
    ))
}

/// `cal_read(event_uid)` → JSON struct of a single event
///
/// Reads a single event by its UID. The event_uid should be the full URL
/// of the .ics resource (as returned by cal_events).
pub fn builtin_cal_read(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cal_read: expected 1 arg (event_uid)".to_string());
    }
    let event_uid = str_arg(args, 0, "cal_read")?;

    // event_uid is actually the href/URL of the .ics resource
    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_read: lock: {}", e))?;
    let session = sessions
        .values()
        .next()
        .ok_or_else(|| "cal_read: no CalDAV session found".to_string())?;

    let client = build_client()?;

    let resp = client
        .get(&event_uid)
        .basic_auth(&session.user, Some(&session.pass))
        .send()
        .map_err(|e| format!("cal_read: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("cal_read: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("cal_read: read body: {}", e))?;

    parse_ical_to_value(&text).map(Value::String)
}

/// `cal_create(cal_id, summary, start, end [,desc, location, attendees_json])` → event UID
///
/// Creates a new calendar event. Dates in YYYY-MM-DDTHH:MM:SS format.
/// Returns the UID of the created event.
#[allow(clippy::too_many_arguments)]
pub fn builtin_cal_create(args: &[Value]) -> Result<Value, String> {
    if args.len() < 4 || args.len() > 7 {
        return Err("cal_create: expected 4..7 args (cal_id, summary, start, end [,desc, location, attendees_json])".to_string());
    }
    let cal_id = str_arg(args, 0, "cal_create")?;
    let summary = str_arg(args, 1, "cal_create")?;
    let start = str_arg(args, 2, "cal_create")?;
    let end = str_arg(args, 3, "cal_create")?;
    let desc = opt_str_arg(args, 4);
    let location = opt_str_arg(args, 5);
    let attendees_json = opt_str_arg(args, 6);

    let uid = gen_uid();

    // Build iCal VEVENT
    let dtstart = format_datetime(&start);
    let dtend = format_datetime(&end);

    let mut ical = String::new();
    ical.push_str("BEGIN:VCALENDAR\r\n");
    ical.push_str("VERSION:2.0\r\n");
    ical.push_str("PRODID:-//Metalogos//MLG-5//RU\r\n");
    ical.push_str("BEGIN:VEVENT\r\n");
    ical.push_str(&format!("UID:{}\r\n", uid));
    ical.push_str(&format!("DTSTART:{}\r\n", dtstart));
    ical.push_str(&format!("DTEND:{}\r\n", dtend));
    ical.push_str(&format!("SUMMARY:{}\r\n", escape_ical_text(&summary)));

    if let Some(d) = &desc {
        ical.push_str(&format!("DESCRIPTION:{}\r\n", escape_ical_text(d)));
    }
    if let Some(loc) = &location {
        ical.push_str(&format!("LOCATION:{}\r\n", escape_ical_text(loc)));
    }
    if let Some(att) = &attendees_json {
        // attendees_json is a JSON array of email addresses
        if let Ok(emails) = serde_json::from_str::<Vec<String>>(att) {
            for email in &emails {
                ical.push_str(&format!("ATTENDEE;CN={}:mailto:{}\r\n", email, email));
            }
        }
    }

    ical.push_str("END:VEVENT\r\n");
    ical.push_str("END:VCALENDAR\r\n");

    // PUT to CalDAV server
    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_create: lock: {}", e))?;
    let session = sessions
        .values()
        .next()
        .ok_or_else(|| "cal_create: no CalDAV session found".to_string())?;

    let client = build_client()?;

    // PUT the .ics resource
    let event_url = format!("{}/{}.ics", cal_id.trim_end_matches('/'), uid);

    let resp = client
        .put(&event_url)
        .header("Content-Type", "text/calendar; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(ical)
        .send()
        .map_err(|e| format!("cal_create: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("cal_create: server returned {}", status));
    }

    Ok(Value::String(uid))
}

/// `cal_update(event_uid, fields_json)` → "ok"
///
/// Updates fields of an existing event. fields_json is a JSON object
/// with fields to update: {"summary": "...", "start": "...", "end": "...", ...}
pub fn builtin_cal_update(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("cal_update: expected 2 args (event_uid, fields_json)".to_string());
    }
    let event_uid = str_arg(args, 0, "cal_update")?;
    let fields_json = str_arg(args, 1, "cal_update")?;

    let fields: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&fields_json)
        .map_err(|e| format!("cal_update: invalid fields_json: {}", e))?;

    // First, GET the existing event
    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_update: lock: {}", e))?;
    let session = sessions
        .values()
        .next()
        .ok_or_else(|| "cal_update: no CalDAV session found".to_string())?;

    let client = build_client()?;

    let resp = client
        .get(&event_uid)
        .basic_auth(&session.user, Some(&session.pass))
        .send()
        .map_err(|e| format!("cal_update GET: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("cal_update GET: server returned {}", status));
    }

    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let text = resp
        .text()
        .map_err(|e| format!("cal_update: read body: {}", e))?;

    // Apply updates to the iCal text
    let mut ical_text = text;

    for (key, value) in &fields {
        let val_str = match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => continue,
            other => other.to_string(),
        };

        let ical_key = match key.as_str() {
            "summary" => "SUMMARY".to_string(),
            "description" => "DESCRIPTION".to_string(),
            "location" => "LOCATION".to_string(),
            "start" => "DTSTART".to_string(),
            "end" => "DTEND".to_string(),
            other => other.to_uppercase(),
        };

        let ical_val = if key == "start" || key == "end" {
            format_datetime(&val_str)
        } else {
            escape_ical_text(&val_str)
        };

        // Replace existing property or append before END:VEVENT
        let pattern = format!("{}:", ical_key);
        if let Some(pos) = ical_text.find(&pattern) {
            // Find end of this line
            let line_end = ical_text[pos..]
                .find("\r\n")
                .map(|e| pos + e + 2)
                .or_else(|| ical_text[pos..].find('\n').map(|e| pos + e + 1))
                .unwrap_or(ical_text.len());
            ical_text = format!(
                "{}:{}{}\r\n{}",
                &ical_text[..pos],
                pattern,
                ical_val,
                &ical_text[line_end..]
            );
        } else {
            // Insert before END:VEVENT
            if let Some(ve) = ical_text.find("END:VEVENT") {
                ical_text = format!(
                    "{}{}:{}\r\n{}",
                    &ical_text[..ve],
                    ical_key,
                    ical_val,
                    &ical_text[ve..]
                );
            }
        }
    }

    // PUT back with If-Match (ETag) for concurrency control
    let mut req = client
        .put(&event_uid)
        .header("Content-Type", "text/calendar; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass));

    if let Some(et) = &etag {
        req = req.header("If-Match", et);
    }

    let resp = req
        .body(ical_text)
        .send()
        .map_err(|e| format!("cal_update PUT: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 204 {
        return Err(format!("cal_update PUT: server returned {}", status));
    }

    Ok(Value::String("ok".to_string()))
}

/// `cal_delete(event_uid)` → "ok"
///
/// Deletes an event. event_uid is the full URL of the .ics resource.
pub fn builtin_cal_delete(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cal_delete: expected 1 arg (event_uid)".to_string());
    }
    let event_uid = str_arg(args, 0, "cal_delete")?;

    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_delete: lock: {}", e))?;
    let session = sessions
        .values()
        .next()
        .ok_or_else(|| "cal_delete: no CalDAV session found".to_string())?;

    let client = build_client()?;

    // First GET to obtain ETag for If-Match
    let get_resp = client
        .get(&event_uid)
        .basic_auth(&session.user, Some(&session.pass))
        .send()
        .map_err(|e| format!("cal_delete GET: {}", e))?;

    let etag = get_resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut req = client
        .delete(&event_uid)
        .basic_auth(&session.user, Some(&session.pass));

    if let Some(et) = &etag {
        req = req.header("If-Match", et);
    }

    let resp = req.send().map_err(|e| format!("cal_delete: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 204 {
        return Err(format!("cal_delete: server returned {}", status));
    }

    Ok(Value::String("ok".to_string()))
}

/// `cal_freebusy(calendar_id, start, end)` → JSON array of busy periods
///
/// Queries free/busy information for a calendar in the given date range.
pub fn builtin_cal_freebusy(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("cal_freebusy: expected 3 args (calendar_id, start, end)".to_string());
    }
    let calendar_id = str_arg(args, 0, "cal_freebusy")?;
    let start = str_arg(args, 1, "cal_freebusy")?;
    let end = str_arg(args, 2, "cal_freebusy")?;

    let sessions = CAL_SESSIONS
        .lock()
        .map_err(|e| format!("cal_freebusy: lock: {}", e))?;
    let session = sessions
        .values()
        .next()
        .ok_or_else(|| "cal_freebusy: no CalDAV session found".to_string())?;

    let client = build_client()?;

    // CalDAV free-busy-query REPORT (RFC 4791 §7.10)
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:free-busy-query xmlns:c="urn:ietf:params:xml:ns:caldav">
  <c:time-range start="{}T000000Z" end="{}T235959Z"/>
</c:free-busy-query>"#,
        start.replace('-', ""),
        end.replace('-', "")
    );

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"REPORT").unwrap_or(reqwest::Method::POST),
            &calendar_id,
        )
        .header("Depth", "0")
        .header("Content-Type", "application/xml; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(body)
        .send()
        .map_err(|e| format!("cal_freebusy: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("cal_freebusy: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("cal_freebusy: read body: {}", e))?;

    // Parse FREEBUSY lines from iCal response
    let mut busy = Vec::new();
    for line in text.lines() {
        if line.starts_with("FREEBUSY") {
            // FREEBUSY:20260813T100000Z/20260813T110000Z
            let value = line.split(':').nth(1).unwrap_or("");
            let parts: Vec<&str> = value.split('/').collect();
            if parts.len() >= 2 {
                busy.push(serde_json::json!({
                    "start": parts[0],
                    "end": parts[1],
                }));
            }
        }
    }

    Ok(Value::String(
        serde_json::to_string(&busy).unwrap_or_else(|_| "[]".to_string()),
    ))
}

// ── iCal parsing / generation ────────────────────────────────────────

/// `ical_parse(text)` → JSON struct of parsed iCal data
///
/// Parses an iCalendar text (VCALENDAR) into a structured JSON representation.
/// Uses the `ical` crate for robust parsing.
pub fn builtin_ical_parse(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("ical_parse: expected 1 arg (text)".to_string());
    }
    let text = str_arg(args, 0, "ical_parse")?;

    parse_ical_to_value(&text).map(Value::String)
}

/// `ical_generate(event_json)` → iCal text
///
/// Generates an iCalendar VCALENDAR text from a JSON event description.
/// event_json format: {"summary": "...", "start": "...", "end": "...",
///   "uid": "...", "description": "...", "location": "...",
///   "attendees": ["email1", "email2"]}
pub fn builtin_ical_generate(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("ical_generate: expected 1 arg (event_json)".to_string());
    }
    let event_json = str_arg(args, 0, "ical_generate")?;

    let event: serde_json::Map<String, serde_json::Value> = serde_json::from_str(&event_json)
        .map_err(|e| format!("ical_generate: invalid JSON: {}", e))?;

    let uid = event
        .get("uid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(gen_uid);
    let summary = event
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled Event");
    let start = event
        .get("start")
        .and_then(|v| v.as_str())
        .unwrap_or("20260101T000000Z");
    let end = event
        .get("end")
        .and_then(|v| v.as_str())
        .unwrap_or("20260101T010000Z");

    let mut ical = String::new();
    ical.push_str("BEGIN:VCALENDAR\r\n");
    ical.push_str("VERSION:2.0\r\n");
    ical.push_str("PRODID:-//Metalogos//MLG-5//RU\r\n");
    ical.push_str("BEGIN:VEVENT\r\n");
    ical.push_str(&format!("UID:{}\r\n", uid));
    ical.push_str(&format!("DTSTART:{}\r\n", format_datetime(start)));
    ical.push_str(&format!("DTEND:{}\r\n", format_datetime(end)));
    ical.push_str(&format!("SUMMARY:{}\r\n", escape_ical_text(summary)));

    if let Some(desc) = event.get("description").and_then(|v| v.as_str()) {
        ical.push_str(&format!("DESCRIPTION:{}\r\n", escape_ical_text(desc)));
    }
    if let Some(loc) = event.get("location").and_then(|v| v.as_str()) {
        ical.push_str(&format!("LOCATION:{}\r\n", escape_ical_text(loc)));
    }
    if let Some(attendees) = event.get("attendees").and_then(|v| v.as_array()) {
        for att in attendees {
            if let Some(email) = att.as_str() {
                ical.push_str(&format!("ATTENDEE;CN={}:mailto:{}\r\n", email, email));
            }
        }
    }

    ical.push_str("END:VEVENT\r\n");
    ical.push_str("END:VCALENDAR\r\n");

    Ok(Value::String(ical))
}

// ── Internal helpers ─────────────────────────────────────────────────

/// Format a datetime string for iCal.
/// Accepts: "2026-08-13T10:00:00", "2026-08-13 10:00:00", "20260813T100000Z"
/// Returns: "20260813T100000Z"
fn format_datetime(input: &str) -> String {
    let s = input.trim();
    // Already in iCal format: "20260813T103000Z" or "20260813T103000"
    if s.len() >= 15 && s.chars().nth(8) == Some('T') && !s.contains('-') {
        // Ensure it ends with Z
        if s.ends_with('Z') {
            return s.to_string();
        }
        return format!("{}Z", s);
    }
    // Parse YYYY-MM-DDTHH:MM:SS or YYYY-MM-DD HH:MM:SS
    let cleaned = s.replace(' ', "T");
    let parts: Vec<&str> = cleaned.split('T').collect();
    if parts.len() >= 2 {
        let date_part = parts[0].replace('-', "");
        let time_part = parts[1].replace(':', "");
        format!("{}T{}Z", date_part, time_part)
    } else {
        // Just a date
        format!("{}T000000Z", parts[0].replace('-', ""))
    }
}

/// Escape text for iCal (RFC 5545 §3.3.11):
/// - Backslash → \\
/// - Semicolon → \;
/// - Comma → \,
/// - Newline → \n
fn escape_ical_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
}

/// Parse iCal text into a JSON string representation.
/// Returns a JSON object with events array.
fn parse_ical_to_value(text: &str) -> Result<String, String> {
    use ical::IcalParser;
    use std::io::BufReader;

    let reader = BufReader::new(text.as_bytes());
    let parser = IcalParser::new(reader);

    let mut result = serde_json::Map::new();
    let mut all_events = Vec::new();

    for calendar_result in parser {
        let calendar = match calendar_result {
            Ok(c) => c,
            Err(_) => continue, // Non-fatal parse error — skip
        };

        // Calendar-level properties
        for property in &calendar.properties {
            result.insert(
                property.name.clone(),
                serde_json::Value::String(property.value.clone().unwrap_or_default()),
            );
        }

        // Events
        for event in &calendar.events {
            let mut ev = serde_json::Map::new();
            for property in &event.properties {
                let name = property.name.clone();
                let value = property.value.clone().unwrap_or_default();
                // Handle multi-valued properties (like ATTENDEE)
                if ev.contains_key(&name) {
                    let existing = ev.get(&name).cloned().unwrap_or(serde_json::Value::Null);
                    if let serde_json::Value::Array(arr) = existing {
                        let mut new_arr = arr;
                        new_arr.push(serde_json::Value::String(value));
                        ev.insert(name, serde_json::Value::Array(new_arr));
                    } else {
                        ev.insert(name, serde_json::json!([existing, value]));
                    }
                } else {
                    ev.insert(name, serde_json::Value::String(value));
                }
            }
            all_events.push(serde_json::Value::Object(ev));
        }

        // Free/Busy periods
        for fb in &calendar.free_busys {
            let mut fb_map = serde_json::Map::new();
            for property in &fb.properties {
                fb_map.insert(
                    property.name.clone(),
                    serde_json::Value::String(property.value.clone().unwrap_or_default()),
                );
            }
            all_events.push(serde_json::Value::Object(fb_map));
        }
    }

    result.insert("events".to_string(), serde_json::Value::Array(all_events));

    serde_json::to_string(&result).map_err(|e| format!("ical_parse: JSON serialize: {}", e))
}

// ── Unit tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_datetime_iso() {
        assert_eq!(format_datetime("2026-08-13T10:30:00"), "20260813T103000Z");
    }

    #[test]
    fn test_format_datetime_space() {
        assert_eq!(format_datetime("2026-08-13 10:30:00"), "20260813T103000Z");
    }

    #[test]
    fn test_format_datetime_ical() {
        assert_eq!(format_datetime("20260813T103000Z"), "20260813T103000Z");
    }

    #[test]
    fn test_escape_ical_text() {
        assert_eq!(escape_ical_text("Hello, World!"), "Hello\\, World!");
        assert_eq!(escape_ical_text("Line1\nLine2"), "Line1\\nLine2");
        assert_eq!(escape_ical_text("A;B"), "A\\;B");
    }

    #[test]
    fn test_ical_generate_basic() {
        let json = r#"{"summary":"Test Meeting","start":"2026-08-13T10:00:00","end":"2026-08-13T11:00:00","uid":"test-uid-123"}"#;
        let result = builtin_ical_generate(&[Value::String(json.to_string())]);
        assert!(result.is_ok());
        let ical = match result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };
        assert!(ical.contains("BEGIN:VCALENDAR"));
        assert!(ical.contains("SUMMARY:Test Meeting"));
        assert!(ical.contains("UID:test-uid-123"));
        assert!(ical.contains("DTSTART:20260813T100000Z"));
        assert!(ical.contains("DTEND:20260813T110000Z"));
        assert!(ical.contains("END:VCALENDAR"));
    }

    #[test]
    fn test_ical_generate_with_attendees() {
        let json = r#"{"summary":"Board Meeting","start":"2026-08-13T14:00:00","end":"2026-08-13T15:30:00","attendees":["alice@example.com","bob@example.com"]}"#;
        let result = builtin_ical_generate(&[Value::String(json.to_string())]);
        assert!(result.is_ok());
        let ical = match result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };
        assert!(ical.contains("ATTENDEE"));
        assert!(ical.contains("alice@example.com"));
        assert!(ical.contains("bob@example.com"));
    }

    #[test]
    fn test_ical_parse_basic() {
        let ical = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:test-123\r\nSUMMARY:Test Event\r\nDTSTART:20260813T100000Z\r\nDTEND:20260813T110000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let result = builtin_ical_parse(&[Value::String(ical.to_string())]);
        assert!(result.is_ok());
        let json_str = match result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };
        // Should be valid JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("ical_parse output should be valid JSON");
        assert!(parsed.get("events").is_some());
    }

    #[test]
    fn test_ical_parse_invalid() {
        let result = builtin_ical_parse(&[Value::String("not valid ical".to_string())]);
        // Should still return something (graceful handling)
        assert!(result.is_ok());
    }

    #[test]
    fn test_cal_connect_wrong_arity() {
        let result = builtin_cal_connect(&[Value::String("url".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cal_list_no_session() {
        let result = builtin_cal_list(&[Value::String("nonexistent".to_string())]);
        assert!(result.is_err());
    }
}
