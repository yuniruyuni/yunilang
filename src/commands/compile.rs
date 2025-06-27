//! Compile command implementation

use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::cli::{CompileOptions, EmitType};
use crate::compiler::{CompilationPipeline, CompilationState};
use crate::error::{YuniError, YuniResult};

pub fn compile(opts: CompileOptions) -> YuniResult<()> {
    let CompileOptions {
        input,
        output,
        emit,
        opt_level,
        dump_ast,
        dump_tokens,
        keep_temps,
        verbose,
    } = opts;
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
    }

    // Run remaining compilation stages
    if let Some(ast) = ast {
        let _ = pipeline.analyze(&ast);
        let codegen = pipeline.codegen(&ast)?;
        let module = codegen.get_module();

        // Optimize the module
        optimize_module(module, opt_level);

        match emit {
            EmitType::LlvmIr => emit_llvm_ir(module, &input, output, verbose),
            EmitType::Obj => emit_object_file(module, &input, output, verbose),
            EmitType::Asm => emit_assembly(module, &input, output, verbose),
            EmitType::Executable => {
                super::executable::emit_executable(module, &input, output, opt_level, keep_temps, verbose)
            }
        }
    } else {
        Err(YuniError::Other("Failed to parse AST".to_string()))
    }
}

fn optimize_module(module: &inkwell::module::Module, opt_level: u8) {
    use inkwell::passes::PassBuilderOptions;
    use inkwell::targets::{Target, TargetMachine, TargetTriple};
    use inkwell::OptimizationLevel;

    let _config = Target::initialize_native(&Default::default())
        .expect("Failed to initialize native target");

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple).expect("Failed to create target from triple");
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            match opt_level {
                0 => OptimizationLevel::None,
                1 => OptimizationLevel::Less,
                2 => OptimizationLevel::Default,
                _ => OptimizationLevel::Aggressive,
            },
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .expect("Failed to create target machine");

    module.set_data_layout(&machine.get_target_data().get_data_layout());
    module.set_triple(&triple);

    // Run module passes using the new pass manager
    let options = PassBuilderOptions::create();
    options.set_verify_each(true);
    options.set_debug_logging(false);
    
    let passes = match opt_level {
        0 => "default<O0>",
        1 => "default<O1>",
        2 => "default<O2>",
        _ => "default<O3>",
    };
    
    module.run_passes(passes, &machine, options)
        .expect("Failed to run optimization passes");
}

fn emit_llvm_ir(module: &inkwell::module::Module, input: &Path, output: Option<PathBuf>, verbose: bool) -> YuniResult<()> {
    let output_path = output.unwrap_or_else(|| input.with_extension("ll"));
    if verbose {
        println!("{}: Writing LLVM IR to {:?}", "info".blue().bold(), output_path);
    }
    module.print_to_file(&output_path)
        .map_err(|e| YuniError::Other(format!("Failed to write LLVM IR: {}", e)))?;
    if verbose {
        println!("{}: Successfully wrote LLVM IR", "success".green().bold());
    }
    Ok(())
}

fn emit_object_file(module: &inkwell::module::Module, input: &Path, output: Option<PathBuf>, verbose: bool) -> YuniResult<()> {
    use inkwell::targets::{Target, TargetMachine, FileType};

    let output_path = output.unwrap_or_else(|| input.with_extension("o"));
    if verbose {
        println!("{}: Writing object file to {:?}", "info".blue().bold(), output_path);
    }

    Target::initialize_native(&Default::default())
        .map_err(|e| YuniError::Other(format!("Failed to initialize native target: {}", e)))?;

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple)
        .map_err(|e| YuniError::Other(format!("Failed to create target from triple: {}", e)))?;
    
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            inkwell::OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .ok_or_else(|| YuniError::Other("Failed to create target machine".to_string()))?;

    machine
        .write_to_file(module, FileType::Object, &output_path)
        .map_err(|e| YuniError::Other(format!("Failed to write object file: {}", e)))?;

    if verbose {
        println!("{}: Successfully wrote object file", "success".green().bold());
    }
    Ok(())
}

fn emit_assembly(module: &inkwell::module::Module, input: &Path, output: Option<PathBuf>, verbose: bool) -> YuniResult<()> {
    use inkwell::targets::{Target, TargetMachine, FileType};

    let output_path = output.unwrap_or_else(|| input.with_extension("s"));
    if verbose {
        println!("{}: Writing assembly to {:?}", "info".blue().bold(), output_path);
    }

    Target::initialize_native(&Default::default())
        .map_err(|e| YuniError::Other(format!("Failed to initialize native target: {}", e)))?;

    let triple = TargetMachine::get_default_triple();
    let target = Target::from_triple(&triple)
        .map_err(|e| YuniError::Other(format!("Failed to create target from triple: {}", e)))?;
    
    let machine = target
        .create_target_machine(
            &triple,
            "generic",
            "",
            inkwell::OptimizationLevel::Default,
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .ok_or_else(|| YuniError::Other("Failed to create target machine".to_string()))?;

    machine
        .write_to_file(module, FileType::Assembly, &output_path)
        .map_err(|e| YuniError::Other(format!("Failed to write assembly file: {}", e)))?;

    if verbose {
        println!("{}: Successfully wrote assembly file", "success".green().bold());
    }
    Ok(())
}