//! スコープ管理とユーティリティ関数

use crate::ast::*;
use crate::analyzer::symbol::{Scope, Symbol, TypeInfo};
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 変数の検索
    pub fn lookup_variable(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return Some(symbol);
            }
        }
        None
    }
    
    /// 型の検索
    pub fn lookup_type(&self, name: &str) -> Option<&TypeInfo> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(type_info) = scope.lookup_type(name) {
                return Some(type_info);
            }
        }
        None
    }
    
    pub fn enter_scope(&mut self) {
        let new_scope = Scope::new();
        self.scope_stack.push(new_scope);
        self.lifetime_context.enter_scope();
    }
    
    pub fn exit_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
        self.lifetime_context.exit_scope();
    }
    
    #[allow(dead_code)]
    pub fn get_statement_span(&self, stmt: &Statement) -> Span {
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
    
    pub fn get_expression_span(&self, expr: &Expression) -> Span {
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
            Expression::Match(m) => m.span,
            Expression::If(i) => i.span,
            Expression::Block(b) => b.span,
        }
    }
}