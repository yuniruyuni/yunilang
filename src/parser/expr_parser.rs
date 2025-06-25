//! 式の解析

// 式解析のサブモジュールをインクルード
include!("expr/binary_expr.rs");
include!("expr/unary_expr.rs");
include!("expr/postfix_expr.rs");
include!("expr/literal_expr.rs");
include!("expr/complex_expr.rs");
include!("expr/control_expr.rs");

use crate::ast::*;
use crate::lexer::Token;
use super::{ParseResult, Parser};

impl Parser {
    /// 式を解析（内部実装）
    pub(super) fn parse_expression_internal(&mut self) -> ParseResult<Expression> {
        self.parse_or_expression()
    }
}