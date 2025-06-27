//! リテラル式のセマンティック解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisResult, AnalysisError};
use crate::analyzer::semantic_analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 整数リテラルの解析
    pub(crate) fn analyze_integer_literal(&self, int_lit: &IntegerLit, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        // 型サフィックスがある場合はそれを使用
        if let Some(suffix) = &int_lit.suffix {
            match suffix.as_str() {
                "i8" => Ok(Type::I8),
                "i16" => Ok(Type::I16),
                "i32" => Ok(Type::I32),
                "i64" => Ok(Type::I64),
                "i128" => Ok(Type::I128),
                "u8" => Ok(Type::U8),
                "u16" => Ok(Type::U16),
                "u32" => Ok(Type::U32),
                "u64" => Ok(Type::U64),
                "u128" => Ok(Type::U128),
                _ => Err(AnalysisError::InvalidType {
                    message: format!("Unknown integer suffix: {}", suffix),
                    span: int_lit.span,
                }),
            }
        } else if let Some(expected) = expected_type {
            // 期待される型がある場合
            match expected {
                Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
                Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 => {
                    // 値が期待される型の範囲内に収まるかチェック
                    // TODO: 範囲チェックの実装
                    Ok(expected.clone())
                }
                _ => Ok(Type::I32), // デフォルト
            }
        } else {
            // デフォルトはi32
            Ok(Type::I32)
        }
    }

    /// 浮動小数点リテラルの解析
    pub(crate) fn analyze_float_literal(&self, float_lit: &FloatLit) -> AnalysisResult<Type> {
        if let Some(suffix) = &float_lit.suffix {
            match suffix.as_str() {
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                _ => Ok(Type::F64), // デフォルト
            }
        } else {
            Ok(Type::F64) // デフォルト
        }
    }

    /// 文字列リテラルの解析
    pub(crate) fn analyze_string_literal(&self, _string_lit: &StringLit) -> AnalysisResult<Type> {
        Ok(Type::String)
    }

    /// ブール値リテラルの解析
    pub(crate) fn analyze_boolean_literal(&self, _bool_lit: &BooleanLit) -> AnalysisResult<Type> {
        Ok(Type::Bool)
    }

    /// テンプレート文字列の解析
    pub(crate) fn analyze_template_string(&mut self, template: &TemplateStringLit) -> AnalysisResult<Type> {
        // 補間される式の型をチェック
        for part in &template.parts {
            if let TemplateStringPart::Interpolation(expr) = part {
                self.analyze_expression(expr)?;
            }
        }
        
        Ok(Type::String)
    }
}