//! 末尾再帰最適化のテスト

#[cfg(test)]
mod tests {
    use yunilang::analyzer::SemanticAnalyzer;
    use yunilang::codegen::CodeGenerator;
    use yunilang::lexer::Lexer;
    use yunilang::parser::Parser;
    
    use inkwell::context::Context;

    /// ソースコードをLLVM IRにコンパイルし、末尾呼び出しが最適化されていることを確認
    fn compile_and_check_tail_call(source: &str) -> Result<String, Box<dyn std::error::Error>> {
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
        let mut codegen = CodeGenerator::new(&context, "test_module");
        codegen.compile_program(&ast)?;
        
        // LLVM IRを文字列として取得
        Ok(codegen.get_module().print_to_string().to_string())
    }

    #[test]
    fn test_tail_recursive_factorial() {
        let source = r#"
            package test
            
            fn factorial_acc(n: i32, acc: i32) : i32 {
                if n <= 1 {
                    return acc;
                } else {
                    return factorial_acc(n - 1, n * acc);
                }
            }
            
            fn factorial(n: i32) : i32 {
                return factorial_acc(n, 1);
            }
        "#;

        let result = compile_and_check_tail_call(source);
        assert!(result.is_ok(), "Compilation should succeed: {:?}", result.unwrap_err());
        
        let ir = result.unwrap();
        
        // 末尾呼び出しがあることを確認
        assert!(ir.contains("tail call"), "Should contain tail call optimization");
    }

    #[test]
    fn test_tail_recursive_sum() {
        let source = r#"
            package test
            
            fn sum_acc(n: i32, acc: i32) : i32 {
                if n <= 0 {
                    return acc;
                }
                return sum_acc(n - 1, acc + n);
            }
            
            fn sum(n: i32) : i32 {
                return sum_acc(n, 0);
            }
        "#;

        let result = compile_and_check_tail_call(source);
        assert!(result.is_ok(), "Compilation should succeed: {:?}", result.unwrap_err());
        
        let ir = result.unwrap();
        
        // 末尾呼び出しがあることを確認
        assert!(ir.contains("tail call"), "Should contain tail call optimization");
    }

    #[test]
    fn test_non_tail_recursive() {
        let source = r#"
            package test
            
            fn factorial(n: i32) : i32 {
                if n <= 1 {
                    return 1;
                } else {
                    return n * factorial(n - 1);
                }
            }
        "#;

        let result = compile_and_check_tail_call(source);
        assert!(result.is_ok(), "Compilation should succeed: {:?}", result.unwrap_err());
        
        let ir = result.unwrap();
        
        // 末尾呼び出しではないことを確認（n * factorial(n-1)の形）
        // factorialの呼び出しはあるが、tail callではない
        assert!(ir.contains("call") && !ir.contains("tail call"), 
                "Should contain normal call, not tail call");
    }

    #[test]
    fn test_mutual_tail_recursion() {
        let source = r#"
            package test
            
            fn is_even(n: i32) : bool {
                if n == 0 {
                    return true;
                }
                return is_odd(n - 1);
            }
            
            fn is_odd(n: i32) : bool {
                if n == 0 {
                    return false;
                }
                return is_even(n - 1);
            }
        "#;

        let result = compile_and_check_tail_call(source);
        assert!(result.is_ok(), "Compilation should succeed: {:?}", result.unwrap_err());
        
        let ir = result.unwrap();
        
        // 相互再帰は自己呼び出しではないため、末尾呼び出し最適化されない
        assert!(!ir.contains("tail call"), 
                "Mutual recursion should not be optimized as tail call");
    }

    #[test]
    #[ignore = "Parser does not yet support match expressions syntax"]
    fn test_tail_call_in_match() {
        let source = r#"
            package test
            
            fn count_down(n: i32) : i32 {
                match n {
                    0 => 0,
                    _ => count_down(n - 1)
                }
            }
        "#;

        let result = compile_and_check_tail_call(source);
        assert!(result.is_ok(), "Compilation should succeed: {:?}", result.unwrap_err());
        
        let ir = result.unwrap();
        
        // match式の中の末尾呼び出しも最適化されることを確認
        assert!(ir.contains("tail call"), "Should optimize tail call in match expression");
    }
}