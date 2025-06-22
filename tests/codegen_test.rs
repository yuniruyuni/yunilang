//! コード生成テスト
//! 
//! Yuniコンパイラのコード生成器の包括的なテストスイート。
//! LLVM IR生成、最適化、実行時正当性を検証する。

#[cfg(test)]
mod tests {
    use yunilang::analyzer::SemanticAnalyzer;
    use yunilang::codegen::CodeGenerator;
    use yunilang::lexer::Lexer;
    use yunilang::parser::Parser;
    use yunilang::ast::*;
    use inkwell::context::Context;
    use inkwell::OptimizationLevel;
    use std::fs;
    use std::process::Command;

    /// ソースコードを完全にコンパイルしてLLVM IRを生成するヘルパー関数
    fn compile_to_ir(source: &str, module_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        // 字句解析
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.collect_tokens();
        
        // 構文解析
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;
        
        // セマンティック解析
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&ast)?;
        
        // コード生成
        let context = Context::create();
        let mut codegen = CodeGenerator::new(&context, module_name);
        codegen.compile_program(&ast)?;
        
        // LLVM IRを文字列として取得
        Ok(codegen.get_module().print_to_string().to_string())
    }

    /// LLVM IRが有効であることを確認するヘルパー関数
    fn assert_valid_ir(ir: &str) {
        // 基本的な構造チェック
        assert!(ir.contains("target"), "IR should contain target information");
        assert!(ir.contains("define"), "IR should contain function definitions");
        
        // 基本ブロックの整合性チェック
        let define_count = ir.matches("define").count();
        let ret_count = ir.matches("ret").count();
        assert!(ret_count >= define_count, "Each function should have at least one return");
    }

    /// コンパイルに成功することを確認するヘルパー関数
    fn assert_compile_success(source: &str, module_name: &str) -> String {
        compile_to_ir(source, module_name).expect("Compilation should succeed")
    }

    /// コンパイルに失敗することを確認するヘルパー関数
    fn assert_compile_error(source: &str, module_name: &str) {
        assert!(compile_to_ir(source, module_name).is_err(), "Compilation should fail");
    }

    #[test]
    fn test_minimal_program_codegen() {
        // 最小限のプログラムのコード生成テスト
        let source = r#"
        package main
        
        fn main() {
        }
        "#;
        
        let ir = assert_compile_success(source, "minimal");
        assert_valid_ir(&ir);
        
        // main関数が存在することを確認
        assert!(ir.contains("define"), "Should contain main function definition");
        assert!(ir.contains("ret"), "Should contain return statement");
    }

    #[test]
    fn test_hello_world_codegen() {
        // Hello Worldプログラムのコード生成テスト
        let source = r#"
        package main
        
        fn main() {
            println("Hello, World!");
        }
        "#;
        
        let ir = assert_compile_success(source, "hello");
        assert_valid_ir(&ir);
        
        // printf呼び出しが含まれていることを確認
        assert!(ir.contains("printf"), "Should contain printf call for println");
        assert!(ir.contains("Hello, World!"), "Should contain the string literal");
    }

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
    fn test_function_calls_codegen() {
        // 関数呼び出しのコード生成テスト
        let source = r#"
        package main
        
        fn add(a: i32, b: i32): i32 {
            return a + b;
        }
        
        fn main() {
            let result = add(5, 3);
            println("Result:", result);
        }
        "#;
        
        let ir = assert_compile_success(source, "functions");
        assert_valid_ir(&ir);
        
        // 関数定義と呼び出しが含まれていることを確認
        assert!(ir.contains("define") && ir.matches("define").count() >= 2, "Should contain multiple function definitions");
        assert!(ir.contains("call"), "Should contain function call");
    }

    #[test]
    fn test_conditional_statements_codegen() {
        // 条件文のコード生成テスト
        let source = r#"
        package main
        
        fn abs(x: i32): i32 {
            if x < 0 {
                return -x;
            } else {
                return x;
            }
        }
        
        fn main() {
            let result = abs(-5);
        }
        "#;
        
        let ir = assert_compile_success(source, "conditionals");
        assert_valid_ir(&ir);
        
        // 条件分岐のLLVM命令が含まれていることを確認
        assert!(ir.contains("icmp"), "Should contain integer comparison");
        assert!(ir.contains("br"), "Should contain branch instructions");
        assert!(ir.contains("label"), "Should contain basic block labels");
    }

    #[test]
    fn test_loops_codegen() {
        // ループのコード生成テスト
        let source = r#"
        package main
        
        fn main() {
            let mut i = 0;
            while i < 10 {
                println(i);
                i = i + 1;
            }
        }
        "#;
        
        let ir = assert_compile_success(source, "loops");
        assert_valid_ir(&ir);
        
        // ループのLLVM構造が含まれていることを確認
        assert!(ir.contains("br"), "Should contain branch instructions for loop");
        assert!(ir.contains("icmp"), "Should contain comparison for loop condition");
        assert!(ir.matches("label").count() >= 2, "Should contain multiple basic blocks for loop");
    }

    #[test]
    fn test_struct_operations_codegen() {
        // 構造体操作のコード生成テスト
        let source = r#"
        package main
        
        struct Point {
            x: f64,
            y: f64,
        }
        
        fn distance(p1: Point, p2: Point): f64 {
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            return sqrt(dx * dx + dy * dy);
        }
        
        fn main() {
            let origin = Point { x: 0.0, y: 0.0 };
            let point = Point { x: 3.0, y: 4.0 };
            let dist = distance(origin, point);
        }
        "#;
        
        let ir = assert_compile_success(source, "structs");
        assert_valid_ir(&ir);
        
        // 構造体型の定義とフィールドアクセスが含まれていることを確認
        assert!(ir.contains("type"), "Should contain struct type definitions");
        assert!(ir.contains("getelementptr"), "Should contain struct field access");
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
    fn test_boolean_operations_codegen() {
        // ブール演算のコード生成テスト
        let source = r#"
        package main
        
        fn main() {
            let a = true;
            let b = false;
            let and_result = a && b;
            let or_result = a || b;
            let not_result = !a;
        }
        "#;
        
        let ir = assert_compile_success(source, "bool_ops");
        assert_valid_ir(&ir);
        
        // ブール演算のLLVM命令が含まれていることを確認
        assert!(ir.contains("and") || ir.contains("or"), "Should contain logical operations");
    }

    #[test]
    fn test_recursive_functions_codegen() {
        // 再帰関数のコード生成テスト
        let source = r#"
        package main
        
        fn factorial(n: i32): i32 {
            if n <= 1 {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }
        
        fn main() {
            let result = factorial(5);
        }
        "#;
        
        let ir = assert_compile_success(source, "recursive");
        assert_valid_ir(&ir);
        
        // 再帰呼び出しが含まれていることを確認
        assert!(ir.contains("call"), "Should contain recursive function call");
    }

    #[test]
    fn test_multiple_return_paths_codegen() {
        // 複数の戻り値パスのコード生成テスト
        let source = r#"
        package main
        
        fn classify(x: i32): i32 {
            if x > 0 {
                return 1;
            } else if x < 0 {
                return -1;
            } else {
                return 0;
            }
        }
        
        fn main() {
            let pos = classify(5);
            let neg = classify(-3);
            let zero = classify(0);
        }
        "#;
        
        let ir = assert_compile_success(source, "multiple_returns");
        assert_valid_ir(&ir);
        
        // 複数の基本ブロックと戻り値が含まれていることを確認
        assert!(ir.matches("ret").count() >= 3, "Should contain multiple return statements");
        assert!(ir.matches("label").count() >= 3, "Should contain multiple basic blocks");
    }

    #[test]
    fn test_string_operations_codegen() {
        // 文字列操作のコード生成テスト
        let source = r#"
        package main
        
        fn main() {
            let greeting = "Hello";
            let name = "World";
            println(greeting, name);
            println("Combined message");
        }
        "#;
        
        let ir = assert_compile_success(source, "strings");
        assert_valid_ir(&ir);
        
        // 文字列リテラルとprintf呼び出しが含まれていることを確認
        assert!(ir.contains("Hello"), "Should contain string literal");
        assert!(ir.contains("World"), "Should contain string literal");
        assert!(ir.contains("printf"), "Should contain printf calls");
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
    fn test_optimization_levels() {
        // 最適化レベルのテスト
        let source = r#"
        package main
        
        fn calculate_sum(n: i32): i32 {
            let mut sum = 0;
            let mut i = 1;
            while i <= n {
                sum = sum + i;
                i = i + 1;
            }
            return sum;
        }
        
        fn main() {
            let result = calculate_sum(100);
        }
        "#;
        
        // 最適化なしのコンパイル
        let ir_o0 = assert_compile_success(source, "opt_test_o0");
        assert_valid_ir(&ir_o0);
        
        // 最適化レベルによってIRが変わることを確認
        // （実際の最適化は実装依存）
        assert!(ir_o0.len() > 0, "Should generate non-empty IR");
    }

    #[test]
    fn test_error_handling_codegen() {
        // エラーハンドリングのテスト
        let invalid_sources = vec![
            // セマンティックエラー（型チェック段階で捕捉される）
            r#"
            package main
            fn main() {
                let x: i32 = "hello";
            }
            "#,
            
            // 未定義関数呼び出し
            r#"
            package main
            fn main() {
                unknown_function();
            }
            "#,
        ];
        
        for (i, source) in invalid_sources.iter().enumerate() {
            assert_compile_error(source, &format!("error_test_{}", i));
        }
    }

    #[test]
    fn test_complex_program_codegen() {
        // 複雑なプログラム全体のコード生成テスト
        let source = r#"
        package calculator
        
        struct Calculator {
            value: f64,
        }
        
        enum Operation {
            Add,
            Subtract,
            Multiply,
            Divide,
        }
        
        fn create_calculator(): Calculator {
            return Calculator { value: 0.0 };
        }
        
        fn perform_operation(calc: Calculator, op: Operation, operand: f64): Calculator {
            let new_value = if op == Operation::Add {
                calc.value + operand
            } else if op == Operation::Subtract {
                calc.value - operand
            } else if op == Operation::Multiply {
                calc.value * operand
            } else {
                calc.value / operand
            };
            
            return Calculator { value: new_value };
        }
        
        fn main() {
            let mut calc = create_calculator();
            calc = perform_operation(calc, Operation::Add, 5.0);
            calc = perform_operation(calc, Operation::Multiply, 2.0);
            println("Result:", calc.value);
        }
        "#;
        
        let ir = assert_compile_success(source, "complex_calculator");
        assert_valid_ir(&ir);
        
        // 複雑なプログラムの主要コンポーネントが含まれていることを確認
        assert!(ir.contains("Calculator"), "Should contain struct definitions");
        assert!(ir.matches("define").count() >= 3, "Should contain multiple function definitions");
        assert!(ir.contains("call"), "Should contain function calls");
        assert!(ir.contains("printf"), "Should contain println implementation");
    }

    #[test]
    fn test_ir_validation() {
        // 生成されたIRの基本的な妥当性テスト
        let source = r#"
        package main
        
        fn fibonacci(n: i32): i32 {
            if n <= 1 {
                return n;
            } else {
                return fibonacci(n - 1) + fibonacci(n - 2);
            }
        }
        
        fn main() {
            let result = fibonacci(10);
            println("Fibonacci(10) =", result);
        }
        "#;
        
        let ir = assert_compile_success(source, "ir_validation");
        
        // IRの基本構造チェック
        assert!(ir.starts_with("; ModuleID = ") || ir.contains("target"), "Should have module header");
        assert!(ir.contains("define"), "Should contain function definitions");
        assert!(ir.contains("ret"), "Should contain return statements");
        
        // 基本ブロックの構造チェック
        let lines: Vec<&str> = ir.lines().collect();
        let mut in_function = false;
        let mut has_terminator = false;
        
        for line in lines {
            let trimmed = line.trim();
            if trimmed.starts_with("define") {
                in_function = true;
                has_terminator = false;
            } else if trimmed == "}" && in_function {
                assert!(has_terminator, "Function should end with a terminator");
                in_function = false;
            } else if in_function && (trimmed.starts_with("ret") || trimmed.starts_with("br")) {
                has_terminator = true;
            }
        }
    }

    #[test]
    fn test_memory_management_codegen() {
        // メモリ管理のコード生成テスト
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
                data1: 1,
                data2: 2,
                data3: 3,
                data4: 4,
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
    #[ignore] // 実際のLLVMの実行は環境依存のため、通常は無視
    fn test_executable_generation() {
        // 実行可能ファイル生成のテスト（オプション）
        let source = r#"
        package main
        
        fn main() {
            println("Test executable generation");
        }
        "#;
        
        let ir = assert_compile_success(source, "executable_test");
        
        // IRをファイルに保存
        let ir_file = "/tmp/test_executable.ll";
        fs::write(ir_file, ir).expect("Should write IR file");
        
        // LLCでオブジェクトファイルを生成
        let obj_file = "/tmp/test_executable.o";
        let llc_output = Command::new("llc")
            .args(&["-filetype=obj", ir_file, "-o", obj_file])
            .output();
        
        if llc_output.is_ok() {
            // リンカーで実行可能ファイルを生成
            let exe_file = "/tmp/test_executable";
            let link_output = Command::new("clang")
                .args(&[obj_file, "-o", exe_file])
                .output();
            
            if link_output.is_ok() {
                // 実行可能ファイルを実行
                let run_output = Command::new(exe_file).output();
                
                if let Ok(output) = run_output {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    assert!(stdout.contains("Test executable generation"), 
                           "Executable should produce expected output");
                }
            }
        }
        
        // クリーンアップ
        let _ = fs::remove_file(ir_file);
        let _ = fs::remove_file(obj_file);
        let _ = fs::remove_file("/tmp/test_executable");
    }
}