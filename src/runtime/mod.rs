//! Runtime support module for the Yuni language.
//!
//! This module provides runtime functions and utilities that compiled Yuni programs can use.

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::alloc::{alloc, dealloc, Layout};

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

// ========== Vec ランタイム関数 ==========

/// Vec構造体の表現
#[repr(C)]
pub struct YuniVec {
    data: *mut c_void,
    len: usize,
    capacity: usize,
    element_size: usize,
}

/// 新しいVecを作成
/// 
/// # Safety
/// element_sizeは正の値である必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_vec_new(element_size: usize) -> *mut YuniVec {
    let vec = Box::new(YuniVec {
        data: ptr::null_mut(),
        len: 0,
        capacity: 0,
        element_size,
    });
    Box::into_raw(vec)
}

/// Vecに要素を追加
/// 
/// # Safety
/// - vecは有効なYuniVecポインタである必要があります
/// - elementはelement_sizeバイトの有効なメモリを指している必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_vec_push(vec: *mut YuniVec, element: *const c_void) {
    if vec.is_null() || element.is_null() {
        return;
    }
    
    let vec = &mut *vec;
    
    // 容量が足りない場合は再割り当て
    if vec.len >= vec.capacity {
        let new_capacity = if vec.capacity == 0 { 4 } else { vec.capacity * 2 };
        let new_layout = Layout::array::<u8>(vec.element_size * new_capacity).unwrap();
        
        let new_data = if vec.data.is_null() {
            alloc(new_layout)
        } else {
            let old_layout = Layout::array::<u8>(vec.element_size * vec.capacity).unwrap();
            std::alloc::realloc(vec.data as *mut u8, old_layout, new_layout.size())
        };
        
        if new_data.is_null() {
            // アロケーション失敗
            return;
        }
        
        vec.data = new_data as *mut c_void;
        vec.capacity = new_capacity;
    }
    
    // 要素をコピー
    let dst = (vec.data as *mut u8).add(vec.len * vec.element_size);
    ptr::copy_nonoverlapping(element as *const u8, dst, vec.element_size);
    vec.len += 1;
}

/// Vecの要素を取得
/// 
/// # Safety
/// - vecは有効なYuniVecポインタである必要があります
/// - indexは有効な範囲内である必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_vec_get(vec: *const YuniVec, index: usize) -> *const c_void {
    if vec.is_null() {
        return ptr::null();
    }
    
    let vec = &*vec;
    if index >= vec.len {
        return ptr::null();
    }
    
    (vec.data as *const u8).add(index * vec.element_size) as *const c_void
}

/// Vecの長さを取得
/// 
/// # Safety
/// vecは有効なYuniVecポインタである必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_vec_len(vec: *const YuniVec) -> usize {
    if vec.is_null() {
        return 0;
    }
    (*vec).len
}

/// Vecを解放
/// 
/// # Safety
/// vecは有効なYuniVecポインタである必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_vec_free(vec: *mut YuniVec) {
    if vec.is_null() {
        return;
    }
    
    let vec = Box::from_raw(vec);
    if !vec.data.is_null() && vec.capacity > 0 {
        let layout = Layout::array::<u8>(vec.element_size * vec.capacity).unwrap();
        dealloc(vec.data as *mut u8, layout);
    }
    // Boxがドロップされることで、YuniVec自体も解放される
}

// ========== HashMap ランタイム関数 ==========

/// HashMap構造体の表現（簡易実装）
#[repr(C)]
pub struct YuniHashMap {
    buckets: *mut *mut YuniHashMapBucket,
    bucket_count: usize,
    size: usize,
    key_size: usize,
    value_size: usize,
}

#[repr(C)]
struct YuniHashMapBucket {
    key: *mut c_void,
    value: *mut c_void,
    next: *mut YuniHashMapBucket,
}

/// 新しいHashMapを作成
/// 
/// # Safety
/// key_sizeとvalue_sizeは正の値である必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_hashmap_new(key_size: usize, value_size: usize) -> *mut YuniHashMap {
    let initial_capacity = 16;
    let buckets_size = std::mem::size_of::<*mut YuniHashMapBucket>() * initial_capacity;
    let buckets = alloc(Layout::from_size_align(buckets_size, 8).unwrap()) as *mut *mut YuniHashMapBucket;
    
    // バケットを初期化
    for i in 0..initial_capacity {
        *buckets.add(i) = ptr::null_mut();
    }
    
    let hashmap = Box::new(YuniHashMap {
        buckets,
        bucket_count: initial_capacity,
        size: 0,
        key_size,
        value_size,
    });
    Box::into_raw(hashmap)
}

