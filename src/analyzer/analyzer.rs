//! セマンティック解析器のメイン実装

use crate::ast::*;
use crate::error::{AnalyzerError, ErrorCollector, YuniError, YuniResult};
use std::collections::HashMap;

use super::borrow_checker::BorrowChecker;
use super::lifetime::{LifetimeContext, UsageKind};
use super::symbol::{AnalysisError, AnalysisResult, FunctionSignature, Scope, Symbol, TypeInfo, TypeKind};
use super::type_checker::TypeChecker;

/// セマンティック解析器
pub struct SemanticAnalyzer {
    /// スコープスタック
    scope_stack: Vec<Scope>,
    /// 型チェッカー
    type_checker: TypeChecker,
    /// インポートエイリアス
    imports: HashMap<String, String>,
    /// 現在の関数の戻り値型（return文のチェック用）
    current_return_type: Option<Type>,
    /// 現在の関数のライフタイムコンテキスト
    lifetime_context: LifetimeContext,
    /// 収集されたエラー
    errors: Vec<AnalysisError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scope_stack: vec![Scope::new()],
            type_checker: TypeChecker::new(),
            imports: HashMap::new(),
            current_return_type: None,
            lifetime_context: LifetimeContext::new(),
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
            Err(self.errors[0].clone())
        }
    }

    fn process_import(&mut self, import: &Import) {
        let alias = import.alias.as_ref().unwrap_or(&import.path);
        self.imports.insert(alias.clone(), import.path.clone());
    }

    fn collect_type_definition(&mut self, type_def: &TypeDef) -> AnalysisResult<()> {
        match type_def {
            TypeDef::Struct(struct_def) => self.collect_struct_definition(struct_def),
            TypeDef::Enum(enum_def) => self.collect_enum_definition(enum_def),
        }
    }

    fn collect_struct_definition(&mut self, struct_def: &StructDef) -> AnalysisResult<()> {
        // フィールドの型を検証
        for field in &struct_def.fields {
            self.type_checker.validate_type(&field.ty, field.span)?;
        }

        let type_info = TypeInfo {
            name: struct_def.name.clone(),
            kind: TypeKind::Struct(struct_def.fields.clone()),
            methods: HashMap::new(),
            span: struct_def.span,
        };

        self.type_checker.register_type(type_info)
    }

    fn collect_enum_definition(&mut self, enum_def: &EnumDef) -> AnalysisResult<()> {
        // バリアントのフィールド型を検証
        for variant in &enum_def.variants {
            for field in &variant.fields {
                self.type_checker.validate_type(&field.ty, field.span)?;
            }
        }

        let type_info = TypeInfo {
            name: enum_def.name.clone(),
            kind: TypeKind::Enum(enum_def.variants.clone()),
            methods: HashMap::new(),
            span: enum_def.span,
        };

        self.type_checker.register_type(type_info)
    }

    fn collect_function_signature(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        // パラメータの型を検証
        for param in &func.params {
            self.type_checker.validate_type(&param.ty, param.span)?;
        }

        // 戻り値型を検証
        if let Some(ref return_type) = func.return_type {
            self.type_checker.validate_type(return_type, func.span)?;
        }

        let func_sig = FunctionSignature {
            name: func.name.clone(),
            params: func.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
            return_type: func.return_type.as_ref().cloned().unwrap_or(Type::Void),
            lives_clause: func.lives_clause.clone(),
            is_method: false,
            receiver_type: None,
            span: func.span,
        };

        self.type_checker.register_function(func_sig)
    }

    fn collect_method_signature(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // レシーバの型を検証
        self.type_checker.validate_type(&method.receiver.ty, method.receiver.span)?;

        // パラメータの型を検証
        for param in &method.params {
            self.type_checker.validate_type(&param.ty, param.span)?;
        }

        // 戻り値型を検証
        if let Some(ref return_type) = method.return_type {
            self.type_checker.validate_type(return_type, method.span)?;
        }

        // メソッドを型のメソッドテーブルに追加
        if let Type::UserDefined(name) = &method.receiver.ty {
            let func_sig = FunctionSignature {
                name: method.name.clone(),
                params: method.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
                return_type: method.return_type.as_ref().cloned().unwrap_or(Type::Void),
                lives_clause: method.lives_clause.clone(),
                is_method: true,
                receiver_type: Some(method.receiver.ty.clone()),
                span: method.span,
            };

            // TODO: メソッドを型情報に追加
        }

        Ok(())
    }

    fn analyze_function(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        // 新しいスコープを開始
        self.enter_scope();
        self.current_return_type = func.return_type.clone();

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

        // 関数本体を解析
        let returns = self.analyze_block(&func.body)?;

        // 戻り値の確認
        if !returns && func.return_type.is_some() {
            self.errors.push(AnalysisError::MissingReturn {
                name: func.name.clone(),
                span: func.span,
            });
        }

        // 借用チェック
        let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, self.scope_stack.last_mut().unwrap());
        if let Err(e) = borrow_checker.check() {
            self.errors.push(e);
        }

        self.current_return_type = None;
        self.exit_scope();

        Ok(())
    }

    fn analyze_method(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // 新しいスコープを開始
        self.enter_scope();
        self.current_return_type = method.return_type.clone();

        // レシーバをスコープに追加
        let receiver_name = method.receiver.name.as_deref().unwrap_or("self");
        let is_mut_ref = matches!(&method.receiver.ty, Type::Reference(_, true));
        let symbol = Symbol {
            name: receiver_name.to_string(),
            ty: method.receiver.ty.clone(),
            is_mutable: is_mut_ref,
            span: method.receiver.span,
            borrow_info: None,
            is_moved: false,
            lifetime: None,
        };
        self.scope_stack.last_mut().unwrap().define(symbol)?;

        // パラメータをスコープに追加
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

        // メソッド本体を解析
        let returns = self.analyze_block(&method.body)?;

        // 戻り値の確認
        if !returns && method.return_type.is_some() {
            self.errors.push(AnalysisError::MissingReturn {
                name: method.name.clone(),
                span: method.span,
            });
        }

        // 借用チェック
        let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, self.scope_stack.last_mut().unwrap());
        if let Err(e) = borrow_checker.check() {
            self.errors.push(e);
        }

        self.current_return_type = None;
        self.exit_scope();

        Ok(())
    }

    fn analyze_block(&mut self, block: &Block) -> AnalysisResult<bool> {
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

    fn analyze_statement(&mut self, stmt: &Statement) -> AnalysisResult<bool> {
        match stmt {
            Statement::Let(let_stmt) => {
                // TODO: implement let statement analysis
                Ok(false)
            }
            Statement::Assignment(assign) => {
                // TODO: implement assignment analysis
                Ok(false)
            }
            Statement::Return(ret) => {
                // TODO: implement return statement analysis
                Ok(true)
            }
            Statement::If(if_stmt) => {
                // TODO: implement if statement analysis
                Ok(false)
            }
            Statement::While(while_stmt) => {
                // TODO: implement while statement analysis
                Ok(false)
            }
            Statement::For(for_stmt) => {
                // TODO: implement for statement analysis
                Ok(false)
            }
            Statement::Expression(expr) => {
                // TODO: implement expression analysis
                Ok(false)
            }
            Statement::Block(block) => self.analyze_block(block),
        }
    }

    // 残りの実装は次のメッセージで続けます...
    
    fn enter_scope(&mut self) {
        let new_scope = Scope::new();
        self.scope_stack.push(new_scope);
        self.lifetime_context.enter_scope();
    }
    
    fn exit_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
        self.lifetime_context.exit_scope();
    }
    
    fn get_statement_span(&self, stmt: &Statement) -> Span {
        match stmt {
            Statement::Let(s) => s.span,
            Statement::Assignment(s) => s.span,
            Statement::Return(s) => s.span,
            Statement::If(s) => s.span,
            Statement::While(s) => s.span,
            Statement::For(s) => s.span,
            Statement::Expression(e) => self.get_expression_span(e),
            Statement::Block(b) => b.span,
        }
    }
    
    fn get_expression_span(&self, expr: &Expression) -> Span {
        match expr {
            Expression::Integer(i) => i.span,
            Expression::Float(f) => f.span,
            Expression::String(s) => s.span,
            Expression::TemplateString(t) => t.span,
            Expression::Boolean(b) => b.span,
            Expression::Identifier(i) => i.span,
            Expression::Path(p) => p.span,
            Expression::Binary(b) => b.span,
            Expression::Unary(u) => u.span,
            Expression::Call(c) => c.span,
            Expression::MethodCall(m) => m.span,
            Expression::Index(i) => i.span,
            Expression::Field(f) => f.span,
            Expression::Reference(r) => r.span,
            Expression::Dereference(d) => d.span,
            Expression::StructLit(s) => s.span,
            Expression::EnumVariant(e) => e.span,
            Expression::Array(a) => a.span,
            Expression::Tuple(t) => t.span,
            Expression::Cast(c) => c.span,
            Expression::Assignment(a) => a.span,
        }
    }
    
    // TODO: 残りのanalyze_*メソッドの実装
}