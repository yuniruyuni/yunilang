//! 宣言（関数、構造体、列挙型）の解析

use crate::ast::*;
use crate::lexer::Token;

use super::{ParseResult, Parser};

impl Parser {
    /// アイテムを解析
    pub(super) fn parse_item(&mut self) -> ParseResult<Item> {
        // 可視性修飾子をチェック
        let is_public = if self.check(&Token::Pub) {
            self.advance();
            true
        } else {
            false
        };

        match self.current_token() {
            Some(Token::Type) => {
                if is_public {
                    return Err(self.error("Type definitions cannot have visibility modifiers".to_string()));
                }
                let type_def = self.parse_type_def()?;
                Ok(Item::TypeDef(type_def))
            }
            Some(Token::Struct) => {
                if is_public {
                    return Err(self.error("Struct definitions cannot have visibility modifiers".to_string()));
                }
                let struct_def = self.parse_struct_def()?;
                Ok(Item::TypeDef(TypeDef::Struct(struct_def)))
            }
            Some(Token::Enum) => {
                if is_public {
                    return Err(self.error("Enum definitions cannot have visibility modifiers".to_string()));
                }
                let enum_def = self.parse_enum_def()?;
                Ok(Item::TypeDef(TypeDef::Enum(enum_def)))
            }
            Some(Token::Fn) => {
                let func = self.parse_function_decl_with_visibility(is_public)?;
                Ok(Item::Function(func))
            }
            Some(Token::Impl) => {
                let method = self.parse_method_decl_with_visibility(is_public)?;
                Ok(Item::Method(method))
            }
            _ => Err(self.error("Expected item declaration".to_string())),
        }
    }

    /// 型定義を解析
    fn parse_type_def(&mut self) -> ParseResult<TypeDef> {
        self.expect(Token::Type)?;
        let name = self.expect_identifier()?;

        match self.current_token() {
            Some(Token::Struct) => {
                self.advance();
                let struct_def = self.parse_struct_body(name)?;
                Ok(TypeDef::Struct(struct_def))
            }
            Some(Token::Enum) => {
                self.advance();
                let enum_def = self.parse_enum_body(name)?;
                Ok(TypeDef::Enum(enum_def))
            }
            _ => Err(self.error("Expected 'struct' or 'enum' after type name".to_string())),
        }
    }

    /// 構造体本体を解析
    fn parse_struct_body(&mut self, name: String) -> ParseResult<StructDef> {
        let start = self.current_span().start;
        self.expect(Token::LeftBrace)?;

        let mut fields = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let field_name = self.expect_identifier()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            let field_span = self.current_span();

            fields.push(Field {
                name: field_name,
                ty,
                span: field_span.into(),
            });

            if !self.check(&Token::RightBrace) {
                self.expect(Token::Comma)?;
            }
        }

        self.expect(Token::RightBrace)?;
        let span = self.span_from(start);

