// ── Наряд MLG-6: Contacts (CardDAV + vCard) ────────────────────────────
// Pure Rust: CardDAV client over reqwest + vCard parsing/generation.
// CardDAV protocol: RFC 6352, vCard: RFC 6350.

use crate::interpreter::Value;

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;

// ── CardDAV session store ─────────────────────────────────────────────

/// A CardDAV session: credentials + HTTP client for a server.
struct CardSession {
    url: String,
    user: String,
    pass: String,
    /// Cached addressbook-home-set URL (discovered via PROPFIND).
    home_set: Option<String>,
}

static CARD_SESSIONS: Lazy<Mutex<HashMap<String, CardSession>>> =
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

/// Generate a unique session/addressbook/contact ID.
fn gen_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("card_{:x}", ts)
}

/// Generate a UUID4-like string for contact UIDs.
fn gen_uid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{:08x}-{:04x}-4{:03x}-b{:03x}-{:012x}",
        (ts >> 32) as u32,
        (ts >> 16) as u16 & 0xFFF,
        ts as u32 & 0xFFF,
        (ts >> 4) as u32 & 0xFFF,
        ts as u64 & 0xFFFFFFFFFFFF
    )
}

/// Build a reqwest blocking client.
fn build_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .danger_accept_invalid_certs(true) // Self-hosted CardDAV servers often use self-signed
        .build()
        .map_err(|e| format!("card_connect: failed to build HTTP client: {}", e))
}

/// Execute a PROPFIND request to discover addressbook-home-set.
fn propfind_home_set(
    client: &reqwest::blocking::Client,
    url: &str,
    user: &str,
    pass: &str,
) -> Result<String, String> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop>
    <addressbook-home-set xmlns="urn:ietf:params:xml:ns:carddav"/>
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
        .map_err(|e| format!("card_connect PROPFIND: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("card_connect PROPFIND: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("card_connect PROPFIND read body: {}", e))?;

    // Extract addressbook-home-set href from XML response.
    if let Some(pos) = text.find("addressbook-home-set") {
        let rest = &text[pos..];
        if let Some(href_start) = rest.find("<href>") {
            let href_end = rest.find("</href>").unwrap_or(rest.len());
            let href = &rest[href_start + 6..href_end];
            return Ok(href.trim().to_string());
        }
    }

    // Fallback: assume /addressbooks/ path
    Ok(format!("{}/addressbooks/", url.trim_end_matches('/')))
}

// ═══════════════════════════════════════════════════════════════════════
// CardDAV builtins
// ═══════════════════════════════════════════════════════════════════════

/// `card_connect(url, user, pass)` → session_id
///
/// Connects to a CardDAV server, discovers addressbook-home-set via PROPFIND,
/// and stores a session. Returns the session ID for use in subsequent calls.
pub fn builtin_card_connect(args: &[Value]) -> Result<Value, String> {
    if args.len() != 3 {
        return Err("card_connect: expected 3 args (url, user, pass)".to_string());
    }
    let url = str_arg(args, 0, "card_connect")?;
    let user = str_arg(args, 1, "card_connect")?;
    let pass = str_arg(args, 2, "card_connect")?;

    let client = build_client()?;

    // Try PROPFIND to discover addressbook-home-set
    let home_set = propfind_home_set(&client, &url, &user, &pass).ok();

    let session_id = gen_id();
    let mut sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_connect: lock: {}", e))?;
    sessions.insert(
        session_id.clone(),
        CardSession {
            url,
            user,
            pass,
            home_set,
        },
    );

    Ok(Value::String(session_id))
}

/// `card_list(session_id)` → JSON array of address books
///
/// Lists available address books from the CardDAV server.
/// Returns a JSON string like: [{"href":"...", "displayname":"Personal"}, ...]
pub fn builtin_card_list(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("card_list: expected 1 arg (session_id)".to_string());
    }
    let session_id = str_arg(args, 0, "card_list")?;

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_list: lock: {}", e))?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| format!("card_list: session '{}' not found", session_id))?;

    let home_url = session.home_set.as_deref().unwrap_or(session.url.as_str());

    let client = build_client()?;

    // PROPFIND Depth:1 on the home set to list address books
    let body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop>
    <displayname/>
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
        .map_err(|e| format!("card_list: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("card_list: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("card_list: read body: {}", e))?;

    // Parse response to extract address books with their hrefs and displaynames.
    let mut addressbooks = Vec::new();
    let mut pos = 0;
    while let Some(start) = text[pos..].find("<response>") {
        let block_start = pos + start;
        let block_end = text[block_start..]
            .find("</response>")
            .map(|e| block_start + e + 11)
            .unwrap_or(text.len());
        let block = &text[block_start..block_end];

        // Check if this is an address book (has <addressbook/> in resourcetype)
        if block.contains("addressbook") {
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

            addressbooks.push(serde_json::json!({
                "href": href,
                "displayname": displayname,
            }));
        }

        pos = block_end;
    }

    Ok(Value::String(
        serde_json::to_string(&addressbooks).unwrap_or_else(|_| "[]".to_string()),
    ))
}

