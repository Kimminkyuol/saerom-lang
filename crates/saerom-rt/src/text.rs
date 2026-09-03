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
