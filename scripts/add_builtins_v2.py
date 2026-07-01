#!/usr/bin/env python3
"""
Add 15 NEW builtins to Metalogos v0.8.0 on top of existing format_date:
  Time/Date: date_parts, days_between, days_in_month, is_leap_year, add_days, add_hours, weekday_name
  Geo:      geo_ip, geo_distance
  Weather:  weather
  Remind:   remind, remind_recurring, cancel_remind, list_reminders, check_reminders
Plus: enhance existing format_date with more tokens, fix VM missing entries.
"""

BASE = "/home/z/my-project/metalogos-src/src"

with open(f"{BASE}/builtins.rs", "r") as f:
    b = f.read()

with open(f"{BASE}/compiler.rs", "r") as f:
    c = f.read()

with open(f"{BASE}/vm.rs", "r") as f:
    v = f.read()

with open("/home/z/my-project/metalogos-src/Cargo.toml", "r") as f:
    cargo = f.read()

# ================================================================
# 1. builtins.rs — Replace format_date implementation with enhanced version
# ================================================================

OLD_FORMAT_DATE = '''// ── format_date ──────────────────────────────────────────
/// `format_date(format)` — format current time.
/// `format_date(format, timestamp)` — format given unix timestamp (Float seconds).
/// Supported specifiers: %Y %m %d %H %M %S %F %T %R %Y-%m-%d %d.%m.%Y
/// Example: format_date("%Y-%m-%d %H:%M:%S") -> "2026-06-30 12:30:45"
fn builtin_format_date(args: &[Value]) -> Result<Value, String> {
    let fmt_str = if args.is_empty() {
        "%Y-%m-%d %H:%M:%S".to_string()
    } else {
        expect_string_arg("format_date", args, 0)?
    };

    let timestamp = if args.len() >= 2 {
        match &args[1] {
            Value::Float(f) => *f,
            _ => return Err(format!("format_date(): timestamp must be Float, got {}", args[1].type_name())),
        }
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    };

    let secs = timestamp as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe {
        libc::localtime_r(&secs, &mut tm);
    }

    let y = (tm.tm_year + 1900) as u32;
    let mo = (tm.tm_mon + 1) as u32;
    let d = tm.tm_mday as u32;
    let h = tm.tm_hour as u32;
    let mi = tm.tm_min as u32;
    let s = tm.tm_sec as u32;

    // Handle shorthand formats first
    let result = match fmt_str.as_str() {
        "%F" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%T" => format!("{:02}:{:02}:{:02}", h, mi, s),
        "%R" => format!("{:02}:{:02}", h, mi),
        "%Y-%m-%d" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%d.%m.%Y" => format!("{:02}.{:02}.{:04}", d, mo, y),
        _ => {
            // General format: replace %Y %m %d %H %M %S
            let mut out = fmt_str;
            out = out.replace("%Y", &format!("{:04}", y));
            out = out.replace("%m", &format!("{:02}", mo));
            out = out.replace("%d", &format!("{:02}", d));
            out = out.replace("%H", &format!("{:02}", h));
            out = out.replace("%M", &format!("{:02}", mi));
            out = out.replace("%S", &format!("{:02}", s));
            // Shorthand expansions that may appear inside longer strings
            out = out.replace("%F", &format!("{:04}-{:02}-{:02}", y, mo, d));
            out = out.replace("%T", &format!("{:02}:{:02}:{:02}", h, mi, s));
            out = out.replace("%R", &format!("{:02}:{:02}", h, mi));
            out
        }
    };
    Ok(Value::String(result))
}'''

