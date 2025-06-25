//! スコープのセマンティック解析テスト

use super::*;

#[test]
fn test_variable_scoping() {
    // 変数スコープのテスト
    let source = r#"
    package main
    
    fn main() {
        let x = 10;
        {
            let y = 20;
            let z = x + y;  // xは外側のスコープから見える
        }
        // yとzはここからは見えない
        let w = x + 5;  // xは引き続き見える
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_function_scoping() {
    // 関数スコープのテスト
    let source = r#"
    package main
    
    fn helper(x: i32): i32 {
        return x * 2;
    }
    
    fn main() {
        let result = helper(21);  // helper関数が見える
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_mutability_checking() {
    // 可変性チェックのテスト
    let source = r#"
    package main
    
    fn main() {
        let mut x = 10;
        x = 20;  // 可変なので代入可能
        
        let y = 30;
        // y = 40;  // 不変なので代入不可（コメントアウト）
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_nested_scope_resolution() {
    // ネストしたスコープ解決のテスト
    let source = r#"
    package main
    
    fn main() {
        let a = 1;
        {
            let b = 2;
            {
                let c = 3;
                let sum = a + b + c;  // すべての変数が見える
            }
            let d = a + b;  // aとbは見えるがcは見えない
        }
        let e = a;  // aのみ見える
    }
    "#;
    
    assert_analysis_success(source);
}