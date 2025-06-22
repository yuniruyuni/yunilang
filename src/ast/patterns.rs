//! パターンマッチングの定義

use serde::{Deserialize, Serialize};

/// パターン
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// 識別子パターン（変数バインディング）
    Identifier(String, bool), // name, is_mutable
    
    /// タプルパターン
    Tuple(Vec<Pattern>),
    
    /// 構造体パターン
    Struct(String, Vec<(String, Pattern)>), // struct_name, fields
}