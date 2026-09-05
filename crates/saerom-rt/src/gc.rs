//! 표시하고 쓸기: 힙 사슬, 뿌리, 안전 지점.

use crate::value::{Table, Value, STR, TABLE};
use std::cell::UnsafeCell;

// 뿌리: 전역 자리 목록과 함수마다의 자리 목록.
static mut FIXED: (*const *mut Value, usize) = (std::ptr::null(), 0);
static mut STACK: Vec<(*const *mut Value, usize)> = Vec::new();

fn frames() -> &'static mut Vec<(*const *mut Value, usize)> {
    unsafe { &mut *std::ptr::addr_of_mut!(STACK) }
}

#[no_mangle]
pub unsafe extern "C" fn sr_roots(spots: *const *mut Value, count: usize) {
    FIXED = (spots, count);
}

#[no_mangle]
pub unsafe extern "C" fn sr_frame_push(spots: *const *mut Value, count: usize) {
    frames().push((spots, count));
}

#[no_mangle]
pub extern "C" fn sr_frame_pop() {
    frames().pop();
}

// 안전 지점. 여기서는 힙을 가리키는 값이 모두 자리 안에 있다.
#[no_mangle]
pub extern "C" fn sr_gc_point() {
    if !crowded() {
        return;
    }
    let fixed = unsafe { std::ptr::read(std::ptr::addr_of!(FIXED)) };
    let spread = |(spots, count): (*const *mut Value, usize)| {
        (0..count).map(move |at| unsafe { *spots.add(at) })
    };
    let roots = spread(fixed).chain(frames().iter().copied().flat_map(spread));
    collect(roots);
}


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

pub fn hold<T>(kind: u64, payload: T) -> u64 {
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

pub fn payload<T>(bits: u64) -> *mut T {
    unsafe { &mut (*(bits as *mut Obj<T>)).payload }
}

fn crowded() -> bool {
    let heap = heap();
    heap.count >= heap.limit
}

// 뿌리에서 닿는 것만 남기고 나머지를 놓아준다.
fn collect(roots: impl Iterator<Item = *mut Value>) {
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
