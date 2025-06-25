// リテラル式と基本的な式の解析
//
// 整数、浮動小数点数、文字列、ブール値、配列、タプルなどのリテラル式を解析する。


impl Parser {
    /// プライマリ式を解析
    pub(crate) fn parse_primary_expression(&mut self) -> ParseResult<Expression> {
        match self.current_token() {
            Some(Token::Integer(value)) => self.parse_integer_literal(*value as i64),
            Some(Token::Float(value)) => self.parse_float_literal(*value),
            Some(Token::String(value)) => self.parse_string_literal(value.clone()),
            Some(Token::TemplateString(value)) => self.parse_template_string(value.clone()),
            Some(Token::True) => self.parse_boolean_literal(true),
            Some(Token::False) => self.parse_boolean_literal(false),
            Some(Token::Identifier(name)) => self.parse_identifier_expression(name.clone()),
            Some(Token::LeftParen) => self.parse_parenthesized_or_tuple(),
            Some(Token::LeftBracket) => self.parse_array_literal(),
            Some(Token::Match) => self.parse_match_expression(),
            Some(Token::If) => self.parse_if_expression(),
            Some(Token::LeftBrace) => self.parse_block_expression_primary(),
            _ => Err(self.error("Expected expression".to_string())),
        }
    }

    /// 整数リテラルを解析
    fn parse_integer_literal(&mut self, value: i64) -> ParseResult<Expression> {
        let span = self.current_span();
        self.advance();

        // 型サフィックスをチェック
        let suffix = self.parse_integer_suffix();

        Ok(Expression::Integer(IntegerLit { value: value as i128, suffix, span: span.into() }))
    }

    /// 整数型サフィックスを解析
    fn parse_integer_suffix(&mut self) -> Option<String> {
        match self.current_token() {
            Some(Token::Identifier(suffix)) 
                if matches!(suffix.as_str(), 
                    "i8" | "i16" | "i32" | "i64" | "i128" | 
                    "u8" | "u16" | "u32" | "u64" | "u128") => {
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
        }
    }

    /// 浮動小数点リテラルを解析
    fn parse_float_literal(&mut self, value: f64) -> ParseResult<Expression> {
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

    /// 文字列リテラルを解析
    fn parse_string_literal(&mut self, value: String) -> ParseResult<Expression> {
        let span = self.current_span();
        self.advance();
        Ok(Expression::String(StringLit { value, span: span.into() }))
    }

    /// ブール値リテラルを解析
    fn parse_boolean_literal(&mut self, value: bool) -> ParseResult<Expression> {
        let span = self.current_span();
        self.advance();
        Ok(Expression::Boolean(BooleanLit { value, span: span.into() }))
    }

    /// 識別子式を解析（パス、構造体リテラルを含む）
    fn parse_identifier_expression(&mut self, name: String) -> ParseResult<Expression> {
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

    /// 括弧付き式またはタプルを解析
    fn parse_parenthesized_or_tuple(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        self.advance();
        
        // 空のタプル
        if self.check(&Token::RightParen) {
            self.advance();
            let span = self.span_from(start);
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
            let span = self.span_from(start);
            Ok(Expression::Tuple(TupleExpr { elements, span }))
        } else {
            // 括弧付き式
            self.expect(Token::RightParen)?;
            Ok(first)
        }
    }

    /// 配列リテラルを解析
    fn parse_array_literal(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        self.advance();
        let mut elements = Vec::new();
        
        while !self.check(&Token::RightBracket) && !self.is_at_end() {
            elements.push(self.parse_expression_internal()?);
            if !self.check(&Token::RightBracket) {
                self.expect(Token::Comma)?;
            }
        }
        
        self.expect(Token::RightBracket)?;
        let span = self.span_from(start);
        Ok(Expression::Array(ArrayExpr { elements, span }))
    }

    /// ブロック式を解析（プライマリ式として）
    fn parse_block_expression_primary(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;
        let (statements, last_expr) = self.parse_block_expression()?;
        let span = self.span_from(start);
        
        Ok(Expression::Block(BlockExpr {
            statements,
            last_expr,
            span,
        }))
    }

    /// テンプレート文字列を解析
    pub(crate) fn parse_template_string(&mut self, value: String) -> ParseResult<Expression> {
        let span = self.current_span();
        self.advance();
        
        // テンプレート文字列をパース
        let mut parts = Vec::new();
        let mut current_text = String::new();
        let mut chars = value.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                // 補間式の開始
                chars.next(); // '{'をスキップ
                
                // 現在のテキストを保存
                if !current_text.is_empty() {
                    parts.push(TemplateStringPart::Text(current_text.clone()));
                    current_text.clear();
                }
                
                // 補間式を収集
                let mut expr_str = String::new();
                let mut brace_count = 1;
                
                while brace_count > 0 {
                    match chars.next() {
                        Some('{') => {
                            brace_count += 1;
                            expr_str.push('{');
                        }
                        Some('}') => {
                            brace_count -= 1;
                            if brace_count > 0 {
                                expr_str.push('}');
                            }
                        }
                        Some(c) => expr_str.push(c),
                        None => {
                            return Err(self.error("Unterminated interpolation in template string".to_string()));
                        }
                    }
                }
                
                // 補間式をパース
                let expr = self.parse_template_string_interpolation(&expr_str)?;
                parts.push(TemplateStringPart::Interpolation(expr));
            } else if ch == '\\' {
                // エスケープシーケンス
                match chars.next() {
                    Some('n') => current_text.push('\n'),
                    Some('r') => current_text.push('\r'),
                    Some('t') => current_text.push('\t'),
                    Some('\\') => current_text.push('\\'),
                    Some('`') => current_text.push('`'),
                    Some('$') => current_text.push('$'),
                    Some(c) => {
                        current_text.push('\\');
                        current_text.push(c);
                    }
                    None => current_text.push('\\'),
                }
            } else {
                current_text.push(ch);
            }
        }
        
        // 残りのテキストを保存
        if !current_text.is_empty() {
            parts.push(TemplateStringPart::Text(current_text));
        }
        
        Ok(Expression::TemplateString(TemplateStringLit { 
            parts, 
            span: span.into() 
        }))
    }
}