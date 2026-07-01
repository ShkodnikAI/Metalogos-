#!/usr/bin/env python3
"""
Add 16 new builtins to Metalogos v0.8.0:
  Time/Date: format_date, date_parts, days_between, days_in_month, is_leap_year, add_days, add_hours, weekday_name
  Geo:      geo_ip, geo_distance
  Weather:  weather
  Remind:   remind, remind_recurring, cancel_remind, list_reminders, check_reminders
"""

import re

BASE = "/home/z/my-project/metalogos-src/src"

# Read files
with open(f"{BASE}/builtins.rs", "r") as f:
    builtins_content = f.read()

with open(f"{BASE}/compiler.rs", "r") as f:
    compiler_content = f.read()

with open(f"{BASE}/vm.rs", "r") as f:
    vm_content = f.read()

with open("/home/z/my-project/metalogos-src/Cargo.toml", "r") as f:
    cargo_content = f.read()

# ============================================================
# 1. builtins.rs — Append new implementations
# ============================================================

NEW_IMPLS = r'''
// ── v0.8.0 — Time / Date / Calendar builtins ────────────────────────

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

/// Helper: convert Unix timestamp (seconds as f64) to Gregorian components.
/// Returns: (year, month, day, hour, minute, second, weekday, day_of_year)
/// weekday: 0=Monday .. 6=Sunday
fn timestamp_to_parts(ts: f64) -> (i32, u32, u32, u32, u32, u32, u32, u32) {
    let total_secs = ts as i64;
    let days_i = total_secs / 86400;
    let time_of_day = ((total_secs % 86400) + 86400) % 86400;
    let hour = (time_of_day / 3600) as u32;
    let minute = ((time_of_day % 3600) / 60) as u32;
    let second = (time_of_day % 60) as u32;

    // Iterative year resolution
    let mut year = 1970i32;
    let mut rem_days = days_i;
    if rem_days < 0 {
        year -= 1;
        loop {
            let diy = if date_is_leap(year) { 366i64 } else { 365i64 };
            if rem_days + diy >= 0 { rem_days += diy; break; }
            rem_days += diy;
            year -= 1;
        }
    } else {
        loop {
            let diy = if date_is_leap(year) { 366i64 } else { 365i64 };
            if rem_days < diy { break; }
            rem_days -= diy;
            year += 1;
        }
    }
    let day_of_year = (rem_days + 1) as u32;

    let mut month = 1u32;
    let mut day = 1u32;
    let mut r = rem_days as u32;
    for m in 1u32..=12 {
        let dim = date_days_in_month(year, m);
        if r < dim { month = m; day = r + 1; break; }
        r -= dim;
    }

    // Weekday: Jan 1 1970 = Thursday. Monday=0.
    // (days_since_epoch + 3) % 7
    let weekday = (((days_i % 7) + 7 + 3) % 7) as u32;

    (year, month, day, hour, minute, second, weekday, day_of_year)
}

/// Helper: make a Struct Value from key-value pairs.
fn make_date_struct(type_name: &str, pairs: Vec<(&str, Value)>) -> Value {
    let mut map = std::collections::HashMap::new();
    for (k, v) in pairs {
        map.insert(k.to_string(), v);
    }
    Value::Struct { type_name: type_name.to_string(), fields: map }
}

/// Weekday names (Monday=0).
const WEEKDAY_NAMES: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Month names (1-indexed).
const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June",
    "July", "August", "September", "October", "November", "December",
];

/// Abbreviated month names.
const MONTH_ABBR: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `format_date(timestamp, format?)` — format Unix timestamp to string.
/// Default format: "%Y-%m-%d %H:%M:%S"
/// Supported tokens: %Y %y %m %d %H %I %M %S %p %A %a %B %b %j %w %W %%
fn builtin_format_date(args: &[Value]) -> Result<Value, String> {
    let ts = expect_float_arg("format_date", args, 0)?;
    let fmt = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => "%Y-%m-%d %H:%M:%S".to_string(),
    };
    let (year, month, day, hour, minute, second, weekday, day_of_year) = timestamp_to_parts(ts);
    let (year, month, day, hour, minute, second, weekday, day_of_year) =
        (year as u32, month, day, hour, minute, second, weekday, day_of_year);

    // ISO week number (simplified)
    let week_num = ((day_of_year as i32 + 6 - weekday as i32) / 7).max(1) as u32;

    let ampm = if hour >= 12 { "PM" } else { "AM" };
    let hour12 = if hour == 0 { 12 } else if hour > 12 { hour - 12 } else { hour };

    let mut result = String::new();
    let mut chars = fmt.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            match chars.next() {
                Some('Y') => result.push_str(&format!("{:04}", year)),
                Some('y') => result.push_str(&format!("{:02}", year % 100)),
                Some('m') => result.push_str(&format!("{:02}", month)),
                Some('d') => result.push_str(&format!("{:02}", day)),
                Some('H') => result.push_str(&format!("{:02}", hour)),
                Some('I') => result.push_str(&format!("{:02}", hour12)),
                Some('M') => result.push_str(&format!("{:02}", minute)),
                Some('S') => result.push_str(&format!("{:02}", second)),
                Some('p') => result.push_str(ampm),
                Some('A') => result.push_str(WEEKDAY_NAMES[weekday as usize]),
                Some('a') => result.push_str(&WEEKDAY_NAMES[weekday as usize][..3]),
                Some('B') => result.push_str(MONTH_NAMES[(month - 1) as usize]),
                Some('b') => result.push_str(MONTH_ABBR[(month - 1) as usize]),
                Some('j') => result.push_str(&format!("{:03}", day_of_year)),
                Some('w') => result.push_str(&format!("{}", weekday)),
                Some('W') => result.push_str(&format!("{:02}", week_num)),
                Some('%') => result.push('%'),
                Some(c) => { result.push('%'); result.push(c); }
                None => result.push('%'),
            }
        } else {
            result.push(ch);
        }
    }
    Ok(Value::String(result))
}

/// `date_parts(timestamp)` — returns struct with all date components.
/// {year, month, day, hour, minute, second, weekday, weekday_name, month_name, day_of_year, week_number, timestamp}
/// If no argument, uses current time.
fn builtin_date_parts(args: &[Value]) -> Result<Value, String> {
    let ts = if args.is_empty() {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    } else {
        expect_float_arg("date_parts", args, 0)?
    };
    let (year, month, day, hour, minute, second, weekday, day_of_year) = timestamp_to_parts(ts);
    let week_num = ((day_of_year as i32 + 6 - weekday as i32) / 7).max(1) as u32;

    Ok(make_date_struct("Date", vec![
        ("year", Value::Float(year as f64)),
        ("month", Value::Float(month as f64)),
        ("day", Value::Float(day as f64)),
        ("hour", Value::Float(hour as f64)),
        ("minute", Value::Float(minute as f64)),
        ("second", Value::Float(second as f64)),
        ("weekday", Value::Float(weekday as f64)),
        ("weekday_name", Value::String(WEEKDAY_NAMES[weekday as usize].to_string())),
        ("month_name", Value::String(MONTH_NAMES[(month - 1) as usize].to_string())),
        ("day_of_year", Value::Float(day_of_year as f64)),
        ("week_number", Value::Float(week_num as f64)),
        ("timestamp", Value::Float(ts)),
    ]))
}

/// `days_between(ts1, ts2)` — absolute difference in days between two timestamps.
fn builtin_days_between(args: &[Value]) -> Result<Value, String> {
    let ts1 = expect_float_arg("days_between", args, 0)?;
    let ts2 = expect_float_arg("days_between", args, 1)?;
    let diff = (ts1 - ts2).abs() / 86400.0;
    Ok(Value::Float(diff))
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
    let year = expect_float_arg("is_leap_year", args, 0)? as i32;
    Ok(Value::Bool(date_is_leap(year)))
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
    let (_, _, _, _, _, _, weekday, _) = timestamp_to_parts(ts);
    Ok(Value::String(WEEKDAY_NAMES[weekday as usize].to_string()))
}

// ── v0.8.0 — Geolocation builtins ───────────────────────────────────

/// `geo_ip(ip?)` — geolocate by IP address. Uses ip-api.com (free, no key).
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

/// `geo_distance(lat1, lon1, lat2, lon2, unit?)` — haversine distance between two coordinates.
/// unit: "km" (default), "mi" (miles), "nm" (nautical miles), "m" (meters).
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
    let r = 6371.0; // Earth radius in km

    let dlat = to_rad(lat2 - lat1);
    let dlon = to_rad(lon2 - lon1);
    let a = (dlat / 2.0).sin() * (dlat / 2.0).sin()
        + to_rad(lat1).cos() * to_rad(lat2).cos()
        * (dlon / 2.0).sin() * (dlon / 2.0).sin();
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    let km = r * c;

    let result = match unit {
        "mi" => km * 0.621371,
        "nm" => km * 0.539957,
        "m" => km * 1000.0,
        _ => km, // default km
    };

    Ok(Value::Float(result))
}

// ── v0.8.0 — Weather builtins ───────────────────────────────────────

/// `weather(city_or_coords...)` — get current weather.
/// Usage 1: weather("London")       — by city name
/// Usage 2: weather(51.5, -0.12)    — by latitude, longitude
/// Requires env var OPENWEATHER_API_KEY.
/// Returns Struct {temp, feels_like, temp_min, temp_max, pressure, humidity,
///                  description, icon, wind_speed, city, country, clouds, visibility}.
fn builtin_weather(args: &[Value]) -> Result<Value, String> {
    let api_key = std::env::var("OPENWEATHER_API_KEY")
        .map_err(|_| "weather() requires OPENWEATHER_API_KEY environment variable".to_string())?;

    let url = if args.len() >= 2 {
        // Two floats -> coordinates
        let lat = expect_float_arg("weather", args, 0)?;
        let lon = expect_float_arg("weather", args, 1)?;
        format!(
            "https://api.openweathermap.org/data/2.5/weather?lat={}&lon={}&appid={}&units=metric",
            lat, lon, api_key
        )
    } else {
        // String -> city name
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

    let f = |path: &str, json: &serde_json::Value| -> f64 {
        path.split('.').fold(json.clone(), |acc, key| {
            acc.get(key).cloned().unwrap_or(serde_json::Value::Null)
        }).as_f64().unwrap_or(0.0)
    };
    let s = |path: &str, json: &serde_json::Value| -> String {
        path.split('.').fold(json.clone(), |acc, key| {
            acc.get(key).cloned().unwrap_or(serde_json::Value::Null)
        }).as_str().unwrap_or("").to_string()
    };

    Ok(make_date_struct("Weather", vec![
        ("temp", Value::Float(f("main.temp", &json))),
        ("feels_like", Value::Float(f("main.feels_like", &json))),
        ("temp_min", Value::Float(f("main.temp_min", &json))),
        ("temp_max", Value::Float(f("main.temp_max", &json))),
        ("pressure", Value::Float(f("main.pressure", &json))),
        ("humidity", Value::Float(f("main.humidity", &json))),
        ("description", Value::String(s("weather.0.description", &json))),
        ("icon", Value::String(s("weather.0.icon", &json))),
        ("wind_speed", Value::Float(f("wind.speed", &json))),
        ("city", Value::String(s("name", &json))),
        ("country", Value::String(s("sys.country", &json))),
        ("clouds", Value::Float(f("clouds.all", &json))),
        ("visibility", Value::Float(f("visibility", &json))),
    ]))
}

// ── v0.8.0 — Reminders builtins ─────────────────────────────────────

use std::sync::Mutex as StdMutex;

/// Reminder entry stored in the global reminder list.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct ReminderEntry {
    id: String,
    message: String,
    fire_at: f64,         // Unix timestamp when reminder should fire
    interval: f64,        // 0 = one-shot, >0 = recurring interval in seconds
    next_fire: f64,       // Next fire time (for recurring)
    data: String,         // Optional user data (JSON string)
    active: bool,         // Whether reminder is still active
    created_at: f64,      // When reminder was created
}

/// Global reminder store.
static REMINDERS: std::sync::OnceLock<StdMutex<Vec<ReminderEntry>>> = std::sync::OnceLock::new();

fn reminders_store() -> &'static StdMutex<Vec<ReminderEntry>> {
    REMINDERS.get_or_init(|| StdMutex::new(Vec::new()))
}

/// `remind(message, timestamp, data?)` — create a one-time reminder.
/// Returns the reminder ID (string).
/// `timestamp` is a Unix timestamp (seconds) when the reminder should fire.
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
    let entry = ReminderEntry {
        id: id.clone(),
        message,
        fire_at,
        interval: 0.0,
        next_fire: fire_at,
        data,
        active: true,
        created_at: now_ts,
    };

    let mut store = reminders_store().lock()
        .map_err(|e| format!("remind() lock error: {}", e))?;
    store.push(entry);
    Ok(Value::String(id))
}

/// `remind_recurring(message, interval_seconds, data?)` — create a recurring reminder.
/// `interval_seconds` is the interval between firings (e.g. 86400 for daily, 604800 for weekly).
/// Returns the reminder ID (string).
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
    let next_fire = now_ts + interval;
    let entry = ReminderEntry {
        id: id.clone(),
        message,
        fire_at: now_ts,
        interval,
        next_fire,
        data,
        active: true,
        created_at: now_ts,
    };

    let mut store = reminders_store().lock()
        .map_err(|e| format!("remind_recurring() lock error: {}", e))?;
    store.push(entry);
    Ok(Value::String(id))
}

/// `cancel_remind(id)` — cancel an active reminder by its ID.
/// Returns "ok" if found and cancelled, "not_found" otherwise.
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

/// `list_reminders()` — list all active reminders as a list of Structs.
/// Each struct: {id, message, fire_at, interval, next_fire, data, created_at, type}
/// type is "once" or "recurring".
fn builtin_list_reminders(args: &[Value]) -> Result<Value, String> {
    let _ = args;
    let store = reminders_store().lock()
        .map_err(|e| format!("list_reminders() lock error: {}", e))?;

    let mut result = Vec::new();
    for entry in store.iter().filter(|r| r.active) {
        let remind_type = if entry.interval > 0.0 { "recurring" } else { "once" };
        let ec = entry.clone();
        result.push(make_date_struct("Reminder", vec![
            ("id", Value::String(ec.id)),
            ("message", Value::String(ec.message)),
            ("fire_at", Value::Float(ec.fire_at)),
            ("interval", Value::Float(ec.interval)),
            ("next_fire", Value::Float(ec.next_fire)),
            ("data", Value::String(ec.data)),
            ("created_at", Value::Float(ec.created_at)),
            ("type", Value::String(remind_type.to_string())),
        ]));
    }
    Ok(Value::List(result))
}

/// `check_reminders()` — check for due reminders, return them as a list of Structs.
/// One-shot reminders are marked as inactive after being returned.
/// Recurring reminders have their next_fire advanced by interval.
/// Returns list of {id, message, data, type, next_fire, overdue_seconds}.
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
            let remind_type = if entry.interval > 0.0 { "recurring" } else { "once" };
            let overdue = now_ts - entry.next_fire;
            due.push(make_date_struct("DueReminder", vec![
                ("id", Value::String(entry.id.clone())),
                ("message", Value::String(entry.message.clone())),
                ("data", Value::String(entry.data.clone())),
                ("type", Value::String(remind_type.to_string())),
                ("next_fire", Value::Float(entry.next_fire)),
                ("overdue_seconds", Value::Float(overdue)),
            ]));

            if entry.interval > 0.0 {
                // Recurring: advance next_fire
                entry.next_fire += entry.interval;
            } else {
                // One-shot: deactivate
                entry.active = false;
            }
        }
    }
    Ok(Value::List(due))
}
'''

