//! 入出力関連のランタイム関数

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

/// Print string with newline (wrapper for yuni_println_str)
/// 
/// # Safety
/// `s`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_println(s: *const c_char) {
    super::print::yuni_println_str(s);
}

/// Read line from stdin
#[no_mangle]
pub extern "C" fn yuni_read_line() -> *mut c_char {
    use std::io::{self, BufRead};

    let stdin = io::stdin();
    let mut line = String::new();

    if stdin.lock().read_line(&mut line).is_ok() {
        // Remove trailing newline
        if line.ends_with('\n') {
            line.pop();
            if line.ends_with('\r') {
                line.pop();
            }
        }

        if let Ok(c_string) = CString::new(line) {
            return c_string.into_raw();
        }
    }

    std::ptr::null_mut()
}

/// Exit the program
#[no_mangle]
pub extern "C" fn yuni_exit(code: c_int) {
    std::process::exit(code);
}