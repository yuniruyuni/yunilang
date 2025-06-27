//! メモリ管理関連のランタイム関数

use std::alloc::{alloc, dealloc, Layout};

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

/// Memory allocation wrapper for malloc compatibility
#[no_mangle]
pub unsafe extern "C" fn yuni_malloc(size: usize) -> *mut std::os::raw::c_void {
    if size == 0 {
        return std::ptr::null_mut();
    }
    
    let layout = match Layout::from_size_align(size, std::mem::align_of::<usize>()) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };
    
    // SAFETY: layoutは上で検証済み
    let ptr = alloc(layout);
    if ptr.is_null() {
        std::ptr::null_mut()
    } else {
        ptr as *mut std::os::raw::c_void
    }
}

/// Memory deallocation wrapper for free compatibility
/// 
/// # Safety
/// 呼び出し側は以下を保証する必要があります：
/// - `ptr`は`yuni_malloc`により割り当てられたポインタである
/// - `size`は割り当て時と同じサイズである
/// - このポインタは一度しか解放されない
#[no_mangle]
pub unsafe extern "C" fn yuni_mfree(ptr: *mut std::os::raw::c_void, size: usize) {
    if !ptr.is_null() && size > 0 {
        let layout = match Layout::from_size_align(size, std::mem::align_of::<usize>()) {
            Ok(layout) => layout,
            Err(_) => return,
        };
        
        // SAFETY: 呼び出し側がyuni_mallocで割り当てられたポインタと
        // 正しいサイズを提供することを前提とする
        dealloc(ptr as *mut u8, layout);
    }
}