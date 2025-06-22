//! メインコード生成器

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::builder::Builder;
use inkwell::context::Context as LLVMContext;
use inkwell::module::{Linkage, Module};
use inkwell::passes::PassManager;
use inkwell::targets::{CodeModel, FileType, RelocMode, Target, TargetMachine};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, FunctionValue, PointerValue};
use inkwell::{AddressSpace, OptimizationLevel};
use std::collections::HashMap;
use std::path::Path;

use super::runtime::RuntimeManager;
use super::symbol_table::{ScopeManager, StructInfo, Symbol};
use super::types::TypeManager;

/// メインコード生成器構造体
pub struct CodeGenerator<'ctx> {
    pub context: &'ctx LLVMContext,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub pass_manager: PassManager<FunctionValue<'ctx>>,

    // マネージャー
    pub scope_manager: ScopeManager<'ctx>,
    pub type_manager: TypeManager<'ctx>,
    pub runtime_manager: RuntimeManager<'ctx>,
    
    // 関数テーブル
    pub functions: HashMap<String, FunctionValue<'ctx>>,
    
    // 構造体のフィールド情報
    pub struct_info: HashMap<String, StructInfo>,

    // 現在コンパイル中の関数
    pub current_function: Option<FunctionValue<'ctx>>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx LLVMContext, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let pass_manager = PassManager::create(&module);

        // パスマネージャを初期化
        pass_manager.initialize();

        let type_manager = TypeManager::new(context);
        let mut runtime_manager = RuntimeManager::new(context);
        
        // ランタイム関数を初期化
        runtime_manager.initialize(&module);

        Self {
            context,
            module,
            builder,
            pass_manager,
            scope_manager: ScopeManager::new(),
            type_manager,
            runtime_manager,
            functions: HashMap::new(),
            struct_info: HashMap::new(),
            current_function: None,
        }
    }
    
    /// LLVMモジュールを取得
    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }

    /// プログラム全体をコンパイル
    pub fn compile_program(&mut self, program: &Program) -> YuniResult<()> {
        // 第一パス: すべての型を宣言
        for item in &program.items {
            if let Item::TypeDef(type_def) = item {
                self.declare_type(type_def)?;
            }
        }

        // 第二パス: すべての関数を宣言
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

        // 第三パス: 関数本体をコンパイル
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

    /// 型を宣言（構造体または列挙型）
    fn declare_type(&mut self, type_def: &TypeDef) -> YuniResult<()> {
        match type_def {
            TypeDef::Struct(struct_def) => {
                let field_types: Vec<BasicTypeEnum> = struct_def
                    .fields
                    .iter()
                    .map(|field| self.type_manager.ast_type_to_llvm(&field.ty))
                    .collect::<YuniResult<Vec<_>>>()?;

                let struct_type = self.context.struct_type(&field_types, false);
                self.type_manager.register_struct(struct_def.name.clone(), struct_type);
                
                // フィールド情報を保存
                let mut struct_info = StructInfo::new();
                for (index, field) in struct_def.fields.iter().enumerate() {
                    struct_info.add_field(field.name.clone(), field.ty.clone());
                }
                self.struct_info.insert(struct_def.name.clone(), struct_info);
            }
            TypeDef::Enum(_enum_def) => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Enum types not yet implemented".to_string(),
                    span: Span::dummy(),
                }));
            }
        }
        Ok(())
    }

    /// 関数を宣言
    fn declare_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        let param_types: Vec<BasicMetadataTypeEnum> = func
            .params
            .iter()
            .map(|param| self.type_manager.ast_type_to_metadata(&param.ty))
            .collect::<YuniResult<Vec<_>>>()?;

        let return_type = func.return_type.as_ref().unwrap_or(&Type::Void);
        let fn_type = self.type_manager.create_function_type(&[], return_type, false)?;

        let function = self.module.add_function(&func.name, fn_type, None);
        self.functions.insert(func.name.clone(), function);

        Ok(())
    }

    /// メソッドを宣言
    fn declare_method(&mut self, method: &MethodDecl) -> YuniResult<()> {
        let receiver_type_name = match &method.receiver.ty {
            Type::UserDefined(name) => name,
            Type::Reference(referent, _) => {
                if let Type::UserDefined(name) = referent.as_ref() {
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
        
        // レシーバーとパラメータの型を集める
        let mut param_types = vec![self.type_manager.ast_type_to_metadata(&method.receiver.ty)?];
        for param in &method.params {
            param_types.push(self.type_manager.ast_type_to_metadata(&param.ty)?);
        }

        let return_type = method.return_type.as_ref().unwrap_or(&Type::Void);
        let fn_type = self.type_manager.create_function_type(
            &method.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
            return_type,
            false,
        )?;

        let function = self.module.add_function(&method_name, fn_type, None);
        self.functions.insert(method_name, function);

        Ok(())
    }

    /// 関数をコンパイル
    fn compile_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        let function = self.functions.get(&func.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: format!("Function {} not found", func.name) 
            }))?
            .clone();

        self.current_function = Some(function);

        // エントリブロックを作成
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // 新しいスコープを作成
        self.scope_manager.push_scope();

        // パラメータをスコープに追加
        for (i, param) in func.params.iter().enumerate() {
            let param_value = function
                .get_nth_param(i as u32)
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                    message: format!("Parameter {} not found", i) 
                }))?;

            // パラメータ用のスタック領域を確保
            let alloca = self.create_entry_block_alloca(&param.name, &param.ty)?;
            self.builder.build_store(alloca, param_value)?;

            self.add_variable(&param.name, alloca, param.ty.clone(), true)?;
        }

        // 関数本体をコンパイル
        self.compile_block(&func.body)?;

        // 必要に応じて暗黙のreturnを追加
        if func.return_type.is_none() && !self.current_block_has_terminator() {
            self.builder.build_return(None)?;
        }

        // スコープを終了
        self.scope_manager.pop_scope();

        // 関数を検証・最適化
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

    /// メソッドをコンパイル
    fn compile_method(&mut self, method: &MethodDecl) -> YuniResult<()> {
        let receiver_type_name = match &method.receiver.ty {
            Type::UserDefined(name) => name,
            Type::Reference(referent, _) => {
                if let Type::UserDefined(name) = referent.as_ref() {
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
        let function = self.functions.get(&method_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: format!("Method {} not found", method_name) 
            }))?
            .clone();

        self.current_function = Some(function);

        // エントリブロックを作成
        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        // 新しいスコープを作成
        self.scope_manager.push_scope();

        // レシーバーをスコープに追加
        let receiver_value = function
            .get_nth_param(0)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: "Receiver parameter not found".to_string() 
            }))?;

        let default_name = "self".to_string();
        let receiver_name = method.receiver.name.as_ref().unwrap_or(&default_name);
        let alloca = self.create_entry_block_alloca(receiver_name, &method.receiver.ty)?;
        self.builder.build_store(alloca, receiver_value)?;
        self.add_variable(receiver_name, alloca, method.receiver.ty.clone(), true)?;

        // その他のパラメータをスコープに追加
        for (i, param) in method.params.iter().enumerate() {
            let param_value = function
                .get_nth_param((i + 1) as u32)
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                    message: format!("Parameter {} not found", i) 
                }))?;

            let alloca = self.create_entry_block_alloca(&param.name, &param.ty)?;
            self.builder.build_store(alloca, param_value)?;
            self.add_variable(&param.name, alloca, param.ty.clone(), true)?;
        }

        // メソッド本体をコンパイル
        self.compile_block(&method.body)?;

        // 必要に応じて暗黙のreturnを追加
        if method.return_type.is_none() && !self.current_block_has_terminator() {
            self.builder.build_return(None)?;
        }

        // スコープを終了
        self.scope_manager.pop_scope();

        // 関数を検証・最適化
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

    /// エントリブロックにallocaを作成
    pub fn create_entry_block_alloca(&self, name: &str, ty: &Type) -> YuniResult<PointerValue<'ctx>> {
        let builder = self.context.create_builder();
        
        let entry = self.current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: "No current function".to_string() 
            }))?
            .get_first_basic_block()
            .unwrap();
            
        match entry.get_first_instruction() {
            Some(first_inst) => builder.position_before(&first_inst),
            None => builder.position_at_end(entry),
        }
        
        let llvm_type = self.type_manager.ast_type_to_llvm(ty)?;
        Ok(builder.build_alloca(llvm_type, name)?)
    }

    /// 変数をスコープに追加
    pub fn add_variable(&mut self, name: &str, ptr: PointerValue<'ctx>, ty: Type, is_mutable: bool) -> YuniResult<()> {
        let symbol = Symbol {
            ptr,
            ty,
            is_mutable,
        };
        self.scope_manager.define(name.to_string(), symbol);
        Ok(())
    }

    /// 型を推論（簡単な実装）
    pub fn infer_type(&mut self, expr: &Expression) -> YuniResult<Type> {
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
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::String(_) => Ok(Type::String),
            Expression::TemplateString(_) => Ok(Type::String),
            _ => Err(YuniError::Codegen(CodegenError::Internal {
                message: "Type inference not implemented for this expression".to_string(),
            })),
        }
    }


    // ヘルパー関数（未実装のものを追加）
    pub fn compile_field_assignment(&mut self, _field_expr: &FieldExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Field assignment not yet implemented".to_string(),
            span: Span::dummy(),
        }))
    }

    pub fn compile_index_assignment(&mut self, _index_expr: &IndexExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Index assignment not yet implemented".to_string(),
            span: Span::dummy(),
        }))
    }

    pub fn compile_deref_assignment(&mut self, _deref_expr: &DereferenceExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Dereference assignment not yet implemented".to_string(),
            span: Span::dummy(),
        }))
    }
    
    /// LLVM IRをファイルに書き込む
    pub fn write_llvm_ir(&self, path: &std::path::Path) -> YuniResult<()> {
        self.module.print_to_file(path)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to write LLVM IR: {}", e),
            }))
    }
    
    /// オブジェクトファイルを生成
    pub fn write_object_file(&self, path: &std::path::Path, opt_level: OptimizationLevel) -> YuniResult<()> {
        use inkwell::targets::{Target, TargetMachine, RelocMode, CodeModel, FileType};
        
        Target::initialize_native(&inkwell::targets::InitializationConfig::default())
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to initialize native target: {}", e),
            }))?;
            
        let target_triple = TargetMachine::get_default_triple();
        let target = Target::from_triple(&target_triple)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to get target: {}", e),
            }))?;
            
        let target_machine = target
            .create_target_machine(
                &target_triple,
                "generic",
                "",
                opt_level,
                RelocMode::Default,
                CodeModel::Default,
            )
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "Failed to create target machine".to_string(),
            }))?;
            
        target_machine
            .write_to_file(&self.module, FileType::Object, path)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to write object file: {}", e),
            }))
    }
}