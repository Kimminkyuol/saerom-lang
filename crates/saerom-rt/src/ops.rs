use crate::msg;

use crate::text::{show, to_text, write_text};
use crate::value::*;
use crate::{fault::fail, io::flush_out};


pub type Nouns = Option<
    unsafe extern "C" fn(
        out: *mut Value,
        owner: *const Value,
        name: *const u8,
        len: usize,
    ) -> i8,
>;

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
pub unsafe extern "C" fn sr_str(out: *mut Value, bytes: *const u8, len: usize) {
    *out = Value::text(name_of(bytes, len).to_string());
}

// 리터럴은 부를 때마다 힙에 복사할 이유가 없다. 자리마다 한 번만 만든다.
// 제자리 잇기(sr_append)가 닿는 슬롯에는 emit 이 이 길을 쓰지 않는다.
#[no_mangle]
pub unsafe extern "C" fn sr_str_kept(
    out: *mut Value,
    cache: *mut Value,
    bytes: *const u8,
    len: usize,
) {
    let held = &mut *cache;
    if held.tag != STR {
        *held = Value::text(name_of(bytes, len).to_string());
    }
    *out = *held;
}

#[no_mangle]
pub unsafe extern "C" fn sr_truthy(found: *const Value) -> i8 {
    at(found).truthy() as i8
}

#[no_mangle]
pub unsafe extern "C" fn sr_table_new(out: *mut Value) {
    *out = Value::table(Table::default());
}

#[no_mangle]
pub unsafe extern "C" fn sr_table_push(table: *const Value, item: *const Value) {
    at(table).as_table().items.push(*at(item));
}

#[no_mangle]
pub unsafe extern "C" fn sr_table_put(
    table: *const Value,
    bytes: *const u8,
    len: usize,
    item: *const Value,
) {
    at(table).as_table().put(name_of(bytes, len), *at(item));
}

#[no_mangle]
pub unsafe extern "C" fn sr_push(table: *const Value, item: *const Value) {
    let found = at(table);
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("추가하다", found.kind()));
    }
    found.as_table().items.push(*at(item));
}

#[no_mangle]
pub unsafe extern "C" fn sr_remove_at(table: *const Value, place: *const Value) {
    let found = at(table);
    let place = at(place);
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("제거하다", found.kind()));
    }
    if place.tag != INT {
        fail(msg::VALUE, msg::place_not_int(&to_text(place)));
    }
    let index = place.as_int();
    let held = found.as_table();
    if index < 1 || index as usize > held.items.len() {
        fail(msg::VALUE, msg::out_of_range(index, held.items.len()));
    }
    held.items.remove(index as usize - 1);
}

