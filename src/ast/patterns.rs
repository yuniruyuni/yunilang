//! パターンマッチングの定義

use serde::{Deserialize, Serialize};

/// リテラルパターン用のリテラル値
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralPattern {
    Integer(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

/// パターン
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// 識別子パターン（変数バインディング）
    Identifier(String, bool), // name, is_mutable
    
    /// リテラルパターン
    Literal(LiteralPattern),
    
    /// タプルパターン
    Tuple(Vec<Pattern>),
    
    /// 構造体パターン
    Struct(String, Vec<(String, Pattern)>), // struct_name, fields
    
    /// 列挙型バリアントパターン
    EnumVariant {
        enum_name: String,
        variant: String,
        fields: EnumVariantPatternFields,
    },
    
    /// ワイルドカードパターン（_）
    Wildcard,
}

/// 列挙型バリアントパターンのフィールド定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnumVariantPatternFields {
    /// タプルライク（位置指定）フィールド: Some(x)
    Tuple(Vec<Pattern>),
    /// 構造体ライク（名前付き）フィールド: Some { value: x }
    Struct(Vec<(String, Pattern)>),
    /// フィールドなし: None
    Unit,
}