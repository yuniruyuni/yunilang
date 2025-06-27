//! 型の登録と検索機能

use std::collections::HashMap;
use crate::ast::*;
use crate::analyzer::symbol::{AnalysisResult, AnalysisError};
use crate::analyzer::type_checker::TypeChecker;
use crate::analyzer::symbol::{TypeInfo, TypeKind, FunctionSignature};

impl TypeChecker {
    /// 型を登録
    pub fn register_type(&mut self, type_info: TypeInfo) -> AnalysisResult<()> {
        if self.types.contains_key(&type_info.name) {
            return Err(AnalysisError::DuplicateType {
                name: type_info.name.clone(),
                span: type_info.span,
            });
        }
        self.types.insert(type_info.name.clone(), type_info);
        Ok(())
    }

    /// 型が存在するかチェック
    pub fn has_type(&self, name: &str) -> bool {
        self.types.contains_key(name) || self.is_primitive_type(name)
    }

    /// 関数を登録
    pub fn register_function(&mut self, func_sig: FunctionSignature) -> AnalysisResult<()> {
        if self.functions.contains_key(&func_sig.name) {
            return Err(AnalysisError::DuplicateFunction {
                name: func_sig.name.clone(),
                span: func_sig.span,
            });
        }
        self.functions.insert(func_sig.name.clone(), func_sig);
        Ok(())
    }

    /// メソッドを登録
    pub fn register_method(&mut self, type_name: &str, method_sig: FunctionSignature) -> AnalysisResult<()> {
        let methods = self.methods.entry(type_name.to_string()).or_insert_with(HashMap::new);
        
        if methods.contains_key(&method_sig.name) {
            return Err(AnalysisError::DuplicateMethod {
                type_name: type_name.to_string(),
                method_name: method_sig.name.clone(),
                span: method_sig.span,
            });
        }
        
        methods.insert(method_sig.name.clone(), method_sig);
        Ok(())
    }

    /// 関数シグネチャを取得
    pub fn get_function_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.get(name)
    }

    /// 型情報を取得
    pub fn get_type_info(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }

    /// メソッドシグネチャを取得
    pub fn get_method_signature(&self, type_name: &str, method_name: &str) -> Option<&FunctionSignature> {
        self.methods.get(type_name)?.get(method_name)
    }

    /// フィールドの型を取得
    pub fn get_field_type(&self, struct_type: &Type, field_name: &str, span: Span) -> AnalysisResult<Type> {
        match struct_type {
            Type::UserDefined(name) => {
                if let Some(type_info) = self.types.get(name) {
                    match &type_info.kind {
                        TypeKind::Struct(fields) => {
                            if let Some(field) = fields.iter().find(|f| f.name == field_name) {
                                Ok(field.ty.clone())
                            } else {
                                Err(AnalysisError::UndefinedField {
                                    struct_name: name.clone(),
                                    field_name: field_name.to_string(),
                                    span,
                                })
                            }
                        }
                        _ => Err(AnalysisError::InvalidType {
                            message: format!("Type {} is not a struct", name),
                            span,
                        }),
                    }
                } else {
                    Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    })
                }
            }
            Type::Reference(inner, _) => {
                // 参照型の場合は内部の型を使用
                self.get_field_type(inner, field_name, span)
            }
            _ => Err(AnalysisError::InvalidType {
                message: format!("Cannot access field on type {:?}", struct_type),
                span,
            }),
        }
    }

    /// プリミティブ型かどうかチェック
    fn is_primitive_type(&self, name: &str) -> bool {
        matches!(name,
            "i8" | "i16" | "i32" | "i64" | "i128" | "i256" |
            "u8" | "u16" | "u32" | "u64" | "u128" | "u256" |
            "f32" | "f64" | "bool" | "string" | "char"
        )
    }
}