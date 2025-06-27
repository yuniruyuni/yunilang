//! Run command implementation

use std::path::PathBuf;
use std::process::Command;

use crate::cli::{CompileOptions, EmitType};
use crate::commands::compile::compile;
use crate::error::{YuniError, YuniResult};

pub fn run(input: PathBuf, args: Vec<String>, opt_level: u8) -> YuniResult<()> {
    log::info!("Running {:?} with args: {:?}", input, args);

    // Create a temporary executable
    let temp_dir = std::env::temp_dir();
    let temp_exe = temp_dir.join(format!("yuni_run_{}", std::process::id()));

    // Compile to executable
    compile(CompileOptions {
        input,
        output: Some(temp_exe.clone()),
        emit: EmitType::Executable,
        opt_level,
        dump_ast: false,
        dump_tokens: false,
        keep_temps: false, // don't keep temps for run
        verbose: false, // not verbose
    })?;

    // Run the executable
    log::debug!("Executing {:?}", temp_exe);
    let status = Command::new(&temp_exe)
        .args(&args)
        .status()
        .map_err(|e| YuniError::Other(format!("Failed to execute compiled program: {}", e)))?;

    // Clean up the temporary executable
    if let Err(e) = std::fs::remove_file(&temp_exe) {
        log::warn!("Failed to remove temporary executable: {}", e);
    }

    // Exit with the same status code as the executed program
    std::process::exit(status.code().unwrap_or(1));
}