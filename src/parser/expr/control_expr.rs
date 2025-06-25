// 制御フロー式の解析
//
// match式を解析する。


impl Parser {
    /// match式を解析
    pub(crate) fn parse_match_expression(&mut self) -> ParseResult<Expression> {
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