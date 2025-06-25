//! パーサーモジュール
//!
//! このモジュールはトークンを抽象構文木（AST）に解析する責任を持ちます。
//! 再帰下降構文解析を使用し、適切な優先順位処理を行います。
//!
//! ## Go-style型宣言構文
//!
//! Yuni言語ではGo言語スタイルの統一された型宣言構文を採用しています：
//!
//! ### 構造体定義
//! ```yuni
//! type Point struct {
//!     x: f64,
//!     y: f64,
//! }
//! ```
//!
//! ### 列挙型定義
//! ```yuni
//! type Result<T, E> enum {
//!     Ok { value: T },
//!     Err { error: E },
//! }
//! ```
//!
//! ### 型エイリアス
//! ```yuni
//! type UserID i32
//! type NodeRef &Node
//! ```
//!
//! すべての型定義は`type`キーワードで始まり、その後に型名、そして定義内容が続きます。
//! これにより、一貫性のある読みやすい構文を実現しています。

mod decl_parser;
mod expr;
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