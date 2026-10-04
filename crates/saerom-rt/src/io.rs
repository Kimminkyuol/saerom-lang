//! 내보내기와 파일.

use crate::fault::fail;
use crate::msg;
use crate::text::{show, to_text, write_text};
use crate::value::*;
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

// 음수나 범위 밖은 없는 서술자. 잘라 쓰면 엉뚱한 파일(1, 2)을 가리킨다.
fn descriptor(verb: &str, found: &Value) -> Option<i32> {
    if found.tag != INT {
        fail(msg::VALUE, msg::not_descriptor(verb, &show(found)));
    }
    i32::try_from(found.as_int()).ok().filter(|fd| *fd >= 0)
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

// 실행 파일 이름은 뺀다.
#[no_mangle]
pub unsafe extern "C" fn sr_args(out: *mut Value) {
    *out = Value::items(std::env::args().skip(1).map(Value::text).collect());
}

#[no_mangle]
pub unsafe extern "C" fn sr_read(out: *mut Value, file: *const Value, count: *const Value) {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;
    let Some(fd) = descriptor("읽다", at(file)) else {
        *out = Value::nothing();
        return;
    };
    // read 는 원래 덜 읽을 수 있다. 큰 수로 메모리를 다 잡지 않게 자른다.
    let count = at(count);
    if count.tag != INT {
        fail(msg::VALUE, msg::not_descriptor("읽다", &show(count)));
    }
    let want = count.as_int().clamp(0, 1 << 20) as usize;
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
    // 글자 중간에서 끊겼으면 그 글자까지 마저 읽는다.
    for _ in 0..3 {
        let cut = std::str::from_utf8(&buffer)
            .err()
            .filter(|e| e.error_len().is_none());
        if cut.is_none() {
            break;
        }
        let mut byte = [0u8];
        match held.read(&mut byte) {
            Ok(1) => buffer.push(byte[0]),
            _ => break,
        }
    }
    *out = Value::text(String::from_utf8_lossy(&buffer).into_owned());
}

#[no_mangle]
pub unsafe extern "C" fn sr_write(out: *mut Value, file: *const Value, text: *const Value) {
    use std::os::unix::io::FromRawFd;
    let Some(fd) = descriptor("쓰다", at(file)) else {
        *out = Value::nothing();
        return;
    };
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
    if let Some(fd) = descriptor("닫다", at(file)) {
        drop(std::fs::File::from_raw_fd(fd));
    }
}
