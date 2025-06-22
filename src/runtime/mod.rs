//! Runtime support module for the Yuni language.
//!
//! This module provides runtime functions and utilities that compiled Yuni programs can use.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

/// Print a string to stdout
#[no_mangle]
pub extern "C" fn yuni_print_str(s: *const c_char) {
    unsafe {
        if !s.is_null() {
            let c_str = CStr::from_ptr(s);
            if let Ok(rust_str) = c_str.to_str() {
                print!("{}", rust_str);
            }
        }
    }
}

/// Print a string to stdout with newline
#[no_mangle]
pub extern "C" fn yuni_println_str(s: *const c_char) {
    unsafe {
        if !s.is_null() {
            let c_str = CStr::from_ptr(s);
            if let Ok(rust_str) = c_str.to_str() {
                println!("{}", rust_str);
            }
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
#[no_mangle]
pub extern "C" fn yuni_free(ptr: *mut u8, size: usize) {
    unsafe {
        if !ptr.is_null() {
            let _ = Vec::from_raw_parts(ptr, size, size);
        }
    }
}

/// String concatenation
#[no_mangle]
pub extern "C" fn yuni_str_concat(s1: *const c_char, s2: *const c_char) -> *mut c_char {
    unsafe {
        if s1.is_null() || s2.is_null() {
            return std::ptr::null_mut();
        }

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
}

/// String length
#[no_mangle]
pub extern "C" fn yuni_str_len(s: *const c_char) -> usize {
    unsafe {
        if s.is_null() {
            return 0;
        }
        let c_str = CStr::from_ptr(s);
        c_str.to_bytes().len()
    }
}

/// Convert integer to string
#[no_mangle]
pub extern "C" fn yuni_int_to_str(n: i64) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Convert float to string
#[no_mangle]
pub extern "C" fn yuni_float_to_str(n: f64) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
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
#[no_mangle]
pub extern "C" fn yuni_panic(msg: *const c_char) {
    unsafe {
        if !msg.is_null() {
            let c_str = CStr::from_ptr(msg);
            if let Ok(rust_str) = c_str.to_str() {
                panic!("{}", rust_str);
            }
        }
        panic!("Unknown panic");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_string_length() {
        let s = CString::new("Hello, Yuni!").unwrap();
        let len = yuni_str_len(s.as_ptr());
        assert_eq!(len, 12);
    }

    #[test]
    fn test_int_to_str() {
        let ptr = yuni_int_to_str(42);
        assert!(!ptr.is_null());
        unsafe {
            let c_str = CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "42");
            // Clean up
            let _ = CString::from_raw(ptr);
        }
    }
}
