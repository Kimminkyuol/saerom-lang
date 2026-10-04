use crate::gc::{hold, hold_text, payload};
use std::cell::UnsafeCell;
use std::cmp::Ordering;
use std::collections::HashMap;

pub unsafe fn at<'a>(found: *const Value) -> &'a Value {
    &*found
}

pub unsafe fn name_of<'a>(bytes: *const u8, len: usize) -> &'a str {
    std::str::from_utf8_unchecked(std::slice::from_raw_parts(bytes, len))
}

pub const NOTHING: u64 = 0;
pub const BOOL: u64 = 1;
pub const INT: u64 = 2;
pub const FLOAT: u64 = 3;
pub const STR: u64 = 4;
pub const TABLE: u64 = 5;

#[derive(Default)]
pub struct Table {
    pub items: Vec<Value>,
    // 순서는 출력과 명칭 순서. 바꾸는 건 put, drop_key 로만.
    pub keys: Vec<(&'static str, Value)>,
    // 명칭이 많을 때만 쓰는 자리 색인. 길이가 keys 와 다르면 다시 만든다.
    index: UnsafeCell<HashMap<&'static str, usize>>,
}

// 이보다 적으면 훑는 쪽이 빠르다.
const INDEXED: usize = 8;

impl Table {
    pub fn new(items: Vec<Value>, keys: Vec<(&'static str, Value)>) -> Table {
        Table {
            items,
            keys,
            ..Table::default()
        }
    }

    fn find(&self, key: &str) -> Option<usize> {
        if self.keys.len() <= INDEXED {
            return self.keys.iter().position(|(found, _)| same(found, key));
        }
        // 런타임은 홀실이라 갈래 다툼이 없다.
        let index = unsafe { &mut *self.index.get() };
        if index.len() != self.keys.len() {
            index.clear();
            for (at, (found, _)) in self.keys.iter().enumerate() {
                index.insert(found, at);
            }
        }
        index.get(key).copied()
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.find(key).map(|at| self.keys[at].1)
    }

    pub fn put(&mut self, key: &'static str, value: Value) {
        match self.find(key) {
            Some(at) => self.keys[at].1 = value,
            None => {
                self.keys.push((key, value));
                let index = self.index.get_mut();
                if !index.is_empty() {
                    index.insert(key, self.keys.len() - 1);
                }
            }
        }
    }

    pub fn drop_key(&mut self, key: &str) -> bool {
        match self.find(key) {
            Some(at) => {
                self.keys.remove(at);
                self.index.get_mut().clear();
                true
            }
            None => false,
        }
    }

    pub fn empty(&self) -> bool {
        self.items.is_empty() && self.keys.is_empty()
    }
}

fn same(left: &str, right: &str) -> bool {
    std::ptr::eq(left, right) || left == right
}

// 짧은 글은 객체 안에 담아 할당을 한 번으로. 길어지면 버퍼로 옮긴다.
const SHORT: usize = 22;

pub enum Text {
    Short(u8, [u8; SHORT]),
    Long(String),
}

impl Text {
    pub fn new(found: &str) -> Text {
        if found.len() > SHORT {
            return Text::Long(found.to_string());
        }
        let mut bytes = [0; SHORT];
        bytes[..found.len()].copy_from_slice(found.as_bytes());
        Text::Short(found.len() as u8, bytes)
    }

    fn owned(found: String) -> Text {
        if found.len() > SHORT {
            Text::Long(found)
        } else {
            Text::new(&found)
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            // 늘 온전한 UTF-8 조각만 담는다.
            Text::Short(len, bytes) => unsafe {
                std::str::from_utf8_unchecked(&bytes[..*len as usize])
            },
            Text::Long(found) => found,
        }
    }

