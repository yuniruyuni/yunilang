//! 構造体・Enumのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 構造体リテラルをコンパイル
    pub fn compile_struct_literal(&mut self, struct_lit: &StructLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 構造体型を取得
        let struct_type = self.type_manager.get_struct(&struct_lit.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: struct_lit.name.clone(),
                span: struct_lit.span,
            }))?;

        // 構造体情報を取得してクローン（借用チェッカーエラーを回避）
        let struct_info = self.struct_info.get(&struct_lit.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_lit.name),
            }))?
            .clone();

        // 各フィールドの値をコンパイル
        let mut field_values = vec![];
        for (index, field_type) in struct_info.field_types.iter().enumerate() {
            // フィールド名を取得
            let field_name = struct_info.field_indices.iter()
                .find(|(_, &idx)| idx == index as u32)
                .map(|(name, _)| name.clone())
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: format!("Field name not found for index {}", index),
                }))?;

            // 初期化されたフィールドを探す
            let field_init = struct_lit.fields.iter()
                .find(|f| f.name == field_name);

            let value = if let Some(init) = field_init {
                // フィールドが明示的に初期化されている場合
                self.compile_expression(&init.value)?
            } else {
                // フィールドが初期化されていない場合はデフォルト値を使用
                self.type_manager.create_default_value(field_type)?
            };

            field_values.push(value);
        }

        // 構造体値を作成
        // 動的な値を含む構造体の場合は、build_insert_valueを使用して構築
        let struct_val = struct_type.get_undef();
        let mut result = struct_val;
        
        for (i, field_value) in field_values.iter().enumerate() {
            result = self.builder.build_insert_value(result, *field_value, i as u32, &format!("field_{}", i))?
                .into_struct_value();
        }
        
        Ok(result.into())
    }

    /// 列挙型バリアントをコンパイル
    pub fn compile_enum_variant(&mut self, enum_var: &EnumVariantExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // バリアントのインデックスを取得
        let key = (enum_var.enum_name.clone(), enum_var.variant.clone());
        let variant_index = self.enum_variants.get(&key)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("{}::{}", enum_var.enum_name, enum_var.variant),
                span: enum_var.span,
            }))?;
        
        match &enum_var.fields {
            crate::ast::EnumVariantFields::Unit => {
                // i32の定数として返す
                Ok(self.context.i32_type().const_int(*variant_index as u64, false).into())
            }
            crate::ast::EnumVariantFields::Tuple(fields) => {
                // タプル形式のデータを持つバリアント
                // 構造: { discriminant: i32, data: tuple }
                let discriminant = self.context.i32_type().const_int(*variant_index as u64, false);
                
                // フィールドの値をコンパイル
                let mut field_values = vec![];
                for field in fields {
                    field_values.push(self.compile_expression(field)?);
                }
                
                // データタプルを作成
                let data_tuple = self.context.const_struct(&field_values, false);
                
                // Enum構造体を作成 { discriminant, data }
                let enum_struct = self.context.struct_type(&[
                    discriminant.get_type().into(),
                    data_tuple.get_type().into(),
                ], false);
                
                let mut enum_value = enum_struct.get_undef();
                enum_value = self.builder.build_insert_value(enum_value, discriminant, 0, "enum_discriminant")?
                    .into_struct_value();
                enum_value = self.builder.build_insert_value(enum_value, data_tuple, 1, "enum_data")?
                    .into_struct_value();
                
                Ok(enum_value.into())
            }
            crate::ast::EnumVariantFields::Struct(fields) => {
                // 構造体形式のデータを持つバリアント
                // 構造: { discriminant: i32, data: struct }
                let discriminant = self.context.i32_type().const_int(*variant_index as u64, false);
                
                // フィールドの値をコンパイル
                let mut field_values = vec![];
                for init in fields {
                    field_values.push(self.compile_expression(&init.value)?);
                }
                
                // データ構造体を作成
                let data_struct = self.context.const_struct(&field_values, false);
                
                // Enum構造体を作成 { discriminant, data }
                let enum_struct = self.context.struct_type(&[
                    discriminant.get_type().into(),
                    data_struct.get_type().into(),
                ], false);
                
                let mut enum_value = enum_struct.get_undef();
                enum_value = self.builder.build_insert_value(enum_value, discriminant, 0, "enum_discriminant")?
                    .into_struct_value();
                enum_value = self.builder.build_insert_value(enum_value, data_struct, 1, "enum_data")?
                    .into_struct_value();
                
                Ok(enum_value.into())
            }
        }
    }

    /// 参照式をコンパイル
    pub fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 参照式は内部式のアドレスを返す
        match &*ref_expr.expr {
            Expression::Identifier(id) => {
                // 変数への参照の場合、そのポインタを直接返す
                let symbol = self.scope_manager.lookup(&id.name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))?;
                
                Ok(symbol.ptr.into())
            }
            Expression::Field(field_expr) => {
                // フィールドへの参照の場合
                self.compile_field_reference(field_expr)
            }
            Expression::Index(index_expr) => {
                // 配列要素への参照の場合
                self.compile_index_reference(index_expr)
            }
            _ => {
                // その他の式への参照は現在未サポート
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: format!("Reference to {:?} expressions not yet implemented", ref_expr.expr),
                    span: ref_expr.span,
                }))
            }
        }
    }

    /// デリファレンス式をコンパイル
    pub fn compile_dereference_expr(&mut self, deref: &DereferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 内部式をコンパイルしてポインタを取得
        let ptr_value = self.compile_expression(&deref.expr)?;
        
        // ポインタ型であることを確認
        let ptr = ptr_value.into_pointer_value();
        
        // ポインタが指す型を推論
        let inner_type = match self.expression_type(&deref.expr)? {
            Type::Reference(inner, _is_mut) => *inner,
            _ => {
                return Err(YuniError::Codegen(CodegenError::TypeError {
                    expected: "reference type".to_string(),
                    actual: format!("{:?}", self.expression_type(&deref.expr)?),
                    span: deref.span,
                }));
            }
        };
        
        // LLVMの型に変換
        let llvm_type = self.type_manager.ast_type_to_llvm(&inner_type)?;
        
        // ポインタから値をロード
        let value = self.builder.build_load(
            llvm_type,
            ptr,
            "deref_value",
        )?;
        
        Ok(value)
    }
    
    /// フィールドへの参照を取得
    fn compile_field_reference(&mut self, field: &FieldExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
    fn compile_index_reference(&mut self, index: &IndexExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
            _ => {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Cannot take reference to index of type: {:?}", object_type),
                    span: index.span,
                }))
            }
        }
    }
}