        Ok(StructDef { name, fields, span })
    }

    /// 列挙型本体を解析
    fn parse_enum_body(&mut self, name: String) -> ParseResult<EnumDef> {
        let start = self.current_span().start;
        self.expect(Token::LeftBrace)?;

        let mut variants = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let variant_name = self.expect_identifier()?;
            let mut fields = Vec::new();

            // タプルライクフィールド: Variant(field1: Type1, field2: Type2)
            if self.match_token(&Token::LeftParen) {
                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    let field_name = self.expect_identifier()?;
                    self.expect(Token::Colon)?;
                    let ty = self.parse_type()?;
                    let field_span = self.current_span();

                    fields.push(Field {
                        name: field_name,
                        ty,
                        span: field_span.into(),
                    });

                    if !self.check(&Token::RightParen) {
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RightParen)?;
            }
            // 構造体ライクフィールド: Variant { field1: Type1, field2: Type2 }
            else if self.match_token(&Token::LeftBrace) {
                while !self.check(&Token::RightBrace) && !self.is_at_end() {
                    let field_name = self.expect_identifier()?;
                    self.expect(Token::Colon)?;
                    let ty = self.parse_type()?;
                    let field_span = self.current_span();

                    fields.push(Field {
                        name: field_name,
                        ty,
                        span: field_span.into(),
                    });

                    if !self.check(&Token::RightBrace) {
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RightBrace)?;
            }

            let variant_span = self.current_span();
            variants.push(Variant {
                name: variant_name,
                fields,
                span: variant_span.into(),
            });

            if !self.check(&Token::RightBrace) {
                self.expect(Token::Comma)?;
            }
        }

        self.expect(Token::RightBrace)?;
        let span = self.span_from(start);

        Ok(EnumDef {
            name,
            variants,
            span,
        })
    }

    /// 構造体定義を解析（`struct Name { ... }` 構文）
    fn parse_struct_def(&mut self) -> ParseResult<StructDef> {
        self.expect(Token::Struct)?;
        let name = self.expect_identifier()?;
        
        self.parse_struct_body(name)
    }

    /// 列挙型定義を解析（`enum Name { ... }` 構文）
    fn parse_enum_def(&mut self) -> ParseResult<EnumDef> {
        self.expect(Token::Enum)?;
        let name = self.expect_identifier()?;
        
        self.parse_enum_body(name)
    }

    /// 関数宣言を解析（可視性修飾子付き）
    fn parse_function_decl_with_visibility(&mut self, is_public: bool) -> ParseResult<FunctionDecl> {
        let start = self.current_span().start;

        self.expect(Token::Fn)?;
        let name = self.expect_identifier()?;

        // 型パラメータ（オプション）
        let type_params = if self.check(&Token::Lt) {
            self.parse_type_params()?
        } else {
            Vec::new()
        };

        // パラメータ
        self.expect(Token::LeftParen)?;
        let params = self.parse_parameters()?;
        self.expect(Token::RightParen)?;

        // 戻り値型
        let return_type = if self.match_token(&Token::Colon) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        // lives句（オプション）
        let lives_clause = if self.match_token(&Token::Lives) {
            Some(self.parse_lives_clause()?)
        } else {
            None
        };

        // 関数本体
        let body = self.parse_block()?;

        let span = self.span_from(start);

        Ok(FunctionDecl {
            is_public,
            name,
            type_params,
            params,
            return_type,
            lives_clause,
            body,
            span,
        })
    }

    /// メソッド宣言を解析（可視性修飾子付き）
    fn parse_method_decl_with_visibility(&mut self, is_public: bool) -> ParseResult<MethodDecl> {
        let start = self.current_span().start;

        self.expect(Token::Impl)?;
        self.expect(Token::Fn)?;
        let name = self.expect_identifier()?;

        // レシーバー
        self.expect(Token::LeftParen)?;
        let receiver = self.parse_receiver()?;

        // その他のパラメータ
        let params = if self.match_token(&Token::Comma) {
            self.parse_parameters()?
        } else {
            Vec::new()
        };
        self.expect(Token::RightParen)?;

        // 戻り値型
        let return_type = if self.match_token(&Token::Colon) {
            Some(Box::new(self.parse_type()?))
        } else {
            None
        };

        // lives句（オプション）
        let lives_clause = if self.match_token(&Token::Lives) {
            Some(self.parse_lives_clause()?)
        } else {
            None
        };

        // メソッド本体
        let body = self.parse_block()?;

        let span = self.span_from(start);

        Ok(MethodDecl {
            is_public,
            name,
            receiver,
            params,
            return_type,
            lives_clause,
            body,
            span,
        })
    }

    /// パラメータリストを解析
    pub(super) fn parse_parameters(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        while !self.check(&Token::RightParen) && !self.is_at_end() {
            let param_start = self.current_span().start;
            let name = self.expect_identifier()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;
            let span = self.span_from(param_start);

            params.push(Param { name, ty, span });

            if !self.check(&Token::RightParen) {
                self.expect(Token::Comma)?;
            }
        }

        Ok(params)
    }

    /// レシーバーを解析
    fn parse_receiver(&mut self) -> ParseResult<Receiver> {
        let start = self.current_span().start;
        
        // self または名前付きレシーバー
        let (name, is_mut) = if self.match_token(&Token::SelfValue) {
            (None, false)
        } else if self.match_token(&Token::Mut) {
            if self.match_token(&Token::SelfValue) {
                (None, true)
            } else {
                let name = self.expect_identifier()?;
                (Some(name), true)
            }
        } else {
            let name = self.expect_identifier()?;
            (Some(name), false)
        };

        self.expect(Token::Colon)?;
        let ty = self.parse_type()?;
        let span = self.span_from(start);

        Ok(Receiver {
            name,
            ty,
            is_mut,
            span,
        })
    }

    /// lives句を解析
    pub(super) fn parse_lives_clause(&mut self) -> ParseResult<LivesClause> {
        let start = self.current_span().start;
        let mut constraints = Vec::new();

        loop {
            let constraint_start = self.current_span().start;
            let target = self.expect_identifier()?;
            self.expect(Token::Colon)?;

            let mut sources = Vec::new();
            sources.push(self.expect_identifier()?);

            while self.match_token(&Token::Plus) {
                sources.push(self.expect_identifier()?);
            }

            let constraint_span = self.span_from(constraint_start);
            constraints.push(LivesConstraint {
                target,
                sources,
                span: constraint_span,
            });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        let span = self.span_from(start);
        Ok(LivesClause { constraints, span })
    }

    /// パッケージ宣言を解析
    pub(super) fn parse_package_decl(&mut self) -> ParseResult<PackageDecl> {
        let start = self.current_span().start;
        self.expect(Token::Package)?;
        let name = self.expect_identifier()?;
        // package宣言はセミコロンを要求しない
        
        let span = self.span_from(start);
        Ok(PackageDecl { name, span })
    }

    /// インポートを解析
    pub(super) fn parse_imports(&mut self) -> ParseResult<Vec<Import>> {
        let mut imports = Vec::new();

        while self.match_token(&Token::Import) {
            let start = self.current_span().start;
            let path = self.expect_string()?;
            
            let alias = if self.match_token(&Token::As) {
                Some(self.expect_identifier()?)
            } else {
                None
            };
            
            // import文はセミコロンを要求しない
            let span = self.span_from(start);
            
            imports.push(Import { path, alias, span });
        }

        Ok(imports)
    }
}