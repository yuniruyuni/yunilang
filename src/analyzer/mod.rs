//! セマンティック解析モジュール
//!
//! このモジュールは型チェック、名前解決、ライフタイム解析、
//! その他のセマンティック検証を行います。

mod analyzer;
mod borrow_checker;
mod lifetime;
mod symbol;
mod type_checker;

// 公開API
pub use analyzer::SemanticAnalyzer;
pub use symbol::{AnalysisError, AnalysisResult, FunctionSignature, Symbol, TypeInfo, TypeKind};
pub use symbol::LifetimeId;

// 後方互換性のための型エイリアス
pub type Analyzer = SemanticAnalyzer;