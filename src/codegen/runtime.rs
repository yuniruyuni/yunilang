//! ランタイム関数の宣言と管理

use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use inkwell::types::FunctionType;
use inkwell::values::FunctionValue;
use inkwell::AddressSpace;
use std::collections::HashMap;

/// ランタイム関数マネージャー
pub struct RuntimeManager<'ctx> {
    context: &'ctx Context,
    /// 宣言されたランタイム関数
    functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> RuntimeManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            functions: HashMap::new(),
        }
    }
    
    /// ランタイム関数を初期化
    pub fn initialize(&mut self, module: &Module<'ctx>) {
        self.declare_standard_functions(module);
    }
    
    /// 標準的なランタイム関数を宣言
    fn declare_standard_functions(&mut self, module: &Module<'ctx>) {
        self.declare_c_standard_functions(module);
        self.declare_yuni_runtime_functions(module);
    }
    
    /// C標準ライブラリ関数を宣言
    fn declare_c_standard_functions(&mut self, module: &Module<'ctx>) {
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let void_type = self.context.void_type();
        
        // printf
        let printf_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        let printf = module.add_function("printf", printf_type, Some(Linkage::External));
        self.functions.insert("printf".to_string(), printf);
        
        // malloc
        let malloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let malloc = module.add_function("malloc", malloc_type, Some(Linkage::External));
        self.functions.insert("malloc".to_string(), malloc);
        
        // free
        let free_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let free = module.add_function("free", free_type, Some(Linkage::External));
        self.functions.insert("free".to_string(), free);
        
        // strlen
        let strlen_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
        let strlen = module.add_function("strlen", strlen_type, Some(Linkage::External));
        self.functions.insert("strlen".to_string(), strlen);
        
        // memcpy
        let memcpy_type = i8_ptr_type.fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into(), i64_type.into()],
            false,
        );
        let memcpy = module.add_function("memcpy", memcpy_type, Some(Linkage::External));
        self.functions.insert("memcpy".to_string(), memcpy);
        
        // strcmp
        let strcmp_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let strcmp = module.add_function("strcmp", strcmp_type, Some(Linkage::External));
        self.functions.insert("strcmp".to_string(), strcmp);
        
        // strcpy
        let strcpy_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let strcpy = module.add_function("strcpy", strcpy_type, Some(Linkage::External));
        self.functions.insert("strcpy".to_string(), strcpy);
    }
    
    /// Yuni固有のランタイム関数を宣言
    fn declare_yuni_runtime_functions(&mut self, module: &Module<'ctx>) {
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let i64_type = self.context.i64_type();
        let f64_type = self.context.f64_type();
        let bool_type = self.context.bool_type();
        let void_type = self.context.void_type();
        
        // 文字列連結
        let concat_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let concat = module.add_function(
            "yuni_string_concat",
            concat_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_string_concat".to_string(), concat);
        
        // 型変換関数
        // int to string (汎用整数変換)
        let int_to_string_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let int_to_string = module.add_function(
            "yuni_int_to_string",
            int_to_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_int_to_string".to_string(), int_to_string);
        
        // i64 to string
        let i64_to_string_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let i64_to_string = module.add_function(
            "yuni_i64_to_string",
            i64_to_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_i64_to_string".to_string(), i64_to_string);
        
        // float to string (汎用浮動小数点変換)
        let float_to_string_type = i8_ptr_type.fn_type(&[f64_type.into()], false);
        let float_to_string = module.add_function(
            "yuni_float_to_string",
            float_to_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_float_to_string".to_string(), float_to_string);
        
        // f64 to string
        let f64_to_string_type = i8_ptr_type.fn_type(&[f64_type.into()], false);
        let f64_to_string = module.add_function(
            "yuni_f64_to_string",
            f64_to_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_f64_to_string".to_string(), f64_to_string);
        
        // bool to string
        let bool_to_string_type = i8_ptr_type.fn_type(&[bool_type.into()], false);
        let bool_to_string = module.add_function(
            "yuni_bool_to_string",
            bool_to_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_bool_to_string".to_string(), bool_to_string);
        
        // println
        let println_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let println = module.add_function(
            "yuni_println",
            println_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_println".to_string(), println);
        
        // 文字列比較
        let string_eq_type = bool_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let string_eq = module.add_function(
            "yuni_string_eq",
            string_eq_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_string_eq".to_string(), string_eq);
        
        // エラーハンドリング
        let panic_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let panic = module.add_function(
            "yuni_panic",
            panic_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_panic".to_string(), panic);
        
        // メモリ管理
        let alloc_string_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let alloc_string = module.add_function(
            "yuni_alloc_string",
            alloc_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_alloc_string".to_string(), alloc_string);
        
        let free_string_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let free_string = module.add_function(
            "yuni_free_string",
            free_string_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_free_string".to_string(), free_string);
        
        // Vecランタイム関数
        // yuni_vec_new(element_size) -> *mut YuniVec
        let _vec_type = self.context.opaque_struct_type("YuniVec");
        let vec_ptr_type = self.context.ptr_type(AddressSpace::default());
        let vec_new_type = vec_ptr_type.fn_type(&[i64_type.into()], false);
        let vec_new = module.add_function(
            "yuni_vec_new",
            vec_new_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_vec_new".to_string(), vec_new);
        
        // yuni_vec_push(vec, element)
        let vec_push_type = void_type.fn_type(&[vec_ptr_type.into(), i8_ptr_type.into()], false);
        let vec_push = module.add_function(
            "yuni_vec_push",
            vec_push_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_vec_push".to_string(), vec_push);
        
        // yuni_vec_get(vec, index) -> *const c_void
        let vec_get_type = i8_ptr_type.fn_type(&[vec_ptr_type.into(), i64_type.into()], false);
        let vec_get = module.add_function(
            "yuni_vec_get",
            vec_get_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_vec_get".to_string(), vec_get);
        
        // yuni_vec_len(vec) -> usize
        let vec_len_type = i64_type.fn_type(&[vec_ptr_type.into()], false);
        let vec_len = module.add_function(
            "yuni_vec_len",
            vec_len_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_vec_len".to_string(), vec_len);
        
        // yuni_vec_free(vec)
        let vec_free_type = void_type.fn_type(&[vec_ptr_type.into()], false);
        let vec_free = module.add_function(
            "yuni_vec_free",
            vec_free_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_vec_free".to_string(), vec_free);
        
        // HashMapランタイム関数
        // yuni_hashmap_new(key_size, value_size) -> *mut YuniHashMap
        let _hashmap_type = self.context.opaque_struct_type("YuniHashMap");
        let hashmap_ptr_type = self.context.ptr_type(AddressSpace::default());
        let hashmap_new_type = hashmap_ptr_type.fn_type(&[i64_type.into(), i64_type.into()], false);
        let hashmap_new = module.add_function(
            "yuni_hashmap_new",
            hashmap_new_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_hashmap_new".to_string(), hashmap_new);
        
        // yuni_hashmap_insert(map, key, value)
        let hashmap_insert_type = void_type.fn_type(&[hashmap_ptr_type.into(), i8_ptr_type.into(), i8_ptr_type.into()], false);
        let hashmap_insert = module.add_function(
            "yuni_hashmap_insert",
            hashmap_insert_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_hashmap_insert".to_string(), hashmap_insert);
        
        // yuni_hashmap_get(map, key) -> *const c_void
        let hashmap_get_type = i8_ptr_type.fn_type(&[hashmap_ptr_type.into(), i8_ptr_type.into()], false);
        let hashmap_get = module.add_function(
            "yuni_hashmap_get",
            hashmap_get_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_hashmap_get".to_string(), hashmap_get);
        
        // yuni_hashmap_contains(map, key) -> bool
        let hashmap_contains_type = bool_type.fn_type(&[hashmap_ptr_type.into(), i8_ptr_type.into()], false);
        let hashmap_contains = module.add_function(
            "yuni_hashmap_contains",
            hashmap_contains_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_hashmap_contains".to_string(), hashmap_contains);
        
        // yuni_hashmap_size(map) -> usize
        let hashmap_size_type = i64_type.fn_type(&[hashmap_ptr_type.into()], false);
        let hashmap_size = module.add_function(
            "yuni_hashmap_size",
            hashmap_size_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_hashmap_size".to_string(), hashmap_size);
        
        // yuni_hashmap_free(map)
        let hashmap_free_type = void_type.fn_type(&[hashmap_ptr_type.into()], false);
        let hashmap_free = module.add_function(
            "yuni_hashmap_free",
            hashmap_free_type,
            Some(Linkage::External),
        );
        self.functions.insert("yuni_hashmap_free".to_string(), hashmap_free);
    }
    
    /// ランタイム関数を取得
    pub fn get_function(&self, name: &str) -> Option<FunctionValue<'ctx>> {
        self.functions.get(name).copied()
    }
    
    /// ランタイム関数を取得または宣言
    pub fn get_or_declare_function(&mut self, name: &str, fn_type: FunctionType<'ctx>, module: &Module<'ctx>) -> crate::error::YuniResult<FunctionValue<'ctx>> {
        if let Some(func) = self.functions.get(name) {
            Ok(*func)
        } else {
            // 関数が存在しない場合は宣言する
            let func = module.add_function(name, fn_type, Some(Linkage::External));
            self.functions.insert(name.to_string(), func);
            Ok(func)
        }
    }
    
    /// 新しいランタイム関数を追加
    #[allow(dead_code)]
    pub fn add_function(&mut self, name: String, function: FunctionValue<'ctx>) {
        self.functions.insert(name, function);
    }
}