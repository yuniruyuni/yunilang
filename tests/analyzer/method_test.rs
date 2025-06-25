//! メソッド関連のセマンティック解析テスト

use super::*;

#[test]
fn test_struct_field_access_error() {
    // 構造体フィールドアクセスエラーのテスト
    let source = r#"
    package main
    
    struct Point {
        x: f64,
        y: f64,
    }
    
    fn main() {
        let p = Point { x: 1.0, y: 2.0 };
        let z = p.z;  // 存在しないフィールド
    }
    "#;
    
    assert_analysis_error(source);
}

#[test]
fn test_method_not_found_error() {
    // メソッド未定義エラーのテスト
    let source = r#"
    package main
    
    struct Point {
        x: f64,
        y: f64,
    }
    
    fn main() {
        let p = Point { x: 1.0, y: 2.0 };
        p.unknown_method();  // 存在しないメソッド
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::MethodNotFound { .. })
    });
}

#[test]
fn test_method_signature_registration() {
    // メソッドシグネチャ登録のテスト
    // 注：現在のパーサーの制限により、メソッドは"impl fn"構文を使用し、
    // レシーバーは最初のパラメータとして明示的に指定される
    let source = r#"
    package main
    
    struct Point {
        x: f64,
        y: f64,
    }
    
    // メソッドとして登録される
    impl fn distance(p1: &Point, p2: &Point): f64 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        return sqrt(dx * dx + dy * dy);
    }
    
    fn main() {
        // メソッドが登録されることを確認するテスト
        // 実際の呼び出しはメソッドコール構文が必要だが、
        // 現在のパーサーはそれをサポートしていない
        let p1 = Point { x: 0.0, y: 0.0 };
        let p2 = Point { x: 3.0, y: 4.0 };
    }
    "#;
    
    assert_analysis_success(source);
}

#[test] 
fn test_method_duplicate_error() {
    // メソッド重複エラーのテスト
    let source = r#"
    package main
    
    struct Rectangle {
        width: f64,
        height: f64,
    }
    
    impl fn area(rect: &Rectangle): f64 {
        return rect.width * rect.height;
    }
    
    impl fn area(rect: &Rectangle): f64 {  // 重複メソッド
        return rect.width * rect.height;
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::DuplicateFunction { .. })
    });
}

#[test]
fn test_method_on_undefined_type_error() {
    // 未定義型へのメソッド定義エラーのテスト
    let source = r#"
    package main
    
    impl fn method(obj: &UndefinedType) {
        println("This should fail");
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UndefinedType { .. })
    });
}

#[test]
fn test_method_parameter_type_checking() {
    // メソッドパラメータの型チェックのテスト
    let source = r#"
    package main
    
    struct Calculator {
        value: i32,
    }
    
    impl fn add(calc: &Calculator, other: i32): i32 {
        return calc.value + other;
    }
    
    impl fn multiply(calc: &Calculator, factor: f64): f64 {
        return calc.value * factor;  // i32 * f64は型エラー
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::TypeMismatch { .. })
    });
}

#[test]
fn test_static_method_registration() {
    // 静的メソッド（関連関数）登録のテスト
    let source = r#"
    package main
    
    struct Vector {
        x: f64,
        y: f64,
        z: f64,
    }
    
    fn zero(): Vector {
        return Vector { x: 0.0, y: 0.0, z: 0.0 };
    }
    
    fn unit_x(): Vector {
        return Vector { x: 1.0, y: 0.0, z: 0.0 };
    }
    
    impl fn dot(v1: &Vector, v2: &Vector): f64 {
        return v1.x * v2.x + v1.y * v2.y + v1.z * v2.z;
    }
    
    fn main() {
        let v1 = zero();
        let v2 = unit_x();
        // dotはメソッドとして登録されるため、通常の関数として呼び出せない
        // 現在のパーサーはメソッドコール構文をサポートしていない
    }
    "#;
    
    assert_analysis_success(source);
}