//! ジェネリック定義の収集とインスタンス化の検出

use std::collections::HashMap;
use crate::ast::*;
use crate::error::YuniResult;
use super::{Monomorphizer, InstantiationType};

impl Monomorphizer {
    /// ジェネリック定義を収集
    pub(super) fn collect_generic_definitions(&mut self) {
        for item in &self.original_program.items {
            match item {
                Item::Function(func) if !func.type_params.is_empty() => {
                    self.generic_functions.insert(func.name.clone(), func.clone());
                }
                Item::TypeDef(TypeDef::Struct(s)) if !s.type_params.is_empty() => {
                    self.generic_structs.insert(s.name.clone(), s.clone());
                }
                Item::TypeDef(TypeDef::Enum(e)) if !e.type_params.is_empty() => {
                    self.generic_enums.insert(e.name.clone(), e.clone());
                }
                Item::TypeDef(TypeDef::Alias(_)) => {
                    // ジェネリック型エイリアスの単相化は現時点では未対応
                }
                _ => {}
            }
        }
    }
    
    /// プログラム全体を走査してジェネリックの使用箇所を検出
    pub(super) fn collect_instantiations(&mut self, program: &Program) -> YuniResult<()> {
        for item in &program.items {
            match item {
                Item::Function(func) => self.collect_instantiations_in_function(func)?,
                Item::Method(method) => self.collect_instantiations_in_method(method)?,
                _ => {}
            }
        }
        Ok(())
    }
    
    /// 関数内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        // 型パラメータのマッピングを作成
        let type_params: HashMap<String, Type> = func.type_params.iter()
            .map(|p| (p.name.clone(), Type::Variable(p.name.clone())))
            .collect();
            