/// 簡易ハッシュ関数
unsafe fn hash_bytes(data: *const c_void, size: usize) -> usize {
    let bytes = std::slice::from_raw_parts(data as *const u8, size);
    let mut hash = 0usize;
    for &byte in bytes {
        hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
    }
    hash
}

/// HashMapに要素を挿入
/// 
/// # Safety
/// - hashmapは有効なYuniHashMapポインタである必要があります
/// - keyとvalueは適切なサイズの有効なメモリを指している必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_hashmap_insert(hashmap: *mut YuniHashMap, key: *const c_void, value: *const c_void) {
    if hashmap.is_null() || key.is_null() || value.is_null() {
        return;
    }
    
    let hashmap = &mut *hashmap;
    let hash = hash_bytes(key, hashmap.key_size);
    let bucket_index = hash % hashmap.bucket_count;
    
    // 新しいバケットエントリを作成
    let new_bucket = alloc(Layout::new::<YuniHashMapBucket>()) as *mut YuniHashMapBucket;
    let key_mem = alloc(Layout::from_size_align(hashmap.key_size, 8).unwrap());
    let value_mem = alloc(Layout::from_size_align(hashmap.value_size, 8).unwrap());
    
    ptr::copy_nonoverlapping(key as *const u8, key_mem, hashmap.key_size);
    ptr::copy_nonoverlapping(value as *const u8, value_mem, hashmap.value_size);
    
    (*new_bucket).key = key_mem as *mut c_void;
    (*new_bucket).value = value_mem as *mut c_void;
    (*new_bucket).next = *hashmap.buckets.add(bucket_index);
    
    *hashmap.buckets.add(bucket_index) = new_bucket;
    hashmap.size += 1;
}

/// HashMapから要素を取得
/// 
/// # Safety
/// - hashmapは有効なYuniHashMapポインタである必要があります
/// - keyは適切なサイズの有効なメモリを指している必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_hashmap_get(hashmap: *const YuniHashMap, key: *const c_void) -> *const c_void {
    if hashmap.is_null() || key.is_null() {
        return ptr::null();
    }
    
    let hashmap = &*hashmap;
    let hash = hash_bytes(key, hashmap.key_size);
    let bucket_index = hash % hashmap.bucket_count;
    
    let mut current = *hashmap.buckets.add(bucket_index);
    while !current.is_null() {
        // キーを比較
        let key_bytes = std::slice::from_raw_parts(key as *const u8, hashmap.key_size);
        let bucket_key_bytes = std::slice::from_raw_parts((*current).key as *const u8, hashmap.key_size);
        
        if key_bytes == bucket_key_bytes {
            return (*current).value;
        }
        
        current = (*current).next;
    }
    
    ptr::null()
}

/// HashMapのサイズを取得
/// 
/// # Safety
/// hashmapは有効なYuniHashMapポインタである必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_hashmap_len(hashmap: *const YuniHashMap) -> usize {
    if hashmap.is_null() {
        return 0;
    }
    (*hashmap).size
}

/// HashMapを解放
/// 
/// # Safety
/// hashmapは有効なYuniHashMapポインタである必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_hashmap_free(hashmap: *mut YuniHashMap) {
    if hashmap.is_null() {
        return;
    }
    
    let hashmap = Box::from_raw(hashmap);
    
    // すべてのバケットを解放
    for i in 0..hashmap.bucket_count {
        let mut current = *hashmap.buckets.add(i);
        while !current.is_null() {
            let next = (*current).next;
            
            // キーと値のメモリを解放
            dealloc((*current).key as *mut u8, Layout::from_size_align(hashmap.key_size, 8).unwrap());
            dealloc((*current).value as *mut u8, Layout::from_size_align(hashmap.value_size, 8).unwrap());
            
            // バケット自体を解放
            dealloc(current as *mut u8, Layout::new::<YuniHashMapBucket>());
            
            current = next;
        }
    }
    
    // バケット配列を解放
    let buckets_size = std::mem::size_of::<*mut YuniHashMapBucket>() * hashmap.bucket_count;
    dealloc(hashmap.buckets as *mut u8, Layout::from_size_align(buckets_size, 8).unwrap());
    
    // YuniHashMap自体はBoxがドロップされることで解放される
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
