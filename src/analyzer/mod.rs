//! セマンティック解析モジュール
//!
//! このモジュールは型チェック、名前解決、ライフタイム解析、
//! その他のセマンティック検証を行います。

mod semantic_analyzer;
mod borrow_checker;
mod lifetime;
mod symbol;
mod type_checker;

// 公開API
pub use semantic_analyzer::SemanticAnalyzer;