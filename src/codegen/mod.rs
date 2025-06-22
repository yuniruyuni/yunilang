//! Code generation module for the Yuni language.
//!
//! This module is responsible for generating LLVM IR from the AST.

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::builder::Builder;
use inkwell::context::Context as LLVMContext;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetMachine};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, StructType};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate, OptimizationLevel};
use std::collections::HashMap;
use std::path::Path;

/// Symbol table entry for tracking variables and their types
#[derive(Debug, Clone)]
struct Symbol<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: Type,
    is_mutable: bool,
}

/// Scope for managing variable lifetimes
struct Scope<'ctx> {
    symbols: HashMap<String, Symbol<'ctx>>,
}

impl<'ctx> Scope<'ctx> {
    fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }
}

/// 構造体のフィールド情報
#[derive(Debug, Clone)]
struct StructInfo {
    /// フィールド名からインデックスへのマッピング
    field_indices: HashMap<String, u32>,
    /// フィールドの型情報（AST型を保持）
    field_types: Vec<Type>,
}

/// Main code generator structure
pub struct CodeGenerator<'ctx> {
    context: &'ctx LLVMContext,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    pass_manager: PassManager<FunctionValue<'ctx>>,

    // Symbol tables and scopes
    scopes: Vec<Scope<'ctx>>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    types: HashMap<String, StructType<'ctx>>,
    
    // 構造体のフィールド情報
    struct_info: HashMap<String, StructInfo>,

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
            struct_info: HashMap::new(),
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
    
    /// Get the LLVM module
    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }

    /// Compile a complete program
    pub fn compile_program(&mut self, program: &Program) -> YuniResult<()> {
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
    fn declare_type(&mut self, type_def: &TypeDef) -> YuniResult<()> {
        match type_def {
            TypeDef::Struct(struct_def) => {
                let field_types: Vec<BasicTypeEnum> = struct_def
                    .fields
                    .iter()
                    .map(|field| self.get_llvm_type(&field.ty))
                    .collect::<YuniResult<Vec<_>>>()?;

                let struct_type = self.context.struct_type(&field_types, false);
                self.types.insert(struct_def.name.clone(), struct_type);
                
                // フィールド情報を保存
                let mut field_indices = HashMap::new();
                let mut ast_field_types = Vec::new();
                for (index, field) in struct_def.fields.iter().enumerate() {
                    field_indices.insert(field.name.clone(), index as u32);
                    ast_field_types.push(field.ty.clone());
                }
                
                self.struct_info.insert(struct_def.name.clone(), StructInfo {
                    field_indices,
                    field_types: ast_field_types,
                });
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
                        .collect::<YuniResult<Vec<_>>>()?;

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
    fn declare_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|param| Ok(self.get_llvm_type(&param.ty)?.into()))
            .collect::<YuniResult<Vec<_>>>()?;

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
    fn declare_method(&mut self, method: &MethodDecl) -> YuniResult<()> {
        let mut param_types: Vec<BasicMetadataTypeEnum> =
            vec![self.get_llvm_type(&method.receiver.ty)?.into()];

        param_types.extend(
            method
                .params
                .iter()
                .map(|param| -> YuniResult<BasicMetadataTypeEnum> {
                    Ok(self.get_llvm_type(&param.ty)?.into())
                })
                .collect::<YuniResult<Vec<_>>>()?,
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
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Invalid receiver type for method".to_string(),
                        span: Span::dummy(),
                    }))
                }
            }
            _ => return Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Invalid receiver type for method".to_string(),
                span: Span::dummy(),
            })),
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
    fn compile_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        let function = self
            .functions
            .get(&func.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Function {} not found", func.name) }))?
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
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Parameter {} not found", i) }))?;

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
            return Err(YuniError::Codegen(CodegenError::Internal {
                message: format!("Function verification failed: {}", func.name),
            }));
        }

        self.current_function = None;
        Ok(())
    }

    /// Compile a method
    fn compile_method(&mut self, method: &MethodDecl) -> YuniResult<()> {
        let receiver_type_name = match &method.receiver.ty {
            Type::UserDefined(name) => name,
            Type::Reference(inner, _) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name
                } else {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Invalid receiver type for method".to_string(),
                        span: Span::dummy(),
                    }))
                }
            }
            _ => return Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Invalid receiver type for method".to_string(),
                span: Span::dummy(),
            })),
        };

        let method_name = format!("{}_{}", receiver_type_name, method.name);
        let function = self
            .functions
            .get(&method_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Method {} not found", method_name) }))?
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
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "Receiver parameter not found".to_string() }))?;

        let default_name = "self".to_string();
        let receiver_name = method.receiver.name.as_ref().unwrap_or(&default_name);
        let alloca = self.create_entry_block_alloca(receiver_name, &method.receiver.ty)?;
        self.builder.build_store(alloca, receiver_value)?;
        self.add_variable(receiver_name, alloca, method.receiver.ty.clone(), true)?;

        // Add other parameters to scope
        for (i, param) in method.params.iter().enumerate() {
            let param_value = function
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Parameter {} not found", i) }))?;

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
            return Err(YuniError::Codegen(CodegenError::Internal {
                message: format!("Method verification failed: {}", method_name),
            }));
        }

        self.current_function = None;
        Ok(())
    }

    /// Compile a block
    fn compile_block(&mut self, block: &Block) -> YuniResult<()> {
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
    fn compile_statement(&mut self, stmt: &Statement) -> YuniResult<()> {
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
    fn compile_let_statement(&mut self, let_stmt: &LetStatement) -> YuniResult<()> {
        match &let_stmt.pattern {
            Pattern::Identifier(name, is_mut) => {
                let ty = if let Some(ty) = &let_stmt.ty {
                    ty.clone()
                } else if let Some(init) = &let_stmt.init {
                    // Type inference
                    self.infer_type(init)?
                } else {
                    return Err(YuniError::Codegen(CodegenError::Internal {
                        message: format!("Cannot infer type for variable {} without initializer", name)
                    }));
                };

                let alloca = self.create_entry_block_alloca(name, &ty)?;

                if let Some(init) = &let_stmt.init {
                    let value = self.compile_expression(init)?;
                    self.builder.build_store(alloca, value)?;
                }

                self.add_variable(name, alloca, ty, *is_mut)?;
            }
            Pattern::Tuple(_patterns) => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Tuple patterns not yet implemented".to_string(), span: Span::dummy() }));
            }
            Pattern::Struct(_name, _fields) => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Struct patterns not yet implemented".to_string(), span: Span::dummy() }));
            }
        }

        Ok(())
    }

    /// Compile an assignment statement
    fn compile_assignment(&mut self, assign: &AssignStatement) -> YuniResult<()> {
        let value = self.compile_expression(&assign.value)?;

        match &assign.target {
            Expression::Identifier(id) => {
                let symbol = self.get_variable(&id.name)?;
                if !symbol.is_mutable {
                    return Err(YuniError::Codegen(CodegenError::Internal {
                        message: format!("Cannot assign to immutable variable {}", id.name)
                    }));
                }
                self.builder.build_store(symbol.ptr, value)?;
            }
            Expression::Field(field_expr) => {
                let struct_ptr = self.compile_lvalue(&field_expr.object)?;
                let field_ptr = self.get_field_pointer(struct_ptr, field_expr)?;
                self.builder.build_store(field_ptr, value)?;
            }
            Expression::Index(_index_expr) => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Array indexing assignment not yet implemented".to_string(), span: Span::dummy() }));
            }
            Expression::Dereference(deref_expr) => {
                let ptr = self.compile_expression(&deref_expr.expr)?;
                if let BasicValueEnum::PointerValue(ptr_val) = ptr {
                    self.builder.build_store(ptr_val, value)?;
                } else {
                    return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Cannot dereference non-pointer value".to_string(), span: Span::dummy() }));
                }
            }
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Invalid assignment target".to_string(), span: Span::dummy() })),
        }

        Ok(())
    }

    /// Compile a return statement
    fn compile_return(&mut self, ret: &ReturnStatement) -> YuniResult<()> {
        if let Some(value) = &ret.value {
            let ret_value = self.compile_expression(value)?;
            self.builder.build_return(Some(&ret_value))?;
        } else {
            self.builder.build_return(None)?;
        }
        Ok(())
    }

    /// Compile an if statement
    fn compile_if_statement(&mut self, if_stmt: &IfStatement) -> YuniResult<()> {
        let condition = self.compile_expression(&if_stmt.condition)?;

        let function = self
            .current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "No current function".to_string() }))?;

        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "merge");

        // Build conditional branch
        match condition {
            BasicValueEnum::IntValue(int_val) => {
                self.builder
                    .build_conditional_branch(int_val, then_block, else_block)?;
            }
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "If condition must be a boolean".to_string(), span: Span::dummy() })),
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
    fn compile_while_statement(&mut self, while_stmt: &WhileStatement) -> YuniResult<()> {
        let function = self
            .current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "No current function".to_string() }))?;

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
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "While condition must be a boolean".to_string(), span: Span::dummy() })),
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
    fn compile_for_statement(&mut self, for_stmt: &ForStatement) -> YuniResult<()> {
        // Create new scope for loop variables
        self.push_scope();

        // Compile initialization
        if let Some(init) = &for_stmt.init {
            self.compile_statement(init)?;
        }

        let function = self
            .current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "No current function".to_string() }))?;

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
                _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "For condition must be a boolean".to_string(), span: Span::dummy() })),
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
    fn compile_expression(&mut self, expr: &Expression) -> YuniResult<BasicValueEnum<'ctx>> {
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
    fn compile_integer_literal(&self, lit: &IntegerLit) -> YuniResult<BasicValueEnum<'ctx>> {
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
    fn compile_float_literal(&self, lit: &FloatLit) -> YuniResult<BasicValueEnum<'ctx>> {
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
    fn compile_string_literal(&self, lit: &StringLit) -> YuniResult<BasicValueEnum<'ctx>> {
        let string_const = self.context.const_string(lit.value.as_bytes(), true);
        let global = self.module.add_global(string_const.get_type(), None, "str");
        global.set_initializer(&string_const);
        global.set_constant(true);

        // 文字列配列の要素へのポインタを安全に取得
        // この操作は安全であるため、unsafeブロックは必要最小限に留める
        let array_type = self.context
            .i8_type()
            .array_type(lit.value.len() as u32 + 1);
        let indices = [
            self.context.i32_type().const_zero(),
            self.context.i32_type().const_zero(),
        ];
        
        let ptr = unsafe {
            // SAFETY: グローバル文字列定数への境界内GEP操作は安全
            // インデックスは配列の境界内を指しており、型も適切に設定されている
            self.builder.build_in_bounds_gep(
                array_type,
                global.as_pointer_value(),
                &indices,
                "str_ptr",
            )?
        };

        Ok(ptr.into())
    }

    /// Compile template string with interpolation
    fn compile_template_string(&mut self, lit: &TemplateStringLit) -> YuniResult<BasicValueEnum<'ctx>> {
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
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "String concat function not found".to_string() }))?;

                self.builder
                    .build_call(*concat_fn, &[prev.into(), part_str.into()], "concat")?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "String concat should return a value".to_string() }))?
            } else {
                part_str
            });
        }

        result.ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "Empty template string".to_string() }))
    }

    /// Convert a value to string
    fn value_to_string(&mut self, value: BasicValueEnum<'ctx>) -> YuniResult<BasicValueEnum<'ctx>> {
        match value {
            BasicValueEnum::IntValue(int_val) => {
                // Check if this is a boolean (i1 type)
                if int_val.get_type().get_bit_width() == 1 {
                    let to_string_fn = self
                        .runtime_functions
                        .get("yuni_bool_to_string")
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "bool to string function not found".to_string() }))?;

                    Ok(self
                        .builder
                        .build_call(*to_string_fn, &[int_val.into()], "to_string")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "to_string should return a value".to_string() }))?)
                } else {
                    // Integer types
                    let to_string_fn = self
                        .runtime_functions
                        .get("yuni_i64_to_string")
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "i64 to string function not found".to_string() }))?;

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
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "to_string should return a value".to_string() }))?)
                }
            }
            BasicValueEnum::FloatValue(float_val) => {
                let to_string_fn = self
                    .runtime_functions
                    .get("yuni_f64_to_string")
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "f64 to string function not found".to_string() }))?;

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
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "to_string should return a value".to_string() }))?)
            }
            BasicValueEnum::PointerValue(_) => {
                // Already a string
                Ok(value)
            }
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Cannot convert value to string".to_string(), span: Span::dummy() })),
        }
    }

    /// Compile boolean literal
    fn compile_boolean_literal(&self, lit: &BooleanLit) -> YuniResult<BasicValueEnum<'ctx>> {
        Ok(self
            .context
            .bool_type()
            .const_int(lit.value as u64, false)
            .into())
    }

    /// Compile identifier
    fn compile_identifier(&mut self, id: &Identifier) -> YuniResult<BasicValueEnum<'ctx>> {
        let symbol = self.get_variable(&id.name)?;
        let value =
            self.builder
                .build_load(self.get_llvm_type(&symbol.ty)?, symbol.ptr, &id.name)?;
        Ok(value)
    }

    /// Compile path expression
    fn compile_path(&mut self, path: &PathExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        if path.segments.len() == 1 {
            // Simple identifier
            self.compile_identifier(&Identifier {
                name: path.segments[0].clone(),
                span: path.span,
            })
        } else {
            return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Path expressions not yet fully implemented".to_string(), span: Span::dummy() }));
        }
    }

    /// Compile binary expression
    fn compile_binary_expr(&mut self, binary: &BinaryExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
                    _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Invalid operator for integers".to_string(), span: Span::dummy() })),
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
                _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Invalid operator for floats".to_string(), span: Span::dummy() })),
            },
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Type mismatch in binary expression".to_string(), span: Span::dummy() })),
        }
    }

    /// Compile unary expression
    fn compile_unary_expr(&mut self, unary: &UnaryExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let operand = self.compile_expression(&unary.operand)?;

        match unary.op {
            UnaryOp::Not => match operand {
                BasicValueEnum::IntValue(int_val) => {
                    Ok(self.builder.build_not(int_val, "not")?.into())
                }
                _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Not operator requires boolean operand".to_string(), span: Span::dummy() })),
            },
            UnaryOp::Negate => match operand {
                BasicValueEnum::IntValue(int_val) => {
                    Ok(self.builder.build_int_neg(int_val, "neg")?.into())
                }
                BasicValueEnum::FloatValue(float_val) => {
                    Ok(self.builder.build_float_neg(float_val, "neg")?.into())
                }
                _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Negate operator requires numeric operand".to_string(), span: Span::dummy() })),
            },
        }
    }

    /// Compile function call
    fn compile_call_expr(&mut self, call: &CallExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Function {} not found", id.name) }))?
                .clone(),
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Complex function calls not yet implemented".to_string(), span: Span::dummy() })),
        };

        let args: Vec<BasicMetadataValueEnum> = call
            .args
            .iter()
            .map(|arg| Ok(self.compile_expression(arg)?.into()))
            .collect::<YuniResult<Vec<_>>>()?;

        let call_value = self.builder.build_call(function, &args, "call")?;

        call_value
            .try_as_basic_value()
            .left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "Function should return a value".to_string() }))
    }

    /// Compile println built-in
    fn compile_println_call(&mut self, call: &CallExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        if call.args.is_empty() {
            // Print empty line
            let empty_str = self.compile_string_literal(&StringLit {
                value: String::new(),
                span: Span::dummy(),
            })?;

            let println_fn = self
                .runtime_functions
                .get("yuni_println")
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "println function not found".to_string() }))?;

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
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "String concat function not found".to_string() }))?;

                    // Concatenate previous result with space
                    let with_space = self.builder
                        .build_call(*concat_fn, &[prev.into(), space_str.into()], "concat_space")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "concat should return a value".to_string() }))?;

                    // Then concatenate with the current string value
                    self.builder
                        .build_call(*concat_fn, &[with_space.into(), str_value.into()], "concat")?
                        .try_as_basic_value()
                        .left()
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "concat should return a value".to_string() }))?
                } else {
                    str_value
                });
            }

            if let Some(final_str) = result {
                let println_fn = self
                    .runtime_functions
                    .get("yuni_println")
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "println function not found".to_string() }))?;

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
    ) -> YuniResult<BasicValueEnum<'ctx>> {
        let receiver = self.compile_expression(&method_call.receiver)?;
        let receiver_type = self.infer_type(&method_call.receiver)?;

        let type_name = match &receiver_type {
            Type::UserDefined(name) => name,
            Type::Reference(inner, _) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name
                } else {
                    return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Invalid receiver type for method call".to_string(), span: Span::dummy() }))
                }
            }
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Invalid receiver type for method call".to_string(), span: Span::dummy() })),
        };

        let method_name = format!("{}_{}", type_name, method_call.method);
        let function = self
            .functions
            .get(&method_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Method {} not found", method_name) }))?
            .clone();

        let mut args: Vec<BasicMetadataValueEnum> = vec![receiver.into()];
        args.extend(
            method_call
                .args
                .iter()
                .map(|arg| -> YuniResult<BasicMetadataValueEnum> {
                    Ok(self.compile_expression(arg)?.into())
                })
                .collect::<YuniResult<Vec<_>>>()?,
        );

        let call_value = self.builder.build_call(function, &args, "method_call")?;

        call_value
            .try_as_basic_value()
            .left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "Method should return a value".to_string() }))
    }

    /// Compile index expression
    fn compile_index_expr(&mut self, _index: &IndexExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Array indexing not yet implemented".to_string(), span: Span::dummy() }))
    }

    /// Compile field access
    fn compile_field_expr(&mut self, field: &FieldExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let struct_ptr = self.compile_lvalue(&field.object)?;
        let field_ptr = self.get_field_pointer(struct_ptr, field)?;

        let field_type = self.get_field_type(&field.object, &field.field)?;
        let value =
            self.builder
                .build_load(self.get_llvm_type(&field_type)?, field_ptr, &field.field)?;

        Ok(value)
    }

    /// Compile reference expression
    fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let ptr = self.compile_lvalue(&ref_expr.expr)?;
        Ok(ptr.into())
    }

    /// Compile dereference expression
    fn compile_dereference_expr(
        &mut self,
        deref: &DereferenceExpr,
    ) -> YuniResult<BasicValueEnum<'ctx>> {
        let ptr = self.compile_expression(&deref.expr)?;

        if let BasicValueEnum::PointerValue(ptr_val) = ptr {
            let pointee_type = self.infer_pointee_type(&deref.expr)?;
            let value =
                self.builder
                    .build_load(self.get_llvm_type(&pointee_type)?, ptr_val, "deref")?;
            Ok(value)
        } else {
            return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Cannot dereference non-pointer value".to_string(), span: Span::dummy() }))
        }
    }

    /// Compile struct literal
    fn compile_struct_literal(&mut self, lit: &StructLit) -> YuniResult<BasicValueEnum<'ctx>> {
        let struct_type = *self
            .types
            .get(&lit.ty)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: format!("Struct type {} not found", lit.ty) }))?;

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
    ) -> YuniResult<BasicValueEnum<'ctx>> {
        return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Enum variants not yet implemented".to_string(), span: Span::dummy() }))
    }

    /// Compile array expression
    fn compile_array_expr(&mut self, _array: &ArrayExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Arrays not yet implemented".to_string(), span: Span::dummy() }))
    }

    /// Compile tuple expression
    fn compile_tuple_expr(&mut self, _tuple: &TupleExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Tuples not yet implemented".to_string(), span: Span::dummy() }))
    }

    /// Compile cast expression
    fn compile_cast_expr(&mut self, cast: &CastExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Invalid cast".to_string(), span: Span::dummy() })),
        }
    }

    fn compile_assignment_expr(&mut self, assign: &AssignmentExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
    fn compile_lvalue(&mut self, expr: &Expression) -> YuniResult<PointerValue<'ctx>> {
        match expr {
            Expression::Identifier(id) => {
                let symbol = self.get_variable(&id.name)?;
                Ok(symbol.ptr)
            }
            Expression::Field(field_expr) => {
                let struct_ptr = self.compile_lvalue(&field_expr.object)?;
                self.get_field_pointer(struct_ptr, field_expr)
            }
            Expression::Dereference(deref) => {
                let ptr = self.compile_expression(&deref.expr)?;
                if let BasicValueEnum::PointerValue(ptr_val) = ptr {
                    Ok(ptr_val)
                } else {
                    return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Cannot get lvalue of non-pointer".to_string(), span: Span::dummy() }))
                }
            }
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Expression is not an lvalue".to_string(), span: Span::dummy() })),
        }
    }

    /// Get pointer to struct field
    fn get_field_pointer(
        &mut self,
        struct_ptr: PointerValue<'ctx>,
        field_expr: &FieldExpr,
    ) -> YuniResult<PointerValue<'ctx>> {
        // 構造体の型を推論
        let struct_type = self.infer_type(&field_expr.object)?;
        
        // 参照の場合はデリファレンス
        let actual_struct_type = match &struct_type {
            Type::Reference(inner, _) => inner.as_ref(),
            _ => &struct_type,
        };
        
        let struct_name = match actual_struct_type {
            Type::UserDefined(name) => name,
            _ => return Err(YuniError::Codegen(CodegenError::TypeError { 
                expected: "struct type".to_string(), 
                actual: format!("{:?}", actual_struct_type),
                span: field_expr.span 
            })),
        };
        
        // 構造体情報を取得
        let struct_info = self.struct_info.get(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined { 
                name: struct_name.clone(), 
                span: field_expr.span 
            }))?;
        
        // フィールドインデックスを取得
        let field_index = *struct_info.field_indices.get(&field_expr.field)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined { 
                name: format!("{}.{}", struct_name, field_expr.field), 
                span: field_expr.span 
            }))?;
        
        // LLVM構造体型を取得
        let llvm_struct_type = self.types.get(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined { 
                name: struct_name.clone(), 
                span: field_expr.span 
            }))?;
        
        // LLVM 18のオペークポインタに対応したGEP命令を生成
        // build_struct_gepは使用せず、明示的に型を指定してGEPを使用
        let indices = [
            self.context.i32_type().const_int(0, false),
            self.context.i32_type().const_int(field_index as u64, false),
        ];
        
        let field_ptr = unsafe {
            self.builder.build_gep(
                *llvm_struct_type,
                struct_ptr,
                &indices,
                &format!("{}_field_{}_ptr", struct_name, field_expr.field),
            )
        }
        .map_err(|_| YuniError::Codegen(CodegenError::CompilationFailed { 
            message: format!("Failed to build GEP for field {}.{}", struct_name, field_expr.field),
            span: field_expr.span 
        }))?;
        
        Ok(field_ptr)
    }

    /// Get type of struct field
    fn get_field_type(&self, struct_expr: &Expression, field_name: &str) -> YuniResult<Type> {
        // 構造体の型を推論
        let struct_type = self.infer_type(struct_expr)?;
        
        // 参照の場合はデリファレンス
        let actual_struct_type = match &struct_type {
            Type::Reference(inner, _) => inner.as_ref(),
            _ => &struct_type,
        };
        
        let struct_name = match actual_struct_type {
            Type::UserDefined(name) => name,
            _ => return Err(YuniError::Codegen(CodegenError::TypeError { 
                expected: "struct type".to_string(), 
                actual: format!("{:?}", actual_struct_type),
                span: Span::dummy() 
            })),
        };
        
        // 構造体情報を取得
        let struct_info = self.struct_info.get(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined { 
                name: struct_name.clone(), 
                span: Span::dummy() 
            }))?;
        
        // フィールドインデックスを取得
        let field_index = *struct_info.field_indices.get(field_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined { 
                name: format!("{}.{}", struct_name, field_name), 
                span: Span::dummy() 
            }))?;
        
        // フィールドの型を返す
        Ok(struct_info.field_types[field_index as usize].clone())
    }

    /// Infer type of expression
    fn infer_type(&self, expr: &Expression) -> YuniResult<Type> {
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
            Expression::Field(field_expr) => {
                self.get_field_type(&field_expr.object, &field_expr.field)
            }
            Expression::StructLit(struct_lit) => {
                Ok(Type::UserDefined(struct_lit.ty.clone()))
            }
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Type inference not implemented for this expression".to_string(), span: Span::dummy() })),
        }
    }

    /// Infer pointee type for dereference
    fn infer_pointee_type(&self, expr: &Expression) -> YuniResult<Type> {
        let ptr_type = self.infer_type(expr)?;
        match ptr_type {
            Type::Reference(inner, _) => Ok(*inner),
            _ => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Cannot dereference non-reference type".to_string(), span: Span::dummy() })),
        }
    }

    /// Get LLVM type from AST type
    fn get_llvm_type(&self, ty: &Type) -> YuniResult<BasicTypeEnum<'ctx>> {
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
            Type::F8 => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "f8 not supported by LLVM".to_string(), span: Span::dummy() })),
            Type::F16 => Ok(self.context.f16_type().into()),
            Type::F32 => Ok(self.context.f32_type().into()),
            Type::F64 => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::String => Ok(self.context.ptr_type(AddressSpace::default()).into()),
            Type::Void => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Void type cannot be used as a value type".to_string(), span: Span::dummy() })),
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
                    .collect::<YuniResult<Vec<_>>>()?;
                Ok(self.context.struct_type(&field_types, false).into())
            }
            Type::Function(_) => return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "Function types not yet implemented".to_string(), span: Span::dummy() })),
            Type::UserDefined(name) => self
                .types
                .get(name)
                .map(|t| (*t).into())
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                    message: format!("Type {} not found", name) 
                })),
        }
    }

    /// Create an alloca instruction in the entry block
    fn create_entry_block_alloca(&self, name: &str, ty: &Type) -> YuniResult<PointerValue<'ctx>> {
        let builder = self.context.create_builder();

        let function = self
            .current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "No current function".to_string() }))?;

        let entry = function
            .get_first_basic_block()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { message: "No entry block".to_string() }))?;

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
    ) -> YuniResult<()> {
        let symbol = Symbol {
            ptr,
            ty,
            is_mutable,
        };

        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(name.to_string(), symbol);
            Ok(())
        } else {
            return Err(YuniError::Codegen(CodegenError::Unimplemented { feature: "No active scope".to_string(), span: Span::dummy() }))
        }
    }

    /// Get a variable from any scope
    fn get_variable(&self, name: &str) -> YuniResult<Symbol<'ctx>> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.symbols.get(name) {
                return Ok(symbol.clone());
            }
        }
        Err(YuniError::Codegen(CodegenError::Internal {
            message: format!("Variable {} not found", name)
        }))
    }

    /// Optimize the module
    pub fn optimize(&self, level: OptimizationLevel) {
        // Module-level optimizations are handled during target machine creation
        log::info!("Optimization level: {:?}", level);
    }

    /// Write LLVM IR to file
    pub fn write_llvm_ir(&self, path: &Path) -> YuniResult<()> {
        self.module
            .print_to_file(path)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to write LLVM IR: {}", e)
            }))
    }

    /// Write object file
    pub fn write_object_file(&self, path: &Path, opt_level: OptimizationLevel) -> YuniResult<()> {
        Target::initialize_native(&Default::default())
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to initialize native target: {}", e)
            }))?;

        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to get target: {}", e)
            }))?;

        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                opt_level,
                RelocMode::PIC,
                CodeModel::Default,
            )
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "Failed to create target machine".to_string()
            }))?;

        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to write object file: {}", e)
            }))
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
