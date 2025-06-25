//! 基本的なプログラム構造のパーサーテスト

use super::*;

#[test]
fn test_minimal_program() {
    // 最小限のプログラムの解析テスト
    let source = r#"
    package main
    
    fn main() {
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    assert_eq!(ast.package.name, "main");
    assert_eq!(ast.imports.len(), 0);
    assert_eq!(ast.items.len(), 1);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.name, "main");
        assert_eq!(func.params.len(), 0);
        assert!(func.return_type.is_none());
        assert_eq!(func.body.statements.len(), 0);
    } else {
        panic!("Expected function item");
    }
}

#[test]
fn test_hello_world() {
    // Hello Worldプログラムの解析テスト
    let source = r#"
    package main
    
    fn main() {
        println("Hello, World!");
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    if let Item::Function(ref func) = ast.items[0] {
        assert_eq!(func.body.statements.len(), 1);
        
        if let Statement::Expression(Expression::Call(ref call)) = &func.body.statements[0] {
            // call.calleeがprintlnを指すことを確認
            assert_eq!(call.args.len(), 1);
        }
    }
}

#[test]
fn test_imports() {
    // import文の解析テスト
    let source = r#"
    package main
    
    import "std/io"
    import "std/math" as math
    import "my/module"
    
    fn main() {
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    assert_eq!(ast.imports.len(), 3);
    
    assert_eq!(ast.imports[0].path, "std/io");
    assert!(ast.imports[0].alias.is_none());
    
    assert_eq!(ast.imports[1].path, "std/math");
    assert_eq!(ast.imports[1].alias.as_ref().unwrap(), "math");
    
    assert_eq!(ast.imports[2].path, "my/module");
    assert!(ast.imports[2].alias.is_none());
}

#[test]
fn test_complex_program() {
    // 複雑なプログラム全体のテスト
    let source = r#"
    package calculator
    
    import "std/io"
    
    type Calculator struct {
        result: f64,
    }
    
    type Operation enum {
        Add,
        Subtract,
        Multiply,
        Divide,
    }
    
    fn create_calculator(): Calculator {
        return Calculator { result: 0.0 };
    }
    
    fn calculate(calc: Calculator, op: Operation, value: f64): Calculator {
        let new_result = if op == Operation::Add {
            calc.result + value
        } else if op == Operation::Subtract {
            calc.result - value
        } else if op == Operation::Multiply {
            calc.result * value
        } else {
            calc.result / value
        };
        
        return Calculator { result: new_result };
    }
    
    fn main() {
        let mut calc = create_calculator();
        calc = calculate(calc, Operation::Add, 10.0);
        calc = calculate(calc, Operation::Multiply, 2.0);
        println("Result:", calc.result);
    }
    "#;
    
    let ast = assert_parse_success(source);
    
    assert_eq!(ast.package.name, "calculator");
    assert_eq!(ast.imports.len(), 1);
    assert_eq!(ast.items.len(), 5); // struct, enum, 3 functions
    
    // 構造体とenumが含まれていることを確認
    let has_struct = ast.items.iter().any(|item| {
        matches!(item, Item::TypeDef(TypeDef::Struct(_)))
    });
    assert!(has_struct);
    
    let has_enum = ast.items.iter().any(|item| {
        matches!(item, Item::TypeDef(TypeDef::Enum(_)))
    });
    assert!(has_enum);
}