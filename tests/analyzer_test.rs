//! セマンティック解析テスト
//! 
//! Yuniコンパイラのセマンティック解析器の包括的なテストスイート。
//! 型チェック、スコープ解決、所有権検証、ライフタイム検証を網羅する。

#[cfg(test)]
mod tests {
    use yunilang::analyzer::SemanticAnalyzer;
    use yunilang::error::{YuniError, AnalyzerError};
    use yunilang::lexer::Lexer;
    use yunilang::parser::Parser;
    use yunilang::ast::*;

    /// ソースコードを解析してASTを取得し、セマンティック解析を実行するヘルパー関数
    fn analyze_source(source: &str) -> Result<Program, YuniError> {
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.collect_tokens();
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");
        
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&ast)?;
        Ok(ast)
    }

    /// 解析に成功することを確認するヘルパー関数
    fn assert_analysis_success(source: &str) -> Program {
        analyze_source(source).expect("Analysis should succeed")
    }

    /// 解析に失敗することを確認するヘルパー関数
    fn assert_analysis_error(source: &str) {
        assert!(analyze_source(source).is_err(), "Analysis should fail");
    }

    /// 特定のエラータイプが発生することを確認するヘルパー関数
    fn assert_specific_error<F>(source: &str, check: F) 
    where F: Fn(&AnalyzerError) -> bool {
        let result = analyze_source(source);
        assert!(result.is_err(), "Analysis should fail");
        if let Err(YuniError::Analyzer(error)) = result {
            assert!(check(&error), "Expected specific error type, got: {:?}", error);
        } else if let Err(e) = result {
            panic!("Expected AnalyzerError, got: {:?}", e);
        }
    }

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
    fn test_struct_type_checking() {
        // 構造体の型チェックのテスト
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
        
        assert_analysis_success(source);
    }

    #[test]
    fn test_enum_type_checking() {
        // 列挙型の型チェックのテスト
        let source = r#"
        package main
        
        enum Color {
            Red,
            Green,
            Blue,
        }
        
        enum Option {
            Some { value: i32 },
            None,
        }
        
        fn get_color_name(color: Color): str {
            return match color {
                Color::Red => "red",
                Color::Green => "green",
                Color::Blue => "blue",
            };
        }
        
        fn main() {
            let color = Color::Red;
            let name = get_color_name(color);
            
            let maybe_value = Option::Some { value: 42 };
            let empty = Option::None;
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
            let a = 10;
            let b = 20;
            let eq = a == b;      // bool
            let ne = a != b;      // bool
            let lt = a < b;       // bool
            let le = a <= b;      // bool
            let gt = a > b;       // bool
            let ge = a >= b;      // bool
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
            let flag1 = true;
            let flag2 = false;
            let and_result = flag1 && flag2;  // bool
            let or_result = flag1 || flag2;   // bool
            let not_result = !flag1;          // bool
        }
        "#;
        
        assert_analysis_success(source);
    }

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

    // エラーケースのテスト

    #[test]
    fn test_undefined_variable_error() {
        // 未定義変数エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x = y + 1;  // yが未定義
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::UndefinedVariable { .. })
        });
    }

    #[test]
    fn test_undefined_function_error() {
        // 未定義関数エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let result = unknown_function(42);  // 未定義関数
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::UndefinedFunction { .. })
        });
    }

    #[test]
    fn test_undefined_type_error() {
        // 未定義型エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x: UnknownType = 42;  // 未定義型
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::UndefinedType { .. })
        });
    }

    #[test]
    fn test_type_mismatch_error() {
        // 型不一致エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x: i32 = "hello";  // 型不一致
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::TypeMismatch { .. })
        });
    }

    #[test]
    fn test_immutable_assignment_error() {
        // 不変変数への代入エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x = 10;
            x = 20;  // 不変変数への代入
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::ImmutableVariable { .. })
        });
    }

    #[test]
    fn test_duplicate_variable_error() {
        // 重複変数エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x = 10;
            let x = 20;  // 同じスコープで重複
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::DuplicateVariable { .. })
        });
    }

    #[test]
    fn test_duplicate_function_error() {
        // 重複関数エラーのテスト
        let source = r#"
        package main
        
        fn test() {
        }
        
        fn test() {  // 重複関数
        }
        
        fn main() {
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::DuplicateFunction { .. })
        });
    }

    #[test]
    fn test_argument_count_mismatch_error() {
        // 引数数不一致エラーのテスト
        let source = r#"
        package main
        
        fn add(a: i32, b: i32): i32 {
            return a + b;
        }
        
        fn main() {
            let result = add(5);  // 引数が不足
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::ArgumentCountMismatch { .. })
        });
    }

    #[test]
    fn test_return_type_mismatch_error() {
        // 戻り値型不一致エラーのテスト
        let source = r#"
        package main
        
        fn get_number(): i32 {
            return "hello";  // 型不一致
        }
        
        fn main() {
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::TypeMismatch { .. })
        });
    }

    #[test]
    fn test_missing_return_error() {
        // 戻り値不足エラーのテスト
        let source = r#"
        package main
        
        fn get_number(): i32 {
            let x = 42;
            // return文がない
        }
        
        fn main() {
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::MissingReturn { .. })
        });
    }

    #[test]
    fn test_arithmetic_type_mismatch_error() {
        // 算術演算型不一致エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x: i32 = 10;
            let y: f64 = 3.14;
            let result = x + y;  // 型が不一致
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::TypeMismatch { .. })
        });
    }

    #[test]
    fn test_comparison_type_mismatch_error() {
        // 比較演算型不一致エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x = 10;
            let y = "hello";
            let result = x == y;  // 型が不一致
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::TypeMismatch { .. })
        });
    }

    #[test]
    fn test_logical_operation_type_error() {
        // 論理演算型エラーのテスト
        let source = r#"
        package main
        
        fn main() {
            let x = 10;
            let result = x && true;  // i32はboolではない
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::TypeMismatch { .. })
        });
    }

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
    #[ignore = "Circular dependency detection not yet implemented"]
    fn test_circular_dependency_detection() {
        // 循環依存の検出テスト
        let source = r#"
        package main
        
        struct A {
            b: B,
        }
        
        struct B {
            a: A,  // 循環依存
        }
        
        fn main() {
        }
        "#;
        
        // 循環依存エラーは設計に依存するため、
        // 具体的な実装に応じて調整が必要
        assert_analysis_error(source);
    }

    #[test]
    fn test_nested_scope_resolution() {
        // ネストしたスコープ解決のテスト
        let source = r#"
        package main
        
        fn main() {
            let x = 10;
            {
                let x = 20;  // シャドウイング
                {
                    let x = 30;  // さらにシャドウイング
                    println(x);  // 30が出力される
                }
                println(x);  // 20が出力される
            }
            println(x);  // 10が出力される
        }
        "#;
        
        assert_analysis_success(source);
    }

    #[test]
    fn test_function_overloading_error() {
        // 関数オーバーロードエラーのテスト（Yuniでは未サポート）
        let source = r#"
        package main
        
        fn process(x: i32) {
            println(x);
        }
        
        fn process(x: f64) {  // 同名関数（オーバーロード）
            println(x);
        }
        
        fn main() {
        }
        "#;
        
        assert_specific_error(source, |e| {
            matches!(e, AnalyzerError::DuplicateFunction { .. })
        });
    }

    #[test]
    fn test_complex_expression_type_checking() {
        // 複雑な式の型チェックのテスト
        let source = r#"
        package main
        
        fn calculate(a: i32, b: i32, c: i32): i32 {
            return (a + b) * c - (a - b) / (c + 1);
        }
        
        fn main() {
            let x = 10;
            let y = 20;
            let z = 5;
            let result = calculate(x, y, z);
            
            let condition = (x > y) && (z < 10) || (result == 0);
            
            if condition {
                println("Complex condition is true");
            }
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
    fn test_multiple_error_detection() {
        // 複数エラーの検出テスト
        let source = r#"
        package main
        
        fn main() {
            let x: i32 = "hello";  // 型不一致
            let y = undefined_var;  // 未定義変数
            unknown_func();         // 未定義関数
            
            let z = 10;
            z = 20;  // 不変変数への代入
        }
        "#;
        
        // 最初のエラーで停止するか、すべてのエラーを収集するかは実装依存
        assert_analysis_error(source);
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
}