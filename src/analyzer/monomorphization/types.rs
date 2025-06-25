//! 単相化に関する型定義

use crate::ast::Type;

/// 単相化された関数の情報
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedFunction {
    /// 元の関数名
    pub original_name: String,
    /// 型引数の具体的な型へのマッピング
    pub type_args: Vec<Type>,
    /// マングルされた名前（例: Vec_i32_new）
    pub mangled_name: String,
}

/// 単相化された構造体の情報
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedStruct {
    /// 元の構造体名
    pub original_name: String,
    /// 型引数の具体的な型へのマッピング
    pub type_args: Vec<Type>,
    /// マングルされた名前（例: Vec_i32）
    pub mangled_name: String,
}

/// インスタンス化の種類
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstantiationType {
    Function,
    Struct,
    Enum,
}