/// `card_contacts(addressbook_id, query)` → JSON array of contacts
///
/// Fetches contacts from a CardDAV address book, optionally filtered by query.
/// If query is empty "", returns all contacts (limited to 100).
/// Returns JSON string of contact structs.
pub fn builtin_card_contacts(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("card_contacts: expected 2 args (addressbook_id, query)".to_string());
    }
    let addressbook_id = str_arg(args, 0, "card_contacts")?;
    let query = str_arg(args, 1, "card_contacts")?;

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_contacts: lock: {}", e))?;
    let session = sessions.values().next().ok_or_else(|| {
        "card_contacts: no CardDAV session found — call card_connect first".to_string()
    })?;

    let client = build_client()?;

    // CardDAV addressbook-query REPORT (RFC 6352 §8.6)
    let filter_xml = if query.is_empty() {
        // No filter — return all contacts
        r#"      <c:filter test="anyof">
        <c:prop-filter name="FN">
          <c:text-match collation="i;unicode-casemap" match-type="contains">*</c:text-match>
        </c:prop-filter>
      </c:filter>"#
            .to_string()
    } else {
        format!(
            r#"      <c:filter test="anyof">
        <c:prop-filter name="FN">
          <c:text-match collation="i;unicode-casemap" match-type="contains">{}</c:text-match>
        </c:prop-filter>
        <c:prop-filter name="EMAIL">
          <c:text-match collation="i;unicode-casemap" match-type="contains">{}</c:text-match>
        </c:prop-filter>
      </c:filter>"#,
            xml_escape(&query),
            xml_escape(&query)
        )
    };

    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<c:addressbook-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
    <c:address-data/>
  </d:prop>
{}
  <c:limit>
    <c:nresults>100</c:nresults>
  </c:limit>
