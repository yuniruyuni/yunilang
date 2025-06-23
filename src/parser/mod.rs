//! パーサーモジュール
//!
//! このモジュールはトークンを抽象構文木（AST）に解析する責任を持ちます。
//! 再帰下降構文解析を使用し、適切な優先順位処理を行います。

mod decl_parser;
mod expr_parser;
mod parser_impl;
mod pattern_parser;
mod stmt_parser;
mod type_parser;

// 公開API
pub use parser_impl::Parser;

// 後方互換性のための型エイリアス
use crate::error::ParserError;
pub type ParseError = ParserError;
pub type ParseResult<T> = Result<T, ParseError>;