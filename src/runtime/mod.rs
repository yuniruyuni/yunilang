//! Runtime support module for the Yuni language.
//!
//! This module provides runtime functions and utilities that compiled Yuni programs can use.

mod print;
mod memory;
mod string;
mod conversion;
mod io;
mod vec;
mod hashmap;
mod panic;

// Re-export all public functions from submodules
pub use print::*;
pub use memory::*;
pub use string::*;
pub use conversion::*;
pub use io::*;
pub use vec::*;
pub use hashmap::*;
// Don't use wildcard for panic to avoid conflicts
pub use panic::yuni_panic;

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
            let c_str = std::ffi::CStr::from_ptr(ptr);
            assert_eq!(c_str.to_str().unwrap(), "42");
            // メモリリークを防ぐためにポインタを解放
            let _ = CString::from_raw(ptr);
        }
    }
}