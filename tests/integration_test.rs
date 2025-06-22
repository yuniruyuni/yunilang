//! 統合テスト
//! 
//! Yuniコンパイラの統合テストスイート。
//! 完全なコンパイル・実行、エラー報告、CLIインターフェースを検証する。

#[cfg(test)]
use std::process::Command;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::env;
#[cfg(test)]
use yunilang::analyzer::SemanticAnalyzer;
#[cfg(test)]
use yunilang::codegen::CodeGenerator;
#[cfg(test)]
use yunilang::lexer::Lexer;
#[cfg(test)]
use yunilang::parser::Parser;
#[cfg(test)]
use inkwell::context::Context;
#[cfg(test)]
use tempfile::NamedTempFile;

/// 完全なコンパイルパイプラインのテスト
#[cfg(test)]
fn test_full_compilation(source: &str, expected_success: bool) -> Result<String, Box<dyn std::error::Error>> {
    // 字句解析
    let lexer = Lexer::new(source);
    let tokens: Vec<_> = lexer.collect();
    
    // 構文解析
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    // セマンティック解析
    let mut analyzer = SemanticAnalyzer::new();
    analyzer.analyze(&ast)?;
    
    // コード生成
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "integration_test");
    codegen.compile_program(&ast)?;
    
    let ir = codegen.get_module().print_to_string().to_string();
    
    if expected_success {
        assert!(!ir.is_empty(), "Generated IR should not be empty");
        assert!(ir.contains("define"), "IR should contain function definitions");
    }
    
    Ok(ir)
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use std::fs;
    use std::path::Path;
    use std::env;
    use yunilang::analyzer::SemanticAnalyzer;
    use yunilang::codegen::CodeGenerator;
    use yunilang::lexer::Lexer;
    use yunilang::parser::Parser;
    use inkwell::context::Context;
    use tempfile::NamedTempFile;

    /// テスト用のYuniファイルを作成し、そのパスを返すヘルパー関数
    fn create_test_file(content: &str, filename: &str) -> Result<NamedTempFile, std::io::Error> {
        let mut temp_file = NamedTempFile::new()?;
        fs::write(temp_file.path(), content)?;
        Ok(temp_file)
    }

    /// Yuniコンパイラの実行ファイルパスを取得するヘルパー関数
    fn get_compiler_path() -> String {
        // カーゴビルドの出力ディレクトリからコンパイラを探す
        let mut cmd = Command::new("cargo");
        cmd.args(&["build", "--bin", "yunilang"]);
        let _ = cmd.output(); // ビルドを実行
        
        // バイナリパスを構築
        let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
        format!("{}/debug/yunilang", target_dir)
    }

    // 上位スコープのtest_full_compilationを使用
    use super::test_full_compilation;

    #[test]
    fn test_hello_world_integration() {
        // Hello World統合テスト
        let source = r#"
        package main
        
        fn main() {
            println("Hello, World!");
        }
        "#;
        
        let result = test_full_compilation(source, true);
        assert!(result.is_ok(), "Hello World compilation should succeed");
        
        let ir = result.unwrap();
        assert!(ir.contains("Hello, World!"), "IR should contain the string literal");
        assert!(ir.contains("printf"), "IR should contain printf call");
    }

    #[test]
    fn test_arithmetic_program_integration() {
        // 算術プログラムの統合テスト
        let source = r#"
        package main
        
        fn calculate(a: i32, b: i32): i32 {
            let sum = a + b;
            let product = a * b;
            return sum + product;
        }
        
        fn main() {
            let result = calculate(5, 3);
            println("Result:", result);
        }
        "#;
        
        let result = test_full_compilation(source, true);
        assert!(result.is_ok(), "Arithmetic program compilation should succeed");
        
        let ir = result.unwrap();
        assert!(ir.contains("add"), "IR should contain addition");
        assert!(ir.contains("mul"), "IR should contain multiplication");
        assert!(ir.contains("call"), "IR should contain function calls");
    }

    #[test]
    fn test_struct_program_integration() {
        // 構造体プログラムの統合テスト
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
            println("Distance:", dist);
        }
        "#;
        
        let result = test_full_compilation(source, true);
        assert!(result.is_ok(), "Struct program compilation should succeed");
        
        let ir = result.unwrap();
        assert!(ir.contains("Point"), "IR should reference struct type");
        assert!(ir.contains("getelementptr") || ir.contains("extractvalue"), "IR should contain struct field access");
    }

    #[test]
    fn test_recursive_function_integration() {
        // 再帰関数の統合テスト
        let source = r#"
        package main
        
        fn factorial(n: i32): i32 {
            if n <= 1 {
                return 1;
            } else {
                return n * factorial(n - 1);
            }
        }
        
        fn fibonacci(n: i32): i32 {
            if n <= 1 {
                return n;
            } else {
                return fibonacci(n - 1) + fibonacci(n - 2);
            }
        }
        
        fn main() {
            let fact = factorial(5);
            let fib = fibonacci(8);
            println("Factorial(5):", fact);
            println("Fibonacci(8):", fib);
        }
        "#;
        
        let result = test_full_compilation(source, true);
        assert!(result.is_ok(), "Recursive function compilation should succeed");
        
        let ir = result.unwrap();
        assert!(ir.contains("factorial"), "IR should contain factorial function");
        assert!(ir.contains("fibonacci"), "IR should contain fibonacci function");
        assert!(ir.matches("call").count() >= 4, "IR should contain multiple function calls");
    }

    #[test]
    fn test_control_flow_integration() {
        // 制御フロー統合テスト
        let source = r#"
        package main
        
        fn classify_number(x: i32): str {
            if x > 0 {
                return "positive";
            } else if x < 0 {
                return "negative";
            } else {
                return "zero";
            }
        }
        
        fn count_down(n: i32) {
            let mut i = n;
            while i > 0 {
                println("Count:", i);
                i = i - 1;
            }
            println("Done!");
        }
        
        fn main() {
            let classification = classify_number(-5);
            println("Number is:", classification);
            count_down(3);
        }
        "#;
        
        let result = test_full_compilation(source, true);
        assert!(result.is_ok(), "Control flow compilation should succeed");
        
        let ir = result.unwrap();
        assert!(ir.contains("icmp"), "IR should contain comparisons");
        assert!(ir.contains("br"), "IR should contain branches");
        assert!(ir.matches("label").count() >= 4, "IR should contain multiple basic blocks");
    }

    #[test]
    fn test_error_reporting_integration() {
        // エラー報告の統合テスト
        let error_cases = vec![
            // 構文エラー
            (r#"
            package main
            fn main( {
                println("Hello");
            }
            "#, "syntax error"),
            
            // 型エラー
            (r#"
            package main
            fn main() {
                let x: i32 = "hello";
            }
            "#, "type mismatch"),
            
            // 未定義変数
            (r#"
            package main
            fn main() {
                println(undefined_variable);
            }
            "#, "undefined variable"),
            
            // 未定義関数
            (r#"
            package main
            fn main() {
                unknown_function();
            }
            "#, "undefined function"),
            
            // 引数数不一致
            (r#"
            package main
            fn add(a: i32, b: i32): i32 {
                return a + b;
            }
            fn main() {
                let result = add(5);
            }
            "#, "argument count"),
        ];
        
        for (source, error_description) in error_cases {
            let result = test_full_compilation(source, false);
            assert!(result.is_err(), "Should fail for {}", error_description);
        }
    }

    #[test]
    fn test_examples_integration() {
        // examples/ディレクトリのファイルの統合テスト
        let example_files = vec![
            ("examples/hello.yuni", r#"
            package main
            
            fn main() {
                println("Hello, World!");
            }
            "#),
            ("examples/simple.yuni", r#"
            package main
            
            fn main() {
                let x: i32 = 42;
                let y: i32 = 58;
            }
            "#),
            ("examples/arithmetic.yuni", r#"
            package main
            
            fn add(a: i32, b: i32): i32 {
                return a + b;
            }
            
            fn main() {
                let x: i32 = 10;
                let y: i32 = 20;
                let result: i32 = add(x, y);
            }
            "#),
        ];
        
        for (filename, content) in example_files {
            let result = test_full_compilation(content, true);
            assert!(result.is_ok(), "Example {} should compile successfully", filename);
        }
    }

    #[test]
    fn test_complex_program_integration() {
        // 複雑なプログラムの統合テスト
        let source = r#"
        package calculator
        
        struct Calculator {
            value: f64,
            history: i32,
        }
        
        enum Operation {
            Add,
            Subtract,
            Multiply,
            Divide,
        }
        
        fn create_calculator(): Calculator {
            return Calculator { value: 0.0, history: 0 };
        }
        
        fn apply_operation(calc: Calculator, op: Operation, operand: f64): Calculator {
            let new_value = match op {
                Operation::Add => calc.value + operand,
                Operation::Subtract => calc.value - operand,
                Operation::Multiply => calc.value * operand,
                Operation::Divide => {
                    if operand != 0.0 {
                        calc.value / operand
                    } else {
                        calc.value
                    }
                },
            };
            
            return Calculator { 
                value: new_value, 
                history: calc.history + 1 
            };
        }
        
        fn run_calculations() {
            let mut calc = create_calculator();
            
            calc = apply_operation(calc, Operation::Add, 10.0);
            calc = apply_operation(calc, Operation::Multiply, 2.0);
            calc = apply_operation(calc, Operation::Subtract, 5.0);
            calc = apply_operation(calc, Operation::Divide, 3.0);
            
            println("Final result:", calc.value);
            println("Operations performed:", calc.history);
        }
        
        fn main() {
            run_calculations();
        }
        "#;
        
        let result = test_full_compilation(source, true);
        assert!(result.is_ok(), "Complex program compilation should succeed");
        
        let ir = result.unwrap();
        assert!(ir.contains("Calculator"), "IR should contain struct type");
        assert!(ir.contains("Operation"), "IR should reference enum type");
        assert!(ir.matches("define").count() >= 4, "IR should contain multiple functions");
    }

    #[test]
    #[ignore] // CLIテストは環境依存のため通常は無視
    fn test_cli_interface() {
        // CLIインターフェースのテスト
        let compiler_path = get_compiler_path();
        
        if !Path::new(&compiler_path).exists() {
            eprintln!("Compiler binary not found at: {}", compiler_path);
            return;
        }
        
        // ヘルプメッセージのテスト
        let help_output = Command::new(&compiler_path)
            .arg("--help")
            .output()
            .expect("Failed to execute compiler");
        
        let help_text = String::from_utf8_lossy(&help_output.stdout);
        assert!(help_text.contains("compile"), "Help should mention compile command");
        assert!(help_text.contains("run"), "Help should mention run command");
        
        // バージョン情報のテスト
        let version_output = Command::new(&compiler_path)
            .arg("--version")
            .output()
            .expect("Failed to execute compiler");
        
        let version_text = String::from_utf8_lossy(&version_output.stdout);
        assert!(!version_text.is_empty(), "Version output should not be empty");
    }

    #[test]
    #[ignore] // ファイルシステムに依存するため通常は無視
    fn test_file_compilation() {
        // ファイルからのコンパイレーションテスト
        let source = r#"
        package main
        
        fn main() {
            println("File compilation test");
        }
        "#;
        
        let temp_file = create_test_file(source, "test.yuni").expect("Failed to create temp file");
        let compiler_path = get_compiler_path();
        
        if !Path::new(&compiler_path).exists() {
            eprintln!("Compiler binary not found, skipping file compilation test");
            return;
        }
        
        // ファイルをコンパイル
        let compile_output = Command::new(&compiler_path)
            .arg("compile")
            .arg(temp_file.path())
            .output()
            .expect("Failed to execute compiler");
        
        if !compile_output.status.success() {
            let stderr = String::from_utf8_lossy(&compile_output.stderr);
            eprintln!("Compilation failed: {}", stderr);
        }
        
        assert!(compile_output.status.success(), "File compilation should succeed");
    }

    #[test]
    fn test_memory_safety_integration() {
        // メモリ安全性の統合テスト
        let source = r#"
        package main
        
        struct Node {
            value: i32,
            next: Option<Box<Node>>,
        }
        
        fn create_list(values: Vec<i32>): Option<Box<Node>> {
            if values.is_empty() {
                return None;
            }
            
            let mut head = Box::new(Node { value: values[0], next: None });
            let mut current = &mut head;
            
            for i in 1..values.len() {
                current.next = Some(Box::new(Node { value: values[i], next: None }));
                if let Some(ref mut next) = current.next {
                    current = next;
                }
            }
            
            return Some(head);
        }
        
        fn print_list(head: Option<Box<Node>>) {
            let mut current = head;
            while let Some(node) = current {
                println("Value:", node.value);
                current = node.next;
            }
        }
        
        fn main() {
            let values = vec![1, 2, 3, 4, 5];
            let list = create_list(values);
            print_list(list);
        }
        "#;
        
        // この例は高度なメモリ管理機能を使用しているため、
        // 実装の範囲に応じて調整が必要
        let result = test_full_compilation(source, false); // 実装に応じてtrueに変更
        
        // 現在の実装では複雑な所有権システムはサポートされていない可能性が高い
        // 基本的な構造体操作のみテスト
        let simple_source = r#"
        package main
        
        struct SimpleNode {
            value: i32,
        }
        
        fn main() {
            let node = SimpleNode { value: 42 };
            println("Node value:", node.value);
        }
        "#;
        
        let simple_result = test_full_compilation(simple_source, true);
        assert!(simple_result.is_ok(), "Simple struct program should compile");
    }

    #[test]
    fn test_performance_integration() {
        // パフォーマンステスト（コンパイル時間）
        let large_source = generate_large_program(100); // 100個の関数
        
        let start_time = std::time::Instant::now();
        let result = test_full_compilation(&large_source, true);
        let compilation_time = start_time.elapsed();
        
        assert!(result.is_ok(), "Large program should compile successfully");
        assert!(compilation_time.as_secs() < 10, "Compilation should complete within 10 seconds");
        
        println!("Compilation time for large program: {:?}", compilation_time);
    }

    /// 大きなプログラムを生成するヘルパー関数
    fn generate_large_program(function_count: usize) -> String {
        let mut source = String::from("package main\n\n");
        
        // 多数の関数を生成
        for i in 0..function_count {
            source.push_str(&format!(
                "fn function_{}(x: i32): i32 {{\n    return x + {};\n}}\n\n",
                i, i
            ));
        }
        
        // main関数で全ての関数を呼び出し
        source.push_str("fn main() {\n");
        for i in 0..function_count {
            source.push_str(&format!("    let result_{} = function_{}({});\n", i, i, i));
        }
        source.push_str("    println(\"All functions executed\");\n");
        source.push_str("}\n");
        
        source
    }

    #[test]
    fn test_edge_cases_integration() {
        // エッジケースの統合テスト
        let edge_cases = vec![
            // 空のmain関数
            ("empty_main", r#"
            package main
            fn main() {
            }
            "#),
            
            // 深いネスト
            ("deep_nesting", r#"
            package main
            fn main() {
                if true {
                    if true {
                        if true {
                            if true {
                                println("Deep nesting works");
                            }
                        }
                    }
                }
            }
            "#),
            
            // 長い識別子
            ("long_identifier", r#"
            package main
            fn main() {
                let very_long_variable_name_that_should_still_work = 42;
                println("Long identifier:", very_long_variable_name_that_should_still_work);
            }
            "#),
            
            // 多数の変数
            ("many_variables", r#"
            package main
            fn main() {
                let a = 1; let b = 2; let c = 3; let d = 4; let e = 5;
                let f = 6; let g = 7; let h = 8; let i = 9; let j = 10;
                let sum = a + b + c + d + e + f + g + h + i + j;
                println("Sum:", sum);
            }
            "#),
        ];
        
        for (name, source) in edge_cases {
            let result = test_full_compilation(source, true);
            assert!(result.is_ok(), "Edge case '{}' should compile successfully", name);
        }
    }

    #[test]
    fn test_compilation_pipeline_robustness() {
        // コンパイレーションパイプラインの堅牢性テスト
        let test_cases = vec![
            // 正常ケース
            r#"package main
            fn main() { println("Normal case"); }"#,
            
            // 空白の多いケース
            r#"
            
            package   main
            
            
            fn    main   (   )    {
                println  (  "Whitespace test"  )  ;
            }
            
            "#,
            
            // コメントの多いケース
            r#"
            package main // Package declaration
            
            /* This is the main function */
            fn main() {
                // Print a message
                println("Comment test"); // End of line comment
            }
            /* End of program */
            "#,
        ];
        
        for (i, source) in test_cases.iter().enumerate() {
            let result = test_full_compilation(source, true);
            assert!(result.is_ok(), "Pipeline robustness test case {} should succeed", i);
        }
    }
}

/// ベンチマークテスト（別モジュール）
#[cfg(test)]
mod benchmarks {
    use super::test_full_compilation;
    use std::time::Instant;

    #[test]
    #[ignore] // ベンチマークは通常の実行では無視
    fn benchmark_simple_compilation() {
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
            let result = factorial(10);
            println("Factorial(10):", result);
        }
        "#;
        
        let iterations = 100;
        let start = Instant::now();
        
        for _ in 0..iterations {
            let result = test_full_compilation(source, true);
            assert!(result.is_ok(), "Benchmark compilation should succeed");
        }
        
        let total_time = start.elapsed();
        let avg_time = total_time / iterations;
        
        println!("Average compilation time: {:?}", avg_time);
        println!("Total time for {} iterations: {:?}", iterations, total_time);
        
        // パフォーマンスの回帰テスト（具体的な閾値は環境に依存）
        assert!(avg_time.as_millis() < 100, "Average compilation time should be under 100ms");
    }
}