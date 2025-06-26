//! 型の置換処理

use std::collections::HashMap;
use crate::ast::*;
use crate::error::YuniResult;
use super::{Monomorphizer, InstantiationType};

impl Monomorphizer {
    /// 型を置換
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn substitute_type(&self, ty: &Type, type_map: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Variable(name) if type_map.contains_key(name) => {
                type_map.get(name).cloned().unwrap()
            }
            Type::UserDefined(name) if type_map.contains_key(name) => {
                type_map.get(name).cloned().unwrap()
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.substitute_type(elem, type_map)))
            }
            Type::Reference(inner, is_mut) => {
                Type::Reference(Box::new(self.substitute_type(inner, type_map)), *is_mut)
            }
            Type::Tuple(elems) => {
                let substituted_elems: Vec<Type> = elems.iter()
                    .map(|elem| self.substitute_type(elem, type_map))
                    .collect();
                Type::Tuple(substituted_elems)
            }
            Type::Function(func_type) => {
                let substituted_params: Vec<Type> = func_type.params.iter()
                    .map(|param| self.substitute_type(param, type_map))
                    .collect();
                let substituted_ret = self.substitute_type(&func_type.return_type, type_map);
                Type::Function(FunctionType {
                    params: substituted_params,
                    return_type: Box::new(substituted_ret),
                })
            }
            _ => ty.clone(),
        }
    }
    
    /// ブロックを置換
    pub(super) fn substitute_block(&mut self, block: &Block, type_map: &HashMap<String, Type>) -> YuniResult<Block> {
        let mut new_statements = Vec::new();
        for stmt in &block.statements {
            new_statements.push(self.substitute_statement(stmt, type_map)?);
        }
        Ok(Block {
            statements: new_statements,
            span: block.span,
        })
    }
    
    /// 文を置換
    pub(super) fn substitute_statement(&mut self, stmt: &Statement, type_map: &HashMap<String, Type>) -> YuniResult<Statement> {
        match stmt {
            Statement::Let(let_stmt) => {
                let new_ty = let_stmt.ty.as_ref().map(|ty| self.substitute_type(ty, type_map));
                let new_init = let_stmt.init.as_ref()
                    .map(|init| self.substitute_expr(init, type_map))
                    .transpose()?;
                Ok(Statement::Let(LetStatement {
                    pattern: let_stmt.pattern.clone(),
                    ty: new_ty,
                    init: new_init,
                    span: let_stmt.span,
                }))
            }
            Statement::Expression(expr) => {
                Ok(Statement::Expression(self.substitute_expr(expr, type_map)?))
            }
            Statement::Assignment(assign) => {
                let new_target = self.substitute_expr(&assign.target, type_map)?;
                let new_value = self.substitute_expr(&assign.value, type_map)?;
                Ok(Statement::Assignment(AssignStatement {
                    target: new_target,
                    value: new_value,
                    span: assign.span,
                }))
            }
            Statement::Return(ret_stmt) => {
                let new_value = ret_stmt.value.as_ref()
                    .map(|value| self.substitute_expr(value, type_map))
                    .transpose()?;
                Ok(Statement::Return(ReturnStatement {
                    value: new_value,
                    span: ret_stmt.span,
                }))
            }
            Statement::If(if_stmt) => {
                let new_condition = self.substitute_expr(&if_stmt.condition, type_map)?;
                let new_then = self.substitute_block(&if_stmt.then_branch, type_map)?;
                let new_else = match &if_stmt.else_branch {
                    Some(ElseBranch::Block(block)) => 
                        Some(ElseBranch::Block(self.substitute_block(block, type_map)?)),
                    Some(ElseBranch::If(if_stmt)) => {
                        if let Statement::If(new_if) = self.substitute_statement(
                            &Statement::If(*if_stmt.clone()), type_map)? {
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
                let new_condition = self.substitute_expr(&while_stmt.condition, type_map)?;
                let new_body = self.substitute_block(&while_stmt.body, type_map)?;
                Ok(Statement::While(WhileStatement {
                    condition: new_condition,
                    body: new_body,
                    span: while_stmt.span,
                }))
            }
            Statement::For(for_stmt) => {
                let new_init = match for_stmt.init.as_ref() {
                    Some(init) => Some(Box::new(self.substitute_statement(init, type_map)?)),
                    None => None,
                };
                let new_condition = for_stmt.condition.as_ref()
                    .map(|cond| self.substitute_expr(cond, type_map))
                    .transpose()?;
                let new_update = for_stmt.update.as_ref()
                    .map(|update| self.substitute_expr(update, type_map))
                    .transpose()?;
                let new_body = self.substitute_block(&for_stmt.body, type_map)?;
                Ok(Statement::For(ForStatement {
                    init: new_init,
                    condition: new_condition,
                    update: new_update,
                    body: new_body,
                    span: for_stmt.span,
                }))
            }
            Statement::Block(block) => {
                Ok(Statement::Block(self.substitute_block(block, type_map)?))
            }
        }
    }
    
    /// 式を置換
    pub(super) fn substitute_expr(&mut self, expr: &Expression, type_map: &HashMap<String, Type>) -> YuniResult<Expression> {
        match expr {
            Expression::Call(call) => {
                let new_callee = self.substitute_expr(&call.callee, type_map)?;
                let mut new_args = Vec::new();
                for arg in &call.args {
                    new_args.push(self.substitute_expr(arg, type_map)?);
                }
                
                // ジェネリック関数呼び出しの場合、単相化された名前に置き換える必要がある
                if let Expression::Identifier(ident) = &new_callee {
                    if self.generic_functions.contains_key(&ident.name) {
                        // 型引数を推論
                        let type_args = self.infer_type_args_from_call(&ident.name, &new_args)?;
                        if !type_args.is_empty() {
                            // インスタンス化をキューに追加
                            self.queue_instantiation(&ident.name, type_args.clone(), InstantiationType::Function);
                            // マングルされた名前に置き換え
                            let mangled_name = crate::analyzer::monomorphization::mangling::mangle_function_name(&ident.name, &type_args);
                            return Ok(Expression::Call(CallExpr {
                                callee: Box::new(Expression::Identifier(Identifier {
                                    name: mangled_name,
                                    span: ident.span,
                                })),
                                args: new_args,
                                span: call.span,
                                is_tail: call.is_tail,
                            }));
                        }
                    }
                }
                
                Ok(Expression::Call(CallExpr {
                    callee: Box::new(new_callee),
                    args: new_args,
                    span: call.span,
                    is_tail: call.is_tail,
                }))
            }
            Expression::StructLit(struct_lit) => {
                let mut new_fields = Vec::new();
                for field in &struct_lit.fields {
                    new_fields.push(StructFieldInit {
                        name: field.name.clone(),
                        value: self.substitute_expr(&field.value, type_map)?,
                    });
                }
                
                // ジェネリック構造体の場合、単相化された名前に置き換える
                let mut new_name = struct_lit.name.clone();
                if let Some(name) = &struct_lit.name {
                    if self.generic_structs.contains_key(name) {
                        // 型引数を推論
                        let type_args = self.infer_type_args_from_struct_lit(struct_lit)?;
                        if !type_args.is_empty() {
                            // インスタンス化をキューに追加
                            self.queue_instantiation(name, type_args.clone(), InstantiationType::Struct);
                            // マングルされた名前に置き換え
                            new_name = Some(crate::analyzer::monomorphization::mangling::mangle_struct_name(name, &type_args));
                        }
                    }
                }
                
                Ok(Expression::StructLit(StructLiteral {
                    name: new_name,
                    fields: new_fields,
                    span: struct_lit.span,
                }))
            }
            Expression::Binary(binary) => {
                let new_left = self.substitute_expr(&binary.left, type_map)?;
                let new_right = self.substitute_expr(&binary.right, type_map)?;
                Ok(Expression::Binary(BinaryExpr {
                    left: Box::new(new_left),
                    op: binary.op.clone(),
                    right: Box::new(new_right),
                    span: binary.span,
                }))
            }
            Expression::Unary(unary) => {
                let new_expr = self.substitute_expr(&unary.expr, type_map)?;
                Ok(Expression::Unary(UnaryExpr {
                    op: unary.op.clone(),
                    expr: Box::new(new_expr),
                    span: unary.span,
                }))
            }
            Expression::Block(block) => {
                let mut new_statements = Vec::new();
                for stmt in &block.statements {
                    new_statements.push(self.substitute_statement(stmt, type_map)?);
                }
                let new_last_expr = match block.last_expr.as_ref() {
                    Some(expr) => Some(Box::new(self.substitute_expr(expr, type_map)?)),
                    None => None,
                };
                Ok(Expression::Block(BlockExpr {
                    statements: new_statements,
                    last_expr: new_last_expr,
                    span: block.span,
                }))
            }
            Expression::If(if_expr) => {
                let new_condition = self.substitute_expr(&if_expr.condition, type_map)?;
                let new_then = self.substitute_expr(&if_expr.then_branch, type_map)?;
                let new_else = match if_expr.else_branch.as_ref() {
                    Some(else_branch) => Some(Box::new(self.substitute_expr(else_branch, type_map)?)),
                    None => None,
                };
                Ok(Expression::If(IfExpr {
                    condition: Box::new(new_condition),
                    then_branch: Box::new(new_then),
                    else_branch: new_else,
                    span: if_expr.span,
                }))
            }
            Expression::Match(match_expr) => {
                let new_expr = self.substitute_expr(&match_expr.expr, type_map)?;
                let mut new_arms = Vec::new();
                for arm in &match_expr.arms {
                    new_arms.push(MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref()
                            .map(|guard| self.substitute_expr(guard, type_map))
                            .transpose()?,
                        expr: self.substitute_expr(&arm.expr, type_map)?,
                    });
                }
                Ok(Expression::Match(MatchExpr {
                    expr: Box::new(new_expr),
                    arms: new_arms,
                    span: match_expr.span,
                }))
            }
            Expression::Array(array) => {
                let mut new_elements = Vec::new();
                for elem in &array.elements {
                    new_elements.push(self.substitute_expr(elem, type_map)?);
                }
                Ok(Expression::Array(ArrayExpr {
                    elements: new_elements,
                    span: array.span,
                }))
            }
            Expression::Tuple(tuple) => {
                let mut new_elements = Vec::new();
                for elem in &tuple.elements {
                    new_elements.push(self.substitute_expr(elem, type_map)?);
                }
                Ok(Expression::Tuple(TupleExpr {
                    elements: new_elements,
                    span: tuple.span,
                }))
            }
            Expression::Index(index) => {
                let new_object = self.substitute_expr(&index.object, type_map)?;
                let new_index = self.substitute_expr(&index.index, type_map)?;
                Ok(Expression::Index(IndexExpr {
                    object: Box::new(new_object),
                    index: Box::new(new_index),
                    span: index.span,
                }))
            }
            Expression::Field(field) => {
                let new_object = self.substitute_expr(&field.object, type_map)?;
                Ok(Expression::Field(FieldExpr {
                    object: Box::new(new_object),
                    field: field.field.clone(),
                    span: field.span,
                }))
            }
            Expression::MethodCall(method) => {
                let new_object = self.substitute_expr(&method.object, type_map)?;
                let mut new_args = Vec::new();
                for arg in &method.args {
                    new_args.push(self.substitute_expr(arg, type_map)?);
                }
                Ok(Expression::MethodCall(MethodCallExpr {
                    object: Box::new(new_object),
                    method: method.method.clone(),
                    args: new_args,
                    span: method.span,
                }))
            }
            Expression::Cast(cast) => {
                let new_expr = self.substitute_expr(&cast.expr, type_map)?;
                let new_ty = self.substitute_type(&cast.ty, type_map);
                Ok(Expression::Cast(CastExpr {
                    expr: Box::new(new_expr),
                    ty: new_ty,
                    span: cast.span,
                }))
            }
            Expression::Reference(ref_expr) => {
                let new_expr = self.substitute_expr(&ref_expr.expr, type_map)?;
                Ok(Expression::Reference(ReferenceExpr {
                    expr: Box::new(new_expr),
                    is_mut: ref_expr.is_mut,
                    span: ref_expr.span,
                }))
            }
            Expression::Dereference(deref) => {
                let new_expr = self.substitute_expr(&deref.expr, type_map)?;
                Ok(Expression::Dereference(DereferenceExpr {
                    expr: Box::new(new_expr),
                    span: deref.span,
                }))
            }
            Expression::ListLiteral(list) => {
                let mut new_elements = Vec::new();
                for elem in &list.elements {
                    new_elements.push(self.substitute_expr(elem, type_map)?);
                }
                Ok(Expression::ListLiteral(ListLiteral {
                    type_name: list.type_name.clone(),
                    elements: new_elements,
                    span: list.span,
                }))
            }
            Expression::MapLiteral(map) => {
                let mut new_pairs = Vec::new();
                for (key, value) in &map.pairs {
                    new_pairs.push((
                        self.substitute_expr(key, type_map)?,
                        self.substitute_expr(value, type_map)?,
                    ));
                }
                Ok(Expression::MapLiteral(MapLiteral {
                    type_name: map.type_name.clone(),
                    pairs: new_pairs,
                    span: map.span,
                }))
            }
            // リテラルや識別子はそのまま
            _ => Ok(expr.clone()),
        }
    }
}