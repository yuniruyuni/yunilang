//! コンパイラのメイン処理モジュール
//!
//! このモジュールは、コンパイルパイプライン全体を管理し、
//! 複数のエラーを蓄積しながら処理を進める機能を提供します。

use crate::analyzer::{SemanticAnalyzer, monomorphize_program};
use crate::codegen::CodeGenerator;
use crate::error::{
    ErrorCollector, LexerError, YuniError, YuniResult,
};
use crate::lexer::{Lexer, Token};
use crate::parser::Parser;
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use inkwell::context::Context;
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
    #[allow(dead_code)]
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
                // エラートークンの位置から実際の文字を取得
                let error_text = self.state.source.get(token.span.clone())
                    .unwrap_or("不明")
                    .to_string();
                
                self.state.add_error(YuniError::Lexer(LexerError::UnrecognizedToken {
                    token: error_text,
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
    
    /// 単相化を実行
    pub fn monomorphize(&mut self, ast: crate::ast::Program) -> Option<crate::ast::Program> {
        if self.verbose {
            println!("ステップ: ジェネリクスの単相化を開始");
        }
        
        match monomorphize_program(ast) {
            Ok(monomorphized_ast) => Some(monomorphized_ast),
            Err(e) => {
                self.state.add_error(e);
                None
            }
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
    #[allow(dead_code)]
    pub fn run(&mut self) -> YuniResult<Option<CodeGenerator<'ctx>>> {
        // レキシカル解析
        let tokens = self.tokenize();
        
        // エラーがあってもパースは続行（より多くのエラーを検出するため）
        let ast = if !self.state.has_errors() {
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

        // 単相化を実行
        let monomorphized_ast = if let Some(ast) = ast {
            self.monomorphize(ast)
        } else {
            None
        };

        // コード生成
        if let Some(ast) = monomorphized_ast {
            let codegen = self.codegen(&ast)?;
            Ok(Some(codegen))
        } else {
            Ok(None)
        }
    }

    /// 状態への可変参照を取得
    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> &mut CompilationState {
        &mut self.state
    }
}