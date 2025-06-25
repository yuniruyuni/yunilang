//! コード生成テスト
//! 
//! Yuniコンパイラのコード生成器の包括的なテストスイート。
//! LLVM IR生成、最適化、実行時正当性を検証する。
//! 
//! 実際のテストはサブモジュールに分割されています：
//! - basic_test: 基本的なプログラム構造
//! - arithmetic_test: 算術演算と型操作
//! - variable_test: 変数とメモリ管理
//! - control_flow_test: 制御フロー
//! - data_structures_test: データ構造（構造体、配列、タプル）
//! - misc_test: その他の機能（文字列、ブール演算、最適化）
//! - advanced_test: 高度な機能（複雑なプログラム、実行可能ファイル生成）

#[cfg(test)]
mod codegen;