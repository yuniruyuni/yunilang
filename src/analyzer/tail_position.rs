//! 末尾位置解析
//! 
//! 末尾再帰最適化のために、関数呼び出しが末尾位置にあるかを判定する

use crate::ast::*;

/// 末尾位置解析器
#[allow(dead_code)]
pub struct TailPositionAnalyzer {
    current_function: Option<String>,
}

impl TailPositionAnalyzer {
    /// 新しい末尾位置解析器を作成
    pub fn new() -> Self {
        Self {
            current_function: None,
        }
    }

    /// 関数の末尾呼び出しをマーク
    pub fn mark_tail_calls_in_function(&mut self, func: &mut FunctionDecl) {
        self.current_function = Some(func.name.clone());
        
        // 関数本体の最後の文を解析
        if let Some(last_stmt) = func.body.statements.last_mut() {
            self.mark_tail_calls_in_statement(last_stmt, true);
        }
        
        self.current_function = None;
    }

    /// 文内の末尾呼び出しをマーク
    fn mark_tail_calls_in_statement(&mut self, stmt: &mut Statement, is_tail_position: bool) {
        match stmt {
            Statement::Return(ret) => {
                // return文の値は常に末尾位置
                if let Some(expr) = &mut ret.value {
                    self.mark_tail_calls_in_expression(expr, true);
                }
            }
            Statement::Expression(expr) => {
                // 式文が関数の最後の文の場合、末尾位置
                self.mark_tail_calls_in_expression(expr, is_tail_position);
            }
            Statement::If(if_stmt) => {
                // if文の各ブランチの最後が末尾位置
                if let Some(last_then) = if_stmt.then_branch.statements.last_mut() {
                    self.mark_tail_calls_in_statement(last_then, is_tail_position);
                }
                
                if let Some(else_branch) = &mut if_stmt.else_branch {
                    match else_branch {
                        ElseBranch::Block(block) => {
                            if let Some(last_else) = block.statements.last_mut() {
                                self.mark_tail_calls_in_statement(last_else, is_tail_position);
                            }
                        }
                        ElseBranch::If(nested_if) => {
                            // ネストしたif文の最後の文をマーク
                            if let Some(last_then) = nested_if.then_branch.statements.last_mut() {
                                self.mark_tail_calls_in_statement(last_then, is_tail_position);
                            }
                            
                            if let Some(nested_else) = &mut nested_if.else_branch {
                                match nested_else {
                                    ElseBranch::Block(block) => {
                                        if let Some(last) = block.statements.last_mut() {
                                            self.mark_tail_calls_in_statement(last, is_tail_position);
                                        }
                                    }
                                    ElseBranch::If(_) => {
                                        // 再帰的に処理
                                        self.mark_tail_calls_in_statement(
                                            &mut Statement::If((**nested_if).clone()),
                                            is_tail_position
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Statement::Block(block) => {
                // ブロックの最後の文が末尾位置
                if let Some(last) = block.statements.last_mut() {
                    self.mark_tail_calls_in_statement(last, is_tail_position);
                }
            }
            _ => {
                // その他の文（変数宣言など）は末尾位置を含まない
            }
        }
    }

    /// 式内の末尾呼び出しをマーク
    fn mark_tail_calls_in_expression(&mut self, expr: &mut Expression, is_tail_position: bool) {
        match expr {
            Expression::Call(call) => {
                // 自己再帰呼び出しかチェック
                if is_tail_position && self.is_self_recursive_call(call) {
                    call.is_tail = true;
                }
                
                // 引数は末尾位置ではない
                for arg in &mut call.args {
                    self.mark_tail_calls_in_expression(arg, false);
                }
            }
            Expression::Block(block_expr) => {
                // ブロック式の最後の式が末尾位置
                for stmt in &mut block_expr.statements {
                    self.mark_tail_calls_in_statement(stmt, false);
                }
                
                if let Some(last_expr) = &mut block_expr.last_expr {
                    self.mark_tail_calls_in_expression(last_expr, is_tail_position);
                }
            }
            Expression::If(if_expr) => {
                // 条件式は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut if_expr.condition, false);
                
                // then/else式は末尾位置を継承
                self.mark_tail_calls_in_expression(&mut if_expr.then_branch, is_tail_position);
                if let Some(else_branch) = &mut if_expr.else_branch {
                    self.mark_tail_calls_in_expression(else_branch, is_tail_position);
                }
            }
            Expression::Match(match_expr) => {
                // マッチ対象式は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut match_expr.expr, false);
                
                // 各アームの本体は末尾位置を継承
                for arm in &mut match_expr.arms {
                    if let Some(guard) = &mut arm.guard {
                        self.mark_tail_calls_in_expression(guard, false);
                    }
                    self.mark_tail_calls_in_expression(&mut arm.expr, is_tail_position);
                }
            }
            Expression::Binary(binary) => {
                // 二項演算の両側は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut binary.left, false);
                self.mark_tail_calls_in_expression(&mut binary.right, false);
            }
            Expression::Unary(unary) => {
                // 単項演算のオペランドは末尾位置ではない
                self.mark_tail_calls_in_expression(&mut unary.expr, false);
            }
            Expression::Index(index) => {
                // インデックスアクセスの各部分は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut index.object, false);
                self.mark_tail_calls_in_expression(&mut index.index, false);
            }
            Expression::Field(field) => {
                // フィールドアクセスのオブジェクトは末尾位置ではない
                self.mark_tail_calls_in_expression(&mut field.object, false);
            }
            Expression::MethodCall(method) => {
                // メソッド呼び出しの各部分は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut method.object, false);
                for arg in &mut method.args {
                    self.mark_tail_calls_in_expression(arg, false);
                }
            }
            Expression::Reference(ref_expr) => {
                // 参照式の内部は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut ref_expr.expr, false);
            }
            Expression::Dereference(deref) => {
                // 参照外しの内部は末尾位置ではない
                self.mark_tail_calls_in_expression(&mut deref.expr, false);
            }
            _ => {
                // リテラル、識別子などは末尾呼び出しを含まない
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tail_recursion_marking() {
        let mut func = FunctionDecl {
            name: "factorial".to_string(),
            params: vec![],
            return_type: Some(Box::new(Type::I32)),
            body: Block {
                statements: vec![
                    Statement::If(IfStatement {
                        condition: Expression::Binary(BinaryExpr {
                            left: Box::new(Expression::Identifier(Identifier {
                                name: "n".to_string(),
                                span: Span::dummy(),
                            })),
                            op: BinaryOp::Le,
                            right: Box::new(Expression::Integer(IntegerLit {
                                value: 1,
                                suffix: None,
                                span: Span::dummy(),
                            })),
                            span: Span::dummy(),
                        }),
                        then_branch: Block {
                            statements: vec![Statement::Return(ReturnStatement {
                                value: Some(Expression::Integer(IntegerLit {
                                    value: 1,
                                    suffix: None,
                                    span: Span::dummy(),
                                })),
                                span: Span::dummy(),
                            })],
                            span: Span::dummy(),
                        },
                        else_branch: Some(ElseBranch::Block(Block {
                            statements: vec![Statement::Return(ReturnStatement {
                                value: Some(Expression::Binary(BinaryExpr {
                                    left: Box::new(Expression::Identifier(Identifier {
                                        name: "n".to_string(),
                                        span: Span::dummy(),
                                    })),
                                    op: BinaryOp::Multiply,
                                    right: Box::new(Expression::Call(CallExpr {
                                        callee: Box::new(Expression::Identifier(Identifier {
                                            name: "factorial".to_string(),
                                            span: Span::dummy(),
                                        })),
                                        args: vec![Expression::Binary(BinaryExpr {
                                            left: Box::new(Expression::Identifier(Identifier {
                                                name: "n".to_string(),
                                                span: Span::dummy(),
                                            })),
                                            op: BinaryOp::Subtract,
                                            right: Box::new(Expression::Integer(IntegerLit {
                                                value: 1,
                                                suffix: None,
                                                span: Span::dummy(),
                                            })),
                                            span: Span::dummy(),
                                        })],
                                        span: Span::dummy(),
                                        is_tail: false,
                                    })),
                                    span: Span::dummy(),
                                })),
                                span: Span::dummy(),
                            })],
                            span: Span::dummy(),
                        })),
                        span: Span::dummy(),
                    }),
                ],
                span: Span::dummy(),
            },
            is_public: false,
            lives_clause: None,
            type_params: vec![],
            span: Span::dummy(),
        };

        let mut analyzer = TailPositionAnalyzer::new();
        analyzer.mark_tail_calls_in_function(&mut func);

        // 再帰呼び出しは末尾位置ではない（n * factorial(n-1)の一部）
        if let Statement::If(if_stmt) = &func.body.statements[0] {
            if let Some(ElseBranch::Block(else_block)) = &if_stmt.else_branch {
                if let Statement::Return(ret) = &else_block.statements[0] {
                    if let Some(Expression::Binary(binary)) = &ret.value {
                        if let Expression::Call(call) = &*binary.right {
                            assert!(!call.is_tail, "Recursive call should not be marked as tail call");
                        }
                    }
                }
            }
        }
    }
}