NEW_FORMAT_DATE = '''// ── v0.8.0 — format_date (enhanced) ──────────────────────
/// Weekday names (Sunday=0 in libc tm_wday, converted to Monday=0 internally).
const WEEKDAY_NAMES: [&str; 7] = ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"];
const WEEKDAY_NAMES_MON: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Month names (1-indexed).
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `format_date(format)` — format current time.
/// `format_date(format, timestamp)` — format given unix timestamp (Float seconds).
/// Enhanced v0.8.0: supports %y %I %p %A %a %B %b %j %w %W %% in addition to %Y %m %d %H %M %S %F %T %R.
fn builtin_format_date(args: &[Value]) -> Result<Value, String> {
    let fmt_str = if args.is_empty() {
        "%Y-%m-%d %H:%M:%S".to_string()
    } else {
        expect_string_arg("format_date", args, 0)?
    };

    let timestamp = if args.len() >= 2 {
        match &args[1] {
            Value::Float(f) => *f,
            _ => return Err(format!("format_date(): timestamp must be Float, got {}", args[1].type_name())),
        }
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    };

    let secs = timestamp as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe {
        libc::localtime_r(&secs, &mut tm);
    }

    let y = (tm.tm_year + 1900) as u32;
    let mo = (tm.tm_mon + 1) as u32;
    let d = tm.tm_mday as u32;
    let h = tm.tm_hour as u32;
    let mi = tm.tm_min as u32;
    let s = tm.tm_sec as u32;
    // tm_wday: 0=Sunday, convert to 0=Monday
    let wday_mon = (tm.tm_wday as u32 + 6) % 7;
    let day_of_year = (tm.tm_yday + 1) as u32;
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;
    let ampm = if h >= 12 { "PM" } else { "AM" };
    let h12 = if h == 0 { 12 } else if h > 12 { h - 12 } else { h };

    // Handle shorthand formats first
    let result = match fmt_str.as_str() {
        "%F" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%T" => format!("{:02}:{:02}:{:02}", h, mi, s),
        "%R" => format!("{:02}:{:02}", h, mi),
        "%Y-%m-%d" => format!("{:04}-{:02}-{:02}", y, mo, d),
        "%d.%m.%Y" => format!("{:02}.{:02}.{:04}", d, mo, y),
        _ => {
            // Character-by-character for proper % handling
            let mut out = String::new();
            let mut chars = fmt_str.chars().peekable();
            while let Some(ch) = chars.next() {
                if ch == '%' {
                    match chars.next() {
                        Some('Y') => out.push_str(&format!("{:04}", y)),
                        Some('y') => out.push_str(&format!("{:02}", y % 100)),
                        Some('m') => out.push_str(&format!("{:02}", mo)),
                        Some('d') => out.push_str(&format!("{:02}", d)),
                        Some('H') => out.push_str(&format!("{:02}", h)),
                        Some('I') => out.push_str(&format!("{:02}", h12)),
                        Some('M') => out.push_str(&format!("{:02}", mi)),
                        Some('S') => out.push_str(&format!("{:02}", s)),
                        Some('p') => out.push_str(ampm),
                        Some('A') => out.push_str(WEEKDAY_NAMES_MON[wday_mon as usize]),
                        Some('a') => out.push_str(&WEEKDAY_NAMES_MON[wday_mon as usize][..3]),
                        Some('B') => out.push_str(MONTH_NAMES[(mo - 1) as usize]),
                        Some('b') => out.push_str(MONTH_ABBR[(mo - 1) as usize]),
                        Some('j') => out.push_str(&format!("{:03}", day_of_year)),
                        Some('w') => out.push_str(&format!("{}", wday_mon)),
                        Some('W') => out.push_str(&format!("{:02}", week_num)),
                        Some('%') => out.push('%'),
                        Some('F') => out.push_str(&format!("{:04}-{:02}-{:02}", y, mo, d)),
                        Some('T') => out.push_str(&format!("{:02}:{:02}:{:02}", h, mi, s)),
                        Some('R') => out.push_str(&format!("{:02}:{:02}", h, mi)),
                        Some(c) => { out.push('%'); out.push(c); }
                        None => out.push('%'),
                    }
                } else {
                    out.push(ch);
                }
            }
            out
        }
    };
    Ok(Value::String(result))
}

// ── v0.8.0 — Additional Time / Date / Calendar builtins ──────────

/// Helper: check if a Gregorian year is a leap year.
fn date_is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Helper: days in a given month (1-indexed).
fn date_days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => if date_is_leap(year) { 29 } else { 28 },
        _ => 30,
    }
}

/// Helper: make a Struct Value from key-value pairs.
fn make_date_struct(type_name: &str, pairs: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Struct { type_name: type_name.to_string(), fields: map }
}

/// `date_parts(timestamp?)` — returns struct with all date components.
/// Uses libc::localtime_r for accuracy. No arg = current time.
fn builtin_date_parts(args: &[Value]) -> Result<Value, String> {
    let ts = if args.is_empty() {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    } else {
        expect_float_arg("date_parts", args, 0)?
    };
    let secs = ts as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe { libc::localtime_r(&secs, &mut tm); }

    let y = (tm.tm_year + 1900) as u32;
    let mo = (tm.tm_mon + 1) as u32;
    let d = tm.tm_mday as u32;
    let h = tm.tm_hour as u32;
    let mi = tm.tm_min as u32;
    let s = tm.tm_sec as u32;
    let wday_mon = (tm.tm_wday as u32 + 6) % 7;
    let day_of_year = (tm.tm_yday + 1) as u32;
    let week_num = ((day_of_year as i32 + 6 - wday_mon as i32) / 7).max(1) as u32;

    Ok(make_date_struct("Date", vec![
        ("year", Value::Float(y as f64)),
        ("month", Value::Float(mo as f64)),
        ("day", Value::Float(d as f64)),
        ("hour", Value::Float(h as f64)),
        ("minute", Value::Float(mi as f64)),
        ("second", Value::Float(s as f64)),
        ("weekday", Value::Float(wday_mon as f64)),
        ("weekday_name", Value::String(WEEKDAY_NAMES_MON[wday_mon as usize].to_string())),
        ("month_name", Value::String(MONTH_NAMES[(mo - 1) as usize].to_string())),
        ("day_of_year", Value::Float(day_of_year as f64)),
        ("week_number", Value::Float(week_num as f64)),
        ("timestamp", Value::Float(ts)),
    ]))
}

/// `days_between(ts1, ts2)` — absolute difference in days between two timestamps.
fn builtin_days_between(args: &[Value]) -> Result<Value, String> {
    let ts1 = expect_float_arg("days_between", args, 0)?;
    let ts2 = expect_float_arg("days_between", args, 1)?;
    Ok(Value::Float((ts1 - ts2).abs() / 86400.0))
}

/// `days_in_month(year, month)` — number of days in given month (1-12).
fn builtin_days_in_month(args: &[Value]) -> Result<Value, String> {
    let year = expect_float_arg("days_in_month", args, 0)? as i32;
    let month = expect_float_arg("days_in_month", args, 1)? as u32;
    if month < 1 || month > 12 {
        return Err("days_in_month() month must be 1-12".to_string());
    }
    Ok(Value::Float(date_days_in_month(year, month) as f64))
}

/// `is_leap_year(year)` — returns true if year is a Gregorian leap year.
fn builtin_is_leap_year(args: &[Value]) -> Result<Value, String> {
    Ok(Value::Bool(date_is_leap(expect_float_arg("is_leap_year", args, 0)? as i32)))
}

/// `add_days(timestamp, days)` — add/subtract days to a Unix timestamp.
fn builtin_add_days(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_days", args, 0)?;
    let days = expect_float_arg("add_days", args, 1)?;
    Ok(Value::Float(ts + days * 86400.0))
}

/// `add_hours(timestamp, hours)` — add/subtract hours to a Unix timestamp.
fn builtin_add_hours(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("add_hours", args, 0)?;
    let hours = expect_float_arg("add_hours", args, 1)?;
    Ok(Value::Float(ts + hours * 3600.0))
}

/// `weekday_name(timestamp)` — returns full weekday name ("Monday".."Sunday").
fn builtin_weekday_name(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("weekday_name", args, 0)?;
    let secs = ts as i64;
    let mut tm = libc::tm {
        tm_sec: 0, tm_min: 0, tm_hour: 0,
        tm_mday: 0, tm_mon: 0, tm_year: 0,
        tm_wday: 0, tm_yday: 0, tm_isdst: 0,
        tm_gmtoff: 0, tm_zone: std::ptr::null(),
    };
    unsafe { libc::localtime_r(&secs, &mut tm); }
    let wday_mon = (tm.tm_wday as u32 + 6) % 7;
    Ok(Value::String(WEEKDAY_NAMES_MON[wday_mon as usize].to_string()))
}

// ── v0.8.0 — Geolocation builtins ───────────────────────────────────

/// `geo_ip(ip?)` — geolocate by IP address. Uses ip-api.com (free, no API key).
/// If no IP given, geolocates the caller's IP.
/// Returns Struct {ip, city, region, country, country_code, lat, lon, isp, timezone}.
fn builtin_geo_ip(args: &[Value]) -> Result<Value, String> {
    let ip = match args.get(0) {
        Some(Value::String(s)) if !s.is_empty() => s.clone(),
        _ => String::new(),
    };
    let url = if ip.is_empty() {
        "http://ip-api.com/json/".to_string()
    } else {
        format!("http://ip-api.com/json/{}", ip)
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("geo_ip() client error: {}", e))?;

    let resp = client.get(&url).send()
        .map_err(|e| format!("geo_ip() request failed: {}", e))?;
    let body = resp.text().unwrap_or_default();

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("geo_ip() failed to parse response: {}", e))?;

    if json.get("status").and_then(|v| v.as_str()) != Some("success") {
        let msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("unknown error");
        return Err(format!("geo_ip() API error: {}", msg));
    }

    let g = |key: &str| -> Value {
        json.get(key).map(|v| match v {
            serde_json::Value::String(s) => Value::String(s.clone()),
            serde_json::Value::Number(n) => Value::Float(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::Null => Value::String(String::new()),
            _ => Value::String(v.to_string()),
        }).unwrap_or(Value::String(String::new()))
    };

    Ok(make_date_struct("GeoLocation", vec![
        ("ip", g("query")),
        ("city", g("city")),
        ("region", g("regionName")),
        ("country", g("country")),
        ("country_code", g("countryCode")),
        ("lat", g("lat")),
        ("lon", g("lon")),
        ("isp", g("isp")),
        ("timezone", g("timezone")),
    ]))
}

/// `geo_distance(lat1, lon1, lat2, lon2, unit?)` — haversine distance.
/// unit: "km" (default), "mi", "nm", "m".
fn builtin_geo_distance(args: &[Value]) -> Result<Value, String> {
    let lat1 = expect_float_arg("geo_distance", args, 0)?;
    let lon1 = expect_float_arg("geo_distance", args, 1)?;
    let lat2 = expect_float_arg("geo_distance", args, 2)?;
    let lon2 = expect_float_arg("geo_distance", args, 3)?;
    let unit = match args.get(4) {
        Some(Value::String(s)) => s.as_str(),
        _ => "km",
    };

    let to_rad = |deg: f64| deg * std::f64::consts::PI / 180.0;
    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin() * (dlat / 2.0).sin()
        + to_rad(lat1).cos() * to_rad(lat2).cos()
        * (dlon / 2.0).sin() * (dlon / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = 6371.0 * c;

    let result = match unit {
        "mi" => km * 0.621371,
        "nm" => km * 0.539957,
        "m" => km * 1000.0,
        _ => km,
    };
    Ok(Value::Float(result))
}

// ── v0.8.0 — Weather builtins ───────────────────────────────────────

/// `weather(city_or_lat, lon?)` — get current weather.
/// Usage: weather("London") or weather(51.5, -0.12)
/// Requires OPENWEATHER_API_KEY env var.
/// Returns Struct {temp, feels_like, temp_min, temp_max, pressure, humidity,
///                  description, icon, wind_speed, city, country, clouds, visibility}.
fn builtin_weather(args: &[Value]) -> Result<Value, String> {
    let api_key = std::env::var("OPENWEATHER_API_KEY")
        .map_err(|_| "weather() requires OPENWEATHER_API_KEY environment variable".to_string())?;

    let url = if args.len() >= 2 {
        let lat = expect_float_arg("weather", args, 0)?;
        let lon = expect_float_arg("weather", args, 1)?;
        format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
            lat, lon, api_key
        )
    } else {
        let city = expect_string_arg("weather", args, 0)?;
        format!(
            "https://api.openweathermap.org/data/2.5/weather?q={}&appid={}&units=metric",
            urlencoding::encode(&city), api_key
        )
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("weather() client error: {}", e))?;

    let resp = client.get(&url).send()
        .map_err(|e| format!("weather() request failed: {}", e))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();

    if status >= 400 {
        return Err(format!("weather() API error {}: {}", status, body));
    }

    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("weather() parse error: {}", e))?;

    let jf = |path: &str, j: &serde_json::Value| -> f64 {
        path.split('.').fold(j.clone(), |a, k| {
            a.get(k).cloned().unwrap_or(serde_json::Value::Null)
        }).as_f64().unwrap_or(0.0)
    };
    let js = |path: &str, j: &serde_json::Value| -> String {
        path.split('.').fold(j.clone(), |a, k| {
            a.get(k).cloned().unwrap_or(serde_json::Value::Null)
        }).as_str().unwrap_or("").to_string()
    };

    Ok(make_date_struct("Weather", vec![
        ("temp", Value::Float(jf("main.temp", &json))),
        ("feels_like", Value::Float(jf("main.feels_like", &json))),
        ("temp_min", Value::Float(jf("main.temp_min", &json))),
        ("temp_max", Value::Float(jf("main.temp_max", &json))),
        ("pressure", Value::Float(jf("main.pressure", &json))),
        ("humidity", Value::Float(jf("main.humidity", &json))),
        ("description", Value::String(js("weather.0.description", &json))),
        ("icon", Value::String(js("weather.0.icon", &json))),
        ("wind_speed", Value::Float(jf("wind.speed", &json))),
        ("city", Value::String(js("name", &json))),
        ("country", Value::String(js("sys.country", &json))),
        ("clouds", Value::Float(jf("clouds.all", &json))),
        ("visibility", Value::Float(jf("visibility", &json))),
    ]))
}

// ── v0.8.0 — Reminders builtins ─────────────────────────────────────

/// Reminder entry stored in the global reminder list.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ReminderEntry {
    id: String,
    message: String,
    fire_at: f64,
    interval: f64,        // 0 = one-shot, >0 = recurring (seconds)
    next_fire: f64,
    data: String,
    active: bool,
    created_at: f64,
}

/// Global reminder store.
static REMINDERS: std::sync::OnceLock<StdMutex<Vec<ReminderEntry>>> = std::sync::OnceLock::new();

fn reminders_store() -> &'static StdMutex<Vec<ReminderEntry>> {
    REMINDERS.get_or_init(|| StdMutex::new(Vec::new()))
}

/// `remind(message, timestamp, data?)` — create a one-time reminder. Returns ID.
fn builtin_remind(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind", args, 0)?;
    let fire_at = expect_float_arg("remind", args, 1)?;
    let data = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => String::new(),
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store().lock()
        .map_err(|e| format!("remind() lock error: {}", e))?;
    store.push(ReminderEntry {
        id: id.clone(), message, fire_at, interval: 0.0,
        next_fire: fire_at, data, active: true, created_at: now_ts,
    });
    Ok(Value::String(id))
}

/// `remind_recurring(message, interval_seconds, data?)` — create recurring reminder. Returns ID.
fn builtin_remind_recurring(args: &[Value]) -> Result<Value, String> {
    let message = expect_string_arg("remind_recurring", args, 0)?;
    let interval = expect_float_arg("remind_recurring", args, 1)?;
    if interval <= 0.0 {
        return Err("remind_recurring() interval must be positive".to_string());
    }
    let data = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        Some(other) => format!("{}", other),
        None => String::new(),
    };
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let mut store = reminders_store().lock()
        .map_err(|e| format!("remind_recurring() lock error: {}", e))?;
    store.push(ReminderEntry {
        id: id.clone(), message, fire_at: now_ts, interval,
        next_fire: now_ts + interval, data, active: true, created_at: now_ts,
    });
    Ok(Value::String(id))
}

/// `cancel_remind(id)` — cancel an active reminder. Returns "ok" or "not_found".
fn builtin_cancel_remind(args: &[Value]) -> Result<Value, String> {
    let id = expect_string_arg("cancel_remind", args, 0)?;
    let mut store = reminders_store().lock()
        .map_err(|e| format!("cancel_remind() lock error: {}", e))?;
    for entry in store.iter_mut() {
        if entry.id == id && entry.active {
            entry.active = false;
            return Ok(Value::String("ok".to_string()));
        }
    }
    Ok(Value::String("not_found".to_string()))
}

/// `list_reminders()` — list all active reminders as list of Structs.
fn builtin_list_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = reminders_store().lock()
        .map_err(|e| format!("list_reminders() lock error: {}", e))?;
    let mut result = Vec::new();
    for entry in store.iter().filter(|r| r.active) {
        let rtype = if entry.interval > 0.0 { "recurring" } else { "once" };
        let ec = entry.clone();
        result.push(make_date_struct("Reminder", vec![
            ("id", Value::String(ec.id)),
            ("message", Value::String(ec.message)),
            ("fire_at", Value::Float(ec.fire_at)),
            ("interval", Value::Float(ec.interval)),
            ("next_fire", Value::Float(ec.next_fire)),
            ("data", Value::String(ec.data)),
            ("created_at", Value::Float(ec.created_at)),
            ("type", Value::String(rtype.to_string())),
        ]));
    }
    Ok(Value::List(result))
}

/// `check_reminders()` — get due reminders. One-shot deactivated; recurring advanced.
fn builtin_check_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0);

    let mut store = reminders_store().lock()
        .map_err(|e| format!("check_reminders() lock error: {}", e))?;
    let mut due = Vec::new();
    for entry in store.iter_mut() {
        if !entry.active { continue; }
        if now_ts >= entry.next_fire {
            let rtype = if entry.interval > 0.0 { "recurring" } else { "once" };
            let overdue = now_ts - entry.next_fire;
            due.push(make_date_struct("DueReminder", vec![
                ("id", Value::String(entry.id.clone())),
                ("message", Value::String(entry.message.clone())),
                ("data", Value::String(entry.data.clone())),
                ("type", Value::String(rtype.to_string())),
                ("next_fire", Value::Float(entry.next_fire)),
                ("overdue_seconds", Value::Float(overdue)),
            ]));
            if entry.interval > 0.0 {
                entry.next_fire += entry.interval;
            } else {
                entry.active = false;
            }
        }
    }
    Ok(Value::List(due))
}'''

