//! 二項演算式の解析
//!
//! 演算子の優先順位に従って二項演算式を解析する。

use crate::ast::*;
use crate::lexer::Token;
use crate::parser::{ParseResult, Parser};

impl Parser {
    /// OR式を解析
    pub(crate) fn parse_or_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.match_token(&Token::OrOr) {
            let op = BinaryOp::Or;
            let right = self.parse_and_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_and_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_or_expression()?;

        while self.match_token(&Token::AndAnd) {
            let op = BinaryOp::And;
            let right = self.parse_bitwise_or_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_bitwise_or_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_xor_expression()?;

        while self.match_token(&Token::Or) {
            let op = BinaryOp::BitOr;
            let right = self.parse_bitwise_xor_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_bitwise_xor_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_bitwise_and_expression()?;

        while self.match_token(&Token::Caret) {
            let op = BinaryOp::BitXor;
            let right = self.parse_bitwise_and_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_bitwise_and_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_equality_expression()?;

        while self.match_token(&Token::Ampersand) {
            let op = BinaryOp::BitAnd;
            let right = self.parse_equality_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_equality_expression(&mut self) -> ParseResult<Expression> {
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
    pub(crate) fn parse_relational_expression(&mut self) -> ParseResult<Expression> {
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
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_shift_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_additive_expression()?;

        while let Some(op) = self.match_tokens(&[Token::LtLt, Token::GtGt]) {
            let op = match op {
                Token::LtLt => BinaryOp::Shl,
                Token::GtGt => BinaryOp::Shr,
                _ => unreachable!(),
            };
            let right = self.parse_additive_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_additive_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_multiplicative_expression()?;

        while let Some(op) = self.match_tokens(&[Token::Plus, Token::Minus]) {
            let op = match op {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative_expression()?;
            let span = Span::new(left.span().start, right.span().end);
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
    pub(crate) fn parse_multiplicative_expression(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_cast_expression()?;

        while let Some(op) = self.match_tokens(&[Token::Star, Token::Slash, Token::Percent]) {
            let op = match op {
                Token::Star => BinaryOp::Multiply,
                Token::Slash => BinaryOp::Divide,
                Token::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = self.parse_cast_expression()?;
            let span = Span::new(left.span().start, right.span().end);
            left = Expression::Binary(BinaryExpr {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            });
        }

        Ok(left)
    }
}