    pub fn push(&mut self, tail: &str) {
        match self {
            Text::Short(len, bytes) if *len as usize + tail.len() <= SHORT => {
                let at = *len as usize;
                bytes[at..at + tail.len()].copy_from_slice(tail.as_bytes());
                *len += tail.len() as u8;
            }
            Text::Short(..) => {
                let mut long = String::with_capacity((self.as_str().len() + tail.len()) * 2);
                long.push_str(self.as_str());
                long.push_str(tail);
                *self = Text::Long(long);
            }
            Text::Long(found) => found.push_str(tail),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Value {
    pub tag: u64,
    pub bits: u64,
}

impl Value {
    pub const fn nothing() -> Value {
        Value {
            tag: NOTHING,
            bits: 0,
        }
    }

    pub fn int(found: i64) -> Value {
        Value {
            tag: INT,
            bits: found as u64,
        }
    }

    pub fn float(found: f64) -> Value {
        Value {
            tag: FLOAT,
            bits: found.to_bits(),
        }
    }

    pub fn bool(found: bool) -> Value {
        Value {
            tag: BOOL,
            bits: found as u64,
        }
    }

    pub fn text(found: String) -> Value {
        Value {
            tag: STR,
            bits: hold_text(Text::owned(found)),
        }
    }

    pub fn text_of(found: &str) -> Value {
        Value {
            tag: STR,
            bits: hold_text(Text::new(found)),
        }
    }

    pub fn table(found: Table) -> Value {
        Value {
            tag: TABLE,
            bits: hold(TABLE, UnsafeCell::new(found)),
        }
    }

    pub fn items(found: Vec<Value>) -> Value {
        Value::table(Table::new(found, Vec::new()))
    }

    pub fn as_int(&self) -> i64 {
        self.bits as i64
    }

    pub fn as_float(&self) -> f64 {
        f64::from_bits(self.bits)
    }

    pub fn as_bool(&self) -> bool {
        self.bits != 0
    }

    pub fn as_text(&self) -> &'static str {
        unsafe { (*payload::<Text>(self.bits)).as_str() }
    }

    pub fn as_string(&self) -> &'static mut Text {
        unsafe { &mut *payload::<Text>(self.bits) }
    }

    pub fn as_table(&self) -> &'static mut Table {
        unsafe { &mut *(*payload::<UnsafeCell<Table>>(self.bits)).get() }
    }

    pub fn number(&self) -> bool {
        self.tag == INT || self.tag == FLOAT
    }

    pub fn number_value(&self) -> f64 {
        if self.tag == INT {
            self.as_int() as f64
        } else {
            self.as_float()
        }
    }

    pub fn kind(&self) -> &'static str {
        match self.tag {
            BOOL => "논리값",
            INT | FLOAT => "수",
            STR => "문자열",
            TABLE => "묶음",
            _ => "값",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.tag {
            NOTHING => "없음",
            BOOL => "논리값",
            INT => "정수",
            FLOAT => "실수",
            STR => "문자열",
            TABLE => "묶음",
            _ => "값",
        }
    }

    pub fn truthy(&self) -> bool {
        match self.tag {
            NOTHING => false,
            BOOL => self.as_bool(),
            INT => self.as_int() != 0,
            FLOAT => self.as_float() != 0.0,
            STR => !self.as_text().is_empty(),
            TABLE => !self.as_table().empty(),
            _ => true,
        }
    }
}

// 정수를 실수로 바꿔 비교하면 2^53 위에서 뭉개진다. 값 그대로 견준다.
pub fn compare_numbers(left: &Value, right: &Value) -> Option<Ordering> {
    match (left.tag, right.tag) {
        (INT, INT) => Some(left.as_int().cmp(&right.as_int())),
        (INT, _) => int_real(left.as_int(), right.as_float()),
        (_, INT) => int_real(right.as_int(), left.as_float()).map(Ordering::reverse),
        _ => left.as_float().partial_cmp(&right.as_float()),
    }
}

pub fn int_real(whole: i64, real: f64) -> Option<Ordering> {
    const EDGE: f64 = 9_223_372_036_854_775_808.0;
    if real.is_nan() {
        return None;
    }
    if real >= EDGE {
        return Some(Ordering::Less);
    }
    if real < -EDGE {
        return Some(Ordering::Greater);
    }
    let cut = real.trunc();
    Some(
        whole
            .cmp(&(cut as i64))
            .then(0.0.partial_cmp(&(real - cut))?),
    )
}

pub fn equal(left: &Value, right: &Value) -> bool {
    equal_in(left, right, &mut Vec::new())
}

// open: 비교 중인 묶음 쌍. 다시 만나면 같다고 본다.
fn equal_in(left: &Value, right: &Value, open: &mut Vec<(u64, u64)>) -> bool {
    if left.number() && right.number() {
        return compare_numbers(left, right) == Some(Ordering::Equal);
    }
    if left.tag != right.tag {
        return false;
    }
    match left.tag {
        NOTHING => true,
        BOOL => left.as_bool() == right.as_bool(),
        STR => left.as_text() == right.as_text(),
        TABLE => {
            let pair = (left.bits, right.bits);
            if left.bits == right.bits || open.contains(&pair) {
                return true;
            }
            open.push(pair);
            let (a, b) = (left.as_table(), right.as_table());
            let same = a.items.len() == b.items.len()
                && a.keys.len() == b.keys.len()
                && a.items
                    .iter()
                    .zip(b.items.iter())
                    .all(|(x, y)| equal_in(x, y, open))
                && a.keys.iter().all(|(key, value)| {
                    b.get(key).is_some_and(|kept| equal_in(value, &kept, open))
                });
            open.pop();
            same
        }
        _ => left.bits == right.bits,
    }
}
