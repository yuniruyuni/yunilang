//! 構造体・Enumのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /* // 古い初期化式の実装（削除予定）
    /// 初期化式をコンパイル
    pub fn compile_initializer_expr(&mut self, init_expr: &InitializerExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        match &init_expr.constructor {
            InitializerConstructor::Type { name, type_args } => {
                // Some, None, Ok, Err は特別に扱う
                if name == "Some" || name == "None" || name == "Ok" || name == "Err" {
                    return self.compile_option_result_constructor(name, init_expr);
                }
                // 構造体の初期化
                if self.type_manager.has_struct(name) {
                    // 既存の構造体リテラルとして変換
                    let mut fields = Vec::new();
                    for element in &init_expr.elements {
                        match element {
                            InitializerElement::Named { name, value } => {
                                fields.push(StructFieldInit {
                                    name: name.clone(),
                                    value: value.clone(),
                                });
                            }
                            _ => {
                                return Err(YuniError::Codegen(CodegenError::InvalidType {
                                    message: format!("Expected named field initialization for struct {}", name),
                                    span: init_expr.span,
                                }));
                            }
                        }
                    }
                    
                    let struct_lit = StructLiteral {
                        name: name.clone(),
                        fields,
                        span: init_expr.span,
                    };
                    
                    self.compile_struct_literal(&struct_lit)
                } 
                // 標準ライブラリ型の初期化
                else if name == "Vec" {
                    // Vec<T>の初期化
                    if type_args.len() != 1 {
                        return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: format!("Vec requires exactly one type argument, found {}", type_args.len()),
                            span: init_expr.span,
                        }));
                    }
                    
                    let element_type = &type_args[0];
                    let element_llvm_type = self.type_manager.ast_type_to_llvm(element_type)?;
                    let element_size = self.get_size_of_type(element_llvm_type);
                    
                    // yuni_vec_new を呼び出してVecを作成
                    let vec_new_fn = self.runtime_manager.get_or_declare_function(
                        "yuni_vec_new",
                        self.context.ptr_type(AddressSpace::default()).fn_type(&[
                            self.context.i64_type().into(),
                        ], false),
                        &self.module,
                    )?;
                    
                    let vec_ptr = self.builder.build_call(
                        vec_new_fn,
                        &[self.context.i64_type().const_int(element_size, false).into()],
                        "vec_new",
                    )?.try_as_basic_value().left().unwrap();
                    
                    // 各要素をVecに追加
                    if !init_expr.elements.is_empty() {
                        let vec_push_fn = self.runtime_manager.get_or_declare_function(
                            "yuni_vec_push",
                            self.context.void_type().fn_type(&[
                                self.context.ptr_type(AddressSpace::default()).into(),
                                self.context.ptr_type(AddressSpace::default()).into(),
                            ], false),
                            &self.module,
                        )?;
                        
                        for element in &init_expr.elements {
                            match element {
                                InitializerElement::Positional(expr) => {
                                    let value = self.compile_expression(expr)?;
                                    
                                    // 値を一時的にスタックに格納
                                    let temp_alloca = self.builder.build_alloca(element_llvm_type, "temp_element")?;
                                    self.builder.build_store(temp_alloca, value)?;
                                    
                                    // yuni_vec_push を呼び出し
                                    self.builder.build_call(
                                        vec_push_fn,
                                        &[
                                            vec_ptr.into(),
                                            temp_alloca.into(),
                                        ],
                                        "",
                                    )?;
                                }
                                _ => {
                                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                                        message: "Vec initialization requires positional elements".to_string(),
                                        span: init_expr.span,
                                    }));
                                }
                            }
                        }
                    }
                    
                    Ok(vec_ptr.into())
                }
                else if name == "HashMap" {
                    // HashMap<K, V>の初期化
                    if type_args.len() != 2 {
                        return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: format!("HashMap requires exactly two type arguments, found {}", type_args.len()),
                            span: init_expr.span,
                        }));
                    }
                    
                    let key_type = &type_args[0];
                    let value_type = &type_args[1];
                    let key_llvm_type = self.type_manager.ast_type_to_llvm(key_type)?;
                    let value_llvm_type = self.type_manager.ast_type_to_llvm(value_type)?;
                    let key_size = self.get_size_of_type(key_llvm_type);
                    let value_size = self.get_size_of_type(value_llvm_type);
                    
                    // yuni_hashmap_new を呼び出してHashMapを作成
                    let hashmap_new_fn = self.runtime_manager.get_or_declare_function(
                        "yuni_hashmap_new",
                        self.context.ptr_type(AddressSpace::default()).fn_type(&[
                            self.context.i64_type().into(), // key_size
                            self.context.i64_type().into(), // value_size
                        ], false),
                        &self.module,
                    )?;
                    
                    let hashmap_ptr = self.builder.build_call(
                        hashmap_new_fn,
                        &[
                            self.context.i64_type().const_int(key_size, false).into(),
                            self.context.i64_type().const_int(value_size, false).into(),
                        ],
                        "hashmap_new",
                    )?.try_as_basic_value().left().unwrap();
                    
                    // 各要素をHashMapに追加
                    if !init_expr.elements.is_empty() {
                        let hashmap_insert_fn = self.runtime_manager.get_or_declare_function(
                            "yuni_hashmap_insert",
                            self.context.void_type().fn_type(&[
                                self.context.ptr_type(AddressSpace::default()).into(), // hashmap
                                self.context.ptr_type(AddressSpace::default()).into(), // key
                                self.context.ptr_type(AddressSpace::default()).into(), // value
                            ], false),
                            &self.module,
                        )?;
                        
                        for element in &init_expr.elements {
                            match element {
                                InitializerElement::KeyValue { key, value } => {
                                    let key_val = self.compile_expression(key)?;
                                    let value_val = self.compile_expression(value)?;
                                    
                                    // キーと値を一時的にスタックに格納
                                    let key_alloca = self.builder.build_alloca(key_llvm_type, "temp_key")?;
                                    self.builder.build_store(key_alloca, key_val)?;
                                    
                                    let value_alloca = self.builder.build_alloca(value_llvm_type, "temp_value")?;
                                    self.builder.build_store(value_alloca, value_val)?;
                                    
                                    // yuni_hashmap_insert を呼び出し
                                    self.builder.build_call(
                                        hashmap_insert_fn,
                                        &[
                                            hashmap_ptr.into(),
                                            key_alloca.into(),
                                            value_alloca.into(),
                                        ],
                                        "",
                                    )?;
                                }
                                _ => {
                                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                                        message: "HashMap initialization requires key-value pairs".to_string(),
                                        span: init_expr.span,
                                    }));
                                }
                            }
                        }
                    }
                    
                    Ok(hashmap_ptr)
                }
                else {
                    // 未定義の型
                    Err(YuniError::Codegen(CodegenError::Undefined {
                        name: name.clone(),
                        span: init_expr.span,
                    }))
                }
            }
            InitializerConstructor::Expression(_) => {
                // TODO: 式コンストラクタの実装
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Expression constructors".to_string(),
                    span: init_expr.span,
                }))
            }
        }
    }
    
    /// Option/Result型のコンストラクタをコンパイル
    fn compile_option_result_constructor(&mut self, name: &str, init_expr: &InitializerExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        match name {
                        "None" => {
                            // Noneは単純なnullポインタとして表現
                            if !init_expr.elements.is_empty() {
                                return Err(YuniError::Codegen(CodegenError::InvalidType {
                                    message: "None takes no arguments".to_string(),
                                    span: init_expr.span,
                                }));
                            }
                            Ok(self.context.ptr_type(AddressSpace::default()).const_null().into())
                        }
                        "Some" => {
                            // Some(value)は値へのポインタとして表現
                            if init_expr.elements.len() != 1 {
                                return Err(YuniError::Codegen(CodegenError::InvalidType {
                                    message: "Some requires exactly one argument".to_string(),
                                    span: init_expr.span,
                                }));
                            }
                            
                            match &init_expr.elements[0] {
                                InitializerElement::Positional(expr) => {
                                    let value = self.compile_expression(expr)?;
                                    let value_type = value.get_type();
                                    
                                    // 値をヒープにアロケート
                                    let alloca = self.builder.build_alloca(value_type, "some_value")?;
                                    self.builder.build_store(alloca, value)?;
                                    
                                    Ok(alloca.into())
                                }
                                _ => {
                                    Err(YuniError::Codegen(CodegenError::InvalidType {
                                        message: "Some requires a positional argument".to_string(),
                                        span: init_expr.span,
                                    }))
                                }
                            }
                        }
                        "Ok" | "Err" => {
                            // Result型: 構造体として表現
                            // { tag: u8, ok: T, err: E }
                            if init_expr.elements.len() != 1 {
                                return Err(YuniError::Codegen(CodegenError::InvalidType {
                                    message: format!("{} requires exactly one argument", name),
                                    span: init_expr.span,
                                }));
                            }
                            
                            match &init_expr.elements[0] {
                                InitializerElement::Positional(expr) => {
                                    let value = self.compile_expression(expr)?;
                                    let value_type = value.get_type();
                                    
                                    // Result構造体を作成
                                    // 簡易実装: tag(i8) + payload(ptr)
                                    let result_type = self.context.struct_type(&[
                                        self.context.i8_type().into(),
                                        self.context.ptr_type(AddressSpace::default()).into(),
                                    ], false);
                                    
                                    let tag = if name == "Ok" { 0i8 } else { 1i8 };
                                    let tag_value = self.context.i8_type().const_int(tag as u64, false);
                                    
                                    // 値をヒープにアロケート
                                    let payload_alloca = self.builder.build_alloca(value_type, "result_payload")?;
                                    self.builder.build_store(payload_alloca, value)?;
                                    
                                    // Result構造体を構築
                                    let mut result_value = result_type.get_undef();
                                    result_value = self.builder.build_insert_value(result_value, tag_value, 0, "result_tag")?
                                        .into_struct_value();
                                    result_value = self.builder.build_insert_value(result_value, payload_alloca, 1, "result_payload")?
                                        .into_struct_value();
                                    
                                    Ok(result_value.into())
                                }
                                _ => {
                                    Err(YuniError::Codegen(CodegenError::InvalidType {
                                        message: format!("{} requires a positional argument", name),
                                        span: init_expr.span,
                                    }))
                                }
                            }
                        }
                _ => unreachable!()
        }
    }
    */

    /// 構造体リテラルをコンパイル
    pub fn compile_struct_literal(&mut self, struct_lit: &StructLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 型名が指定されていない場合はエラー（現在は型推論未実装）
        let struct_name = struct_lit.name.as_ref()
            .ok_or_else(|| YuniError::Codegen(CodegenError::InvalidType {
                message: "Type inference for anonymous struct literals not yet implemented".to_string(),
                span: struct_lit.span,
            }))?;
        
        // 構造体型を取得
        let struct_type = self.type_manager.get_struct(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: struct_name.clone(),
                span: struct_lit.span,
            }))?;

        // 構造体情報を取得してクローン（借用チェッカーエラーを回避）
        let struct_info = self.struct_info.get(struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_name),
            }))?
            .clone();

        // 各フィールドの値をコンパイル
        let mut field_values = vec![];
        for (index, field_type) in struct_info.field_types.iter().enumerate() {
            // フィールド名を取得
            let field_name = struct_info.field_indices.iter()
                .find(|(_, &idx)| idx == index as u32)
                .map(|(name, _)| name.clone())
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: format!("Field name not found for index {}", index),
                }))?;

            // 初期化されたフィールドを探す
            let field_init = struct_lit.fields.iter()
                .find(|f| f.name == field_name);

            let value = if let Some(init) = field_init {
                // フィールドが明示的に初期化されている場合
                self.compile_expression(&init.value)?
            } else {
                // フィールドが初期化されていない場合はデフォルト値を使用
                self.type_manager.create_default_value(field_type)?
            };

            field_values.push(value);
        }

        // 構造体値を作成
        // 動的な値を含む構造体の場合は、build_insert_valueを使用して構築
        let struct_val = struct_type.get_undef();
        let mut result = struct_val;
        
        for (i, field_value) in field_values.iter().enumerate() {
            result = self.builder.build_insert_value(result, *field_value, i as u32, &format!("field_{}", i))?
                .into_struct_value();
        }
        
        Ok(result.into())
    }

    /// 列挙型バリアントをコンパイル
    pub fn compile_enum_variant(&mut self, enum_var: &EnumVariantExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // Option型の特別処理
        if enum_var.enum_name == "Option" {
            match enum_var.variant.as_str() {
                "None" => {
                    // Noneは単純なnullポインタとして表現
                    match &enum_var.fields {
                        crate::ast::EnumVariantFields::Unit => {
                            Ok(self.context.ptr_type(AddressSpace::default()).const_null().into())
                        }
                        _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "None takes no arguments".to_string(),
                            span: enum_var.span,
                        }))
                    }
                }
                "Some" => {
                    // Some(value)は値へのポインタとして表現
                    match &enum_var.fields {
                        crate::ast::EnumVariantFields::Tuple(fields) if fields.len() == 1 => {
                            let value = self.compile_expression(&fields[0])?;
                            let value_type = value.get_type();
                            
                            // 値をヒープにアロケート
                            let alloca = self.builder.build_alloca(value_type, "some_value")?;
                            self.builder.build_store(alloca, value)?;
                            
                            Ok(alloca.into())
                        }
                        _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "Some requires exactly one argument".to_string(),
                            span: enum_var.span,
                        }))
                    }
                }
                _ => Err(YuniError::Codegen(CodegenError::Undefined {
                    name: format!("Option::{}", enum_var.variant),
                    span: enum_var.span,
                }))
            }
        }
        // Result型の特別処理
        else if enum_var.enum_name == "Result" {
            match enum_var.variant.as_str() {
                "Ok" | "Err" => {
                    match &enum_var.fields {
                        crate::ast::EnumVariantFields::Tuple(fields) if fields.len() == 1 => {
                            let value = self.compile_expression(&fields[0])?;
                            let value_type = value.get_type();
                            
                            // Result構造体を作成
                            // 簡易実装: tag(i8) + payload(ptr)
                            let result_type = self.context.struct_type(&[
                                self.context.i8_type().into(),
                                self.context.ptr_type(AddressSpace::default()).into(),
                            ], false);
                            
                            let tag = if enum_var.variant == "Ok" { 0i8 } else { 1i8 };
                            let tag_value = self.context.i8_type().const_int(tag as u64, false);
                            
                            // 値をヒープにアロケート
                            let payload_alloca = self.builder.build_alloca(value_type, "result_payload")?;
                            self.builder.build_store(payload_alloca, value)?;
                            
                            // Result構造体を構築
                            let mut result_value = result_type.get_undef();
                            result_value = self.builder.build_insert_value(result_value, tag_value, 0, "result_tag")?
                                .into_struct_value();
                            result_value = self.builder.build_insert_value(result_value, payload_alloca, 1, "result_payload")?
                                .into_struct_value();
                            
                            Ok(result_value.into())
                        }
                        _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: format!("{} requires exactly one argument", enum_var.variant),
                            span: enum_var.span,
                        }))
                    }
                }
                _ => Err(YuniError::Codegen(CodegenError::Undefined {
                    name: format!("Result::{}", enum_var.variant),
                    span: enum_var.span,
                }))
            }
        }
        // 通常のEnumの場合
        else {
            // バリアントのインデックスを取得
            let key = (enum_var.enum_name.clone(), enum_var.variant.clone());
            let variant_index = self.enum_variants.get(&key)
                .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                    name: format!("{}::{}", enum_var.enum_name, enum_var.variant),
                    span: enum_var.span,
                }))?;
            
            match &enum_var.fields {
            crate::ast::EnumVariantFields::Unit => {
                // i32の定数として返す
                Ok(self.context.i32_type().const_int(*variant_index as u64, false).into())
            }
            crate::ast::EnumVariantFields::Tuple(fields) => {
                // タプル形式のデータを持つバリアント
                // 構造: { discriminant: i32, data: tuple }
                let discriminant = self.context.i32_type().const_int(*variant_index as u64, false);
                
                // フィールドの値をコンパイル
                let mut field_values = vec![];
                for field in fields {
                    field_values.push(self.compile_expression(field)?);
                }
                
                // データタプルを作成
                let data_tuple = self.context.const_struct(&field_values, false);
                
                // Enum構造体を作成 { discriminant, data }
                let enum_struct = self.context.struct_type(&[
                    discriminant.get_type().into(),
                    data_tuple.get_type().into(),
                ], false);
                
                let mut enum_value = enum_struct.get_undef();
                enum_value = self.builder.build_insert_value(enum_value, discriminant, 0, "enum_discriminant")?
                    .into_struct_value();
                enum_value = self.builder.build_insert_value(enum_value, data_tuple, 1, "enum_data")?
                    .into_struct_value();
                
                Ok(enum_value.into())
            }
            crate::ast::EnumVariantFields::Struct(fields) => {
                // 構造体形式のデータを持つバリアント
                // 構造: { discriminant: i32, data: struct }
                let discriminant = self.context.i32_type().const_int(*variant_index as u64, false);
                
                // フィールドの値をコンパイル
                let mut field_values = vec![];
                for init in fields {
                    field_values.push(self.compile_expression(&init.value)?);
                }
                
                // データ構造体を作成
                let data_struct = self.context.const_struct(&field_values, false);
                
                // Enum構造体を作成 { discriminant, data }
                let enum_struct = self.context.struct_type(&[
                    discriminant.get_type().into(),
                    data_struct.get_type().into(),
                ], false);
                
                let mut enum_value = enum_struct.get_undef();
                enum_value = self.builder.build_insert_value(enum_value, discriminant, 0, "enum_discriminant")?
                    .into_struct_value();
                enum_value = self.builder.build_insert_value(enum_value, data_struct, 1, "enum_data")?
                    .into_struct_value();
                
                Ok(enum_value.into())
            }
        }
        }
    }

    /// 参照式をコンパイル
    pub fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 参照式は内部式のアドレスを返す
        match &*ref_expr.expr {
            Expression::Identifier(id) => {
                // 変数への参照の場合、そのポインタを直接返す
                let symbol = self.scope_manager.lookup(&id.name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))?;
                
                Ok(symbol.ptr.into())
            }
            Expression::Field(field_expr) => {
                // フィールドへの参照の場合
                self.compile_field_reference(field_expr)
            }
            Expression::Index(index_expr) => {
                // 配列要素への参照の場合
                self.compile_index_reference(index_expr)
            }
            _ => {
                // その他の式への参照は現在未サポート
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: format!("Reference to {:?} expressions not yet implemented", ref_expr.expr),
                    span: ref_expr.span,
                }))
            }
        }
    }

    /// デリファレンス式をコンパイル
    pub fn compile_dereference_expr(&mut self, deref: &DereferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 内部式をコンパイルしてポインタを取得
        let ptr_value = self.compile_expression(&deref.expr)?;
        
        // ポインタ型であることを確認
        let ptr = ptr_value.into_pointer_value();
        
        // ポインタが指す型を推論
        let inner_type = match self.expression_type(&deref.expr)? {
            Type::Reference(inner, _is_mut) => *inner,
            _ => {
                return Err(YuniError::Codegen(CodegenError::TypeError {
                    expected: "reference type".to_string(),
                    actual: format!("{:?}", self.expression_type(&deref.expr)?),
                    span: deref.span,
                }));
            }
        };
        
        // LLVMの型に変換
        let llvm_type = self.type_manager.ast_type_to_llvm(&inner_type)?;
        
        // ポインタから値をロード
        let value = self.builder.build_load(
            llvm_type,
            ptr,
            "deref_value",
        )?;
        
        Ok(value)
    }
    
    /// フィールドへの参照を取得
    fn compile_field_reference(&mut self, field: &FieldExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // オブジェクトの式をコンパイル
        let object_value = self.compile_expression(&field.object)?;
        
        // オブジェクトの型を推論
        let object_type = self.expression_type(&field.object)?;
        
        // 構造体名を取得
        let struct_name = match &object_type {
            Type::UserDefined(name) => name.clone(),
            Type::Reference(inner, _is_mut) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name.clone()
                } else {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Field access on non-struct type".to_string(),
                        span: field.span,
                    }));
                }
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Field access on non-struct type".to_string(),
                    span: field.span,
                }));
            }
        };
        
        // 構造体情報を取得
        let struct_info = self.struct_info.get(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_name),
            }))?;
        
        // フィールドのインデックスを取得
        let field_index = struct_info.get_field_index(&field.field)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("{}.{}", struct_name, field.field),
                span: field.span,
            }))?;
        
        // 構造体へのポインタを取得
        let struct_ptr = match object_value {
            BasicValueEnum::StructValue(_) => {
                // 構造体値の場合、変数として格納されている必要がある
                // TODO: 一時変数に格納してポインタを取得
                return Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Reference to temporary struct field not yet implemented".to_string(),
                    span: field.span,
                }));
            }
            BasicValueEnum::PointerValue(ptr) => ptr,
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Expected struct or pointer to struct".to_string(),
                    span: field.span,
                }));
            }
        };
        
        // 構造体型を取得
        let struct_type = self.type_manager.get_struct(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct type not found for {}", struct_name),
            }))?;
        
        // フィールドへのポインタを計算（GEP）
        let field_ptr = unsafe {
            self.builder.build_gep(
                struct_type,
                struct_ptr,
                &[
                    self.context.i32_type().const_zero(),
                    self.context.i32_type().const_int(field_index as u64, false)
                ],
                &format!("{}_ptr", field.field)
            )?
        };
        
        Ok(field_ptr.into())
    }
    
    /// インデックスへの参照を取得
    fn compile_index_reference(&mut self, index: &IndexExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // オブジェクト（配列）の式をコンパイル
        let object_value = self.compile_expression(&index.object)?;
        
        // インデックスの式をコンパイル
        let index_value = self.compile_expression(&index.index)?;
        
        // オブジェクトの型を推論
        let object_type = self.expression_type(&index.object)?;
        
        match &object_type {
            Type::Array(element_type) => {
                // 配列のインデックスアクセス
                let array_ptr = object_value.into_pointer_value();
                
                // インデックスが整数型であることを確認
                let index_int = index_value.into_int_value();
                
                // 要素のLLVM型を取得
                let element_llvm_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // GEPで要素のアドレスを計算（参照として返す）
                let element_ptr = unsafe {
                    self.builder.build_gep(
                        element_llvm_type,
                        array_ptr,
                        &[index_int],
                        "element_ref"
                    )?
                };
                
                Ok(element_ptr.into())
            }
            Type::Generic(name, type_args) if name == "Vec" && type_args.len() == 1 => {
                // Vecのインデックスアクセス
                // Vecの場合、要素は動的にヒープに格納されているため、
                // 直接的な参照を取ることができない
                // 一時的な解決策として、要素の値を一時変数に格納してその参照を返す
                let element_type = &type_args[0];
                let vec_ptr = object_value.into_pointer_value();
                let index_int = index_value.into_int_value();
                let element_llvm_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // vec_getを使用して要素を取得
                let value = self.vec_get(vec_ptr, index_int, element_llvm_type)?;
                
                // 一時変数に格納
                let temp_alloca = self.builder.build_alloca(element_llvm_type, "temp_vec_elem")?;
                self.builder.build_store(temp_alloca, value)?;
                
                Ok(temp_alloca.into())
            }
            _ => {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Cannot take reference to index of type: {:?}", object_type),
                    span: index.span,
                }))
            }
        }
    }
    
    /// リストリテラルをコンパイル
    pub fn compile_list_literal(&mut self, list: &ListLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &list.type_name {
            if type_name == "Vec" && type_args.len() == 1 {
                let element_type = &type_args[0];
                let llvm_element_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // Vecの作成
                let vec_ptr = self.create_vec_new(llvm_element_type)?;
                
                // 各要素を追加
                for elem in &list.elements {
                    let value = self.compile_expression(elem)?;
                    self.vec_push(vec_ptr, value, llvm_element_type)?;
                }
                
                Ok(vec_ptr.into())
            } else {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Unknown list type: {}", type_name),
                    span: list.span,
                }))
            }
        } else {
            // 型名が省略されている場合、デフォルトでVecとして扱う
            if list.elements.is_empty() {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Cannot infer type for empty list literal".to_string(),
                    span: list.span,
                }))
            } else {
                // 最初の要素の型から推論
                let first_elem = self.compile_expression(&list.elements[0])?;
                let element_type = first_elem.get_type();
                
                // Vecの作成
                let vec_ptr = self.create_vec_new(element_type)?;
                
                // 最初の要素を追加
                self.vec_push(vec_ptr, first_elem, element_type)?;
                
                // 残りの要素を追加
                for elem in &list.elements[1..] {
                    let value = self.compile_expression(elem)?;
                    self.vec_push(vec_ptr, value, element_type)?;
                }
                
                Ok(vec_ptr.into())
            }
        }
    }
    
    /// マップリテラルをコンパイル
    pub fn compile_map_literal(&mut self, map: &MapLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &map.type_name {
            if type_name == "HashMap" && type_args.len() == 2 {
                let key_type = &type_args[0];
                let value_type = &type_args[1];
                let llvm_key_type = self.type_manager.ast_type_to_llvm(key_type)?;
                let llvm_value_type = self.type_manager.ast_type_to_llvm(value_type)?;
                
                // HashMapの作成
                let map_ptr = self.create_hashmap_new(llvm_key_type, llvm_value_type)?;
                
                // 各ペアを挿入
                for (key_expr, value_expr) in &map.pairs {
                    let key = self.compile_expression(key_expr)?;
                    let value = self.compile_expression(value_expr)?;
                    self.hashmap_insert(map_ptr, key, value, llvm_key_type, llvm_value_type)?;
                }
                
                Ok(map_ptr.into())
            } else {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Unknown map type: {}", type_name),
                    span: map.span,
                }))
            }
        } else {
            // 型名が省略されている場合、デフォルトでHashMapとして扱う
            if map.pairs.is_empty() {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Cannot infer type for empty map literal".to_string(),
                    span: map.span,
                }))
            } else {
                // 最初のペアの型から推論
                let (first_key, first_value) = &map.pairs[0];
                let key = self.compile_expression(first_key)?;
                let value = self.compile_expression(first_value)?;
                let key_type = key.get_type();
                let value_type = value.get_type();
                
                // HashMapの作成
                let map_ptr = self.create_hashmap_new(key_type, value_type)?;
                
                // 最初のペアを挿入
                self.hashmap_insert(map_ptr, key, value, key_type, value_type)?;
                
                // 残りのペアを挿入
                for (key_expr, value_expr) in &map.pairs[1..] {
                    let key = self.compile_expression(key_expr)?;
                    let value = self.compile_expression(value_expr)?;
                    self.hashmap_insert(map_ptr, key, value, key_type, value_type)?;
                }
                
                Ok(map_ptr.into())
            }
        }
    }
}