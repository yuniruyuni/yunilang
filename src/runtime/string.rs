//! 文字列操作関連のランタイム関数

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

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

/// String equality check
/// 
/// # Safety
/// `s1`と`s2`は有効なnull終端C文字列を指すポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_string_eq(s1: *const c_char, s2: *const c_char) -> bool {
    if s1.is_null() || s2.is_null() {
        return s1.is_null() && s2.is_null();
    }
    
    // SAFETY: 呼び出し側が有効なnull終端C文字列を提供することを前提とする
    let c_str1 = CStr::from_ptr(s1);
    let c_str2 = CStr::from_ptr(s2);
    
    c_str1 == c_str2
}

/// Free a C string
/// 
/// # Safety
/// `s`は`CString::into_raw()`により作成されたポインタである必要があります。
#[no_mangle]
pub unsafe extern "C" fn yuni_cstring_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: 呼び出し側がCString::into_raw()で作成されたポインタを
        // 提供することを前提とする
        let _ = CString::from_raw(s);
    }
}