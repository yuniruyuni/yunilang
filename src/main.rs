use clap::Parser;
use colored::Colorize;

mod analyzer;
mod ast;
mod cli;
mod codegen;
mod commands;
mod compiler;
mod error;
mod lexer;
mod parser;
mod runtime;

use crate::cli::{Cli, Commands, CompileOptions};
use crate::commands::{check, compile, run};
use crate::error::YuniResult;

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
        } => compile(CompileOptions {
            input,
            output,
            emit,
            opt_level,
            dump_ast,
            dump_tokens,
            keep_temps,
            verbose: cli.verbose,
        }),
        Commands::Run {
            input,
            args,
            opt_level,
        } => run(input, args, opt_level),
        Commands::Check { input } => check(input),
    };

    if let Err(e) = result {
        eprintln!("{}: {}", "error".red().bold(), e);
        // Debug情報を表示
        eprintln!("{:?}", e);
        std::process::exit(1);
    }

    Ok(())
}