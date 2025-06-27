//! 関数・メソッド呼び出しのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 関数呼び出し式をコンパイル
    pub fn compile_call_expr(&mut self, call: &CallExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 関数名を取得
        let func_name = match call.callee.as_ref() {
            Expression::Identifier(id) => &id.name,
            Expression::Path(path) => {
                if path.segments.len() == 1 {
                    &path.segments[0]
                } else {
                    return Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Multi-segment function paths not yet implemented".to_string(),
                        span: call.span,
                    }));
                }
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Invalid function callee".to_string(),
                    span: call.span,
                }));
            }
        };

        // printlnの特別な処理
        if func_name == "println" {
            return self.compile_println_call(&call.args, call.span);
        }

        // 関数情報を取得（コピーして借用を解放）
        let func = *self.functions.get(func_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: func_name.clone(),
                span: call.span,
            }))?;
            
        let func_type = func.get_type();
        let param_types = func_type.get_param_types();

        // 通常の関数呼び出し
        // 引数を先にコンパイルし、必要に応じて型変換
        let mut args = Vec::new();
        
        for (i, arg) in call.args.iter().enumerate() {
            let arg_value = self.compile_expression(arg)?;
            
            // パラメータの型に合わせて変換
            if i < param_types.len() {
                let expected_type = param_types[i];
                let coerced_value = self.coerce_to_type(arg_value, expected_type, arg.span())?;
                args.push(coerced_value.into());
            } else {
                args.push(arg_value.into());
            }
        }

        // 関数呼び出し
        let call_site = self.builder.build_call(func, &args, "call_result")?;
        
        // 末尾呼び出し最適化を適用
        if self.tail_context.is_tail_call(call) {
            call_site.set_tail_call(true);
        }
        
        if let Some(value) = call_site.try_as_basic_value().left() {
            Ok(value)
        } else {
            // void関数の場合、unit値を返す
            Ok(self.context.i32_type().const_zero().into())
        }
    }

    /// println呼び出しのコンパイル
    fn compile_println_call(&mut self, args: &[Expression], _span: Span) -> YuniResult<BasicValueEnum<'ctx>> {
        if args.is_empty() {
            // 引数なしの場合は改行のみ
            let newline_str = self.context.const_string(b"\n", true);
            let global = self.module.add_global(newline_str.get_type(), None, "newline");
            global.set_initializer(&newline_str);
            global.set_constant(true);

            let printf_fn = self.runtime_manager.get_function("printf")
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: "printf function not found".to_string(),
                }))?;

            let ptr = global.as_pointer_value();
            self.builder.build_call(printf_fn, &[ptr.into()], "println_call")?;
            return Ok(self.context.i32_type().const_zero().into());
        }

        // 最初の引数をコンパイルして、文字列に変換
        let arg_value = self.compile_expression(&args[0])?;
        let format_arg = self.value_to_string(arg_value)?;
        
        if args.len() == 1 {
            // 引数が1つの場合
            let printf_fn = self.runtime_manager.get_function("printf")
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: "printf function not found".to_string(),
                }))?;

            // 改行を追加したフォーマット文字列を作成
            let newline_format = "%s\n";
            let format_str = self.context.const_string(newline_format.as_bytes(), true);
            let format_global = self.module.add_global(format_str.get_type(), None, "printf_format");
            format_global.set_initializer(&format_str);
            format_global.set_constant(true);

            let format_ptr = format_global.as_pointer_value();
            self.builder.build_call(printf_fn, &[format_ptr.into(), format_arg.into()], "println_call")?;
        } else {
            // 複数の引数がある場合 - すべての値を文字列として連結して出力
            let printf_fn = self.runtime_manager.get_function("printf")
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: "printf function not found".to_string(),
                }))?;

            // すべての引数を文字列に変換して連結
            let mut result_str = format_arg;
            for arg in args.iter().skip(1) {
                let arg_value = self.compile_expression(arg)?;
                let arg_str = self.value_to_string(arg_value)?;
                
                // 文字列連結
                let concat_fn = self.runtime_manager.get_function("yuni_string_concat")
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: "yuni_string_concat function not found".to_string(),
                    }))?;
                
                result_str = self.builder.build_call(
                    concat_fn, 
                    &[result_str.into(), arg_str.into()], 
                    "concat_result"
                )?.try_as_basic_value().left().unwrap();
            }

            // 改行を追加
            let newline_str = self.context.const_string(b"\n", true);
            let newline_global = self.module.add_global(newline_str.get_type(), None, "newline");
            newline_global.set_initializer(&newline_str);
            newline_global.set_constant(true);
            let newline_ptr = newline_global.as_pointer_value();
            
            let concat_fn = self.runtime_manager.get_function("yuni_string_concat")
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: "yuni_string_concat function not found".to_string(),
                }))?;
            
            result_str = self.builder.build_call(
                concat_fn, 
                &[result_str.into(), newline_ptr.into()], 
                "concat_newline"
            )?.try_as_basic_value().left().unwrap();

            // %s形式で出力
            let format_str = self.context.const_string(b"%s", true);
            let format_global = self.module.add_global(format_str.get_type(), None, "printf_format");
            format_global.set_initializer(&format_str);
            format_global.set_constant(true);

            let format_ptr = format_global.as_pointer_value();
            self.builder.build_call(printf_fn, &[format_ptr.into(), result_str.into()], "println_call")?;
        }

        Ok(self.context.i32_type().const_zero().into())
    }

    /// メソッド呼び出し式をコンパイル
    pub fn compile_method_call(&mut self, method_call: &MethodCallExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // オブジェクトの式をコンパイル
        let object_value = self.compile_expression(&method_call.object)?;
        
        // オブジェクトの型を推論
        let object_type = self.expression_type(&method_call.object)?;
        
        // 構造体名を取得
        let struct_name = match &object_type {
            Type::UserDefined(name) => name.clone(),
            Type::Reference(inner, _is_mut) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name.clone()
                } else {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Method call on non-struct type".to_string(),
                        span: method_call.span,
                    }));
                }
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Method call on non-struct type".to_string(),
                    span: method_call.span,
                }));
            }
        };
        
        // 構造体のメソッドを検索
        let methods = self.struct_methods.get(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("No methods found for type {}", struct_name),
                span: method_call.span,
            }))?;
        
        // メソッドを探す
        let (_, mangled_name) = methods.iter()
            .find(|(method_name, _)| method_name == &method_call.method)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("Method '{}' not found for type '{}'", method_call.method, struct_name),
                span: method_call.span,
            }))?;
        
        // 関数を取得（コピーして借用を解放）
        let func = *self.functions.get(mangled_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Method function '{}' not found", mangled_name),
            }))?;
        
        // 引数を準備（最初の引数はレシーバー）
        let mut args = vec![object_value.into()];
        
        // 残りの引数をコンパイル
        for arg in &method_call.args {
            let arg_value = self.compile_expression(arg)?;
            args.push(arg_value.into());
        }
        
        // メソッドを呼び出し
        let call_result = self.builder.build_call(func, &args, "method_call_result")?;
        
        if let Some(value) = call_result.try_as_basic_value().left() {
            Ok(value)
        } else {
            // voidを返す関数の場合
            Ok(self.context.i32_type().const_zero().into())
        }
    }

    /// インデックスアクセス式をコンパイル
    pub fn compile_index_expr(&mut self, index: &IndexExpr) -> YuniResult<BasicValueEnum<'ctx>> {
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
                
                // GEPで要素のアドレスを計算
                let element_ptr = unsafe {
                    self.builder.build_gep(
                        element_llvm_type,
                        array_ptr,
                        &[index_int],
                        "element_ptr"
                    )?
                };
                
                // 値をロード
                let value = self.builder.build_load(
                    element_llvm_type,
                    element_ptr,
                    "element_value"
                )?;
                
                Ok(value)
            }
            Type::String => {
                // 文字列のインデックスアクセス（文字を取得）
                // TODO: 実装が必要
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "String indexing not yet implemented".to_string(),
                    span: index.span,
                }))
            }
            Type::Generic(name, type_args) if name == "Vec" && type_args.len() == 1 => {
                // Vecのインデックスアクセス
                let element_type = &type_args[0];
                let vec_ptr = object_value.into_pointer_value();
                
                // インデックスが整数型であることを確認
                let index_int = index_value.into_int_value();
                
                // 要素のLLVM型を取得
                let element_llvm_type = self.type_manager.ast_type_to_llvm(element_type)?;
                
                // vec_getヘルパー関数を使用
                let value = self.vec_get(vec_ptr, index_int, element_llvm_type)?;
                
                Ok(value)
            }
            _ => {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: format!("Cannot index into type: {:?}", object_type),
                    span: index.span,
                }))
            }
        }
    }

    /// フィールドアクセス式をコンパイル
    pub fn compile_field_expr(&mut self, field: &FieldExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // オブジェクトの式をコンパイル
        let object_value = self.compile_expression(&field.object)?;
        
        // オブジェクトの型を推論
        let object_type = self.expression_type(&field.object)?;
        
        // 構造体名を取得
        let struct_name = match &object_type {
            Type::UserDefined(name) => name.clone(),
            Type::Reference(inner, _) => {
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
        
        // 構造体値からフィールドを抽出
        match object_value {
            BasicValueEnum::StructValue(struct_val) => {
                // 直接構造体値の場合
                let field_value = self.builder.build_extract_value(
                    struct_val,
                    field_index,
                    &field.field
                )?;
                Ok(field_value)
            }
            BasicValueEnum::PointerValue(ptr_val) => {
                // ポインタの場合はGEPを使用
                let struct_type = self.type_manager.get_struct(&struct_name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: format!("Struct type not found for {}", struct_name),
                    }))?;
                
                let indices = [
                    self.context.i32_type().const_zero(),
                    self.context.i32_type().const_int(field_index as u64, false),
                ];
                
                let field_ptr = unsafe {
                    self.builder.build_in_bounds_gep(
                        struct_type,
                        ptr_val,
                        &indices,
                        &format!("{}_ptr", field.field),
                    )?
                };
                
                // フィールドの型を取得
                let field_type = struct_info.get_field_type(field_index as usize)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: format!("Field type not found for index {}", field_index),
                    }))?;
                let llvm_field_type = self.type_manager.ast_type_to_llvm(field_type)?;
                
                // フィールドの値をロード
                let field_value = self.builder.build_load(
                    llvm_field_type,
                    field_ptr,
                    &field.field
                )?;
                
                Ok(field_value)
            }
            _ => {
                Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Invalid object type for field access".to_string(),
                    span: field.span,
                }))
            }
        }
    }
}