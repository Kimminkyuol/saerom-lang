use std::cell::UnsafeCell;

use crate::value::{Value, BOOL, FLOAT, INT, STR, TABLE};

pub fn to_text(value: &Value) -> String {
    let mut out = String::new();
    write_text(&mut out, value);
    out
}

pub fn write_text(into: &mut String, value: &Value) {
    use std::fmt::Write;
    match value.tag {
        BOOL => into.push_str(if value.as_bool() { "참" } else { "거짓" }),
        INT => {
            let _ = write!(into, "{}", value.as_int());
        }
        FLOAT => into.push_str(&float_text(value.as_float())),
        STR => into.push_str(value.as_text()),
        TABLE => {
            let table = value.as_table();
            into.push('{');
            for (at, item) in table.items.iter().enumerate() {
                if at > 0 {
                    into.push_str(", ");
                }
                write_text(into, item);
            }
            for (at, (key, item)) in table.keys.iter().enumerate() {
                if at > 0 || !table.items.is_empty() {
                    into.push_str(", ");
                }
                into.push_str(key);
                into.push_str(": ");
                write_text(into, item);
            }
            into.push('}');
        }
        _ => into.push_str("없음"),
    }
}

pub fn show(value: &Value) -> String {
    if value.tag == STR {
        format!("\"{}\"", value.as_text())
    } else {
        to_text(value)
    }
}

fn float_text(found: f64) -> String {
    if found.is_infinite() {
        return if found > 0.0 { "무한" } else { "-무한" }.to_string();
    }
    if found.is_nan() {
        return "수가 아님".to_string();
    }
    if found == found.trunc() && found.abs() < 9.2e18 {
        return format!("{}", found as i64);
    }
    significant(found, 12)
}

fn significant(found: f64, digits: usize) -> String {
    let exponent = found.abs().log10().floor() as i32;
    if exponent < -4 || exponent >= digits as i32 {
        let shown = format!("{:.*e}", digits - 1, found);
        let (mantissa, power) = shown.split_once('e').expect("지수");
        return format!("{}e{power}", trim(mantissa));
    }
    let places = (digits as i32 - 1 - exponent).max(0) as usize;
    trim(&format!("{found:.places$}"))
}

fn trim(shown: &str) -> String {
    if !shown.contains('.') {
        return shown.to_string();
    }
    shown
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

// 글자 단위 색인 커서. UTF-8 을 그대로 두는 대신 마지막으로 짚은 자리를
// 하나만 기억한다. `1부터 길이까지` 앞으로 훑는 고리가 O(n^2) → O(n) 이 된다.
struct Cursor {
    ptr: *const u8,
    bytes: usize,
    count: usize,
    at: usize,
    off: usize,
}

struct Slot(UnsafeCell<Cursor>);
unsafe impl Sync for Slot {}

static CURSOR: Slot = Slot(UnsafeCell::new(Cursor {
    ptr: std::ptr::null(),
    bytes: 0,
    count: 0,
    at: 0,
    off: 0,
}));

fn cursor(text: &str) -> &'static mut Cursor {
    // 런타임은 홀실이라 갈래 다툼이 없다.
    let held = unsafe { &mut *CURSOR.0.get() };
    if held.ptr != text.as_ptr() || held.bytes != text.len() {
        *held = Cursor {
            ptr: text.as_ptr(),
            bytes: text.len(),
            count: text.chars().count(),
            at: 0,
            off: 0,
        };
    }
    held
}

pub fn char_len(text: &str) -> usize {
    cursor(text).count
}

pub fn char_at(text: &str, index: usize) -> Option<char> {
    let held = cursor(text);
    if index >= held.count {
        return None;
    }
    if held.count == held.bytes {
        return text.as_bytes().get(index).map(|&byte| byte as char);
    }
    if index < held.at {
        held.at = 0;
        held.off = 0;
    }
    let bytes = text.as_bytes();
    while held.at < index {
        held.off += width(bytes[held.off]);
        held.at += 1;
    }
    text[held.off..].chars().next()
}

fn width(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}
