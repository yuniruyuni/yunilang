//! 統一的なエラーハンドリングモジュール
//!
//! このモジュールは、Yuniコンパイラ全体で使用される統一的なエラー型と
//! エラー報告システムを提供します。

use crate::ast::Span;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use thiserror::Error;

/// Yuniコンパイラの統一エラー型
#[derive(Error, Debug, Clone)]
pub enum YuniError {
    /// レキサーエラー
    #[error("字句解析エラー")]
    Lexer(#[from] LexerError),

    /// パーサーエラー
    #[error("構文解析エラー")]
    Parser(#[from] ParserError),

    /// セマンティック解析エラー
    #[error("意味解析エラー")]
    Analyzer(#[from] AnalyzerError),

    /// コード生成エラー
    #[error("コード生成エラー")]
    Codegen(#[from] CodegenError),

    /// ファイルI/Oエラー
    #[error("ファイル操作エラー: {0}")]
    Io(String),

    /// その他のエラー
    #[error("{0}")]
    Other(String),
}

/// レキサーエラーの詳細
#[derive(Error, Debug, Clone)]
#[allow(dead_code)]
pub enum LexerError {
    #[error("認識できないトークン: '{token}'")]
    UnrecognizedToken { token: String, span: Span },

    #[error("未終了の文字列リテラル")]
    UnterminatedString { span: Span },

    #[error("不正な数値リテラル: {message}")]
    InvalidNumber { message: String, span: Span },

    #[error("不正なエスケープシーケンス: '{sequence}'")]
    InvalidEscape { sequence: String, span: Span },
}

/// パーサーエラーの詳細
#[derive(Error, Debug, Clone)]
#[allow(dead_code)]
pub enum ParserError {
    #[error("予期しないトークン: {expected}を期待しましたが、{found}が見つかりました")]
    UnexpectedToken {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("予期しない入力の終了")]
    UnexpectedEof { expected: String, span: Span },

    #[error("不正な構文: {message}")]
    InvalidSyntax { message: String, span: Span },

    #[error("構文エラー: {message}")]
    SyntaxError { message: String, span: Span },
}

/// セマンティック解析エラーの詳細
#[derive(Error, Debug, Clone)]
#[allow(dead_code)]
pub enum AnalyzerError {
    #[error("未定義の変数: {name}")]
    UndefinedVariable { name: String, span: Span },

    #[error("未定義の型: {name}")]
    UndefinedType { name: String, span: Span },

    #[error("未定義の関数: {name}")]
    UndefinedFunction { name: String, span: Span },

    #[error("型の不一致: {expected}を期待しましたが、{found}が見つかりました")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("関数 {name} は既に定義されています")]
    DuplicateFunction { name: String, span: Span },

    #[error("型 {name} は既に定義されています")]
    DuplicateType { name: String, span: Span },

    #[error("変数 {name} は既にこのスコープで定義されています")]
    DuplicateVariable { name: String, span: Span },

    #[error("{name} の型を推論できません")]
    TypeInferenceError { name: String, span: Span },

    #[error("不正な操作: {message}")]
    InvalidOperation { message: String, span: Span },

    #[error("不変変数 {name} を変更することはできません")]
    ImmutableVariable { name: String, span: Span },

    #[error("関数 {name} にreturn文がありません")]
    MissingReturn { name: String, span: Span },

    #[error("ライフタイム制約違反: {message}")]
    LifetimeError { message: String, span: Span },

    #[error("パターンマッチが網羅的ではありません")]
    NonExhaustiveMatch { span: Span },

    #[error("移動された値 {name} を使用しようとしました")]
    UseAfterMove { name: String, span: Span },

    #[error("借用された値 {name} を移動しようとしました")]
    MoveWhileBorrowed { name: String, span: Span },
    
    #[error("複数の可変借用: {name}")]
    MultipleMutableBorrows { name: String, span: Span },
    
    #[error("不変借用中の可変借用: {name}")]
    MutableBorrowConflict { name: String, span: Span },
    
    #[error("引数の数が一致しません: {expected}個を期待しましたが、{found}個が見つかりました")]
    ArgumentCountMismatch { expected: usize, found: usize, span: Span },
    
    #[error("メソッド {method} が型 {ty} に見つかりません")]
    MethodNotFound { method: String, ty: String, span: Span },
    
    #[error("一時的な値の参照を取得することはできません")]
    TemporaryReference { span: Span },
}

/// コード生成エラーの詳細
#[derive(Error, Debug, Clone)]
#[allow(dead_code)]
pub enum CodegenError {
    #[error("LLVM初期化エラー: {message}")]
    LLVMInit { message: String },

    #[error("不正な型: {message}")]
    InvalidType { message: String, span: Span },

    #[error("未実装の機能: {feature}")]
    Unimplemented { feature: String, span: Span },

    #[error("内部エラー: {message}")]
    Internal { message: String },
    
    #[error("型エラー: 期待される型 {expected}, 実際の型 {actual}")]
    TypeError { expected: String, actual: String, span: Span },
    
    #[error("未定義: {name}")]
    Undefined { name: String, span: Span },
    
    #[error("コンパイル失敗: {message}")]
    CompilationFailed { message: String, span: Span },
}

/// エラー情報とソースコードの位置情報を含むエラー
#[derive(Debug, Clone)]
pub struct DiagnosticError {
    pub error: YuniError,
    pub file_id: usize,
}

impl DiagnosticError {
    pub fn new(error: YuniError, file_id: usize) -> Self {
        Self { error, file_id }
    }

    /// codespan-reportingのDiagnosticに変換
    pub fn to_diagnostic(&self) -> Diagnostic<usize> {
        let (message, labels) = match &self.error {
            YuniError::Lexer(e) => match e {
                LexerError::UnrecognizedToken { token, span } => (
                    format!("認識できないトークン: '{}'", token),
                    vec![Label::primary(self.file_id, span.start..span.end)
                        .with_message("ここに不正なトークンがあります")],
                ),
                LexerError::UnterminatedString { span } => (
                    "未終了の文字列リテラル".to_string(),
                    vec![Label::primary(self.file_id, span.start..span.end)
                        .with_message("文字列が閉じられていません")],
                ),
                LexerError::InvalidNumber { message, span } => (
                    format!("不正な数値リテラル: {}", message),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                LexerError::InvalidEscape { sequence, span } => (
                    format!("不正なエスケープシーケンス: '{}'", sequence),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
            },
            YuniError::Parser(e) => match e {
                ParserError::UnexpectedToken { expected, found, span } => (
                    format!("予期しないトークン: {}を期待しましたが、{}が見つかりました", expected, found),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                ParserError::UnexpectedEof { expected, span } => (
                    format!("予期しない入力の終了: {}を期待していました", expected),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                ParserError::InvalidSyntax { message, span } => (
                    format!("不正な構文: {}", message),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                ParserError::SyntaxError { message, span } => (
                    format!("構文エラー: {}", message),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
            },
            YuniError::Analyzer(e) => self.analyzer_error_to_diagnostic(e),
            YuniError::Codegen(e) => match e {
                CodegenError::LLVMInit { message } => (
                    format!("LLVM初期化エラー: {}", message),
                    vec![],
                ),
                CodegenError::InvalidType { message, span } => (
                    format!("不正な型: {}", message),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                CodegenError::Unimplemented { feature, span } => (
                    format!("未実装の機能: {}", feature),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                CodegenError::Internal { message } => (
                    format!("内部エラー: {}", message),
                    vec![],
                ),
                CodegenError::TypeError { expected, actual, span } => (
                    format!("型エラー: 期待される型 {}, 実際の型 {}", expected, actual),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                CodegenError::Undefined { name, span } => (
                    format!("未定義: {}", name),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
                CodegenError::CompilationFailed { message, span } => (
                    format!("コンパイル失敗: {}", message),
                    vec![Label::primary(self.file_id, span.start..span.end)],
                ),
            },
            YuniError::Io(message) => (
                format!("ファイル操作エラー: {}", message),
                vec![],
            ),
            YuniError::Other(message) => (
                message.clone(),
                vec![],
            ),
        };

        Diagnostic::error()
            .with_message(message)
            .with_labels(labels)
    }

    fn analyzer_error_to_diagnostic(&self, e: &AnalyzerError) -> (String, Vec<Label<usize>>) {
        match e {
            AnalyzerError::UndefinedVariable { name, span } => (
                format!("未定義の変数: {}", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この変数は定義されていません")],
            ),
            AnalyzerError::UndefinedType { name, span } => (
                format!("未定義の型: {}", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この型は定義されていません")],
            ),
            AnalyzerError::UndefinedFunction { name, span } => (
                format!("未定義の関数: {}", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この関数は定義されていません")],
            ),
            AnalyzerError::TypeMismatch { expected, found, span } => (
                format!("型の不一致: {}を期待しましたが、{}が見つかりました", expected, found),
                vec![Label::primary(self.file_id, span.start..span.end)],
            ),
            AnalyzerError::DuplicateFunction { name, span } => (
                format!("関数 {} は既に定義されています", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("重複した定義")],
            ),
            AnalyzerError::DuplicateType { name, span } => (
                format!("型 {} は既に定義されています", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("重複した定義")],
            ),
            AnalyzerError::DuplicateVariable { name, span } => (
                format!("変数 {} は既にこのスコープで定義されています", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("重複した定義")],
            ),
            AnalyzerError::TypeInferenceError { name, span } => (
                format!("{} の型を推論できません", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("型注釈を追加してください")],
            ),
            AnalyzerError::InvalidOperation { message, span } => (
                format!("不正な操作: {}", message),
                vec![Label::primary(self.file_id, span.start..span.end)],
            ),
            AnalyzerError::ImmutableVariable { name, span } => (
                format!("不変変数 {} を変更することはできません", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この変数はmutで宣言されていません")],
            ),
            AnalyzerError::MissingReturn { name, span } => (
                format!("関数 {} にreturn文がありません", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("戻り値を返す必要があります")],
            ),
            AnalyzerError::LifetimeError { message, span } => (
                format!("ライフタイム制約違反: {}", message),
                vec![Label::primary(self.file_id, span.start..span.end)],
            ),
            AnalyzerError::NonExhaustiveMatch { span } => (
                "パターンマッチが網羅的ではありません".to_string(),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("すべてのケースを処理する必要があります")],
            ),
            AnalyzerError::UseAfterMove { name, span } => (
                format!("移動された値 {} を使用しようとしました", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この値は既に移動されています")],
            ),
            AnalyzerError::MoveWhileBorrowed { name, span } => (
                format!("借用された値 {} を移動しようとしました", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この値は借用されています")],
            ),
            AnalyzerError::MultipleMutableBorrows { name, span } => (
                format!("複数の可変借用: {}", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この値は既に可変借用されています")],
            ),
            AnalyzerError::MutableBorrowConflict { name, span } => (
                format!("不変借用中の可変借用: {}", name),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("この値は不変借用されています")],
            ),
            AnalyzerError::ArgumentCountMismatch { expected, found, span } => (
                format!("引数の数が一致しません: {}個を期待しましたが、{}個が見つかりました", expected, found),
                vec![Label::primary(self.file_id, span.start..span.end)],
            ),
            AnalyzerError::MethodNotFound { method, ty, span } => (
                format!("メソッド {} が型 {} に見つかりません", method, ty),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("このメソッドは定義されていません")],
            ),
            AnalyzerError::TemporaryReference { span } => (
                "一時的な値の参照を取得することはできません".to_string(),
                vec![Label::primary(self.file_id, span.start..span.end)
                    .with_message("一時的な値への参照は無効です")],
            ),
        }
    }
}

/// 複数のエラーを蓄積するためのコレクター
#[derive(Debug, Default)]
pub struct ErrorCollector {
    errors: Vec<DiagnosticError>,
    warnings: Vec<DiagnosticError>,
}

impl ErrorCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// エラーを追加
    pub fn add_error(&mut self, error: YuniError, file_id: usize) {
        self.errors.push(DiagnosticError::new(error, file_id));
    }

    /// 警告を追加（将来の拡張用）
    #[allow(dead_code)]
    pub fn add_warning(&mut self, error: YuniError, file_id: usize) {
        self.warnings.push(DiagnosticError::new(error, file_id));
    }

    /// エラーがあるかどうか
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 警告があるかどうか
    #[allow(dead_code)]
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// エラーの数
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// 警告の数
    #[allow(dead_code)]
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// すべてのエラーを取得
    pub fn errors(&self) -> &[DiagnosticError] {
        &self.errors
    }

    /// すべての警告を取得
    pub fn warnings(&self) -> &[DiagnosticError] {
        &self.warnings
    }

    /// エラーと警告をクリア
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.errors.clear();
        self.warnings.clear();
    }

    /// 最初のエラーを取得
    #[allow(dead_code)]
    pub fn first_error(&self) -> Option<&DiagnosticError> {
        self.errors.first()
    }
}

/// Result型のエイリアス
pub type YuniResult<T> = Result<T, YuniError>;

/// エラー変換用のヘルパートレイト
pub trait IntoYuniError {
    fn into_yuni_error(self) -> YuniError;
}

impl IntoYuniError for std::io::Error {
    fn into_yuni_error(self) -> YuniError {
        YuniError::Io(self.to_string())
    }
}

impl IntoYuniError for anyhow::Error {
    fn into_yuni_error(self) -> YuniError {
        YuniError::Other(self.to_string())
    }
}

impl From<inkwell::builder::BuilderError> for YuniError {
    fn from(e: inkwell::builder::BuilderError) -> Self {
        YuniError::Codegen(CodegenError::Internal {
            message: format!("LLVM builder error: {:?}", e),
        })
    }
}

impl From<std::io::Error> for YuniError {
    fn from(e: std::io::Error) -> Self {
        YuniError::Io(e.to_string())
    }
}

/// エラーコンテキスト追加用のヘルパートレイト
pub trait WithContext<T> {
    fn with_context<F>(self, f: F) -> YuniResult<T>
    where
        F: FnOnce() -> String;
}

impl<T, E> WithContext<T> for Result<T, E>
where
    E: IntoYuniError,
{
    fn with_context<F>(self, f: F) -> YuniResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let base_error = e.into_yuni_error();
            match base_error {
                YuniError::Other(msg) => YuniError::Other(format!("{}: {}", f(), msg)),
                _ => base_error,
            }
        })
    }
}