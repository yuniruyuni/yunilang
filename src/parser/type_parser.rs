//! 型の解析

use crate::ast::*;
use crate::lexer::Token;

use super::{ParseResult, Parser};

impl Parser {
    /// 型を解析
    pub(super) fn parse_type(&mut self) -> ParseResult<Type> {
        match self.current_token() {
            // 基本型
            Some(Token::I8) => {
                self.advance();
                Ok(Type::I8)
            }
            Some(Token::I16) => {
                self.advance();
                Ok(Type::I16)
            }
            Some(Token::I32) => {
                self.advance();
                Ok(Type::I32)
            }
            Some(Token::I64) => {
                self.advance();
                Ok(Type::I64)
            }
            Some(Token::I128) => {
                self.advance();
                Ok(Type::I128)
            }
            Some(Token::I256) => {
                self.advance();
                Ok(Type::I256)
            }
            Some(Token::U8) => {
                self.advance();
                Ok(Type::U8)
            }
            Some(Token::U16) => {
                self.advance();
                Ok(Type::U16)
            }
            Some(Token::U32) => {
                self.advance();
                Ok(Type::U32)
            }
            Some(Token::U64) => {
                self.advance();
                Ok(Type::U64)
            }
            Some(Token::U128) => {
                self.advance();
                Ok(Type::U128)
            }
            Some(Token::U256) => {
                self.advance();
                Ok(Type::U256)
            }
            Some(Token::F8) => {
                self.advance();
                Ok(Type::F8)
            }
            Some(Token::F16) => {
                self.advance();
                Ok(Type::F16)
            }
            Some(Token::F32) => {
                self.advance();
                Ok(Type::F32)
            }
            Some(Token::F64) => {
                self.advance();
                Ok(Type::F64)
            }
            Some(Token::Identifier(name)) if name == "bool" => {
                self.advance();
                Ok(Type::Bool)
            }
            Some(Token::Identifier(name)) if name == "str" => {
                self.advance();
                Ok(Type::Str)
            }
            Some(Token::Identifier(name)) if name == "String" => {
                self.advance();
                Ok(Type::String)
            }
            Some(Token::Identifier(name)) if name == "void" => {
                self.advance();
                Ok(Type::Void)
            }

            // 参照型
            Some(Token::Ampersand) => {
                self.advance();
                let is_mut = self.match_token(&Token::Mut);
                let inner_type = self.parse_type()?;
                Ok(Type::Reference(Box::new(inner_type), is_mut))
            }

            // 配列型
            Some(Token::LeftBracket) => {
                self.advance();
                let element_type = self.parse_type()?;
                self.expect(Token::RightBracket)?;
                Ok(Type::Array(Box::new(element_type)))
            }

            // タプル型
            Some(Token::LeftParen) => {
                self.advance();
                let mut types = Vec::new();

                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    types.push(self.parse_type()?);
                    if !self.check(&Token::RightParen) {
                        self.expect(Token::Comma)?;
                    }
                }

                self.expect(Token::RightParen)?;
                Ok(Type::Tuple(types))
            }

            // 関数型
            Some(Token::Fn) => {
                self.advance();
                self.expect(Token::LeftParen)?;
                
                let mut params = Vec::new();
                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    params.push(self.parse_type()?);
                    if !self.check(&Token::RightParen) {
                        self.expect(Token::Comma)?;
                    }
                }
                
                self.expect(Token::RightParen)?;
                
                let return_type = if self.match_token(&Token::Arrow) {
                    Box::new(self.parse_type()?)
                } else {
                    Box::new(Type::Void)
                };
                
                Ok(Type::Function(FunctionType {
                    params,
                    return_type,
                }))
            }

            // ユーザー定義型またはジェネリック型
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                
                // ジェネリック型の型引数をチェック
                if self.check(&Token::Lt) {
                    let type_args = self.parse_type_arguments()?;
                    Ok(Type::Generic(name, type_args))
                } else {
                    // 単一の大文字で始まる識別子は型変数として扱う
                    if name.len() == 1 && name.chars().next().unwrap().is_uppercase() {
                        Ok(Type::Variable(name))
                    } else {
                        Ok(Type::UserDefined(name))
                    }
                }
            }

            _ => Err(self.error("Expected type".to_string())),
        }
    }

    /// 型パラメータを解析
    pub(super) fn parse_type_params(&mut self) -> ParseResult<Vec<TypeParam>> {
        let mut params = Vec::new();
        self.expect(Token::Lt)?;

        while !self.check(&Token::Gt) && !self.is_at_end() {
            let name = self.expect_identifier()?;
            let span = self.current_span();
            
            params.push(TypeParam {
                name,
                span: span.into(),
            });

            if !self.check(&Token::Gt) {
                self.expect(Token::Comma)?;
            }
        }

        self.expect(Token::Gt)?;
        Ok(params)
    }
    
    /// 型引数を解析（例：<i32>, <T, U>）
    pub(super) fn parse_type_arguments(&mut self) -> ParseResult<Vec<Type>> {
        let mut args = Vec::new();
        self.expect(Token::Lt)?;

        while !self.check(&Token::Gt) && !self.is_at_end() {
            args.push(self.parse_type()?);

            if !self.check(&Token::Gt) {
                self.expect(Token::Comma)?;
            }
        }

        self.expect(Token::Gt)?;
        Ok(args)
    }
}