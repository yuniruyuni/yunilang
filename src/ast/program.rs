//! プログラム構造

use serde::{Deserialize, Serialize};

use super::{Item, Span};

/// ASTのルートノード（完全なYuniプログラムを表す）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub package: PackageDecl,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
    pub span: Span,
}

/// パッケージ宣言
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageDecl {
    pub name: String,
    pub span: Span,
}

/// インポート文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
    pub span: Span,
}