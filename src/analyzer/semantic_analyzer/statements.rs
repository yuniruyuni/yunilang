//! 文の解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisError, AnalysisResult, Symbol};
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// ブロックを解析
    pub fn analyze_block(&mut self, block: &Block) -> AnalysisResult<bool> {
        let mut returns = false;
        
        for stmt in &block.statements {
            if returns {
                // TODO: Implement unreachable code detection
            }
            
            match self.analyze_statement(stmt) {
                Ok(stmt_returns) => returns = stmt_returns,
                Err(e) => self.errors.push(e),
            }
        }
        
        Ok(returns)
    }

    /// 文を解析
    pub fn analyze_statement(&mut self, stmt: &Statement) -> AnalysisResult<bool> {
        match stmt {
            Statement::Let(let_stmt) => self.analyze_let_statement(let_stmt),
            Statement::Assignment(assign) => self.analyze_assignment(assign),
            Statement::Return(ret) => self.analyze_return_statement(ret),
            Statement::If(if_stmt) => self.analyze_if_statement(if_stmt),
            Statement::While(while_stmt) => self.analyze_while_statement(while_stmt),
            Statement::For(for_stmt) => self.analyze_for_statement(for_stmt),
            Statement::Expression(expr) => {
                self.analyze_expression(expr)?;
                Ok(false)
            }
            Statement::Block(block) => {
                self.enter_scope();
                let returns = self.analyze_block(block)?;
                self.exit_scope();
                Ok(returns)
            }
        }
    }

    /// let文の解析
    pub fn analyze_let_statement(&mut self, let_stmt: &LetStatement) -> AnalysisResult<bool> {
        // 初期化式がある場合は型チェック
        let inferred_type = if let Some(ref init_expr) = let_stmt.init {
            // 型注釈がある場合はそれを期待される型として使用
            
            
            if let Some(ref annotated_type) = let_stmt.ty {
                self.type_checker.validate_type(annotated_type, let_stmt.span)?;
                let expr_type = self.analyze_expression_with_type(init_expr, Some(annotated_type))?;
                self.type_checker.check_type_compatibility(annotated_type, &expr_type, let_stmt.span)?;
                annotated_type.clone()
            } else {
                self.analyze_expression(init_expr)?
            }
        } else if let Some(ref annotated_type) = let_stmt.ty {
            self.type_checker.validate_type(annotated_type, let_stmt.span)?;
            annotated_type.clone()
        } else {
            return Err(AnalysisError::TypeInferenceError {
                name: "variable".to_string(), // パターンの名前は後で取得
                span: let_stmt.span,
            });
        };

        // パターンの解析（今は簡単な識別子のみ対応）
        if let Pattern::Identifier(name, is_mutable) = &let_stmt.pattern {
            let symbol = Symbol {
                name: name.clone(),
                ty: inferred_type,
                is_mutable: *is_mutable,
                span: let_stmt.span,
                borrow_info: None,
                is_moved: false,
                lifetime: None,
            };
            
            self.scope_stack.last_mut().unwrap().define(symbol)?;
        }
        
        Ok(false)
    }

    /// 代入文の解析
    pub fn analyze_assignment(&mut self, assign: &AssignStatement) -> AnalysisResult<bool> {
        // 左辺の解析
        let target_type = self.analyze_expression(&assign.target)?;
        
        // 右辺の解析
        let value_type = self.analyze_expression(&assign.value)?;
        
        // 型の互換性チェック
        self.type_checker.check_type_compatibility(&target_type, &value_type, assign.span)?;
        
        // 変更可能性のチェック
        if let Expression::Identifier(ident) = &assign.target {
            if let Some(symbol) = self.lookup_variable(&ident.name) {
                if !symbol.is_mutable {
                    return Err(AnalysisError::ImmutableVariable {
                        name: ident.name.clone(),
                        span: assign.span,
                    });
                }
            }
        }
        
        Ok(false)
    }

    /// return文の解析
    pub fn analyze_return_statement(&mut self, ret: &ReturnStatement) -> AnalysisResult<bool> {
        let expected_type = self.current_return_type.clone();
        let return_type = if let Some(ref expr) = ret.value {
            // 現在の関数の戻り値型を期待される型として渡す
            self.analyze_expression_with_type(expr, expected_type.as_ref())?
        } else {
            Type::Void
        };
        
        // 関数の戻り値型と一致するかチェック
        if let Some(ref expected_type) = self.current_return_type {
            self.type_checker.check_type_compatibility(expected_type, &return_type, ret.span)?;
        }
        
        Ok(true)
    }

    /// if文の解析
    pub fn analyze_if_statement(&mut self, if_stmt: &IfStatement) -> AnalysisResult<bool> {
        // 条件式の型チェック
        let condition_type = self.analyze_expression(&if_stmt.condition)?;
        if !matches!(condition_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_checker.type_to_string(&condition_type),
                span: self.get_expression_span(&if_stmt.condition),
            });
        }
        
        // then節の解析
        let then_returns = self.analyze_block(&if_stmt.then_branch)?;
        
        // else節の解析（存在する場合）
        let else_returns = if let Some(ref else_branch) = if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => self.analyze_block(block)?,
                ElseBranch::If(if_stmt) => self.analyze_if_statement(if_stmt)?,
            }
        } else {
            false
        };
        
        // 両方の分岐でreturnする場合のみ、このif文がreturnする
        Ok(then_returns && else_returns)
    }

    /// while文の解析
    pub fn analyze_while_statement(&mut self, while_stmt: &WhileStatement) -> AnalysisResult<bool> {
        // 条件式の型チェック
        let condition_type = self.analyze_expression(&while_stmt.condition)?;
        if !matches!(condition_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_checker.type_to_string(&condition_type),
                span: self.get_expression_span(&while_stmt.condition),
            });
        }
        
        // ループ本体の解析
        self.analyze_block(&while_stmt.body)?;
        
        // while文は必ずしもreturnしない（条件がfalseの場合実行されない可能性）
        Ok(false)
    }

    /// for文の解析
    pub fn analyze_for_statement(&mut self, for_stmt: &ForStatement) -> AnalysisResult<bool> {
        // 新しいスコープを作成（ループ変数用）
        self.enter_scope();
        
        // init文の解析（存在する場合）
        if let Some(ref init) = for_stmt.init {
            self.analyze_statement(init)?;
        }
        
        // 条件式の解析（存在する場合）
        if let Some(ref condition) = for_stmt.condition {
            let condition_type = self.analyze_expression(condition)?;
            if !matches!(condition_type, Type::Bool) {
                return Err(AnalysisError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: self.type_checker.type_to_string(&condition_type),
                    span: self.get_expression_span(condition),
                });
            }
        }
        
        // ループ本体の解析
        self.analyze_block(&for_stmt.body)?;
        
        // update式の解析（存在する場合）
        if let Some(ref update) = for_stmt.update {
            self.analyze_expression(update)?;
        }
        
        self.exit_scope();
        
        // for文は必ずしもreturnしない
        Ok(false)
    }
}