b = b.replace(OLD_FORMAT_DATE, NEW_FORMAT_DATE)

# ================================================================
# 2. builtins.rs — Insert new registrations in Builtins::new()
# ================================================================

old_reg = '''        // format_date() — format unix timestamp or current time
        funcs.insert("format_date".to_string(), builtin_format_date as BuiltinFn);
        // request_body() alias for json_body() — common in web frameworks'''

new_reg = '''        // format_date() — format unix timestamp or current time (enhanced v0.8.0)
        funcs.insert("format_date".to_string(), builtin_format_date as BuiltinFn);
        // v0.8.0 — Time / Date / Calendar (additional)
        funcs.insert("date_parts".to_string(), builtin_date_parts as BuiltinFn);
        funcs.insert("days_between".to_string(), builtin_days_between as BuiltinFn);
        funcs.insert("days_in_month".to_string(), builtin_days_in_month as BuiltinFn);
        funcs.insert("is_leap_year".to_string(), builtin_is_leap_year as BuiltinFn);
        funcs.insert("add_days".to_string(), builtin_add_days as BuiltinFn);
        funcs.insert("add_hours".to_string(), builtin_add_hours as BuiltinFn);
        funcs.insert("weekday_name".to_string(), builtin_weekday_name as BuiltinFn);
        // v0.8.0 — Geolocation
        funcs.insert("geo_ip".to_string(), builtin_geo_ip as BuiltinFn);
        funcs.insert("geo_distance".to_string(), builtin_geo_distance as BuiltinFn);
        // v0.8.0 — Weather
        funcs.insert("weather".to_string(), builtin_weather as BuiltinFn);
        // v0.8.0 — Reminders
        funcs.insert("remind".to_string(), builtin_remind as BuiltinFn);
        funcs.insert("remind_recurring".to_string(), builtin_remind_recurring as BuiltinFn);
        funcs.insert("cancel_remind".to_string(), builtin_cancel_remind as BuiltinFn);
        funcs.insert("list_reminders".to_string(), builtin_list_reminders as BuiltinFn);
        funcs.insert("check_reminders".to_string(), builtin_check_reminders as BuiltinFn);
        // request_body() alias for json_body() — common in web frameworks'''

