use crate::msg;
use crate::report::{self, Report};
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

#[repr(C)]
pub struct Source {
    name: *const u8,
    name_len: usize,
    text: *const u8,
    text_len: usize,
}

#[no_mangle]
pub static mut SR_POS: u64 = 0;

static mut SOURCES: (*const Source, usize) = (std::ptr::null(), 0);
static mut TRACED: bool = false;

#[no_mangle]
pub unsafe extern "C" fn sr_sources(table: *const Source, count: usize, traced: i8) {
    SOURCES = (table, count);
    TRACED = traced != 0;
}

unsafe fn source_at(index: usize) -> Option<(&'static str, &'static str)> {
    let (table, count) = SOURCES;
    if table.is_null() || index >= count {
        return None;
    }
    let found = &*table.add(index);
    Some((
        name_of(found.name, found.name_len),
        name_of(found.text, found.text_len),
    ))
}

#[repr(C)]
pub struct Site {
    name: *const u8,
    name_len: usize,
}

pub const FRAMES: usize = 1024;

#[no_mangle]
pub static mut SR_FRAMES: [*const Site; FRAMES] = [std::ptr::null(); FRAMES];

#[no_mangle]
pub static mut SR_AT: [u64; FRAMES] = [0; FRAMES];

#[no_mangle]
pub static mut SR_DEPTH: u32 = 0;

struct Spot {
    path: &'static str,
    text: &'static str,
    line: usize,
    col: usize,
    end: usize,
}

fn spot(packed: u64) -> Option<Spot> {
    let module = (packed >> 48) as usize;
    let line = ((packed >> 24) & 0xFF_FFFF) as usize;
    let col = ((packed >> 12) & 0xFFF) as usize;
    let width = (packed & 0xFFF) as usize;
    if line == 0 {
        return None;
    }
    let (path, text) = unsafe { source_at(module) }?;
    Some(Spot {
        path,
        text,
        line,
        col,
        end: col + width,
    })
}

fn where_at(packed: u64) -> Option<String> {
    let spot = spot(packed)?;
    Some(format!("{}:{}:{}", spot.path, spot.line, spot.col + 1))
}

fn frame_name(level: usize) -> String {
    let site =
        unsafe { std::ptr::read((&raw const SR_FRAMES).cast::<*const Site>().add(level)) };
    if site.is_null() {
        return msg::FRAME_UNKNOWN.to_string();
    }
    let site = unsafe { &*site };
    unsafe { name_of(site.name, site.name_len) }.to_string()
}

fn frame(level: usize, name: &str, here: u64) -> String {
    let mut out = format!("{}: {name}\n", report::blue(&format!("{level:>4}")));
    if let Some(at) = where_at(here) {
        out.push_str(&format!("             at {at}\n"));
    }
    out
}

fn trace() -> String {
    if !unsafe { std::ptr::read(&raw const TRACED) } {
        return report::note(msg::TRACE_OFF);
    }
    let depth = unsafe { std::ptr::read(&raw const SR_DEPTH) } as usize;
    let shown = depth.min(FRAMES);
    let mut out = format!("{}\n", report::bold(msg::TRACE));
    let mut here = unsafe { std::ptr::read(&raw const SR_POS) };
    for level in (0..shown).rev() {
        out.push_str(&frame(shown - 1 - level, &frame_name(level), here));
        here = unsafe { std::ptr::read((&raw const SR_AT).cast::<u64>().add(level)) };
    }
    out.push_str(&frame(shown, msg::FRAME_TOP, here));
    out
}

fn fail(kind: &str, message: String) -> ! {
    let _ = std::io::stdout().flush();
    let here = unsafe { std::ptr::read(&raw const SR_POS) };
    match spot(here) {
        Some(spot) => eprint!(
            "{}",
            Report {
                kind,
                msg: &message,
                hint: None,
                path: spot.path,
                source: Some(spot.text),
                line: spot.line,
                col: spot.col,
                end: spot.end,
            }
            .render()
        ),
        None => eprint!("{}", report::plain(kind, &message)),
    }
    eprint!("\n{}", trace());
    std::process::exit(1);
}

#[no_mangle]
pub unsafe extern "C" fn sr_stop(message: *const Value) {
    fail(msg::STOP, to_text(at(message)));
}