</c:addressbook-query>"#,
        filter_xml
    );

    let resp = client
        .request(
            reqwest::Method::from_bytes(b"REPORT").unwrap_or(reqwest::Method::GET),
            &addressbook_id,
        )
        .header("Depth", "1")
        .header("Content-Type", "application/xml; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(body)
        .send()
        .map_err(|e| format!("card_contacts: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 207 {
        return Err(format!("card_contacts: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("card_contacts: read body: {}", e))?;

    // Parse response: extract vCard data from <address-data> elements
    let mut contacts = Vec::new();
    let mut pos = 0;
    while let Some(start) = text[pos..].find("<response>") {
        let block_start = pos + start;
        let block_end = text[block_start..]
            .find("</response>")
            .map(|e| block_start + e + 11)
            .unwrap_or(text.len());
        let block = &text[block_start..block_end];

        // Extract vCard data from <address-data> element
        if let Some(ad_start) = block.find("<c:address-data>") {
            let ad_content_start = ad_start + 16;
            let ad_end = block[ad_content_start..]
                .find("</c:address-data>")
                .map(|e| ad_content_start + e)
                .unwrap_or(block.len());
            let vcard_text = &block[ad_content_start..ad_end];

            // Parse the vCard into a JSON object
            if let Ok(parsed) = vcard_to_json(vcard_text) {
                contacts.push(parsed);
            }
        } else if let Some(ad_start) = block.find("<address-data>") {
            let ad_content_start = ad_start + 15;
            let ad_end = block[ad_content_start..]
                .find("</address-data>")
                .map(|e| ad_content_start + e)
                .unwrap_or(block.len());
            let vcard_text = &block[ad_content_start..ad_end];

            if let Ok(parsed) = vcard_to_json(vcard_text) {
                contacts.push(parsed);
            }
        }

        pos = block_end;
    }

    Ok(Value::String(
        serde_json::to_string(&contacts).unwrap_or_else(|_| "[]".to_string()),
    ))
}

/// `card_read(contact_uid)` → JSON object of contact
///
/// Reads a single contact by its URL (the contact_uid is actually the href).
pub fn builtin_card_read(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("card_read: expected 1 arg (contact_uid)".to_string());
    }
    let contact_url = str_arg(args, 0, "card_read")?;

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_read: lock: {}", e))?;
    let session = sessions.values().next().ok_or_else(|| {
        "card_read: no CardDAV session found — call card_connect first".to_string()
    })?;

    let client = build_client()?;

    let resp = client
        .get(&contact_url)
        .basic_auth(&session.user, Some(&session.pass))
        .send()
        .map_err(|e| format!("card_read: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        return Err(format!("card_read: server returned {}", status));
    }

    let text = resp
        .text()
        .map_err(|e| format!("card_read: read body: {}", e))?;

    // Parse the vCard response
    let contact = vcard_to_json(&text)?;
    Ok(Value::String(
        serde_json::to_string(&contact).unwrap_or_else(|_| "{}".to_string()),
    ))
}

/// `card_create(addressbook_id, fn, email [,tel, org, title, note])`
///
/// Creates a new contact in the address book. Returns the UID of the new contact.
/// Arity: 3..7
pub fn builtin_card_create(args: &[Value]) -> Result<Value, String> {
    if args.len() < 3 || args.len() > 7 {
        return Err(
            "card_create: expected 3..7 args (addressbook_id, fn, email [,tel, org, title, note])"
                .to_string(),
        );
    }
    let addressbook_id = str_arg(args, 0, "card_create")?;
    let fn_name = str_arg(args, 1, "card_create")?;
    let email = str_arg(args, 2, "card_create")?;
    let tel = opt_str_arg(args, 3);
    let org = opt_str_arg(args, 4);
    let title = opt_str_arg(args, 5);
    let note = opt_str_arg(args, 6);

    let uid = gen_uid();

    // Build vCard text
    let mut vcard = format!(
        "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{}\r\nFN:{}\r\nEMAIL:{}\r\n",
        uid, fn_name, email
    );
    if let Some(ref t) = tel {
        vcard.push_str(&format!("TEL:{}\r\n", t));
    }
    if let Some(ref o) = org {
        vcard.push_str(&format!("ORG:{}\r\n", o));
    }
    if let Some(ref t) = title {
        vcard.push_str(&format!("TITLE:{}\r\n", t));
    }
    if let Some(ref n) = note {
        vcard.push_str(&format!("NOTE:{}\r\n", vcard_escape_text(n)));
    }
    vcard.push_str("END:VCARD\r\n");

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_create: lock: {}", e))?;
    let session = sessions.values().next().ok_or_else(|| {
        "card_create: no CardDAV session found — call card_connect first".to_string()
    })?;

    let client = build_client()?;

    // PUT the new .vcf file
    let url = format!("{}/{}.vcf", addressbook_id.trim_end_matches('/'), uid);

    let resp = client
        .put(&url)
        .header("Content-Type", "text/vcard; charset=utf-8")
        .header("If-None-Match", "*") // Create only (no overwrite)
        .basic_auth(&session.user, Some(&session.pass))
        .body(vcard)
        .send()
        .map_err(|e| format!("card_create: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 201 {
        return Err(format!("card_create: server returned {}", status));
    }

    Ok(Value::String(uid))
}

/// `card_update(contact_uid, fields_json)` → Struct { ok: true }
///
/// Updates fields of an existing contact. fields_json is a JSON object
/// with keys like "FN", "EMAIL", "TEL", "ORG", "TITLE", "NOTE".
pub fn builtin_card_update(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("card_update: expected 2 args (contact_uid, fields_json)".to_string());
    }
    let contact_url = str_arg(args, 0, "card_update")?;
    let fields_json = str_arg(args, 1, "card_update")?;

    let fields: serde_json::Value = serde_json::from_str(&fields_json)
        .map_err(|e| format!("card_update: invalid fields_json: {}", e))?;

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_update: lock: {}", e))?;
    let session = sessions.values().next().ok_or_else(|| {
        "card_update: no CardDAV session found — call card_connect first".to_string()
    })?;

    let client = build_client()?;

    // GET the existing contact
    let get_resp = client
        .get(&contact_url)
        .basic_auth(&session.user, Some(&session.pass))
        .send()
        .map_err(|e| format!("card_update GET: {}", e))?;

    let status = get_resp.status();
    if !status.is_success() {
        return Err(format!("card_update GET: server returned {}", status));
    }

    let etag = get_resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let existing_vcard = get_resp
        .text()
        .map_err(|e| format!("card_update GET read body: {}", e))?;

    // Apply updates to the vCard
    let updated_vcard = apply_vcard_updates(&existing_vcard, &fields);

    // PUT the updated vCard with If-Match for concurrency control
    let mut req = client
        .put(&contact_url)
        .header("Content-Type", "text/vcard; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(updated_vcard);

    if let Some(ref et) = etag {
        req = req.header("If-Match", et);
    }

    let put_resp = req.send().map_err(|e| format!("card_update PUT: {}", e))?;

    let put_status = put_resp.status();
    if !put_status.is_success() && put_status.as_u16() != 204 {
        return Err(format!("card_update PUT: server returned {}", put_status));
    }

    Ok(Value::String("{\"ok\":true}".to_string()))
}

/// `card_delete(contact_uid)` → Struct { ok: true }
///
/// Deletes a contact by its URL.
pub fn builtin_card_delete(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("card_delete: expected 1 arg (contact_uid)".to_string());
    }
    let contact_url = str_arg(args, 0, "card_delete")?;

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_delete: lock: {}", e))?;
    let session = sessions.values().next().ok_or_else(|| {
        "card_delete: no CardDAV session found — call card_connect first".to_string()
    })?;

    let client = build_client()?;

    // GET the ETag first for If-Match
    let get_resp = client
        .get(&contact_url)
        .basic_auth(&session.user, Some(&session.pass))
        .send()
        .map_err(|e| format!("card_delete GET: {}", e))?;

    let etag = get_resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut req = client
        .delete(&contact_url)
        .basic_auth(&session.user, Some(&session.pass));

    if let Some(ref et) = etag {
        req = req.header("If-Match", et);
    }

    let resp = req.send().map_err(|e| format!("card_delete: {}", e))?;

    let status = resp.status();
    if !status.is_success() && status.as_u16() != 204 {
        return Err(format!("card_delete: server returned {}", status));
    }

    Ok(Value::String("{\"ok\":true}".to_string()))
}

/// `card_search(session_id, query)` → JSON array of contacts
///
/// Searches across all address books for contacts matching the query.
/// Searches FN (formatted name) and EMAIL fields.
pub fn builtin_card_search(args: &[Value]) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("card_search: expected 2 args (session_id, query)".to_string());
    }
    let session_id = str_arg(args, 0, "card_search")?;
    let query = str_arg(args, 1, "card_search")?;

    let sessions = CARD_SESSIONS
        .lock()
        .map_err(|e| format!("card_search: lock: {}", e))?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| format!("card_search: session '{}' not found", session_id))?;

    let home_url = session.home_set.as_deref().unwrap_or(session.url.as_str());

    let client = build_client()?;

    // First, list address books
    let list_body = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:">
  <prop>
    <displayname/>
    <resourcetype/>
  </prop>
</propfind>"#;

    let list_resp = client
        .request(
            reqwest::Method::from_bytes(b"PROPFIND").unwrap_or(reqwest::Method::GET),
            home_url,
        )
        .header("Depth", "1")
        .header("Content-Type", "application/xml; charset=utf-8")
        .basic_auth(&session.user, Some(&session.pass))
        .body(list_body)
        .send()
        .map_err(|e| format!("card_search list: {}", e))?;

    let list_text = list_resp
        .text()
        .map_err(|e| format!("card_search list: read body: {}", e))?;

    // Find address book hrefs
    let mut ab_hrefs = Vec::new();
    let mut pos = 0;
    while let Some(start) = list_text[pos..].find("<response>") {
        let block_start = pos + start;
        let block_end = list_text[block_start..]
            .find("</response>")
            .map(|e| block_start + e + 11)
            .unwrap_or(list_text.len());
        let block = &list_text[block_start..block_end];

        if block.contains("addressbook") {
            if let Some(hs) = block.find("<href>") {
                let he = block[hs + 6..]
                    .find("</href>")
                    .map(|e| hs + 6 + e)
                    .unwrap_or(block.len());
                ab_hrefs.push(block[hs + 6..he].trim().to_string());
            }
        }
        pos = block_end;
    }

    // Search in each address book
    let mut all_contacts = Vec::new();
    for ab_href in &ab_hrefs {
        let filter_xml = format!(
            r#"      <c:filter test="anyof">
        <c:prop-filter name="FN">
          <c:text-match collation="i;unicode-casemap" match-type="contains">{}</c:text-match>
        </c:prop-filter>
        <c:prop-filter name="EMAIL">
          <c:text-match collation="i;unicode-casemap" match-type="contains">{}</c:text-match>
        </c:prop-filter>
      </c:filter>"#,
            xml_escape(&query),
            xml_escape(&query)
        );

        let search_body = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<c:addressbook-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
    <c:address-data/>
  </d:prop>
{}
  <c:limit>
    <c:nresults>50</c:nresults>
  </c:limit>
</c:addressbook-query>"#,
            filter_xml
        );

        let search_resp = client
            .request(
                reqwest::Method::from_bytes(b"REPORT").unwrap_or(reqwest::Method::GET),
                ab_href,
            )
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .basic_auth(&session.user, Some(&session.pass))
            .body(search_body)
            .send()
            .map_err(|e| format!("card_search: {}", e))?;

        let search_text = search_resp
            .text()
            .map_err(|e| format!("card_search: read body: {}", e))?;

        // Parse vCard data from response
        let mut inner_pos = 0;
        while let Some(start) = search_text[inner_pos..].find("<response>") {
            let bs = inner_pos + start;
            let be = search_text[bs..]
                .find("</response>")
                .map(|e| bs + e + 11)
                .unwrap_or(search_text.len());
            let block = &search_text[bs..be];

            if let Some(ad_start) = block.find("<c:address-data>") {
                let cs = ad_start + 16;
                let ce = block[cs..]
                    .find("</c:address-data>")
                    .map(|e| cs + e)
                    .unwrap_or(block.len());
                if let Ok(parsed) = vcard_to_json(&block[cs..ce]) {
                    all_contacts.push(parsed);
                }
            } else if let Some(ad_start) = block.find("<address-data>") {
                let cs = ad_start + 15;
                let ce = block[cs..]
                    .find("</address-data>")
                    .map(|e| cs + e)
                    .unwrap_or(block.len());
                if let Ok(parsed) = vcard_to_json(&block[cs..ce]) {
                    all_contacts.push(parsed);
                }
            }

            inner_pos = be;
        }
    }

    Ok(Value::String(
        serde_json::to_string(&all_contacts).unwrap_or_else(|_| "[]".to_string()),
    ))
}

