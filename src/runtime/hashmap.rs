//! HashMap型のランタイム関数

use std::os::raw::c_void;
use std::ptr;
use std::alloc::{alloc, dealloc, Layout};

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
/// - 戻り値のポインタはhashmapが生きている間のみ有効です
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
        let bucket = &*current;
        
        // キーを比較
        let key_bytes = std::slice::from_raw_parts(key as *const u8, hashmap.key_size);
        let bucket_key_bytes = std::slice::from_raw_parts(bucket.key as *const u8, hashmap.key_size);
        
        if key_bytes == bucket_key_bytes {
            return bucket.value;
        }
        
        current = bucket.next;
    }
    
    ptr::null()
}

/// HashMapの要素数を取得
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
/// - hashmapは有効なYuniHashMapポインタである必要があります
/// - hashmapは一度しか解放されない必要があります
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
            let bucket = Box::from_raw(current);
            current = bucket.next;
            
            // キーと値のメモリを解放
            dealloc(bucket.key as *mut u8, Layout::from_size_align_unchecked(hashmap.key_size, 8));
            dealloc(bucket.value as *mut u8, Layout::from_size_align_unchecked(hashmap.value_size, 8));
        }
    }
    
    // バケット配列を解放
    let buckets_size = std::mem::size_of::<*mut YuniHashMapBucket>() * hashmap.bucket_count;
    dealloc(hashmap.buckets as *mut u8, Layout::from_size_align_unchecked(buckets_size, 8));
}