b = b.replace(old_reg, new_reg)

with open(f"{BASE}/builtins.rs", "w") as f:
    f.write(b)

# ================================================================
# 3. compiler.rs — Add 15 new builtin names (format_date already exists)
# ================================================================

old_comp = '''            // Format
            "format",
            // Memory (recall)
            "recall","''

new_comp = '''            // Format
            "format",
            // v0.8.0 — Time / Date / Calendar
            "date_parts", "days_between", "days_in_month",
            "is_leap_year", "add_days", "add_hours", "weekday_name",
            // v0.8.0 — Geolocation
            "geo_ip", "geo_distance",
            // v0.8.0 — Weather
            "weather",
            // v0.8.0 — Reminders
            "remind", "remind_recurring", "cancel_remind", "list_reminders", "check_reminders",
            // Memory (recall)
            "recall","''

c = c.replace(old_comp, new_comp)

with open(f"{BASE}/compiler.rs", "w") as f:
    f.write(c)

# ================================================================
# 4. vm.rs — Fix missing entries + add 15 new builtin names
# ================================================================

old_vm = '''            // Misc
            "dict_get".to_string(),     // 91
            "type_of".to_string(),      // 92
            // Memory (recall)
            "recall".to_string(),       // 93'''

new_vm = '''            // Misc
            "dict_get".to_string(),     // 91
            "dict_set".to_string(),     // 92
            "dict_keys".to_string(),    // 93
            "dict_values".to_string(),  // 94
            "dict_has".to_string(),     // 95
            "type_of".to_string(),      // 96
            // Format
            "format".to_string(),       // 97
            // v0.8.0 — Time / Date / Calendar
            "date_parts".to_string(),   // 98
            "days_between".to_string(), // 99
            "days_in_month".to_string(), // 100
            "is_leap_year".to_string(), // 101
            "add_days".to_string(),     // 102
            "add_hours".to_string(),    // 103
            "weekday_name".to_string(), // 104
            // v0.8.0 — Geolocation
            "geo_ip".to_string(),       // 105
            "geo_distance".to_string(), // 106
            // v0.8.0 — Weather
            "weather".to_string(),      // 107
            // v0.8.0 — Reminders
            "remind".to_string(),       // 108
            "remind_recurring".to_string(), // 109
            "cancel_remind".to_string(),    // 110
            "list_reminders".to_string(),   // 111
            "check_reminders".to_string(),  // 112
            // Memory (recall)
            "recall".to_string(),       // 113'''

v = v.replace(old_vm, new_vm)

with open(f"{BASE}/vm.rs", "w") as f:
    f.write(v)

# ================================================================
# 5. Cargo.toml — Bump version to 0.8.0
# ================================================================

cargo = cargo.replace('version = "0.7.11"', 'version = "0.8.0"')

with open("/home/z/my-project/metalogos-src/Cargo.toml", "w") as f:
    f.write(cargo)

print("OK: All files modified on top of origin/main (a61c478)")