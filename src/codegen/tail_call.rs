//! 末尾呼び出し最適化のためのコード生成時解析
//!
//! コード生成時に末尾位置を判定し、最適化を適用する

use crate::ast::*;
use std::collections::HashSet;

/// 末尾位置コンテキスト
#[derive(Default)]
pub struct TailContext {
    /// 現在の関数名
    pub current_function: Option<String>,
    /// 末尾位置にある呼び出しのセット（式のアドレスで識別）
    pub tail_calls: HashSet<usize>,
}

impl TailContext {
    /// 新しい末尾位置コンテキストを作成
    pub fn new() -> Self {
        Self::default()
    }

    /// 関数の末尾位置を解析
    pub fn analyze_function(&mut self, func: &FunctionDecl) {
        self.current_function = Some(func.name.clone());
        self.tail_calls.clear();
        
        // 関数本体の最後の文を解析
        if let Some(last_stmt) = func.body.statements.last() {
            self.analyze_statement_tail_position(last_stmt, true);
        }
    }

    /// 文の末尾位置を解析
    fn analyze_statement_tail_position(&mut self, stmt: &Statement, is_tail: bool) {
        match stmt {
            Statement::Return(ret) => {
                // return文の値は常に末尾位置
                if let Some(expr) = &ret.value {
                    self.analyze_expression_tail_position(expr, true);
                }
            }
            Statement::Expression(expr) => {
                // 式文が関数の最後の文の場合、末尾位置
                self.analyze_expression_tail_position(expr, is_tail);
            }
            Statement::If(if_stmt) => {
                // 条件式は末尾位置ではない
                self.analyze_expression_tail_position(&if_stmt.condition, false);
                
                // if文の各ブランチの最後が末尾位置
                if let Some(last_then) = if_stmt.then_branch.statements.last() {
                    self.analyze_statement_tail_position(last_then, is_tail);
                }
                
                if let Some(else_branch) = &if_stmt.else_branch {
                    match else_branch {
                        ElseBranch::Block(block) => {
                            if let Some(last_else) = block.statements.last() {
                                self.analyze_statement_tail_position(last_else, is_tail);
                            }
                        }
                        ElseBranch::If(nested_if) => {
                            self.analyze_statement_tail_position(
                                &Statement::If((**nested_if).clone()),
                                is_tail
                            );
                        }
                    }
                }
            }
            Statement::Block(block) => {
                // ブロックの最後の文が末尾位置
                if let Some(last) = block.statements.last() {
                    self.analyze_statement_tail_position(last, is_tail);
                }
            }
            _ => {
                // その他の文（変数宣言など）は末尾位置を含まない
            }
        }
    }

    /// 式の末尾位置を解析
    fn analyze_expression_tail_position(&mut self, expr: &Expression, is_tail: bool) {
        match expr {
            Expression::Call(call) => {
                // 自己再帰呼び出しかチェック
                if is_tail && self.is_self_recursive_call(call) {
                    // 呼び出しのアドレスを記録
                    self.tail_calls.insert(call as *const _ as usize);
                }
                
                // 引数は末尾位置ではない
                for arg in &call.args {
                    self.analyze_expression_tail_position(arg, false);
                }
            }
            Expression::Block(block_expr) => {
                // ブロック式の最後の式が末尾位置
                for stmt in &block_expr.statements {
                    match stmt {
                        Statement::Expression(expr) => {
                            self.analyze_expression_tail_position(expr, false);
                        }
                        _ => self.analyze_statement_tail_position(stmt, false),
                    }
                }
                
                if let Some(last_expr) = &block_expr.last_expr {
                    self.analyze_expression_tail_position(last_expr, is_tail);
                }
            }
            Expression::If(if_expr) => {
                // 条件式は末尾位置ではない
                self.analyze_expression_tail_position(&if_expr.condition, false);
                
                // then/else式は末尾位置を継承
                self.analyze_expression_tail_position(&if_expr.then_branch, is_tail);
                if let Some(else_branch) = &if_expr.else_branch {
                    self.analyze_expression_tail_position(else_branch, is_tail);
                }
            }
            Expression::Match(match_expr) => {
                // マッチ対象式は末尾位置ではない
                self.analyze_expression_tail_position(&match_expr.expr, false);
                
                // 各アームの本体は末尾位置を継承
                for arm in &match_expr.arms {
                    if let Some(guard) = &arm.guard {
                        self.analyze_expression_tail_position(guard, false);
                    }
                    self.analyze_expression_tail_position(&arm.expr, is_tail);
                }
            }
            Expression::Binary(binary) => {
                // 二項演算の両側は末尾位置ではない
                self.analyze_expression_tail_position(&binary.left, false);
                self.analyze_expression_tail_position(&binary.right, false);
            }
            Expression::Unary(unary) => {
                // 単項演算のオペランドは末尾位置ではない
                self.analyze_expression_tail_position(&unary.expr, false);
            }
            _ => {
                // その他の式は末尾呼び出しを含まない
            }
        }
    }

    /// 呼び出しが自己再帰呼び出しかチェック
    fn is_self_recursive_call(&self, call: &CallExpr) -> bool {
        if let Some(current_func) = &self.current_function {
            if let Expression::Identifier(id) = &*call.callee {
                return id.name == *current_func;
            }
        }
        false
    }

    /// 呼び出しが末尾位置にあるかチェック
    pub fn is_tail_call(&self, call: &CallExpr) -> bool {
        self.tail_calls.contains(&(call as *const _ as usize))
    }
}