use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use codespan_reporting::diagnostic::{Diagnostic, Label};
use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use colored::Colorize;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

mod analyzer;
mod ast;
mod codegen;
mod lexer;
mod parser;
mod runtime;

use crate::lexer::{Lexer, Token};
use crate::parser::Parser as YuniParser;

#[derive(Parser)]
#[command(name = "yunilang")]
#[command(author, version, about = "The Yuni language compiler", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum EmitType {
    /// Emit LLVM IR (.ll)
    #[value(name = "llvm-ir")]
    LlvmIr,
    /// Emit object file (.o)
    #[value(name = "obj")]
    Obj,
    /// Emit assembly (.s)
    #[value(name = "asm")]
    Asm,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile a Yuni source file
    Compile {
        /// The source file to compile
        input: PathBuf,

        /// Output file (defaults to input filename with appropriate extension)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// What to emit
        #[arg(long = "emit", value_enum, default_value = "obj")]
        emit: EmitType,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "2", value_parser = clap::value_parser!(u8).range(0..=3))]
        opt_level: u8,

        /// Dump the AST to stdout
        #[arg(long)]
        dump_ast: bool,

        /// Dump tokens to stdout
        #[arg(long)]
        dump_tokens: bool,

        /// Link with runtime to produce executable
        #[arg(long)]
        link: bool,
    },

    /// Run a Yuni source file
    Run {
        /// The source file to run
        input: PathBuf,

        /// Arguments to pass to the program
        args: Vec<String>,

        /// Optimization level (0-3)
        #[arg(short = 'O', long, default_value = "0", value_parser = clap::value_parser!(u8).range(0..=3))]
        opt_level: u8,
    },

    /// Start an interactive REPL
    Repl,

    /// Check a Yuni source file for errors without compiling
    Check {
        /// The source file to check
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    // Initialize logger before parsing CLI args
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = Cli::parse();

    // Set log level based on verbose flag
    if cli.verbose {
        log::set_max_level(log::LevelFilter::Debug);
    }

    let result = match cli.command {
        Commands::Compile {
            input,
            output,
            emit,
            opt_level,
            dump_ast,
            dump_tokens,
            link,
        } => compile(input, output, emit, opt_level, dump_ast, dump_tokens, link),
        Commands::Run {
            input,
            args,
            opt_level,
        } => run(input, args, opt_level),
        Commands::Repl => repl(),
        Commands::Check { input } => check(input),
    };

    if let Err(e) = result {
        eprintln!("{}: {}", "error".red().bold(), e);
        std::process::exit(1);
    }

    Ok(())
}

/// Compilation pipeline state
struct CompilationState {
    source_file: PathBuf,
    source: String,
    files: SimpleFiles<String, String>,
    file_id: usize,
}

impl CompilationState {
    fn new(source_file: PathBuf) -> Result<Self> {
        let source = fs::read_to_string(&source_file)
            .with_context(|| format!("Failed to read source file: {:?}", source_file))?;

        let mut files = SimpleFiles::new();
        let file_id = files.add(source_file.display().to_string(), source.clone());

        Ok(Self {
            source_file,
            source,
            files,
            file_id,
        })
    }

    fn report_error(&self, diagnostic: &Diagnostic<usize>) -> Result<()> {
        let writer = StandardStream::stderr(ColorChoice::Always);
        let config = codespan_reporting::term::Config::default();
        codespan_reporting::term::emit(&mut writer.lock(), &config, &self.files, diagnostic)?;
        Ok(())
    }
}

