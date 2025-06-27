//! パニック処理関連のランタイム関数

use std::ffi::CStr;
use std::os::raw::c_char;

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