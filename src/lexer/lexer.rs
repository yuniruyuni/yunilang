//! レキサーのメイン実装

use logos::{Lexer as LogosLexer, Logos, Span};

use super::token::Token;
use super::template_string::find_interpolations;

/// 位置情報付きトークン
#[derive(Debug, Clone)]
pub struct TokenWithPosition {
    pub token: Token,
    pub span: Span,
}

/// Yuni言語のレキサー
pub struct Lexer<'a> {
    inner: LogosLexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    /// 新しいレキサーを作成
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: Token::lexer(input),
        }
    }

    /// 次のトークンを取得
    pub fn next_token(&mut self) -> Option<TokenWithPosition> {
        let token = self.inner.next()?;
        let span = self.inner.span();
        
        match token {
            Ok(token) => Some(TokenWithPosition { token, span }),
            Err(_) => {
                // エラートークンをスキップして次のトークンを試す
                self.next_token()
            }
        }
    }

    /// すべてのトークンを収集
    pub fn collect_tokens(mut self) -> Vec<TokenWithPosition> {
        let mut tokens = Vec::new();
        while let Some(token) = self.next_token() {
            tokens.push(token);
        }
        tokens
    }
}

/// ソースコードをトークン化
pub fn tokenize(input: &str) -> Vec<TokenWithPosition> {
    let mut tokens = Vec::new();
    let mut lexer = Token::lexer(input);
    
    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => {
                let span = lexer.span();
                
                // テンプレート文字列の特別処理
                if let Token::TemplateString(content) = &token {
                    // 補間位置を見つける
                    let interpolations = find_interpolations(content);
                    if !interpolations.is_empty() {
                        // テンプレート文字列に補間がある場合の処理
                        // 実際の補間式の解析はパーサーで行う
                        tokens.push(TokenWithPosition {
                            token: Token::TemplateString(content.clone()),
                            span,
                        });
                    } else {
                        // 通常のテンプレート文字列
                        tokens.push(TokenWithPosition { token, span });
                    }
                } else {
                    tokens.push(TokenWithPosition { token, span });
                }
            }
            Err(_) => {
                // レキサーエラー（未知の文字など）
                // エラーをスキップして続行
                continue;
            }
        }
    }
    
    tokens
}

/// デバッグ用：トークンストリームを文字列として出力
pub fn format_tokens(tokens: &[TokenWithPosition]) -> String {
    tokens
        .iter()
        .map(|t| format!("{:?} @ {:?}", t.token, t.span))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let input = "let x = 42;";
        let tokens = tokenize(input);
        
        assert_eq!(tokens.len(), 5);
        assert!(matches!(tokens[0].token, Token::Let));
        assert!(matches!(tokens[1].token, Token::Identifier(_)));
        assert!(matches!(tokens[2].token, Token::Assign));
        assert!(matches!(tokens[3].token, Token::Integer(_)));
        assert!(matches!(tokens[4].token, Token::Semicolon));
    }

    #[test]
    fn test_string_literal() {
        let input = r#""hello world""#;
        let tokens = tokenize(input);
        
        assert_eq!(tokens.len(), 1);
        if let Token::String(s) = &tokens[0].token {
            assert_eq!(s, "hello world");
        } else {
            panic!("Expected string token");
        }
    }

    #[test]
    fn test_template_string() {
        let input = r#"`Hello, ${name}!`"#;
        let tokens = tokenize(input);
        
        assert_eq!(tokens.len(), 1);
        if let Token::TemplateString(s) = &tokens[0].token {
            assert_eq!(s, "Hello, ${name}!");
        } else {
            panic!("Expected template string token");
        }
    }

    #[test]
    fn test_numeric_literals() {
        let input = "42 3.14 100 2.718";
        let tokens = tokenize(input);
        
        assert_eq!(tokens.len(), 4); // 数値4つ
        assert!(matches!(tokens[0].token, Token::Integer(42)));
        assert!(matches!(tokens[1].token, Token::Float(_)));
        assert!(matches!(tokens[2].token, Token::Integer(100)));
        assert!(matches!(tokens[3].token, Token::Float(_)));
    }

    #[test]
    fn test_operators() {
        let input = "+ - * / == != < > && ||";
        let tokens = tokenize(input);
        
        assert_eq!(tokens.len(), 10);
        assert!(matches!(tokens[0].token, Token::Plus));
        assert!(matches!(tokens[1].token, Token::Minus));
        assert!(matches!(tokens[2].token, Token::Star));
        assert!(matches!(tokens[3].token, Token::Slash));
        assert!(matches!(tokens[4].token, Token::EqEq));
        assert!(matches!(tokens[5].token, Token::NotEq));
        assert!(matches!(tokens[6].token, Token::Lt));
        assert!(matches!(tokens[7].token, Token::Gt));
        assert!(matches!(tokens[8].token, Token::AndAnd));
        assert!(matches!(tokens[9].token, Token::OrOr));
    }
}