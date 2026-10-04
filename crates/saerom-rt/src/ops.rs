use crate::msg;
use crate::text::{float_text, show, to_text, write_text};
use crate::value::*;
use crate::{fault::fail, io::flush_out};
use std::cell::UnsafeCell;
use std::collections::{HashMap, HashSet};

pub type Nouns = Option<
    unsafe extern "C" fn(
        out: *mut Value,
        owner: *const Value,
        name: *const u8,
        len: usize,
    ) -> i8,
>;

#[no_mangle]
// 글이면 메시지와 1, 수면 그 코드로 조용히.
pub unsafe extern "C" fn sr_stop(message: *const Value) {
    let found = at(message);
    if found.tag == INT {
        flush_out();
        std::process::exit(found.as_int() as i32);
    }
    fail(msg::STOP, to_text(found));
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
    *out = Value::text_of(name_of(bytes, len));
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

// 길이+1번째는 끝에 붙인다.
#[no_mangle]
pub unsafe extern "C" fn sr_insert(
    table: *const Value,
    place: *const Value,
    item: *const Value,
) {
    let found = at(table);
    let place = at(place);
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("삽입하다", found.kind()));
    }
    if place.tag != INT {
        fail(msg::VALUE, msg::place_not_int(&to_text(place)));
    }
    let index = place.as_int();
    let held = found.as_table();
    if index < 1 || index as usize > held.items.len() + 1 {
        fail(msg::VALUE, msg::out_of_range(index, held.items.len()));
    }
    held.items.insert(index as usize - 1, *at(item));
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
    copy_in(found, &mut HashMap::new())
}

// made: 원본 → 사본. 순환과 공유를 그대로 옮긴다.
fn copy_in(found: &Value, made: &mut HashMap<u64, Value>) -> Value {
    if found.tag != TABLE {
        return *found;
    }
    if let Some(copy) = made.get(&found.bits) {
        return *copy;
    }
    let copy = Value::table(Table::default());
    made.insert(found.bits, copy);
    let held = found.as_table();
    let items = held.items.iter().map(|item| copy_in(item, made)).collect();
    let keys = held
        .keys
        .iter()
        .map(|(key, value)| (*key, copy_in(value, made)))
        .collect();
    *copy.as_table() = Table::new(items, keys);
    copy
}

