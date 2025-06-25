//! 型チェックのセマンティック解析テスト

use super::*;

#[test]
fn test_basic_type_checking() {
    // 基本的な型チェックのテスト
    let source = r#"
    package main
    
    fn main() {
        let x: i32 = 42;
        let y: f64 = 3.14;
        let s: String = "hello";
        let b: bool = true;
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_type_inference() {
    // 型推論のテスト
    let source = r#"
    package main
    
    fn main() {
        let x = 42;       // i32と推論される
        let y = 3.14;     // f64と推論される
        let s = "hello";  // Stringと推論される
        let b = true;     // boolと推論される
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_function_type_checking() {
    // 関数の型チェックのテスト
    let source = r#"
    package main
    
    fn add(a: i32, b: i32): i32 {
        return a + b;
    }
    
    fn greet(name: str): str {
        return "Hello, " + name;
    }
    
    fn main() {
        let result = add(5, 3);
        let message = greet("World");
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_arithmetic_type_checking() {
    // 算術演算の型チェックのテスト
    let source = r#"
    package main
    
    fn main() {
        let a: i32 = 10;
        let b: i32 = 20;
        let sum = a + b;      // i32 + i32 = i32
        let diff = a - b;     // i32 - i32 = i32
        let prod = a * b;     // i32 * i32 = i32
        let quot = a / b;     // i32 / i32 = i32
        let rem = a % b;      // i32 % i32 = i32
        
        let x: f64 = 3.14;
        let y: f64 = 2.71;
        let float_sum = x + y;  // f64 + f64 = f64
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_comparison_type_checking() {
    // 比較演算の型チェックのテスト
    let source = r#"
    package main
    
    fn main() {
        let a: i32 = 10;
        let b: i32 = 20;
        let gt = a > b;       // i32 > i32 = bool
        let lt = a < b;       // i32 < i32 = bool
        let gte = a >= b;     // i32 >= i32 = bool
        let lte = a <= b;     // i32 <= i32 = bool
        let eq = a == b;      // i32 == i32 = bool
        let ne = a != b;      // i32 != i32 = bool
        
        let x: f64 = 3.14;
        let y: f64 = 2.71;
        let float_gt = x > y;  // f64 > f64 = bool
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_logical_type_checking() {
    // 論理演算の型チェックのテスト
    let source = r#"
    package main
    
    fn main() {
        let a: bool = true;
        let b: bool = false;
        let and_result = a && b;  // bool && bool = bool
        let or_result = a || b;   // bool || bool = bool
        let not_result = !a;      // !bool = bool
        
        let x = 10;
        let y = 20;
        let complex = (x > 5) && (y < 30);  // bool && bool = bool
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_complex_expression_type_checking() {
    // 複雑な式の型チェックのテスト
    let source = r#"
    package main
    
    fn calculate(x: i32, y: i32, z: i32): i32 {
        return (x + y) * z / 2;
    }
    
    fn main() {
        let a = 10;
        let b = 20;
        let c = 30;
        
        let result1 = calculate(a, b, c);
        let result2 = calculate(a + 5, b - 3, c * 2);
        
        let condition = (a > 0) && (b < 100) || (c == 30);
        let value = if condition { a + b } else { c - a };
    }
    "#;
    
    assert_analysis_success(source);
}