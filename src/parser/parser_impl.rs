//! メインパーサー構造とユーティリティ

use crate::ast::*;
use crate::error::ParserError;
use crate::lexer::{Token, TokenWithPosition};

use super::{ParseError, ParseResult};

/// Yuniパーサー
pub struct Parser {
    pub(super) tokens: Vec<TokenWithPosition>,
    pub(super) current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<TokenWithPosition>) -> Self {
        // 改行トークンは意味を持たないのでフィルタリング
        let tokens: Vec<_> = tokens
            .into_iter()
            .filter(|t| !matches!(t.token, Token::Newline))
            .collect();
        Self { tokens, current: 0 }
    }

    /// 完全なプログラムを解析
    pub fn parse(&mut self) -> ParseResult<Program> {
        // パッケージ宣言を解析
        let package = self.parse_package_decl()?;

        // インポートを解析（オプション）
        let imports = self.parse_imports()?;

        // トップレベルアイテムを解析
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

    /// 単一の式を解析（REPL用）
    pub fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_expression_internal()
    }

    /// 単一の文を解析（REPL用）
    pub fn parse_statement(&mut self) -> ParseResult<Statement> {
        self.parse_statement_internal()
    }

    // ==================== ユーティリティメソッド ====================

    /// 現在のトークンを取得
    pub(super) fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.current).map(|t| &t.token)
    }

    /// 現在のトークンを位置情報付きで取得
    pub(super) fn current_token_with_pos(&self) -> Option<&TokenWithPosition> {
        self.tokens.get(self.current)
    }

    /// 特定のオフセット先のトークンを取得
    pub(super) fn peek(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.current + offset).map(|t| &t.token)
    }

    /// 現在のスパンを取得
    pub(super) fn current_span(&self) -> logos::Span {
        self.current_token_with_pos()
            .map(|t| t.span.clone())
            .unwrap_or(logos::Span { start: 0, end: 0 })
    }

    /// 開始位置から現在位置までのスパンを作成
    pub(super) fn span_from(&self, start: usize) -> Span {
        let end = if self.current > 0 {
            // 前のトークンの終了位置を使用
            self.tokens.get(self.current - 1)
                .map(|t| t.span.end)
                .unwrap_or(start)
        } else {
            self.current_span().end
        };
        Span::new(start, end)
    }

    /// 次のトークンに進む
    pub(super) fn advance(&mut self) {
        if !self.is_at_end() {
            self.current += 1;
        }
    }

    /// 終端に到達したかチェック
    pub(super) fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// 特定のトークンをチェック（進まない）
    pub(super) fn check(&self, token_type: &Token) -> bool {
        if let Some(token) = self.current_token() {
            std::mem::discriminant(token) == std::mem::discriminant(token_type)
        } else {
            false
        }
    }

    /// 特定のトークンにマッチしたら進む
    pub(super) fn match_token(&mut self, token_type: &Token) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// 複数のトークンタイプのいずれかにマッチしたら進む
    pub(super) fn match_tokens(&mut self, token_types: &[Token]) -> Option<Token> {
        for token_type in token_types {
            if self.check(token_type) {
                let token = self.current_token()?.clone();
                self.advance();
                return Some(token);
            }
        }
        None
    }

    /// 特定のトークンを期待
    pub(super) fn expect(&mut self, token_type: Token) -> ParseResult<()> {
        if self.check(&token_type) {
            self.advance();
            Ok(())
        } else {
            let found = self
                .current_token()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "EOF".to_string());
            Err(self.error(format!("Expected {:?}, found {}", token_type, found)))
        }
    }

    /// 識別子を期待
    pub(super) fn expect_identifier(&mut self) -> ParseResult<String> {
        match self.current_token() {
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error("Expected identifier".to_string())),
        }
    }

    /// 文字列リテラルを期待
    pub(super) fn expect_string(&mut self) -> ParseResult<String> {
        match self.current_token() {
            Some(Token::String(value)) => {
                let value = value.clone();
                self.advance();
                Ok(value)
            }
            _ => Err(self.error("Expected string literal".to_string())),
        }
    }

    /// エラーを作成
    pub(super) fn error(&self, message: String) -> ParseError {
        let span = self.current_span();
        ParserError::SyntaxError {
            message,
            span: span.into(),
        }
    }

    /// 予期しないトークンエラーを作成
    #[allow(dead_code)]
    pub(super) fn unexpected_token(&self) -> ParseError {
        let token = self
            .current_token()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| "EOF".to_string());
        self.error(format!("Unexpected token: {}", token))
    }
}