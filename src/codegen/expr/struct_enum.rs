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
        // データを持たないバリアントのみ現在サポート
        match &enum_var.fields {
            crate::ast::EnumVariantFields::Unit => {
                // バリアントのインデックスを取得
                let key = (enum_var.enum_name.clone(), enum_var.variant.clone());
                let variant_index = self.enum_variants.get(&key)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("{}::{}", enum_var.enum_name, enum_var.variant),
                        span: enum_var.span,
                    }))?;
                
                // i32の定数として返す
                Ok(self.context.i32_type().const_int(*variant_index as u64, false).into())
            }
            _ => {
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Enum variants with data not yet implemented".to_string(),
                    span: enum_var.span,
                }))
            }
        }
    }

    /// 参照式をコンパイル
    pub fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Reference expressions not yet implemented".to_string(),
            span: ref_expr.span,
        }))
    }

    /// デリファレンス式をコンパイル
    pub fn compile_dereference_expr(&mut self, deref: &DereferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Dereference expressions not yet implemented".to_string(),
            span: deref.span,
        }))
    }
}