fn numbers(verb: &str, values: [&Value; 2]) {
    if let Some(odd) = values.iter().find(|value| !value.number()) {
        fail(
            msg::VALUE,
            msg::arg_not_number(verb, odd.kind(), &show(odd)),
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

fn at_index(found: &Value, index: usize, field: &str, size: usize) -> Value {
    if index >= size {
        fail(msg::VALUE, msg::empty_at(field, size))
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
        fail(msg::NAME, msg::no_field(field));
    }
    fail(msg::NAME, msg::no_field_on(owner.kind(), field));
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
        fail(msg::VALUE, msg::no_field_on(found.kind(), field));
    }
    sr_dict_put(owner, bytes, len, value);
}

#[no_mangle]
pub unsafe extern "C" fn sr_index(out: *mut Value, owner: *const Value, place: *const Value) {
    let owner = at(owner);
    let place = at(place);
    if place.tag != INT {
        fail(msg::VALUE, msg::place_not_int(&to_text(place)));
    }
    let size = match owner.tag {
        LIST => owner.as_list().borrow().len(),
        STR => owner.as_text().chars().count(),
        _ => {
            fail(msg::NAME, msg::no_place(owner.kind()));
        }
    };
    let index = place.as_int();
    if index < 1 || index as usize > size {
        fail(msg::VALUE, msg::out_of_range(index, size));
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
    if left.tag == STR {
        *out = Value::text(format!("{}{}", left.as_text(), to_text(right)));
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
        fail(msg::ARITH, msg::DIV_ZERO.to_string());
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
        msg::VALUE,
        msg::cannot_order(
            verb,
            &format!("{} {}", left.kind(), show(left)),
            &format!("{} {}", right.kind(), show(right)),
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
    let refuse = || -> Value { fail(msg::VALUE, msg::cannot_convert(&kind, &show(found))) };
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
        _ => fail(msg::VALUE, msg::unknown_kind(&kind)),
    };
}

#[no_mangle]
pub unsafe extern "C" fn sr_check_bool(found: *const Value, bytes: *const u8, len: usize) {
    let found = at(found);
    if found.tag != BOOL {
        let name = name_of(bytes, len);
        if found.tag == NOTHING {
            fail(msg::VALUE, msg::no_return(name));
        } else {
            fail(msg::VALUE, msg::not_bool(name, found.kind(), &show(found)));
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_check_value(found: *const Value, bytes: *const u8, len: usize) {
    if at(found).tag == NOTHING {
        fail(msg::VALUE, msg::no_return(name_of(bytes, len)));
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

thread_local! {
    static PARKED: std::cell::RefCell<Vec<Option<(String, String)>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[no_mangle]
pub extern "C" fn sr_finish() {
    let _ = std::io::stdout().flush();
}

#[no_mangle]
pub unsafe extern "C" fn sr_clone(out: *mut Value, found: *const Value) {
    *out = deep_copy(at(found));
}

#[no_mangle]
pub unsafe extern "C" fn sr_open(out: *mut Value, path: *const Value, how: *const Value) {
    use std::os::unix::ffi::OsStrExt;
    let name = to_text(at(path));
    let mode = to_text(at(how));
    let mut open = std::fs::OpenOptions::new();
    match mode.as_str() {
        "읽기" => open.read(true),
        "쓰기" => open.write(true).create(true).truncate(true),
        "추가" => open.append(true).create(true),
        _ => fail(msg::VALUE, msg::bad_mode(&mode)),
    };
    let found = open.open(std::ffi::OsStr::from_bytes(name.as_bytes()));
    *out = match found {
        Ok(file) => {
            use std::os::unix::io::IntoRawFd;
            Value::int(file.into_raw_fd() as i64)
        }
        Err(error) => fail(msg::FILE, msg::cannot_open(&name, &error.to_string())),
    };
}

fn descriptor(verb: &str, found: &Value) -> i32 {
    if found.tag != INT {
        fail(msg::VALUE, msg::not_descriptor(verb, &show(found)));
    }
    found.as_int() as i32
}

#[no_mangle]
pub unsafe extern "C" fn sr_read(out: *mut Value, file: *const Value, count: *const Value) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;
    let fd = descriptor("읽다", at(file));
    let want = descriptor("읽다", at(count)).max(0) as usize;
    let mut held = std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd));
    let mut buffer = vec![0u8; want];
    let read = held.read(&mut buffer).unwrap_or_else(|error| {
        fail(msg::FILE, msg::cannot_read(&error.to_string()));
    });
    buffer.truncate(read);
    *out = Value::text(String::from_utf8_lossy(&buffer).into_owned());
}

#[no_mangle]
pub unsafe extern "C" fn sr_write(out: *mut Value, file: *const Value, text: *const Value) {
    use std::os::unix::io::FromRawFd;
    let fd = descriptor("쓰다", at(file));
    let body = to_text(at(text));
    let mut held = std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd));
    let written = held.write(body.as_bytes()).unwrap_or_else(|error| {
        fail(msg::FILE, msg::cannot_write(&error.to_string()));
    });
    let _ = held.flush();
    *out = Value::int(written as i64);
}

#[no_mangle]
pub unsafe extern "C" fn sr_close(file: *const Value) {
    use std::os::unix::io::FromRawFd;
    let fd = descriptor("닫다", at(file));
    drop(std::fs::File::from_raw_fd(fd));
}
