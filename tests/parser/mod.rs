//! パーサーテストの共通モジュール
//! 
//! パーサーテストで使用する共通のヘルパー関数と型を定義する。

use yunilang::ast::*;
use yunilang::lexer::Lexer;
use yunilang::parser::{Parser, ParseError};

/// ソースコードを解析してASTを取得するヘルパー関数
pub fn parse_source(source: &str) -> Result<Program, ParseError> {
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// 解析に成功することを確認するヘルパー関数
pub fn assert_parse_success(source: &str) -> Program {
    parse_source(source).expect("Parsing should succeed")
}

/// 解析に失敗することを確認するヘルパー関数
pub fn assert_parse_error(source: &str) {
    assert!(parse_source(source).is_err(), "Parsing should fail");
}

// サブモジュールの宣言
#[cfg(test)]
mod basic_test;
#[cfg(test)]
mod expression_test;
#[cfg(test)]
mod literal_test;
#[cfg(test)]
mod statement_test;
#[cfg(test)]
mod type_test;
#[cfg(test)]
mod control_flow_test;
#[cfg(test)]
mod function_test;
#[cfg(test)]
mod template_string_test;
#[cfg(test)]
mod template_string_advanced_test;
#[cfg(test)]
mod error_test;
#[cfg(test)]
mod visibility_test;