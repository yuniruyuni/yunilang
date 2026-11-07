//! 構造体・Enumのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 構造体リテラルをコンパイル
    pub fn compile_struct_literal(&mut self, struct_lit: &StructLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 型名が指定されていない場合はエラー（現在は型推論未実装）
        let struct_name = struct_lit.name.as_ref()
            .ok_or_else(|| YuniError::Codegen(CodegenError::InvalidType {
                message: "Type inference for anonymous struct literals not yet implemented".to_string(),
                span: struct_lit.span,
            }))?;
        
        // 構造体型を取得
        let struct_type = self.type_manager.get_struct(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: struct_name.clone(),
                span: struct_lit.span,
            }))?;

        // 構造体情報を取得してクローン（借用チェッカーエラーを回避）
        let struct_info = self.struct_info.get(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_name),
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
        // Option型の特別処理
        if enum_var.enum_name == "Option" {
            match enum_var.variant.as_str() {
                "None" => {
                    // Noneは単純なnullポインタとして表現
                    match &enum_var.fields {
                        crate::ast::EnumVariantFields::Unit => {
                            Ok(self.context.ptr_type(AddressSpace::default()).const_null().into())
                        }
                        _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "None takes no arguments".to_string(),
                            span: enum_var.span,
                        }))
                    }
                }
                "Some" => {
                    // Some(value)は値へのポインタとして表現
                    match &enum_var.fields {
                        crate::ast::EnumVariantFields::Tuple(fields) if fields.len() == 1 => {
                            let value = self.compile_expression(&fields[0])?;
                            let value_type = value.get_type();
                            
                            // 値をヒープにアロケート
                            let malloc_fn = self.runtime_manager.get_function("malloc")
                                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                                    message: "malloc function not found".to_string(),
                                }))?;
                            
                            let size = self.get_size_of_type(value_type);
                            let size_value = self.context.i64_type().const_int(size, false);
                            
                            let ptr = self.builder.build_call(
                                malloc_fn,
                                &[size_value.into()],
                                "some_ptr"
                            )?.try_as_basic_value().basic().unwrap();
                            
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let typed_ptr = self.builder.build_pointer_cast(
                                ptr.into_pointer_value(),
                                ptr_type,
                                "typed_ptr"
                            )?;
                            
                            // 値を格納
                            self.builder.build_store(typed_ptr, value)?;
                            
                            Ok(typed_ptr.into())
                        }
                        _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "Some requires exactly one argument".to_string(),
                            span: enum_var.span,
                        }))
                    }
                }
                _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Unknown Option variant: {}", enum_var.variant),
                    span: enum_var.span,
                }))
            }
        }
        // Result型の特別処理
        else if enum_var.enum_name == "Result" {
            match enum_var.variant.as_str() {
                "Ok" | "Err" => {
                    match &enum_var.fields {
                        crate::ast::EnumVariantFields::Tuple(fields) if fields.len() == 1 => {
                            let value = self.compile_expression(&fields[0])?;
                            let value_type = value.get_type();
                            
                            // Result型の構造体を作成（タグ + 値）
                            let result_type = self.context.struct_type(&[
                                self.context.i32_type().into(), // タグ
                                self.context.ptr_type(AddressSpace::default()).into(), // 値へのポインタ
                            ], false);
                            
                            // 値をヒープにアロケート
                            let malloc_fn = self.runtime_manager.get_function("malloc")
                                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                                    message: "malloc function not found".to_string(),
                                }))?;
                            
                            let size = self.get_size_of_type(value_type);
                            let size_value = self.context.i64_type().const_int(size, false);
                            
                            let ptr = self.builder.build_call(
                                malloc_fn,
                                &[size_value.into()],
                                "result_value_ptr"
                            )?.try_as_basic_value().basic().unwrap();
                            
                            let ptr_type = self.context.ptr_type(AddressSpace::default());
                            let typed_ptr = self.builder.build_pointer_cast(
                                ptr.into_pointer_value(),
                                ptr_type,
                                "typed_ptr"
                            )?;
                            
                            // 値を格納
                            self.builder.build_store(typed_ptr, value)?;
                            
                            // Result構造体を作成
                            let tag_value = if enum_var.variant == "Ok" { 0 } else { 1 };
                            let tag = self.context.i32_type().const_int(tag_value, false);
                            
                            let mut result_val = result_type.get_undef();
                            result_val = self.builder.build_insert_value(result_val, tag, 0, "tag")?
                                .into_struct_value();
                            result_val = self.builder.build_insert_value(
                                result_val,
                                typed_ptr,
                                1,
                                "value"
                            )?.into_struct_value();
                            
                            Ok(result_val.into())
                        }
                        _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: format!("{} requires exactly one argument", enum_var.variant),
                            span: enum_var.span,
                        }))
                    }
                }
                _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Unknown Result variant: {}", enum_var.variant),
                    span: enum_var.span,
                }))
            }
        }
        // 通常のEnum型の処理
        else {
            // Enumのタグ値を取得
            let variant_tag = self.enum_variants.get(&(enum_var.enum_name.clone(), enum_var.variant.clone()))
                .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                    name: format!("{}::{}", enum_var.enum_name, enum_var.variant),
                    span: enum_var.span,
                }))?;
            
            // Enum型を取得（Enumはi32として表現される）
            let enum_type = self.context.i32_type();
            
            // タグ値を作成
            let tag_value = enum_type.const_int(*variant_tag as u64, false);
            
            // フィールドの処理
            match &enum_var.fields {
                EnumVariantFields::Unit => {
                    // Unitバリアントはタグ値のみ
                    Ok(tag_value.into())
                }
                EnumVariantFields::Tuple(_fields) => {
                    // TODO: タプルバリアントの実装
                    Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Enum tuple variants not yet implemented".to_string(),
                        span: enum_var.span,
                    }))
                }
                EnumVariantFields::Struct { .. } => {
                    // TODO: 構造体バリアントの実装
                    Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Enum struct variants not yet implemented".to_string(),
                        span: enum_var.span,
                    }))
                }
            }
        }
    }
}