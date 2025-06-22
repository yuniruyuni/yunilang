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
use tempfile::TempDir;

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
    /// Emit executable (default)
    #[value(name = "executable")]
    Executable,
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

        /// Output file (for executables or object files)
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,

        /// What to emit
        #[arg(long = "emit", value_enum, default_value = "executable")]
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

        /// Keep intermediate files (LLVM IR, object files)
        #[arg(long)]
        keep_temps: bool,
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
            keep_temps,
        } => compile(input, output, emit, opt_level, dump_ast, dump_tokens, keep_temps, cli.verbose),
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
    keep_temps: bool,
    verbose: bool,
) -> Result<()> {
    if verbose {
        println!("{}: Compiling {:?} with optimization level O{}", 
                "info".blue().bold(), input, opt_level);
    }

    // Initialize compilation state
    let state = CompilationState::new(input.clone())?;

    // 1. Tokenize
    if verbose { println!("{}: Starting lexical analysis", "step".cyan().bold()); }
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
    if verbose { println!("{}: Starting parsing", "step".cyan().bold()); }
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
    if verbose { println!("{}: Starting semantic analysis", "step".cyan().bold()); }
    let mut analyzer = analyzer::SemanticAnalyzer::new();
    if let Err(e) = analyzer.analyze(&ast) {
        let diagnostic = Diagnostic::error()
            .with_message(format!("Semantic error: {}", e))
            .with_labels(vec![Label::primary(state.file_id, 0..1)]);
        state.report_error(&diagnostic)?;
        anyhow::bail!("Semantic analysis failed");
    }

    // 4. Code generation
    if verbose { println!("{}: Starting code generation", "step".cyan().bold()); }
    let context = inkwell::context::Context::create();
    let mut codegen =
        codegen::CodeGenerator::new(&context, &state.source_file.display().to_string());

    // Compile the AST
    codegen.compile_program(&ast)?;

    // Create temporary directory for intermediate files
    let temp_dir = if keep_temps {
        None
    } else {
        Some(TempDir::new().context("Failed to create temporary directory")?)
    };

    let get_temp_path = |name: &str| -> PathBuf {
        if let Some(ref dir) = temp_dir {
            dir.path().join(name)
        } else {
            input.parent().unwrap_or(Path::new(".")).join(name)
        }
    };

    // Determine output path and handle different emit types
    match emit {
        EmitType::LlvmIr => {
            let output_path = output.unwrap_or_else(|| {
                let mut path = input.clone();
                path.set_extension("ll");
                path
            });
            if verbose { println!("{}: Writing LLVM IR to {:?}", "step".cyan().bold(), output_path); }
            codegen.write_llvm_ir(&output_path)?;
            println!("{}: Created LLVM IR file {:?}", "success".green().bold(), output_path);
            return Ok(());
        }
        EmitType::Obj => {
            let output_path = output.unwrap_or_else(|| {
                let mut path = input.clone();
                path.set_extension("o");
                path
            });
            if verbose { println!("{}: Writing object file to {:?}", "step".cyan().bold(), output_path); }
            let opt = inkwell_opt_level(opt_level);
            codegen.write_object_file(&output_path, opt)?;
            println!("{}: Created object file {:?}", "success".green().bold(), output_path);
            return Ok(());
        }
        EmitType::Asm => {
            let output_path = output.unwrap_or_else(|| {
                let mut path = input.clone();
                path.set_extension("s");
                path
            });
            if verbose { println!("{}: Writing assembly to {:?}", "step".cyan().bold(), output_path); }
            
            // First write LLVM IR to a temp file
            let temp_ll = get_temp_path("temp.ll");
            codegen.write_llvm_ir(&temp_ll)?;

            // Use llc to convert to assembly
            let llc_cmd = find_llc_command()?;
            let status = Command::new(&llc_cmd)
                .arg(format!("-O{}", opt_level))
                .arg("-o")
                .arg(&output_path)
                .arg(&temp_ll)
                .status()
                .context("Failed to run llc")?;

            if !status.success() {
                anyhow::bail!("llc failed with status: {}", status);
            }

            // Clean up temp file if not keeping temps
            if temp_dir.is_some() {
                fs::remove_file(&temp_ll).ok();
            }
            
            println!("{}: Created assembly file {:?}", "success".green().bold(), output_path);
            return Ok(());
        }
        EmitType::Executable => {
            // This is the main case - build a complete executable
            let executable_path = output.unwrap_or_else(|| {
                let mut path = input.clone();
                path.set_extension("");
                if path.file_name().unwrap() == input.file_stem().unwrap() {
                    // If input was "file.yuni", output should be "file", not "file."
                    path
                } else {
                    path
                }
            });

            if verbose {
                println!("{}: Building executable {:?}", "step".cyan().bold(), executable_path);
                println!("{}: Step 1 - Generating LLVM IR", "substep".yellow());
            }

            // Step 1: Generate LLVM IR for the main program
            let program_ll = get_temp_path("program.ll");
            codegen.write_llvm_ir(&program_ll)?;

            if verbose { println!("{}: Step 2 - Compiling program to object file", "substep".yellow()); }

            // Step 2: Compile LLVM IR to object file
            let program_obj = get_temp_path("program.o");
            let llc_cmd = find_llc_command()?;
            let status = Command::new(&llc_cmd)
                .arg("-filetype=obj")
                .arg(format!("-O{}", opt_level))
                .arg("-o")
                .arg(&program_obj)
                .arg(&program_ll)
                .status()
                .context("Failed to run llc on program")?;

            if !status.success() {
                anyhow::bail!("Failed to compile program LLVM IR to object file");
            }

            if verbose { println!("{}: Step 3 - Compiling runtime to object file", "substep".yellow()); }

            // Step 3: Compile runtime.c to object file
            let runtime_obj = get_temp_path("runtime.o");
            let runtime_c_path = Path::new("src/runtime.c");
            
            if !runtime_c_path.exists() {
                anyhow::bail!("Runtime C file not found at {:?}", runtime_c_path);
            }

            let status = Command::new("clang")
                .arg("-c")
                .arg(format!("-O{}", opt_level))
                .arg("-o")
                .arg(&runtime_obj)
                .arg(runtime_c_path)
                .status()
                .context("Failed to compile runtime.c")?;

            if !status.success() {
                anyhow::bail!("Failed to compile runtime.c");
            }

            if verbose { println!("{}: Step 4 - Linking executable", "substep".yellow()); }

            // Step 4: Link everything together
            let mut cmd = Command::new("clang");
            cmd.arg("-o")
                .arg(&executable_path)
                .arg(&program_obj)
                .arg(&runtime_obj)
                .arg("-lm"); // Math library

            let status = cmd.status().context("Failed to link executable")?;

            if !status.success() {
                anyhow::bail!("Failed to link executable");
            }

            // Clean up intermediate files if not keeping them
            if temp_dir.is_some() {
                // Files will be automatically cleaned up when temp_dir is dropped
            } else if !keep_temps {
                // If we're using the source directory but not keeping temps, clean up manually
                fs::remove_file(&program_ll).ok();
                fs::remove_file(&program_obj).ok();
                fs::remove_file(&runtime_obj).ok();
            }

            println!("{}: Created executable {:?}", "success".green().bold(), executable_path);
            if keep_temps {
                println!("{}: Intermediate files kept in {:?}", "info".blue(), 
                    input.parent().unwrap_or(Path::new(".")));
            }
        }
    }

    Ok(())
}

