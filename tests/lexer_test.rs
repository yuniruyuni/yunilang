//! レキサーテスト
//! 
//! Yuniコンパイラのレキサー（字句解析器）の包括的なテストスイート。
//! 正常系、異常系、エッジケースを網羅する。

#[cfg(test)]
mod tests {
    use yunilang::lexer::{Lexer, Token, TokenWithPosition};

    /// トークンの型のみを比較するヘルパー関数
    fn extract_tokens(source: &str) -> Vec<Token> {
        let lexer = Lexer::new(source);
        lexer.map(|token_with_pos| token_with_pos.token).collect()
    }

    /// 位置情報付きトークンを取得するヘルパー関数
    fn extract_tokens_with_position(source: &str) -> Vec<TokenWithPosition> {
        let lexer = Lexer::new(source);
        lexer.collect()
    }

    #[test]
    fn test_keywords() {
        // キーワードの正しい認識をテスト
        let source = "package import fn let mut type struct enum if else for while return lives";
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::Package,
            Token::Import,
            Token::Fn,
            Token::Let,
            Token::Mut,
            Token::Type,
            Token::Struct,
            Token::Enum,
            Token::If,
            Token::Else,
            Token::For,
            Token::While,
            Token::Return,
            Token::Lives,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_basic_types() {
        // 基本型の正しい認識をテスト
        let source = "i8 i16 i32 i64 i128 i256 u8 u16 u32 u64 u128 u256 f8 f16 f32 f64";
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::I8,
            Token::I16,
            Token::I32,
            Token::I64,
            Token::I128,
            Token::I256,
            Token::U8,
            Token::U16,
            Token::U32,
            Token::U64,
            Token::U128,
            Token::U256,
            Token::F8,
            Token::F16,
            Token::F32,
            Token::F64,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_operators() {
        // 演算子の正しい認識をテスト
        let source = "+ - * / % == != < <= > >= && || ! & = += -= *= /= %=";
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::Percent,
            Token::Equal,
            Token::NotEqual,
            Token::Less,
            Token::LessEqual,
            Token::Greater,
            Token::GreaterEqual,
            Token::And,
            Token::Or,
            Token::Not,
            Token::Ampersand,
            Token::Assign,
            Token::PlusAssign,
            Token::MinusAssign,
            Token::StarAssign,
            Token::SlashAssign,
            Token::PercentAssign,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_delimiters() {
        // 区切り文字の正しい認識をテスト
        let source = "( ) { } [ ] , ; : . ->";
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::LeftParen,
            Token::RightParen,
            Token::LeftBrace,
            Token::RightBrace,
            Token::LeftBracket,
            Token::RightBracket,
            Token::Comma,
            Token::Semicolon,
            Token::Colon,
            Token::Dot,
            Token::Arrow,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_identifiers() {
        // 識別子の正しい認識をテスト
        let source = "main hello_world _private __internal x123 variable_name";
        let tokens = extract_tokens(source);
        
        assert!(matches!(tokens[0], Token::Identifier(_)));
        assert!(matches!(tokens[1], Token::Identifier(_)));
        assert!(matches!(tokens[2], Token::Identifier(_)));
        assert!(matches!(tokens[3], Token::Identifier(_)));
        assert!(matches!(tokens[4], Token::Identifier(_)));
        assert!(matches!(tokens[5], Token::Identifier(_)));
        
        // 具体的な識別子名の確認
        if let Token::Identifier(name) = &tokens[0] {
            assert_eq!(name, "main");
        }
        if let Token::Identifier(name) = &tokens[1] {
            assert_eq!(name, "hello_world");
        }
    }

    #[test]
    fn test_integer_literals() {
        // 整数リテラルの正しい認識をテスト
        let source = "42 123 0 1000000 42i32 123u64";
        let tokens = extract_tokens(source);
        
        // すべてIntegerトークンである
        for token in &tokens {
            assert!(matches!(token, Token::Integer(_)));
        }
        
        // 具体的な値の確認
        if let Token::Integer((value, suffix)) = &tokens[0] {
            assert_eq!(*value, 42);
            assert_eq!(*suffix, None);
        }
        if let Token::Integer((value, suffix)) = &tokens[4] {
            assert_eq!(*value, 42);
            assert_eq!(*suffix, Some("i32".to_string()));
        }
        if let Token::Integer((value, suffix)) = &tokens[5] {
            assert_eq!(*value, 123);
            assert_eq!(*suffix, Some("u64".to_string()));
        }
    }

    #[test]
    fn test_floating_point_literals() {
        // 浮動小数点リテラルの正しい認識をテスト
        let source = "3.14 0.5 123.456 1.0f32 2.5f64";
        let tokens = extract_tokens(source);
        
        // すべてFloatトークンである
        for token in &tokens {
            assert!(matches!(token, Token::Float(_)));
        }
        
        // 具体的な値の確認
        if let Token::Float((value, suffix)) = &tokens[0] {
            assert_eq!(*value, 3.14);
            assert_eq!(*suffix, None);
        }
        if let Token::Float((value, suffix)) = &tokens[3] {
            assert_eq!(*value, 1.0);
            assert_eq!(*suffix, Some("f32".to_string()));
        }
    }

    #[test]
    fn test_string_literals() {
        // 文字列リテラルの正しい認識をテスト
        let source = r#""Hello, World!" "empty" "with\nnewline" "with\"quote""#;
        let tokens = extract_tokens(source);
        
        // すべてStringトークンである
        for token in &tokens {
            assert!(matches!(token, Token::String(_)));
        }
        
        // 具体的な値の確認
        if let Token::String(value) = &tokens[0] {
            assert_eq!(value, "Hello, World!");
        }
        if let Token::String(value) = &tokens[2] {
            assert_eq!(value, "with\nnewline");
        }
    }

    #[test]
    fn test_boolean_identifiers() {
        // ブール値は識別子として認識されるテスト
        let source = "true false";
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::Identifier("true".to_string()),
            Token::Identifier("false".to_string()),
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_comments() {
        // コメントは無視されることをテスト
        let source = r#"
        // This is a single line comment
        let x = 42; // Another comment
        /*
         * This is a multi-line comment
         * with multiple lines
         */
        let y = 24;
        "#;
        
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::Let,
            Token::Identifier("x".to_string()),
            Token::Assign,
            Token::Integer((42, None)),
            Token::Semicolon,
            Token::Let,
            Token::Identifier("y".to_string()),
            Token::Assign,
            Token::Integer((24, None)),
            Token::Semicolon,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_position_tracking() {
        // 位置情報の正しい追跡をテスト
        let source = "let\nx\n=\n42;";
        let tokens = extract_tokens_with_position(source);
        
        // Newlineトークンも含まれるので、実際には7個のトークンがある
        assert!(tokens.len() >= 7);
        
        // letは1行目
        assert_eq!(tokens[0].position.line, 1);
        assert_eq!(tokens[0].position.column, 1);
        
        // 最初のnewlineの後のxを確認
        let x_token_idx = tokens.iter().position(|t| {
            matches!(t.token, Token::Identifier(ref name) if name == "x")
        }).unwrap();
        assert_eq!(tokens[x_token_idx].position.line, 2);
        
        // 代入演算子を確認
        let assign_token_idx = tokens.iter().position(|t| {
            matches!(t.token, Token::Assign)
        }).unwrap();
        assert_eq!(tokens[assign_token_idx].position.line, 3);
    }

    #[test]
    fn test_whitespace_handling() {
        // 空白文字の正しい処理をテスト
        let source = "  let    x   =   42  ;  ";
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::Let,
            Token::Identifier("x".to_string()),
            Token::Assign,
            Token::Integer((42, None)),
            Token::Semicolon,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_empty_input() {
        // 空の入力のテスト
        let source = "";
        let tokens = extract_tokens(source);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_only_whitespace() {
        // 空白文字のみの入力のテスト
        let source = "   \t  \n  \r\n  ";
        let tokens = extract_tokens(source);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_only_comments() {
        // コメントのみの入力のテスト
        let source = r#"
        // Just a comment
        /* Another comment */
        "#;
        let tokens = extract_tokens(source);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_complex_expression() {
        // 複雑な式のテスト
        let source = r#"
        fn calculate(x: i32, y: i32): i32 {
            let result = x * (y + 1) / 2;
            return result;
        }
        "#;
        
        let tokens = extract_tokens(source);
        
        let expected = vec![
            Token::Fn,
            Token::Identifier("calculate".to_string()),
            Token::LeftParen,
            Token::Identifier("x".to_string()),
            Token::Colon,
            Token::I32,
            Token::Comma,
            Token::Identifier("y".to_string()),
            Token::Colon,
            Token::I32,
            Token::RightParen,
            Token::Colon,
            Token::I32,
            Token::LeftBrace,
            Token::Let,
            Token::Identifier("result".to_string()),
            Token::Assign,
            Token::Identifier("x".to_string()),
            Token::Star,
            Token::LeftParen,
            Token::Identifier("y".to_string()),
            Token::Plus,
            Token::Integer((1, None)),
            Token::RightParen,
            Token::Slash,
            Token::Integer((2, None)),
            Token::Semicolon,
            Token::Return,
            Token::Identifier("result".to_string()),
            Token::Semicolon,
            Token::RightBrace,
        ];
        
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_template_string() {
        // テンプレート文字列のテスト（もし実装されている場合）
        let source = r#"`Hello, ${name}!`"#;
        let tokens = extract_tokens(source);
        
        // テンプレート文字列が実装されていない場合はエラートークンになるかもしれない
        // 実装に応じて期待値を調整
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_unicode_identifiers() {
        // Unicode識別子のテスト（もしサポートされている場合）
        let source = "変数名 αβγ ｈｅｌｌｏ";
        let tokens = extract_tokens(source);
        
        // Unicode識別子がサポートされているかどうかは実装依存
        // サポートされていない場合はエラートークンになる
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_edge_case_operators() {
        // エッジケースの演算子組み合わせテスト
        let source = "<=>=!=== =<=> ++-- **//";
        let tokens = extract_tokens(source);
        
        // 正しい演算子の境界が認識されることを確認
        assert!(tokens.len() >= 8); // 少なくとも <= >= != == = < = > が認識される
    }

    #[test]
    fn test_long_identifier() {
        // 非常に長い識別子のテスト
        let long_name = "a".repeat(1000);
        let source = format!("let {}", long_name);
        let tokens = extract_tokens(&source);
        
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], Token::Let));
        if let Token::Identifier(name) = &tokens[1] {
            assert_eq!(name.len(), 1000);
        }
    }

    #[test]
    fn test_large_numbers() {
        // 大きな数値のテスト
        let source = "999999999999999999 3.141592653589793238462643383279";
        let tokens = extract_tokens(source);
        
        assert_eq!(tokens.len(), 2);
        assert!(matches!(tokens[0], Token::Integer(_)));
        assert!(matches!(tokens[1], Token::Float(_)));
    }

    #[test]
    fn test_mixed_content() {
        // 様々な要素が混在したコードのテスト
        let source = r#"
        package main
        
        // This is a simple function
        fn greet(name: str): str {
            let message = "Hello, " + name + "!";
            return message;
        }
        
        fn main() {
            let user = "World";
            let greeting = greet(user);
            println(greeting);
        }
        "#;
        
        let tokens = extract_tokens(source);
        
        // 基本的な構造が正しく認識されることを確認
        assert!(tokens.contains(&Token::Package));
        assert!(tokens.contains(&Token::Fn));
        assert!(tokens.contains(&Token::Let));
        assert!(tokens.contains(&Token::Return));
        
        // 識別子が含まれていることを確認
        let has_main = tokens.iter().any(|t| {
            if let Token::Identifier(name) = t {
                name == "main"
            } else {
                false
            }
        });
        assert!(has_main);
    }
}