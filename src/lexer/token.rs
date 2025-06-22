//! トークン定義

use logos::Logos;
use std::fmt;

/// Yuni言語のトークン型
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\f]+")] // 空白文字をスキップ（改行以外）
pub enum Token {
    // キーワード
    #[token("package")]
    Package,
    #[token("import")]
    Import,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("mut")]
    Mut,
    #[token("type")]
    Type,
    #[token("struct")]
    Struct,
    #[token("enum")]
    Enum,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("for")]
    For,
    #[token("while")]
    While,
    #[token("return")]
    Return,
    #[token("lives")]
    Lives,
    #[token("as")]
    As,
    #[token("impl")]
    Impl,
    #[token("self")]
    SelfValue,
    #[token("match")]
    Match,

    // 基本型
    #[token("i8")]
    I8,
    #[token("i16")]
    I16,
    #[token("i32")]
    I32,
    #[token("i64")]
    I64,
    #[token("i128")]
    I128,
    #[token("i256")]
    I256,
    #[token("u8")]
    U8,
    #[token("u16")]
    U16,
    #[token("u32")]
    U32,
    #[token("u64")]
    U64,
    #[token("u128")]
    U128,
    #[token("u256")]
    U256,
    #[token("f8")]
    F8,
    #[token("f16")]
    F16,
    #[token("f32")]
    F32,
    #[token("f64")]
    F64,

    // 真偽値リテラル
    #[token("true")]
    True,
    #[token("false")]
    False,

    // 識別子（キーワードの後に来る必要がある）
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned(), priority = 1)]
    Identifier(String),

    // 数値リテラル（型サフィックス付き）
    #[regex(
        r"-?[0-9]+",
        |lex| lex.slice().parse::<i128>().ok()
    )]
    Integer(i128),

    #[regex(r"-?[0-9]+\.[0-9]+", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    // 文字列リテラル
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        super::literal_parser::unescape_string(&s[1..s.len()-1])
    })]
    String(String),

    // テンプレート文字列リテラル（バッククォート）
    #[regex(r"`([^`\\]|\\.)*`", |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_owned()
    })]
    TemplateString(String),

    // 演算子
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("==")]
    EqEq,
    #[token("!=")]
    NotEq,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("<=")]
    LtEq,
    #[token(">=")]
    GtEq,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Bang,
    #[token("&")]
    Ampersand,
    #[token("|")]
    Or,
    #[token("^")]
    Caret,
    #[token("~")]
    Tilde,
    #[token("<<")]
    LtLt,
    #[token(">>")]
    GtGt,
    #[token("=")]
    Assign,
    #[token("=>")]
    FatArrow,

    // デリミタ
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("->")]
    Arrow,

    // 特殊トークン
    #[regex(r"\n")]
    Newline,

    // エラートークン
    Error,

    // コメント
    #[regex(r"//[^\n]*", logos::skip)]
    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    #[token("/*", |lex| {
        // ネストしたブロックコメントを処理
        let mut depth = 1;
        let mut chars = lex.remainder().chars();
        let mut prev = '\0';
        
        while depth > 0 {
            match chars.next() {
                Some('/') if prev == '*' => depth -= 1,
                Some('*') if prev == '/' => depth += 1,
                Some(c) => prev = c,
                None => break,
            }
            lex.bump(prev.len_utf8());
        }
        
        if depth == 0 {
            logos::Skip
        } else {
            // 未完了のコメントはエラー
            logos::Skip
        }
    })]
    _Comment,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Package => write!(f, "package"),
            Token::Import => write!(f, "import"),
            Token::Fn => write!(f, "fn"),
            Token::Let => write!(f, "let"),
            Token::Mut => write!(f, "mut"),
            Token::Type => write!(f, "type"),
            Token::Struct => write!(f, "struct"),
            Token::Enum => write!(f, "enum"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::For => write!(f, "for"),
            Token::While => write!(f, "while"),
            Token::Return => write!(f, "return"),
            Token::Lives => write!(f, "lives"),
            Token::As => write!(f, "as"),
            Token::Impl => write!(f, "impl"),
            Token::SelfValue => write!(f, "self"),
            Token::Match => write!(f, "match"),
            Token::I8 => write!(f, "i8"),
            Token::I16 => write!(f, "i16"),
            Token::I32 => write!(f, "i32"),
            Token::I64 => write!(f, "i64"),
            Token::I128 => write!(f, "i128"),
            Token::I256 => write!(f, "i256"),
            Token::U8 => write!(f, "u8"),
            Token::U16 => write!(f, "u16"),
            Token::U32 => write!(f, "u32"),
            Token::U64 => write!(f, "u64"),
            Token::U128 => write!(f, "u128"),
            Token::U256 => write!(f, "u256"),
            Token::F8 => write!(f, "f8"),
            Token::F16 => write!(f, "f16"),
            Token::F32 => write!(f, "f32"),
            Token::F64 => write!(f, "f64"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Integer(n) => write!(f, "{}", n),
            Token::Float(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::TemplateString(s) => write!(f, "`{}`", s),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::EqEq => write!(f, "=="),
            Token::NotEq => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Gt => write!(f, ">"),
            Token::LtEq => write!(f, "<="),
            Token::GtEq => write!(f, ">="),
            Token::AndAnd => write!(f, "&&"),
            Token::OrOr => write!(f, "||"),
            Token::Bang => write!(f, "!"),
            Token::Ampersand => write!(f, "&"),
            Token::Or => write!(f, "|"),
            Token::Caret => write!(f, "^"),
            Token::Tilde => write!(f, "~"),
            Token::LtLt => write!(f, "<<"),
            Token::GtGt => write!(f, ">>"),
            Token::Assign => write!(f, "="),
            Token::FatArrow => write!(f, "=>"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Colon => write!(f, ":"),
            Token::Dot => write!(f, "."),
            Token::Arrow => write!(f, "->"),
            Token::Newline => write!(f, "\\n"),
            Token::Error => write!(f, "error"),
            Token::_Comment => write!(f, "comment"),
        }
    }
}