use std::cell::UnsafeCell;

// 힙에 올린 것마다 붙는 머리. 수집기가 이 사슬을 따라 훑는다.
#[repr(C)]
pub struct Head {
    next: *mut Head,
    kind: u64,
    mark: u64,
}

#[repr(C)]
struct Obj<T> {
    head: Head,
    payload: T,
}

struct Heap {
    first: *mut Head,
    count: usize,
    limit: usize,
}

struct Cell(UnsafeCell<Heap>);
unsafe impl Sync for Cell {}

const FIRST_LIMIT: usize = 1 << 14;

static HEAP: Cell = Cell(UnsafeCell::new(Heap {
    first: std::ptr::null_mut(),
    count: 0,
    limit: FIRST_LIMIT,
}));

// 런타임은 홀실이라 갈래 다툼이 없다.
fn heap() -> &'static mut Heap {
    unsafe { &mut *HEAP.0.get() }
}

fn hold<T>(kind: u64, payload: T) -> u64 {
    let made = Box::into_raw(Box::new(Obj {
        head: Head {
            next: std::ptr::null_mut(),
            kind,
            mark: 0,
        },
        payload,
    }));
    let heap = heap();
    unsafe { (*made).head.next = heap.first };
    heap.first = made as *mut Head;
    heap.count += 1;
    made as u64
}

fn payload<T>(bits: u64) -> *mut T {
    unsafe { &mut (*(bits as *mut Obj<T>)).payload }
}

pub fn crowded() -> bool {
    let heap = heap();
    heap.count >= heap.limit
}

// 뿌리에서 닿는 것만 남기고 나머지를 놓아준다.
pub fn collect(roots: impl Iterator<Item = *mut Value>) {
    let mut gray: Vec<Value> = Vec::new();
    for root in roots {
        gray.push(unsafe { *root });
    }
    while let Some(found) = gray.pop() {
        if found.tag != STR && found.tag != TABLE {
            continue;
        }
        let head = found.bits as *mut Head;
        unsafe {
            if (*head).mark != 0 {
                continue;
            }
            (*head).mark = 1;
        }
        if found.tag == TABLE {
            let table = found.as_table();
            gray.extend(table.items.iter().copied());
            gray.extend(table.keys.iter().map(|(_, value)| *value));
        }
    }
    let heap = heap();
    let mut link = &mut heap.first as *mut *mut Head;
    let mut kept = 0usize;
    unsafe {
        let mut here = heap.first;
        while !here.is_null() {
            let next = (*here).next;
            if (*here).mark != 0 {
                (*here).mark = 0;
                kept += 1;
                link = &mut (*here).next;
            } else {
                *link = next;
                release(here);
            }
            here = next;
        }
    }
    heap.count = kept;
    heap.limit = (kept * 2).max(FIRST_LIMIT);
    // 글자 커서는 주소로 기억하므로 자리가 바뀌면 못 믿는다.
    crate::text::forget();
}

unsafe fn release(head: *mut Head) {
    match (*head).kind {
        STR => drop(Box::from_raw(head as *mut Obj<String>)),
        _ => drop(Box::from_raw(head as *mut Obj<UnsafeCell<Table>>)),
    }
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
