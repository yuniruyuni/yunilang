//! コード生成モジュール
//!
//! このモジュールはASTからLLVM IRを生成する責任を持ちます。

mod code_generator;
mod expr_codegen;
mod runtime;
mod stmt_codegen;
mod symbol_table;
mod types;

// 公開API
pub use code_generator::CodeGenerator;