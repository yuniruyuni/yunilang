//! 式の解析

use crate::ast::*;
use crate::lexer::Token;

use super::{ParseResult, Parser};

impl Parser {
    /// 式を解析（内部実装）
    pub(super) fn parse_expression_internal(&mut self) -> ParseResult<Expression> {
        self.parse_or_expression()
    }

    /// OR式を解析
    fn parse_or_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.match_token(&Token::OrOr) {
            let op = BinaryOp::Or;
            let right = self.parse_and_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// AND式を解析
    fn parse_and_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_or_expression()?;

        while self.match_token(&Token::AndAnd) {
            let op = BinaryOp::And;
            let right = self.parse_bitwise_or_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// ビット演算OR式を解析
    fn parse_bitwise_or_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_xor_expression()?;

        while self.match_token(&Token::Or) {
            let op = BinaryOp::BitOr;
            let right = self.parse_bitwise_xor_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// ビット演算XOR式を解析
    fn parse_bitwise_xor_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_and_expression()?;

        while self.match_token(&Token::Caret) {
            let op = BinaryOp::BitXor;
            let right = self.parse_bitwise_and_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// ビット演算AND式を解析
    fn parse_bitwise_and_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_equality_expression()?;

        while self.match_token(&Token::Ampersand) {
            let op = BinaryOp::BitAnd;
            let right = self.parse_equality_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// 等価式を解析
    fn parse_equality_expression(&mut self) -> ParseResult<Expression> {
        let start_pos = self.current_span().start;
        let mut left = self.parse_relational_expression()?;

        while let Some(op) = self.match_tokens(&[Token::EqEq, Token::NotEq]) {
            let op = match op {
                Token::EqEq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::Ne,
                _ => unreachable!(),
            };
            let right = self.parse_relational_expression()?;
            let span = self.span_from(start_pos);
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// 関係式を解析
    fn parse_relational_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_shift_expression()?;

        while let Some(op) = self.match_tokens(&[Token::Lt, Token::Gt, Token::LtEq, Token::GtEq]) {
            let op = match op {
                Token::Lt => BinaryOp::Lt,
                Token::Gt => BinaryOp::Gt,
                Token::LtEq => BinaryOp::Le,
                Token::GtEq => BinaryOp::Ge,
                _ => unreachable!(),
            };
            let right = self.parse_shift_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// シフト式を解析
    fn parse_shift_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_additive_expression()?;

        while let Some(op) = self.match_tokens(&[Token::LtLt, Token::GtGt]) {
            let op = match op {
                Token::LtLt => BinaryOp::Shl,
                Token::GtGt => BinaryOp::Shr,
                _ => unreachable!(),
            };
            let right = self.parse_additive_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// 加算式を解析
    fn parse_additive_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_multiplicative_expression()?;

        while let Some(op) = self.match_tokens(&[Token::Plus, Token::Minus]) {
            let op = match op {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// 乗算式を解析
    fn parse_multiplicative_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_cast_expression()?;

        while let Some(op) = self.match_tokens(&[Token::Star, Token::Slash, Token::Percent]) {
            let op = match op {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = self.parse_cast_expression()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }

    /// キャスト式を解析
    fn parse_cast_expression(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_unary_expression()?;

        if self.match_token(&Token::As) {
            let ty = self.parse_type()?;
            let span = Span::dummy(); // TODO: 適切なspan計算
            Ok(Expression::Cast(CastExpr {
                expr: Box::new(expr),
                ty,
                span,
            }))
        } else {
            Ok(expr)
        }
    }

    /// 単項式を解析
    fn parse_unary_expression(&mut self) -> ParseResult<Expression> {
        match self.current_token() {
            Some(Token::Bang) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span,
                }))
            }
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                    span,
                }))
            }
            Some(Token::Tilde) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::BitNot,
                    expr: Box::new(expr),
                    span,
                }))
            }
            Some(Token::Ampersand) => {
                self.advance();
                let is_mut = self.match_token(&Token::Mut);
                let expr = self.parse_unary_expression()?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                Ok(Expression::Reference(ReferenceExpr {
                    expr: Box::new(expr),
                    is_mut,
                    span,
                }))
            }
            Some(Token::Star) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                Ok(Expression::Dereference(DereferenceExpr {
                    expr: Box::new(expr),
                    span,
                }))
            }
            _ => self.parse_postfix_expression(),
        }
    }

    /// 後置式を解析
    fn parse_postfix_expression(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary_expression()?;

        loop {
            match self.current_token() {
                Some(Token::LeftBracket) => {
                    self.advance();
                    let index = self.parse_expression_internal()?;
                    self.expect(Token::RightBracket)?;
                    let span = Span::dummy(); // TODO: 適切なspan計算
                    expr = Expression::Index(IndexExpr {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    });
                }
                Some(Token::Dot) => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    let span = Span::dummy(); // TODO: 適切なspan計算
                    
                    // メソッド呼び出しかフィールドアクセスかを判定
                    if self.check(&Token::LeftParen) {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.expect(Token::RightParen)?;
                        expr = Expression::MethodCall(MethodCallExpr {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        });
                    } else {
                        expr = Expression::Field(FieldExpr {
                            object: Box::new(expr),
                            field,
                            span,
                        });
                    }
                }
                Some(Token::LeftParen) => {
                    self.advance();
                    let args = self.parse_arguments()?;
                    self.expect(Token::RightParen)?;
                    let span = Span::dummy(); // TODO: 適切なspan計算
                    expr = Expression::Call(CallExpr {
                        callee: Box::new(expr),
                        args,
                        span,
                    });
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// 引数リストを解析
    fn parse_arguments(&mut self) -> ParseResult<Vec<Expression>> {
        let mut args = Vec::new();

        while !self.check(&Token::RightParen) && !self.is_at_end() {
            args.push(self.parse_expression_internal()?);
            if !self.check(&Token::RightParen) {
                self.expect(Token::Comma)?;
            }
        }

        Ok(args)
    }

    /// プライマリ式を解析（リテラル、識別子、括弧付き式など）
    pub(super) fn parse_primary_expression(&mut self) -> ParseResult<Expression> {
        match self.current_token() {
            Some(Token::Integer(value)) => {
                let value = *value;
                let span = self.current_span();
                self.advance();

                // 型サフィックスをチェック
                let suffix = match self.current_token() {
                    Some(Token::Identifier(suffix)) if matches!(suffix.as_str(), "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128") => {
                        let suffix = suffix.clone();
                        self.advance();
                        Some(suffix)
                    },
                    Some(Token::I8) => { self.advance(); Some("i8".to_string()) },
                    Some(Token::I16) => { self.advance(); Some("i16".to_string()) },
                    Some(Token::I32) => { self.advance(); Some("i32".to_string()) },
                    Some(Token::I64) => { self.advance(); Some("i64".to_string()) },
                    Some(Token::I128) => { self.advance(); Some("i128".to_string()) },
                    Some(Token::U8) => { self.advance(); Some("u8".to_string()) },
                    Some(Token::U16) => { self.advance(); Some("u16".to_string()) },
                    Some(Token::U32) => { self.advance(); Some("u32".to_string()) },
                    Some(Token::U64) => { self.advance(); Some("u64".to_string()) },
                    Some(Token::U128) => { self.advance(); Some("u128".to_string()) },
                    _ => None,
                };

                Ok(Expression::Integer(IntegerLit { value, suffix, span: span.into() }))
            }
            Some(Token::Float(value)) => {
                let value = *value;
                let span = self.current_span();
                self.advance();

                // 型サフィックスをチェック
                let suffix = match self.current_token() {
                    Some(Token::Identifier(suffix)) if matches!(suffix.as_str(), "f32" | "f64") => {
                        let suffix = suffix.clone();
                        self.advance();
                        Some(suffix)
                    },
                    Some(Token::F32) => { self.advance(); Some("f32".to_string()) },
                    Some(Token::F64) => { self.advance(); Some("f64".to_string()) },
                    _ => None,
                };

                Ok(Expression::Float(FloatLit { value, suffix, span: span.into() }))
            }
            Some(Token::String(value)) => {
                let value = value.clone();
                let span = self.current_span();
                self.advance();
                Ok(Expression::String(StringLit { value, span: span.into() }))
            }
            Some(Token::TemplateString(value)) => {
                let value = value.clone();
                let span = self.current_span();
                self.advance();
                // TODO: テンプレート文字列の適切な解析
                Ok(Expression::String(StringLit { value, span: span.into() }))
            }
            Some(Token::True) => {
                let span = self.current_span();
                self.advance();
                Ok(Expression::Boolean(BooleanLit { value: true, span: span.into() }))
            }
            Some(Token::False) => {
                let span = self.current_span();
                self.advance();
                Ok(Expression::Boolean(BooleanLit { value: false, span: span.into() }))
            }
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                let span = self.current_span();
                self.advance();
                
                
                // パス（Enum::Variant など）を解析
                if self.check(&Token::ColonColon) {
                    return self.parse_path_expression(name, span.into());
                }
                
                // 構造体リテラルの場合
                // ただし、識別子が小文字で始まる場合（変数名）は構造体リテラルとして扱わない
                if self.check(&Token::LeftBrace) && name.chars().next().unwrap_or('a').is_uppercase() {
                    return self.parse_struct_literal(name);
                }
                
                Ok(Expression::Identifier(Identifier { name, span: span.into() }))
            }
            Some(Token::LeftParen) => {
                self.advance();
                
                // 空のタプル
                if self.check(&Token::RightParen) {
                    self.advance();
                    let span = Span::dummy(); // TODO: 適切なspan計算
                    return Ok(Expression::Tuple(TupleExpr { elements: vec![], span }));
                }
                
                let first = self.parse_expression_internal()?;
                
                // タプルの場合
                if self.match_token(&Token::Comma) {
                    let mut elements = vec![first];
                    
                    while !self.check(&Token::RightParen) && !self.is_at_end() {
                        elements.push(self.parse_expression_internal()?);
                        if !self.check(&Token::RightParen) {
                            self.expect(Token::Comma)?;
                        }
                    }
                    
                    self.expect(Token::RightParen)?;
                    let span = Span::dummy(); // TODO: 適切なspan計算
                    Ok(Expression::Tuple(TupleExpr { elements, span }))
                } else {
                    // 括弧付き式
                    self.expect(Token::RightParen)?;
                    Ok(first)
                }
            }
            Some(Token::LeftBracket) => {
                self.advance();
                let mut elements = Vec::new();
                
                while !self.check(&Token::RightBracket) && !self.is_at_end() {
                    elements.push(self.parse_expression_internal()?);
                    if !self.check(&Token::RightBracket) {
                        self.expect(Token::Comma)?;
                    }
                }
                
                self.expect(Token::RightBracket)?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                Ok(Expression::Array(ArrayExpr { elements, span }))
            }
            Some(Token::Match) => {
                self.parse_match_expression()
            }
            Some(Token::If) => {
                self.parse_if_expression()
            }
            Some(Token::LeftBrace) => {
                // ブロック式を解析
                let start = self.current_span().start;
                let (statements, last_expr) = self.parse_block_expression()?;
                let span = self.span_from(start);
                
                Ok(Expression::Block(BlockExpr {
                    statements,
                    last_expr,
                    span,
                }))
            }
            _ => Err(self.error("Expected expression".to_string())),
        }
    }

    /// 構造体リテラルを解析
    fn parse_struct_literal(&mut self, name: String) -> ParseResult<Expression> {
        self.expect(Token::LeftBrace)?;
        let mut fields = Vec::new();
        
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let field_name = self.expect_identifier()?;
            
            // フィールド名の後にコロンがない場合、これは構造体リテラルではない
            if !self.check(&Token::Colon) {
                return Err(self.error(format!("Expected ':' after field name '{}' in struct literal", field_name)));
            }
            self.expect(Token::Colon)?;
            let value = self.parse_expression_internal()?;
            
            fields.push(StructFieldInit {
                name: field_name,
                value,
            });
            
            if !self.check(&Token::RightBrace) {
                self.expect(Token::Comma)?;
            }
        }
        
        self.expect(Token::RightBrace)?;
        let span = Span::dummy(); // TODO: 適切なspan計算
        
        Ok(Expression::StructLit(StructLiteral {
            name,
            fields,
            span,
        }))
    }

    /// パス式を解析（Enum::Variant のような構文）
    fn parse_path_expression(&mut self, first_segment: String, start_span: Span) -> ParseResult<Expression> {
        let start = start_span.start;
        let mut segments = vec![first_segment];
        
        while self.match_token(&Token::ColonColon) {
            let segment = self.expect_identifier()?;
            segments.push(segment);
        }
        
        let span = self.span_from(start);
        
        // 2つのセグメントかつ構造体/タプル/ユニット構文がある場合、Enum Variantとして扱う
        if segments.len() == 2 {
            let enum_name = segments[0].clone();
            let variant_name = segments[1].clone();
            
            // 構造体ライクフィールド: Enum::Variant { field: value }
            if self.check(&Token::LeftBrace) {
                // 先読みして、これが本当に構造体スタイルのEnum variantかを確認
                // { identifier : ... } の形式かをチェック
                let is_struct_variant = self.peek_struct_variant_pattern();
                
                if is_struct_variant {
                    self.advance();
                    let mut fields = Vec::new();
                    
                    while !self.check(&Token::RightBrace) && !self.is_at_end() {
                        let field_name = self.expect_identifier()?;
                        self.expect(Token::Colon)?;
                        let value = self.parse_expression_internal()?;
                    
                    fields.push(StructFieldInit {
                        name: field_name,
                        value,
                    });
                    
                    if !self.check(&Token::RightBrace) {
                        self.expect(Token::Comma)?;
                    }
                    }
                    
                    self.expect(Token::RightBrace)?;
                    
                    return Ok(Expression::EnumVariant(EnumVariantExpr {
                        enum_name,
                        variant: variant_name,
                        fields: crate::ast::EnumVariantFields::Struct(fields),
                        span,
                    }));
                }
            }
            // タプルライクフィールド: Enum::Variant(args)
            else if self.check(&Token::LeftParen) {
                self.advance();
                let args = self.parse_arguments()?;
                self.expect(Token::RightParen)?;
                
                return Ok(Expression::EnumVariant(EnumVariantExpr {
                    enum_name,
                    variant: variant_name,
                    fields: crate::ast::EnumVariantFields::Tuple(args),
                    span,
                }));
            }
            // ユニットバリアント: Enum::Variant
            else {
                return Ok(Expression::EnumVariant(EnumVariantExpr {
                    enum_name,
                    variant: variant_name,
                    fields: crate::ast::EnumVariantFields::Unit,
                    span,
                }));
            }
        }
        
        // 通常のパス式として扱う
        Ok(Expression::Path(PathExpr { segments, span }))
    }

    /// if式を解析
    fn parse_if_expression(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        self.expect(Token::If)?;

        let condition = self.parse_expression_internal()?;
        
        // if式の条件式の後は必ずブロックが来るため、{を明示的にチェック
        if !self.check(&Token::LeftBrace) {
            return Err(self.error("Expected '{' after if condition".to_string()));
        }
        
        // ブロック式として解析
        let (then_statements, then_last_expr) = self.parse_block_expression()?;
        let then_span = self.span_from(start);

        let else_branch = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                // else if の場合、再帰的にif式を解析
                Some(Box::new(self.parse_if_expression()?))
            } else {
                // else ブロックの場合
                let else_start = self.current_span().start;
                let (else_statements, else_last_expr) = self.parse_block_expression()?;
                let else_span = self.span_from(else_start);
                
                Some(Box::new(Expression::Block(BlockExpr {
                    statements: else_statements,
                    last_expr: else_last_expr,
                    span: else_span,
                })))
            }
        } else {
            None
        };

        let span = self.span_from(start);
        
        // then_branchをBlock式として扱う
        let then_expr = Expression::Block(BlockExpr {
            statements: then_statements,
            last_expr: then_last_expr,
            span: then_span,
        });

        Ok(Expression::If(IfExpr {
            condition: Box::new(condition),
            then_branch: Box::new(then_expr),
            else_branch,
            span,
        }))
    }

    /// 構造体スタイルのEnum variantパターンかを先読みで確認
    fn peek_struct_variant_pattern(&self) -> bool {
        // { identifier : ... } のパターンかをチェック
        // 現在のトークンは { なので、次を見る
        if let Some(Token::Identifier(_)) = self.peek(1) {
            // identifier の次が : であれば構造体スタイル
            if let Some(Token::Colon) = self.peek(2) {
                return true;
            }
        }
        // { } の空の構造体リテラルもサポート
        if let Some(Token::RightBrace) = self.peek(1) {
            return true;
        }
        false
    }

    /// match式を解析
    fn parse_match_expression(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        self.expect(Token::Match)?;
        
        let expr = self.parse_expression_internal()?;
        self.expect(Token::LeftBrace)?;
        
        let mut arms = Vec::new();
        
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let pattern = self.parse_pattern(false)?; // false = not mutable pattern
            
            let guard = if self.match_token(&Token::If) {
                Some(self.parse_expression_internal()?)
            } else {
                None
            };
            
            self.expect(Token::FatArrow)?;
            
            // matchアームの値部分を解析
            let expr = if self.check(&Token::LeftBrace) {
                // ブロック式の場合
                let start = self.current_span().start;
                let (statements, last_expr) = self.parse_block_expression()?;
                let span = self.span_from(start);
                Expression::Block(BlockExpr {
                    statements,
                    last_expr,
                    span,
                })
            } else {
                // 通常の式
                self.parse_expression_internal()?
            };
            
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