//! 宣言のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::types::BasicType;
use inkwell::values::BasicValue;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 型定義を処理
    pub fn process_type_definition(&mut self, type_def: &TypeDef) -> YuniResult<()> {
        match type_def {
            TypeDef::Struct(struct_def) => self.process_struct_definition(struct_def),
            TypeDef::Enum(enum_def) => self.process_enum_definition(enum_def),
            TypeDef::Alias(alias) => self.process_type_alias(alias),
        }
    }

    /// 構造体定義を処理
    fn process_struct_definition(&mut self, struct_def: &StructDef) -> YuniResult<()> {
        // フィールドの型を収集
        let field_types: Vec<_> = struct_def.fields.iter()
            .map(|field| self.type_manager.ast_type_to_llvm(&field.ty))
            .collect::<YuniResult<Vec<_>>>()?;

        // LLVM構造体型を作成
        let struct_type = self.context.struct_type(&field_types, false);
        
        // 型を登録
        self.type_manager.register_struct(struct_def.name.clone(), struct_type);
        
        // フィールド情報を保存
        let mut struct_info = super::super::scope::StructInfo::new();
        for (i, field) in struct_def.fields.iter().enumerate() {
            struct_info.add_field(field.name.clone(), field.ty.clone());
        }
        self.struct_info.insert(struct_def.name.clone(), struct_info);
        
        Ok(())
    }

    /// 列挙型定義を処理
    fn process_enum_definition(&mut self, enum_def: &EnumDef) -> YuniResult<()> {
        // 列挙型はタグ（i32）として表現
        let enum_type = self.context.i32_type();
        self.type_manager.register_enum(enum_def.name.clone(), enum_type);
        
        // バリアントにタグ値を割り当て
        for (i, variant) in enum_def.variants.iter().enumerate() {
            let key = (enum_def.name.clone(), variant.name.clone());
            self.enum_variants.insert(key, i as u32);
        }
        
        Ok(())
    }

    /// 型エイリアスを処理
    fn process_type_alias(&mut self, alias: &TypeAlias) -> YuniResult<()> {
        self.type_manager.register_type_alias(alias.name.clone(), alias.underlying_type.clone());
        Ok(())
    }

    /// 関数を宣言
    pub fn declare_function(&mut self, func_decl: &FunctionDecl) -> YuniResult<()> {
        let fn_type = self.create_function_type(func_decl)?;
        let fn_value = self.module.add_function(&func_decl.name, fn_type, None);
        
        // 関数を登録
        self.functions.insert(func_decl.name.clone(), fn_value);
        
        // 関数型をAST型として登録
        let func_type = Type::Function(FunctionType {
            params: func_decl.params.iter().map(|p| p.ty.clone()).collect(),
            return_type: Box::new(func_decl.return_type.as_ref()
                .map(|ty| ty.as_ref().clone())
                .unwrap_or(Type::Void)),
        });
        self.function_types.insert(func_decl.name.clone(), func_type);
        
        Ok(())
    }

    /// メソッドを宣言
    pub fn declare_method(&mut self, method_decl: &MethodDecl) -> YuniResult<()> {
        // メソッドのマングル名を生成
        let receiver_type = self.get_receiver_type(&method_decl.receiver)?;
        let mangled_name = format!("{}__{}", receiver_type, method_decl.name);
        
        let fn_type = self.create_method_type(method_decl)?;
        let fn_value = self.module.add_function(&mangled_name, fn_type, None);
        
        // メソッドを登録
        self.functions.insert(mangled_name.clone(), fn_value);
        
        // メソッド型をAST型として登録
        let method_type = Type::Function(FunctionType {
            params: method_decl.params.iter().map(|p| p.ty.clone()).collect(),
            return_type: Box::new(method_decl.return_type.as_ref()
                .map(|ty| ty.as_ref().clone())
                .unwrap_or(Type::Void)),
        });
        self.function_types.insert(mangled_name.clone(), method_type);
        
        // 構造体のメソッドリストに追加
        self.struct_methods
            .entry(receiver_type.clone())
            .or_insert_with(Vec::new)
            .push((method_decl.name.clone(), mangled_name));
        
        Ok(())
    }

    /// 関数をコンパイル
    pub fn compile_function(&mut self, func_decl: &FunctionDecl) -> YuniResult<()> {
        let fn_value = self.functions.get(&func_decl.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: func_decl.name.clone(),
                span: func_decl.span,
            }))?;
        
        // 現在の関数を設定
        self.current_function = Some(*fn_value);
        self.current_return_type = func_decl.return_type.as_ref().map(|ty| ty.as_ref().clone());
        
        // エントリブロックを作成
        let entry = self.context.append_basic_block(*fn_value, "entry");
        self.builder.position_at_end(entry);
        
        // パラメータをスコープに追加
        self.scope_manager.push_scope();
        for (i, param) in func_decl.params.iter().enumerate() {
            let param_value = fn_value.get_nth_param(i as u32).unwrap();
            param_value.set_name(&param.name);
            
            // パラメータをアロケート
            let param_type = self.type_manager.ast_type_to_llvm(&param.ty)?;
            let alloca = self.builder.build_alloca(param_type, &param.name)?;
            self.builder.build_store(alloca, param_value)?;
            
            self.scope_manager.define_variable(
                param.name.clone(),
                alloca,
                param.ty.clone(),
                false,
            );
        }
        
        // 関数本体をコンパイル
        self.compile_block(&func_decl.body)?;
        
        // 明示的なreturnがない場合の処理
        if !self.current_block_has_terminator() {
            if matches!(func_decl.return_type, Some(ref ty) if !matches!(**ty, Type::Void)) {
                return Err(YuniError::Codegen(CodegenError::CompilationFailed {
                    message: "Function must return a value".to_string(),
                    span: func_decl.span,
                }));
            }
            self.builder.build_return(None)?;
        }
        
        self.scope_manager.pop_scope();
        
        Ok(())
    }

    /// メソッドをコンパイル
    pub fn compile_method(&mut self, method_decl: &MethodDecl) -> YuniResult<()> {
        let receiver_type = self.get_receiver_type(&method_decl.receiver)?;
        let mangled_name = format!("{}__{}", receiver_type, method_decl.name);
        
        let fn_value = self.functions.get(&mangled_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: mangled_name.clone(),
                span: method_decl.span,
            }))?;
        
        // 現在の関数を設定
        self.current_function = Some(*fn_value);
        self.current_return_type = method_decl.return_type.as_ref().map(|ty| ty.as_ref().clone());
        
        // エントリブロックを作成
        let entry = self.context.append_basic_block(*fn_value, "entry");
        self.builder.position_at_end(entry);
        
        // スコープを開始
        self.scope_manager.push_scope();
        
        // レシーバーを処理
        let receiver_param = fn_value.get_nth_param(0).unwrap();
        receiver_param.set_name("self");
        
        // レシーバーの型をチェック
        if matches!(&method_decl.receiver.ty, Type::Reference(_, _)) {
            // 参照レシーバーはそのまま使用
            self.scope_manager.define_variable(
                "self".to_string(),
                receiver_param.into_pointer_value(),
                method_decl.receiver.ty.clone(),
                false,
            );
        } else {
            // 所有権レシーバーはアロケート
            let self_type = self.type_manager.ast_type_to_llvm(&method_decl.receiver.ty)?;
            let alloca = self.builder.build_alloca(self_type, "self")?;
            self.builder.build_store(alloca, receiver_param)?;
            
            self.scope_manager.define_variable(
                "self".to_string(),
                alloca,
                method_decl.receiver.ty.clone(),
                false,
            );
        }
        
        // その他のパラメータを処理
        for (i, param) in method_decl.params.iter().enumerate() {
            let param_value = fn_value.get_nth_param((i + 1) as u32).unwrap();
            param_value.set_name(&param.name);
            
            let param_type = self.type_manager.ast_type_to_llvm(&param.ty)?;
            let alloca = self.builder.build_alloca(param_type, &param.name)?;
            self.builder.build_store(alloca, param_value)?;
            
            self.scope_manager.define_variable(
                param.name.clone(),
                alloca,
                param.ty.clone(),
                false,
            );
        }
        
        // メソッド本体をコンパイル
        self.compile_block(&method_decl.body)?;
        
        // 明示的なreturnがない場合の処理
        if !self.current_block_has_terminator() {
            if matches!(method_decl.return_type, Some(ref ty) if !matches!(**ty, Type::Void)) {
                return Err(YuniError::Codegen(CodegenError::CompilationFailed {
                    message: "Method must return a value".to_string(),
                    span: method_decl.span,
                }));
            }
            self.builder.build_return(None)?;
        }
        
        self.scope_manager.pop_scope();
        
        Ok(())
    }

    /// 関数型を作成
    fn create_function_type(&self, func_decl: &FunctionDecl) -> YuniResult<inkwell::types::FunctionType<'ctx>> {
        let param_types: Vec<Type> = func_decl.params.iter()
            .map(|param| param.ty.clone())
            .collect();
        
        let return_type = func_decl.return_type.as_ref()
            .map(|ty| ty.as_ref())
            .unwrap_or(&Type::Void);
        
        self.type_manager.create_function_type(&param_types, return_type, false)
    }

    /// メソッド型を作成
    fn create_method_type(&self, method_decl: &MethodDecl) -> YuniResult<inkwell::types::FunctionType<'ctx>> {
        let self_type = self.type_manager.ast_type_to_llvm(&method_decl.receiver.ty)?;
        
        let mut param_types = vec![self_type];
        for param in &method_decl.params {
            param_types.push(self.type_manager.ast_type_to_llvm(&param.ty)?);
        }
        
        let return_type = method_decl.return_type.as_ref()
            .map(|ty| ty.as_ref())
            .unwrap_or(&Type::Void);
        
        // 簡易的な実装
        Ok(if matches!(return_type, Type::Void) {
            self.context.void_type().fn_type(&param_types.iter().map(|t| (*t).into()).collect::<Vec<_>>(), false)
        } else {
            let ret_type = self.type_manager.ast_type_to_llvm(return_type)?;
            ret_type.fn_type(&param_types.iter().map(|t| (*t).into()).collect::<Vec<_>>(), false)
        })
    }

    /// レシーバーの型名を取得
    fn get_receiver_type(&self, receiver: &Receiver) -> YuniResult<String> {
        match &receiver.ty {
            Type::UserDefined(name) => Ok(name.clone()),
            Type::Reference(inner, _) => {
                match inner.as_ref() {
                    Type::UserDefined(name) => Ok(name.clone()),
                    _ => Ok("UnknownType".to_string()),
                }
            }
            _ => Ok("UnknownType".to_string()),
        }
    }

}