fn compile(
    input: PathBuf,
    output: Option<PathBuf>,
    emit: EmitType,
    opt_level: u8,
    dump_ast: bool,
    dump_tokens: bool,
    link: bool,
) -> Result<()> {
    log::info!(
        "Compiling {:?} with optimization level O{}",
        input,
        opt_level
    );

    // Initialize compilation state
    let state = CompilationState::new(input.clone())?;

    // 1. Tokenize
    log::debug!("Starting lexical analysis");
    let lexer = Lexer::new(&state.source);
    let tokens: Vec<_> = lexer.collect();

    if dump_tokens {
        println!("{}", "=== Tokens ===".blue().bold());
        for (i, token) in tokens.iter().enumerate() {
            println!("{:4}: {:?}", i, token);
        }
        println!();
    }

    // Check for lexer errors
    let lexer_errors: Vec<_> = tokens
        .iter()
        .filter(|t| matches!(t.token, Token::Error))
        .collect();

    if !lexer_errors.is_empty() {
        for error in lexer_errors {
            let diagnostic = Diagnostic::error()
                .with_message("Lexical error: unrecognized token")
                .with_labels(vec![Label::primary(
                    state.file_id,
                    error.span.start..error.span.end,
                )]);
            state.report_error(&diagnostic)?;
        }
        anyhow::bail!("Lexical analysis failed");
    }

    // 2. Parse
    log::debug!("Starting parsing");
    let mut parser = YuniParser::new(tokens);
    let ast = match parser.parse() {
        Ok(program) => program,
        Err(e) => {
            let diagnostic = Diagnostic::error()
                .with_message(format!("Parse error: {}", e))
                .with_labels(vec![Label::primary(state.file_id, 0..1)]);
            state.report_error(&diagnostic)?;
            anyhow::bail!("Parsing failed");
        }
    };

    if dump_ast {
        println!("{}", "=== AST ===".blue().bold());
        println!("{}", serde_json::to_string_pretty(&ast)?);
        println!();
    }

    // 3. Semantic analysis
    log::debug!("Starting semantic analysis");
    let mut analyzer = analyzer::SemanticAnalyzer::new();
    if let Err(e) = analyzer.analyze(&ast) {
        let diagnostic = Diagnostic::error()
            .with_message(format!("Semantic error: {}", e))
            .with_labels(vec![Label::primary(state.file_id, 0..1)]);
        state.report_error(&diagnostic)?;
        anyhow::bail!("Semantic analysis failed");
    }

    // 4. Code generation
    log::debug!("Starting code generation");
    let context = inkwell::context::Context::create();
    let mut codegen =
        codegen::CodeGenerator::new(&context, &state.source_file.display().to_string());

    // Compile the AST
    codegen.compile_program(&ast)?;

    // 5. Generate output
    let output_path = output.unwrap_or_else(|| {
        let mut path = input.clone();
        path.set_extension(match emit {
            EmitType::LlvmIr => "ll",
            EmitType::Obj => "o",
            EmitType::Asm => "s",
        });
        path
    });

    match emit {
        EmitType::LlvmIr => {
            log::info!("Writing LLVM IR to {:?}", output_path);
            codegen.write_llvm_ir(&output_path)?;
        }
        EmitType::Obj => {
            log::info!("Writing object file to {:?}", output_path);
            let opt = match opt_level {
                0 => inkwell::OptimizationLevel::None,
                1 => inkwell::OptimizationLevel::Less,
                2 => inkwell::OptimizationLevel::Default,
                3 => inkwell::OptimizationLevel::Aggressive,
                _ => unreachable!(),
            };
            codegen.write_object_file(&output_path, opt)?;
        }
        EmitType::Asm => {
            log::info!("Writing assembly to {:?}", output_path);
            // First write LLVM IR to a temp file
            let temp_ll = output_path.with_extension("ll");
            codegen.write_llvm_ir(&temp_ll)?;

            // Use llc to convert to assembly
            let status = Command::new("llc")
                .arg("-O")
                .arg(opt_level.to_string())
                .arg("-o")
                .arg(&output_path)
                .arg(&temp_ll)
                .status()
                .context("Failed to run llc")?;

            if !status.success() {
                anyhow::bail!("llc failed with status: {}", status);
            }

            // Clean up temp file
            fs::remove_file(temp_ll).ok();
        }
    }

    // 6. Link if requested
    if link && matches!(emit, EmitType::Obj) {
        let exe_path = output_path.with_extension("");
        log::info!("Linking executable to {:?}", exe_path);

        // Link with runtime
        let runtime_path = Path::new("runtime.ll");
        let runtime_obj = runtime_path.with_extension("o");

        // Compile runtime to object file if needed
        if !runtime_obj.exists()
            || runtime_path.metadata()?.modified()? > runtime_obj.metadata()?.modified()?
        {
            log::debug!("Compiling runtime.ll");
            let status = Command::new("llc")
                .arg("-filetype=obj")
                .arg("-o")
                .arg(&runtime_obj)
                .arg(runtime_path)
                .status()
                .context("Failed to compile runtime")?;

            if !status.success() {
                anyhow::bail!("Failed to compile runtime");
            }
        }

        // Link with clang
        let status = Command::new("clang")
            .arg("-o")
            .arg(&exe_path)
            .arg(&output_path)
            .arg(&runtime_obj)
            .arg("-lm") // Link math library
            .status()
            .context("Failed to link executable")?;

        if !status.success() {
            anyhow::bail!("Linking failed");
        }

        println!(
            "{}: Created executable {:?}",
            "success".green().bold(),
            exe_path
        );
    } else {
        println!("{}: Created {:?}", "success".green().bold(), output_path);
    }

    Ok(())
}

