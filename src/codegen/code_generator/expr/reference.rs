//! 参照と参照外しのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 参照式をコンパイル
    pub fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        match ref_expr.expr.as_ref() {
            // フィールドへの参照
            Expression::Field(field) => self.compile_field_reference(field),
            
            // インデックスへの参照
            Expression::Index(index) => self.compile_index_reference(index),
            
            // それ以外の式への参照
            _ => {
                // 式を評価
                let value = self.compile_expression(&ref_expr.expr)?;
                
                // 値をメモリに格納してポインタを返す
                let value_type = value.get_type();
                let alloca = self.builder.build_alloca(value_type, "ref_tmp")?;
                self.builder.build_store(alloca, value)?;
                
                Ok(alloca.into())
            }
        }
    }

    /// 参照外し式をコンパイル
    pub fn compile_dereference_expr(&mut self, deref: &DereferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 参照式をコンパイル（ポインタが返される）
        let ptr_value = self.compile_expression(&deref.expr)?;
        
        // ポインタから値をロード
        if let BasicValueEnum::PointerValue(ptr) = ptr_value {
            // 参照外しする型を取得
            let expr_type = self.expression_type(&deref.expr)?;
            if let Type::Reference(inner_type, _is_mut) = expr_type {
                let llvm_type = self.type_manager.ast_type_to_llvm(&inner_type)?;
                let value = self.builder.build_load(llvm_type, ptr, "deref")?;
                Ok(value)
            } else {
                Err(YuniError::Codegen(CodegenError::TypeError {
                    expected: "reference type".to_string(),
                    actual: format!("{:?}", expr_type),
                    span: deref.span,
                }))
            }
        } else {
            Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Expected pointer value for dereference".to_string(),
                span: deref.span,
            }))
        }
    }

    /// フィールドへの参照を取得
    pub(crate) fn compile_field_reference(&mut self, field: &FieldExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // オブジェクトの式をコンパイル
        let object_value = self.compile_expression(&field.object)?;
        
        // オブジェクトの型を推論
        let object_type = self.expression_type(&field.object)?;
        
        // 構造体名を取得
        let struct_name = match &object_type {
            Type::UserDefined(name) => name.clone(),
            Type::Reference(inner, _is_mut) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name.clone()
                } else {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Field access on non-struct type".to_string(),
                        span: field.span,
                    }));
                }
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Field access on non-struct type".to_string(),
                    span: field.span,
                }));
            }
        };
        
        // 構造体情報を取得
        let struct_info = self.struct_info.get(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_name),
            }))?;
        
        // フィールドのインデックスを取得
        let field_index = struct_info.get_field_index(&field.field)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("{}.{}", struct_name, field.field),
                span: field.span,
            }))?;
        
        // 構造体へのポインタを取得
        let struct_ptr = match object_value {
            BasicValueEnum::StructValue(_) => {
                // 構造体値の場合、変数として格納されている必要がある
                // TODO: 一時変数に格納してポインタを取得
                return Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Reference to temporary struct field not yet implemented".to_string(),
                    span: field.span,
                }));
            }
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Expected struct or pointer to struct".to_string(),
                    span: field.span,
                }));
            }
        };
        
        // 構造体型を取得
        let struct_type = self.type_manager.get_struct(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct type not found for {}", struct_name),
            }))?;
        
        // フィールドへのポインタを計算（GEP）
        let field_ptr = unsafe {
            self.builder.build_gep(
                struct_type,
                struct_ptr,
                &[
                    self.context.i32_type().const_zero(),
                    self.context.i32_type().const_int(field_index as u64, false)
                ],
                &format!("{}_ptr", field.field)
            )?
        };
        
        Ok(field_ptr.into())
    }
    
    /// インデックスへの参照を取得
    pub(crate) fn compile_index_reference(&mut self, index: &IndexExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // オブジェクト（配列）の式をコンパイル
        let object_value = self.compile_expression(&index.object)?;
        
        // インデックスの式をコンパイル
        let index_value = self.compile_expression(&index.index)?;
        
        // オブジェクトの型を推論
        let object_type = self.expression_type(&index.object)?;
        
        match &object_type {
            Type::Array(element_type) => {
                // 配列のインデックスアクセス
                let array_ptr = object_value.into_pointer_value();
                
                // インデックスが整数型であることを確認
                let index_int = index_value.into_int_value();
                
                // 要素のLLVM型を取得
                let element_llvm_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // GEPで要素のアドレスを計算（参照として返す）
                let element_ptr = unsafe {
                    self.builder.build_gep(
                        element_llvm_type,
                        array_ptr,
                        &[index_int],
                        "element_ref"
                    )?
                };
                
                Ok(element_ptr.into())
            }
            Type::Generic(name, type_args) if name == "Vec" && type_args.len() == 1 => {
                // Vecのインデックスアクセス
                // Vecの場合、要素は動的にヒープに格納されているため、
                // 直接的な参照を取ることができない
                // 一時的な解決策として、要素の値を一時変数に格納してその参照を返す
                let element_type = &type_args[0];
                let vec_ptr = object_value.into_pointer_value();
                let index_int = index_value.into_int_value();
                let element_llvm_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // vec_getを使用して要素を取得
                let value = self.vec_get(vec_ptr, index_int, element_llvm_type)?;
                
                // 一時変数に格納
                let temp_alloca = self.builder.build_alloca(element_llvm_type, "temp_vec_elem")?;
                self.builder.build_store(temp_alloca, value)?;
                
                Ok(temp_alloca.into())
            }
            _ => {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Cannot take reference to index of type: {:?}", object_type),
                    span: index.span,
                }))
            }
        }
    }
}