//! Lexical analysis module for the Yuni language.
//!
//! This module is responsible for tokenizing Yuni source code into a stream of tokens.
//! It supports all Yuni language features including keywords, identifiers, literals,
//! operators, and template strings with interpolation.

use logos::{Lexer as LogosLexer, Logos};
use std::fmt;

/// Token types for the Yuni language
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f]+")] // Skip whitespace (except newlines)
pub enum Token {
    // Keywords
    #[token("package")]
    Package,
    #[token("import")]
    Import,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("type")]
    Type,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("lives")]
    Lives,

    // Basic types
    #[token("i8")]
    I8,
    #[token("i16")]
    I16,
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("i128")]
    I128,
    #[token("i256")]
    I256,
    #[token("u8")]
    U8,
    #[token("u16")]
    U16,
    #[token("u32")]
    U32,
    #[token("u64")]
    U64,
    #[token("u128")]
    U128,
    #[token("u256")]
    U256,
    #[token("f8")]
    F8,
    #[token("f16")]
    F16,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,

    // Identifiers (must come after keywords to avoid conflicts)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned(), priority = 1)]
    Identifier(String),

    // Numeric literals with optional type suffixes
    #[regex(
        r"-?[0-9]+(?:i8|i16|i32|i64|i128|i256|u8|u16|u32|u64|u128|u256)?",
        parse_integer
    )]
    Integer((i128, Option<String>)),

    #[regex(r"-?[0-9]+\.[0-9]+(?:f8|f16|f32|f64)?", parse_float)]
    Float((f64, Option<String>)),

    // String literals
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        unescape_string(&s[1..s.len()-1])
    })]
    String(String),

    // Template string literals (backticks)
    #[regex(r"`([^`\\]|\\.)*`", |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_owned()
    })]
    TemplateString(String),

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("=")]
    Assign,
    #[token("+=")]
    PlusAssign,
    #[token("-=")]
    MinusAssign,
    #[token("*=")]
    StarAssign,
    #[token("/=")]
    SlashAssign,
    #[token("%=")]
    PercentAssign,
    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("!")]
    Not,
    #[token("&")]
    Ampersand,
    #[token("::")]
    DoubleColon,

    // Delimiters
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token("->")]
    Arrow,
    #[token(".")]
    Dot,

    // Special
    #[regex(r"\r?\n")]
    Newline,

    // Comments (skip them)
    #[regex(r"//[^\n]*", logos::skip)]
    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    // Error token for unrecognized input
    Error,
}

/// Parse an integer literal with optional type suffix
fn parse_integer(lex: &mut LogosLexer<Token>) -> Option<(i128, Option<String>)> {
    let s = lex.slice();

    // Find where the suffix starts (if any)
    let suffix_start = s.find(|c: char| c.is_alphabetic());

    let (num_part, suffix_part) = if let Some(idx) = suffix_start {
        (&s[..idx], Some(s[idx..].to_owned()))
    } else {
        (s, None)
    };

    num_part
        .parse::<i128>()
        .ok()
        .map(|value| (value, suffix_part))
}

/// Parse a float literal with optional type suffix
fn parse_float(lex: &mut LogosLexer<Token>) -> Option<(f64, Option<String>)> {
    let s = lex.slice();

    // Find where the suffix starts (if any)
    let suffix_start = s.rfind('f');

    let (num_part, suffix_part) = if let Some(idx) = suffix_start {
        (&s[..idx], Some(s[idx..].to_owned()))
    } else {
        (s, None)
    };

    num_part
        .parse::<f64>()
        .ok()
        .map(|value| (value, suffix_part))
}

/// Unescape a string literal
fn unescape_string(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('0') => result.push('\0'),
                Some(c) => {
                    result.push('\\');
                    result.push(c);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }

    result
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Integer((value, suffix)) => {
                if let Some(s) = suffix {
                    write!(f, "Integer({}{s})", value)
                } else {
                    write!(f, "Integer({})", value)
                }
            }
            Token::Float((value, suffix)) => {
                if let Some(s) = suffix {
                    write!(f, "Float({}{s})", value)
                } else {
                    write!(f, "Float({})", value)
                }
            }
            Token::String(s) => write!(f, "String(\"{}\")", s),
            Token::TemplateString(s) => write!(f, "TemplateString(`{}`)", s),
            Token::Identifier(s) => write!(f, "Identifier({})", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// Position tracking for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new() -> Self {
        Position { line: 1, column: 1 }
    }

    pub fn advance(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }
}

