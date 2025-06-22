//! コンパイラのメイン処理モジュール
//!
//! このモジュールは、コンパイルパイプライン全体を管理し、
//! 複数のエラーを蓄積しながら処理を進める機能を提供します。

use crate::analyzer::SemanticAnalyzer;
use crate::ast::Span;
use crate::codegen::CodeGenerator;
use crate::error::{
    DiagnosticError, ErrorCollector, LexerError, YuniError, YuniResult,
};
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use inkwell::context::Context;
use inkwell::OptimizationLevel;
use std::fs;
use std::path::Path;

/// コンパイル状態を管理する構造体
pub struct CompilationState {
    pub source_file: String,
    pub source: String,
    pub files: SimpleFiles<String, String>,
    pub file_id: usize,
    pub error_collector: ErrorCollector,
}

impl CompilationState {
    /// 新しいコンパイル状態を作成
    pub fn new<P: AsRef<Path>>(source_file: P) -> YuniResult<Self> {
        let source_file_str = source_file.as_ref().display().to_string();
        let source = fs::read_to_string(source_file.as_ref())
            .map_err(|e| YuniError::Io(format!("Failed to read source file: {}", e)))?;

        let mut files = SimpleFiles::new();
        let file_id = files.add(source_file_str.clone(), source.clone());

        Ok(Self {
            source_file: source_file_str,
            source,
            files,
            file_id,
            error_collector: ErrorCollector::new(),
        })
    }
    
    /// 文字列からコンパイル状態を作成（テスト用）
    pub fn new_from_string(filename: &str, source: String) -> YuniResult<Self> {
        let mut files = SimpleFiles::new();
        let file_id = files.add(filename.to_string(), source.clone());
        
        Ok(Self {
            source_file: filename.to_string(),
            source,
            files,
            file_id,
            error_collector: ErrorCollector::new(),
        })
    }

    /// エラーを追加
    pub fn add_error(&mut self, error: YuniError) {
        self.error_collector.add_error(error, self.file_id);
    }

    /// 診断情報を報告
    pub fn report_diagnostics(&self) -> YuniResult<()> {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();

        // エラーを報告
        for error in self.error_collector.errors() {
            let diagnostic = error.to_diagnostic();
            codespan_reporting::term::emit(&mut writer.lock(), &config, &self.files, &diagnostic)
                .map_err(|e| YuniError::Io(format!("Failed to emit diagnostic: {}", e)))?;
        }

        // 警告を報告
        for warning in self.error_collector.warnings() {
            let diagnostic = warning.to_diagnostic();
            codespan_reporting::term::emit(&mut writer.lock(), &config, &self.files, &diagnostic)
                .map_err(|e| YuniError::Io(format!("Failed to emit diagnostic: {}", e)))?;
        }

        Ok(())
    }

    /// エラーがあるかチェック
    pub fn has_errors(&self) -> bool {
        self.error_collector.has_errors()
    }

    /// エラー数を取得
    pub fn error_count(&self) -> usize {
        self.error_collector.error_count()
    }
}

/// コンパイルパイプライン
pub struct CompilationPipeline<'ctx> {
    state: CompilationState,
    context: &'ctx Context,
    verbose: bool,
}

impl<'ctx> CompilationPipeline<'ctx> {
    /// 新しいコンパイルパイプラインを作成
    pub fn new(state: CompilationState, context: &'ctx Context, verbose: bool) -> Self {
        Self {
            state,
            context,
            verbose,
        }
    }
    
    /// コンパイル状態への参照を取得
    pub fn state(&self) -> &CompilationState {
        &self.state
    }

    /// レキシカル解析を実行
    pub fn tokenize(&mut self) -> Vec<crate::lexer::TokenWithPosition> {
        if self.verbose {
            println!("ステップ: レキシカル解析を開始");
        }

        let lexer = Lexer::new(&self.state.source);
        let tokens: Vec<_> = lexer.collect_tokens();

        // レキサーエラーをチェック
        for token in &tokens {
            if matches!(token.token, Token::Error) {
                self.state.add_error(YuniError::Lexer(LexerError::UnrecognizedToken {
                    token: "不明".to_string(),
                    span: token.span.clone().into(),
                }));
            }
        }

        tokens
    }

    /// 構文解析を実行
    pub fn parse(&mut self, tokens: Vec<crate::lexer::TokenWithPosition>) -> Option<crate::ast::Program> {
        if self.verbose {
            println!("ステップ: 構文解析を開始");
        }

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(program) => Some(program),
            Err(e) => {
                self.state.add_error(YuniError::Parser(e));
                None
            }
        }
    }

    /// セマンティック解析を実行
    pub fn analyze(&mut self, ast: &crate::ast::Program) -> bool {
        if self.verbose {
            println!("ステップ: セマンティック解析を開始");
        }

        let mut analyzer = SemanticAnalyzer::new();
        if let Err(e) = analyzer.analyze(ast) {
            self.state.add_error(YuniError::Analyzer(e));
            false
        } else {
            true
        }
    }

    /// コード生成を実行
    pub fn codegen(&mut self, ast: &crate::ast::Program) -> YuniResult<CodeGenerator<'ctx>> {
        if self.verbose {
            println!("ステップ: コード生成を開始");
        }

        let mut codegen = CodeGenerator::new(self.context, &self.state.source_file);
        codegen.compile_program(ast)?;
        Ok(codegen)
    }

    /// エラーレポートを生成
    pub fn report_errors(&self) -> YuniResult<()> {
        self.state.report_diagnostics()?;
        
        if self.state.has_errors() {
            let error_count = self.state.error_count();
            eprintln!(
                "\nコンパイルエラー: {} 個のエラーが見つかりました",
                error_count
            );
        }

        Ok(())
    }

    /// パイプライン全体を実行
    pub fn run(&mut self) -> YuniResult<Option<CodeGenerator<'ctx>>> {
        // レキシカル解析
        let tokens = self.tokenize();
        
        // エラーがあってもパースは続行（より多くのエラーを検出するため）
        let ast = if !self.state.has_errors() || true {
            self.parse(tokens)
        } else {
            None
        };

        // ASTが取得できた場合のみセマンティック解析を実行
        if let Some(ref ast) = ast {
            self.analyze(ast);
        }

        // エラーレポートを出力
        self.report_errors()?;

        // エラーがある場合はコード生成をスキップ
        if self.state.has_errors() {
            return Ok(None);
        }

        // コード生成
        if let Some(ast) = ast {
            let codegen = self.codegen(&ast)?;
            Ok(Some(codegen))
        } else {
            Ok(None)
        }
    }

    /// 状態への可変参照を取得
    pub fn state_mut(&mut self) -> &mut CompilationState {
        &mut self.state
    }
}