//! レキシカル解析モジュール
//!
//! このモジュールはYuniソースコードをトークンストリームにトークン化する責任を持ちます。
//! キーワード、識別子、リテラル、演算子、補間付きテンプレート文字列など、
//! すべてのYuni言語機能をサポートしています。

mod lexer;
mod literal_parser;
mod template_string;
mod token;

// 公開API
pub use lexer::{tokenize, Lexer, TokenWithPosition};
pub use token::Token;