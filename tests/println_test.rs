#[cfg(test)]
mod tests {
    use yunilang::analyzer::SemanticAnalyzer;
    use yunilang::codegen::CodeGenerator;
    use yunilang::lexer::Lexer;
    use yunilang::parser::Parser;

    #[test]
    fn test_println_builtin() {
        let source = r#"
        package test
        
        fn main() {
            println("Hello, World!");
            println("Value is", 42);
            println();
        }
        "#;

        // Lex the source
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.collect();

        // Parse the tokens
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        // Analyze the AST
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&ast).expect("Analysis should succeed");

        // Code generation should also work
        let context = inkwell::context::Context::create();
        let mut codegen = CodeGenerator::new(&context, "test");
        codegen.compile_program(&ast).expect("Code generation should succeed");
    }

    #[test]
    fn test_println_variadic() {
        let source = r#"
        package test
        
        fn main() {
            let x = 10;
            let y = 20;
            let name = "Alice";
            let is_happy = true;
            
            println(x, y, name, is_happy);
            println("x =", x, "y =", y, "name =", name, "happy =", is_happy);
        }
        "#;

        // Lex the source
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.collect();

        // Parse the tokens
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        // Analyze the AST
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&ast).expect("Analysis should succeed");
    }

    #[test]
    fn test_println_empty() {
        let source = r#"
        package test
        
        fn main() {
            println();
        }
        "#;

        // Lex the source
        let lexer = Lexer::new(source);
        let tokens: Vec<_> = lexer.collect();

        // Parse the tokens
        let mut parser = Parser::new(tokens);
        let ast = parser.parse().expect("Parsing should succeed");

        // Analyze the AST
        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&ast).expect("Analysis should succeed");
    }
}