//! 文の解析

use crate::ast::*;
use crate::error::{ParserError, YuniError};
use crate::lexer::Token;

use super::{ParseResult, Parser};

impl Parser {
    /// 文を解析（内部実装）
    pub(super) fn parse_statement_internal(&mut self) -> ParseResult<Statement> {
        match self.current_token() {
            Some(Token::Let) => Ok(Statement::Let(self.parse_let_statement()?)),
            Some(Token::Return) => Ok(Statement::Return(self.parse_return_statement()?)),
            Some(Token::If) => Ok(Statement::If(self.parse_if_statement()?)),
            Some(Token::While) => Ok(Statement::While(self.parse_while_statement()?)),
            Some(Token::For) => Ok(Statement::For(self.parse_for_statement()?)),
            Some(Token::LeftBrace) => Ok(Statement::Block(self.parse_block()?)),
            _ => {
                // 式文または代入文として解析を試みる
                let expr = self.parse_expression_internal()?;

                // 代入かどうかチェック
                if self.match_token(&Token::Assign) {
                    let value = self.parse_expression_internal()?;
                    let span = Span::dummy(); // TODO: 適切なspan計算
                    self.expect(Token::Semicolon)?;
                    Ok(Statement::Assignment(AssignStatement {
                        target: expr,
                        value,
                        span,
                    }))
                } else {
                    // セミコロンがあるか、次がブロック終了なら省略可能
                    if self.check(&Token::Semicolon) {
                        self.advance();
                    } else if !self.check(&Token::RightBrace) {
                        return Err(self.error("Expected semicolon".to_string()));
                    }
                    Ok(Statement::Expression(expr))
                }
            }
        }
    }

    /// let文を解析
    pub(super) fn parse_let_statement(&mut self) -> ParseResult<LetStatement> {
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

    /// return文を解析
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

    /// if文を解析
    pub(super) fn parse_if_statement(&mut self) -> ParseResult<IfStatement> {
        let start = self.current_span().start;
        self.expect(Token::If)?;

        let condition = self.parse_expression_internal()?;
        // if文の条件式の後は必ずブロックが来るため、{を明示的にチェック
        if !self.check(&Token::LeftBrace) {
            return Err(self.error("Expected '{' after if condition".to_string()));
        }
        let then_branch = self.parse_block()?;

        let else_branch = if self.match_token(&Token::Else) {
            if self.check(&Token::If) {
                Some(ElseBranch::If(Box::new(self.parse_if_statement()?)))
            } else {
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

    /// while文を解析
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

    /// for文を解析
    fn parse_for_statement(&mut self) -> ParseResult<ForStatement> {
        let start = self.current_span().start;
        self.expect(Token::For)?;

        // 初期化部
        let init = if self.check(&Token::Semicolon) {
            None
        } else if self.check(&Token::Let) {
            Some(Statement::Let(self.parse_let_statement()?))
        } else {
            // 式文または代入文
            let expr = self.parse_expression_internal()?;
            if self.match_token(&Token::Assign) {
                let value = self.parse_expression_internal()?;
                let span = Span::dummy(); // TODO: 適切なspan計算
                self.expect(Token::Semicolon)?;
                Some(Statement::Assignment(AssignStatement {
                    target: expr,
                    value,
                    span,
                }))
            } else {
                self.expect(Token::Semicolon)?;
                Some(Statement::Expression(expr))
            }
        };

        if init.is_none() {
            self.expect(Token::Semicolon)?;
        }

        // 条件部
        let condition = if self.check(&Token::Semicolon) {
            None
        } else {
            Some(self.parse_expression_internal()?)
        };
        self.expect(Token::Semicolon)?;

        // 更新部
        let update = if self.check(&Token::LeftBrace) {
            None
        } else {
            Some(self.parse_expression_internal()?)
        };

        // 本体
        let body = self.parse_block()?;

        let span = self.span_from(start);

        Ok(ForStatement {
            init: init.map(Box::new),
            condition,
            update,
            body,
            span,
        })
    }

    /// ブロックを解析
    pub(super) fn parse_block(&mut self) -> ParseResult<Block> {
        let start = self.current_span().start;
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let stmt = self.parse_statement_internal()?;
            statements.push(stmt);
        }

        self.expect(Token::RightBrace)?;
        let span = self.span_from(start);

        Ok(Block { statements, span })
    }

    /// ブロック式を解析（最後の式を戻り値として扱う）
    pub(super) fn parse_block_expression(&mut self) -> ParseResult<(Vec<Statement>, Option<Box<Expression>>)> {
        self.expect(Token::LeftBrace)?;

        let mut statements = Vec::new();
        let mut last_expr = None;

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            // 式の可能性があるかチェック
            if self.is_expression_start() {
                // デバッグ: 現在のトークンを出力
                if let Some(token) = self.current_token() {
                    eprintln!("DEBUG: parse_block_expression - current token: {:?}", token);
                }
                let expr = self.parse_expression_internal()?;
                
                // セミコロンがあるか、次がブロック終了でないかチェック
                if self.check(&Token::Semicolon) {
                    self.advance();
                    statements.push(Statement::Expression(expr));
                } else if self.check(&Token::RightBrace) {
                    // ブロックの最後の式（セミコロンなし）は戻り値として扱う
                    last_expr = Some(Box::new(expr));
                } else {
                    // セミコロンが必要だがない場合はエラー
                    return Err(self.error("Expected semicolon or end of block".to_string()));
                }
            } else {
                // 文として解析
                let stmt = self.parse_statement_internal()?;
                statements.push(stmt);
            }
        }

        self.expect(Token::RightBrace)?;

        Ok((statements, last_expr))
    }

    /// 式の開始トークンかどうかをチェック
    fn is_expression_start(&self) -> bool {
        match self.current_token() {
            Some(Token::Let) | Some(Token::Return) | Some(Token::While) | Some(Token::For) => false,
            Some(Token::If) => true, // if式は式として扱える
            Some(Token::LeftBrace) => true, // ブロック式
            _ => true, // その他は式として扱う
        }
    }
}