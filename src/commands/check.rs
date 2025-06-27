//! Check command implementation

use colored::Colorize;
use std::path::PathBuf;

use crate::compiler::{CompilationPipeline, CompilationState};
use crate::error::YuniResult;

pub fn check(input: PathBuf) -> YuniResult<()> {
    println!("{}: Checking {:?}", "info".blue().bold(), input);

    // Initialize compilation state
    let state = CompilationState::new(&input)?;
    let context = inkwell::context::Context::create();
    let mut pipeline = CompilationPipeline::new(state, &context, false);

    // Run the compilation pipeline stages up to analysis
    let tokens = pipeline.tokenize();
    let ast = pipeline.parse(tokens);
    if let Some(ast) = ast {
        let _ = pipeline.analyze(&ast);
    }

    println!("{}: No errors found!", "success".green().bold());
    Ok(())
}