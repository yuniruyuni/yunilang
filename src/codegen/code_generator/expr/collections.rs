//! コレクション型（リスト、マップ）のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// リストリテラルをコンパイル
    pub fn compile_list_literal(&mut self, list: &ListLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &list.type_name {
            if type_name == "Vec" && type_args.len() == 1 {
                let element_type = &type_args[0];
                let llvm_element_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // Vecの作成
                let vec_ptr = self.create_vec_new(llvm_element_type)?;
                
                // 各要素を追加
                for elem in &list.elements {
                    let value = self.compile_expression(elem)?;
                    self.vec_push(vec_ptr, value, llvm_element_type)?;
                }
                
                Ok(vec_ptr.into())
            } else {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Unknown list type: {}", type_name),
                    span: list.span,
                }))
            }
        } else {
            // 型名が省略されている場合、デフォルトでVecとして扱う
            if list.elements.is_empty() {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Cannot infer type for empty list literal".to_string(),
                    span: list.span,
                }))
            } else {
                // 最初の要素の型から推論
                let first_elem = self.compile_expression(&list.elements[0])?;
                let element_type = first_elem.get_type();
                
                // Vecの作成
                let vec_ptr = self.create_vec_new(element_type)?;
                
                // 最初の要素を追加
                self.vec_push(vec_ptr, first_elem, element_type)?;
                
                // 残りの要素を追加
                for elem in &list.elements[1..] {
                    let value = self.compile_expression(elem)?;
                    self.vec_push(vec_ptr, value, element_type)?;
                }
                
                Ok(vec_ptr.into())
            }
        }
    }
    
    /// マップリテラルをコンパイル
    pub fn compile_map_literal(&mut self, map: &MapLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &map.type_name {
            if type_name == "HashMap" && type_args.len() == 2 {
                let key_type = &type_args[0];
                let value_type = &type_args[1];
                let llvm_key_type = self.type_manager.ast_type_to_llvm(key_type)?;
                let llvm_value_type = self.type_manager.ast_type_to_llvm(value_type)?;
                
                // HashMapの作成
                let map_ptr = self.create_hashmap_new(llvm_key_type, llvm_value_type)?;
                
                // 各ペアを挿入
                for (key_expr, value_expr) in &map.pairs {
                    let key = self.compile_expression(key_expr)?;
                    let value = self.compile_expression(value_expr)?;
                    self.hashmap_insert(map_ptr, key, value, llvm_key_type, llvm_value_type)?;
                }
                
                Ok(map_ptr.into())
            } else {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Unknown map type: {}", type_name),
                    span: map.span,
                }))
            }
        } else {
            // 型名が省略されている場合、デフォルトでHashMapとして扱う
            if map.pairs.is_empty() {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Cannot infer type for empty map literal".to_string(),
                    span: map.span,
                }))
            } else {
                // 最初のペアの型から推論
                let (first_key_expr, first_value_expr) = &map.pairs[0];
                let first_key = self.compile_expression(first_key_expr)?;
                let first_value = self.compile_expression(first_value_expr)?;
                let key_type = first_key.get_type();
                let value_type = first_value.get_type();
                
                // HashMapの作成
                let map_ptr = self.create_hashmap_new(key_type, value_type)?;
                
                // 最初のペアを挿入
                self.hashmap_insert(map_ptr, first_key, first_value, key_type, value_type)?;
                
                // 残りのペアを挿入
                for (key_expr, value_expr) in &map.pairs[1..] {
                    let key = self.compile_expression(key_expr)?;
                    let value = self.compile_expression(value_expr)?;
                    self.hashmap_insert(map_ptr, key, value, key_type, value_type)?;
                }
                
                Ok(map_ptr.into())
            }
        }
    }
}