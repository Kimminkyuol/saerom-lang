//! 내보내기와 파일.

use crate::msg;
use crate::value::*;
use crate::fault::fail;
use crate::text::{show, to_text, write_text};
use std::io::Write;

pub(crate) static mut OUT: String = String::new();

const OUT_LIMIT: usize = 1 << 16;

fn out_buffer() -> &'static mut String {
    unsafe { &mut *std::ptr::addr_of_mut!(OUT) }
}

pub fn flush_out() {
    let buffer = out_buffer();
    if buffer.is_empty() {
        return;
    }
    let stdout = std::io::stdout();
    let mut held = stdout.lock();
    let _ = held.write_all(buffer.as_bytes());
    let _ = held.flush();
    buffer.clear();
}

#[no_mangle]
pub unsafe extern "C" fn sr_print(found: *const Value) {
    write_text(out_buffer(), at(found));
    brim();
}

// 보간 문자열은 새 글을 짓지 않고 내보내기 통에 바로 쓴다.
#[no_mangle]
pub unsafe extern "C" fn sr_print_parts(parts: *const Value, count: usize) {
    let buffer = out_buffer();
    for index in 0..count {
        write_text(buffer, at(parts.add(index)));
    }
    brim();
}

fn brim() {
    if out_buffer().len() >= OUT_LIMIT {
        flush_out();
    }
}

fn descriptor(verb: &str, found: &Value) -> i32 {
    if found.tag != INT {
        fail(msg::VALUE, msg::not_descriptor(verb, &show(found)));
    }
    found.as_int() as i32
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
        Err(_) => Value::nothing(),
    };
}


#[no_mangle]
pub unsafe extern "C" fn sr_read(out: *mut Value, file: *const Value, count: *const Value) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;
    let fd = descriptor("읽다", at(file));
    let want = descriptor("읽다", at(count)).max(0) as usize;
    flush_out();
    let mut held = std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd));
    let mut buffer = vec![0u8; want];
    // 실패도 파일 끝도 없음이다. 빈 글과 섞이면 가릴 수 없다.
    let Ok(read) = held.read(&mut buffer) else {
        *out = Value::nothing();
        return;
    };
    if read == 0 && want > 0 {
        *out = Value::nothing();
        return;
    }
    buffer.truncate(read);
    *out = Value::text(String::from_utf8_lossy(&buffer).into_owned());
}

#[no_mangle]
pub unsafe extern "C" fn sr_write(out: *mut Value, file: *const Value, text: *const Value) {
    use std::os::unix::io::FromRawFd;
    let fd = descriptor("쓰다", at(file));
    if fd == 1 || fd == 2 {
        flush_out();
    }
    let body = to_text(at(text));
    let mut held = std::mem::ManuallyDrop::new(std::fs::File::from_raw_fd(fd));
    let written = held.write(body.as_bytes());
    let _ = held.flush();
    *out = match written {
        Ok(count) => Value::int(count as i64),
        Err(_) => Value::nothing(),
    };
}

#[no_mangle]
pub unsafe extern "C" fn sr_close(file: *const Value) {
    use std::os::unix::io::FromRawFd;
    let fd = descriptor("닫다", at(file));
    drop(std::fs::File::from_raw_fd(fd));
}
