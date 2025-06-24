//! 宣言の定義

use serde::{Deserialize, Serialize};

use super::{Block, LivesClause, Span, Type, TypeParam};

/// トップレベルアイテム
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Function(FunctionDecl),
    Method(MethodDecl),
    TypeDef(TypeDef),
}

/// 型定義（構造体または列挙型）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
}

/// 構造体定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// 構造体のフィールド
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// 列挙型定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub variants: Vec<Variant>,
    pub span: Span,
}

/// 列挙型のバリアント
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// 関数宣言
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub is_public: bool,
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Box<Type>>,
    pub lives_clause: Option<LivesClause>,
    pub body: Block,
    pub span: Span,
}

/// メソッド宣言
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodDecl {
    pub is_public: bool,
    pub name: String,
    pub type_params: Vec<TypeParam>,
    pub receiver: Receiver,
    pub params: Vec<Param>,
    pub return_type: Option<Box<Type>>,
    pub lives_clause: Option<LivesClause>,
    pub body: Block,
    pub span: Span,
}

/// 関数パラメータ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// メソッドレシーバー
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receiver {
    pub name: Option<String>,
    pub ty: Type,
    pub is_mut: bool,
    pub span: Span,
}