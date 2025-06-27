//! 出力関連のランタイム関数

use std::ffi::CStr;
use std::os::raw::c_char;

/// Print a string to stdout
/// 
/// # Safety
/// `s`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_print_str(s: *const c_char) {
    if !s.is_null() {
        // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
        let c_str = CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            print!("{}", rust_str);
        }
    }
}

/// Print a string to stdout with newline
/// 
/// # Safety
/// `s`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_println_str(s: *const c_char) {
    if !s.is_null() {
        // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
        let c_str = CStr::from_ptr(s);
        if let Ok(rust_str) = c_str.to_str() {
            println!("{}", rust_str);
        }
    }
}

/// Print an integer to stdout
#[no_mangle]
pub extern "C" fn yuni_print_int(n: i64) {
    print!("{}", n);
}

/// Print an integer to stdout with newline
#[no_mangle]
pub extern "C" fn yuni_println_int(n: i64) {
    println!("{}", n);
}

/// Print a float to stdout
#[no_mangle]
pub extern "C" fn yuni_print_float(n: f64) {
    print!("{}", n);
}

/// Print a float to stdout with newline
#[no_mangle]
pub extern "C" fn yuni_println_float(n: f64) {
    println!("{}", n);
}