//! 型変換関連のランタイム関数

use std::ffi::CString;
use std::os::raw::c_char;

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

/// Convert i8 to string
#[no_mangle]
pub extern "C" fn yuni_i8_to_string(n: i8) -> *mut c_char {
    yuni_i64_to_string(n as i64)
}

/// Convert i16 to string
#[no_mangle]
pub extern "C" fn yuni_i16_to_string(n: i16) -> *mut c_char {
    yuni_i64_to_string(n as i64)
}

/// Convert i32 to string
#[no_mangle]
pub extern "C" fn yuni_i32_to_string(n: i32) -> *mut c_char {
    yuni_i64_to_string(n as i64)
}

/// Convert i128 to string
#[no_mangle]
pub extern "C" fn yuni_i128_to_string(n: i128) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Convert u8 to string
#[no_mangle]
pub extern "C" fn yuni_u8_to_string(n: u8) -> *mut c_char {
    yuni_u64_to_string(n as u64)
}

/// Convert u16 to string
#[no_mangle]
pub extern "C" fn yuni_u16_to_string(n: u16) -> *mut c_char {
    yuni_u64_to_string(n as u64)
}

/// Convert u32 to string
#[no_mangle]
pub extern "C" fn yuni_u32_to_string(n: u32) -> *mut c_char {
    yuni_u64_to_string(n as u64)
}

/// Convert u64 to string
#[no_mangle]
pub extern "C" fn yuni_u64_to_string(n: u64) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Convert u128 to string
#[no_mangle]
pub extern "C" fn yuni_u128_to_string(n: u128) -> *mut c_char {
    let s = n.to_string();
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}

/// Convert f32 to string
#[no_mangle]
pub extern "C" fn yuni_f32_to_string(n: f32) -> *mut c_char {
    yuni_f64_to_string(n as f64)
}

/// Convert bool to string
#[no_mangle]
pub extern "C" fn yuni_bool_to_string(b: bool) -> *mut c_char {
    let s = if b { "true" } else { "false" };
    if let Ok(c_string) = CString::new(s) {
        c_string.into_raw()
    } else {
        std::ptr::null_mut()
    }
}