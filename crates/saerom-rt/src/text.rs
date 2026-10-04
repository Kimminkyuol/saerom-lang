use std::cell::UnsafeCell;
use unicode_segmentation::UnicodeSegmentation;

use crate::value::{Value, BOOL, FLOAT, INT, STR, TABLE};

pub fn to_text(value: &Value) -> String {
    let mut out = String::new();
    write_text(&mut out, value);
    out
}

pub fn write_text(into: &mut String, value: &Value) {
    write(into, value, &mut Vec::new());
}

// open: 지금 펼치는 묶음들. 순환이면 줄인다.
fn write(into: &mut String, value: &Value, open: &mut Vec<u64>) {
    use std::fmt::Write;
    match value.tag {
        BOOL => into.push_str(if value.as_bool() { "참" } else { "거짓" }),
        INT => {
            let _ = write!(into, "{}", value.as_int());
        }
        FLOAT => into.push_str(&float_text(value.as_float())),
        STR => into.push_str(value.as_text()),
        TABLE if open.contains(&value.bits) => into.push_str("{…}"),
        TABLE => {
            open.push(value.bits);
            let table = value.as_table();
            into.push('{');
            for (at, item) in table.items.iter().enumerate() {
                if at > 0 {
                    into.push_str(", ");
                }
                write(into, item, open);
            }
            for (at, (key, item)) in table.keys.iter().enumerate() {
                if at > 0 || !table.items.is_empty() {
                    into.push_str(", ");
                }
                into.push_str(key);
                into.push_str(": ");
                write(into, item, open);
            }
            into.push('}');
            open.pop();
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

// 다시 읽으면 같은 값이 되는 최단 표기. 정숫값은 정수처럼 (`1`과 `1.0`은 같은 값).
pub fn float_text(found: f64) -> String {
    if found == found.trunc() && found.abs() < 9.2e18 {
        return format!("{}", found as i64);
    }
    format!("{found:?}")
}

// 글자(grapheme) 단위 색인 커서. UTF-8 을 그대로 두는 대신 마지막으로 짚은 자리를
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
            count: text.graphemes(true).count(),
            at: 0,
            off: 0,
        };
    }
    held
}

// 수집 뒤에는 주소가 재활용될 수 있어 기억을 버린다.
pub fn forget() {
    let held = unsafe { &mut *CURSOR.0.get() };
    held.ptr = std::ptr::null();
    held.bytes = usize::MAX;
}

// 제자리 잇기로 버퍼가 옮겨지면 옛 주소를 새 글이 받을 수 있다.
pub fn moved(old: *const u8, now: *const u8) {
    let held = unsafe { &mut *CURSOR.0.get() };
    if old != now && held.ptr == old {
        forget();
    }
}

pub fn char_len(text: &str) -> usize {
    cursor(text).count
}

pub fn char_at(text: &str, index: usize) -> Option<&str> {
    let held = cursor(text);
    if index >= held.count {
        return None;
    }
    if held.count == held.bytes {
        return text.get(index..index + 1);
    }
    if index < held.at {
        held.at = 0;
        held.off = 0;
    }
    while held.at < index {
        held.off += next_char(&text[held.off..]).len();
        held.at += 1;
    }
    Some(next_char(&text[held.off..]))
}

fn next_char(text: &str) -> &str {
    text.graphemes(true).next().unwrap_or("")
}
