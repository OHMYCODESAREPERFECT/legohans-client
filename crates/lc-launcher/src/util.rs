//! Small shared helpers: ids, time, text, base64, opening folders.

use std::io;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// Seconds since the Unix epoch.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Create a process-unique identifier such as `profile-18f3a2b-0`.
pub fn new_id(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{nanos:x}-{n:x}")
}

/// Describe `then` relative to `now`, e.g. "5 minutes ago".
pub fn relative_time(now: u64, then: u64) -> String {
    fn plural(n: u64, unit: &str) -> String {
        if n == 1 {
            format!("1 {unit} ago")
        } else {
            format!("{n} {unit}s ago")
        }
    }
    let d = now.saturating_sub(then);
    match d {
        0..=59 => "just now".to_string(),
        60..=3599 => plural(d / 60, "minute"),
        3600..=86_399 => plural(d / 3600, "hour"),
        86_400..=604_799 => plural(d / 86_400, "day"),
        604_800..=5_183_999 => plural(d / 604_800, "week"),
        _ => plural(d / 2_592_000, "month"),
    }
}

/// Format a byte count for display.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// 64-bit FNV-1a hash, used for stable pseudo-random colors.
pub fn hash_str(s: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Remove Minecraft `§x` formatting codes.
pub fn strip_formatting(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// Flatten a Minecraft JSON text component (string, object or array) to plain text.
pub fn component_text(value: &Value) -> String {
    match value {
        Value::String(s) => strip_formatting(s),
        Value::Array(items) => items.iter().map(component_text).collect(),
        Value::Object(map) => {
            let mut out = String::new();
            if let Some(Value::String(text)) = map.get("text") {
                out.push_str(&strip_formatting(text));
            }
            if let Some(extra) = map.get("extra") {
                out.push_str(&component_text(extra));
            }
            out
        }
        _ => String::new(),
    }
}

/// Decode standard base64 (padding optional, whitespace ignored).
pub fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for c in input.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(v);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// The program used to open a folder in the system file manager.
pub fn open_folder_program() -> &'static str {
    if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    }
}

/// Open `path` in the system file manager, creating it first if needed.
pub fn open_folder(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)?;
    let mut child = Command::new(open_folder_program()).arg(path).spawn()?;
    // Reap the child so it does not linger as a zombie process.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_prefixed() {
        let a = new_id("p");
        let b = new_id("p");
        assert!(a.starts_with("p-"));
        assert_ne!(a, b);
    }

    #[test]
    fn relative_time_buckets() {
        assert_eq!(relative_time(100, 100), "just now");
        assert_eq!(relative_time(100, 200), "just now");
        assert_eq!(relative_time(1000, 1000 - 60), "1 minute ago");
        assert_eq!(relative_time(10_000, 10_000 - 7200), "2 hours ago");
        assert_eq!(
            relative_time(1_000_000, 1_000_000 - 86_400 * 3),
            "3 days ago"
        );
        assert_eq!(
            relative_time(10_000_000, 10_000_000 - 604_800 * 2),
            "2 weeks ago"
        );
        assert_eq!(relative_time(100_000_000, 0), "38 months ago");
    }

    #[test]
    fn bytes_are_formatted() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(2048), "2.0 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn hash_is_stable() {
        assert_eq!(hash_str("abc"), hash_str("abc"));
        assert_ne!(hash_str("abc"), hash_str("abd"));
    }

    #[test]
    fn formatting_codes_are_stripped() {
        assert_eq!(strip_formatting("§aHello §lWorld"), "Hello World");
    }

    #[test]
    fn components_are_flattened() {
        let v: Value = serde_json::from_str(
            r#"{"text":"A ","extra":[{"text":"B"},"C",{"text":"D","extra":["E"]}]}"#,
        )
        .unwrap();
        assert_eq!(component_text(&v), "A BCDE");
        assert_eq!(component_text(&Value::String("§cRed".into())), "Red");
    }

    #[test]
    fn base64_decodes() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("aGVs\nbG8").unwrap(), b"hello");
        assert!(base64_decode("a$b").is_none());
    }

    #[test]
    fn folder_program_is_known() {
        assert!(["explorer", "open", "xdg-open"].contains(&open_folder_program()));
    }
}
