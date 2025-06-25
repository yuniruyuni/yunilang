//! 制御フローのセマンティック解析テスト

use super::*;

#[test]
fn test_control_flow_type_checking() {
    // 制御フローの型チェックのテスト
    let source = r#"
    package main
    
    fn abs(x: i32): i32 {
        if x < 0 {
            return -x;
        } else {
            return x;
        }
    }
    
    fn factorial(n: i32): i32 {
        if n <= 1 {
            return 1;
        } else {
            return n * factorial(n - 1);
        }
    }
    
    fn main() {
        let result1 = abs(-5);
        let result2 = factorial(5);
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_recursive_function_analysis() {
    // 再帰関数の解析テスト
    let source = r#"
    package main
    
    fn fibonacci(n: i32): i32 {
        if n <= 1 {
            return n;
        } else {
            return fibonacci(n - 1) + fibonacci(n - 2);
        }
    }
    
    fn factorial(n: i32): i32 {
        if n <= 1 {
            return 1;
        } else {
            return n * factorial(n - 1);
        }
    }
    
    fn main() {
        let fib_result = fibonacci(10);
        let fact_result = factorial(5);
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_unreachable_code_after_return() {
    // return文の後の到達不能コード検出
    let source = r#"
    package main
    
    fn test(): i32 {
        return 42;
        let x = 10;  // 到達不能コード
        return x;    // 到達不能コード
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UnreachableCode { .. })
    });
}

#[test]
fn test_unreachable_code_after_if_else_return() {
    // if-elseの両方でreturnする場合の到達不能コード検出
    let source = r#"
    package main
    
    fn test(x: bool): i32 {
        if x {
            return 1;
        } else {
            return 2;
        }
        let y = 3;  // 到達不能コード
        return y;   // 到達不能コード
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UnreachableCode { .. })
    });
}

#[test]
fn test_reachable_code_if_without_else() {
    // else節がない場合は到達可能
    let source = r#"
    package main
    
    fn test(x: bool): i32 {
        if x {
            return 1;
        }
        let y = 2;  // 到達可能
        return y;   // 到達可能
    }
    
    fn main() {
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_unreachable_code_nested_if() {
    // ネストされたif文での到達不能コード検出
    let source = r#"
    package main
    
    fn test(x: bool, y: bool): i32 {
        if x {
            if y {
                return 1;
            } else {
                return 2;
            }
        } else {
            return 3;
        }
        let z = 4;  // 到達不能コード
        return z;   // 到達不能コード
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UnreachableCode { .. })
    });
}

#[test]
fn test_unreachable_code_in_block() {
    // ブロック内の到達不能コード検出
    let source = r#"
    package main
    
    fn test(): i32 {
        {
            return 42;
            let x = 10;  // 到達不能コード
        }
    }
    
    fn main() {
    }
    "#;
    
    assert_specific_error(source, |e| {
        matches!(e, AnalyzerError::UnreachableCode { .. })
    });
}