//! セマンティック解析テスト
//! 
//! Yuniコンパイラのセマンティック解析器の包括的なテストスイート。
//! 型チェック、スコープ解決、所有権検証、ライフタイム検証を網羅する。
//! 
//! 実際のテストはサブモジュールに分割されています：
//! - type_checking_test: 基本的な型チェック
//! - scope_test: 変数・関数スコープ
//! - struct_enum_test: 構造体と列挙型
//! - control_flow_test: 制御フローと到達不能コード検出
//! - error_variable_function_test: 変数・関数エラー
//! - error_type_test: 型エラー
//! - method_test: メソッド関連
//! - advanced_test: 高度なテスト（循環依存等）

#[cfg(test)]
mod analyzer;