// 単項演算式とキャスト式の解析
//
// 単項演算子（!、-、~、&、*）とas演算子によるキャスト式を解析する。


impl Parser {
    /// キャスト式を解析
    pub(crate) fn parse_cast_expression(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_unary_expression()?;
        let start = expr.span().start;

        if self.match_token(&Token::As) {
            let ty = self.parse_type()?;
            let span = self.span_from(start);
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
    pub(crate) fn parse_unary_expression(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        
        match self.current_token() {
            Some(Token::Bang) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::new(start, expr.span().end);
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    expr: Box::new(expr),
                    span,
                }))
            }
            Some(Token::Minus) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::new(start, expr.span().end);
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::Negate,
                    expr: Box::new(expr),
                    span,
                }))
            }
            Some(Token::Tilde) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::new(start, expr.span().end);
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
                let span = Span::new(start, expr.span().end);
                Ok(Expression::Reference(ReferenceExpr {
                    expr: Box::new(expr),
                    is_mut,
                    span,
                }))
            }
            Some(Token::Star) => {
                self.advance();
                let expr = self.parse_unary_expression()?;
                let span = Span::new(start, expr.span().end);
                Ok(Expression::Dereference(DereferenceExpr {
                    expr: Box::new(expr),
                    span,
                }))
            }
            _ => self.parse_postfix_expression(),
        }
    }
}