# Append implementations
with open(f"{BASE}/builtins.rs", "w") as f:
    f.write(builtins_content)
    f.write(NEW_IMPLS)

# ============================================================
# 2. builtins.rs — Insert registrations in Builtins::new()
# ============================================================

with open(f"{BASE}/builtins.rs", "r") as f:
    builtins_content = f.read()

reg_code = """        // v0.8.0 — Time / Date / Calendar
        funcs.insert("format_date".to_string(), builtin_format_date as BuiltinFn);
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

"""

builtins_content = builtins_content.replace(
    "        Builtins { funcs }",
    reg_code + "        Builtins { funcs }"
)

with open(f"{BASE}/builtins.rs", "w") as f:
    f.write(builtins_content)

# ============================================================
# 3. compiler.rs — Add new builtin names
# ============================================================

old_compiler = """            // Format
            "format",
            // Memory (recall)
            "recall","""

new_compiler = """            // Format
            "format",
            // v0.8.0 — Time / Date / Calendar
            "format_date", "date_parts", "days_between", "days_in_month",
            "is_leap_year", "add_days", "add_hours", "weekday_name",
            // v0.8.0 — Geolocation
            "geo_ip", "geo_distance",
            // v0.8.0 — Weather
            "weather",
            // v0.8.0 — Reminders
            "remind", "remind_recurring", "cancel_remind", "list_reminders", "check_reminders",
            // Memory (recall)
            "recall","""

