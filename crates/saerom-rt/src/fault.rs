//! 실행 중 오류: 자리 표시, 되짚기, 멈춤, 스택 가드.

use crate::msg;
use crate::report::{self, Report};
use crate::io::flush_out;
use crate::value::name_of;

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

// 스택 가드. 되돌이가 깊어지면 세그폴트 대신 한국어 오류로 끝낸다.
// 바닥은 main 들머리의 주소에서 예산만큼 뺀 값이다.
static mut STACK_FLOOR: usize = 0;
const STACK_BUDGET: usize = 6 << 20;

fn here() -> usize {
    let probe = 0u8;
    std::hint::black_box(&probe) as *const u8 as usize
}

#[no_mangle]
pub extern "C" fn sr_stack_base() {
    unsafe { STACK_FLOOR = here().saturating_sub(STACK_BUDGET) };
}

#[no_mangle]
pub extern "C" fn sr_stack_check() {
    if here() < unsafe { std::ptr::read(&raw const STACK_FLOOR) } {
        fail(msg::VALUE, msg::STACK_DEEP.to_string());
    }
}

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

pub fn fail(kind: &str, message: String) -> ! {
    flush_out();
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

