use crate::value::{Value, BOOL, DICT, FLOAT, INT, LIST, NOTHING, STR};

pub fn to_text(value: &Value) -> String {
    match value.tag {
        NOTHING => "없음".to_string(),
        BOOL => if value.as_bool() { "참" } else { "거짓" }.to_string(),
        INT => value.as_int().to_string(),
        FLOAT => float_text(value.as_float()),
        STR => value.as_text().to_string(),
        LIST => {
            let items = value.as_list().borrow();
            let shown: Vec<String> = items.iter().map(to_text).collect();
            format!("[{}]", shown.join(", "))
        }
        DICT => {
            let entries = value.as_dict().borrow();
            let shown: Vec<String> = entries
                .iter()
                .map(|(key, value)| format!("{key}: {}", to_text(value)))
                .collect();
            format!("{{{}}}", shown.join(", "))
        }
        _ => format!("파일 {}", value.as_handle().path),
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
        return "수아님".to_string();
    }
    if found == found.trunc() && found.abs() < 9.2e18 {
        return format!("{}", found as i64);
    }
    significant(found, 12)
}

/// C 의 %g 와 같은 꼴. 유효숫자를 정해 두고 꼬리의 0을 지운다.
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
