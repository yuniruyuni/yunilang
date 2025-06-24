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
}