// ═══════════════════════════════════════════════════════════════════════
// vCard parsing and generation
// ═══════════════════════════════════════════════════════════════════════

/// `vcard_parse(text)` → JSON object
///
/// Parses a vCard (RFC 6350) text into a JSON object.
/// Handles folded lines, basic properties: FN, N, EMAIL, TEL, ORG, TITLE, NOTE, UID, PHOTO, URL, ADR.
pub fn builtin_vcard_parse(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("vcard_parse: expected 1 arg (text)".to_string());
    }
    let text = str_arg(args, 0, "vcard_parse")?;

    let parsed = vcard_to_json(&text)?;
    Ok(Value::String(
        serde_json::to_string(&parsed).unwrap_or_else(|_| "{}".to_string()),
    ))
}

/// `vcard_generate(contact_json)` → vCard text
///
/// Generates a vCard (version 4.0) text from a JSON object.
/// JSON keys map to vCard properties: fn→FN, email→EMAIL, tel→TEL, org→ORG,
/// title→TITLE, note→NOTE, uid→UID, photo→PHOTO, url→URL, adr→ADR.
pub fn builtin_vcard_generate(args: &[Value]) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("vcard_generate: expected 1 arg (contact_json)".to_string());
    }
    let json_str = str_arg(args, 0, "vcard_generate")?;

    let contact: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("vcard_generate: invalid JSON: {}", e))?;

    let uid = contact
        .get("uid")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(gen_uid);
    let fn_name = contact
        .get("fn")
        .or_else(|| contact.get("FN"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown");

    let mut vcard = format!(
        "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{}\r\nFN:{}\r\n",
        uid, fn_name
    );

    // N (structured name) — generate from FN if not provided
    if let Some(n) = contact.get("N").or_else(|| contact.get("n")) {
        if let Some(s) = n.as_str() {
            vcard.push_str(&format!("N:{}\r\n", s));
        }
    } else {
        // Simple: assume FN is the family name for auto-N
        vcard.push_str(&format!("N:{};;;;\r\n", fn_name));
    }

    // EMAIL — can be array or single
    if let Some(email) = contact.get("email").or_else(|| contact.get("EMAIL")) {
        append_vcard_prop(&mut vcard, "EMAIL", email);
    }

    // TEL
    if let Some(tel) = contact.get("tel").or_else(|| contact.get("TEL")) {
        append_vcard_prop(&mut vcard, "TEL", tel);
    }

    // ORG
    if let Some(org) = contact.get("org").or_else(|| contact.get("ORG")) {
        if let Some(s) = org.as_str() {
            vcard.push_str(&format!("ORG:{}\r\n", s));
        }
    }

    // TITLE
    if let Some(title) = contact.get("title").or_else(|| contact.get("TITLE")) {
        if let Some(s) = title.as_str() {
            vcard.push_str(&format!("TITLE:{}\r\n", s));
        }
    }

    // NOTE
    if let Some(note) = contact.get("note").or_else(|| contact.get("NOTE")) {
        if let Some(s) = note.as_str() {
            vcard.push_str(&format!("NOTE:{}\r\n", vcard_escape_text(s)));
        }
    }

    // PHOTO
    if let Some(photo) = contact.get("photo").or_else(|| contact.get("PHOTO")) {
        if let Some(s) = photo.as_str() {
            vcard.push_str(&format!("PHOTO:{}\r\n", s));
        }
    }

    // URL
    if let Some(url) = contact.get("url").or_else(|| contact.get("URL")) {
        if let Some(s) = url.as_str() {
            vcard.push_str(&format!("URL:{}\r\n", s));
        }
    }

    // ADR (address)
    if let Some(adr) = contact.get("adr").or_else(|| contact.get("ADR")) {
        if let Some(s) = adr.as_str() {
            vcard.push_str(&format!("ADR:{}\r\n", s));
        }
    }

    vcard.push_str("END:VCARD\r\n");
    Ok(Value::String(vcard))
}

// ═══════════════════════════════════════════════════════════════════════
// Internal vCard utilities
// ═══════════════════════════════════════════════════════════════════════

/// Parse a vCard text into a JSON Value.
fn vcard_to_json(text: &str) -> Result<serde_json::Value, String> {
    let mut contact = serde_json::Map::new();

    // Unfold lines (RFC 6350 §3.2): lines starting with space/tab are continuations
    let unfolded = unfold_vcard_lines(text);

    for line in unfolded.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("BEGIN:") || line.starts_with("END:") {
            continue;
        }

        // Parse property: NAME;PARAM1=VAL1;PARAM2=VAL2:VALUE
        let (prop_name, value) = if let Some(colon_pos) = line.find(':') {
            let name_part = &line[..colon_pos];
            let value_part = &line[colon_pos + 1..];
            // Strip parameters from name
            let name = if let Some(semi) = name_part.find(';') {
                &name_part[..semi]
            } else {
                name_part
            };
            (name.to_uppercase(), value_part.to_string())
        } else {
            continue;
        };

        // Handle multi-valued properties (EMAIL, TEL, URL, ADR)
        match prop_name.as_str() {
            "VERSION" | "PRODID" => {
                contact.insert(prop_name, serde_json::Value::String(value));
            }
            "EMAIL" | "TEL" | "URL" | "ADR" => {
                // Accumulate into array
                let key = prop_name.clone();
                let arr = contact
                    .entry(key)
                    .or_insert_with(|| serde_json::Value::Array(Vec::new()));
                if let Some(arr) = arr.as_array_mut() {
                    arr.push(serde_json::Value::String(value));
                }
            }
            _ => {
                contact.insert(prop_name, serde_json::Value::String(value));
            }
        }
    }

    Ok(serde_json::Value::Object(contact))
}

