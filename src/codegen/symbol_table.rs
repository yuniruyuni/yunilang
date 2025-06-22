//! シンボルテーブルとスコープ管理

use crate::ast::Type;
use inkwell::values::PointerValue;
use std::collections::HashMap;

/// 変数とその型を追跡するシンボルテーブルエントリ
#[derive(Debug, Clone)]
pub struct Symbol<'ctx> {
    pub ptr: PointerValue<'ctx>,
    pub ty: Type,
    pub is_mutable: bool,
}

/// 変数のライフタイムを管理するスコープ
pub struct Scope<'ctx> {
    pub symbols: HashMap<String, Symbol<'ctx>>,
}

impl<'ctx> Scope<'ctx> {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }
    
    /// シンボルを定義
    pub fn define(&mut self, name: String, symbol: Symbol<'ctx>) {
        self.symbols.insert(name, symbol);
    }
    
    /// シンボルを検索
    pub fn lookup(&self, name: &str) -> Option<&Symbol<'ctx>> {
        self.symbols.get(name)
    }
    
    /// シンボルを検索（可変参照）
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol<'ctx>> {
        self.symbols.get_mut(name)
    }
}

/// 構造体のフィールド情報
#[derive(Debug, Clone)]
pub struct StructInfo {
    /// フィールド名からインデックスへのマッピング
    pub field_indices: HashMap<String, u32>,
    /// フィールドの型情報（AST型を保持）
    pub field_types: Vec<Type>,
}

impl StructInfo {
    pub fn new() -> Self {
        Self {
            field_indices: HashMap::new(),
            field_types: Vec::new(),
        }
    }
    
    /// フィールドを追加
    pub fn add_field(&mut self, name: String, ty: Type) {
        let index = self.field_types.len() as u32;
        self.field_indices.insert(name, index);
        self.field_types.push(ty);
    }
    
    /// フィールドのインデックスを取得
    pub fn get_field_index(&self, name: &str) -> Option<u32> {
        self.field_indices.get(name).copied()
    }
    
    /// フィールドの型を取得
    pub fn get_field_type(&self, index: usize) -> Option<&Type> {
        self.field_types.get(index)
    }
}

/// スコープマネージャー
pub struct ScopeManager<'ctx> {
    scopes: Vec<Scope<'ctx>>,
}

impl<'ctx> ScopeManager<'ctx> {
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new()], // グローバルスコープ
        }
    }
    
    /// 新しいスコープを開始
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }
    
    /// 現在のスコープを終了
    pub fn pop_scope(&mut self) -> Option<Scope<'ctx>> {
        if self.scopes.len() > 1 {
            self.scopes.pop()
        } else {
            None
        }
    }
    
    /// 変数を定義
    pub fn define(&mut self, name: String, symbol: Symbol<'ctx>) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.define(name, symbol);
        }
    }
    
    /// 変数を検索（全スコープから）
    pub fn lookup(&self, name: &str) -> Option<&Symbol<'ctx>> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return Some(symbol);
            }
        }
        None
    }
    
    /// 現在のスコープから変数を検索
    pub fn lookup_in_current_scope(&self, name: &str) -> Option<&Symbol<'ctx>> {
        self.scopes.last()?.lookup(name)
    }
}