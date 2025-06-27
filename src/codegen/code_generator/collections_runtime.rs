//! コレクション型のランタイム関数

use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::{BasicValueEnum, PointerValue, IntValue};
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::AddressSpace;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 新しいVecを作成
    pub fn create_vec_new(&mut self, element_type: BasicTypeEnum<'ctx>) -> YuniResult<PointerValue<'ctx>> {
        let vec_new_fn = self.runtime_manager.get_or_declare_function(
            "yuni_vec_new",
            self.context.ptr_type(AddressSpace::default()).fn_type(&[
                self.context.i64_type().into(),
            ], false),
            &self.module,
        )?;
        
        let element_size = self.get_size_of_type(element_type);
        let size_value = self.context.i64_type().const_int(element_size, false);
        
        let vec_ptr = self.builder.build_call(
            vec_new_fn,
            &[size_value.into()],
            "vec_new"
        )?.try_as_basic_value().left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "yuni_vec_new should return a value".to_string(),
            }))?;
        
        Ok(vec_ptr.into_pointer_value())
    }
    
    /// Vecに要素を追加
    pub fn vec_push(&mut self, vec_ptr: PointerValue<'ctx>, value: BasicValueEnum<'ctx>, element_type: BasicTypeEnum<'ctx>) -> YuniResult<()> {
        let vec_push_fn = self.runtime_manager.get_or_declare_function(
            "yuni_vec_push",
            self.context.void_type().fn_type(&[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ], false),
            &self.module,
        )?;
        
        // 値を一時的にスタックに格納
        let temp_alloca = self.builder.build_alloca(element_type, "temp_element")?;
        self.builder.build_store(temp_alloca, value)?;
        
        // yuni_vec_push を呼び出し
        self.builder.build_call(
            vec_push_fn,
            &[vec_ptr.into(), temp_alloca.into()],
            ""
        )?;
        
        Ok(())
    }
    
    /// Vecから要素を取得
    pub fn vec_get(&mut self, vec_ptr: PointerValue<'ctx>, index: IntValue<'ctx>, element_type: BasicTypeEnum<'ctx>) -> YuniResult<BasicValueEnum<'ctx>> {
        let vec_get_fn = self.runtime_manager.get_or_declare_function(
            "yuni_vec_get",
            self.context.ptr_type(AddressSpace::default()).fn_type(&[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.i64_type().into(),
            ], false),
            &self.module,
        )?;
        
        // インデックスをi64に変換
        let index_i64 = if index.get_type().get_bit_width() < 64 {
            self.builder.build_int_z_extend(index, self.context.i64_type(), "index_i64")?
        } else {
            index
        };
        
        let element_ptr = self.builder.build_call(
            vec_get_fn,
            &[vec_ptr.into(), index_i64.into()],
            "element_ptr"
        )?.try_as_basic_value().left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "yuni_vec_get should return a value".to_string(),
            }))?;
        
        // 要素の値をロード
        let typed_ptr = self.builder.build_pointer_cast(
            element_ptr.into_pointer_value(),
            element_type.ptr_type(AddressSpace::default()),
            "typed_element_ptr"
        )?;
        
        let value = self.builder.build_load(element_type, typed_ptr, "element_value")?;
        Ok(value)
    }
    
    /// Vecの長さを取得
    pub fn vec_len(&mut self, vec_ptr: PointerValue<'ctx>) -> YuniResult<IntValue<'ctx>> {
        let vec_len_fn = self.runtime_manager.get_or_declare_function(
            "yuni_vec_len",
            self.context.i64_type().fn_type(&[
                self.context.ptr_type(AddressSpace::default()).into(),
            ], false),
            &self.module,
        )?;
        
        let len = self.builder.build_call(
            vec_len_fn,
            &[vec_ptr.into()],
            "vec_len"
        )?.try_as_basic_value().left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "yuni_vec_len should return a value".to_string(),
            }))?;
        
        Ok(len.into_int_value())
    }
    
    /// 新しいHashMapを作成
    pub fn create_hashmap_new(&mut self, key_type: BasicTypeEnum<'ctx>, value_type: BasicTypeEnum<'ctx>) -> YuniResult<PointerValue<'ctx>> {
        let hashmap_new_fn = self.runtime_manager.get_or_declare_function(
            "yuni_hashmap_new",
            self.context.ptr_type(AddressSpace::default()).fn_type(&[
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ], false),
            &self.module,
        )?;
        
        let key_size = self.get_size_of_type(key_type);
        let value_size = self.get_size_of_type(value_type);
        
        let key_size_value = self.context.i64_type().const_int(key_size, false);
        let value_size_value = self.context.i64_type().const_int(value_size, false);
        
        let map_ptr = self.builder.build_call(
            hashmap_new_fn,
            &[key_size_value.into(), value_size_value.into()],
            "hashmap_new"
        )?.try_as_basic_value().left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "yuni_hashmap_new should return a value".to_string(),
            }))?;
        
        Ok(map_ptr.into_pointer_value())
    }
    
    /// HashMapに要素を挿入
    pub fn hashmap_insert(&mut self, map_ptr: PointerValue<'ctx>, key: BasicValueEnum<'ctx>, value: BasicValueEnum<'ctx>, key_type: BasicTypeEnum<'ctx>, value_type: BasicTypeEnum<'ctx>) -> YuniResult<()> {
        let hashmap_insert_fn = self.runtime_manager.get_or_declare_function(
            "yuni_hashmap_insert",
            self.context.void_type().fn_type(&[
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
                self.context.ptr_type(AddressSpace::default()).into(),
            ], false),
            &self.module,
        )?;
        
        // キーと値を一時的にスタックに格納
        let key_alloca = self.builder.build_alloca(key_type, "temp_key")?;
        let value_alloca = self.builder.build_alloca(value_type, "temp_value")?;
        
        self.builder.build_store(key_alloca, key)?;
        self.builder.build_store(value_alloca, value)?;
        
        // yuni_hashmap_insert を呼び出し
        self.builder.build_call(
            hashmap_insert_fn,
            &[map_ptr.into(), key_alloca.into(), value_alloca.into()],
            ""
        )?;
        
        Ok(())
    }
    
    /// 型のサイズを取得（バイト単位）
    pub fn get_size_of_type(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            BasicTypeEnum::ArrayType(array_ty) => {
                let element_size = self.get_size_of_type(array_ty.get_element_type());
                element_size * array_ty.len() as u64
            }
            BasicTypeEnum::FloatType(float_ty) => {
                // F32 = 4 bytes, F64 = 8 bytes
                if float_ty == self.context.f32_type() {
                    4
                } else if float_ty == self.context.f64_type() {
                    8
                } else {
                    // デフォルトでF64サイズ
                    8
                }
            }
            BasicTypeEnum::IntType(int_ty) => {
                (int_ty.get_bit_width() / 8) as u64
            }
            BasicTypeEnum::PointerType(_) => {
                // ポインタサイズ（通常8バイト）
                8
            }
            BasicTypeEnum::StructType(struct_ty) => {
                // 構造体のサイズ計算（簡易版）
                // TODO: より正確なアライメントを考慮した計算
                let mut size = 0u64;
                for i in 0..struct_ty.count_fields() {
                    if let Some(field_ty) = struct_ty.get_field_type_at_index(i) {
                        size += self.get_size_of_type(field_ty);
                    }
                }
                size
            }
            BasicTypeEnum::VectorType(vec_ty) => {
                let element_size = self.get_size_of_type(vec_ty.get_element_type());
                element_size * vec_ty.get_size() as u64
            }
        }
    }
}