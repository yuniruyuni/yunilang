/// 構造体フィールドアクセスのテスト

use yunilang::{
    analyzer::SemanticAnalyzer,
    codegen::CodeGenerator,
    lexer::{Lexer, TokenWithPosition},
    parser::Parser,
    ast::Program,
};
use inkwell::context::Context;

/// 基本的な構造体フィールドアクセスのテスト
#[test]
fn test_struct_field_access() {
    let input = r#"
package test

type Point struct {
    x: i32,
    y: i32,
}

fn main() {
    let p: Point = Point { x: 10i32, y: 20i32 };
    let x: i32 = p.x;
    let y: i32 = p.y;
    println("x = {}, y = {}", x, y);
}
"#;

    // レクサー
    let lexer = Lexer::new(input);
    let tokens: Vec<TokenWithPosition> = lexer.collect_tokens();
    
    // パーサー
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing failed");
    
    // 解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast).expect("Analysis failed");
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test");
    codegen.compile_program(&ast).expect("Code generation failed");
    let module = codegen.get_module();
    
    // モジュールが正しく生成されたことを確認
    assert!(module.verify().is_ok());
    
    // main関数が存在することを確認
    let main_fn = module.get_function("main").expect("main function not found");
    assert_eq!(main_fn.get_name().to_str(), Ok("main"));
}

/// ネストした構造体フィールドアクセスのテスト
#[test]
fn test_nested_struct_field_access() {
    let input = r#"
package test

type Point struct {
    x: i32,
    y: i32,
}

type Rectangle struct {
    top_left: Point,
    bottom_right: Point,
}

fn main() {
    let rect: Rectangle = Rectangle {
        top_left: Point { x: 0i32, y: 0i32 },
        bottom_right: Point { x: 100i32, y: 100i32 },
    };
    
    let x1: i32 = rect.top_left.x;
    let y1: i32 = rect.top_left.y;
    let x2: i32 = rect.bottom_right.x;
    let y2: i32 = rect.bottom_right.y;
    
    println("Rectangle: ({}, {}) - ({}, {})", x1, y1, x2, y2);
}
"#;

    // レクサー
    let lexer = Lexer::new(input);
    let tokens: Vec<TokenWithPosition> = lexer.collect_tokens();
    
    // パーサー
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing failed");
    
    // 解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast).expect("Analysis failed");
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test");
    codegen.compile_program(&ast).expect("Code generation failed");
    let module = codegen.get_module();
    
    // モジュールが正しく生成されたことを確認
    assert!(module.verify().is_ok());
}

/// 構造体フィールドへの代入のテスト
#[test]
fn test_struct_field_assignment() {
    let input = r#"
package test

type Point struct {
    x: i32,
    y: i32,
}

fn main() {
    let mut p: Point = Point { x: 10i32, y: 20i32 };
    p.x = 30i32;
    p.y = 40i32;
    println("x = {}, y = {}", p.x, p.y);
}
"#;

    // レクサー
    let lexer = Lexer::new(input);
    let tokens: Vec<TokenWithPosition> = lexer.collect_tokens();
    
    // パーサー
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing failed");
    
    // 解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast).expect("Analysis failed");
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test");
    codegen.compile_program(&ast).expect("Code generation failed");
    let module = codegen.get_module();
    
    // モジュールが正しく生成されたことを確認
    assert!(module.verify().is_ok());
}

/// 異なる型の構造体フィールドアクセスのテスト
#[test]
fn test_mixed_type_struct_fields() {
    let input = r#"
package test

type Person struct {
    name: String,
    age: i32,
    height: f64,
    is_student: bool,
}

fn main() {
    let person: Person = Person {
        name: "Alice",
        age: 25i32,
        height: 165.5,
        is_student: false,
    };
    
    let name: String = person.name;
    let age: i32 = person.age;
    let height: f64 = person.height;
    let is_student: bool = person.is_student;
    
    println("Name: {}, Age: {}, Height: {}, Student: {}", name, age, height, is_student);
}
"#;

    // レクサー
    let lexer = Lexer::new(input);
    let tokens: Vec<TokenWithPosition> = lexer.collect_tokens();
    
    // パーサー
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing failed");
    
    // 解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast).expect("Analysis failed");
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test");
    codegen.compile_program(&ast).expect("Code generation failed");
    let module = codegen.get_module();
    
    // モジュールが正しく生成されたことを確認
    assert!(module.verify().is_ok());
}

/// 構造体フィールドアクセスのエラーケースのテスト
#[test]
fn test_struct_field_error_cases() {
    // 存在しないフィールドへのアクセス
    let input = r#"
package test

type Point struct {
    x: i32,
    y: i32,
}

fn main() {
    let p: Point = Point { x: 10i32, y: 20i32 };
    let z: i32 = p.z;  // エラー: フィールドzは存在しない
}
"#;

    let lexer = Lexer::new(input);
    let tokens: Vec<TokenWithPosition> = lexer.collect_tokens();
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing failed");
    
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast).expect("Analysis failed");
    
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test");
    
    // コード生成がエラーになることを確認
    assert!(codegen.compile_program(&ast).is_err());
}

/// 参照経由のフィールドアクセステスト
#[test]
fn test_reference_field_access() {
    let input = r#"
package test

type Point struct {
    x: i32,
    y: i32,
}

fn main() {
    let p: Point = Point { x: 10i32, y: 20i32 };
    let p_ref: &Point = &p;
    let x: i32 = p_ref.x;
    println("x = {}", x);
}
"#;

    // レクサー
    let lexer = Lexer::new(input);
    let tokens: Vec<TokenWithPosition> = lexer.collect_tokens();
    
    // パーサー
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().expect("Parsing failed");
    
    // 解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast).expect("Analysis failed");
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "test");
    codegen.compile_program(&ast).expect("Code generation failed");
    let module = codegen.get_module();
    
    // モジュールが正しく生成されたことを確認
    assert!(module.verify().is_ok());
}