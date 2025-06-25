//! パーサーテスト
//! 
//! Yuniコンパイラのパーサー（構文解析器）の包括的なテストスイート。
//! 各種構文、エラーハンドリング、演算子優先順位を網羅する。
//! 
//! 実際のテストはサブモジュールに分割されています：
//! - basic_test: 基本的なプログラム構造
//! - expression_test: 式の解析
//! - statement_test: 文の解析
//! - type_test: 型定義（構造体、列挙型、エイリアス）
//! - control_flow_test: 制御フロー（if/while/for）
//! - function_test: 関数定義と呼び出し
//! - template_string_test: テンプレート文字列
//! - error_test: エラーケース
//! - visibility_test: 可視性修飾子

#[cfg(test)]
mod parser;