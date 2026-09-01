use crate::text::{show, to_text};
use crate::value::*;
use std::io::Write;

pub type Nouns = Option<
    unsafe extern "C" fn(
        out: *mut Value,
        owner: *const Value,
        name: *const u8,
        len: usize,
    ) -> i8,
>;

unsafe fn at<'a>(found: *const Value) -> &'a Value {
    &*found
}

unsafe fn name_of<'a>(bytes: *const u8, len: usize) -> &'a str {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(bytes, len))
}

fn fail(kind: &str, message: String) -> ! {
    let _ = std::io::stdout().flush();
    eprintln!("{kind}: {message}");
    std::process::exit(1);
}

#[no_mangle]
pub unsafe extern "C" fn sr_stop(message: *const Value) {
    fail("멈춤", to_text(at(message)));
}

fn numbers(verb: &str, values: [&Value; 2]) {
    if let Some(odd) = values.iter().find(|value| !value.number()) {
        fail(
            "값 오류",
            format!("'{verb}'의 인자가 수가 아님: {} {}", odd.kind(), show(odd)),
        )
    }
}

fn both_int(left: &Value, right: &Value) -> bool {
    left.tag == INT && right.tag == INT
}

#[no_mangle]
pub unsafe extern "C" fn sr_nothing(out: *mut Value) {
    *out = Value::nothing();
}

#[no_mangle]
pub unsafe extern "C" fn sr_int(out: *mut Value, found: i64) {
    *out = Value::int(found);
}

#[no_mangle]
pub unsafe extern "C" fn sr_float(out: *mut Value, found: f64) {
    *out = Value::float(found);
}

#[no_mangle]
pub unsafe extern "C" fn sr_bool(out: *mut Value, found: i8) {
    *out = Value::bool(found != 0);
}

#[no_mangle]
pub unsafe extern "C" fn sr_str(out: *mut Value, bytes: *const u8, len: usize) {
    *out = Value::text(name_of(bytes, len).to_string());
}

#[no_mangle]
pub unsafe extern "C" fn sr_copy(dst: *mut Value, src: *const Value) {
    *dst = *src;
}

#[no_mangle]
pub unsafe extern "C" fn sr_truthy(found: *const Value) -> i8 {
    at(found).truthy() as i8
}

#[no_mangle]
pub unsafe extern "C" fn sr_list_new(out: *mut Value) {
    *out = Value::list(Vec::new());
}

#[no_mangle]
pub unsafe extern "C" fn sr_list_push(list: *const Value, item: *const Value) {
    at(list).as_list().borrow_mut().push(*at(item));
}

#[no_mangle]
pub unsafe extern "C" fn sr_dict_new(out: *mut Value) {
    *out = Value::dict(Vec::new());
}

