//! セマンティック解析器のメイン実装

use crate::ast::*;
use std::collections::HashMap;

use super::lifetime::LifetimeContext;
use super::symbol::{AnalysisError, AnalysisResult, Scope};
use super::type_checker::TypeChecker;
use super::type_env::TypeEnvironment;

// サブモジュール
mod complex_expressions;
mod declarations;
mod expressions;
mod scope;
mod statements;
mod validation;
mod literals;
mod operations;
mod collections_analyzer;
mod basic;

/// セマンティック解析器
pub struct SemanticAnalyzer {
    /// スコープスタック
    pub scope_stack: Vec<Scope>,
    /// 型チェッカー
    pub type_checker: TypeChecker,
    /// インポートエイリアス
    pub imports: HashMap<String, String>,
    /// 現在の関数の戻り値型（return文のチェック用）
    pub current_return_type: Option<Type>,
    /// 現在の関数のライフタイムコンテキスト
    pub lifetime_context: LifetimeContext,
    /// 型パラメータ環境
    pub type_env: TypeEnvironment,
    /// 収集されたエラー
    pub errors: Vec<AnalysisError>,
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scope_stack: vec![Scope::new()],
            type_checker: TypeChecker::new(),
            imports: HashMap::new(),
            current_return_type: None,
            lifetime_context: LifetimeContext::new(),
            type_env: TypeEnvironment::new(),
            errors: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> AnalysisResult<()> {
        // インポートを処理
        for import in &program.imports {
            self.process_import(import);
        }

        // 第一パス: 型定義と関数シグネチャを収集
        for item in &program.items {
            match item {
                Item::TypeDef(type_def) => {
                    if let Err(e) = self.collect_type_definition(type_def) {
                        self.errors.push(e);
                    }
                }
                Item::Function(func) => {
                    if let Err(e) = self.collect_function_signature(func) {
                        self.errors.push(e);
                    }
                }
                Item::Method(method) => {
                    if let Err(e) = self.collect_method_signature(method) {
                        self.errors.push(e);
                    }
                }
            }
        }

        // 第二パス: 関数とメソッドの本体を解析
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    if let Err(e) = self.analyze_function(func) {
                        self.errors.push(e);
                    }
                }
                Item::Method(method) => {
                    if let Err(e) = self.analyze_method(method) {
                        self.errors.push(e);
                    }
                }
                _ => {}
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            // 最初のエラーを返すが、Spanがdummyの場合は実際のエラー箇所が分からないので
            // より詳細なエラーメッセージを構築
            Err(self.errors[0].clone())
        }
    }
    
    /// フィールド型から型パラメータを統一
    fn unify_field_types(&self, expected_type: &Type, actual_type: &Type, type_map: &mut HashMap<String, Type>) {
        match (expected_type, actual_type) {
            (Type::Variable(name), _) => {
                // 型変数の場合、実際の型を記録
                type_map.insert(name.clone(), actual_type.clone());
            }
            (Type::Generic(expected_name, expected_args), Type::Generic(actual_name, actual_args)) 
                if expected_name == actual_name && expected_args.len() == actual_args.len() => {
                // ジェネリック型の場合、再帰的に型引数を統一
                for (expected_arg, actual_arg) in expected_args.iter().zip(actual_args.iter()) {
                    self.unify_field_types(expected_arg, actual_arg, type_map);
                }
            }
            (Type::Array(expected_elem), Type::Array(actual_elem)) => {
                // 配列の場合、要素型を統一
                self.unify_field_types(expected_elem, actual_elem, type_map);
            }
            (Type::Reference(expected_inner, _), Type::Reference(actual_inner, _)) => {
                // 参照の場合、内部型を統一
                self.unify_field_types(expected_inner, actual_inner, type_map);
            }
            _ => {
                // その他の場合は何もしない
            }
        }
    }
}