//! データ構造のコード生成テスト

use super::*;

#[test]
fn test_struct_operations_codegen() {
    // 構造体操作のコード生成テスト
    let source = r#"
    package main
    
    struct Point {
        x: f64,
        y: f64,
    }
    
    fn distance_squared(p1: Point, p2: Point): f64 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        return dx * dx + dy * dy;
    }
    
    fn main() {
        let origin = Point { x: 0.0, y: 0.0 };
        let point = Point { x: 3.0, y: 4.0 };
        let dist_sq = distance_squared(origin, point);
    }
    "#;
    
    let ir = assert_compile_success(source, "structs");
    assert_valid_ir(&ir);
    
    // デバッグ用にIRを出力
    println!("Generated IR:\n{}", ir);
    
    // 構造体型の定義とフィールドアクセスが含まれていることを確認
    // LLVMのバージョンによっては型定義が異なる形式で出力される可能性がある
    assert!(ir.contains("%Point") || ir.contains("struct") || ir.contains("{ double, double }"), 
            "Should contain struct type definitions or struct literals");
    assert!(ir.contains("extractvalue") || ir.contains("getelementptr"), 
            "Should contain struct field access");
}

#[test]
fn test_tuple_expressions() {
    let source = r#"
    package test

    fn main() {
        // 空のタプル
        let empty = ();
        
        // 単一要素のタプル
        let single = (42,);
        
        // 複数要素のタプル
        let pair = (1, 2.5);
        let triple = ("hello", 42, true);
        
        // ネストしたタプル
        let nested = ((1, 2), (3, 4));
    }
    "#;
    
    let ir = compile_to_ir(source, "test_tuples").unwrap();
    assert_valid_ir(&ir);
    
    // タプルが構造体として実装されていることを確認
    assert!(ir.contains("alloca"), "IR should contain alloca for tuples");
    assert!(ir.contains("getelementptr"), "IR should contain GEP for tuple field access");
    assert!(ir.contains("store"), "IR should contain store instructions for tuple elements");
}

#[test]
fn test_array_expressions() {
    let source = r#"
    package test

    fn main() {
        // 固定サイズ配列
        let arr1 = [1, 2, 3, 4, 5];
        // let arr2 = [0; 10];  // TODO: 配列の繰り返し初期化構文は未実装
        
        // 異なる型の配列
        let float_arr = [1.0, 2.0, 3.0];
        let bool_arr = [true, false, true];
        
        // 配列の要素アクセス
        let first = arr1[0];
        let last = arr1[4];
    }
    "#;
    
    let ir = compile_to_ir(source, "test_arrays").unwrap();
    assert_valid_ir(&ir);
    
    // 配列の初期化と要素アクセスが含まれていることを確認
    assert!(ir.contains("alloca") || ir.contains("malloc"), "IR should contain memory allocation for arrays");
    assert!(ir.contains("store"), "IR should contain store instructions for array initialization");
    assert!(ir.contains("getelementptr"), "IR should contain GEP for array indexing");
}

#[test]
fn test_index_access() {
    let source = r#"
        package test_index

        fn test_array_index(): i32 {
            let arr = [10, 20, 30, 40, 50];
            return arr[2];  // Should return 30
        }
        
        fn test_array_index_ref(): i32 {
            let arr = [100, 200, 300];
            let ref_elem = &arr[1];
            return *ref_elem;  // Should return 200
        }
        
        fn test_array_index_expr(): i32 {
            let arr = [5, 10, 15, 20];
            let i = 3;
            return arr[i];  // Should return 20
        }
    "#;

    let result = compile_to_ir(source, "test_index");
    assert!(result.is_ok(), "Compilation should succeed: {:?}", result.unwrap_err());
    
    let ir = result.unwrap();
    
    // インデックスアクセスの確認
    assert!(ir.contains("getelementptr"), "Should use GEP for array indexing");
    assert!(ir.contains("load i32"), "Should load value from array");
    
    // 配列の初期化を確認
    assert!(ir.contains("malloc"), "Should allocate array on heap");
    assert!(ir.contains("store i32 30"), "Should store array element 30");
}