/// A token with its position information
#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithPosition {
    pub token: Token,
    pub position: Position,
    pub span: logos::Span,
}

/// Lexer for the Yuni language
pub struct Lexer<'a> {
    inner: logos::Lexer<'a, Token>,
    position: Position,
    input: &'a str,
    last_end: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: Token::lexer(input),
            position: Position::new(),
            input,
            last_end: 0,
        }
    }

    /// Process template string interpolation
    pub fn process_template_string(&self, template: &str) -> Vec<TemplateStringPart> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                // Found interpolation start
                if !current.is_empty() {
                    parts.push(TemplateStringPart::Text(current.clone()));
                    current.clear();
                }

                chars.next(); // consume '{'
                let mut expr = String::new();
                let mut brace_count = 1;

                // Extract the expression inside ${}
                while let Some(ch) = chars.next() {
                    if ch == '{' {
                        brace_count += 1;
                        expr.push(ch);
                    } else if ch == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            break;
                        }
                        expr.push(ch);
                    } else {
                        expr.push(ch);
                    }
                }

                parts.push(TemplateStringPart::Interpolation(expr));
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            parts.push(TemplateStringPart::Text(current));
        }

        parts
    }
}

/// Parts of a template string
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateStringPart {
    Text(String),
    Interpolation(String),
}

impl<'a> Iterator for Lexer<'a> {
    type Item = TokenWithPosition;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(result) = self.inner.next() {
            let span = self.inner.span();

            // Update position for any skipped content since last token
            if span.start > self.last_end {
                for ch in self.input[self.last_end..span.start].chars() {
                    self.position.advance(ch);
                }
            }

            // Store the position at the start of the token
            let position = self.position;

            // Update position based on the consumed text
            for ch in self.input[span.start..span.end].chars() {
                self.position.advance(ch);
            }

            // Update last_end for next iteration
            self.last_end = span.end;

            // Handle the Result<Token, ()>
            match result {
                Ok(token) => {
                    return Some(TokenWithPosition {
                        token,
                        position,
                        span,
                    });
                }
                Err(_) => {
                    // Handle error tokens
                    return Some(TokenWithPosition {
                        token: Token::Error,
                        position,
                        span,
                    });
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let input = "package import fn let mut type struct enum if else for while return lives";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
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
            ]
        );
    }

