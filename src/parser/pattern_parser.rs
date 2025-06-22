//! パターンの解析

use crate::ast::*;
use crate::error::{ParserError, YuniError};
use crate::lexer::Token;

use super::{ParseResult, Parser};

impl Parser {
    /// パターンを解析
    pub(super) fn parse_pattern(&mut self, is_mut: bool) -> ParseResult<Pattern> {
        match self.current_token() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                
                // 構造体パターンかどうかチェック
                if self.check(&Token::LeftBrace) {
                    self.parse_struct_pattern(name)
                } else {
                    Ok(Pattern::Identifier(name, is_mut))
                }
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

    /// match式を解析
    pub(super) fn parse_match_expression(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        self.expect(Token::Match)?;
        
        let expr = self.parse_expression_internal()?;
        self.expect(Token::LeftBrace)?;
        
        let mut arms = Vec::new();
        
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern(false)?;
            
            let guard = if self.match_token(&Token::If) {
                Some(self.parse_expression_internal()?)
            } else {
                None
            };
            
            self.expect(Token::FatArrow)?;
            let expr = self.parse_expression_internal()?;
            
            arms.push(MatchArm {
                pattern,
                guard,
                expr,
            });
            
            if !self.check(&Token::RightBrace) {
                self.expect(Token::Comma)?;
            }
        }
        
        self.expect(Token::RightBrace)?;
        let span = self.span_from(start);
        
        Ok(Expression::Match(MatchExpr {
            expr: Box::new(expr),
            arms,
            span,
        }))
    }
}