fn inkwell_opt_level(level: u8) -> inkwell::OptimizationLevel {
    match level {
        0 => inkwell::OptimizationLevel::None,
        1 => inkwell::OptimizationLevel::Less,
        2 => inkwell::OptimizationLevel::Default,
        3 => inkwell::OptimizationLevel::Aggressive,
        _ => unreachable!(),
    }
}

fn find_llc_command() -> Result<String> {
    // Try to find llc command in system paths
    if let Ok(output) = Command::new("which").arg("llc").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(path);
            }
        }
    }

    // On macOS, try homebrew paths for LLVM 18
    #[cfg(target_os = "macos")]
    {
        let homebrew_paths = [
            "/opt/homebrew/opt/llvm@18/bin/llc",
            "/usr/local/opt/llvm@18/bin/llc",  
            "/opt/homebrew/Cellar/llvm@18/18.1.8/bin/llc",
            "/opt/homebrew/bin/llc",
        ];
        
        for path in &homebrew_paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
    }

    // Try common Linux paths
    #[cfg(target_os = "linux")]
    {
        let linux_paths = [
            "/usr/bin/llc-18",
            "/usr/bin/llc",
            "/usr/local/bin/llc-18",
            "/usr/local/bin/llc",
        ];
        
        for path in &linux_paths {
            if Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }
    }

    anyhow::bail!("Could not find llc command. Please install LLVM 18 or add llc to your PATH")
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
        EmitType::Executable,
        opt_level,
        false,
        false,
        false, // don't keep temps for run
        false, // not verbose
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
