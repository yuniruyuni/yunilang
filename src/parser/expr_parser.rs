//! 式の解析
//!
//! このファイルは式解析のメインエントリーポイントを提供する。
//! 実際の解析処理は expr/ モジュール以下に分散されている。

use crate::ast::*;

use super::{ParseResult, Parser};

// expr モジュールから必要な機能をインポート
use super::expr::*;

impl Parser {
    /// 式を解析（内部実装）
    pub(super) fn parse_expression_internal(&mut self) -> ParseResult<Expression> {
        self.parse_or_expression()
    }
}