#[no_mangle]
pub unsafe extern "C" fn sr_remove_key(table: *const Value, key: *const Value) {
    let found = at(table);
    let key = at(key).as_text();
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("제거하다", found.kind()));
    }
    if !found.as_table().drop_key(key) {
        fail(msg::NAME, msg::no_field(key));
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_template(out: *mut Value, parts: *const Value, count: usize) {
    let mut text = String::new();
    for index in 0..count {
        write_text(&mut text, at(parts.add(index)));
    }
    *out = Value::text(text);
}

fn deep_copy(found: &Value) -> Value {
    if found.tag != TABLE {
        return *found;
    }
    let held = found.as_table();
    Value::table(Table {
        items: held.items.iter().map(deep_copy).collect(),
        keys: held
            .keys
            .iter()
            .map(|(key, value)| (*key, deep_copy(value)))
            .collect(),
    })
}

fn at_index(found: &Value, index: usize) -> Value {
    match found.tag {
        TABLE => found.as_table().items[index],
        _ => Value::text(
            crate::text::char_at(found.as_text(), index)
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
    // 내장 필드 > 파생 필드 > 열쇠. 컴파일 때 아는 것이 먼저다.
    // 가려진 열쇠는 `X의 (식)` 으로 꺼낸다.
    if field == "자료형" {
        *out = Value::text(owner.type_name().to_string());
        return;
    }
    if field == "명칭" && owner.tag == TABLE {
        let names = owner
            .as_table()
            .keys
            .iter()
            .map(|(key, _)| Value::text(key.to_string()))
            .collect();
        *out = Value::items(names);
        return;
    }
    if field == "길이" {
        let size = match owner.tag {
            TABLE => Some(owner.as_table().items.len()),
            STR => Some(crate::text::char_len(owner.as_text())),
            _ => None,
        };
        if let Some(size) = size {
            *out = Value::int(size as i64);
            return;
        }
    }
    if let Some(lookup) = nouns {
        if lookup(out, owner, bytes, len) != 0 {
            return;
        }
    }
    if sr_key_get(out, owner, bytes, len) {
        return;
    }
    if owner.tag == TABLE {
        fail(msg::NAME, msg::no_field(field));
    }
    fail(msg::NAME, msg::no_field_on(owner.kind(), field));
}

unsafe fn sr_key_get(out: *mut Value, owner: &Value, bytes: *const u8, len: usize) -> bool {
    if owner.tag != TABLE {
        return false;
    }
    match owner.as_table().get(name_of(bytes, len)) {
        Some(value) => {
            *out = value;
            true
        }
        None => false,
    }
}

fn key_text(key: &Value) -> &'static str {
    if key.tag != STR {
        fail(msg::VALUE, msg::name_not_text(&show(key)));
    }
    key.as_text()
}

#[no_mangle]
pub unsafe extern "C" fn sr_pick_get(
    out: *mut Value,
    owner: *const Value,
    key: *const Value,
    nouns: Nouns,
) {
    // 괄호는 열쇠 전용이다. 필드는 앞말에 붙어 읽히고 열쇠는 따로 선다.
    let _ = nouns;
    let owner = at(owner);
    let name = key_text(at(key));
    if !sr_key_get(out, owner, name.as_ptr(), name.len()) {
        fail(msg::NAME, msg::no_field(name));
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_pick_set(
    owner: *const Value,
    key: *const Value,
    value: *const Value,
) {
    // 빌린 글자를 명칭으로 쥐면 그 글자가 수집될 때 무너진다. 따로 남긴다.
    let name: &'static str = key_text(at(key)).to_string().leak();
    sr_field_set(owner, name.as_ptr(), name.len(), value);
}

#[no_mangle]
pub unsafe extern "C" fn sr_field_set(
    owner: *const Value,
    bytes: *const u8,
    len: usize,
    value: *const Value,
) {
    let found = at(owner);
    if found.tag != TABLE {
        let field = name_of(bytes, len);
        fail(msg::VALUE, msg::no_field_on(found.kind(), field));
    }
    sr_table_put(owner, bytes, len, value);
}

#[no_mangle]
pub unsafe extern "C" fn sr_index(out: *mut Value, owner: *const Value, place: *const Value) {
    let owner = at(owner);
    let place = at(place);
    if place.tag != INT {
        fail(msg::VALUE, msg::place_not_int(&to_text(place)));
    }
    let size = match owner.tag {
        TABLE => owner.as_table().items.len(),
        STR => crate::text::char_len(owner.as_text()),
        _ => {
            fail(msg::NAME, msg::no_place(owner.kind()));
        }
    };
    let index = place.as_int();
    if index < 1 || index as usize > size {
        fail(msg::VALUE, msg::out_of_range(index, size));
    }
    *out = at_index(owner, index as usize - 1);
}

fn arith(
    verb: &str,
    out: *mut Value,
    left: &Value,
    right: &Value,
    whole: fn(i64, i64) -> Option<i64>,
    real: fn(f64, f64) -> f64,
) {
    numbers(verb, [left, right]);
    let made = if both_int(left, right) {
        // 조용한 랩어라운드 대신 멈춘다.
        match whole(left.as_int(), right.as_int()) {
            Some(found) => Value::int(found),
            None => fail(msg::ARITH, msg::overflow(verb)),
        }
    } else {
        Value::float(real(left.number_value(), right.number_value()))
    };
    unsafe { *out = made };
}

#[no_mangle]
pub unsafe extern "C" fn sr_append(dst: *mut Value, tail: *const Value) {
    let held = &mut *dst;
    let tail = at(tail);
    if held.tag != STR || held.bits == tail.bits {
        return sr_add(dst, dst, tail);
    }
    let text = held.as_string();
    if tail.tag == STR {
        text.push_str(tail.as_text());
    } else {
        write_text(text, tail);
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_add(out: *mut Value, left: *const Value, right: *const Value) {
    let (left, right) = (at(left), at(right));
    if left.tag == STR {
        let head = left.as_text();
        let tail = if right.tag == STR {
            right.as_text().len()
        } else {
            16
        };
        let mut made = String::with_capacity(head.len() + tail);
        made.push_str(head);
        write_text(&mut made, right);
        *out = Value::text(made);
        return;
    }
    arith(
        "더하다",
        out,
        left,
        right,
        |a, b| a.checked_add(b),
        |a, b| a + b,
    );
}

#[no_mangle]
pub unsafe extern "C" fn sr_sub(out: *mut Value, left: *const Value, right: *const Value) {
    arith(
        "빼다",
        out,
        at(left),
        at(right),
        |a, b| a.checked_sub(b),
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
        |a, b| a.checked_mul(b),
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
    // 늘 실수. 나누어 떨어지는지에 따라 갈래가 바뀌면 타입이 값에 매인다.
    let (a, b) = divisor(at(left), at(right));
    *out = Value::float(a / b);
}

#[no_mangle]
pub unsafe extern "C" fn sr_quot(out: *mut Value, left: *const Value, right: *const Value) {
    let (left, right) = (at(left), at(right));
    if both_int(left, right) {
        *out = Value::int(sr_quot_int(left.as_int(), right.as_int()));
        return;
    }
    let (a, b) = divisor(left, right);
    *out = Value::float((a / b).floor());
}

// 나머지가 몫에 맞물리도록 내림 나눗셈을 쓴다.
#[no_mangle]
pub extern "C" fn sr_quot_int(left: i64, right: i64) -> i64 {
    if right == 0 {
        fail(msg::ARITH, msg::DIV_ZERO.to_string());
    }
    let made = left.wrapping_div(right);
    let rest = left.wrapping_rem(right);
    if rest != 0 && (rest < 0) != (right < 0) {
        made - 1
    } else {
        made
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_overflow(bytes: *const u8, len: usize) -> ! {
    fail(msg::ARITH, msg::overflow(name_of(bytes, len)))
}

#[no_mangle]
pub extern "C" fn sr_rem_int(left: i64, right: i64) -> i64 {
    if right == 0 {
        fail(msg::ARITH, msg::DIV_ZERO.to_string());
    }
    let made = left.wrapping_rem(right);
    if made != 0 && (made < 0) != (right < 0) {
        made + right
    } else {
        made
    }
}

#[no_mangle]
pub extern "C" fn sr_rem_real(left: f64, right: f64) -> f64 {
    if right == 0.0 {
        fail(msg::ARITH, msg::DIV_ZERO.to_string());
    }
    left - right * (left / right).floor()
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
pub unsafe extern "C" fn sr_each_len(found: *const Value) -> i64 {
    let found = at(found);
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("반복하다", found.kind()));
    }
    found.as_table().items.len() as i64
}

#[no_mangle]
pub unsafe extern "C" fn sr_index_set(
    owner: *const Value,
    place: *const Value,
    value: *const Value,
) {
    let owner = at(owner);
    let place = at(place);
    if owner.tag != TABLE {
        fail(msg::VALUE, msg::not_table("자리", owner.kind()));
    }
    if place.tag != INT {
        fail(msg::VALUE, msg::place_not_int(&to_text(place)));
    }
    let index = place.as_int();
    let held = owner.as_table();
    if index < 1 || index as usize > held.items.len() {
        fail(msg::VALUE, msg::out_of_range(index, held.items.len()));
    }
    held.items[index as usize - 1] = *at(value);
}

#[no_mangle]
pub unsafe extern "C" fn sr_table_len(found: *const Value) -> i64 {
    at(found).as_table().items.len() as i64
}

#[no_mangle]
pub unsafe extern "C" fn sr_table_get(out: *mut Value, found: *const Value, index: i64) {
    let items = &at(found).as_table().items;
    match items.get(index as usize) {
        Some(found) => *out = *found,
        // 고리가 길이를 미리 재 두므로, 도는 중에 줄면 여기로 온다.
        None => fail(msg::VALUE, msg::SHRANK.to_string()),
    }
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
    // 방향은 간격의 부호가 정한다. `1부터 0까지`는 한 번도 안 돈다.
    let by = step.number_value();
    if by == 0.0 {
        fail(msg::VALUE, msg::ZERO_STEP.to_string());
    }
    let down = by < 0.0;
    let mut made = Vec::new();
    let mut now = from;
    while if down { now >= to } else { now <= to } {
        made.push(if whole {
            Value::int(now as i64)
        } else {
            Value::float(now)
        });
        now += by;
    }
    *out = Value::items(made);
}

#[no_mangle]
pub extern "C" fn sr_bad_step() -> ! {
    fail(msg::VALUE, msg::ZERO_STEP.to_string())
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
    flush_out();
}

#[no_mangle]
pub unsafe extern "C" fn sr_clone(out: *mut Value, found: *const Value) {
    *out = deep_copy(at(found));
}

