//! 型定義

use serde::{Deserialize, Serialize};

use super::Span;

/// 型の表現
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    // 基本型
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    F8,
    F16,
    F32,
    F64,
    Bool,
    Str,
    String,
    Void,

    // 参照型
    Reference(Box<Type>, bool), // type, is_mut

    // 複合型
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Function(FunctionType),

    // ユーザー定義型
    UserDefined(String),
}

/// 関数型
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

/// 型パラメータ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeParam {
    pub name: String,
    pub span: Span,
}

/// ライフタイム句
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivesClause {
    pub constraints: Vec<LivesConstraint>,
    pub span: Span,
}

/// ライフタイム制約
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivesConstraint {
    pub target: String,
    pub sources: Vec<String>,
    pub span: Span,
}