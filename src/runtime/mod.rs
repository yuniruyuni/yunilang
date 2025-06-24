//! Runtime support module for the Yuni language.
//!
//! This module provides runtime functions and utilities that compiled Yuni programs can use.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

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

/// Allocate memory
#[no_mangle]
pub extern "C" fn yuni_alloc(size: usize) -> *mut u8 {
    let mut vec = Vec::<u8>::with_capacity(size);
    let ptr = vec.as_mut_ptr();
    std::mem::forget(vec);
    ptr
}

/// Free memory
/// 
/// # Safety
/// 呼び出し側は以下を保証する必要があります：
/// - `ptr`は`yuni_alloc`により割り当てられたポインタである
/// - `size`は割り当て時と同じサイズである
/// - このポインタは一度しか解放されない
#[no_mangle]
pub unsafe extern "C" fn yuni_free(ptr: *mut u8, size: usize) {
    if !ptr.is_null() && size > 0 {
        // SAFETY: 呼び出し側がyuni_allocで割り当てられたポインタと
        // 正しいサイズを提供することを前提とする
        let _ = Vec::from_raw_parts(ptr, size, size);
    }
}

/// String concatenation
/// 
/// # Safety
/// `s1`と`s2`は有効なnull終端C文字列を指すポインタである必要があります。
/// 戻り値のポインタは呼び出し側が`yuni_free`で解放する必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_string_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    if s1.is_null() || s2.is_null() {
        return std::ptr::null_mut();
    }

    // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
    let c_str1 = CStr::from_ptr(s1);
    let c_str2 = CStr::from_ptr(s2);

    if let (Ok(str1), Ok(str2)) = (c_str1.to_str(), c_str2.to_str()) {
        let concatenated = format!("{}{}", str1, str2);
        if let Ok(c_string) = CString::new(concatenated) {
            c_string.into_raw()
        } else {
            std::ptr::null_mut()
        }
    } else {
        std::ptr::null_mut()
    }
}

/// String length
/// 
/// # Safety
/// `s`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_str_len(s: *const c_char) -> usize {
    if s.is_null() {
        return 0;
    }
    // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
    let c_str = CStr::from_ptr(s);
    c_str.to_bytes().len()
}

/// Convert integer to string (alias for compatibility)
#[no_mangle]
pub extern "C" fn yuni_int_to_string(n: i64) -> *mut c_char {
    yuni_i64_to_string(n)
}

/// Convert integer to string
#[no_mangle]
pub extern "C" fn yuni_i64_to_string(n: i64) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Convert float to string (alias for compatibility)
#[no_mangle]
pub extern "C" fn yuni_float_to_string(n: f64) -> *mut c_char {
    yuni_f64_to_string(n)
}

/// Convert float to string
#[no_mangle]
pub extern "C" fn yuni_f64_to_string(n: f64) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Convert boolean to string
#[no_mangle]
pub extern "C" fn yuni_bool_to_string(b: bool) -> *mut c_char {
    let s = if b { "true" } else { "false" };
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Allocate string with given size
#[no_mangle]
pub extern "C" fn yuni_alloc_string(size: i64) -> *mut c_char {
    if size <= 0 {
        return std::ptr::null_mut();
    }
    let mut vec = vec![0u8; size as usize + 1]; // +1 for null terminator
    let ptr = vec.as_mut_ptr() as *mut c_char;
    std::mem::forget(vec);
    ptr
}

/// Free allocated string
/// 
/// # Safety
/// `s`は`yuni_alloc_string`で割り当てられたポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_free_string(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: 呼び出し側がyuni_alloc_stringで割り当てられたポインタを
        // 提供することを前提とする
        let _ = CString::from_raw(s);
    }
}

/// Compare two strings for equality
/// 
/// # Safety
/// `s1`と`s2`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_string_eq(s1: *const c_char, s2: *const c_char) -> bool {
    if s1.is_null() && s2.is_null() {
        return true;
    }
    if s1.is_null() || s2.is_null() {
        return false;
    }
    
    // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
    let c_str1 = CStr::from_ptr(s1);
    let c_str2 = CStr::from_ptr(s2);
    
    c_str1 == c_str2
}

/// Print string with newline (wrapper for yuni_println_str)
/// 
/// # Safety
/// `s`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_println(s: *const c_char) {
    yuni_println_str(s);
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

/// Panic handler
/// 
/// # Safety
/// `msg`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_panic(msg: *const c_char) {
    if !msg.is_null() {
        // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
        let c_str = CStr::from_ptr(msg);
        if let Ok(rust_str) = c_str.to_str() {
            panic!("{}", rust_str);
        }
    }
    panic!("Unknown panic");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_string_length() {
        let s = CString::new("Hello, Yuni!").unwrap();
        let len = unsafe { yuni_str_len(s.as_ptr()) };
        assert_eq!(len, 12);
    }

    #[test]
    fn test_int_to_str() {
        let ptr = yuni_i64_to_string(42);
        assert!(!ptr.is_null());
        unsafe {
            // SAFETY: yuni_int_to_strが返すポインタは有効なC文字列
            let c_str = CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "42");
            // メモリリークを防ぐためにポインタを解放
            let _ = CString::from_raw(ptr);
        }
    }
}
