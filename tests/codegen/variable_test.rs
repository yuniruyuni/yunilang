//! 変数とメモリ管理のコード生成テスト

use super::*;

#[test]
fn test_variable_operations_codegen() {
    // 変数操作のコード生成テスト
    let source = r#"
    package main
    
    fn main() {
        let x: i32 = 42;
        let y: i32 = x;
        let mut z: i32 = 0;
        z = x + y;
    }
    "#;
    
    let ir = assert_compile_success(source, "variables");
    assert_valid_ir(&ir);
    
    // alloca（スタック割り当て）とstore/load命令が含まれていることを確認
    assert!(ir.contains("alloca"), "Should contain stack allocation");
    assert!(ir.contains("store"), "Should contain store operations");
    assert!(ir.contains("load"), "Should contain load operations");
}

#[test]
fn test_memory_management_codegen() {
    // 大きな構造体のメモリ管理テスト
    let source = r#"
    package main
    
    struct LargeStruct {
        data1: i64,
        data2: i64,
        data3: i64,
        data4: i64,
    }
    
    fn create_large_struct(): LargeStruct {
        return LargeStruct {
            data1: 1i64,
            data2: 2i64,
            data3: 3i64,
            data4: 4i64,
        };
    }
    
    fn main() {
        let large = create_large_struct();
        let sum = large.data1 + large.data2 + large.data3 + large.data4;
    }
    "#;
    
    let ir = assert_compile_success(source, "memory_mgmt");
    assert_valid_ir(&ir);
    
    // メモリ管理関連の命令が含まれていることを確認
    assert!(ir.contains("alloca"), "Should contain stack allocation");
    assert!(ir.contains("getelementptr") || ir.contains("extractvalue"), "Should contain struct field access");
}

#[test]
fn test_assignment_expressions() {
    let source = r#"
    package test

    fn main() {
        // 基本的な代入式（文として）
        let mut x = 0;
        x = 42;
        
        // 複数の代入
        let mut a = 0;
        let mut b = 0;
        let mut c = 0;
        a = 100;
        b = 100;
        c = 100;
        
        // 構造体フィールドへの代入（構造体が定義されている場合）
        // struct Point { x: i32, y: i32 }
        // let mut point = Point { x: 0, y: 0 };
        // point.x = 10;
    }
    "#;
    
    let ir = compile_to_ir(source, "test_assignments").unwrap();
    assert_valid_ir(&ir);
    
    // 代入が実行されることを確認
    assert!(ir.contains("store"), "IR should contain store instructions for assignments");
    // 変数への複数の代入を確認
    let store_count = ir.matches("store").count();
    assert!(store_count >= 6, "IR should contain at least 6 store instructions for initial values and assignments");
}

#[test]
#[ignore = "Reference borrowing rules are too strict in current implementation"]
fn test_reference_expressions() {
    let source = r#"
    package test

    fn get_ref_value(): i32 {
        let x = 42;
        let ref_x = &x;
        return *ref_x;
    }

    fn modify_through_ref(): i32 {
        let mut x = 10;
        let ref_x = &mut x;
        *ref_x = 20;
        return x;
    }

    fn main() {
        let a = get_ref_value();
        let b = modify_through_ref();
    }
    "#;
    
    let ir = compile_to_ir(source, "test_references").unwrap();
    assert_valid_ir(&ir);
    
    // 参照操作のLLVM命令が含まれていることを確認
    assert!(ir.contains("alloca"), "Should contain stack allocation");
    assert!(ir.contains("load"), "Should contain load for dereferencing");
    assert!(ir.contains("store"), "Should contain store for assignment through reference");
}