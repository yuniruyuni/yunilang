//! パターンの解析

use crate::ast::*;
use crate::lexer::Token;

use super::{ParseResult, Parser};

impl Parser {
    /// パターンを解析
    pub(super) fn parse_pattern(&mut self, is_mut: bool) -> ParseResult<Pattern> {
        match self.current_token() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                
                // ワイルドカードパターンのチェック
                if name == "_" {
                    return Ok(Pattern::Wildcard);
                }
                
                // パス（Enum::Variant など）をチェック
                if self.check(&Token::ColonColon) {
                    return self.parse_path_pattern(name);
                }
                
                // 構造体パターンかどうかチェック
                if self.check(&Token::LeftBrace) {
                    self.parse_struct_pattern(name)
                } else if self.check(&Token::LeftParen) {
                    // タプルライクパターン: Some(x), None(), Point(x, y) など
                    self.advance();
                    let mut patterns = Vec::new();
                    
                    while !self.check(&Token::RightParen) && !self.is_at_end() {
                        let pattern = self.parse_pattern(false)?;
                        patterns.push(pattern);
                        
                        if !self.check(&Token::RightParen) {
                            self.expect(Token::Comma)?;
                        }
                    }
                    
                    self.expect(Token::RightParen)?;
                    
                    // タプルライクなEnumバリアントパターンとして扱う
                    // 注：ここでは簡単のため、enum名を空文字列として扱う
                    // 実際のアプリケーションでは、型推論やインポートを考慮する必要がある
                    Ok(Pattern::EnumVariant {
                        enum_name: String::new(), // 型推論で解決される
                        variant: name,
                        fields: crate::ast::EnumVariantPatternFields::Tuple(patterns),
                    })
                } else {
                    Ok(Pattern::Identifier(name, is_mut))
                }
            }
            Some(Token::Integer(n)) => {
                let value = *n as i64;
                self.advance();
                Ok(Pattern::Literal(LiteralPattern::Integer(value)))
            }
            Some(Token::Float(f)) => {
                let value = *f;
                self.advance();
                Ok(Pattern::Literal(LiteralPattern::Float(value)))
            }
            Some(Token::String(s)) => {
                let value = s.clone();
                self.advance();
                Ok(Pattern::Literal(LiteralPattern::String(value)))
            }
            Some(Token::True) => {
                self.advance();
                Ok(Pattern::Literal(LiteralPattern::Bool(true)))
            }
            Some(Token::False) => {
                self.advance();
                Ok(Pattern::Literal(LiteralPattern::Bool(false)))
            }
            Some(Token::LeftParen) => {
                self.advance();
                let mut patterns = Vec::new();
                
                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    let pattern = self.parse_pattern(false)?;
                    patterns.push(pattern);
                    
                    if !self.check(&Token::RightParen) {
                        self.expect(Token::Comma)?;
                    }
                }
                
                self.expect(Token::RightParen)?;
                Ok(Pattern::Tuple(patterns))
            }
            _ => Err(self.error("Expected pattern".to_string())),
        }
    }

    /// 構造体パターンを解析
    fn parse_struct_pattern(&mut self, name: String) -> ParseResult<Pattern> {
        self.expect(Token::LeftBrace)?;
        let mut fields = Vec::new();
        
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let field_name = self.expect_identifier()?;
            
            let pattern = if self.match_token(&Token::Colon) {
                self.parse_pattern(false)?
            } else {
                // フィールド名と同じ名前の変数にバインド
                Pattern::Identifier(field_name.clone(), false)
            };
            
            fields.push((field_name, pattern));
            
            if !self.check(&Token::RightBrace) {
                self.expect(Token::Comma)?;
            }
        }
        
        self.expect(Token::RightBrace)?;
        Ok(Pattern::Struct(name, fields))
    }

    /// パスパターンを解析（Enum::Variant のような構文）
    fn parse_path_pattern(&mut self, first_segment: String) -> ParseResult<Pattern> {
        let mut segments = vec![first_segment];
        
        while self.match_token(&Token::ColonColon) {
            let segment = self.expect_identifier()?;
            segments.push(segment);
        }
        
        // 2つのセグメントの場合、Enum Variantパターンとして扱う
        if segments.len() == 2 {
            let enum_name = segments[0].clone();
            let variant_name = segments[1].clone();
            
            // 構造体ライクフィールド: Enum::Variant { field: pattern }
            if self.check(&Token::LeftBrace) {
                self.advance();
                let mut fields = Vec::new();
                
                while !self.check(&Token::RightBrace) && !self.is_at_end() {
                    let field_name = self.expect_identifier()?;
                    
                    let pattern = if self.match_token(&Token::Colon) {
                        self.parse_pattern(false)?
                    } else {
                        // フィールド名と同じ名前の変数にバインド
                        Pattern::Identifier(field_name.clone(), false)
                    };
                    
                    fields.push((field_name, pattern));
                    
                    if !self.check(&Token::RightBrace) {
                        self.expect(Token::Comma)?;
                    }
                }
                
                self.expect(Token::RightBrace)?;
                
                return Ok(Pattern::EnumVariant {
                    enum_name,
                    variant: variant_name,
                    fields: crate::ast::EnumVariantPatternFields::Struct(fields),
                });
            }
            // タプルライクフィールド: Enum::Variant(patterns)
            else if self.check(&Token::LeftParen) {
                self.advance();
                let mut patterns = Vec::new();
                
                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    let pattern = self.parse_pattern(false)?;
                    patterns.push(pattern);
                    
                    if !self.check(&Token::RightParen) {
                        self.expect(Token::Comma)?;
                    }
                }
                
                self.expect(Token::RightParen)?;
                
                return Ok(Pattern::EnumVariant {
                    enum_name,
                    variant: variant_name,
                    fields: crate::ast::EnumVariantPatternFields::Tuple(patterns),
                });
            }
            // ユニットバリアント: Enum::Variant
            else {
                return Ok(Pattern::EnumVariant {
                    enum_name,
                    variant: variant_name,
                    fields: crate::ast::EnumVariantPatternFields::Unit,
                });
            }
        }
        
        // 現時点では、複数セグメントのパスパターンはサポートしない
        Err(self.error("Unsupported path pattern".to_string()))
    }

}