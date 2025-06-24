//! ASTテスト
//! 
//! Yuniコンパイラの抽象構文木（AST）の包括的なテストスイート。
//! AST構造の正当性、シリアライゼーション/デシリアライゼーション、
//! スパン情報の正確性をテストする。

#[cfg(test)]
mod tests {
    use yunilang::ast::*;
    use yunilang::lexer::Lexer;
    use yunilang::parser::Parser;

    /// ソースコードからASTを構築するヘルパー関数
    fn build_ast(source: &str) -> Program {
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.collect_tokens();
        let mut parser = Parser::new(tokens);
        parser.parse().expect("Parsing should succeed")
    }

    #[test]
    fn test_span_creation() {
        // Span構造体の基本テスト
        let span = Span::new(10, 20);
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
        
        let dummy_span = Span::dummy();
        assert_eq!(dummy_span.start, 0);
        assert_eq!(dummy_span.end, 0);
    }

    #[test]
    fn test_program_structure() {
        // プログラム全体のAST構造テスト
        let source = r#"
        package test
        
        import "std/io"
        
        fn main() {
            println("Hello");
        }
        "#;
        
        let ast = build_ast(source);
        
        // パッケージ名の確認
        assert_eq!(ast.package.name, "test");
        
        // インポートの確認
        assert_eq!(ast.imports.len(), 1);
        assert_eq!(ast.imports[0].path, "std/io");
        assert!(ast.imports[0].alias.is_none());
        
        // アイテムの確認
        assert_eq!(ast.items.len(), 1);
        assert!(matches!(ast.items[0], Item::Function(_)));
    }

    #[test]
    fn test_function_declaration_ast() {
        // 関数宣言のAST構造テスト
        let source = r#"
        package test
        
        fn calculate(x: i32, y: f64): f64 {
            let result = x as f64 + y;
            return result;
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            // 関数名
            assert_eq!(func.name, "calculate");
            
            // パラメータ
            assert_eq!(func.params.len(), 2);
            assert_eq!(func.params[0].name, "x");
            assert!(matches!(func.params[0].ty, Type::I32));
            assert_eq!(func.params[1].name, "y");
            assert!(matches!(func.params[1].ty, Type::F64));
            
            // 戻り値の型
            assert!(func.return_type.is_some());
            if let Some(ref ret_type) = func.return_type {
                assert!(matches!(**ret_type, Type::F64));
            }
            
            // 関数本体
            assert_eq!(func.body.statements.len(), 2);
        } else {
            panic!("Expected function item");
        }
    }

    #[test]
    fn test_variable_declaration_ast() {
        // 変数宣言のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            let x: i32 = 42;
            let mut y = 3.14;
            let z;
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.body.statements.len(), 3);
            
            // let x: i32 = 42;
            if let Statement::Let(ref let_stmt) = func.body.statements[0] {
                if let Pattern::Identifier(ref name, is_mut) = let_stmt.pattern {
                    assert_eq!(name, "x");
                    assert!(!is_mut);
                }
                assert!(let_stmt.ty.is_some());
                assert!(let_stmt.init.is_some());
                
                if let Some(ref init) = let_stmt.init {
                    assert!(matches!(init, Expression::Integer(_)));
                }
            } else {
                panic!("Expected let statement");
            }
            
            // let mut y = 3.14;
            if let Statement::Let(ref let_stmt) = func.body.statements[1] {
                if let Pattern::Identifier(ref name, is_mut) = let_stmt.pattern {
                    assert_eq!(name, "y");
                    assert!(is_mut);
                }
                assert!(let_stmt.ty.is_none()); // 型推論
                assert!(let_stmt.init.is_some());
            } else {
                panic!("Expected mutable let statement");
            }
            
