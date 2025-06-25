//! ジェネリック呼び出しの置換処理

use crate::ast::*;
use crate::error::YuniResult;
use super::Monomorphizer;

impl Monomorphizer {
    /// プログラム内のジェネリック呼び出しを単相化バージョンに置き換え
    pub(super) fn replace_generic_calls(&self, program: &mut Program) -> YuniResult<()> {
        for item in &mut program.items {
            match item {
                Item::Function(func) => {
                    func.body = self.replace_calls_in_block(&func.body)?;
                }
                Item::Method(method) => {
                    method.body = self.replace_calls_in_block(&method.body)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    /// ブロック内の呼び出しを置き換え
    fn replace_calls_in_block(&self, block: &Block) -> YuniResult<Block> {
        let mut new_statements = Vec::new();
        for stmt in &block.statements {
            new_statements.push(self.replace_calls_in_statement(stmt)?);
        }
        Ok(Block {
            statements: new_statements,
            span: block.span,
        })
    }
    
    /// 文内の呼び出しを置き換え
    fn replace_calls_in_statement(&self, stmt: &Statement) -> YuniResult<Statement> {
        match stmt {
            Statement::Let(let_stmt) => {
                let new_init = let_stmt.init.as_ref()
                    .map(|init| self.replace_calls_in_expr(init))
                    .transpose()?;
                Ok(Statement::Let(LetStatement {
                    pattern: let_stmt.pattern.clone(),
                    ty: let_stmt.ty.clone(),
                    init: new_init,
                    span: let_stmt.span,
                }))
            }
            Statement::Expression(expr) => {
                Ok(Statement::Expression(self.replace_calls_in_expr(expr)?))
            }
            Statement::Assignment(assign) => {
                Ok(Statement::Assignment(AssignStatement {
                    target: self.replace_calls_in_expr(&assign.target)?,
                    value: self.replace_calls_in_expr(&assign.value)?,
                    span: assign.span,
                }))
            }
            Statement::Return(ret_stmt) => {
                let new_value = ret_stmt.value.as_ref()
                    .map(|value| self.replace_calls_in_expr(value))
                    .transpose()?;
                Ok(Statement::Return(ReturnStatement {
                    value: new_value,
                    span: ret_stmt.span,
                }))
            }
            Statement::If(if_stmt) => {
                let new_condition = self.replace_calls_in_expr(&if_stmt.condition)?;
                let new_then = self.replace_calls_in_block(&if_stmt.then_branch)?;
                let new_else = match &if_stmt.else_branch {
                    Some(ElseBranch::Block(block)) => 
                        Some(ElseBranch::Block(self.replace_calls_in_block(block)?)),
                    Some(ElseBranch::If(if_stmt)) => {
                        if let Statement::If(new_if) = self.replace_calls_in_statement(
                            &Statement::If(*if_stmt.clone()))? {
                            Some(ElseBranch::If(Box::new(new_if)))
                        } else {
                            None
                        }
                    }
                    None => None,
                };
                Ok(Statement::If(IfStatement {
                    condition: new_condition,
                    then_branch: new_then,
                    else_branch: new_else,
                    span: if_stmt.span,
                }))
            }
            Statement::While(while_stmt) => {
                Ok(Statement::While(WhileStatement {
                    condition: self.replace_calls_in_expr(&while_stmt.condition)?,
                    body: self.replace_calls_in_block(&while_stmt.body)?,
                    span: while_stmt.span,
                }))
            }
            Statement::For(for_stmt) => {
                let new_init = match for_stmt.init.as_ref() {
                    Some(init) => Some(Box::new(self.replace_calls_in_statement(init)?)),
                    None => None,
                };
                let new_condition = for_stmt.condition.as_ref()
                    .map(|cond| self.replace_calls_in_expr(cond))
                    .transpose()?;
                let new_update = for_stmt.update.as_ref()
                    .map(|update| self.replace_calls_in_expr(update))
                    .transpose()?;
                Ok(Statement::For(ForStatement {
                    init: new_init,
                    condition: new_condition,
                    update: new_update,
                    body: self.replace_calls_in_block(&for_stmt.body)?,
                    span: for_stmt.span,
                }))
            }
            Statement::Block(block) => {
                Ok(Statement::Block(self.replace_calls_in_block(block)?))
            }
        }
    }
    
    /// 式内の呼び出しを置き換え
    fn replace_calls_in_expr(&self, expr: &Expression) -> YuniResult<Expression> {
        match expr {
            Expression::Call(call) => {
                if let Expression::Identifier(ident) = &*call.callee {
                    // ジェネリック関数の呼び出しかチェック
                    if self.generic_functions.contains_key(&ident.name) {
                        // 型引数を推論
                        let type_args = self.infer_type_args_from_call(&ident.name, &call.args)?;
                        if !type_args.is_empty() {
                            // マングルされた名前に置き換え
                            let mangled_name = super::mangle_function_name(&ident.name, &type_args);
                            return Ok(Expression::Call(CallExpr {
                                callee: Box::new(Expression::Identifier(Identifier {
                                    name: mangled_name,
                                    span: ident.span,
                                })),
                                args: call.args.clone(),
                                span: call.span,
                                is_tail: call.is_tail,
                            }));
                        }
                    }
                }
                
                // 引数も再帰的に処理
                let mut new_args = Vec::new();
                for arg in &call.args {
                    new_args.push(self.replace_calls_in_expr(arg)?);
                }
                Ok(Expression::Call(CallExpr {
                    callee: call.callee.clone(),
                    args: new_args,
                    span: call.span,
                    is_tail: call.is_tail,
                }))
            }
            Expression::StructLit(struct_lit) => {
                // ジェネリック構造体のインスタンス化かチェック
                if self.generic_structs.contains_key(&struct_lit.name) {
                    // 型引数を推論
                    let type_args = self.infer_type_args_from_struct_lit(struct_lit)?;
                    if !type_args.is_empty() {
                        // マングルされた名前に置き換え
                        let mangled_name = super::mangle_struct_name(&struct_lit.name, &type_args);
                        let mut new_fields = Vec::new();
                        for field in &struct_lit.fields {
                            new_fields.push(StructFieldInit {
                                name: field.name.clone(),
                                value: self.replace_calls_in_expr(&field.value)?,
                            });
                        }
                        return Ok(Expression::StructLit(StructLiteral {
                            name: mangled_name,
                            fields: new_fields,
                            span: struct_lit.span,
                        }));
                    }
                }
                
                // フィールドの値も再帰的に処理
                let mut new_fields = Vec::new();
                for field in &struct_lit.fields {
                    new_fields.push(StructFieldInit {
                        name: field.name.clone(),
                        value: self.replace_calls_in_expr(&field.value)?,
                    });
                }
                Ok(Expression::StructLit(StructLiteral {
                    name: struct_lit.name.clone(),
                    fields: new_fields,
                    span: struct_lit.span,
                }))
            }
            Expression::Binary(binary) => {
                let new_left = self.replace_calls_in_expr(&binary.left)?;
                let new_right = self.replace_calls_in_expr(&binary.right)?;
                Ok(Expression::Binary(BinaryExpr {
                    left: Box::new(new_left),
                    op: binary.op.clone(),
                    right: Box::new(new_right),
                    span: binary.span,
                }))
            }
            Expression::Unary(unary) => {
                let new_expr = self.replace_calls_in_expr(&unary.expr)?;
                Ok(Expression::Unary(UnaryExpr {
                    op: unary.op.clone(),
                    expr: Box::new(new_expr),
                    span: unary.span,
                }))
            }
            Expression::Block(block) => {
                let mut new_statements = Vec::new();
                for stmt in &block.statements {
                    new_statements.push(self.replace_calls_in_statement(stmt)?);
                }
                let new_last_expr = match block.last_expr.as_ref() {
                    Some(expr) => Some(Box::new(self.replace_calls_in_expr(expr)?)),
                    None => None,
                };
                Ok(Expression::Block(BlockExpr {
                    statements: new_statements,
                    last_expr: new_last_expr,
                    span: block.span,
                }))
            }
            Expression::Field(field) => {
                let new_object = self.replace_calls_in_expr(&field.object)?;
                Ok(Expression::Field(FieldExpr {
                    object: Box::new(new_object),
                    field: field.field.clone(),
                    span: field.span,
                }))
            }
            Expression::If(if_expr) => {
                let new_condition = Box::new(self.replace_calls_in_expr(&if_expr.condition)?);
                let new_then = Box::new(self.replace_calls_in_expr(&if_expr.then_branch)?);
                let new_else = match if_expr.else_branch.as_ref() {
                    Some(else_branch) => Some(Box::new(self.replace_calls_in_expr(else_branch)?)),
                    None => None,
                };
                Ok(Expression::If(IfExpr {
                    condition: new_condition,
                    then_branch: new_then,
                    else_branch: new_else,
                    span: if_expr.span,
                }))
            }
            Expression::Match(match_expr) => {
                let new_expr = Box::new(self.replace_calls_in_expr(&match_expr.expr)?);
                let mut new_arms = Vec::new();
                for arm in &match_expr.arms {
                    new_arms.push(MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref()
                            .map(|guard| self.replace_calls_in_expr(guard))
                            .transpose()?,
                        expr: self.replace_calls_in_expr(&arm.expr)?,
                    });
                }
                Ok(Expression::Match(MatchExpr {
                    expr: new_expr,
                    arms: new_arms,
                    span: match_expr.span,
                }))
            }
            Expression::Array(array) => {
                let mut new_elements = Vec::new();
                for elem in &array.elements {
                    new_elements.push(self.replace_calls_in_expr(elem)?);
                }
                Ok(Expression::Array(ArrayExpr {
                    elements: new_elements,
                    span: array.span,
                }))
            }
            Expression::Tuple(tuple) => {
                let mut new_elements = Vec::new();
                for elem in &tuple.elements {
                    new_elements.push(self.replace_calls_in_expr(elem)?);
                }
                Ok(Expression::Tuple(TupleExpr {
                    elements: new_elements,
                    span: tuple.span,
                }))
            }
            Expression::Index(index) => {
                let new_object = self.replace_calls_in_expr(&index.object)?;
                let new_index = self.replace_calls_in_expr(&index.index)?;
                Ok(Expression::Index(IndexExpr {
                    object: Box::new(new_object),
                    index: Box::new(new_index),
                    span: index.span,
                }))
            }
            Expression::MethodCall(method) => {
                let new_object = self.replace_calls_in_expr(&method.object)?;
                let mut new_args = Vec::new();
                for arg in &method.args {
                    new_args.push(self.replace_calls_in_expr(arg)?);
                }
                Ok(Expression::MethodCall(MethodCallExpr {
                    object: Box::new(new_object),
                    method: method.method.clone(),
                    args: new_args,
                    span: method.span,
                }))
            }
            Expression::Cast(cast) => {
                let new_expr = self.replace_calls_in_expr(&cast.expr)?;
                Ok(Expression::Cast(CastExpr {
                    expr: Box::new(new_expr),
                    ty: cast.ty.clone(),
                    span: cast.span,
                }))
            }
            Expression::Reference(ref_expr) => {
                let new_expr = self.replace_calls_in_expr(&ref_expr.expr)?;
                Ok(Expression::Reference(ReferenceExpr {
                    expr: Box::new(new_expr),
                    is_mut: ref_expr.is_mut,
                    span: ref_expr.span,
                }))
            }
            Expression::Dereference(deref) => {
                let new_expr = self.replace_calls_in_expr(&deref.expr)?;
                Ok(Expression::Dereference(DereferenceExpr {
                    expr: Box::new(new_expr),
                    span: deref.span,
                }))
            }
            // リテラルや識別子などはそのまま
            _ => Ok(expr.clone()),
        }
    }
}