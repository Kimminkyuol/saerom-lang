use std::cell::RefCell;

pub const NOTHING: u64 = 0;
pub const BOOL: u64 = 1;
pub const INT: u64 = 2;
pub const FLOAT: u64 = 3;
pub const STR: u64 = 4;
pub const LIST: u64 = 5;
pub const DICT: u64 = 6;

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
            bits: leak(found),
        }
    }

    pub fn list(found: Vec<Value>) -> Value {
        Value {
            tag: LIST,
            bits: leak(RefCell::new(found)),
        }
    }

    pub fn dict(found: Vec<(String, Value)>) -> Value {
        Value {
            tag: DICT,
            bits: leak(RefCell::new(found)),
        }
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
        unsafe { &*(self.bits as *const String) }
    }

    pub fn as_list(&self) -> &'static RefCell<Vec<Value>> {
        unsafe { &*(self.bits as *const RefCell<Vec<Value>>) }
    }

    pub fn as_dict(&self) -> &'static RefCell<Vec<(String, Value)>> {
        unsafe { &*(self.bits as *const RefCell<Vec<(String, Value)>>) }
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
            LIST => "목록",
            DICT => "사전",
            _ => "값",
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self.tag {
            BOOL => "논리값",
            INT => "정수",
            FLOAT => "실수",
            STR => "문자열",
            LIST => "목록",
            DICT => "사전",
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
            LIST => !self.as_list().borrow().is_empty(),
            DICT => !self.as_dict().borrow().is_empty(),
            _ => true,
        }
    }
}

fn leak<T>(found: T) -> u64 {
    Box::into_raw(Box::new(found)) as u64
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
        LIST => {
            let (a, b) = (left.as_list().borrow(), right.as_list().borrow());
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| equal(x, y))
        }
        DICT => {
            let (a, b) = (left.as_dict().borrow(), right.as_dict().borrow());
            a.len() == b.len()
                && a.iter().all(|(key, value)| {
                    b.iter()
                        .any(|(other, kept)| other == key && equal(value, kept))
                })
        }
        _ => left.bits == right.bits,
    }
}