        self.collect_instantiations_in_block(&func.body, &type_params)
    }
    
    /// メソッド内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_method(&mut self, method: &MethodDecl) -> YuniResult<()> {
        // 型パラメータのマッピングを作成
        let type_params: HashMap<String, Type> = method.type_params.iter()
            .map(|p| (p.name.clone(), Type::Variable(p.name.clone())))
            .collect();
            
        self.collect_instantiations_in_block(&method.body, &type_params)
    }
    
    /// ブロック内でのジェネリックの使用箇所を検出
    pub(super) fn collect_instantiations_in_block(&mut self, block: &Block, type_params: &HashMap<String, Type>) -> YuniResult<()> {
        for stmt in &block.statements {
            self.collect_instantiations_in_statement(stmt, type_params)?;
        }
        Ok(())
    }
    
    /// 文内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_statement(&mut self, stmt: &Statement, type_params: &HashMap<String, Type>) -> YuniResult<()> {
        match stmt {
            Statement::Let(let_stmt) => {
                if let Some(init) = &let_stmt.init {
                    self.collect_instantiations_in_expr(init, type_params)?;
                }
                if let Some(ty) = &let_stmt.ty {
                    self.collect_instantiations_in_type(ty, type_params)?;
                }
            }
            Statement::Assignment(assign) => {
                self.collect_instantiations_in_expr(&assign.target, type_params)?;
                self.collect_instantiations_in_expr(&assign.value, type_params)?;
            }
            Statement::Expression(expr) => {
                self.collect_instantiations_in_expr(expr, type_params)?;
            }
            Statement::Return(ret_stmt) => {
                if let Some(value) = &ret_stmt.value {
                    self.collect_instantiations_in_expr(value, type_params)?;
                }
            }
            Statement::If(if_stmt) => {
                self.collect_instantiations_in_expr(&if_stmt.condition, type_params)?;
                self.collect_instantiations_in_block(&if_stmt.then_branch, type_params)?;
                if let Some(ElseBranch::Block(else_block)) = &if_stmt.else_branch {
                    self.collect_instantiations_in_block(else_block, type_params)?;
                }
            }
            Statement::While(while_stmt) => {
                self.collect_instantiations_in_expr(&while_stmt.condition, type_params)?;
                self.collect_instantiations_in_block(&while_stmt.body, type_params)?;
            }
            Statement::For(for_stmt) => {
                if let Some(init) = &for_stmt.init {
                    self.collect_instantiations_in_statement(init, type_params)?;
                }
                if let Some(condition) = &for_stmt.condition {
                    self.collect_instantiations_in_expr(condition, type_params)?;
                }
                if let Some(update) = &for_stmt.update {
                    self.collect_instantiations_in_expr(update, type_params)?;
                }
                self.collect_instantiations_in_block(&for_stmt.body, type_params)?;
            }
            Statement::Block(block) => {
                self.collect_instantiations_in_block(block, type_params)?;
            }
        }
        Ok(())
    }
    
    /// 式内でのジェネリックの使用箇所を検出
    pub(super) fn collect_instantiations_in_expr(&mut self, expr: &Expression, type_params: &HashMap<String, Type>) -> YuniResult<()> {
        match expr {
            Expression::Call(call) => {
                // 呼び出し先の式を確認
                if let Expression::Identifier(ident) = &*call.callee {
                    // ジェネリック関数の呼び出しかチェック
                    if self.generic_functions.contains_key(&ident.name) {
                        // TODO: 型推論結果から実際の型引数を取得
                        // 現在は単純化のため、引数から推論
                        let type_args = self.infer_type_args_from_call(&ident.name, &call.args)?;
                        if !type_args.is_empty() {
                            self.queue_instantiation(&ident.name, type_args, InstantiationType::Function);
                        }
                    }
                }
                
                // 引数も再帰的に処理
                for arg in &call.args {
                    self.collect_instantiations_in_expr(arg, type_params)?;
                }
            }
            Expression::StructLit(struct_lit) => {
                // ジェネリック構造体のインスタンス化かチェック
                if self.generic_structs.contains_key(&struct_lit.name) {
                    // TODO: フィールドの型から型引数を推論
                    let type_args = self.infer_type_args_from_struct_lit(struct_lit)?;
                    if !type_args.is_empty() {
                        self.queue_instantiation(&struct_lit.name, type_args, InstantiationType::Struct);
                    }
                }
                
                // フィールドの値も再帰的に処理
                for field in &struct_lit.fields {
                    self.collect_instantiations_in_expr(&field.value, type_params)?;
                }
            }
            Expression::Binary(binary) => {
                self.collect_instantiations_in_expr(&binary.left, type_params)?;
                self.collect_instantiations_in_expr(&binary.right, type_params)?;
            }
            Expression::Block(block) => {
                for stmt in &block.statements {
                    self.collect_instantiations_in_statement(stmt, type_params)?;
                }
                if let Some(last_expr) = &block.last_expr {
                    self.collect_instantiations_in_expr(last_expr, type_params)?;
                }
            }
            Expression::If(if_expr) => {
                self.collect_instantiations_in_expr(&if_expr.condition, type_params)?;
                self.collect_instantiations_in_expr(&if_expr.then_branch, type_params)?;
                if let Some(else_branch) = &if_expr.else_branch {
                    self.collect_instantiations_in_expr(else_branch, type_params)?;
                }
            }
            Expression::Match(match_expr) => {
                self.collect_instantiations_in_expr(&match_expr.expr, type_params)?;
                for arm in &match_expr.arms {
                    self.collect_instantiations_in_expr(&arm.expr, type_params)?;
                }
            }
            // 他の式タイプも必要に応じて処理
            _ => {}
        }
        Ok(())
    }
    
    /// 型内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_type(&mut self, ty: &Type, _type_params: &HashMap<String, Type>) -> YuniResult<()> {
        match ty {
            Type::Generic(name, args) => {
                // ジェネリック型の使用を検出
                if self.generic_structs.contains_key(name) {
                    self.queue_instantiation(name, args.clone(), InstantiationType::Struct);
                } else if self.generic_enums.contains_key(name) {
                    self.queue_instantiation(name, args.clone(), InstantiationType::Enum);
                }
                
                // 型引数も再帰的に処理
                for arg in args {
                    self.collect_instantiations_in_type(arg, _type_params)?;
                }
            }
            Type::Array(elem) => {
                self.collect_instantiations_in_type(elem, _type_params)?;
            }
            Type::Reference(inner, _) => {
                self.collect_instantiations_in_type(inner, _type_params)?;
            }
            Type::Tuple(elems) => {
                for elem in elems {
                    self.collect_instantiations_in_type(elem, _type_params)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// インスタンス化をキューに追加
    pub(super) fn queue_instantiation(&mut self, name: &str, type_args: Vec<Type>, inst_type: InstantiationType) {
        let key = (name.to_string(), type_args.clone());
        
        // 既に単相化済みかチェック
        let already_monomorphized = match inst_type {
            InstantiationType::Function => self.monomorphized_functions.contains(&key),
            InstantiationType::Struct | InstantiationType::Enum => self.monomorphized_structs.contains(&key),
        };
        
        if !already_monomorphized {
            self.instantiation_queue.push((name.to_string(), type_args, inst_type));
            
            // キューに追加したことを記録
            match inst_type {
                InstantiationType::Function => {
                    self.monomorphized_functions.insert(key);
                }
                InstantiationType::Struct | InstantiationType::Enum => {
                    self.monomorphized_structs.insert(key);
                }
            }
        }
    }
}