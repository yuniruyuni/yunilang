//! Executable generation implementation

use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use crate::error::{YuniError, YuniResult};

pub fn emit_executable(
    module: &inkwell::module::Module,
    input: &Path,
    output: Option<PathBuf>,
    opt_level: u8,
    keep_temps: bool,
    verbose: bool,
) -> YuniResult<()> {
    // Create temporary directory for intermediate files
    let temp_dir = if keep_temps {
        None
    } else {
        Some(TempDir::new().map_err(|e| YuniError::Other(format!("Failed to create temp directory: {}", e)))?)
    };

    let get_temp_path = |filename: &str| -> PathBuf {
        if let Some(ref dir) = temp_dir {
            dir.path().join(filename)
        } else {
            input.parent().unwrap_or(Path::new(".")).join(filename)
        }
    };

    // Determine output path
    let executable_path = output.unwrap_or_else(|| {
        let mut path = input.to_path_buf();
        path.set_extension("");
        path
    });

    if verbose {
        println!("{}: Building executable {:?}", "step".cyan().bold(), executable_path);
        println!("{}: Step 1 - Generating LLVM IR", "substep".yellow());
    }

    // Step 1: Generate LLVM IR for the main program
    let program_ll = get_temp_path("program.ll");
    module.print_to_file(&program_ll)
        .map_err(|e| YuniError::Other(format!("Failed to write LLVM IR: {}", e)))?;

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
        let _ = fs::remove_file(&program_ll);
        let _ = fs::remove_file(&program_obj);
        let _ = fs::remove_file(&runtime_lib);
    }

    if verbose {
        println!("{}: Successfully built executable at {:?}", "success".green().bold(), executable_path);
    }

    Ok(())
}

/// Find the appropriate LLVM compiler command
fn find_llc_command() -> YuniResult<String> {
    // Try to find llc in the order of preference
    let commands = ["llc-18", "llc"];
    
    for cmd in &commands {
        if Command::new(cmd).arg("--version").output().is_ok() {
            return Ok(cmd.to_string());
        }
    }
    
    Err(YuniError::Other("Could not find llc (LLVM compiler). Please install LLVM 18.".to_string()))
}