/// Unfold vCard lines per RFC 6350 §3.2.
/// Lines starting with SP/HTAB are continuations of the previous line.
fn unfold_vcard_lines(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_ends_with_newline = false;

    for line in text.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // Continuation line — append without newline
            result.push_str(&line[1..]);
            prev_ends_with_newline = false;
        } else {
            if prev_ends_with_newline || !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
            prev_ends_with_newline = true;
        }
    }

    result
}

/// Apply field updates to an existing vCard text.
fn apply_vcard_updates(vcard: &str, fields: &serde_json::Value) -> String {
    let obj = match fields.as_object() {
        Some(o) => o,
        None => return vcard.to_string(),
    };

    let unfolded = unfold_vcard_lines(vcard);
    let mut lines: Vec<String> = unfolded.lines().map(|l| l.to_string()).collect();

    for (key, value) in obj {
        let vcard_key = key.to_uppercase();
        let val_str = match value.as_str() {
            Some(s) => s.to_string(),
            None => value.to_string(),
        };

        // Try to find and replace existing property
        let mut found = false;
        for line in &mut lines {
            let upper = line.to_uppercase();
            if upper.starts_with(&format!("{}:", vcard_key))
                || upper.starts_with(&format!("{};", vcard_key))
            {
                *line = format!("{}:{}", vcard_key, val_str);
                found = true;
                break;
            }
        }

        // If not found, add before END:VCARD
        if !found {
            if let Some(end_pos) = lines.iter().position(|l| l.to_uppercase() == "END:VCARD") {
                lines.insert(end_pos, format!("{}:{}", vcard_key, val_str));
            }
        }
    }

    // Re-fold long lines (RFC 6350: max 75 octets per line)
    let mut result = String::new();
    for line in &lines {
        result.push_str(&fold_vcard_line(line));
        result.push_str("\r\n");
    }
    result
}