compiler_content = compiler_content.replace(old_compiler, new_compiler)

with open(f"{BASE}/compiler.rs", "w") as f:
    f.write(compiler_content)

# ============================================================
# 4. vm.rs — Fix missing entries + add new builtin names
# ============================================================

old_vm = """            // Misc
            "dict_get".to_string(),     // 90
            "type_of".to_string(),      // 91
            // Memory (recall)
            "recall".to_string(),       // 92"""

new_vm = """            // Misc
            "dict_get".to_string(),     // 90
            "dict_set".to_string(),     // 91
            "dict_keys".to_string(),    // 92
            "dict_values".to_string(),  // 93
            "dict_has".to_string(),     // 94
            "type_of".to_string(),      // 95
            // Format
            "format".to_string(),       // 96
            // v0.8.0 — Time / Date / Calendar
            "format_date".to_string(),  // 97
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
            "recall".to_string(),       // 113"""

vm_content = vm_content.replace(old_vm, new_vm)

with open(f"{BASE}/vm.rs", "w") as f:
    f.write(vm_content)

# ============================================================
# 5. Cargo.toml — Bump version
# ============================================================

cargo_content = cargo_content.replace(
    'version = "0.7.10"',
    'version = "0.8.0"'
)

with open("/home/z/my-project/metalogos-src/Cargo.toml", "w") as f:
    f.write(cargo_content)

print("OK: All files modified successfully")