//! シンボルテーブルおよび関連するデータ構造

use crate::ast::{Field, LivesClause, Span, Type, Variant};
use crate::error::AnalyzerError;
use std::collections::HashMap;

pub type AnalysisError = AnalyzerError;
pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// シンボル情報
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub is_mutable: bool,
    pub span: Span,
    /// 変数が借用されているかどうか
    pub borrow_info: Option<BorrowInfo>,
    /// 変数が移動されたかどうか
    pub is_moved: bool,
    /// 変数のライフタイム（参照の場合）
    pub lifetime: Option<LifetimeId>,
}

/// 関数シグネチャ情報
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub lives_clause: Option<LivesClause>,
    pub is_method: bool,
    pub receiver_type: Option<Type>,
    pub span: Span,
}

/// 型定義情報
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub methods: HashMap<String, FunctionSignature>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct(Vec<Field>),
    Enum(Vec<Variant>),
    Builtin,
}

/// ライフタイムID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifetimeId {
    Static,
    Named(usize),
}

/// 借用情報
#[derive(Debug, Clone)]
pub struct BorrowInfo {
    pub kind: BorrowKind,
    pub lifetime: LifetimeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

/// 変数のスコープ管理
#[derive(Debug)]
pub struct Scope {
    symbols: HashMap<String, Symbol>,
    types: HashMap<String, TypeInfo>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            types: HashMap::new(),
            parent: None,
        }
    }

    pub fn with_parent(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            types: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn define(&mut self, symbol: Symbol) -> AnalysisResult<()> {
        if self.symbols.contains_key(&symbol.name) {
            return Err(AnalysisError::DuplicateVariable {
                name: symbol.name.clone(),
                span: symbol.span,
            });
        }
        self.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(name)))
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        if self.symbols.contains_key(name) {
            self.symbols.get_mut(name)
        } else {
            self.parent.as_mut().and_then(|parent| parent.lookup_mut(name))
        }
    }

    pub fn mark_moved(&mut self, name: &str) -> AnalysisResult<()> {
        match self.lookup_mut(name) {
            Some(symbol) => {
                symbol.is_moved = true;
                Ok(())
            }
            None => Err(AnalysisError::UndefinedVariable {
                name: name.to_string(),
                span: Span::dummy(),
            }),
        }
    }

    pub fn mark_borrowed(&mut self, name: &str, borrow_info: BorrowInfo) -> AnalysisResult<()> {
        match self.lookup_mut(name) {
            Some(symbol) => {
                symbol.borrow_info = Some(borrow_info);
                Ok(())
            }
            None => Err(AnalysisError::UndefinedVariable {
                name: name.to_string(),
                span: Span::dummy(),
            }),
        }
    }

    pub fn define_type(&mut self, type_info: TypeInfo) -> AnalysisResult<()> {
        if self.types.contains_key(&type_info.name) {
            return Err(AnalysisError::DuplicateType {
                name: type_info.name.clone(),
                span: type_info.span,
            });
        }
        self.types.insert(type_info.name.clone(), type_info);
        Ok(())
    }

    pub fn lookup_type(&self, name: &str) -> Option<&TypeInfo> {
        self.types
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup_type(name)))
    }
}