#[no_mangle]
pub unsafe extern "C" fn sr_dict_put(
    dict: *const Value,
    bytes: *const u8,
    len: usize,
    item: *const Value,
) {
    let key = name_of(bytes, len);
    let dict = at(dict).as_dict();
    let mut entries = dict.borrow_mut();
    match entries.iter_mut().find(|(found, _)| found == key) {
        Some(slot) => slot.1 = *at(item),
        None => entries.push((key.to_string(), *at(item))),
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_template(out: *mut Value, parts: *const Value, count: usize) {
    let mut text = String::new();
    for index in 0..count {
        text.push_str(&to_text(at(parts.add(index))));
    }
    *out = Value::text(text);
}

#[no_mangle]
pub unsafe extern "C" fn sr_print(found: *const Value) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(to_text(at(found)).as_bytes());
}

fn deep_copy(found: &Value) -> Value {
    match found.tag {
        LIST => Value::list(found.as_list().borrow().iter().map(deep_copy).collect()),
        DICT => Value::dict(
            found
                .as_dict()
                .borrow()
                .iter()
                .map(|(key, value)| (key.clone(), deep_copy(value)))
                .collect(),
        ),
        _ => *found,
    }
}

const ORDINALS: [&str; 10] = [
    "첫째",
    "둘째",
    "셋째",
    "넷째",
    "다섯째",
    "여섯째",
    "일곱째",
    "여덟째",
    "아홉째",
    "열째",
];

fn at_index(found: &Value, index: usize, field: &str, size: usize) -> Value {
    if index >= size {
        fail("값 오류", format!("'{field}' 없음. 개수는 {size}"))
    }
    match found.tag {
        LIST => found.as_list().borrow()[index],
        _ => Value::text(
            found
                .as_text()
                .chars()
                .nth(index)
                .expect("글자")
                .to_string(),
        ),
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_field_get(
    out: *mut Value,
    owner: *const Value,
    bytes: *const u8,
    len: usize,
    nouns: Nouns,
) {
    let owner = at(owner);
    let field = name_of(bytes, len);
    if field == "복사본" {
        *out = deep_copy(owner);
        return;
    }
    if field == "자료형" {
        *out = Value::text(owner.type_name().to_string());
        return;
    }
    if owner.tag == DICT {
        if let Some((_, value)) = owner
            .as_dict()
            .borrow()
            .iter()
            .find(|(key, _)| key == field)
        {
            *out = *value;
            return;
        }
    }
    let size = match owner.tag {
        LIST => Some(owner.as_list().borrow().len()),
        STR => Some(owner.as_text().chars().count()),
        _ => None,
    };
    if let Some(size) = size {
        if (field == "개수" && owner.tag == LIST) || (field == "글자수" && owner.tag == STR)
        {
            *out = Value::int(size as i64);
            return;
        }
        if let Some(index) = ORDINALS.iter().position(|found| *found == field) {
            *out = at_index(owner, index, field, size);
            return;
        }
        if field == "마지막" {
            *out = at_index(owner, size.wrapping_sub(1), field, size);
            return;
        }
    }
    if let Some(lookup) = nouns {
        if lookup(out, owner, bytes, len) != 0 {
            return;
        }
    }
    if owner.tag == DICT {
        fail("이름 오류", format!("사전에 '{field}' 없음"));
    }
    fail(
        "이름 오류",
        format!("{}에 필드 '{field}' 없음", owner.kind()),
    );
}

#[no_mangle]
pub unsafe extern "C" fn sr_field_set(
    owner: *const Value,
    bytes: *const u8,
    len: usize,
    value: *const Value,
) {
    let found = at(owner);
    if found.tag != DICT {
        let field = name_of(bytes, len);
        fail(
            "값 오류",
            format!("{}에 열쇠 '{field}' 매길 수 없음", found.kind()),
        );
    }
    sr_dict_put(owner, bytes, len, value);
}

#[no_mangle]
pub unsafe extern "C" fn sr_index(out: *mut Value, owner: *const Value, place: *const Value) {
    let owner = at(owner);
    let place = at(place);
    if place.tag != INT {
        fail("값 오류", format!("자리가 정수가 아님: {}", to_text(place)));
    }
    let size = match owner.tag {
        LIST => owner.as_list().borrow().len(),
        STR => owner.as_text().chars().count(),
        _ => {
            fail("이름 오류", format!("{}에 자리가 없음", owner.kind()));
        }
    };
    let index = place.as_int();
    if index < 1 || index as usize > size {
        fail("값 오류", format!("{index}번째 없음. 개수는 {size}"));
    }
    *out = at_index(owner, index as usize - 1, "자리", size);
}

fn arith(
    verb: &str,
    out: *mut Value,
    left: &Value,
    right: &Value,
    whole: fn(i64, i64) -> i64,
    real: fn(f64, f64) -> f64,
) {
    numbers(verb, [left, right]);
    let made = if both_int(left, right) {
        Value::int(whole(left.as_int(), right.as_int()))
    } else {
        Value::float(real(left.number_value(), right.number_value()))
    };
    unsafe { *out = made };
}

#[no_mangle]
pub unsafe extern "C" fn sr_add(out: *mut Value, left: *const Value, right: *const Value) {
    let (left, right) = (at(left), at(right));
    if left.tag == LIST {
        let target = left.as_list();
        if right.tag == LIST {
            let extra: Vec<Value> = right.as_list().borrow().clone();
            target.borrow_mut().extend(extra);
        } else {
            target.borrow_mut().push(*right);
        }
        *out = *left;
        return;
    }
    arith(
        "더하다",
        out,
        left,
        right,
        |a, b| a.wrapping_add(b),
        |a, b| a + b,
    );
}

#[no_mangle]
pub unsafe extern "C" fn sr_add_copy(out: *mut Value, left: *const Value, right: *const Value) {
    let left = at(left);
    if left.tag == LIST {
        let copy = Value::list(left.as_list().borrow().clone());
        return sr_add(out, &copy, right);
    }
    sr_add(out, left, right);
}

#[no_mangle]
pub unsafe extern "C" fn sr_sub(out: *mut Value, left: *const Value, right: *const Value) {
    arith(
        "빼다",
        out,
        at(left),
        at(right),
        |a, b| a.wrapping_sub(b),
        |a, b| a - b,
    );
}

#[no_mangle]
pub unsafe extern "C" fn sr_mul(out: *mut Value, left: *const Value, right: *const Value) {
    arith(
        "곱하다",
        out,
        at(left),
        at(right),
        |a, b| a.wrapping_mul(b),
        |a, b| a * b,
    );
}

fn divisor(left: &Value, right: &Value) -> (f64, f64) {
    numbers("나누다", [left, right]);
    let divisor = right.number_value();
    if divisor == 0.0 {
        fail("산술 오류", "0으로 나눌 수 없음".to_string());
    }
    (left.number_value(), divisor)
}

#[no_mangle]
pub unsafe extern "C" fn sr_div(out: *mut Value, left: *const Value, right: *const Value) {
    let (a, b) = divisor(at(left), at(right));
    let made = a / b;
    *out = if made == made.trunc() && made.abs() < 9.2e18 {
        Value::int(made as i64)
    } else {
        Value::float(made)
    };
}

#[no_mangle]
pub unsafe extern "C" fn sr_rem(out: *mut Value, left: *const Value, right: *const Value) {
    let (a, b) = divisor(at(left), at(right));
    let made = a - b * (a / b).floor();
    *out = if both_int(at(left), at(right)) {
        Value::int(made as i64)
    } else {
        Value::float(made)
    };
}

fn order(verb: &str, left: &Value, right: &Value) -> std::cmp::Ordering {
    if left.number() && right.number() {
        if let Some(found) = left.number_value().partial_cmp(&right.number_value()) {
            return found;
        }
    } else if left.tag == STR && right.tag == STR {
        return left.as_text().cmp(right.as_text());
    }
    fail(
        "값 오류",
        format!(
            "'{verb}'로 견줄 수 없음: {} {}, {} {}",
            left.kind(),
            show(left),
            right.kind(),
            show(right)
        ),
    )
}

#[no_mangle]
pub unsafe extern "C" fn sr_greater(out: *mut Value, left: *const Value, right: *const Value) {
    *out = Value::bool(order("크다", at(left), at(right)).is_gt());
}

#[no_mangle]
pub unsafe extern "C" fn sr_less(out: *mut Value, left: *const Value, right: *const Value) {
    *out = Value::bool(order("작다", at(left), at(right)).is_lt());
}

#[no_mangle]
pub unsafe extern "C" fn sr_equal(out: *mut Value, left: *const Value, right: *const Value) {
    *out = Value::bool(equal(at(left), at(right)));
}

#[no_mangle]
pub unsafe extern "C" fn sr_not(out: *mut Value, found: *const Value) {
    *out = Value::bool(!at(found).truthy());
}

#[no_mangle]
pub unsafe extern "C" fn sr_convert(out: *mut Value, found: *const Value, kind: *const Value) {
    let found = at(found);
    let kind = to_text(at(kind));
    let refuse = || -> Value {
        fail("값 오류", format!("{kind}로 바꿀 수 없음: {}", show(found)))
    };
    *out = match kind.as_str() {
        "정수" | "수" => match found.tag {
            INT => *found,
            FLOAT => Value::int(found.as_float() as i64),
            BOOL => Value::int(found.as_bool() as i64),
            STR => match found.as_text().trim().parse::<f64>() {
                Ok(number) => Value::int(number as i64),
                Err(_) => refuse(),
            },
            _ => refuse(),
        },
        "실수" => match found.tag {
            INT => Value::float(found.as_int() as f64),
            FLOAT => *found,
            BOOL => Value::float(found.as_bool() as i64 as f64),
            STR => match found.as_text().trim().parse::<f64>() {
                Ok(number) => Value::float(number),
                Err(_) => refuse(),
            },
            _ => refuse(),
        },
        "문자열" => Value::text(to_text(found)),
        "논리값" => match found.tag {
            STR => match found.as_text() {
                "참" => Value::bool(true),
                "거짓" => Value::bool(false),
                _ => refuse(),
            },
            _ => Value::bool(found.truthy()),
        },
        _ => fail("값 오류", format!("바꿀 수 없는 자료형: '{kind}'")),
    };
}

fn pieces(found: &Value) -> Vec<String> {
    match found.tag {
        LIST => found.as_list().borrow().iter().map(to_text).collect(),
        _ => vec![to_text(found)],
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_join(out: *mut Value, found: *const Value) {
    *out = Value::text(pieces(at(found)).concat());
}

#[no_mangle]
pub unsafe extern "C" fn sr_join_with(out: *mut Value, found: *const Value, sep: *const Value) {
    *out = Value::text(pieces(at(found)).join(&to_text(at(sep))));
}

#[no_mangle]
pub unsafe extern "C" fn sr_split(out: *mut Value, found: *const Value, sep: *const Value) {
    let text = to_text(at(found));
    let mark = to_text(at(sep));
    let parts: Vec<Value> = if mark.is_empty() {
        text.chars().map(|ch| Value::text(ch.to_string())).collect()
    } else {
        text.split(&mark)
            .map(|part| Value::text(part.to_string()))
            .collect()
    };
    *out = Value::list(parts);
}

#[no_mangle]
pub unsafe extern "C" fn sr_contains(out: *mut Value, whole: *const Value, part: *const Value) {
    let (whole, part) = (at(whole), at(part));
    *out = Value::bool(if whole.tag == LIST {
        whole
            .as_list()
            .borrow()
            .iter()
            .any(|item| equal(item, part))
    } else {
        to_text(whole).contains(&to_text(part))
    });
}

#[no_mangle]
pub unsafe extern "C" fn sr_starts(out: *mut Value, whole: *const Value, part: *const Value) {
    *out = Value::bool(to_text(at(whole)).starts_with(&to_text(at(part))));
}

#[no_mangle]
pub unsafe extern "C" fn sr_ends(out: *mut Value, whole: *const Value, part: *const Value) {
    *out = Value::bool(to_text(at(whole)).ends_with(&to_text(at(part))));
}

#[no_mangle]
pub unsafe extern "C" fn sr_trim(out: *mut Value, found: *const Value) {
    *out = Value::text(to_text(at(found)).trim().to_string());
}

#[no_mangle]
pub unsafe extern "C" fn sr_check_bool(found: *const Value, bytes: *const u8, len: usize) {
    let found = at(found);
    if found.tag != BOOL {
        let name = name_of(bytes, len);
        if found.tag == NOTHING {
            fail("값 오류", format!("'{name}' 아무 값도 돌려주지 않음"));
        } else {
            fail(
                "값 오류",
                format!(
                    "'{name}' 낸 값이 논리값이 아님: {} {}",
                    found.kind(),
                    show(found)
                ),
            );
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_check_value(found: *const Value, bytes: *const u8, len: usize) {
    if at(found).tag == NOTHING {
        fail(
            "값 오류",
            format!("'{}' 아무 값도 돌려주지 않음", name_of(bytes, len)),
        );
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_name_is(
    name: *const u8,
    len: usize,
    other: *const u8,
    other_len: usize,
) -> i8 {
    (name_of(name, len) == name_of(other, other_len)) as i8
}

#[no_mangle]
pub unsafe extern "C" fn sr_list_len(found: *const Value) -> i64 {
    at(found).as_list().borrow().len() as i64
}

#[no_mangle]
pub unsafe extern "C" fn sr_list_get(out: *mut Value, found: *const Value, index: i64) {
    *out = at(found).as_list().borrow()[index as usize];
}

#[no_mangle]
pub unsafe extern "C" fn sr_range(
    out: *mut Value,
    start: *const Value,
    stop: *const Value,
    step: *const Value,
) {
    let (start, stop, step) = (at(start), at(stop), at(step));
    numbers("반복하다", [start, stop]);
    numbers("반복하다", [step, step]);
    let whole = start.tag == INT && stop.tag == INT && step.tag == INT;
    let (from, to) = (start.number_value(), stop.number_value());
    let mut by = step.number_value().abs();
    if by == 0.0 {
        by = 1.0;
    }
    let down = from > to;
    let mut made = Vec::new();
    let mut now = from;
    while if down { now >= to } else { now <= to } {
        made.push(if whole {
            Value::int(now as i64)
        } else {
            Value::float(now)
        });
        now += if down { -by } else { by };
    }
    *out = Value::list(made);
}

#[no_mangle]
pub unsafe extern "C" fn sr_truthy_value(out: *mut Value, found: *const Value) {
    *out = Value::bool(at(found).truthy());
}

#[no_mangle]
pub unsafe extern "C" fn sr_open(out: *mut Value, target: *const Value) {
    let path = to_text(at(target));
    let found = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path);
    *out = match found {
        Ok(file) => Value::handle(Handle {
            file: std::cell::RefCell::new(Some(file)),
            path,
            written: std::cell::Cell::new(false),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            fail("파일 오류", "권한없음".to_string())
        }
        Err(_) => fail("파일 오류", "파일없음".to_string()),
    };
}

#[no_mangle]
pub unsafe extern "C" fn sr_read(out: *mut Value, target: *const Value) {
    use std::io::{Read, Seek};
    let target = at(target);
    if target.tag == FILE {
        let handle = target.as_handle();
        let mut slot = handle.file.borrow_mut();
        let Some(file) = slot.as_mut() else {
            fail("파일 오류", "닫힌파일".to_string());
        };
        let _ = file.flush();
        let _ = file.seek(std::io::SeekFrom::Start(0));
        let mut text = String::new();
        *out = match file.read_to_string(&mut text) {
            Ok(_) => Value::text(text),
            Err(_) => fail("파일 오류", "읽을수없음".to_string()),
        };
        return;
    }
    *out = match std::fs::read_to_string(to_text(target)) {
        Ok(text) => Value::text(text),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            fail("파일 오류", "권한없음".to_string())
        }
        Err(_) => fail("파일 오류", "파일없음".to_string()),
    };
}

#[no_mangle]
pub unsafe extern "C" fn sr_write(target: *const Value, value: *const Value) {
    use std::io::Seek;
    let target = at(target);
    if target.tag != FILE {
        fail("값 오류", "'쓰다'의 '~에' 자리가 파일이 아님".to_string())
    }
    let handle = target.as_handle();
    let mut slot = handle.file.borrow_mut();
    let Some(file) = slot.as_mut() else {
        fail("파일 오류", "닫힌파일".to_string());
    };
    if !handle.written.get() {
        let _ = file.seek(std::io::SeekFrom::Start(0));
        let _ = file.set_len(0);
        handle.written.set(true);
    }
    let _ = file.write_all(to_text(at(value)).as_bytes());
}

#[no_mangle]
pub unsafe extern "C" fn sr_close(target: *const Value) {
    let target = at(target);
    if target.tag == FILE {
        let _ = target.as_handle().file.borrow_mut().take();
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_readline(out: *mut Value) {
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    *out = match std::io::stdin().read_line(&mut line) {
        Ok(0) => fail("파일 오류", "입력끝".to_string()),
        Ok(_) => Value::text(line.trim_end_matches('\n').to_string()),
        Err(_) => fail("파일 오류", "입력끝".to_string()),
    };
}

thread_local! {
    static PARKED: std::cell::RefCell<Vec<Option<(String, String)>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}
