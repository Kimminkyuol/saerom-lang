use std::io::Write;

/// # Safety
/// `ptr` 부터 `len` 바이트가 읽을 수 있어야 함
#[no_mangle]
pub unsafe extern "C" fn sr_print_bytes(ptr: *const u8, len: usize) {
    if ptr.is_null() {
        return;
    }
    let bytes = std::slice::from_raw_parts(ptr, len);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(bytes);
}
