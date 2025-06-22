//! メモリ安全性のテストケース
//! 
//! このファイルはYuniコンパイラのメモリ安全性を検証するテストを含みます。

use yunilang::runtime::*;
use std::ffi::CString;

#[cfg(test)]
mod memory_safety_tests {
    use super::*;

    #[test]
    fn test_yuni_alloc_and_free() {
        // メモリ割り当てと解放のテスト
        let size = 1024_usize;
        let ptr = yuni_alloc(size);
        
        // 割り当てが成功したことを確認
        assert!(!ptr.is_null(), "メモリ割り当てが失敗しました");
        
        // メモリを安全に解放
        yuni_free(ptr, size);
        
        // 二重解放のテスト（これは実際には未定義動作だが、テストとして記録）
        // 注意: 実際のコードでは二重解放は行わない
    }

    #[test]
    fn test_zero_size_allocation() {
        // サイズ0の割り当てテスト
        let ptr = yuni_alloc(0);
        
        // サイズ0でも有効なポインタが返されることを確認
        // （実装によっては null が返される場合もある）
        
        // サイズ0の解放をテスト
        yuni_free(ptr, 0);
    }

    #[test]
    fn test_null_string_safety() {
        // null ポインタでの文字列操作の安全性テスト
        
        // null ポインタでの文字列長取得
        let len = yuni_str_len(std::ptr::null());
        assert_eq!(len, 0, "null ポインタの文字列長は0である必要があります");
        
        // null ポインタでの文字列結合
        let result = yuni_string_concat(std::ptr::null(), std::ptr::null());
        assert!(result.is_null(), "null ポインタの結合はnullを返す必要があります");
        
        let valid_str = CString::new("test").unwrap();
        let result = yuni_string_concat(valid_str.as_ptr(), std::ptr::null());
        assert!(result.is_null(), "片方がnullの場合はnullを返す必要があります");
        
        let result = yuni_string_concat(std::ptr::null(), valid_str.as_ptr());
        assert!(result.is_null(), "片方がnullの場合はnullを返す必要があります");
    }

    #[test]
    fn test_string_memory_management() {
        // 文字列のメモリ管理テスト
        let s1 = CString::new("Hello, ").unwrap();
        let s2 = CString::new("Yuni!").unwrap();
        
        let result_ptr = yuni_string_concat(s1.as_ptr(), s2.as_ptr());
        assert!(!result_ptr.is_null(), "文字列結合は成功する必要があります");
        
        // 結果を確認してからメモリを解放
        unsafe {
            // SAFETY: yuni_str_concatが返すポインタは有効なC文字列
            let result_cstr = std::ffi::CStr::from_ptr(result_ptr);
            let result_str = result_cstr.to_str().unwrap();
            assert_eq!(result_str, "Hello, Yuni!");
            
            // メモリリークを防ぐためにポインタを解放
            let _ = CString::from_raw(result_ptr);
        }
    }

    #[test]
    fn test_int_to_str_memory_management() {
        // 整数から文字列への変換のメモリ管理テスト
        let test_values = [0, 42, -123, i64::MAX, i64::MIN];
        
        for &value in &test_values {
            let ptr = yuni_i64_to_string(value);
            assert!(!ptr.is_null(), "整数から文字列への変換は成功する必要があります");
            
            unsafe {
                // SAFETY: yuni_int_to_strが返すポインタは有効なC文字列
                let c_str = std::ffi::CStr::from_ptr(ptr);
                let rust_str = c_str.to_str().unwrap();
                assert_eq!(rust_str, value.to_string());
                
                // メモリリークを防ぐためにポインタを解放
                let _ = CString::from_raw(ptr);
            }
        }
    }

    #[test]
    fn test_float_to_str_memory_management() {
        // 浮動小数点数から文字列への変換のメモリ管理テスト
        let test_values = [0.0, 3.14159, -2.71828, f64::MAX, f64::MIN];
        
        for &value in &test_values {
            let ptr = yuni_f64_to_string(value);
            assert!(!ptr.is_null(), "浮動小数点数から文字列への変換は成功する必要があります");
            
            unsafe {
                // SAFETY: yuni_float_to_strが返すポインタは有効なC文字列
                let c_str = std::ffi::CStr::from_ptr(ptr);
                let rust_str = c_str.to_str().unwrap();
                assert_eq!(rust_str, value.to_string());
                
                // メモリリークを防ぐためにポインタを解放
                let _ = CString::from_raw(ptr);
            }
        }
    }

    #[test]
    fn test_multiple_string_operations() {
        // 複数の文字列操作を組み合わせたテスト
        let s1 = CString::new("Hello").unwrap();
        let s2 = CString::new(", ").unwrap();
        let s3 = CString::new("Yuni").unwrap();
        let s4 = CString::new("!").unwrap();
        
        // 段階的な文字列結合
        let temp1 = yuni_string_concat(s1.as_ptr(), s2.as_ptr());
        assert!(!temp1.is_null());
        
        let temp2 = yuni_string_concat(temp1, s3.as_ptr());
        assert!(!temp2.is_null());
        
        let final_result = yuni_string_concat(temp2, s4.as_ptr());
        assert!(!final_result.is_null());
        
        unsafe {
            // 最終結果を確認
            let result_cstr = std::ffi::CStr::from_ptr(final_result);
            let result_str = result_cstr.to_str().unwrap();
            assert_eq!(result_str, "Hello, Yuni!");
            
            // 全てのポインタを解放
            let _ = CString::from_raw(temp1);
            let _ = CString::from_raw(temp2);
            let _ = CString::from_raw(final_result);
        }
    }
}