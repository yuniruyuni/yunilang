//! 関数・メソッドの本体の解析と検証

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisError, AnalysisResult, Symbol};
use super::SemanticAnalyzer;
use crate::analyzer::borrow_checker::BorrowChecker;

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
        
        // 型パラメータを環境に登録
        self.type_env.enter_scope();
        if let Err(e) = self.type_env.register_type_params(&func.type_params) {
            return match e {
                crate::error::YuniError::Analyzer(ae) => Err(ae),
                _ => Err(AnalysisError::InvalidOperation {
                    message: format!("Unexpected error in type parameter registration: {:?}", e),
                    span: func.span,
                }),
            };
        }

        // ライフタイムパラメータを設定
        if let Some(lives_clause) = &func.lives_clause {
            // ライフタイムパラメータをコンテキストに登録
            for constraint in &lives_clause.constraints {
                // ターゲットライフタイムを登録
                let target_lifetime = self.lifetime_context.register_named_lifetime(
                    constraint.target.clone(),
                    constraint.span,
                )?;
                
                // ソースライフタイムを登録し、制約を追加
                for source in &constraint.sources {
                    let source_lifetime = self.lifetime_context.register_named_lifetime(
                        source.clone(),
                        constraint.span,
                    )?;
                    
                    // 'source: 'target の制約を追加（sourceはtargetより長く生きる）
                    self.lifetime_context.add_outlives_constraint(source_lifetime, target_lifetime);
                    self.lifetime_context.add_constraint(
                        crate::analyzer::lifetime::LivesConstraint::Outlives {
                            longer: source_lifetime,
                            shorter: target_lifetime,
                            span: constraint.span,
                        }
                    );
                }
            }
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
        {
            let current_scope = self.scope_stack.last_mut().unwrap();
            let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, current_scope);
            for stmt in &func.body.statements {
                if let Err(e) = borrow_checker.check_statement(stmt) {
                    self.errors.push(e);
                }
            }
            // 借用チェックの最終検証
            if let Err(e) = borrow_checker.check() {
                self.errors.push(e);
            }
        }

        self.current_return_type = None;
        self.exit_scope();
        
        // 型パラメータのスコープを終了
        self.type_env.exit_scope();

        Ok(())
    }

    /// メソッドの解析
    pub fn analyze_method(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // 新しいスコープを作成
        self.enter_scope();
        
        // 型パラメータを環境に登録
        self.type_env.enter_scope();
        if let Err(e) = self.type_env.register_type_params(&method.type_params) {
            return match e {
                crate::error::YuniError::Analyzer(ae) => Err(ae),
                _ => Err(AnalysisError::InvalidOperation {
                    message: format!("Unexpected error in type parameter registration: {:?}", e),
                    span: method.span,
                }),
            };
        }

        // ライフタイムパラメータを設定
        if let Some(lives_clause) = &method.lives_clause {
            // ライフタイムパラメータをコンテキストに登録
            for constraint in &lives_clause.constraints {
                // ターゲットライフタイムを登録
                let target_lifetime = self.lifetime_context.register_named_lifetime(
                    constraint.target.clone(),
                    constraint.span,
                )?;
                
                // ソースライフタイムを登録し、制約を追加
                for source in &constraint.sources {
                    let source_lifetime = self.lifetime_context.register_named_lifetime(
                        source.clone(),
                        constraint.span,
                    )?;
                    
                    // 'source: 'target の制約を追加（sourceはtargetより長く生きる）
                    self.lifetime_context.add_outlives_constraint(source_lifetime, target_lifetime);
                    self.lifetime_context.add_constraint(
                        crate::analyzer::lifetime::LivesConstraint::Outlives {
                            longer: source_lifetime,
                            shorter: target_lifetime,
                            span: constraint.span,
                        }
                    );
                }
            }
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

        // 借用チェック
        {
            let current_scope = self.scope_stack.last_mut().unwrap();
            let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, current_scope);
            for stmt in &method.body.statements {
                if let Err(e) = borrow_checker.check_statement(stmt) {
                    self.errors.push(e);
                }
            }
            // 借用チェックの最終検証
            if let Err(e) = borrow_checker.check() {
                self.errors.push(e);
            }
        }

        self.current_return_type = None;
        self.exit_scope();
        
        // 型パラメータのスコープを終了
        self.type_env.exit_scope();

        Ok(())
    }
}