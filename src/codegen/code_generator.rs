//! コード生成器

use inkwell::builder::Builder;
use inkwell::context::Context as LLVMContext;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{InitializationConfig, Target, TargetTriple, RelocMode, CodeModel, FileType};
use inkwell::OptimizationLevel;
use std::collections::HashMap;

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};

use super::runtime::RuntimeManager;
use super::scope::ScopeManager;
use super::tail_call::TailContext;
use super::types::TypeManager;
use super::scope::StructInfo;

pub(crate) mod expr;
mod collections_runtime;
mod decl_codegen;

/// ループ制御のコンテキスト
pub struct BreakableContext<'ctx> {
    /// ループの継続ブロック
    pub continue_block: inkwell::basic_block::BasicBlock<'ctx>,
    /// ループの終了ブロック
    pub break_block: inkwell::basic_block::BasicBlock<'ctx>,
}

/// コード生成器
pub struct CodeGenerator<'ctx> {
    pub context: &'ctx LLVMContext,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub pass_manager: PassManager<inkwell::values::FunctionValue<'ctx>>,
    pub functions: HashMap<String, inkwell::values::FunctionValue<'ctx>>,
    pub scope_manager: ScopeManager<'ctx>,
    pub type_manager: TypeManager<'ctx>,
    pub runtime_manager: RuntimeManager<'ctx>,
    /// 構造体情報（フィールド名とインデックスのマッピング）
    pub struct_info: HashMap<String, StructInfo>,
    /// 構造体のメソッド情報（構造体名 -> (メソッド名, マングル名)）
    pub struct_methods: HashMap<String, Vec<(String, String)>>,
    /// Enumのバリアント情報（(Enum名, バリアント名) -> タグ値）
    pub enum_variants: HashMap<(String, String), u32>,
    /// 関数の戻り値型情報
    pub function_types: HashMap<String, Type>,
    /// 末尾呼び出し最適化のコンテキスト
    pub tail_context: TailContext,
    /// ループ制御のコンテキスト
    pub loop_context: Vec<BreakableContext<'ctx>>,
    /// 現在コンパイル中の関数
    pub current_function: Option<inkwell::values::FunctionValue<'ctx>>,
    /// 現在の関数の戻り値型
    pub current_return_type: Option<Type>,
}

impl<'ctx> CodeGenerator<'ctx> {
    pub fn new(context: &'ctx LLVMContext, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        
        // ターゲット情報を設定
        Target::initialize_native(&InitializationConfig::default())
            .expect("Failed to initialize native target");
        
        let target_triple = TargetTriple::create("x86_64-apple-darwin");
        module.set_triple(&target_triple);
        
        let builder = context.create_builder();
        
        // 最適化パスマネージャーを設定
        let pass_manager = PassManager::create(&module);
        
        let mut runtime_manager = RuntimeManager::new(context);
        runtime_manager.initialize(&module);
        
        Self {
            context,
            module,
            builder,
            pass_manager,
            functions: HashMap::new(),
            scope_manager: ScopeManager::new(),
            type_manager: TypeManager::new(context),
            runtime_manager,
            struct_info: HashMap::new(),
            struct_methods: HashMap::new(),
            enum_variants: HashMap::new(),
            function_types: HashMap::new(),
            tail_context: TailContext::new(),
            loop_context: Vec::new(),
            current_function: None,
            current_return_type: None,
        }
    }

    /// モジュールを取得
    pub fn get_module(&self) -> &Module<'ctx> {
        &self.module
    }

    /// プログラムをコンパイル
    pub fn compile_program(&mut self, program: &Program) -> YuniResult<()> {
        // 第一パス: 型定義を処理
        for item in &program.items {
            if let Item::TypeDef(type_def) = item {
                self.process_type_definition(type_def)?;
            }
        }
        
        // 第二パス: 関数シグネチャを収集
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
        
        // main関数が存在することを確認
        if !self.functions.contains_key("main") {
            return Err(YuniError::Codegen(CodegenError::MissingMain));
        }
        
        // モジュールを検証
        if let Err(err) = self.module.verify() {
            return Err(YuniError::Codegen(CodegenError::ModuleVerification {
                message: err.to_string(),
            }));
        }
        
        // 最適化を実行（FunctionPassManagerはfunction単位で実行される）
        // ModulePassManagerが必要な場合は別途実装が必要
        
        Ok(())
    }

    /// LLVM IRをファイルに書き出す
    pub fn write_llvm_ir(&self, path: &std::path::Path) -> YuniResult<()> {
        self.module.print_to_file(path)
            .map_err(|e| YuniError::Codegen(CodegenError::Internal {
                message: format!("Failed to write LLVM IR: {}", e),
            }))
    }

    /// オブジェクトファイルを生成
    pub fn write_object_file(&self, path: &std::path::Path, opt_level: OptimizationLevel) -> YuniResult<()> {
        let target_triple = self.module.get_triple();
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