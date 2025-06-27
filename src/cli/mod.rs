//! Command-line interface module

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "yunilang")]
#[command(author, version, about = "The Yuni language compiler", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum EmitType {
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
pub enum Commands {
    /// Compile a Yuni source file
    Compile {
        /// The input source file
        input: PathBuf,

        /// The output file (defaults to input basename)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// What to emit
        #[arg(long, value_enum, default_value = "executable")]
        emit: EmitType,

        /// Optimization level (0-3)
        #[arg(short = 'O', default_value = "0")]
        opt_level: u8,

        /// Dump the AST after parsing
        #[arg(long)]
        dump_ast: bool,

        /// Dump tokens after lexing
        #[arg(long)]
        dump_tokens: bool,

        /// Keep temporary files (LLVM IR, object files)
        #[arg(long)]
        keep_temps: bool,
    },

    /// Run a Yuni program directly (JIT compilation)
    Run {
        /// The input source file
        input: PathBuf,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Optimization level (0-3)
        #[arg(short = 'O', default_value = "0")]
        opt_level: u8,
    },

    /// Check a Yuni source file for errors without compiling
    Check {
        /// The input source file
        input: PathBuf,
    },
}

pub struct CompileOptions {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub emit: EmitType,
    pub opt_level: u8,
    pub dump_ast: bool,
    pub dump_tokens: bool,
    pub keep_temps: bool,
    pub verbose: bool,
}