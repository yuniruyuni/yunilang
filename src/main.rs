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
mod compiler;
mod error;
mod lexer;
mod parser;
mod runtime;

use crate::compiler::{CompilationPipeline, CompilationState};
use crate::error::{DiagnosticError, ErrorCollector, YuniError, YuniResult};
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

fn main() -> YuniResult<()> {
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


fn compile(
    input: PathBuf,
    output: Option<PathBuf>,
    emit: EmitType,
    opt_level: u8,
    dump_ast: bool,
    dump_tokens: bool,
    keep_temps: bool,
    verbose: bool,
) -> YuniResult<()> {
    if verbose {
        println!("{}: Compiling {:?} with optimization level O{}", 
                "info".blue().bold(), input, opt_level);
    }

    // Initialize compilation state
    let state = CompilationState::new(&input)?;
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, verbose);

    // Run the compilation pipeline
    let tokens = pipeline.tokenize();

    if dump_tokens {
        println!("{}", "=== Tokens ===".blue().bold());
        for (i, token) in tokens.iter().enumerate() {
            println!("{:4}: {:?}", i, token);
        }
        println!();
    }

    let ast = pipeline.parse(tokens);

    if let Some(ref ast) = ast {
        if dump_ast {
            println!("{}", "=== AST ===".blue().bold());
            println!("{}", serde_json::to_string_pretty(ast)
                .map_err(|e| YuniError::Other(format!("Failed to serialize AST: {}", e)))?);
            println!();
        }

        pipeline.analyze(ast);
    }

    // エラーがある場合は早期リターン
    if pipeline.state().has_errors() {
        pipeline.report_errors()?;
        return Err(YuniError::Other("Compilation failed".to_string()));
    }

    // コード生成
    let codegen = if let Some(ast) = ast {
        pipeline.codegen(&ast)?
    } else {
        return Err(YuniError::Other("No AST generated".to_string()));
    };

    // Create temporary directory for intermediate files
    let temp_dir = if keep_temps {
        None
    } else {
        Some(TempDir::new().map_err(|e| YuniError::Io(format!("Failed to create temporary directory: {}", e)))?)
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
                .map_err(|e| YuniError::Other(format!("Failed to run llc: {}", e)))?;

            if !status.success() {
                return Err(YuniError::Other(format!("llc failed with status: {}", status)));
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
                .map_err(|e| YuniError::Other(format!("Failed to run llc on program: {}", e)))?;

            if !status.success() {
                return Err(YuniError::Other("Failed to compile program LLVM IR to object file".to_string()));
            }

            if verbose { println!("{}: Step 3 - Compiling Rust runtime to object file", "substep".yellow()); }

            // Step 3: Compile Rust runtime to object file
            let runtime_rs_path = Path::new("src/runtime/mod.rs");
            
            if !runtime_rs_path.exists() {
                return Err(YuniError::Other(format!("Runtime Rust file not found at {:?}", runtime_rs_path)));
            }

            // First compile the Rust runtime to a static library
            let runtime_lib = get_temp_path("libyuniruntime.a");
            let status = Command::new("rustc")
                .arg("--crate-type=staticlib")
                .arg("--crate-name=yuniruntime")
                .arg(format!("-Copt-level={}", opt_level))
                .arg("-o")
                .arg(&runtime_lib)
                .arg(runtime_rs_path)
                .status()
                .map_err(|e| YuniError::Other(format!("Failed to compile Rust runtime: {}", e)))?;

            if !status.success() {
                return Err(YuniError::Other("Failed to compile Rust runtime".to_string()));
            }

            if verbose { println!("{}: Step 4 - Linking executable", "substep".yellow()); }

            // Step 4: Link everything together
            let mut cmd = Command::new("clang");
            cmd.arg("-o")
                .arg(&executable_path)
                .arg(&program_obj)
                .arg(&runtime_lib)
                .arg("-lm") // Math library
                .arg("-lpthread"); // Thread library for Rust runtime
            
            // On macOS, we need to link against system libraries
            #[cfg(target_os = "macos")]
            {
                cmd.arg("-framework").arg("System");
                cmd.arg("-lc++");
            }
            
            // On Linux, link against standard C++ library
            #[cfg(target_os = "linux")]
            {
                cmd.arg("-lstdc++");
            }

            let status = cmd.status().map_err(|e| YuniError::Other(format!("Failed to link executable: {}", e)))?;

            if !status.success() {
                return Err(YuniError::Other("Failed to link executable".to_string()));
            }

            // Clean up intermediate files if not keeping them
            if temp_dir.is_some() {
                // Files will be automatically cleaned up when temp_dir is dropped
            } else if !keep_temps {
                // If we're using the source directory but not keeping temps, clean up manually
                fs::remove_file(&program_ll).ok();
                fs::remove_file(&program_obj).ok();
                fs::remove_file(&runtime_lib).ok();
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

fn find_llc_command() -> YuniResult<String> {
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

    Err(YuniError::Other("Could not find llc command. Please install LLVM 18 or add llc to your PATH".to_string()))
}

fn run(input: PathBuf, args: Vec<String>, opt_level: u8) -> YuniResult<()> {
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
        .map_err(|e| YuniError::Other(format!("Failed to execute compiled program: {}", e)))?;

    // Clean up
    fs::remove_file(&temp_exe).ok();

    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        } else {
            return Err(YuniError::Other("Program terminated by signal".to_string()));
        }
    }

    Ok(())
}

fn repl() -> YuniResult<()> {
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
) -> YuniResult<Option<String>> {
    // For now, just parse and check syntax
    let lexer = Lexer::new(input);
    let tokens: Vec<_> = lexer.collect_tokens();

    // Check for lexer errors
    for token in &tokens {
        if matches!(token.token, Token::Error) {
            return Err(YuniError::Other("Lexical error: unrecognized token".to_string()));
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
                Err(e) => Err(YuniError::Other(format!("Parse error: {}", e))),
            }
        }
    }
}

fn check(input: PathBuf) -> YuniResult<()> {
    log::info!("Checking {:?}", input);

    // コンパイルパイプラインを使用
    let state = CompilationState::new(&input)?;
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, false);

    // レキシカル解析
    let tokens = pipeline.tokenize();
    
    // 構文解析
    let ast = pipeline.parse(tokens);
    
    // セマンティック解析
    if let Some(ref ast) = ast {
        pipeline.analyze(ast);
    }
    
    // エラーレポート
    pipeline.report_errors()?;
    
    if !pipeline.state().has_errors() {
        println!("{}: No errors found", "success".green().bold());
        Ok(())
    } else {
        Err(YuniError::Other("Check failed".to_string()))
    }
}
