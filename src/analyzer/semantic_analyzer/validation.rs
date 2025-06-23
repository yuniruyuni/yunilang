//! 関数・メソッドの本体の解析と検証

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisError, AnalysisResult, Symbol};
use super::SemanticAnalyzer;
// use crate::analyzer::borrow_checker::BorrowChecker;

impl SemanticAnalyzer {
    /// インポートを処理
    pub fn process_import(&mut self, import: &Import) {
        let alias = import.alias.as_ref().unwrap_or(&import.path);
        self.imports.insert(alias.clone(), import.path.clone());
    }

    /// 関数の解析
    pub fn analyze_function(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        // 新しいスコープを作成
        self.enter_scope();

        // ライフタイムパラメータを設定
        if let Some(_lives_clause) = &func.lives_clause {
            // TODO: ライフタイムパラメータの処理
        }

        // パラメータをスコープに追加
        for param in &func.params {
            let symbol = Symbol {
                name: param.name.clone(),
                ty: param.ty.clone(),
                is_mutable: false,
                span: param.span,
                borrow_info: None,
                is_moved: false,
                lifetime: None,
            };
            self.scope_stack.last_mut().unwrap().define(symbol)?;
        }

        // 現在の関数の戻り値型を設定
        let return_type = func.return_type.as_ref()
            .map(|t| t.as_ref().clone())
            .unwrap_or(Type::Void);
        self.current_return_type = Some(return_type.clone());

        // 関数本体を解析
        let body_returns = self.analyze_block(&func.body)?;

        // 戻り値型がvoidでない場合、必ずreturnする必要がある
        if !matches!(return_type, Type::Void) && !body_returns {
            self.errors.push(AnalysisError::MissingReturn {
                name: func.name.clone(),
                span: func.span,
            });
        }

        // 借用チェック
        // TODO: BorrowCheckerの統合
        // let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, self.scope_stack.last_mut().unwrap());
        // for stmt in &func.body.statements {
        //     if let Err(e) = borrow_checker.check_statement(stmt) {
        //         self.errors.push(e);
        //     }
        // }

        self.current_return_type = None;
        self.exit_scope();

        Ok(())
    }

    /// メソッドの解析
    pub fn analyze_method(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // 新しいスコープを作成
        self.enter_scope();

        // ライフタイムパラメータを設定
        if let Some(_lives_clause) = &method.lives_clause {
            // TODO: ライフタイムパラメータの処理
        }

        // self パラメータをスコープに追加
        let self_symbol = Symbol {
            name: method.receiver.name.as_ref().unwrap_or(&"self".to_string()).clone(),
            ty: method.receiver.ty.clone(),
            is_mutable: method.receiver.is_mut,
            span: method.span,
            borrow_info: None,
            is_moved: false,
            lifetime: None,
        };
        self.scope_stack.last_mut().unwrap().define(self_symbol)?;

        // その他のパラメータをスコープに追加
        for param in &method.params {
            let symbol = Symbol {
                name: param.name.clone(),
                ty: param.ty.clone(),
                is_mutable: false,
                span: param.span,
                borrow_info: None,
                is_moved: false,
                lifetime: None,
            };
            self.scope_stack.last_mut().unwrap().define(symbol)?;
        }

        // 現在の関数の戻り値型を設定
        let return_type = method.return_type.as_ref()
            .map(|t| t.as_ref().clone())
            .unwrap_or(Type::Void);
        self.current_return_type = Some(return_type.clone());

        // メソッド本体を解析
        let body_returns = self.analyze_block(&method.body)?;

        // 戻り値型がvoidでない場合、必ずreturnする必要がある
        if !matches!(return_type, Type::Void) && !body_returns {
            self.errors.push(AnalysisError::MissingReturn {
                name: method.name.clone(),
                span: method.span,
            });
        }

        self.current_return_type = None;
        self.exit_scope();

        Ok(())
    }
}