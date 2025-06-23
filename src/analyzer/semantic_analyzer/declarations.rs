//! 宣言（型定義、関数シグネチャ）の解析

use crate::ast::*;
use std::collections::HashMap;

use crate::analyzer::symbol::{AnalysisError, AnalysisResult, FunctionSignature, TypeInfo, TypeKind};
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 型定義を収集
    pub fn collect_type_definition(&mut self, type_def: &TypeDef) -> AnalysisResult<()> {
        match type_def {
            TypeDef::Struct(struct_def) => self.collect_struct_definition(struct_def),
            TypeDef::Enum(enum_def) => self.collect_enum_definition(enum_def),
        }
    }

    /// 構造体定義を収集
    pub fn collect_struct_definition(&mut self, struct_def: &StructDef) -> AnalysisResult<()> {
        // フィールドの型を検証
        for field in &struct_def.fields {
            self.type_checker.validate_type(&field.ty, field.span)?;
        }

        let type_info = TypeInfo {
            name: struct_def.name.clone(),
            kind: TypeKind::Struct(struct_def.fields.clone()),
            methods: HashMap::new(),
            span: struct_def.span,
        };

        // type_checkerとscopeの両方に登録
        self.type_checker.register_type(type_info.clone())?;
        self.scope_stack.last_mut().unwrap().define_type(type_info)?;
        Ok(())
    }

    /// Enum定義を収集
    pub fn collect_enum_definition(&mut self, enum_def: &EnumDef) -> AnalysisResult<()> {
        // バリアントのフィールド型を検証
        for variant in &enum_def.variants {
            for field in &variant.fields {
                self.type_checker.validate_type(&field.ty, field.span)?;
            }
        }

        let type_info = TypeInfo {
            name: enum_def.name.clone(),
            kind: TypeKind::Enum(enum_def.variants.clone()),
            methods: HashMap::new(),
            span: enum_def.span,
        };

        // type_checkerとscopeの両方に登録
        self.type_checker.register_type(type_info.clone())?;
        self.scope_stack.last_mut().unwrap().define_type(type_info)?;
        Ok(())
    }

    /// 関数シグネチャを収集
    pub fn collect_function_signature(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        // パラメータの型を検証
        for param in &func.params {
            self.type_checker.validate_type(&param.ty, param.span)?;
        }
        
        // 戻り値型を検証
        let return_type = func.return_type.as_ref()
            .map(|t| t.as_ref().clone())
            .unwrap_or(Type::Void);
        self.type_checker.validate_type(&return_type, func.span)?;

        let signature = FunctionSignature {
            name: func.name.clone(),
            params: func.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
            return_type,
            lives_clause: func.lives_clause.clone(),
            is_method: false,
            receiver_type: None,
            span: func.span,
        };

        // グローバルスコープに関数を登録
        // TypeCheckerに関数シグネチャを登録
        self.type_checker.register_function(signature)?;
        Ok(())
    }

    /// メソッドシグネチャを収集
    pub fn collect_method_signature(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // レシーバー型が定義されているか確認
        self.type_checker.validate_type(&method.receiver.ty, method.span)?;

        // パラメータの型を検証
        for param in &method.params {
            self.type_checker.validate_type(&param.ty, param.span)?;
        }

        // 戻り値型を検証
        let return_type = method.return_type.as_ref()
            .map(|t| t.as_ref().clone())
            .unwrap_or(Type::Void);
        self.type_checker.validate_type(&return_type, method.span)?;

        // メソッドを対応する型に登録
        let receiver_name = match &method.receiver.ty {
            Type::UserDefined(name) => name.clone(),
            _ => return Err(AnalysisError::TypeMismatch {
                expected: "user-defined type".to_string(),
                found: format!("{:?}", method.receiver.ty),
                span: method.span,
            }),
        };

        // TODO: メソッドシグネチャの登録
        // 現在の実装では、TypeCheckerに直接メソッドを登録する方法がないため、
        // 一時的にスキップする
        Ok(())
    }
}