fn run(input: PathBuf, args: Vec<String>, opt_level: u8) -> Result<()> {
    log::info!("Running {:?} with args: {:?}", input, args);

    // Create a temporary executable
    let temp_dir = std::env::temp_dir();
    let temp_exe = temp_dir.join(format!("yuni_run_{}", std::process::id()));

    // Compile to executable
    compile(
        input,
        Some(temp_exe.clone()),
        EmitType::Obj,
        opt_level,
        false,
        false,
        true,
    )?;

    // Run the executable
    log::debug!("Executing {:?}", temp_exe);
    let status = Command::new(&temp_exe)
        .args(&args)
        .status()
        .context("Failed to execute compiled program")?;

    // Clean up
    fs::remove_file(&temp_exe).ok();

    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        } else {
            anyhow::bail!("Program terminated by signal");
        }
    }

    Ok(())
}

fn repl() -> Result<()> {
    println!("{}", "Yuni Language REPL".blue().bold());
    println!("Type ':quit' or ':q' to exit, ':help' for help\n");

    let context = inkwell::context::Context::create();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut line_number = 1;

    loop {
        // Print prompt
        print!("yuni:{:03}> ", line_number);
        stdout.flush()?;

        // Read input
        let mut input = String::new();
        stdin.read_line(&mut input)?;
        let input = input.trim();

        // Handle REPL commands
        match input {
            ":quit" | ":q" => {
                println!("Goodbye!");
                break;
            }
            ":help" | ":h" => {
                println!("REPL commands:");
                println!("  :quit, :q    Exit the REPL");
                println!("  :help, :h    Show this help message");
                println!("  :clear, :c   Clear the screen");
                println!("\nEnter Yuni expressions or statements to evaluate them.");
                continue;
            }
            ":clear" | ":c" => {
                print!("\x1B[2J\x1B[1;1H"); // ANSI escape codes to clear screen
                continue;
            }
            "" => continue,
            _ => {}
        }

        // Try to evaluate the input
        match evaluate_repl_input(input, &context, line_number) {
            Ok(Some(result)) => {
                println!("{}: {}", "result".green(), result);
            }
            Ok(None) => {
                // Statement executed successfully, no result to print
            }
            Err(e) => {
                eprintln!("{}: {}", "error".red(), e);
            }
        }

        line_number += 1;
    }

    Ok(())
}

fn evaluate_repl_input(
    input: &str,
    _context: &inkwell::context::Context,
    _line_number: usize,
) -> Result<Option<String>> {
    // For now, just parse and check syntax
    let lexer = Lexer::new(input);
    let tokens: Vec<_> = lexer.collect();

    // Check for lexer errors
    for token in &tokens {
        if matches!(token.token, Token::Error) {
            anyhow::bail!("Lexical error: unrecognized token");
        }
    }

    // Try to parse as an expression first
    let mut parser = YuniParser::new(tokens.clone());
    match parser.parse_expression() {
        Ok(expr) => {
            // TODO: Evaluate expression and return result
            Ok(Some(format!("{:?}", expr)))
        }
        Err(_) => {
            // Try to parse as a statement
            let mut parser = YuniParser::new(tokens);
            match parser.parse_statement() {
                Ok(_stmt) => {
                    // TODO: Execute statement
                    Ok(None)
                }
                Err(e) => anyhow::bail!("Parse error: {}", e),
            }
        }
    }
}

fn check(input: PathBuf) -> Result<()> {
    log::info!("Checking {:?}", input);

    let state = CompilationState::new(input)?;
    let mut has_errors = false;

    // 1. Tokenize
    log::debug!("Starting lexical analysis");
    let lexer = Lexer::new(&state.source);
    let tokens: Vec<_> = lexer.collect();

    // Check for lexer errors
    for token in &tokens {
        if matches!(token.token, Token::Error) {
            has_errors = true;
            let diagnostic = Diagnostic::error()
                .with_message("Lexical error: unrecognized token")
                .with_labels(vec![Label::primary(
                    state.file_id,
                    token.span.start..token.span.end,
                )]);
            state.report_error(&diagnostic)?;
        }
    }

    if has_errors {
        anyhow::bail!("Lexical analysis failed");
    }

    // 2. Parse
    log::debug!("Starting parsing");
    let mut parser = YuniParser::new(tokens);
    let ast = match parser.parse() {
        Ok(program) => program,
        Err(e) => {
            let diagnostic = Diagnostic::error()
                .with_message(format!("Parse error: {}", e))
                .with_labels(vec![Label::primary(state.file_id, 0..1)]);
            state.report_error(&diagnostic)?;
            anyhow::bail!("Parsing failed");
        }
    };

    // 3. Semantic analysis
    log::debug!("Starting semantic analysis");
    let mut analyzer = analyzer::SemanticAnalyzer::new();
    if let Err(e) = analyzer.analyze(&ast) {
        let diagnostic = Diagnostic::error()
            .with_message(format!("Semantic error: {}", e))
            .with_labels(vec![Label::primary(state.file_id, 0..1)]);
        state.report_error(&diagnostic)?;
        anyhow::bail!("Semantic analysis failed");
    }

    println!("{}: No errors found", "success".green().bold());
    Ok(())
}
