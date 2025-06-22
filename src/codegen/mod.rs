//! Code generation module for the Yuni language.
//!
//! This module is responsible for generating LLVM IR from the AST.

use crate::ast::*;
use anyhow::{bail, Context, Result};
use inkwell::builder::Builder;
use inkwell::context::Context as LLVMContext;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetMachine, TargetTriple};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate, OptimizationLevel};
use std::collections::HashMap;
use std::path::Path;

/// Symbol table entry for tracking variables and their types
#[derive(Debug, Clone)]
struct Symbol {
    ptr: PointerValue<'static>,
    ty: Type,
    is_mutable: bool,
}

/// Scope for managing variable lifetimes
struct Scope {
    symbols: HashMap<String, Symbol>,
}

impl Scope {
    fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }
}

/// Main code generator structure
pub struct CodeGenerator<'ctx> {
    context: &'ctx LLVMContext,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    pass_manager: PassManager<FunctionValue<'ctx>>,

    // Symbol tables and scopes
    scopes: Vec<Scope>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    types: HashMap<String, StructType<'ctx>>,

    // Current function being compiled
    current_function: Option<FunctionValue<'ctx>>,

    // Runtime functions
    runtime_functions: HashMap<String, FunctionValue<'ctx>>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx LLVMContext, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let pass_manager = PassManager::create(&module);

        // Initialize the pass manager
        pass_manager.initialize();

        let mut codegen = Self {
            context,
            module,
            builder,
            pass_manager,
            scopes: vec![Scope::new()], // Global scope
            functions: HashMap::new(),
            types: HashMap::new(),
            current_function: None,
            runtime_functions: HashMap::new(),
        };

        // Initialize runtime functions
        codegen.declare_runtime_functions();

        codegen
    }

    /// Declare external runtime functions
    fn declare_runtime_functions(&mut self) {
        // Printf for println
        let _i8_type = self.context.i8_type();
        let i8_ptr_type = self.context.ptr_type(AddressSpace::default());
        let i32_type = self.context.i32_type();

        let printf_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
        let printf = self
            .module
            .add_function("printf", printf_type, Some(Linkage::External));
        self.runtime_functions.insert("printf".to_string(), printf);

        // Malloc for memory allocation
        let i64_type = self.context.i64_type();
        let malloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let malloc = self
            .module
            .add_function("malloc", malloc_type, Some(Linkage::External));
        self.runtime_functions.insert("malloc".to_string(), malloc);

        // Free for memory deallocation
        let void_type = self.context.void_type();
        let free_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let free = self
            .module
            .add_function("free", free_type, Some(Linkage::External));
        self.runtime_functions.insert("free".to_string(), free);

        // String functions
        let strlen_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
        let strlen = self
            .module
            .add_function("strlen", strlen_type, Some(Linkage::External));
        self.runtime_functions.insert("strlen".to_string(), strlen);

        let memcpy_type = i8_ptr_type.fn_type(
            &[i8_ptr_type.into(), i8_ptr_type.into(), i64_type.into()],
            false,
        );
        let memcpy = self
            .module
            .add_function("memcpy", memcpy_type, Some(Linkage::External));
        self.runtime_functions.insert("memcpy".to_string(), memcpy);

        // Runtime helper functions
        let concat_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], false);
        let concat =
            self.module
                .add_function("yuni_string_concat", concat_type, Some(Linkage::External));
        self.runtime_functions
            .insert("yuni_string_concat".to_string(), concat);

        let i64_to_string_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
        let i64_to_string = self.module.add_function(
            "yuni_i64_to_string",
            i64_to_string_type,
            Some(Linkage::External),
        );
        self.runtime_functions
            .insert("yuni_i64_to_string".to_string(), i64_to_string);

        let f64_type = self.context.f64_type();
        let f64_to_string_type = i8_ptr_type.fn_type(&[f64_type.into()], false);
        let f64_to_string = self.module.add_function(
            "yuni_f64_to_string",
            f64_to_string_type,
            Some(Linkage::External),
        );
        self.runtime_functions
            .insert("yuni_f64_to_string".to_string(), f64_to_string);

        let bool_type = self.context.bool_type();
        let bool_to_string_type = i8_ptr_type.fn_type(&[bool_type.into()], false);
        let bool_to_string = self.module.add_function(
            "yuni_bool_to_string",
            bool_to_string_type,
            Some(Linkage::External),
        );
        self.runtime_functions
            .insert("yuni_bool_to_string".to_string(), bool_to_string);

        let println_type = void_type.fn_type(&[i8_ptr_type.into()], false);
        let println =
            self.module
                .add_function("yuni_println", println_type, Some(Linkage::External));
        self.runtime_functions
            .insert("yuni_println".to_string(), println);
    }

    /// Compile a complete program
    pub fn compile_program(&mut self, program: &Program) -> Result<()> {
        // First pass: declare all types
        for item in &program.items {
            if let Item::TypeDef(type_def) = item {
                self.declare_type(type_def)?;
            }
        }

        // Second pass: declare all functions
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    self.declare_function(func)?;
                }
                Item::Method(method) => {
                    self.declare_method(method)?;
                }
                _ => {}
            }
        }

        // Third pass: compile function bodies
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    self.compile_function(func)?;
                }
                Item::Method(method) => {
                    self.compile_method(method)?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Declare a type (struct or enum)
    fn declare_type(&mut self, type_def: &TypeDef) -> Result<()> {
        match type_def {
            TypeDef::Struct(struct_def) => {
                let field_types: Vec<BasicTypeEnum> = struct_def
                    .fields
                    .iter()
                    .map(|field| self.get_llvm_type(&field.ty))
                    .collect::<Result<Vec<_>>>()?;

                let struct_type = self.context.struct_type(&field_types, false);
                self.types.insert(struct_def.name.clone(), struct_type);
            }
            TypeDef::Enum(enum_def) => {
                // Enums are represented as tagged unions
                // Tag (i32) + largest variant
                let tag_type = self.context.i32_type();
                let mut max_size = 0;
                let mut largest_fields = vec![];

                for variant in &enum_def.variants {
                    let field_types: Vec<BasicTypeEnum> = variant
                        .fields
                        .iter()
                        .map(|field| self.get_llvm_type(&field.ty))
                        .collect::<Result<Vec<_>>>()?;

                    let variant_size = field_types.len();
                    if variant_size > max_size {
                        max_size = variant_size;
                        largest_fields = field_types;
                    }
                }

                let mut enum_fields = vec![tag_type.into()];
                enum_fields.extend(largest_fields);

                let enum_type = self.context.struct_type(&enum_fields, false);
                self.types.insert(enum_def.name.clone(), enum_type);
            }
        }
        Ok(())
    }

    /// Declare a function
    fn declare_function(&mut self, func: &FunctionDecl) -> Result<()> {
        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|param| Ok(self.get_llvm_type(&param.ty)?.into()))
            .collect::<Result<Vec<_>>>()?;

        let fn_type = if let Some(ret_ty) = &func.return_type {
            let return_type = self.get_llvm_type(ret_ty)?;
            match return_type {
                BasicTypeEnum::ArrayType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::FloatType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::IntType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::PointerType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::StructType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::VectorType(t) => t.fn_type(&param_types, false),
            }
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        let linkage = if func.is_public || func.name == "main" {
            Linkage::External
        } else {
            Linkage::Private
        };
        let function = self.module.add_function(&func.name, fn_type, Some(linkage));
        self.functions.insert(func.name.clone(), function);

        Ok(())
    }

    /// Declare a method (convert to function with receiver as first parameter)
    fn declare_method(&mut self, method: &MethodDecl) -> Result<()> {
        let mut param_types: Vec<BasicMetadataTypeEnum> =
            vec![self.get_llvm_type(&method.receiver.ty)?.into()];

        param_types.extend(
            method
                .params
                .iter()
                .map(|param| -> Result<BasicMetadataTypeEnum> {
                    Ok(self.get_llvm_type(&param.ty)?.into())
                })
                .collect::<Result<Vec<_>>>()?,
        );

        let fn_type = if let Some(ret_ty) = &method.return_type {
            let return_type = self.get_llvm_type(ret_ty)?;
            match return_type {
                BasicTypeEnum::ArrayType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::FloatType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::IntType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::PointerType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::StructType(t) => t.fn_type(&param_types, false),
                BasicTypeEnum::VectorType(t) => t.fn_type(&param_types, false),
            }
        } else {
            self.context.void_type().fn_type(&param_types, false)
        };

        // Methods are named as Type_method
        let receiver_type_name = match &method.receiver.ty {
            Type::UserDefined(name) => name,
            Type::Reference(inner, _) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name
                } else {
                    bail!("Invalid receiver type for method")
                }
            }
            _ => bail!("Invalid receiver type for method"),
        };

        let method_name = format!("{}_{}", receiver_type_name, method.name);
        let linkage = if method.is_public {
            Linkage::External
        } else {
            Linkage::Private
        };
        let function = self
            .module
            .add_function(&method_name, fn_type, Some(linkage));
        self.functions.insert(method_name, function);

        Ok(())
    }

    /// Compile a function
    fn compile_function(&mut self, func: &FunctionDecl) -> Result<()> {
        let function = self
            .functions
            .get(&func.name)
            .ok_or_else(|| anyhow::anyhow!("Function {} not found", func.name))?
            .clone();

        self.current_function = Some(function);

        // Create entry block
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Create new scope
        self.push_scope();

        // Add parameters to scope
        for (i, param) in func.params.iter().enumerate() {
            let param_value = function
                .get_nth_param(i as u32)
                .ok_or_else(|| anyhow::anyhow!("Parameter {} not found", i))?;

            // Allocate stack space for parameter
            let alloca = self.create_entry_block_alloca(&param.name, &param.ty)?;
            self.builder.build_store(alloca, param_value)?;

            self.add_variable(&param.name, alloca, param.ty.clone(), true)?;
        }

        // Compile function body
        self.compile_block(&func.body)?;

        // Add implicit return if needed
        if func.return_type.is_none() && !self.current_block_has_terminator() {
            self.builder.build_return(None)?;
        }

        // Pop scope
        self.pop_scope();

        // Verify and optimize function
        if function.verify(true) {
            self.pass_manager.run_on(&function);
        } else {
            bail!("Function verification failed: {}", func.name);
        }

        self.current_function = None;
        Ok(())
    }

    /// Compile a method
    fn compile_method(&mut self, method: &MethodDecl) -> Result<()> {
        let receiver_type_name = match &method.receiver.ty {
            Type::UserDefined(name) => name,
            Type::Reference(inner, _) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name
                } else {
                    bail!("Invalid receiver type for method")
                }
            }
            _ => bail!("Invalid receiver type for method"),
        };

        let method_name = format!("{}_{}", receiver_type_name, method.name);
        let function = self
            .functions
            .get(&method_name)
            .ok_or_else(|| anyhow::anyhow!("Method {} not found", method_name))?
            .clone();

        self.current_function = Some(function);

        // Create entry block
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // Create new scope
        self.push_scope();

        // Add receiver to scope
        let receiver_value = function
            .get_nth_param(0)
            .ok_or_else(|| anyhow::anyhow!("Receiver parameter not found"))?;

        let default_name = "self".to_string();
        let receiver_name = method.receiver.name.as_ref().unwrap_or(&default_name);
        let alloca = self.create_entry_block_alloca(receiver_name, &method.receiver.ty)?;
        self.builder.build_store(alloca, receiver_value)?;
        self.add_variable(receiver_name, alloca, method.receiver.ty.clone(), true)?;

        // Add other parameters to scope
        for (i, param) in method.params.iter().enumerate() {
            let param_value = function
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| anyhow::anyhow!("Parameter {} not found", i))?;

            let alloca = self.create_entry_block_alloca(&param.name, &param.ty)?;
            self.builder.build_store(alloca, param_value)?;
            self.add_variable(&param.name, alloca, param.ty.clone(), true)?;
        }

        // Compile method body
        self.compile_block(&method.body)?;

        // Add implicit return if needed
        if method.return_type.is_none() && !self.current_block_has_terminator() {
            self.builder.build_return(None)?;
        }

        // Pop scope
        self.pop_scope();

        // Verify and optimize function
        if function.verify(true) {
            self.pass_manager.run_on(&function);
        } else {
            bail!("Method verification failed: {}", method_name);
        }

        self.current_function = None;
        Ok(())
    }

    /// Compile a block
    fn compile_block(&mut self, block: &Block) -> Result<()> {
        self.push_scope();

        for stmt in &block.statements {
            self.compile_statement(stmt)?;

            // Stop if we hit a terminator
            if self.current_block_has_terminator() {
                break;
            }
        }

        self.pop_scope();
        Ok(())
    }

    /// Compile a statement
    fn compile_statement(&mut self, stmt: &Statement) -> Result<()> {
        match stmt {
            Statement::Let(let_stmt) => self.compile_let_statement(let_stmt),
            Statement::Assignment(assign) => self.compile_assignment(assign),
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                Ok(())
            }
            Statement::Return(ret) => self.compile_return(ret),
            Statement::If(if_stmt) => self.compile_if_statement(if_stmt),
            Statement::While(while_stmt) => self.compile_while_statement(while_stmt),
            Statement::For(for_stmt) => self.compile_for_statement(for_stmt),
            Statement::Block(block) => self.compile_block(block),
        }
    }

    /// Compile a let statement
    fn compile_let_statement(&mut self, let_stmt: &LetStatement) -> Result<()> {
        match &let_stmt.pattern {
            Pattern::Identifier(name, is_mut) => {
                let ty = if let Some(ty) = &let_stmt.ty {
                    ty.clone()
                } else if let Some(init) = &let_stmt.init {
                    // Type inference
                    self.infer_type(init)?
                } else {
                    bail!(
                        "Cannot infer type for variable {} without initializer",
                        name
                    );
                };

                let alloca = self.create_entry_block_alloca(name, &ty)?;

                if let Some(init) = &let_stmt.init {
                    let value = self.compile_expression(init)?;
                    self.builder.build_store(alloca, value)?;
                }

                self.add_variable(name, alloca, ty, *is_mut)?;
            }
            Pattern::Tuple(_patterns) => {
                bail!("Tuple patterns not yet implemented");
            }
            Pattern::Struct(_name, _fields) => {
                bail!("Struct patterns not yet implemented");
            }
        }

        Ok(())
    }

    /// Compile an assignment statement
    fn compile_assignment(&mut self, assign: &AssignStatement) -> Result<()> {
        let value = self.compile_expression(&assign.value)?;

        match &assign.target {
            Expression::Identifier(id) => {
                let symbol = self.get_variable(&id.name)?;
                if !symbol.is_mutable {
                    bail!("Cannot assign to immutable variable {}", id.name);
                }
                self.builder.build_store(symbol.ptr, value)?;
            }
            Expression::Field(field_expr) => {
                let struct_ptr = self.compile_lvalue(&field_expr.object)?;
                let field_ptr = self.get_field_pointer(struct_ptr, &field_expr.field)?;
                self.builder.build_store(field_ptr, value)?;
            }
            Expression::Index(_index_expr) => {
                bail!("Array indexing assignment not yet implemented");
            }
            Expression::Dereference(deref_expr) => {
                let ptr = self.compile_expression(&deref_expr.expr)?;
                if let BasicValueEnum::PointerValue(ptr_val) = ptr {
                    self.builder.build_store(ptr_val, value)?;
                } else {
                    bail!("Cannot dereference non-pointer value");
                }
            }
            _ => bail!("Invalid assignment target"),
        }

        Ok(())
    }

    /// Compile a return statement
    fn compile_return(&mut self, ret: &ReturnStatement) -> Result<()> {
        if let Some(value) = &ret.value {
            let ret_value = self.compile_expression(value)?;
            self.builder.build_return(Some(&ret_value))?;
        } else {
            self.builder.build_return(None)?;
        }
        Ok(())
    }

    /// Compile an if statement
    fn compile_if_statement(&mut self, if_stmt: &IfStatement) -> Result<()> {
        let condition = self.compile_expression(&if_stmt.condition)?;

        let function = self
            .current_function
            .ok_or_else(|| anyhow::anyhow!("No current function"))?;

        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");

        // Build conditional branch
        match condition {
            BasicValueEnum::IntValue(int_val) => {
                self.builder
                    .build_conditional_branch(int_val, then_block, else_block)?;
            }
            _ => bail!("If condition must be a boolean"),
        }

        // Compile then branch
        self.builder.position_at_end(then_block);
        self.compile_block(&if_stmt.then_branch)?;
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(merge_block)?;
        }

        // Compile else branch
        self.builder.position_at_end(else_block);
        if let Some(else_branch) = &if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => {
                    self.compile_block(block)?;
                }
                ElseBranch::If(nested_if) => {
                    self.compile_if_statement(nested_if)?;
                }
            }
        }
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(merge_block)?;
        }

        // Continue at merge block
        self.builder.position_at_end(merge_block);

        Ok(())
    }

    /// Compile a while statement
    fn compile_while_statement(&mut self, while_stmt: &WhileStatement) -> Result<()> {
        let function = self
            .current_function
            .ok_or_else(|| anyhow::anyhow!("No current function"))?;

        let cond_block = self.context.append_basic_block(function, "while.cond");
        let body_block = self.context.append_basic_block(function, "while.body");
        let exit_block = self.context.append_basic_block(function, "while.exit");

        // Jump to condition block
        self.builder.build_unconditional_branch(cond_block)?;

        // Compile condition
        self.builder.position_at_end(cond_block);
        let condition = self.compile_expression(&while_stmt.condition)?;

        match condition {
            BasicValueEnum::IntValue(int_val) => {
                self.builder
                    .build_conditional_branch(int_val, body_block, exit_block)?;
            }
            _ => bail!("While condition must be a boolean"),
        }

        // Compile body
        self.builder.position_at_end(body_block);
        self.compile_block(&while_stmt.body)?;
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(cond_block)?;
        }

        // Continue at exit block
        self.builder.position_at_end(exit_block);

        Ok(())
    }

    /// Compile a for statement
    fn compile_for_statement(&mut self, for_stmt: &ForStatement) -> Result<()> {
        // Create new scope for loop variables
        self.push_scope();

        // Compile initialization
        if let Some(init) = &for_stmt.init {
            self.compile_statement(init)?;
        }

        let function = self
            .current_function
            .ok_or_else(|| anyhow::anyhow!("No current function"))?;

        let cond_block = self.context.append_basic_block(function, "for.cond");
        let body_block = self.context.append_basic_block(function, "for.body");
        let update_block = self.context.append_basic_block(function, "for.update");
        let exit_block = self.context.append_basic_block(function, "for.exit");

        // Jump to condition block
        self.builder.build_unconditional_branch(cond_block)?;

        // Compile condition
        self.builder.position_at_end(cond_block);
        if let Some(condition) = &for_stmt.condition {
            let cond_value = self.compile_expression(condition)?;
            match cond_value {
                BasicValueEnum::IntValue(int_val) => {
                    self.builder
                        .build_conditional_branch(int_val, body_block, exit_block)?;
                }
                _ => bail!("For condition must be a boolean"),
            }
        } else {
            // No condition means infinite loop
            self.builder.build_unconditional_branch(body_block)?;
        }

        // Compile body
        self.builder.position_at_end(body_block);
        self.compile_block(&for_stmt.body)?;
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(update_block)?;
        }

        // Compile update
        self.builder.position_at_end(update_block);
        if let Some(update) = &for_stmt.update {
            self.compile_expression(update)?;
        }
        self.builder.build_unconditional_branch(cond_block)?;

        // Continue at exit block
        self.builder.position_at_end(exit_block);

        // Pop loop scope
        self.pop_scope();

        Ok(())
    }

    /// Compile an expression
    fn compile_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum<'ctx>> {
        match expr {
            Expression::Integer(lit) => self.compile_integer_literal(lit),
            Expression::Float(lit) => self.compile_float_literal(lit),
            Expression::String(lit) => self.compile_string_literal(lit),
            Expression::TemplateString(lit) => self.compile_template_string(lit),
            Expression::Boolean(lit) => self.compile_boolean_literal(lit),
            Expression::Identifier(id) => self.compile_identifier(id),
            Expression::Path(path) => self.compile_path(path),
            Expression::Binary(binary) => self.compile_binary_expr(binary),
            Expression::Unary(unary) => self.compile_unary_expr(unary),
            Expression::Call(call) => self.compile_call_expr(call),
            Expression::MethodCall(method_call) => self.compile_method_call(method_call),
            Expression::Index(index) => self.compile_index_expr(index),
            Expression::Field(field) => self.compile_field_expr(field),
            Expression::Reference(ref_expr) => self.compile_reference_expr(ref_expr),
            Expression::Dereference(deref) => self.compile_dereference_expr(deref),
            Expression::StructLit(struct_lit) => self.compile_struct_literal(struct_lit),
            Expression::EnumVariant(enum_var) => self.compile_enum_variant(enum_var),
            Expression::Array(array) => self.compile_array_expr(array),
            Expression::Tuple(tuple) => self.compile_tuple_expr(tuple),
            Expression::Cast(cast) => self.compile_cast_expr(cast),
            Expression::Assignment(assign) => self.compile_assignment_expr(assign),
        }
    }

    /// Compile integer literal
    fn compile_integer_literal(&self, lit: &IntegerLit) -> Result<BasicValueEnum<'ctx>> {
        let int_type = if let Some(suffix) = &lit.suffix {
            match suffix.as_str() {
                "i8" => self.context.i8_type(),
                "i16" => self.context.i16_type(),
                "i32" => self.context.i32_type(),
                "i64" => self.context.i64_type(),
                "i128" => self.context.i128_type(),
                "u8" => self.context.i8_type(),
                "u16" => self.context.i16_type(),
                "u32" => self.context.i32_type(),
                "u64" => self.context.i64_type(),
                "u128" => self.context.i128_type(),
                _ => self.context.i64_type(), // Default
            }
        } else {
            self.context.i64_type() // Default to i64
        };

        Ok(int_type.const_int(lit.value as u64, false).into())
    }

    /// Compile float literal
    fn compile_float_literal(&self, lit: &FloatLit) -> Result<BasicValueEnum<'ctx>> {
        let float_type = if let Some(suffix) = &lit.suffix {
            match suffix.as_str() {
                "f32" => self.context.f32_type(),
                "f64" => self.context.f64_type(),
                _ => self.context.f64_type(), // Default
            }
        } else {
            self.context.f64_type() // Default to f64
        };

        Ok(float_type.const_float(lit.value).into())
    }

    /// Compile string literal
    fn compile_string_literal(&self, lit: &StringLit) -> Result<BasicValueEnum<'ctx>> {
        let string_const = self.context.const_string(lit.value.as_bytes(), true);
        let global = self.module.add_global(string_const.get_type(), None, "str");
        global.set_initializer(&string_const);
        global.set_constant(true);

        let ptr = unsafe {
            self.builder.build_in_bounds_gep(
                self.context
                    .i8_type()
                    .array_type(lit.value.len() as u32 + 1),
                global.as_pointer_value(),
                &[
                    self.context.i32_type().const_zero(),
                    self.context.i32_type().const_zero(),
                ],
                "str_ptr",
            )?
        };

        Ok(ptr.into())
    }

    /// Compile template string with interpolation
    fn compile_template_string(&mut self, lit: &TemplateStringLit) -> Result<BasicValueEnum<'ctx>> {
        if lit.parts.is_empty() {
            return self.compile_string_literal(&StringLit {
                value: String::new(),
                span: lit.span,
            });
        }

        let mut result: Option<BasicValueEnum> = None;

        for part in &lit.parts {
            let part_str = match part {
                TemplateStringPart::Text(text) => self.compile_string_literal(&StringLit {
                    value: text.clone(),
                    span: lit.span,
                })?,
                TemplateStringPart::Interpolation(expr) => {
                    let value = self.compile_expression(expr)?;
                    self.value_to_string(value)?
                }
            };

            result = Some(if let Some(prev) = result {
                let concat_fn = self
                    .runtime_functions
                    .get("yuni_string_concat")
                    .ok_or_else(|| anyhow::anyhow!("String concat function not found"))?;

                self.builder
                    .build_call(*concat_fn, &[prev.into(), part_str.into()], "concat")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow::anyhow!("String concat should return a value"))?
            } else {
                part_str
            });
        }

        result.ok_or_else(|| anyhow::anyhow!("Empty template string"))
    }

    /// Convert a value to string
    fn value_to_string(&mut self, value: BasicValueEnum<'ctx>) -> Result<BasicValueEnum<'ctx>> {
        match value {
            BasicValueEnum::IntValue(int_val) => {
                // Check if this is a boolean (i1 type)
                if int_val.get_type().get_bit_width() == 1 {
                    let to_string_fn = self
                        .runtime_functions
                        .get("yuni_bool_to_string")
                        .ok_or_else(|| anyhow::anyhow!("bool to string function not found"))?;

                    Ok(self
                        .builder
                        .build_call(*to_string_fn, &[int_val.into()], "to_string")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow::anyhow!("to_string should return a value"))?)
                } else {
                    // Integer types
                    let to_string_fn = self
                        .runtime_functions
                        .get("yuni_i64_to_string")
                        .ok_or_else(|| anyhow::anyhow!("i64 to string function not found"))?;

                    let i64_val = if int_val.get_type().get_bit_width() != 64 {
                        self.builder
                            .build_int_s_extend(int_val, self.context.i64_type(), "extend")?
                    } else {
                        int_val
                    };

                    Ok(self
                        .builder
                        .build_call(*to_string_fn, &[i64_val.into()], "to_string")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow::anyhow!("to_string should return a value"))?)
                }
            }
            BasicValueEnum::FloatValue(float_val) => {
                let to_string_fn = self
                    .runtime_functions
                    .get("yuni_f64_to_string")
                    .ok_or_else(|| anyhow::anyhow!("f64 to string function not found"))?;

                let f64_val = if float_val.get_type() != self.context.f64_type() {
                    self.builder
                        .build_float_cast(float_val, self.context.f64_type(), "cast")?
                } else {
                    float_val
                };

                Ok(self
                    .builder
                    .build_call(*to_string_fn, &[f64_val.into()], "to_string")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| anyhow::anyhow!("to_string should return a value"))?)
            }
            BasicValueEnum::PointerValue(_) => {
                // Already a string
                Ok(value)
            }
            _ => bail!("Cannot convert value to string"),
        }
    }

    /// Compile boolean literal
    fn compile_boolean_literal(&self, lit: &BooleanLit) -> Result<BasicValueEnum<'ctx>> {
        Ok(self
            .context
            .bool_type()
            .const_int(lit.value as u64, false)
            .into())
    }

    /// Compile identifier
    fn compile_identifier(&mut self, id: &Identifier) -> Result<BasicValueEnum<'ctx>> {
        let symbol = self.get_variable(&id.name)?;
        let value =
            self.builder
                .build_load(self.get_llvm_type(&symbol.ty)?, symbol.ptr, &id.name)?;
        Ok(value)
    }

    /// Compile path expression
    fn compile_path(&mut self, path: &PathExpr) -> Result<BasicValueEnum<'ctx>> {
        if path.segments.len() == 1 {
            // Simple identifier
            self.compile_identifier(&Identifier {
                name: path.segments[0].clone(),
                span: path.span,
            })
        } else {
            bail!("Path expressions not yet fully implemented");
        }
    }

    /// Compile binary expression
    fn compile_binary_expr(&mut self, binary: &BinaryExpr) -> Result<BasicValueEnum<'ctx>> {
        let left = self.compile_expression(&binary.left)?;
        let right = self.compile_expression(&binary.right)?;

        match (&left, &right) {
            (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                let result = match binary.op {
                    BinaryOp::Add => self.builder.build_int_add(*lhs, *rhs, "add")?,
                    BinaryOp::Subtract => self.builder.build_int_sub(*lhs, *rhs, "sub")?,
                    BinaryOp::Multiply => self.builder.build_int_mul(*lhs, *rhs, "mul")?,
                    BinaryOp::Divide => self.builder.build_int_signed_div(*lhs, *rhs, "div")?,
                    BinaryOp::Modulo => self.builder.build_int_signed_rem(*lhs, *rhs, "rem")?,
                    BinaryOp::Equal => {
                        self.builder
                            .build_int_compare(IntPredicate::EQ, *lhs, *rhs, "eq")?
                    }
                    BinaryOp::NotEqual => {
                        self.builder
                            .build_int_compare(IntPredicate::NE, *lhs, *rhs, "ne")?
                    }
                    BinaryOp::Less => {
                        self.builder
                            .build_int_compare(IntPredicate::SLT, *lhs, *rhs, "lt")?
                    }
                    BinaryOp::Greater => {
                        self.builder
                            .build_int_compare(IntPredicate::SGT, *lhs, *rhs, "gt")?
                    }
                    BinaryOp::LessEqual => {
                        self.builder
                            .build_int_compare(IntPredicate::SLE, *lhs, *rhs, "le")?
                    }
                    BinaryOp::GreaterEqual => {
                        self.builder
                            .build_int_compare(IntPredicate::SGE, *lhs, *rhs, "ge")?
                    }
                    BinaryOp::And => self.builder.build_and(*lhs, *rhs, "and")?,
                    BinaryOp::Or => self.builder.build_or(*lhs, *rhs, "or")?,
                    _ => bail!("Invalid operator for integers"),
                };
                Ok(result.into())
            }
            (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => match binary.op {
                BinaryOp::Add => Ok(self.builder.build_float_add(*lhs, *rhs, "add")?.into()),
                BinaryOp::Subtract => Ok(self.builder.build_float_sub(*lhs, *rhs, "sub")?.into()),
                BinaryOp::Multiply => Ok(self.builder.build_float_mul(*lhs, *rhs, "mul")?.into()),
                BinaryOp::Divide => Ok(self.builder.build_float_div(*lhs, *rhs, "div")?.into()),
                BinaryOp::Modulo => Ok(self.builder.build_float_rem(*lhs, *rhs, "rem")?.into()),
                BinaryOp::Equal => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, *lhs, *rhs, "eq")?
                    .into()),
                BinaryOp::NotEqual => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::ONE, *lhs, *rhs, "ne")?
                    .into()),
                BinaryOp::Less => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OLT, *lhs, *rhs, "lt")?
                    .into()),
                BinaryOp::Greater => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OGT, *lhs, *rhs, "gt")?
                    .into()),
                BinaryOp::LessEqual => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OLE, *lhs, *rhs, "le")?
                    .into()),
                BinaryOp::GreaterEqual => Ok(self
                    .builder
                    .build_float_compare(FloatPredicate::OGE, *lhs, *rhs, "ge")?
                    .into()),
                _ => bail!("Invalid operator for floats"),
            },
            _ => bail!("Type mismatch in binary expression"),
        }
    }

    /// Compile unary expression
    fn compile_unary_expr(&mut self, unary: &UnaryExpr) -> Result<BasicValueEnum<'ctx>> {
        let operand = self.compile_expression(&unary.operand)?;

        match unary.op {
            UnaryOp::Not => match operand {
                BasicValueEnum::IntValue(int_val) => {
                    Ok(self.builder.build_not(int_val, "not")?.into())
                }
                _ => bail!("Not operator requires boolean operand"),
            },
            UnaryOp::Negate => match operand {
                BasicValueEnum::IntValue(int_val) => {
                    Ok(self.builder.build_int_neg(int_val, "neg")?.into())
                }
                BasicValueEnum::FloatValue(float_val) => {
                    Ok(self.builder.build_float_neg(float_val, "neg")?.into())
                }
                _ => bail!("Negate operator requires numeric operand"),
            },
        }
    }

    /// Compile function call
    fn compile_call_expr(&mut self, call: &CallExpr) -> Result<BasicValueEnum<'ctx>> {
        // Handle built-in functions
        if let Expression::Identifier(id) = &*call.callee {
            if id.name == "println" {
                return self.compile_println_call(call);
            }
        }

        // Regular function call
        let function = match &*call.callee {
            Expression::Identifier(id) => self
                .functions
                .get(&id.name)
                .ok_or_else(|| anyhow::anyhow!("Function {} not found", id.name))?
                .clone(),
            _ => bail!("Complex function calls not yet implemented"),
        };

        let args: Vec<BasicMetadataValueEnum> = call
            .args
            .iter()
            .map(|arg| Ok(self.compile_expression(arg)?.into()))
            .collect::<Result<Vec<_>>>()?;

        let call_value = self.builder.build_call(function, &args, "call")?;

        call_value
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow::anyhow!("Function should return a value"))
    }

    /// Compile println built-in
    fn compile_println_call(&mut self, call: &CallExpr) -> Result<BasicValueEnum<'ctx>> {
        if call.args.is_empty() {
            // Print empty line
            let empty_str = self.compile_string_literal(&StringLit {
                value: String::new(),
                span: Span::dummy(),
            })?;

            let println_fn = self
                .runtime_functions
                .get("yuni_println")
                .ok_or_else(|| anyhow::anyhow!("println function not found"))?;

            self.builder
                .build_call(*println_fn, &[empty_str.into()], "println")?;
        } else {
            // Convert all arguments to strings and concatenate with spaces
            let mut result: Option<BasicValueEnum> = None;

            for (_i, arg) in call.args.iter().enumerate() {
                let value = self.compile_expression(arg)?;
                let str_value = self.value_to_string(value)?;

                result = Some(if let Some(prev) = result {
                    // Add space before concatenating (except for the first argument)
                    let space_str = self.compile_string_literal(&StringLit {
                        value: " ".to_string(),
                        span: Span::dummy(),
                    })?;

                    let concat_fn = self
                        .runtime_functions
                        .get("yuni_string_concat")
                        .ok_or_else(|| anyhow::anyhow!("String concat function not found"))?;

                    // Concatenate previous result with space
                    let with_space = self.builder
                        .build_call(*concat_fn, &[prev.into(), space_str.into()], "concat_space")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow::anyhow!("concat should return a value"))?;

                    // Then concatenate with the current string value
                    self.builder
                        .build_call(*concat_fn, &[with_space.into(), str_value.into()], "concat")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| anyhow::anyhow!("concat should return a value"))?
                } else {
                    str_value
                });
            }

            if let Some(final_str) = result {
                let println_fn = self
                    .runtime_functions
                    .get("yuni_println")
                    .ok_or_else(|| anyhow::anyhow!("println function not found"))?;

                self.builder
                    .build_call(*println_fn, &[final_str.into()], "println")?;
            }
        }

        // Return void
        Ok(self.context.i32_type().const_zero().into())
    }

    /// Compile method call
    fn compile_method_call(
        &mut self,
        method_call: &MethodCallExpr,
    ) -> Result<BasicValueEnum<'ctx>> {
        let receiver = self.compile_expression(&method_call.receiver)?;
        let receiver_type = self.infer_type(&method_call.receiver)?;

        let type_name = match &receiver_type {
            Type::UserDefined(name) => name,
            Type::Reference(inner, _) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name
                } else {
                    bail!("Invalid receiver type for method call")
                }
            }
            _ => bail!("Invalid receiver type for method call"),
        };

        let method_name = format!("{}_{}", type_name, method_call.method);
        let function = self
            .functions
            .get(&method_name)
            .ok_or_else(|| anyhow::anyhow!("Method {} not found", method_name))?
            .clone();

        let mut args: Vec<BasicMetadataValueEnum> = vec![receiver.into()];
        args.extend(
            method_call
                .args
                .iter()
                .map(|arg| -> Result<BasicMetadataValueEnum> {
                    Ok(self.compile_expression(arg)?.into())
                })
                .collect::<Result<Vec<_>>>()?,
        );

        let call_value = self.builder.build_call(function, &args, "method_call")?;

        call_value
            .try_as_basic_value()
            .left()
            .ok_or_else(|| anyhow::anyhow!("Method should return a value"))
    }

    /// Compile index expression
    fn compile_index_expr(&mut self, _index: &IndexExpr) -> Result<BasicValueEnum<'ctx>> {
        bail!("Array indexing not yet implemented")
    }

    /// Compile field access
    fn compile_field_expr(&mut self, field: &FieldExpr) -> Result<BasicValueEnum<'ctx>> {
        let struct_ptr = self.compile_lvalue(&field.object)?;
        let field_ptr = self.get_field_pointer(struct_ptr, &field.field)?;

        let field_type = self.get_field_type(&field.object, &field.field)?;
        let value =
            self.builder
                .build_load(self.get_llvm_type(&field_type)?, field_ptr, &field.field)?;

        Ok(value)
    }

    /// Compile reference expression
    fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> Result<BasicValueEnum<'ctx>> {
        let ptr = self.compile_lvalue(&ref_expr.expr)?;
        Ok(ptr.into())
    }

    /// Compile dereference expression
    fn compile_dereference_expr(
        &mut self,
        deref: &DereferenceExpr,
    ) -> Result<BasicValueEnum<'ctx>> {
        let ptr = self.compile_expression(&deref.expr)?;

        if let BasicValueEnum::PointerValue(ptr_val) = ptr {
            let pointee_type = self.infer_pointee_type(&deref.expr)?;
            let value =
                self.builder
                    .build_load(self.get_llvm_type(&pointee_type)?, ptr_val, "deref")?;
            Ok(value)
        } else {
            bail!("Cannot dereference non-pointer value")
        }
    }

    /// Compile struct literal
    fn compile_struct_literal(&mut self, lit: &StructLit) -> Result<BasicValueEnum<'ctx>> {
        let struct_type = *self
            .types
            .get(&lit.ty)
            .ok_or_else(|| anyhow::anyhow!("Struct type {} not found", lit.ty))?;

        let mut values = vec![];
        for field in &lit.fields {
            let value = self.compile_expression(&field.value)?;
            values.push(value);
        }

        let struct_value = struct_type.const_named_struct(&values);
        Ok(struct_value.into())
    }

    /// Compile enum variant
    fn compile_enum_variant(
        &mut self,
        _enum_var: &EnumVariantExpr,
    ) -> Result<BasicValueEnum<'ctx>> {
        bail!("Enum variants not yet implemented")
    }

    /// Compile array expression
    fn compile_array_expr(&mut self, _array: &ArrayExpr) -> Result<BasicValueEnum<'ctx>> {
        bail!("Arrays not yet implemented")
    }

    /// Compile tuple expression
    fn compile_tuple_expr(&mut self, _tuple: &TupleExpr) -> Result<BasicValueEnum<'ctx>> {
        bail!("Tuples not yet implemented")
    }

    /// Compile cast expression
    fn compile_cast_expr(&mut self, cast: &CastExpr) -> Result<BasicValueEnum<'ctx>> {
        let value = self.compile_expression(&cast.expr)?;
        let target_type = self.get_llvm_type(&cast.ty)?;

        match (value, target_type) {
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int)) => {
                let current_width = int_val.get_type().get_bit_width();
                let target_width = target_int.get_bit_width();

                let result = if current_width < target_width {
                    self.builder
                        .build_int_s_extend(int_val, target_int, "sext")?
                } else if current_width > target_width {
                    self.builder
                        .build_int_truncate(int_val, target_int, "trunc")?
                } else {
                    int_val
                };
                Ok(result.into())
            }
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float)) => {
                let result = self
                    .builder
                    .build_float_cast(float_val, target_float, "fcast")?;
                Ok(result.into())
            }
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::FloatType(target_float)) => {
                let result =
                    self.builder
                        .build_signed_int_to_float(int_val, target_float, "sitofp")?;
                Ok(result.into())
            }
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::IntType(target_int)) => {
                let result = self
                    .builder
                    .build_float_to_signed_int(float_val, target_int, "fptosi")?;
                Ok(result.into())
            }
            _ => bail!("Invalid cast"),
        }
    }

    fn compile_assignment_expr(&mut self, assign: &AssignmentExpr) -> Result<BasicValueEnum<'ctx>> {
        // Get the pointer to the target location
        let target_ptr = self.compile_lvalue(&assign.target)?;

        // Compile the value to assign
        let value = self.compile_expression(&assign.value)?;

        // Store the value
        self.builder.build_store(target_ptr, value)?;

        // Assignment expressions return the assigned value
        Ok(value)
    }

    /// Compile an lvalue (returns pointer)
    fn compile_lvalue(&mut self, expr: &Expression) -> Result<PointerValue<'ctx>> {
        match expr {
            Expression::Identifier(id) => {
                let symbol = self.get_variable(&id.name)?;
                Ok(symbol.ptr)
            }
            Expression::Field(field_expr) => {
                let struct_ptr = self.compile_lvalue(&field_expr.object)?;
                self.get_field_pointer(struct_ptr, &field_expr.field)
            }
            Expression::Dereference(deref) => {
                let ptr = self.compile_expression(&deref.expr)?;
                if let BasicValueEnum::PointerValue(ptr_val) = ptr {
                    Ok(ptr_val)
                } else {
                    bail!("Cannot get lvalue of non-pointer")
                }
            }
            _ => bail!("Expression is not an lvalue"),
        }
    }

    /// Get pointer to struct field
    fn get_field_pointer(
        &mut self,
        _struct_ptr: PointerValue<'ctx>,
        _field_name: &str,
    ) -> Result<PointerValue<'ctx>> {
        // TODO: Look up field index and struct type from type information
        // For now, this is a placeholder that won't work correctly
        bail!("Field access not yet implemented for LLVM 15+ (opaque pointers)")
    }

    /// Get type of struct field
    fn get_field_type(&self, _struct_expr: &Expression, _field_name: &str) -> Result<Type> {
        // TODO: Implement proper type lookup
        bail!("Field type lookup not yet implemented")
    }

    /// Infer type of expression
    fn infer_type(&self, expr: &Expression) -> Result<Type> {
        match expr {
            Expression::Integer(lit) => {
                if let Some(suffix) = &lit.suffix {
                    match suffix.as_str() {
                        "i8" => Ok(Type::I8),
                        "i16" => Ok(Type::I16),
                        "i32" => Ok(Type::I32),
                        "i64" => Ok(Type::I64),
                        "i128" => Ok(Type::I128),
                        "u8" => Ok(Type::U8),
                        "u16" => Ok(Type::U16),
                        "u32" => Ok(Type::U32),
                        "u64" => Ok(Type::U64),
                        "u128" => Ok(Type::U128),
                        _ => Ok(Type::I64),
                    }
                } else {
                    Ok(Type::I64)
                }
            }
            Expression::Float(lit) => {
                if let Some(suffix) = &lit.suffix {
                    match suffix.as_str() {
                        "f32" => Ok(Type::F32),
                        "f64" => Ok(Type::F64),
                        _ => Ok(Type::F64),
                    }
                } else {
                    Ok(Type::F64)
                }
            }
            Expression::String(_) => Ok(Type::String),
            Expression::TemplateString(_) => Ok(Type::String),
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::Identifier(id) => {
                let symbol = self.get_variable(&id.name)?;
                Ok(symbol.ty.clone())
            }
            Expression::Reference(ref_expr) => {
                let inner_type = self.infer_type(&ref_expr.expr)?;
                Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
            }
            _ => bail!("Type inference not implemented for this expression"),
        }
    }

    /// Infer pointee type for dereference
    fn infer_pointee_type(&self, expr: &Expression) -> Result<Type> {
        let ptr_type = self.infer_type(expr)?;
        match ptr_type {
            Type::Reference(inner, _) => Ok(*inner),
            _ => bail!("Cannot dereference non-reference type"),
        }
    }

    /// Get LLVM type from AST type
    fn get_llvm_type(&self, ty: &Type) -> Result<BasicTypeEnum<'ctx>> {
        match ty {
            Type::I8 => Ok(self.context.i8_type().into()),
            Type::I16 => Ok(self.context.i16_type().into()),
            Type::I32 => Ok(self.context.i32_type().into()),
            Type::I64 => Ok(self.context.i64_type().into()),
            Type::I128 => Ok(self.context.i128_type().into()),
            Type::I256 => Ok(self.context.custom_width_int_type(256).into()),
            Type::U8 => Ok(self.context.i8_type().into()),
            Type::U16 => Ok(self.context.i16_type().into()),
            Type::U32 => Ok(self.context.i32_type().into()),
            Type::U64 => Ok(self.context.i64_type().into()),
            Type::U128 => Ok(self.context.i128_type().into()),
            Type::U256 => Ok(self.context.custom_width_int_type(256).into()),
            Type::F8 => bail!("f8 not supported by LLVM"),
            Type::F16 => Ok(self.context.f16_type().into()),
            Type::F32 => Ok(self.context.f32_type().into()),
            Type::F64 => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::String => Ok(self.context.ptr_type(AddressSpace::default()).into()),
            Type::Void => bail!("Void type cannot be used as a value type"),
            Type::Reference(inner, _) => {
                let _inner_type = self.get_llvm_type(inner)?;
                Ok(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Array(elem_ty) => {
                let _elem_type = self.get_llvm_type(elem_ty)?;
                // Dynamic arrays are pointers to elements
                Ok(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Tuple(types) => {
                let field_types: Vec<BasicTypeEnum> = types
                    .iter()
                    .map(|t| self.get_llvm_type(t))
                    .collect::<Result<Vec<_>>>()?;
                Ok(self.context.struct_type(&field_types, false).into())
            }
            Type::Function(_) => bail!("Function types not yet implemented"),
            Type::UserDefined(name) => self
                .types
                .get(name)
                .map(|t| (*t).into())
                .ok_or_else(|| anyhow::anyhow!("Type {} not found", name)),
        }
    }

    /// Create an alloca instruction in the entry block
    fn create_entry_block_alloca(&self, name: &str, ty: &Type) -> Result<PointerValue<'ctx>> {
        let builder = self.context.create_builder();

        let function = self
            .current_function
            .ok_or_else(|| anyhow::anyhow!("No current function"))?;

        let entry = function
            .get_first_basic_block()
            .ok_or_else(|| anyhow::anyhow!("No entry block"))?;

        match entry.get_first_instruction() {
            Some(first_inst) => builder.position_before(&first_inst),
            None => builder.position_at_end(entry),
        }

        let llvm_type = self.get_llvm_type(ty)?;
        Ok(builder.build_alloca(llvm_type, name)?)
    }

    /// Check if current block has a terminator
    fn current_block_has_terminator(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_some()
    }

    /// Push a new scope
    fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Pop the current scope
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Add a variable to the current scope
    fn add_variable(
        &mut self,
        name: &str,
        ptr: PointerValue<'ctx>,
        ty: Type,
        is_mutable: bool,
    ) -> Result<()> {
        let symbol = Symbol {
            ptr: unsafe { std::mem::transmute(ptr) },
            ty,
            is_mutable,
        };

        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(name.to_string(), symbol);
            Ok(())
        } else {
            bail!("No active scope")
        }
    }

    /// Get a variable from any scope
    fn get_variable(&self, name: &str) -> Result<Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                return Ok(symbol.clone());
            }
        }
        bail!("Variable {} not found", name)
    }

    /// Optimize the module
    pub fn optimize(&self, level: OptimizationLevel) {
        // Module-level optimizations are handled during target machine creation
        log::info!("Optimization level: {:?}", level);
    }

    /// Write LLVM IR to file
    pub fn write_llvm_ir(&self, path: &Path) -> Result<()> {
        self.module
            .print_to_file(path)
            .map_err(|e| anyhow::anyhow!("Failed to write LLVM IR: {}", e))
    }

    /// Write object file
    pub fn write_object_file(&self, path: &Path, opt_level: OptimizationLevel) -> Result<()> {
        Target::initialize_native(&Default::default())
            .map_err(|e| anyhow::anyhow!("Failed to initialize native target: {}", e))?;

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| anyhow::anyhow!("Failed to get target: {}", e))?;

        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                opt_level,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .context("Failed to create target machine")?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| anyhow::anyhow!("Failed to write object file: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkwell::context::Context;

    #[test]
    fn test_codegen_creation() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test_module");
        assert_eq!(codegen.module.get_name().to_str().unwrap(), "test_module");
    }
}
