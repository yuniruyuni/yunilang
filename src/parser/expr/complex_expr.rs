// 複雑な式の解析
//
// 構造体リテラル、enumバリアント、パス式、if式を解析する。


impl Parser {
    /// 構造体リテラルを解析
    pub(crate) fn parse_struct_literal(&mut self, name: String) -> ParseResult<Expression> {
        let start = self.current_span().start - name.len();
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
        let span = self.span_from(start);
        
        Ok(Expression::StructLit(StructLiteral {
            name,
            fields,
            span,
        }))
    }

    /// パス式を解析（Enum::Variant のような構文）
    pub(crate) fn parse_path_expression(&mut self, first_segment: String, start_span: Span) -> ParseResult<Expression> {
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
    pub(crate) fn parse_if_expression(&mut self) -> ParseResult<Expression> {
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
    pub(crate) fn peek_struct_variant_pattern(&self) -> bool {
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
}