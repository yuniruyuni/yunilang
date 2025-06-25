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