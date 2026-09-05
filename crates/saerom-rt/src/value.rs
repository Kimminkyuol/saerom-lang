use crate::gc::{hold, payload};
use std::cell::UnsafeCell;

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
    pub keys: Vec<(&'static str, Value)>,
}

impl Table {
    pub fn get(&self, key: &str) -> Option<Value> {
        self.keys
            .iter()
            .find(|(found, _)| same(found, key))
            .map(|(_, value)| *value)
    }

    pub fn put(&mut self, key: &'static str, value: Value) {
        match self.keys.iter_mut().find(|(found, _)| same(found, key)) {
            Some(slot) => slot.1 = value,
            None => self.keys.push((key, value)),
        }
    }

    pub fn drop_key(&mut self, key: &str) -> bool {
        match self.keys.iter().position(|(found, _)| same(found, key)) {
            Some(at) => {
                self.keys.remove(at);
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
            bits: hold(STR, found),
        }
    }

    pub fn table(found: Table) -> Value {
        Value {
            tag: TABLE,
            bits: hold(TABLE, UnsafeCell::new(found)),
        }
    }

    pub fn items(found: Vec<Value>) -> Value {
        Value::table(Table {
            items: found,
            keys: Vec::new(),
        })
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
        unsafe { &*payload::<String>(self.bits) }
    }

    pub fn as_string(&self) -> &'static mut String {
        unsafe { &mut *payload::<String>(self.bits) }
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


pub fn equal(left: &Value, right: &Value) -> bool {
    if left.number() && right.number() {
        if left.tag == INT && right.tag == INT {
            return left.as_int() == right.as_int();
        }
        return left.number_value() == right.number_value();
    }
    if left.tag != right.tag {
        return false;
    }
    match left.tag {
        NOTHING => true,
        BOOL => left.as_bool() == right.as_bool(),
        STR => left.as_text() == right.as_text(),
        TABLE => {
            let (a, b) = (left.as_table(), right.as_table());
            a.items.len() == b.items.len()
                && a.keys.len() == b.keys.len()
                && a.items.iter().zip(b.items.iter()).all(|(x, y)| equal(x, y))
                && a.keys
                    .iter()
                    .all(|(key, value)| b.get(key).is_some_and(|kept| equal(value, &kept)))
        }
        _ => left.bits == right.bits,
    }
}
