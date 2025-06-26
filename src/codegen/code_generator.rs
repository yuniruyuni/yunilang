//! メインコード生成器

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::builder::Builder;
use inkwell::context::Context as LLVMContext;
use inkwell::module::Module;
use inkwell::passes::PassManager;
use inkwell::targets::{Target, TargetMachine};
use inkwell::types::BasicTypeEnum;
use inkwell::values::FunctionValue;
use inkwell::OptimizationLevel;
use std::collections::HashMap;

use super::runtime::RuntimeManager;
use super::symbol_table::{ScopeManager, StructInfo};
use super::tail_call::TailContext;
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
    // 関数の戻り値型情報
    pub function_types: HashMap<String, Type>,
    
    // 構造体のフィールド情報
    pub struct_info: HashMap<String, StructInfo>,
    
    // 構造体のメソッド情報 (構造体名 -> Vec<(メソッド名, 関数名)>)
    pub struct_methods: HashMap<String, Vec<(String, String)>>,
    
    // Enumのバリアント情報（名前 -> (Enum名, バリアントインデックス)）
    pub enum_variants: HashMap<(String, String), u32>,

    // 現在コンパイル中の関数
    pub current_function: Option<FunctionValue<'ctx>>,
    // 現在の関数の戻り値型（型推論用）
    pub current_return_type: Option<Type>,
    // 末尾呼び出しコンテキスト
    pub tail_context: TailContext,
}

impl<'ctx> CodeGenerator<'ctx> {
    /// 型のサイズを取得（バイト単位）
    pub fn get_size_of_type(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            BasicTypeEnum::IntType(int_type) => {
                // ビット数をバイト数に変換
                (int_type.get_bit_width() / 8) as u64
            }
            BasicTypeEnum::FloatType(float_type) => {
                // FloatTypeのビット幅を取得
                match float_type {
                    t if t == self.context.f32_type() => 4,
                    t if t == self.context.f64_type() => 8,
                    t if t == self.context.f128_type() => 16,
                    _ => 8, // デフォルト
                }
            }
            BasicTypeEnum::PointerType(_) => {
                // ポインタサイズ（64ビットシステムを仮定）
                8
            }
            BasicTypeEnum::StructType(struct_type) => {
                // 構造体の場合、各フィールドのサイズの合計（簡易実装）
                // 実際にはアライメントを考慮する必要がある
                let mut size = 0u64;
                for i in 0..struct_type.count_fields() {
                    if let Some(field_type) = struct_type.get_field_type_at_index(i) {
                        size += self.get_size_of_type(field_type);
                    }
                }
                size
            }
            BasicTypeEnum::ArrayType(array_type) => {
                let element_size = self.get_size_of_type(array_type.get_element_type());
                element_size * array_type.len() as u64
            }
            BasicTypeEnum::VectorType(vector_type) => {
                let element_size = self.get_size_of_type(vector_type.get_element_type());
                element_size * vector_type.get_size() as u64
            }
        }
    }

    pub fn new(context: &'ctx LLVMContext, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        let pass_manager = PassManager::create(&module);

        // ターゲット情報を設定
        Target::initialize_all(&inkwell::targets::InitializationConfig::default());
        let target_triple = TargetMachine::get_default_triple();
        module.set_triple(&target_triple);

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
            function_types: HashMap::new(),
            struct_info: HashMap::new(),
            struct_methods: HashMap::new(),
            enum_variants: HashMap::new(),
            current_function: None,
            current_return_type: None,
            tail_context: TailContext::new(),
        }
    }
    
    /// LLVMモジュールを取得
    #[allow(dead_code)]
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

    /// 型を宣言（構造体、列挙型、または型エイリアス）
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
                for field in struct_def.fields.iter() {
                    struct_info.add_field(field.name.clone(), field.ty.clone());
                }
                self.struct_info.insert(struct_def.name.clone(), struct_info);
            }
            TypeDef::Enum(enum_def) => {
                // Enumは整数型として表現
                // 各バリアントに0から順にインデックスを割り当てる
                for (index, variant) in enum_def.variants.iter().enumerate() {
                    let key = (enum_def.name.clone(), variant.name.clone());
                    self.enum_variants.insert(key, index as u32);
                }
                
                // Enum型をi32として登録
                let enum_type = self.context.i32_type();
                self.type_manager.register_enum(enum_def.name.clone(), enum_type);
            }
            TypeDef::Alias(type_alias) => {
                // 型エイリアスをTypeManagerに登録
                self.type_manager.register_type_alias(
                    type_alias.name.clone(),
                    type_alias.underlying_type.clone()
                );
            }
        }
        Ok(())
    }

    /// 関数を宣言
    fn declare_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        let param_types: Vec<Type> = func
            .params
            .iter()
            .map(|param| param.ty.clone())
            .collect();

        let return_type = func.return_type.as_deref().unwrap_or(&Type::Void);
        let fn_type = self.type_manager.create_function_type(&param_types, return_type, false)?;

        let function = self.module.add_function(&func.name, fn_type, None);
        self.functions.insert(func.name.clone(), function);
        self.function_types.insert(func.name.clone(), return_type.clone());

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

        let return_type = method.return_type.as_deref().unwrap_or(&Type::Void);
        let fn_type = self.type_manager.create_function_type(
            &method.params.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
            return_type,
            false,
        )?;

        let function = self.module.add_function(&method_name, fn_type, None);
        self.functions.insert(method_name.clone(), function);
        self.function_types.insert(method_name.clone(), return_type.clone());
        
        // 構造体とメソッドの関連を保存
        self.struct_methods
            .entry(receiver_type_name.clone())
            .or_default()
            .push((method.name.clone(), method_name));

        Ok(())
    }

    /// 関数をコンパイル
    fn compile_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        let function = *self.functions.get(&func.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: format!("Function {} not found", func.name) 
            }))?;

        self.current_function = Some(function);
        self.current_return_type = func.return_type.as_ref().map(|t| (**t).clone());
        
        // 末尾位置解析を実行
        self.tail_context.analyze_function(func);

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
            // 検証失敗時にLLVM IRを出力してデバッグ
            eprintln!("Function verification failed: {}", func.name);
            function.print_to_stderr();
            return Err(YuniError::Codegen(CodegenError::Internal {
                message: format!("Function verification failed: {}", func.name),
            }));
        }

        self.current_function = None;
        self.current_return_type = None;
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
        let function = *self.functions.get(&method_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: format!("Method {} not found", method_name) 
            }))?;

        self.current_function = Some(function);
        self.current_return_type = method.return_type.as_ref().map(|t| (**t).clone());

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
        self.current_return_type = None;
        Ok(())
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