    #[test]
    fn test_basic_types() {
        let input = "i8 i16 i32 i64 i128 i256 u8 u16 u32 u64 u128 u256 f8 f16 f32 f64";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
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
            ]
        );
    }

    #[test]
    fn test_identifiers() {
        let input = "myVariable MyClass camelCase PascalCase _underscore var123";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
                Token::Identifier("myVariable".to_string()),
                Token::Identifier("MyClass".to_string()),
                Token::Identifier("camelCase".to_string()),
                Token::Identifier("PascalCase".to_string()),
                Token::Identifier("_underscore".to_string()),
                Token::Identifier("var123".to_string()),
            ]
        );
    }

    #[test]
    fn test_numeric_literals_with_suffixes() {
        let input = "100 200i8 -42i32 1000u64 3.14 2.5f32 -1.0f64";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
                Token::Integer((100, None)),
                Token::Integer((200, Some("i8".to_string()))),
                Token::Integer((-42, Some("i32".to_string()))),
                Token::Integer((1000, Some("u64".to_string()))),
                Token::Float((3.14, None)),
                Token::Float((2.5, Some("f32".to_string()))),
                Token::Float((-1.0, Some("f64".to_string()))),
            ]
        );
    }

    #[test]
    fn test_string_literals() {
        let input = r#""Hello, World!" "Escaped \"quotes\"" "New\nLine""#;
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
                Token::String("Hello, World!".to_string()),
                Token::String("Escaped \"quotes\"".to_string()),
                Token::String("New\nLine".to_string()),
            ]
        );
    }

    #[test]
    fn test_template_strings() {
        let input = r#"`Hello, ${name}!` `Simple template`"#;
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
                Token::TemplateString("Hello, ${name}!".to_string()),
                Token::TemplateString("Simple template".to_string()),
            ]
        );
    }

    #[test]
    fn test_operators() {
        let input = "+ - * / % = += -= *= /= %= == != < > <= >= && || ! & :: ->";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
                Token::Plus,
                Token::Minus,
                Token::Star,
                Token::Slash,
                Token::Percent,
                Token::Assign,
                Token::PlusAssign,
                Token::MinusAssign,
                Token::StarAssign,
                Token::SlashAssign,
                Token::PercentAssign,
                Token::Equal,
                Token::NotEqual,
                Token::Less,
                Token::Greater,
                Token::LessEqual,
                Token::GreaterEqual,
                Token::And,
                Token::Or,
                Token::Not,
                Token::Ampersand,
                Token::DoubleColon,
                Token::Arrow,
            ]
        );
    }

    #[test]
    fn test_delimiters() {
        let input = "( ) { } [ ] , ; : .";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        assert_eq!(
            tokens,
            vec![
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
            ]
        );
    }

    #[test]
    fn test_comments() {
        let input = "fn main() { // This is a comment\n    let x = 42; /* Block comment */ }";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        // Comments should be skipped
        assert_eq!(
            tokens,
            vec![
                Token::Fn,
                Token::Identifier("main".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::LeftBrace,
                Token::Newline,
                Token::Let,
                Token::Identifier("x".to_string()),
                Token::Assign,
                Token::Integer((42, None)),
                Token::Semicolon,
                Token::RightBrace,
            ]
        );
    }

    #[test]
    fn test_position_tracking() {
        let input = "fn main() {\n    let x = 42;\n}";
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.collect();

        // Check first line positions
        assert_eq!(tokens[0].position, Position { line: 1, column: 1 }); // fn
        assert_eq!(tokens[1].position, Position { line: 1, column: 4 }); // main

        // Check second line positions (after newline)
        let newline_idx = tokens
            .iter()
            .position(|t| matches!(t.token, Token::Newline))
            .unwrap();
        assert_eq!(
            tokens[newline_idx + 1].position,
            Position { line: 2, column: 5 }
        ); // let
    }

    #[test]
    fn test_template_string_interpolation() {
        let lexer = Lexer::new("");

        let parts = lexer.process_template_string("Hello, ${name}!");
        assert_eq!(
            parts,
            vec![
                TemplateStringPart::Text("Hello, ".to_string()),
                TemplateStringPart::Interpolation("name".to_string()),
                TemplateStringPart::Text("!".to_string()),
            ]
        );

        let parts = lexer.process_template_string("${a} + ${b} = ${a + b}");
        assert_eq!(
            parts,
            vec![
                TemplateStringPart::Interpolation("a".to_string()),
                TemplateStringPart::Text(" + ".to_string()),
                TemplateStringPart::Interpolation("b".to_string()),
                TemplateStringPart::Text(" = ".to_string()),
                TemplateStringPart::Interpolation("a + b".to_string()),
            ]
        );
    }

    #[test]
    fn test_complete_example() {
        let input = r#"package main

import (
    "math"
)

type Point struct {
    x: f32,
    y: f32,
}

fn (p: &Point) Length(): f32 {
    math.sqrt((p.x * p.x) + (p.y * p.y))
}

fn main() {
    let mut p = Point{ x: 12.0f32, y: 16.0f32 };
    println(`Length: ${p.Length()}`);
}"#;

        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.map(|t| t.token).collect();

        // Verify it parses without errors
        assert!(tokens.iter().all(|t| !matches!(t, Token::Error)));

        // Check some key tokens
        assert_eq!(tokens[0], Token::Package);
        assert_eq!(tokens[1], Token::Identifier("main".to_string()));
        assert!(tokens.contains(&Token::Import));
        assert!(tokens.contains(&Token::Type));
        assert!(tokens.contains(&Token::Struct));
        assert!(tokens.contains(&Token::F32));
        assert!(tokens.contains(&Token::Mut));
    }
}
