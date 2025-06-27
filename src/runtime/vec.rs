//! Vec型のランタイム関数

use std::os::raw::c_void;
use std::ptr;
use std::alloc::{alloc, dealloc, realloc, Layout};

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
        let new_size = new_capacity * vec.element_size;
        
        let layout = if vec.capacity == 0 {
            Layout::from_size_align_unchecked(new_size, std::mem::align_of::<usize>())
        } else {
            Layout::from_size_align_unchecked(vec.capacity * vec.element_size, std::mem::align_of::<usize>())
        };
        
        if vec.capacity == 0 {
            vec.data = alloc(Layout::from_size_align_unchecked(new_size, std::mem::align_of::<usize>())) as *mut c_void;
        } else {
            vec.data = realloc(vec.data as *mut u8, layout, new_size) as *mut c_void;
        }
        
        vec.capacity = new_capacity;
    }
    
    // 要素をコピー
    let dst = (vec.data as *mut u8).add(vec.len * vec.element_size);
    ptr::copy_nonoverlapping(element as *const u8, dst, vec.element_size);
    vec.len += 1;
}

/// Vecから要素を取得
/// 
/// # Safety
/// - vecは有効なYuniVecポインタである必要があります
/// - indexは有効な範囲内である必要があります
/// - 戻り値のポインタはvecが生きている間のみ有効です
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
/// - vecは有効なYuniVecポインタである必要があります
/// - vecは一度しか解放されない必要があります
#[no_mangle]
pub unsafe extern "C" fn yuni_vec_free(vec: *mut YuniVec) {
    if vec.is_null() {
        return;
    }
    
    let vec = Box::from_raw(vec);
    if !vec.data.is_null() && vec.capacity > 0 {
        let layout = Layout::from_size_align_unchecked(
            vec.capacity * vec.element_size,
            std::mem::align_of::<usize>()
        );
        dealloc(vec.data as *mut u8, layout);
    }
}