            // let z;
            if let Statement::Let(ref let_stmt) = func.body.statements[2] {
                if let Pattern::Identifier(ref name, is_mut) = let_stmt.pattern {
                    assert_eq!(name, "z");
                    assert!(!is_mut);
                }
                assert!(let_stmt.ty.is_none());
                assert!(let_stmt.init.is_none());
            } else {
                panic!("Expected uninitialized let statement");
            }
        }
    }

    #[test]
    fn test_expression_ast() {
        // 式のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            let a = 1 + 2 * 3;
            let b = func_call(arg1, arg2);
            let c = array[index];
            let d = obj.field;
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.body.statements.len(), 4);
            
            // すべて変数宣言で、初期化子が式である
            for stmt in &func.body.statements {
                if let Statement::Let(ref let_stmt) = stmt {
                    assert!(let_stmt.init.is_some());
                } else {
                    panic!("Expected let statement");
                }
            }
        }
    }

    #[test]
    fn test_literal_expressions() {
        // リテラル式のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            let int_val = 42;
            let float_val = 3.14;
            let string_val = "hello";
            let bool_val = true;
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.body.statements.len(), 4);
            
            // 各変数宣言の初期化子がリテラルであることを確認
            for (i, stmt) in func.body.statements.iter().enumerate() {
                if let Statement::Let(ref let_stmt) = stmt {
                    if let Some(ref init) = let_stmt.init {
                        match i {
                            0 => assert!(matches!(init, Expression::Integer(_))),
                            1 => assert!(matches!(init, Expression::Float(_))),
                            2 => assert!(matches!(init, Expression::String(_))),
                            3 => assert!(matches!(init, Expression::Boolean(_))),
                            _ => panic!("Unexpected statement index"),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_binary_expression_ast() {
        // 二項演算式のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            let add = a + b;
            let sub = c - d;
            let mul = e * f;
            let div = g / h;
            let eq = i == j;
            let ne = k != l;
            let and = m && n;
            let or = o || p;
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            // 各変数宣言の初期化子が二項演算式であることを確認
            for stmt in &func.body.statements {
                if let Statement::Let(ref let_stmt) = stmt {
                    if let Some(ref init) = let_stmt.init {
                        assert!(matches!(init, Expression::Binary(_)));
                    }
                }
            }
        }
    }

    #[test]
    fn test_if_statement_ast() {
        // if文のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            if condition {
                do_something();
            } else if other_condition {
                do_other();
            } else {
                do_default();
            }
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.body.statements.len(), 1);
            
            if let Statement::If(ref if_stmt) = func.body.statements[0] {
                // 条件式があること
                assert!(matches!(if_stmt.condition, Expression::Identifier(_)));
                
                // then節があること
                assert!(!if_stmt.then_branch.statements.is_empty());
                
                // else節があること
                assert!(if_stmt.else_branch.is_some());
            } else {
                panic!("Expected if statement");
            }
        }
    }

    #[test]
    fn test_while_statement_ast() {
        // while文のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            while x > 0 {
                x = x - 1;
                println(x);
            }
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.body.statements.len(), 1);
            
            if let Statement::While(ref while_stmt) = func.body.statements[0] {
                // 条件式があること
                assert!(matches!(while_stmt.condition, Expression::Binary(_)));
                
                // ループ本体があること
                assert_eq!(while_stmt.body.statements.len(), 2);
            } else {
                panic!("Expected while statement");
            }
        }
    }

    #[test]
    fn test_struct_definition_ast() {
        // 構造体定義のAST構造テスト
        let source = r#"
        package test
        
        type Point struct {
            x: f64,
            y: f64,
        }
        
        type Empty struct {
        }
        
        fn main() {
        }
        "#;
        
        let ast = build_ast(source);
        
        assert_eq!(ast.items.len(), 3); // 2つの構造体 + 1つの関数
        
        // 最初の構造体（Point）
        if let Item::TypeDef(TypeDef::Struct(ref struct_def)) = ast.items[0] {
            assert_eq!(struct_def.name, "Point");
            assert_eq!(struct_def.fields.len(), 2);
            
            assert_eq!(struct_def.fields[0].name, "x");
            assert!(matches!(struct_def.fields[0].ty, Type::F64));
            
            assert_eq!(struct_def.fields[1].name, "y");
            assert!(matches!(struct_def.fields[1].ty, Type::F64));
        } else {
            panic!("Expected struct definition");
        }
        
        // 2番目の構造体（Empty）
        if let Item::TypeDef(TypeDef::Struct(ref struct_def)) = ast.items[1] {
            assert_eq!(struct_def.name, "Empty");
            assert_eq!(struct_def.fields.len(), 0);
        } else {
            panic!("Expected empty struct definition");
        }
    }

    #[test]
    fn test_enum_definition_ast() {
        // 列挙型定義のAST構造テスト
        let source = r#"
        package test
        
        type Color enum {
            Red,
            Green,
            Blue,
        }
        
        type Option enum {
            Some { value: i32 },
            None,
        }
        
        fn main() {
        }
        "#;
        
        let ast = build_ast(source);
        
        assert_eq!(ast.items.len(), 3); // 2つの列挙型 + 1つの関数
        
        // 最初の列挙型（Color）
        if let Item::TypeDef(TypeDef::Enum(ref enum_def)) = ast.items[0] {
            assert_eq!(enum_def.name, "Color");
            assert_eq!(enum_def.variants.len(), 3);
            
            assert_eq!(enum_def.variants[0].name, "Red");
            assert_eq!(enum_def.variants[0].fields.len(), 0);
            
            assert_eq!(enum_def.variants[1].name, "Green");
            assert_eq!(enum_def.variants[1].fields.len(), 0);
            
            assert_eq!(enum_def.variants[2].name, "Blue");
            assert_eq!(enum_def.variants[2].fields.len(), 0);
        } else {
            panic!("Expected enum definition");
        }
        
        // 2番目の列挙型（Option）
        if let Item::TypeDef(TypeDef::Enum(ref enum_def)) = ast.items[1] {
            assert_eq!(enum_def.name, "Option");
            assert_eq!(enum_def.variants.len(), 2);
            
            // Some variant with field
            assert_eq!(enum_def.variants[0].name, "Some");
            assert_eq!(enum_def.variants[0].fields.len(), 1);
            assert_eq!(enum_def.variants[0].fields[0].name, "value");
            
            // None variant without fields
            assert_eq!(enum_def.variants[1].name, "None");
            assert_eq!(enum_def.variants[1].fields.len(), 0);
        } else {
            panic!("Expected enum definition with variants");
        }
    }

    #[test]
    fn test_function_call_ast() {
        // 関数呼び出しのAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            println();
            println("hello");
            println("x =", x, "y =", y);
            let result = add(1, 2);
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            // 3つのprintln + 1つの変数宣言
            assert_eq!(func.body.statements.len(), 4);
            
            // 最初の3つは式文（関数呼び出し）
            for i in 0..3 {
                if let Statement::Expression(ref expr) = func.body.statements[i] {
                    if let Expression::Call(ref call) = expr {
                        // call.calleeがprintln関数を指すことを確認
                        match i {
                            0 => assert_eq!(call.args.len(), 0),  // println()
                            1 => assert_eq!(call.args.len(), 1),  // println("hello")
                            2 => assert_eq!(call.args.len(), 4),  // println("x =", x, "y =", y)
                            _ => {}
                        }
                    } else {
                        panic!("Expected function call expression");
                    }
                } else {
                    panic!("Expected expression statement");
                }
            }
            
            // 最後は変数宣言
            if let Statement::Let(ref let_stmt) = func.body.statements[3] {
                if let Some(ref init) = let_stmt.init {
                    if let Expression::Call(ref call) = init {
                        // call.calleeが"add"関数を指すことを確認
                        assert_eq!(call.args.len(), 2);
                    } else {
                        panic!("Expected function call in initializer");
                    }
                }
            }
        }
    }

    #[test]
    fn test_complex_nested_expressions() {
        // 複雑にネストした式のAST構造テスト
        let source = r#"
        package test
        
        fn main() {
            let result = func1(
                func2(a + b, c * d),
                func3(
                    if x > 0 { y } else { z },
                    array[index].field
                )
            );
        }
        "#;
        
        let ast = build_ast(source);
        
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.body.statements.len(), 1);
            
            if let Statement::Let(ref let_stmt) = func.body.statements[0] {
                if let Some(ref init) = let_stmt.init {
                    // 最外層は関数呼び出し
                    assert!(matches!(init, Expression::Call(_)));
                    
                    if let Expression::Call(ref call) = init {
                        // func1の呼び出し
                        assert_eq!(call.args.len(), 2);
                        
                        // 引数も関数呼び出し
                        assert!(matches!(call.args[0], Expression::Call(_)));
                        assert!(matches!(call.args[1], Expression::Call(_)));
                    }
                }
            }
        }
    }

    #[test]
    fn test_ast_serialization() {
        // ASTのシリアライゼーション/デシリアライゼーションテスト
        let source = r#"
        package test
        
        fn add(a: i32, b: i32): i32 {
            return a + b;
        }
        "#;
        
        let original_ast = build_ast(source);
        
        // JSONにシリアライズ
        let serialized = serde_json::to_string(&original_ast)
            .expect("Serialization should succeed");
        
        // JSONからデシリアライズ
        let deserialized_ast: Program = serde_json::from_str(&serialized)
            .expect("Deserialization should succeed");
        
        // 元のASTと等しいことを確認
        assert_eq!(original_ast.package.name, deserialized_ast.package.name);
        assert_eq!(original_ast.imports.len(), deserialized_ast.imports.len());
        assert_eq!(original_ast.items.len(), deserialized_ast.items.len());
        
        // 完全な等価性チェック
        assert_eq!(original_ast, deserialized_ast);
    }

    #[test]
    fn test_ast_pretty_print() {
        // ASTの可読形式出力テスト
        let source = r#"
        package test
        
        type Point struct {
            x: f64,
            y: f64,
        }
        
        fn distance(p1: Point, p2: Point): f64 {
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            return sqrt(dx * dx + dy * dy);
        }
        "#;
        
        let ast = build_ast(source);
        
        // 整形されたJSONとして出力
        let pretty_json = serde_json::to_string_pretty(&ast)
            .expect("Pretty printing should succeed");
        
        // 基本的な構造が含まれていることを確認
        assert!(pretty_json.contains("\"package\""));
        assert!(pretty_json.contains("\"test\""));
        assert!(pretty_json.contains("\"Point\""));
        assert!(pretty_json.contains("\"distance\""));
        assert!(pretty_json.len() > 100); // 十分な長さがある
    }

    #[test]
    fn test_span_information() {
        // スパン情報の正確性テスト
        let source = "package main\n\nfn main() {\n    let x = 42;\n}";
        
        let ast = build_ast(source);
        
        // プログラム全体のスパン
        assert!(ast.span.start < ast.span.end);
        
        // パッケージのスパン
        assert!(ast.package.span.start < ast.package.span.end);
        
        // 関数のスパン
        if let Item::Function(ref func) = ast.items[0] {
            assert!(func.span.start < func.span.end);
            assert!(func.body.span.start < func.body.span.end);
            
            // 変数宣言のスパン
            if let Statement::Let(ref let_stmt) = func.body.statements[0] {
                assert!(let_stmt.span.start < let_stmt.span.end);
                
                if let Some(ref init) = let_stmt.init {
                    // Expressionのspanは各バリアント内にある
                    match init {
                        Expression::Integer(lit) => assert!(lit.span.start < lit.span.end),
                        Expression::Float(lit) => assert!(lit.span.start < lit.span.end),
                        Expression::String(lit) => assert!(lit.span.start < lit.span.end),
                        Expression::Boolean(lit) => assert!(lit.span.start < lit.span.end),
                        Expression::Identifier(id) => assert!(id.span.start < id.span.end),
                        Expression::Binary(expr) => assert!(expr.span.start < expr.span.end),
                        Expression::Call(expr) => assert!(expr.span.start < expr.span.end),
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn test_ast_node_count() {
        // ASTノード数の計算テスト（デバッグ用）
        let source = r#"
        package test
        
        fn factorial(n: i32): i32 {
            if n <= 1 {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }
        
        fn main() {
            let result = factorial(5);
            println("5! =", result);
        }
        "#;
        
        let ast = build_ast(source);
        
        // 基本的な構造カウント
        assert_eq!(ast.items.len(), 2); // 2つの関数
        
        // factorial関数
        if let Item::Function(ref func) = ast.items[0] {
            assert_eq!(func.name, "factorial");
            assert_eq!(func.params.len(), 1);
            assert!(func.return_type.is_some());
            assert_eq!(func.body.statements.len(), 1); // if文1つ
        }
        
        // main関数
        if let Item::Function(ref func) = ast.items[1] {
            assert_eq!(func.name, "main");
            assert_eq!(func.params.len(), 0);
            assert!(func.return_type.is_none());
            assert_eq!(func.body.statements.len(), 2); // let文とprintln文
        }
    }

    #[test]
    fn test_empty_constructs() {
        // 空の構造体のテスト
        let source = r#"
        package test
        
        struct Empty {}
        
        enum Unit {
            Value,
        }
        
        fn do_nothing() {}
        
        fn main() {
            let _empty = Empty {};
        }
        "#;
        
        let ast = build_ast(source);
        
        assert_eq!(ast.items.len(), 4); // 構造体、列挙型、2つの関数
        
        // 空の構造体
        if let Item::TypeDef(TypeDef::Struct(ref struct_def)) = ast.items[0] {
            assert_eq!(struct_def.name, "Empty");
            assert_eq!(struct_def.fields.len(), 0);
        }
        
        // 単一バリアント列挙型
        if let Item::TypeDef(TypeDef::Enum(ref enum_def)) = ast.items[1] {
            assert_eq!(enum_def.name, "Unit");
            assert_eq!(enum_def.variants.len(), 1);
            assert_eq!(enum_def.variants[0].name, "Value");
            assert_eq!(enum_def.variants[0].fields.len(), 0);
        }
        
        // 空の関数
        if let Item::Function(ref func) = ast.items[2] {
            assert_eq!(func.name, "do_nothing");
            assert_eq!(func.params.len(), 0);
            assert!(func.return_type.is_none());
            assert_eq!(func.body.statements.len(), 0);
        }
    }
}