fn at_index(found: &Value, index: usize) -> Value {
    match found.tag {
        TABLE => found.as_table().items[index],
        _ => Value::text_of(crate::text::char_at(found.as_text(), index).expect("글자")),
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
    if field == "제곱근" && owner.number() {
        *out = Value::float(sr_root(owner.number_value()));
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
    let name = intern(key_text(at(key)));
    sr_field_set(owner, name.as_ptr(), name.len(), value);
}

// 빌린 글자를 명칭으로 쥐면 그 글자가 수집될 때 무너진다. 따로 남기되
// 같은 명칭은 한 번만 남긴다. 쓸 때마다 남기면 고리에서 샌다.
fn intern(text: &str) -> &'static str {
    struct Names(UnsafeCell<Option<HashSet<&'static str>>>);
    unsafe impl Sync for Names {}
    static NAMES: Names = Names(UnsafeCell::new(None));
    // 런타임은 홀실이라 갈래 다툼이 없다.
    let names = unsafe { &mut *NAMES.0.get() }.get_or_insert_with(HashSet::new);
    if let Some(&found) = names.get(text) {
        return found;
    }
    let made: &'static str = text.to_string().leak();
    names.insert(made);
    made
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

// 정수처럼 실수도 넘치면 멈춘다. 무한이 없으니 수가 아님도 생기지 않는다.
fn finite(verb: &str, made: f64) -> f64 {
    if !made.is_finite() {
        fail(msg::ARITH, msg::overflow(verb));
    }
    made
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
        Value::float(finite(
            verb,
            real(left.number_value(), right.number_value()),
        ))
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
    let old = text.as_str().as_ptr();
    if tail.tag == STR {
        text.push(tail.as_text());
    } else {
        let mut shown = String::new();
        write_text(&mut shown, tail);
        text.push(&shown);
    }
    crate::text::moved(old, text.as_str().as_ptr());
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
pub unsafe extern "C" fn sr_neg(out: *mut Value, found: *const Value) {
    let found = at(found);
    *out = match found.tag {
        INT => match found.as_int().checked_neg() {
            Some(made) => Value::int(made),
            None => fail(msg::ARITH, msg::overflow("-")),
        },
        FLOAT => Value::float(-found.as_float()),
        _ => fail(
            msg::VALUE,
            msg::negate_not_number(found.kind(), &show(found)),
        ),
    };
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
    *out = Value::float(finite("나누다", a / b));
}

#[no_mangle]
pub unsafe extern "C" fn sr_quot(out: *mut Value, left: *const Value, right: *const Value) {
    let (left, right) = (at(left), at(right));
    if both_int(left, right) {
        *out = Value::int(sr_quot_int(left.as_int(), right.as_int()));
        return;
    }
    let (a, b) = divisor(left, right);
    *out = Value::float(finite("나누다", a / b).floor());
}

// 나머지가 몫에 맞물리도록 내림 나눗셈을 쓴다.
#[no_mangle]
pub extern "C" fn sr_quot_int(left: i64, right: i64) -> i64 {
    if right == 0 {
        fail(msg::ARITH, msg::DIV_ZERO.to_string());
    }
    let Some(made) = left.checked_div(right) else {
        fail(msg::ARITH, msg::overflow("나누다"))
    };
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

// 정수와 실수를 견준다. 작으면 -1, 같으면 0, 크면 1.
#[no_mangle]
pub extern "C" fn sr_order_int_real(whole: i64, real: f64) -> i32 {
    crate::value::int_real(whole, real).map_or(0, |found| found as i32)
}

// 음수의 제곱근은 수가 아니다. 넘침처럼 멈춘다.
#[no_mangle]
pub extern "C" fn sr_root(found: f64) -> f64 {
    if found < 0.0 {
        fail(msg::ARITH, msg::negative_root(&float_text(found)));
    }
    found.sqrt()
}

#[no_mangle]
pub extern "C" fn sr_div_zero() -> ! {
    fail(msg::ARITH, msg::DIV_ZERO.to_string())
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
    // fmod 는 정확하다. a - b×내림(a/b) 는 몫이 넘치면 수가 아님이 된다.
    let made = left % right;
    if made != 0.0 && (made < 0.0) != (right < 0.0) {
        made + right
    } else {
        made
    }
}

#[no_mangle]
pub unsafe extern "C" fn sr_rem(out: *mut Value, left: *const Value, right: *const Value) {
    let (left, right) = (at(left), at(right));
    if both_int(left, right) {
        *out = Value::int(sr_rem_int(left.as_int(), right.as_int()));
        return;
    }
    let (a, b) = divisor(left, right);
    *out = Value::float(sr_rem_real(a, b));
}

fn order(verb: &str, left: &Value, right: &Value) -> std::cmp::Ordering {
    if left.number() && right.number() {
        if let Some(found) = crate::value::compare_numbers(left, right) {
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

// 자리만 정렬한 새 묶음. 명칭은 그대로 옮긴다.
#[no_mangle]
pub unsafe extern "C" fn sr_sort(out: *mut Value, found: *const Value) {
    let found = at(found);
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("정렬하다", found.kind()));
    }
    let held = found.as_table();
    let mut items = held.items.clone();
    items.sort_by(|left, right| order("정렬하다", left, right));
    *out = Value::table(Table::new(items, held.keys.clone()));
}

#[no_mangle]
pub unsafe extern "C" fn sr_sort_len(found: *const Value) -> i64 {
    let found = at(found);
    if found.tag != TABLE {
        fail(msg::VALUE, msg::not_table("정렬하다", found.kind()));
    }
    found.as_table().items.len() as i64
}

#[no_mangle]
pub unsafe extern "C" fn sr_sort_by(out: *mut Value, found: *const Value, keys: *const Value) {
    let held = at(found).as_table();
    let keys = &at(keys).as_table().items;
    // 기준 동사가 묶음 길이를 바꿨을 수 있다.
    if keys.len() != held.items.len() {
        fail(msg::VALUE, msg::RESIZED.to_string());
    }
    let mut pairs: Vec<(Value, Value)> = keys
        .iter()
        .copied()
        .zip(held.items.iter().copied())
        .collect();
    pairs.sort_by(|left, right| order("정렬하다", &left.0, &right.0));
    let items = pairs.into_iter().map(|(_, item)| item).collect();
    *out = Value::table(Table::new(items, held.keys.clone()));
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

// "inf", "nan" 은 수로 치지 않는다.
fn real(text: &str) -> Option<f64> {
    text.parse::<f64>().ok().filter(|number| number.is_finite())
}

// 몫처럼 내림한다. 범위를 넘으면 없음.
fn whole(number: f64) -> Option<i64> {
    let cut = number.floor();
    (cut >= i64::MIN as f64 && cut < i64::MAX as f64).then_some(cut as i64)
}

#[no_mangle]
pub unsafe extern "C" fn sr_convert(out: *mut Value, found: *const Value, kind: *const Value) {
    let found = at(found);
    let kind = to_text(at(kind));
    // 바꿀 수 없으면 없음
    let refuse = Value::nothing;
    *out = match kind.as_str() {
        "정수" | "수" => match found.tag {
            INT => *found,
            FLOAT => whole(found.as_float()).map_or_else(refuse, Value::int),
            BOOL => Value::int(i64::from(found.as_bool())),
            STR => {
                let text = found.as_text().trim();
                // 큰 정수가 실수를 거치며 뭉개지지 않게 먼저 정수로 읽는다.
                match text.parse::<i64>() {
                    Ok(number) => Value::int(number),
                    Err(_) => real(text).and_then(whole).map_or_else(refuse, Value::int),
                }
            }
            _ => refuse(),
        },
        "실수" => match found.tag {
            INT => Value::float(found.as_int() as f64),
            FLOAT => *found,
            BOOL => Value::float(f64::from(u8::from(found.as_bool()))),
            STR => real(found.as_text().trim()).map_or_else(refuse, Value::float),
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
    match found.tag {
        TABLE => found.as_table().items.len() as i64,
        STR => crate::text::char_len(found.as_text()) as i64,
        _ => fail(msg::VALUE, msg::not_table("반복하다", found.kind())),
    }
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
pub unsafe extern "C" fn sr_each_get(
    out: *mut Value,
    found: *const Value,
    index: i64,
    count: i64,
) {
    let found = at(found);
    if found.tag == STR {
        *out = at_index(found, index as usize);
        return;
    }
    sr_each_check(found, count);
    *out = found.as_table().items[index as usize];
}

// 처음 잰 길이와 다르면 줄든 늘든 멈춘다. 줄면 원소를 건너뛰고 늘면 놓친다.
#[no_mangle]
pub unsafe extern "C" fn sr_each_check(found: *const Value, count: i64) {
    let found = at(found);
    if found.tag == TABLE && found.as_table().items.len() as i64 != count {
        fail(msg::VALUE, msg::RESIZED.to_string());
    }
}

fn range_whole(start: &Value, stop: &Value, step: &Value) -> bool {
    start.tag == INT && stop.tag == INT && step.tag == INT
}

// 도는 횟수. 값은 시작 + k×간격으로 매번 구해 누적 오차가 없다.
#[no_mangle]
pub unsafe extern "C" fn sr_range_count(
    start: *const Value,
    stop: *const Value,
    step: *const Value,
) -> i64 {
    let (start, stop, step) = (at(start), at(stop), at(step));
    numbers("반복하다", [start, stop]);
    numbers("반복하다", [step, step]);
    if step.number_value() == 0.0 {
        fail(msg::VALUE, msg::ZERO_STEP.to_string());
    }
    // 방향은 간격의 부호가 정한다. `1부터 0까지`는 한 번도 안 돈다.
    if range_whole(start, stop, step) {
        let gap = i128::from(stop.as_int()) - i128::from(start.as_int());
        let by = i128::from(step.as_int());
        if gap != 0 && (gap < 0) != (by < 0) {
            return 0;
        }
        return i64::try_from(gap / by + 1).unwrap_or(i64::MAX);
    }
    let times = (stop.number_value() - start.number_value()) / step.number_value();
    // 0.1씩 더한 끝값이 오차로 빠지지 않게 조금 봐준다.
    if times.is_nan() || times < -1e-9 {
        return 0;
    }
    (times + 1e-9).floor().min(i64::MAX as f64 - 1.0) as i64 + 1
}

#[no_mangle]
pub unsafe extern "C" fn sr_range_at(
    out: *mut Value,
    start: *const Value,
    stop: *const Value,
    step: *const Value,
    index: i64,
) {
    let (start, stop, step) = (at(start), at(stop), at(step));
    *out = if range_whole(start, stop, step) {
        Value::int(start.as_int() + index * step.as_int())
    } else {
        Value::float(start.number_value() + index as f64 * step.number_value())
    };
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
