//! コード生成テストの共通モジュール
//! 
//! コード生成テストで使用する共通のヘルパー関数と型を定義する。

use yunilang::analyzer::SemanticAnalyzer;
use yunilang::codegen::CodeGenerator;
use yunilang::lexer::Lexer;
use yunilang::parser::Parser;

use inkwell::context::Context;

use std::fs;
use std::process::Command;

/// ソースコードを完全にコンパイルしてLLVM IRを生成するヘルパー関数
pub fn compile_to_ir(source: &str, module_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 字句解析
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect_tokens();
    
    // 構文解析
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    // セマンティック解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, module_name);
    codegen.compile_program(&ast)?;
    
    // LLVM IRを文字列として取得
    Ok(codegen.get_module().print_to_string().to_string())
}

/// LLVM IRが有効であることを確認するヘルパー関数
pub fn assert_valid_ir(ir: &str) {
    // 基本的な構造チェック
    assert!(ir.contains("target"), "IR should contain target information");
    assert!(ir.contains("define"), "IR should contain function definitions");
    
    // 基本ブロックの整合性チェック
    let define_count = ir.matches("define").count();
    let ret_count = ir.matches("ret").count();
    assert!(ret_count >= define_count, "Each function should have at least one return");
}

/// コンパイルに成功することを確認するヘルパー関数
pub fn assert_compile_success(source: &str, module_name: &str) -> String {
    compile_to_ir(source, module_name).expect("Compilation should succeed")
}

/// コンパイルに失敗することを確認するヘルパー関数
pub fn assert_compile_error(source: &str, module_name: &str) {
    assert!(compile_to_ir(source, module_name).is_err(), "Compilation should fail");
}

// サブモジュールの宣言
#[cfg(test)]
mod basic_test;
#[cfg(test)]
mod arithmetic_test;
#[cfg(test)]
mod variable_test;
#[cfg(test)]
mod control_flow_test;
#[cfg(test)]
mod data_structures_test;
#[cfg(test)]
mod misc_test;
#[cfg(test)]
mod advanced_test;