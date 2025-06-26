// 後置演算式の解析
//
// インデックスアクセス、フィールドアクセス、メソッド呼び出し、関数呼び出しを解析する。


impl Parser {
    /// 後置式を解析
    pub(crate) fn parse_postfix_expression(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary_expression()?;

        loop {
            let start = expr.span().start;
            
            match self.current_token() {
                Some(Token::LeftBracket) => {
                    self.advance();
                    let index = self.parse_expression_internal()?;
                    self.expect(Token::RightBracket)?;
                    let span = self.span_from(start);
                    expr = Expression::Index(IndexExpr {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    });
                }
                Some(Token::Dot) => {
                    self.advance();
                    let field = self.expect_identifier()?;
                    
                    // メソッド呼び出しかフィールドアクセスかを判定
                    if self.check(&Token::LeftParen) {
                        self.advance();
                        let args = self.parse_arguments()?;
                        self.expect(Token::RightParen)?;
                        let span = self.span_from(start);
                        expr = Expression::MethodCall(MethodCallExpr {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span,
                        });
                    } else {
                        let span = self.span_from(start);
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
                    let span = self.span_from(start);
                    expr = Expression::Call(CallExpr {
                        callee: Box::new(expr),
                        args,
                        span,
                        is_tail: false,
                    });
                }
                Some(Token::Lt) => {
                    // ジェネリック型引数の可能性をチェック
                    if let Expression::Identifier(id) = &expr {
                        if id.name.chars().next().unwrap_or('a').is_uppercase() {
                            // 型引数を解析
                            let type_args = self.parse_type_arguments()?;
                            
                            // 初期化式の種類を判別
                            if self.check(&Token::LeftBrace) {
                                // マップまたは構造体初期化子
                                expr = self.parse_initializer_expr(id.name.clone(), type_args)?;
                            } else if self.check(&Token::LeftBracket) {
                                // リスト初期化子
                                self.advance(); // '[' をスキップ
                                let mut elements = Vec::new();
                                
                                while !self.check(&Token::RightBracket) && !self.is_at_end() {
                                    elements.push(self.parse_expression_internal()?);
                                    if !self.check(&Token::RightBracket) {
                                        self.expect(Token::Comma)?;
                                    }
                                }
                                
                                self.expect(Token::RightBracket)?;
                                let span = self.span_from(start);
                                
                                expr = Expression::ListLiteral(ListLiteral {
                                    type_name: Some((id.name.clone(), type_args)),
                                    elements,
                                    span,
                                });
                            } else {
                                // TODO: ジェネリック型式として扱う（将来の実装）
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// 引数リストを解析
    pub(crate) fn parse_arguments(&mut self) -> ParseResult<Vec<Expression>> {
        let mut args = Vec::new();

        while !self.check(&Token::RightParen) && !self.is_at_end() {
            args.push(self.parse_expression_internal()?);
            if !self.check(&Token::RightParen) {
                self.expect(Token::Comma)?;
            }
        }

        Ok(args)
    }
}