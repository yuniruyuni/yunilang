//! 算術演算と型操作のコード生成テスト

use super::*;

#[test]
fn test_arithmetic_expressions_codegen() {
    // 算術式のコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let a = 10;
        let b = 20;
        let sum = a + b;
        let diff = a - b;
        let prod = a * b;
        let quot = a / b;
        let rem = a % b;
    }
    "#;
    
    let ir = assert_compile_success(source, "arithmetic");
    assert_valid_ir(&ir);
    
    // 算術演算のLLVM命令が含まれていることを確認
    assert!(ir.contains("add"), "Should contain add instruction");
    assert!(ir.contains("sub"), "Should contain sub instruction");
    assert!(ir.contains("mul"), "Should contain mul instruction");
    assert!(ir.contains("div") || ir.contains("sdiv"), "Should contain division instruction");
    assert!(ir.contains("rem") || ir.contains("srem"), "Should contain remainder instruction");
}

#[test]
fn test_floating_point_operations_codegen() {
    // 浮動小数点演算のコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let x: f64 = 3.14;
        let y: f64 = 2.71;
        let sum = x + y;
        let prod = x * y;
        let is_greater = x > y;
    }
    "#;
    
    let ir = assert_compile_success(source, "float_ops");
    assert_valid_ir(&ir);
    
    // 浮動小数点演算のLLVM命令が含まれていることを確認
    assert!(ir.contains("fadd"), "Should contain floating point addition");
    assert!(ir.contains("fmul"), "Should contain floating point multiplication");
    assert!(ir.contains("fcmp"), "Should contain floating point comparison");
}

#[test]
fn test_type_casting_codegen() {
    // 型キャストのコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let int_val: i32 = 42;
        let float_val: f64 = int_val as f64;
        let back_to_int: i32 = float_val as i32;
    }
    "#;
    
    let ir = assert_compile_success(source, "casting");
    assert_valid_ir(&ir);
    
    // 型変換のLLVM命令が含まれていることを確認
    assert!(ir.contains("sitofp") || ir.contains("fptosi"), "Should contain type conversion instructions");
}

#[test]
fn test_integer_literal_type_inference() {
    // 整数リテラルの型推論テスト
    let source = r#"
    package main
    
    fn main() {
        let a = 42;         // i32として推論
        let b = 42i64;      // i64として明示
        let c = 255u8;      // u8として明示
        let d = -128i8;     // i8として明示
        
        // 演算での型の整合性確認
        let sum = a + a;    // i32 + i32
        let sum2 = b + b;   // i64 + i64
    }
    "#;
    
    let ir = assert_compile_success(source, "int_literals");
    assert_valid_ir(&ir);
    
    // 異なるビット幅の整数型が生成されていることを確認
    assert!(ir.contains("i32"), "Should contain i32 type");
    assert!(ir.contains("i64"), "Should contain i64 type");
    assert!(ir.contains("i8"), "Should contain i8 type");
}

#[test]
fn test_signed_unsigned_integers() {
    // 符号付き/符号なし整数のテスト
    let source = r#"
    package main
    
    fn main() {
        let signed: i32 = -42;
        let unsigned: u32 = 42u32;
        
        // 符号付き演算
        let s_div = signed / 2;
        let s_rem = signed % 3;
        
        // 符号なし演算
        let u_div = unsigned / 2u32;
        let u_rem = unsigned % 3u32;
    }
    "#;
    
    let ir = assert_compile_success(source, "signed_unsigned");
    assert_valid_ir(&ir);
    
    // 符号付き/符号なし演算が含まれていることを確認
    assert!(ir.contains("sdiv") || ir.contains("udiv"), "Should contain signed/unsigned division");
    assert!(ir.contains("srem") || ir.contains("urem"), "Should contain signed/unsigned remainder");
}