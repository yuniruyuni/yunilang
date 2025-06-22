//! Parser module for the Yuni language.
//!
//! This module is responsible for parsing tokens into an Abstract Syntax Tree (AST).
//! It uses a recursive descent approach with proper precedence handling.

use crate::ast::*;
use crate::lexer::{Token, TokenWithPosition};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ParseError {
    #[error(
        "Unexpected token: expected {expected}, found {found} at line {line}, column {column}"
    )]
    UnexpectedToken {
        expected: String,
        found: String,
        line: usize,
        column: usize,
    },

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Invalid syntax at line {line}, column {column}: {message}")]
    InvalidSyntax {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Invalid number literal at line {line}, column {column}: {message}")]
    InvalidNumber {
        line: usize,
        column: usize,
        message: String,
    },
}

pub type ParseResult<T> = Result<T, ParseError>;

/// Parser for the Yuni language
pub struct Parser {
    tokens: Vec<TokenWithPosition>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<TokenWithPosition>) -> Self {
        // Filter out newline tokens as they are not semantically significant
        let tokens: Vec<_> = tokens
            .into_iter()
            .filter(|t| !matches!(t.token, Token::Newline))
            .collect();
        Self { tokens, current: 0 }
    }

    /// Parse a complete program
    pub fn parse(&mut self) -> ParseResult<Program> {
        // Parse package declaration
        let package = self.parse_package_decl()?;

        // Parse imports (optional)
        let imports = self.parse_imports()?;

        // Parse top-level items
        let mut items = Vec::new();
        while !self.is_at_end() {
            items.push(self.parse_item()?);
        }

        let span = if let Some(first) = self.tokens.first() {
            if let Some(last) = self.tokens.last() {
                Span::new(first.span.start, last.span.end)
            } else {
                Span::new(first.span.start, first.span.end)
            }
        } else {
            Span::dummy()
        };

        Ok(Program {
            package,
            imports,
            items,
            span,
        })
    }

    /// Parse a single expression (for REPL)
    pub fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_expression_internal()
    }

    /// Parse a single statement (for REPL)
    pub fn parse_statement(&mut self) -> ParseResult<Statement> {
        self.parse_statement_internal()
    }

    /// Get the current token
    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.current).map(|t| &t.token)
    }

    /// Get the current token with position
    fn current_token_with_pos(&self) -> Option<&TokenWithPosition> {
        self.tokens.get(self.current)
    }

    /// Get a specific token ahead
    fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset).map(|t| &t.token)
    }

    /// Get the current span
    fn current_span(&self) -> logos::Span {
        self.current_token_with_pos()
            .map(|t| t.span.clone())
            .unwrap_or(logos::Span { start: 0, end: 0 })
    }

    /// Create a span from start position to current position
    fn span_from(&self, start: usize) -> Span {
        let end = self.current_span().end;
        Span::new(start, end)
    }

    /// Advance to the next token
    fn advance(&mut self) -> Option<&Token> {
        self.current += 1;
        self.current_token()
    }

    /// Check if we're at the end of input
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Check if the current token matches
    fn check(&self, token: &Token) -> bool {
        matches!(self.current_token(), Some(t) if t == token)
    }

    /// Check if the current token matches any of the given tokens
    fn _check_any(&self, tokens: &[Token]) -> bool {
        if let Some(current) = self.current_token() {
            tokens.iter().any(|t| t == current)
        } else {
            false
        }
    }

    /// Consume a token if it matches
    fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Consume any of the given tokens if one matches
    fn match_any(&mut self, tokens: &[Token]) -> Option<Token> {
        if let Some(current) = self.current_token() {
            for token in tokens {
                if token == current {
                    let matched = current.clone();
                    self.advance();
                    return Some(matched);
                }
            }
        }
        None
    }

    /// Expect a specific token
    fn expect(&mut self, expected: Token) -> ParseResult<()> {
        match self.current_token() {
            Some(token) if *token == expected => {
                self.advance();
                Ok(())
            }
            Some(token) => {
                let pos = self.current_token_with_pos().unwrap();
                Err(ParseError::UnexpectedToken {
                    expected: format!("{:?}", expected),
                    found: format!("{:?}", token),
                    line: pos.position.line,
                    column: pos.position.column,
                })
            }
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Expect an identifier and return its value
    fn expect_identifier(&mut self) -> ParseResult<String> {
        match self.current_token() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            Some(token) => {
                let pos = self.current_token_with_pos().unwrap();
                Err(ParseError::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: format!("{:?}", token),
                    line: pos.position.line,
                    column: pos.position.column,
                })
            }
            None => Err(ParseError::UnexpectedEof),
        }
    }

    /// Create an error for the current position
    fn error(&self, message: String) -> ParseError {
        if let Some(pos) = self.current_token_with_pos() {
            ParseError::InvalidSyntax {
                line: pos.position.line,
                column: pos.position.column,
                message,
            }
        } else {
            ParseError::UnexpectedEof
        }
    }

    /// Synchronize after an error by skipping to a likely recovery point
    fn _synchronize(&mut self) {
        while !self.is_at_end() {
            match self.current_token() {
                Some(Token::Semicolon) => {
                    self.advance();
                    break;
                }
                Some(Token::Fn) | Some(Token::Type) | Some(Token::Let) | Some(Token::If)
                | Some(Token::While) | Some(Token::For) | Some(Token::Return) => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    // ==================== Program Structure Parsing ====================

    /// Parse package declaration
    fn parse_package_decl(&mut self) -> ParseResult<PackageDecl> {
        let start = self.current_span().start;
        self.expect(Token::Package)?;
        let name = self.expect_identifier()?;
        let span = self.span_from(start);
        Ok(PackageDecl { name, span })
    }

    /// Parse import statements
    fn parse_imports(&mut self) -> ParseResult<Vec<Import>> {
        let mut imports = Vec::new();

        while self.match_token(&Token::Import) {
            let start = self.current_span().start;
            self.expect(Token::LeftParen)?;

            while !self.check(&Token::RightParen) && !self.is_at_end() {
                match self.current_token() {
                    Some(Token::String(path)) => {
                        let path = path.clone();
                        self.advance();

                        // Check for alias
                        let alias = if self.match_token(&Token::Identifier("as".to_string())) {
                            Some(self.expect_identifier()?)
                        } else {
                            None
                        };

                        let span = self.span_from(start);
                        imports.push(Import { path, alias, span });

                        // Allow optional comma at the end, or newline-separated imports
                        if !self.check(&Token::RightParen) {
                            // Comma is optional between imports on different lines
                            self.match_token(&Token::Comma);
                        }
                    }
                    _ => return Err(self.error("Expected import path string".to_string())),
                }
            }

            self.expect(Token::RightParen)?;
        }

        Ok(imports)
    }

    /// Parse a top-level item
    fn parse_item(&mut self) -> ParseResult<Item> {
        match self.current_token() {
            Some(Token::Type) => {
                let type_def = self.parse_type_def()?;
                Ok(Item::TypeDef(type_def))
            }
            Some(Token::Fn) => {
                // Check if this is a method by looking ahead
                if self.peek(1) == Some(&Token::LeftParen) {
                    // This might be a method
                    let saved_pos = self.current;
                    self.advance(); // skip 'fn'
                    self.advance(); // skip '('

                    // Check if we have a receiver
                    let is_method = match self.current_token() {
                        Some(Token::Identifier(_)) => {
                            // Check for : after identifier
                            self.advance();
                            self.check(&Token::Colon)
                        }
                        _ => false,
                    };

                    // Restore position
                    self.current = saved_pos;

                    if is_method {
                        let method = self.parse_method_decl()?;
                        Ok(Item::Method(method))
                    } else {
                        let func = self.parse_function_decl()?;
                        Ok(Item::Function(func))
                    }
                } else {
                    let func = self.parse_function_decl()?;
                    Ok(Item::Function(func))
                }
            }
            _ => Err(self.error("Expected type definition or function".to_string())),
        }
    }

    // ==================== Type Definition Parsing ====================

    /// Parse type definition (struct or enum)
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

    /// Parse struct body
    fn parse_struct_body(&mut self, name: String) -> ParseResult<StructDef> {
        let start = self.current_span().start;
        self.expect(Token::LeftBrace)?;

        let mut fields = Vec::new();
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let field_start = self.current_span().start;
            let field_name = self.expect_identifier()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;

            let field_span = self.span_from(field_start);
            fields.push(Field {
                name: field_name,
                ty,
                span: field_span,
            });

            // Fields can be separated by commas or semicolons
            if !self.check(&Token::RightBrace) && !self.match_token(&Token::Comma) {
                self.match_token(&Token::Semicolon);
            }
        }

        self.expect(Token::RightBrace)?;
        let span = self.span_from(start);

        Ok(StructDef { name, fields, span })
    }

    /// Parse enum body
    fn parse_enum_body(&mut self, name: String) -> ParseResult<EnumDef> {
        let start = self.current_span().start;
        self.expect(Token::LeftBrace)?;

        let mut variants = Vec::new();
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let variant_start = self.current_span().start;
            let variant_name = self.expect_identifier()?;

            let mut fields = Vec::new();
            if self.match_token(&Token::LeftParen) {
                // Tuple-style variant with fields
                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    let field_start = self.current_span().start;
                    let field_name = self.expect_identifier()?;
                    self.expect(Token::Colon)?;
                    let ty = self.parse_type()?;

                    let field_span = self.span_from(field_start);
                    fields.push(Field {
                        name: field_name,
                        ty,
                        span: field_span,
                    });

                    if !self.check(&Token::RightParen) {
                        self.expect(Token::Comma)?;
                    }
                }
                self.expect(Token::RightParen)?;
            }

            let variant_span = self.span_from(variant_start);
            variants.push(Variant {
                name: variant_name,
                fields,
                span: variant_span,
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

    // ==================== Function and Method Parsing ====================

    /// Parse function declaration
    fn parse_function_decl(&mut self) -> ParseResult<FunctionDecl> {
        let start = self.current_span().start;
        let is_public = false; // TODO: Handle visibility modifiers

        self.expect(Token::Fn)?;
        let name = self.expect_identifier()?;

        self.expect(Token::LeftParen)?;
        let params = self.parse_parameters()?;
        self.expect(Token::RightParen)?;

        // Parse return type (might be named)
        let return_type = if self.match_token(&Token::Colon) {
            // Check if this is a named return like (ret: Type)
            if self.check(&Token::LeftParen) {
                self.advance(); // consume '('
                let _name = self.expect_identifier()?; // consume the name
                self.expect(Token::Colon)?;
                let ty = self.parse_type()?;
                self.expect(Token::RightParen)?;
                Some(ty)
            } else {
                Some(self.parse_type()?)
            }
        } else {
            None
        };

        // Parse lives clause
        let lives_clause = if self.match_token(&Token::Lives) {
            Some(self.parse_lives_clause()?)
        } else {
            None
        };

        // Parse body
        let body = self.parse_block()?;
        let span = self.span_from(start);

        Ok(FunctionDecl {
            name,
            params,
            return_type,
            lives_clause,
            body,
            is_public,
            span,
        })
    }

    /// Parse method declaration
    fn parse_method_decl(&mut self) -> ParseResult<MethodDecl> {
        let start = self.current_span().start;
        let is_public = false; // TODO: Handle visibility modifiers

        self.expect(Token::Fn)?;
        self.expect(Token::LeftParen)?;

        // Parse receiver
        let receiver_start = self.current_span().start;
        let receiver_name = if matches!(self.peek(1), Some(Token::Colon)) {
            Some(self.expect_identifier()?)
        } else {
            None
        };

        if receiver_name.is_some() {
            self.expect(Token::Colon)?;
        }

        let receiver_ty = self.parse_type()?;
        let receiver_span = self.span_from(receiver_start);
        let receiver = Receiver {
            ty: receiver_ty,
            name: receiver_name,
            span: receiver_span,
        };

        self.expect(Token::RightParen)?;

        let name = self.expect_identifier()?;

        self.expect(Token::LeftParen)?;
        let params = self.parse_parameters()?;
        self.expect(Token::RightParen)?;

        // Parse return type (might be named)
        let return_type = if self.match_token(&Token::Colon) {
            // Check if this is a named return like (ret: Type)
            if self.check(&Token::LeftParen) {
                self.advance(); // consume '('
                let _name = self.expect_identifier()?; // consume the name
                self.expect(Token::Colon)?;
                let ty = self.parse_type()?;
                self.expect(Token::RightParen)?;
                Some(ty)
            } else {
                Some(self.parse_type()?)
            }
        } else {
            None
        };

        // Parse lives clause
        let lives_clause = if self.match_token(&Token::Lives) {
            Some(self.parse_lives_clause()?)
        } else {
            None
        };

        // Parse body
        let body = self.parse_block()?;
        let span = self.span_from(start);

        Ok(MethodDecl {
            receiver,
            name,
            params,
            return_type,
            lives_clause,
            body,
            is_public,
            span,
        })
    }

    /// Parse function parameters
    fn parse_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();

        while !self.check(&Token::RightParen) && !self.is_at_end() {
            let param_start = self.current_span().start;
            let name = self.expect_identifier()?;
            self.expect(Token::Colon)?;
            let ty = self.parse_type()?;

            let param_span = self.span_from(param_start);
            params.push(Parameter {
                name,
                ty,
                span: param_span,
            });

            if !self.check(&Token::RightParen) {
                self.expect(Token::Comma)?;
            }
        }

        Ok(params)
    }

    /// Parse lives clause
    fn parse_lives_clause(&mut self) -> ParseResult<LivesClause> {
        let start = self.current_span().start;
        let mut constraints = Vec::new();

        loop {
            let constraint_start = self.current_span().start;
            let target = self.expect_identifier()?;
            self.expect(Token::Assign)?;

            let mut sources = vec![self.expect_identifier()?];

            while self.match_token(&Token::Comma) {
                if self.check(&Token::LeftBrace) || self.is_at_end() {
                    break;
                }
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

    // ==================== Type Parsing ====================

    /// Parse a type
    fn parse_type(&mut self) -> ParseResult<Type> {
        match self.current_token() {
            // Basic types
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
            Some(Token::Identifier(name)) if name == "String" => {
                self.advance();
                Ok(Type::String)
            }
            Some(Token::Identifier(name)) if name == "void" => {
                self.advance();
                Ok(Type::Void)
            }

            // Reference types
            Some(Token::Ampersand) => {
                self.advance();
                let is_mut = self.match_token(&Token::Mut);
                let inner_type = self.parse_type()?;
                Ok(Type::Reference(Box::new(inner_type), is_mut))
            }

            // Array types
            Some(Token::LeftBracket) => {
                self.advance();
                let element_type = self.parse_type()?;
                self.expect(Token::RightBracket)?;
                Ok(Type::Array(Box::new(element_type)))
            }

            // Tuple types
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

            // User-defined types
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Ok(Type::UserDefined(name))
            }

            _ => Err(self.error("Expected type".to_string())),
        }
    }

    // ==================== Statement Parsing ====================

    /// Parse a statement
    fn parse_statement_internal(&mut self) -> ParseResult<Statement> {
        match self.current_token() {
            Some(Token::Let) => Ok(Statement::Let(self.parse_let_statement()?)),
            Some(Token::Return) => Ok(Statement::Return(self.parse_return_statement()?)),
            Some(Token::If) => Ok(Statement::If(self.parse_if_statement()?)),
            Some(Token::While) => Ok(Statement::While(self.parse_while_statement()?)),
            Some(Token::For) => Ok(Statement::For(self.parse_for_statement()?)),
            Some(Token::LeftBrace) => Ok(Statement::Block(self.parse_block()?)),
            _ => {
                // Try to parse as expression statement or assignment
                let expr = self.parse_expression_internal()?;

                // Check if this is an assignment
                if self.match_token(&Token::Assign) {
                    let value = self.parse_expression_internal()?;
                    let span = Span::new(0, 0); // TODO: Proper span calculation
                    self.expect(Token::Semicolon)?;
                    Ok(Statement::Assignment(AssignStatement {
                        target: expr,
                        value,
                        span,
                    }))
                } else {
                    self.expect(Token::Semicolon)?;
                    Ok(Statement::Expression(expr))
                }
            }
        }
    }

    /// Parse let statement
    fn parse_let_statement(&mut self) -> ParseResult<LetStatement> {
        let start = self.current_span().start;
        self.expect(Token::Let)?;

        let is_mut = self.match_token(&Token::Mut);
        let pattern = self.parse_pattern(is_mut)?;

        let ty = if self.match_token(&Token::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.match_token(&Token::Assign) {
            Some(self.parse_expression_internal()?)
        } else {
            None
        };

        self.expect(Token::Semicolon)?;
        let span = self.span_from(start);

        Ok(LetStatement {
            pattern,
            ty,
            init,
            span,
        })
    }

    /// Parse pattern for destructuring
    fn parse_pattern(&mut self, is_mut: bool) -> ParseResult<Pattern> {
        match self.current_token() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();

                // Check if this is a struct pattern
                if self.check(&Token::LeftBrace) {
                    self.advance();
                    let mut fields = Vec::new();

                    while !self.check(&Token::RightBrace) && !self.is_at_end() {
                        let field_name = self.expect_identifier()?;
                        self.expect(Token::Colon)?;
                        let field_pattern = self.parse_pattern(is_mut)?;
                        fields.push((field_name, field_pattern));

                        if !self.check(&Token::RightBrace) {
                            self.expect(Token::Comma)?;
                        }
                    }

                    self.expect(Token::RightBrace)?;
                    Ok(Pattern::Struct(name, fields))
                } else {
                    Ok(Pattern::Identifier(name, is_mut))
                }
            }
            Some(Token::LeftParen) => {
                self.advance();
                let mut patterns = Vec::new();

                while !self.check(&Token::RightParen) && !self.is_at_end() {
                    patterns.push(self.parse_pattern(is_mut)?);
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

    /// Parse return statement
    fn parse_return_statement(&mut self) -> ParseResult<ReturnStatement> {
        let start = self.current_span().start;
        self.expect(Token::Return)?;

        let value = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(self.parse_expression_internal()?)
        };

        self.expect(Token::Semicolon)?;
        let span = self.span_from(start);

        Ok(ReturnStatement { value, span })
    }

    /// Parse if statement
    fn parse_if_statement(&mut self) -> ParseResult<IfStatement> {
        let start = self.current_span().start;
        self.expect(Token::If)?;

        let condition = self.parse_expression_internal()?;
        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                // else if
                Some(ElseBranch::If(Box::new(self.parse_if_statement()?)))
            } else {
                // else block
                Some(ElseBranch::Block(self.parse_block()?))
            }
        } else {
            None
        };

        let span = self.span_from(start);
        Ok(IfStatement {
            condition,
            then_branch,
            else_branch,
            span,
        })
    }

    /// Parse while statement
    fn parse_while_statement(&mut self) -> ParseResult<WhileStatement> {
        let start = self.current_span().start;
        self.expect(Token::While)?;

        let condition = self.parse_expression_internal()?;
        let body = self.parse_block()?;
        let span = self.span_from(start);

        Ok(WhileStatement {
            condition,
            body,
            span,
        })
    }

    /// Parse for statement
    fn parse_for_statement(&mut self) -> ParseResult<ForStatement> {
        let start = self.current_span().start;
        self.expect(Token::For)?;
        self.expect(Token::LeftParen)?;

        // Parse init (optional)
        let init = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(Box::new(self.parse_statement_internal()?))
        };

        // Parse condition (optional)
        let condition = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(self.parse_expression_internal()?)
        };
        self.expect(Token::Semicolon)?;

        // Parse update (optional)
        let update = if self.check(&Token::RightParen) {
            None
        } else {
            Some(self.parse_expression_internal()?)
        };

        self.expect(Token::RightParen)?;
        let body = self.parse_block()?;
        let span = self.span_from(start);

        Ok(ForStatement {
            init,
            condition,
            update,
            body,
            span,
        })
    }

    /// Parse block
    fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.current_span().start;
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();
        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement_internal()?);
        }

        self.expect(Token::RightBrace)?;
        let span = self.span_from(start);

        Ok(Block { statements, span })
    }

    // ==================== Expression Parsing ====================

    /// Parse expression with precedence climbing
    fn parse_expression_internal(&mut self) -> ParseResult<Expression> {
        self.parse_assignment_expr()
    }

    /// Parse assignment expression (lowest precedence)
    fn parse_assignment_expr(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_or_expr()?;

        // Check for regular assignment
        if self.match_token(&Token::Assign) {
            let right = self.parse_assignment_expr()?; // Right-associative
            let span = Span::new(0, 0); // TODO: Proper span
            return Ok(Expression::Assignment(AssignmentExpr {
                target: Box::new(expr),
                value: Box::new(right),
                span,
            }));
        }

        // Check for compound assignment operators
        let assignment_ops = [
            (Token::PlusAssign, BinaryOp::AddAssign),
            (Token::MinusAssign, BinaryOp::SubtractAssign),
            (Token::StarAssign, BinaryOp::MultiplyAssign),
            (Token::SlashAssign, BinaryOp::DivideAssign),
            (Token::PercentAssign, BinaryOp::ModuloAssign),
        ];

        for (token, op) in &assignment_ops {
            if self.match_token(token) {
                let right = self.parse_expression_internal()?;
                let span = Span::new(0, 0); // TODO: Proper span
                return Ok(Expression::Binary(BinaryExpr {
                    left: Box::new(expr),
                    op: *op,
                    right: Box::new(right),
                    span,
                }));
            }
        }

        Ok(expr)
    }

    /// Parse logical OR expression
    fn parse_or_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_and_expr()?;

        while self.match_token(&Token::Or) {
            let right = self.parse_and_expr()?;
            let span = Span::new(0, 0); // TODO: Proper span
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse logical AND expression
    fn parse_and_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_equality_expr()?;

        while self.match_token(&Token::And) {
            let right = self.parse_equality_expr()?;
            let span = Span::new(0, 0); // TODO: Proper span
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse equality expression
    fn parse_equality_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_relational_expr()?;

        while let Some(op) = self.match_any(&[Token::Equal, Token::NotEqual]) {
            let right = self.parse_relational_expr()?;
            let span = Span::new(0, 0); // TODO: Proper span
            let binary_op = match op {
                Token::Equal => BinaryOp::Equal,
                Token::NotEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: binary_op,
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse relational expression
    fn parse_relational_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_additive_expr()?;

        while let Some(op) = self.match_any(&[
            Token::Less,
            Token::Greater,
            Token::LessEqual,
            Token::GreaterEqual,
        ]) {
            let right = self.parse_additive_expr()?;
            let span = Span::new(0, 0); // TODO: Proper span
            let binary_op = match op {
                Token::Less => BinaryOp::Less,
                Token::Greater => BinaryOp::Greater,
                Token::LessEqual => BinaryOp::LessEqual,
                Token::GreaterEqual => BinaryOp::GreaterEqual,
                _ => unreachable!(),
            };
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: binary_op,
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse additive expression
    fn parse_additive_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_multiplicative_expr()?;

        while let Some(op) = self.match_any(&[Token::Plus, Token::Minus]) {
            let right = self.parse_multiplicative_expr()?;
            let span = Span::new(0, 0); // TODO: Proper span
            let binary_op = match op {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: binary_op,
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse multiplicative expression
    fn parse_multiplicative_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_unary_expr()?;

        while let Some(op) = self.match_any(&[Token::Star, Token::Slash, Token::Percent]) {
            let right = self.parse_unary_expr()?;
            let span = Span::new(0, 0); // TODO: Proper span
            let binary_op = match op {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            expr = Expression::Binary(BinaryExpr {
                left: Box::new(expr),
                op: binary_op,
                right: Box::new(right),
                span,
            });
        }

        Ok(expr)
    }

    /// Parse unary expression
    fn parse_unary_expr(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;

        match self.current_token() {
            Some(Token::Not) => {
                self.advance();
                let operand = self.parse_unary_expr()?;
                let span = self.span_from(start);
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                }))
            }
            Some(Token::Minus) => {
                self.advance();
                let operand = self.parse_unary_expr()?;
                let span = self.span_from(start);
                Ok(Expression::Unary(UnaryExpr {
                    op: UnaryOp::Negate,
                    operand: Box::new(operand),
                    span,
                }))
            }
            Some(Token::Ampersand) => {
                self.advance();
                let is_mut = self.match_token(&Token::Mut);
                let expr = self.parse_unary_expr()?;
                let span = self.span_from(start);
                Ok(Expression::Reference(ReferenceExpr {
                    is_mut,
                    expr: Box::new(expr),
                    span,
                }))
            }
            Some(Token::Star) => {
                self.advance();
                let expr = self.parse_unary_expr()?;
                let span = self.span_from(start);
                Ok(Expression::Dereference(DereferenceExpr {
                    expr: Box::new(expr),
                    span,
                }))
            }
            _ => self.parse_postfix_expr(),
        }
    }

    /// Parse postfix expression
    fn parse_postfix_expr(&mut self) -> ParseResult<Expression> {
        let mut expr = self.parse_primary_expr()?;

        loop {
            let start = self.current_span().start;
            match self.current_token() {
                Some(Token::LeftParen) => {
                    // Function call
                    self.advance();
                    let mut args = Vec::new();

                    while !self.check(&Token::RightParen) && !self.is_at_end() {
                        args.push(self.parse_expression_internal()?);
                        if !self.check(&Token::RightParen) {
                            self.expect(Token::Comma)?;
                        }
                    }

                    self.expect(Token::RightParen)?;
                    let span = self.span_from(start);

                    expr = Expression::Call(CallExpr {
                        callee: Box::new(expr),
                        args,
                        span,
                    });
                }
                Some(Token::LeftBracket) => {
                    // Array index
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
                    // Field access or method call
                    self.advance();
                    let name = self.expect_identifier()?;

                    if self.check(&Token::LeftParen) {
                        // Method call
                        self.advance();
                        let mut args = Vec::new();

                        while !self.check(&Token::RightParen) && !self.is_at_end() {
                            args.push(self.parse_expression_internal()?);
                            if !self.check(&Token::RightParen) {
                                self.expect(Token::Comma)?;
                            }
                        }

                        self.expect(Token::RightParen)?;
                        let span = self.span_from(start);

                        expr = Expression::MethodCall(MethodCallExpr {
                            receiver: Box::new(expr),
                            method: name,
                            args,
                            span,
                        });
                    } else {
                        // Field access
                        let span = self.span_from(start);
                        expr = Expression::Field(FieldExpr {
                            object: Box::new(expr),
                            field: name,
                            span,
                        });
                    }
                }
                Some(Token::DoubleColon) => {
                    // Path expression (e.g., Enum::Variant)
                    if let Expression::Identifier(id) = expr {
                        let mut segments = vec![id.name];

                        while self.match_token(&Token::DoubleColon) {
                            segments.push(self.expect_identifier()?);
                        }

                        let span = self.span_from(start);
                        expr = Expression::Path(PathExpr { segments, span });
                    } else {
                        return Err(self.error("Invalid path expression".to_string()));
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    /// Parse primary expression
    fn parse_primary_expr(&mut self) -> ParseResult<Expression> {
        let start = self.current_span().start;

        match self.current_token() {
            // Literals
            Some(Token::Integer((value, suffix))) => {
                let value = *value as i64;
                let suffix = suffix.clone();
                self.advance();
                let span = self.span_from(start);
                Ok(Expression::Integer(IntegerLit {
                    value,
                    suffix,
                    span,
                }))
            }
            Some(Token::Float((value, suffix))) => {
                let value = *value;
                let suffix = suffix.clone();
                self.advance();
                let span = self.span_from(start);
                Ok(Expression::Float(FloatLit {
                    value,
                    suffix,
                    span,
                }))
            }
            Some(Token::String(value)) => {
                let value = value.clone();
                self.advance();
                let span = self.span_from(start);
                Ok(Expression::String(StringLit { value, span }))
            }
            Some(Token::TemplateString(value)) => {
                let value = value.clone();
                self.advance();
                let span = self.span_from(start);

                // Parse template string interpolation
                let lexer = crate::lexer::Lexer::new("");
                let parts = lexer
                    .process_template_string(&value)
                    .into_iter()
                    .map(|part| match part {
                        crate::lexer::TemplateStringPart::Text(text) => {
                            crate::ast::TemplateStringPart::Text(text)
                        }
                        crate::lexer::TemplateStringPart::Interpolation(expr_str) => {
                            // Parse the interpolated expression
                            let lexer = crate::lexer::Lexer::new(&expr_str);
                            let tokens: Vec<_> = lexer.collect();
                            let mut parser = Parser::new(tokens);
                            match parser.parse_expression() {
                                Ok(expr) => {
                                    crate::ast::TemplateStringPart::Interpolation(Box::new(expr))
                                }
                                Err(_) => crate::ast::TemplateStringPart::Text(format!(
                                    "${{{}}}",
                                    expr_str
                                )),
                            }
                        }
                    })
                    .collect();

                Ok(Expression::TemplateString(TemplateStringLit {
                    parts,
                    span,
                }))
            }
            Some(Token::Identifier(name)) if name == "true" => {
                self.advance();
                let span = self.span_from(start);
                Ok(Expression::Boolean(BooleanLit { value: true, span }))
            }
            Some(Token::Identifier(name)) if name == "false" => {
                self.advance();
                let span = self.span_from(start);
                Ok(Expression::Boolean(BooleanLit { value: false, span }))
            }

            // Identifiers
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();

                // Check for struct literal or enum variant
                if self.check(&Token::LeftBrace) {
                    // Struct literal
                    self.advance();
                    let mut fields = Vec::new();

                    while !self.check(&Token::RightBrace) && !self.is_at_end() {
                        let field_start = self.current_span().start;
                        let field_name = self.expect_identifier()?;

                        let value = if self.match_token(&Token::Colon) {
                            // Field with explicit value: field: expr
                            self.parse_expression_internal()?
                        } else {
                            // Field shorthand: field (same as field: field)
                            let field_span = self.span_from(field_start);
                            Expression::Identifier(Identifier {
                                name: field_name.clone(),
                                span: field_span,
                            })
                        };

                        let field_span = self.span_from(field_start);
                        fields.push(FieldInit {
                            name: field_name,
                            value,
                            span: field_span,
                        });

                        if !self.check(&Token::RightBrace) {
                            self.expect(Token::Comma)?;
                        }
                    }

                    self.expect(Token::RightBrace)?;
                    let span = self.span_from(start);

                    Ok(Expression::StructLit(StructLit {
                        ty: name,
                        fields,
                        span,
                    }))
                } else if self.check(&Token::DoubleColon) {
                    // Could be enum variant or path
                    self.advance();
                    let variant = self.expect_identifier()?;

                    if self.check(&Token::LeftParen) {
                        // Enum variant with arguments
                        self.advance();
                        let mut args = Vec::new();

                        while !self.check(&Token::RightParen) && !self.is_at_end() {
                            args.push(self.parse_expression_internal()?);
                            if !self.check(&Token::RightParen) {
                                self.expect(Token::Comma)?;
                            }
                        }

                        self.expect(Token::RightParen)?;
                        let span = self.span_from(start);

                        Ok(Expression::EnumVariant(EnumVariantExpr {
                            enum_name: name,
                            variant,
                            args,
                            span,
                        }))
                    } else {
                        // Simple enum variant or path
                        let span = self.span_from(start);
                        Ok(Expression::EnumVariant(EnumVariantExpr {
                            enum_name: name,
                            variant,
                            args: vec![],
                            span,
                        }))
                    }
                } else {
                    // Simple identifier
                    let span = self.span_from(start);
                    Ok(Expression::Identifier(Identifier { name, span }))
                }
            }

            // Grouped expression
            Some(Token::LeftParen) => {
                self.advance();

                // Check for empty tuple
                if self.check(&Token::RightParen) {
                    self.advance();
                    let span = self.span_from(start);
                    return Ok(Expression::Tuple(TupleExpr {
                        elements: vec![],
                        span,
                    }));
                }

                let first = self.parse_expression_internal()?;

                if self.check(&Token::Comma) {
                    // Tuple
                    let mut elements = vec![first];

                    while self.match_token(&Token::Comma) {
                        if self.check(&Token::RightParen) {
                            break;
                        }
                        elements.push(self.parse_expression_internal()?);
                    }

                    self.expect(Token::RightParen)?;
                    let span = self.span_from(start);

                    Ok(Expression::Tuple(TupleExpr { elements, span }))
                } else {
                    // Grouped expression
                    self.expect(Token::RightParen)?;
                    Ok(first)
                }
            }

            // Array literal
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
                let span = self.span_from(start);

                Ok(Expression::Array(ArrayExpr { elements, span }))
            }

            _ => Err(self.error("Expected expression".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_program(input: &str) -> ParseResult<Program> {
        let lexer = Lexer::new(input);
        let tokens: Vec<_> = lexer.collect();
        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    #[test]
    fn test_empty_program() {
        let input = "package main";
        let program = parse_program(input).unwrap();
        assert_eq!(program.package.name, "main");
        assert_eq!(program.imports.len(), 0);
        assert_eq!(program.items.len(), 0);
    }

    #[test]
    fn test_package_and_imports() {
        let input = r#"package main

import (
    "math"
    "fmt" as format
)"#;

        let program = parse_program(input).unwrap();
        assert_eq!(program.package.name, "main");
        assert_eq!(program.imports.len(), 2);
        assert_eq!(program.imports[0].path, "math");
        assert_eq!(program.imports[0].alias, None);
        assert_eq!(program.imports[1].path, "fmt");
        assert_eq!(program.imports[1].alias, Some("format".to_string()));
    }

    #[test]
    fn test_struct_definition() {
        let input = r#"package main

type Point struct {
    x: f32,
    y: f32
}"#;

        let program = parse_program(input).unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::TypeDef(TypeDef::Struct(s)) => {
                assert_eq!(s.name, "Point");
                assert_eq!(s.fields.len(), 2);
                assert_eq!(s.fields[0].name, "x");
                assert_eq!(s.fields[1].name, "y");
            }
            _ => panic!("Expected struct definition"),
        }
    }

    #[test]
    fn test_enum_definition() {
        let input = r#"package main

type State enum {
    Init,
    Running(count: i32),
    Finished
}"#;

        let program = parse_program(input).unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::TypeDef(TypeDef::Enum(e)) => {
                assert_eq!(e.name, "State");
                assert_eq!(e.variants.len(), 3);
                assert_eq!(e.variants[0].name, "Init");
                assert_eq!(e.variants[0].fields.len(), 0);
                assert_eq!(e.variants[1].name, "Running");
                assert_eq!(e.variants[1].fields.len(), 1);
                assert_eq!(e.variants[2].name, "Finished");
            }
            _ => panic!("Expected enum definition"),
        }
    }

    #[test]
    fn test_function_declaration() {
        let input = r#"package main

fn add(a: i32, b: i32): i32 {
    return a + b;
}"#;

        let program = parse_program(input).unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "a");
                assert_eq!(f.params[1].name, "b");
                assert!(f.return_type.is_some());
            }
            _ => panic!("Expected function declaration"),
        }
    }

    #[test]
    fn test_method_declaration() {
        let input = r#"package main

fn (p: &Point) Length(): f32 {
    return p.x;
}"#;

        let program = parse_program(input).unwrap();
        assert_eq!(program.items.len(), 1);

        match &program.items[0] {
            Item::Method(m) => {
                assert_eq!(m.name, "Length");
                assert_eq!(m.receiver.name, Some("p".to_string()));
                match &m.receiver.ty {
                    Type::Reference(inner, false) => match inner.as_ref() {
                        Type::UserDefined(name) => assert_eq!(name, "Point"),
                        _ => panic!("Expected user-defined type"),
                    },
                    _ => panic!("Expected reference type"),
                }
            }
            _ => panic!("Expected method declaration"),
        }
    }

    #[test]
    fn test_let_statement() {
        let input = r#"package main

fn main() {
    let x: i32 = 42;
    let mut y = 3.14;
    let (a, b) = (1, 2);
}"#;

        let program = parse_program(input).unwrap();
        match &program.items[0] {
            Item::Function(f) => {
                assert_eq!(f.body.statements.len(), 3);

                // Check first let statement
                match &f.body.statements[0] {
                    Statement::Let(let_stmt) => {
                        match &let_stmt.pattern {
                            Pattern::Identifier(name, is_mut) => {
                                assert_eq!(name, "x");
                                assert!(!is_mut);
                            }
                            _ => panic!("Expected identifier pattern"),
                        }
                        assert!(let_stmt.ty.is_some());
                        assert!(let_stmt.init.is_some());
                    }
                    _ => panic!("Expected let statement"),
                }

                // Check mutable let statement
                match &f.body.statements[1] {
                    Statement::Let(let_stmt) => match &let_stmt.pattern {
                        Pattern::Identifier(name, is_mut) => {
                            assert_eq!(name, "y");
                            assert!(is_mut);
                        }
                        _ => panic!("Expected identifier pattern"),
                    },
                    _ => panic!("Expected let statement"),
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_expressions() {
        let input = r#"package main

fn main() {
    let a = 1 + 2 * 3;
    let b = -a;
    let c = !true;
    let d = &mut a;
    let e = *d;
    let f = a.method(b, c);
    let g = array[0];
    let h = Point { x: 1.0, y: 2.0 };
    let i = State::Running(42);
}"#;

        let program = parse_program(input).unwrap();
        match &program.items[0] {
            Item::Function(f) => {
                assert_eq!(f.body.statements.len(), 9);
                // Basic validation that expressions were parsed
                for stmt in &f.body.statements {
                    match stmt {
                        Statement::Let(_) => {}
                        _ => panic!("Expected let statement"),
                    }
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_control_flow() {
        let input = r#"package main

fn main() {
    if x > 0 {
        println("positive");
    } else if x < 0 {
        println("negative");
    } else {
        println("zero");
    }
    
    while x > 0 {
        x = x - 1;
    }
    
    for (let i = 0; i < 10; i = i + 1) {
        println(i);
    }
}"#;

        let program = parse_program(input).unwrap();
        match &program.items[0] {
            Item::Function(f) => {
                assert_eq!(f.body.statements.len(), 3);

                // Check if statement
                match &f.body.statements[0] {
                    Statement::If(if_stmt) => {
                        assert!(if_stmt.else_branch.is_some());
                    }
                    _ => panic!("Expected if statement"),
                }

                // Check while statement
                match &f.body.statements[1] {
                    Statement::While(_) => {}
                    _ => panic!("Expected while statement"),
                }

                // Check for statement
                match &f.body.statements[2] {
                    Statement::For(for_stmt) => {
                        assert!(for_stmt.init.is_some());
                        assert!(for_stmt.condition.is_some());
                        assert!(for_stmt.update.is_some());
                    }
                    _ => panic!("Expected for statement"),
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_template_strings() {
        let input = r#"package main

fn main() {
    let name = "World";
    let msg = `Hello, ${name}!`;
}"#;

        let program = parse_program(input).unwrap();
        match &program.items[0] {
            Item::Function(f) => match &f.body.statements[1] {
                Statement::Let(let_stmt) => match &let_stmt.init {
                    Some(Expression::TemplateString(ts)) => {
                        assert_eq!(ts.parts.len(), 3);
                        match &ts.parts[0] {
                            crate::ast::TemplateStringPart::Text(t) => assert_eq!(t, "Hello, "),
                            _ => panic!("Expected text part"),
                        }
                        match &ts.parts[1] {
                            crate::ast::TemplateStringPart::Interpolation(_) => {}
                            _ => panic!("Expected interpolation"),
                        }
                        match &ts.parts[2] {
                            crate::ast::TemplateStringPart::Text(t) => assert_eq!(t, "!"),
                            _ => panic!("Expected text part"),
                        }
                    }
                    _ => panic!("Expected template string"),
                },
                _ => panic!("Expected let statement"),
            },
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_lives_clause() {
        let input = r#"package main

fn new(message: &String): (ret: Messenger)
lives
    ret = message
{
    return Messenger { message };
}"#;

        let program = parse_program(input).unwrap();
        match &program.items[0] {
            Item::Function(f) => {
                assert!(f.lives_clause.is_some());
                let lives = f.lives_clause.as_ref().unwrap();
                assert_eq!(lives.constraints.len(), 1);
                assert_eq!(lives.constraints[0].target, "ret");
                assert_eq!(lives.constraints[0].sources, vec!["message"]);
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_operator_precedence() {
        let input = r#"package main

fn main() {
    let a = 1 + 2 * 3;  // Should be 1 + (2 * 3) = 7
    let b = 1 * 2 + 3;  // Should be (1 * 2) + 3 = 5
    let c = 1 < 2 && 3 < 4;  // Should be (1 < 2) && (3 < 4)
    let d = a || b && c;  // Should be a || (b && c)
}"#;

        let program = parse_program(input).unwrap();
        match &program.items[0] {
            Item::Function(f) => {
                // Test first expression: 1 + 2 * 3
                match &f.body.statements[0] {
                    Statement::Let(let_stmt) => {
                        match &let_stmt.init {
                            Some(Expression::Binary(bin)) => {
                                assert_eq!(bin.op, BinaryOp::Add);
                                // Right side should be 2 * 3
                                match bin.right.as_ref() {
                                    Expression::Binary(inner) => {
                                        assert_eq!(inner.op, BinaryOp::Multiply);
                                    }
                                    _ => panic!("Expected binary expression"),
                                }
                            }
                            _ => panic!("Expected binary expression"),
                        }
                    }
                    _ => panic!("Expected let statement"),
                }
            }
            _ => panic!("Expected function"),
        }
    }

    #[test]
    fn test_error_recovery() {
        // Test various syntax errors
        let inputs = vec![
            "package",                             // Missing package name
            "package main\nfn main(",              // Incomplete function
            "package main\ntype Point struct",     // Missing struct body
            "package main\nfn main() { let x = }", // Missing expression
        ];

        for input in inputs {
            let lexer = Lexer::new(input);
            let tokens: Vec<_> = lexer.collect();
            let mut parser = Parser::new(tokens);
            assert!(parser.parse().is_err());
        }
    }
}