/// Fold a vCard line to max 75 octets (RFC 6350 §3.2).
fn fold_vcard_line(line: &str) -> String {
    if line.len() <= 75 {
        return line.to_string();
    }

    let mut result = String::with_capacity(line.len() + 20);
    let bytes = line.as_bytes();
    let mut pos = 0;

    // First chunk: 75 bytes
    let first_end = std::cmp::min(75, bytes.len());
    result.push_str(&line[pos..first_end]);
    pos = first_end;

    // Subsequent chunks: 74 bytes (1 space + 74 content = 75)
    while pos < bytes.len() {
        result.push_str("\r\n ");
        let chunk_end = std::cmp::min(pos + 74, bytes.len());
        // Safety: we're slicing at byte boundaries within the same string
        result.push_str(std::str::from_utf8(&bytes[pos..chunk_end]).unwrap_or(""));
        pos = chunk_end;
    }

    result
}

/// Escape text for vCard NOTE and other text fields.
fn vcard_escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

/// Escape text for XML content.
fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Append a vCard property that may be a single value or an array.
fn append_vcard_prop(vcard: &mut String, prop: &str, value: &serde_json::Value) {
    match value {
        serde_json::Value::String(s) => {
            vcard.push_str(&format!("{}:{}\r\n", prop, s));
        }
        serde_json::Value::Array(arr) => {
            for item in arr {
                if let Some(s) = item.as_str() {
                    vcard.push_str(&format!("{}:{}\r\n", prop, s));
                }
            }
        }
        _ => {}
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Inline tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_uid_format() {
        let uid = gen_uid();
        // Should be UUID-like: 8-4-4-4-12 hex groups
        let parts: Vec<&str> = uid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
        // Version nibble
        assert!(parts[2].starts_with('4'));
        // Variant nibble
        assert!(parts[3].starts_with('b'));
    }

    #[test]
    fn test_vcard_to_json_basic() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:test-123\r\nFN:John Doe\r\nEMAIL:john@example.com\r\nTEL:+1-555-0100\r\nORG:Acme Corp\r\nTITLE:Engineer\r\nEND:VCARD\r\n";
        let json = vcard_to_json(vcard).unwrap();
        assert_eq!(json["VERSION"], "4.0");
        assert_eq!(json["UID"], "test-123");
        assert_eq!(json["FN"], "John Doe");
        assert_eq!(json["ORG"], "Acme Corp");
        assert_eq!(json["TITLE"], "Engineer");
        // EMAIL and TEL should be arrays
        let emails = json["EMAIL"].as_array().unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0], "john@example.com");
        let tels = json["TEL"].as_array().unwrap();
        assert_eq!(tels.len(), 1);
        assert_eq!(tels[0], "+1-555-0100");
    }

    #[test]
    fn test_vcard_to_json_multi_email() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Jane\r\nEMAIL:work@example.com\r\nEMAIL:home@example.com\r\nEND:VCARD\r\n";
        let json = vcard_to_json(vcard).unwrap();
        let emails = json["EMAIL"].as_array().unwrap();
        assert_eq!(emails.len(), 2);
        assert_eq!(emails[0], "work@example.com");
        assert_eq!(emails[1], "home@example.com");
    }

    #[test]
    fn test_vcard_to_json_with_params() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Bob\r\nTEL;TYPE=work,voice:+1-555-0200\r\nEMAIL;TYPE=work:bob@corp.com\r\nEND:VCARD\r\n";
        let json = vcard_to_json(vcard).unwrap();
        assert_eq!(json["FN"], "Bob");
        let tels = json["TEL"].as_array().unwrap();
        assert_eq!(tels[0], "+1-555-0200");
        let emails = json["EMAIL"].as_array().unwrap();
        assert_eq!(emails[0], "bob@corp.com");
    }

    #[test]
    fn test_unfold_vcard_lines() {
        let folded = "NOTE:This is a long\r\n note that was\r\n folded across lines";
        let unfolded = unfold_vcard_lines(folded);
        assert_eq!(
            unfolded,
            "NOTE:This is a longnote that wasfolded across lines"
        );
    }

    #[test]
    fn test_vcard_escape_text() {
        assert_eq!(vcard_escape_text("hello, world"), "hello\\, world");
        assert_eq!(vcard_escape_text("a;b"), "a\\;b");
        assert_eq!(vcard_escape_text("line1\nline2"), "line1\\nline2");
        assert_eq!(vcard_escape_text("a\\b"), "a\\\\b");
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("a&b"), "a&amp;b");
        assert_eq!(xml_escape("<tag>"), "&lt;tag&gt;");
        assert_eq!(xml_escape("\"q\""), "&quot;q&quot;");
    }

    #[test]
    fn test_fold_vcard_line_short() {
        assert_eq!(fold_vcard_line("FN:John Doe"), "FN:John Doe");
    }

    #[test]
    fn test_fold_vcard_line_long() {
        let long_val = "x".repeat(200);
        let line = format!("NOTE:{}", long_val);
        let folded = fold_vcard_line(&line);
        // First line should be <= 75 chars
        let first_line = folded.lines().next().unwrap();
        assert!(first_line.len() <= 75);
        // Continuation lines should start with space
        for line in folded.lines().skip(1) {
            assert!(line.starts_with(' '));
        }
    }

    #[test]
    fn test_apply_vcard_updates() {
        let vcard =
            "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Old Name\r\nEMAIL:old@example.com\r\nEND:VCARD\r\n";
        let fields = serde_json::json!({"FN": "New Name", "TEL": "+1-555-0100"});
        let updated = apply_vcard_updates(vcard, &fields);
        assert!(updated.contains("FN:New Name"));
        assert!(updated.contains("TEL:+1-555-0100"));
        // Old email should still be there
        assert!(updated.contains("EMAIL:old@example.com"));
    }

    #[test]
    fn test_vcard_generate_basic() {
        let json = r#"{"fn":"Alice Smith","email":"alice@example.com","tel":"+1-555-0300","org":"TestCorp","uid":"alice-1"}"#;
        let result = builtin_vcard_generate(&[Value::String(json.to_string())]);
        assert!(result.is_ok());
        let vcard = match result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };
        assert!(vcard.contains("BEGIN:VCARD"));
        assert!(vcard.contains("END:VCARD"));
        assert!(vcard.contains("VERSION:4.0"));
        assert!(vcard.contains("UID:alice-1"));
        assert!(vcard.contains("FN:Alice Smith"));
        assert!(vcard.contains("EMAIL:alice@example.com"));
        assert!(vcard.contains("TEL:+1-555-0300"));
        assert!(vcard.contains("ORG:TestCorp"));
    }

    #[test]
    fn test_vcard_generate_multi_email() {
        let json = r#"{"fn":"Bob","email":["bob@work.com","bob@home.com"]}"#;
        let result = builtin_vcard_generate(&[Value::String(json.to_string())]);
        assert!(result.is_ok());
        let vcard = match result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };
        assert!(vcard.contains("EMAIL:bob@work.com"));
        assert!(vcard.contains("EMAIL:bob@home.com"));
    }

    #[test]
    fn test_vcard_parse_basic() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:parse-1\r\nFN:Carol White\r\nEMAIL:carol@example.com\r\nTEL:+44-20-1234\r\nORG:British Corp\r\nTITLE:Director\r\nEND:VCARD\r\n";
        let result = builtin_vcard_parse(&[Value::String(vcard.to_string())]);
        assert!(result.is_ok());
        let json_str = match result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("should be valid JSON");
        assert_eq!(parsed["FN"], "Carol White");
        assert_eq!(parsed["UID"], "parse-1");
        assert_eq!(parsed["ORG"], "British Corp");
        assert_eq!(parsed["TITLE"], "Director");
    }

    #[test]
    fn test_vcard_roundtrip() {
        let json = r#"{"fn":"Dave Round","email":"dave@rt.com","tel":"+1-555-9999","org":"RoundCorp","uid":"dave-rt-1"}"#;
        // Generate
        let gen_result = builtin_vcard_generate(&[Value::String(json.to_string())]);
        assert!(gen_result.is_ok());
        let vcard = match gen_result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };

        // Parse
        let parse_result = builtin_vcard_parse(&[Value::String(vcard)]);
        assert!(parse_result.is_ok());
        let parsed_json = match parse_result.unwrap() {
            Value::String(s) => s,
            _ => panic!("expected string"),
        };

        let parsed: serde_json::Value =
            serde_json::from_str(&parsed_json).expect("should be valid JSON");
        assert_eq!(parsed["FN"], "Dave Round");
        assert_eq!(parsed["UID"], "dave-rt-1");
        assert_eq!(parsed["ORG"], "RoundCorp");
        let emails = parsed["EMAIL"].as_array().unwrap();
        assert_eq!(emails[0], "dave@rt.com");
    }
}
