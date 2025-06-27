//! 型の互換性チェック

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisResult, AnalysisError};
use crate::analyzer::type_checker::TypeChecker;
use crate::analyzer::symbol::TypeKind;

impl TypeChecker {
    /// 型の互換性をチェック
    pub fn check_type_compatibility(&self, expected: &Type, actual: &Type, span: Span) -> AnalysisResult<()> {
        // 型変数の場合は常に互換性あり（後で具体的な型に置き換えられる）
        if matches!(expected, Type::Variable(_)) {
            return Ok(());
        }
        
        if self.types_compatible(expected, actual) {
            Ok(())
        } else {
            Err(AnalysisError::TypeMismatch {
                expected: self.type_to_string(expected),
                found: self.type_to_string(actual),
                span,
            })
        }
    }

    /// 型が互換性があるか判定（内部使用）
    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // 完全一致
            (t1, t2) if t1 == t2 => true,
            
            // 参照型の互換性
            (Type::Reference(inner1, mut1), Type::Reference(inner2, mut2)) => {
                // ミュータブル参照からイミュータブル参照への変換は可能
                (!*mut1 || *mut2) && self.types_compatible(inner1, inner2)
            }
            
            // 配列型の互換性
            (Type::Array(elem1), Type::Array(elem2)) => {
                self.types_compatible(elem1, elem2)
            }
            
            // タプル型の互換性
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                types1.len() == types2.len() &&
                types1.iter().zip(types2.iter()).all(|(t1, t2)| self.types_compatible(t1, t2))
            }
            
            // ジェネリック型の互換性
            (Type::Generic(name1, args1), Type::Generic(name2, args2)) => {
                name1 == name2 && args1.len() == args2.len() &&
                args1.iter().zip(args2.iter()).all(|(t1, t2)| self.types_compatible(t1, t2))
            }
            
            _ => false,
        }
    }

    /// 型を検証（存在確認など）
    pub fn validate_type(&self, ty: &Type, span: Span) -> AnalysisResult<()> {
        match ty {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 |
            Type::F8 | Type::F16 | Type::F32 | Type::F64 | Type::Bool | Type::String | Type::Str | Type::Void => Ok(()),
            
            Type::UserDefined(name) => {
                if self.types.contains_key(name) {
                    Ok(())
                } else {
                    Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    })
                }
            }
            
            Type::Reference(inner, _) => self.validate_type(inner, span),
            Type::Array(inner) => self.validate_type(inner, span),
            Type::Tuple(types) => {
                for ty in types {
                    self.validate_type(ty, span)?;
                }
                Ok(())
            }
            
            Type::Function(func_type) => {
                for param in &func_type.params {
                    self.validate_type(param, span)?;
                }
                self.validate_type(&func_type.return_type, span)
            }
            
            Type::Generic(name, type_args) => {
                // ジェネリック型の検証
                if !self.has_type(name) && name != "Vec" && name != "HashMap" && name != "Option" && name != "Result" {
                    return Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    });
                }
                
                for arg in type_args {
                    self.validate_type(arg, span)?;
                }
                Ok(())
            }
            
            Type::Variable(_) => {
                // 型変数は型推論中に使用される
                Ok(())
            }
        }
    }


    /// 型を文字列に変換
    pub fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::I128 => "i128".to_string(),
            Type::I256 => "i256".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::U128 => "u128".to_string(),
            Type::U256 => "u256".to_string(),
            Type::F8 => "f8".to_string(),
            Type::F16 => "f16".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Str => "str".to_string(),
            Type::Void => "void".to_string(),
            Type::UserDefined(name) => name.clone(),
            Type::Reference(inner, is_mut) => {
                format!("&{}{}", if *is_mut { "mut " } else { "" }, self.type_to_string(inner))
            }
            Type::Array(inner) => format!("[{}]", self.type_to_string(inner)),
            Type::Tuple(types) => {
                let types_str = types.iter()
                    .map(|t| self.type_to_string(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", types_str)
            }
            Type::Function(func_type) => {
                let params_str = func_type.params.iter()
                    .map(|t| self.type_to_string(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", params_str, self.type_to_string(&func_type.return_type))
            }
            Type::Generic(name, type_args) => {
                if type_args.is_empty() {
                    name.clone()
                } else {
                    let args_str = type_args.iter()
                        .map(|t| self.type_to_string(t))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{}<{}>", name, args_str)
                }
            }
            Type::Variable(name) => format!("'{}", name),
        }
    }
}