//! セマンティック解析モジュール
//!
//! このモジュールは型チェック、名前解決、ライフタイム解析、
//! その他のセマンティック検証を行います。

pub mod semantic_analyzer;
mod borrow_checker;
mod lifetime;
pub mod monomorphization;
mod symbol;
mod tail_position;
mod type_checker;
mod type_env;
mod type_inference;

// 公開API
pub use semantic_analyzer::SemanticAnalyzer;
pub use monomorphization::monomorphize_program;