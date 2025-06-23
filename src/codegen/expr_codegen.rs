//! 式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::{FloatPredicate, IntPredicate};
use inkwell::types::BasicTypeEnum;

use super::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 式をコンパイル（期待される型のコンテキストなし）
    pub fn compile_expression(&mut self, expr: &Expression) -> YuniResult<BasicValueEnum<'ctx>> {
        self.compile_expression_with_type(expr, None)
    }

    /// 式をコンパイル（期待される型のコンテキスト付き）
    pub fn compile_expression_with_type(&mut self, expr: &Expression, expected_type: Option<&Type>) -> YuniResult<BasicValueEnum<'ctx>> {
        match expr {
            Expression::Integer(lit) => self.compile_integer_literal_with_type(lit, expected_type),
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
            Expression::Match(match_expr) => self.compile_match_expr(match_expr),
            Expression::If(if_expr) => self.compile_if_expr(if_expr),
            Expression::Block(block_expr) => self.compile_block_expr(block_expr),
        }
    }

    /// 整数リテラルをコンパイル（期待される型のコンテキストなし）
    pub fn compile_integer_literal(&self, lit: &IntegerLit) -> YuniResult<BasicValueEnum<'ctx>> {
        self.compile_integer_literal_with_type(lit, None)
    }

    /// 整数リテラルをコンパイル（期待される型のコンテキスト付き）
    pub fn compile_integer_literal_with_type(&self, lit: &IntegerLit, expected_type: Option<&Type>) -> YuniResult<BasicValueEnum<'ctx>> {
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
                _ => self.context.i32_type(), // デフォルト
            }
        } else {
            // 期待される型が指定されている場合はそれを使用
            if let Some(expected) = expected_type {
                match expected {
                    Type::I8 => self.context.i8_type(),
                    Type::I16 => self.context.i16_type(),
                    Type::I32 => self.context.i32_type(),
                    Type::I64 => self.context.i64_type(),
                    Type::I128 => self.context.i128_type(),
                    Type::U8 => self.context.i8_type(),
                    Type::U16 => self.context.i16_type(),
                    Type::U32 => self.context.i32_type(),
                    Type::U64 => self.context.i64_type(),
                    Type::U128 => self.context.i128_type(),
                    _ => self.context.i32_type(), // 整数型でない場合はデフォルト
                }
            } else {
                self.context.i32_type() // デフォルトはi32（Rustと同じ）
            }
        };

        Ok(int_type.const_int(lit.value as u64, false).into())
    }

    /// 浮動小数点リテラルをコンパイル
    pub fn compile_float_literal(&self, lit: &FloatLit) -> YuniResult<BasicValueEnum<'ctx>> {
        let float_type = if let Some(suffix) = &lit.suffix {
            match suffix.as_str() {
                "f32" => self.context.f32_type(),
                "f64" => self.context.f64_type(),
                _ => self.context.f64_type(), // デフォルト
            }
        } else {
            self.context.f64_type() // デフォルトはf64
        };

        Ok(float_type.const_float(lit.value).into())
    }

    /// 文字列リテラルをコンパイル
    pub fn compile_string_literal(&self, lit: &StringLit) -> YuniResult<BasicValueEnum<'ctx>> {
        let string_const = self.context.const_string(lit.value.as_bytes(), true);
        let global = self.module.add_global(string_const.get_type(), None, "str");
        global.set_initializer(&string_const);
        global.set_constant(true);

        let array_type = self.context
            .i8_type()
            .array_type(lit.value.len() as u32 + 1);
        let indices = [
            self.context.i32_type().const_zero(),
            self.context.i32_type().const_zero(),
        ];
        
        let ptr = unsafe {
            self.builder.build_in_bounds_gep(
                array_type,
                global.as_pointer_value(),
                &indices,
                "str_ptr",
            )?
        };

        Ok(ptr.into())
    }

    /// テンプレート文字列をコンパイル
    pub fn compile_template_string(&mut self, lit: &TemplateStringLit) -> YuniResult<BasicValueEnum<'ctx>> {
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

            result = match result {
                None => Some(part_str),
                Some(prev) => {
                    let concat_fn = self.runtime_manager.get_function("yuni_string_concat")
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                            message: "Runtime function yuni_string_concat not found".to_string(),
                        }))?;
                    Some(self.builder.build_call(
                        concat_fn,
                        &[prev.into(), part_str.into()],
                        "concat_result",
                    )?.try_as_basic_value().left().unwrap())
                }
            };
        }

        result.ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
            message: "Empty template string".to_string(),
        }))
    }

    /// 真偽値リテラルをコンパイル
    pub fn compile_boolean_literal(&self, lit: &BooleanLit) -> YuniResult<BasicValueEnum<'ctx>> {
        Ok(self.context.bool_type().const_int(lit.value as u64, false).into())
    }

    /// 識別子をコンパイル
    pub fn compile_identifier(&mut self, id: &Identifier) -> YuniResult<BasicValueEnum<'ctx>> {
        let symbol = self.scope_manager.lookup(&id.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: id.name.clone(),
                span: id.span,
            }))?;

        let value = self.builder.build_load(
            self.type_manager.ast_type_to_llvm(&symbol.ty)?,
            symbol.ptr,
            &id.name,
        )?;

        Ok(value)
    }

    /// パス式をコンパイル
    pub fn compile_path(&mut self, path: &PathExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        if path.segments.len() == 1 {
            let name = &path.segments[0];
            
            // 関数を探す
            if let Some(func) = self.functions.get(name) {
                return Ok(func.as_global_value().as_pointer_value().into());
            }
            
            // 変数として扱う
            return self.compile_identifier(&Identifier {
                name: name.clone(),
                span: path.span,
            });
        } else if path.segments.len() == 2 {
            // Enum::Variant のパターンを処理
            // これはEnumVariantExprとして処理されるべきだが、
            // 現在は単純にその形式でコンパイル
            let enum_variant = EnumVariantExpr {
                enum_name: path.segments[0].clone(),
                variant: path.segments[1].clone(),
                fields: crate::ast::EnumVariantFields::Unit,
                span: path.span,
            };
            return self.compile_enum_variant(&enum_variant);
        }
        
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Multi-segment paths not yet implemented".to_string(),
            span: path.span,
        }))
    }

    /// 二項演算式をコンパイル
    pub fn compile_binary_expr(&mut self, binary: &BinaryExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let left = self.compile_expression(&binary.left)?;
        let right = self.compile_expression(&binary.right)?;

        match (&binary.op, left, right) {
            // 整数演算
            (op, BasicValueEnum::IntValue(left_int), BasicValueEnum::IntValue(right_int)) => {
                // 型が異なる場合は型変換を行う
                let (left_int, right_int) = if left_int.get_type() != right_int.get_type() {
                    self.coerce_int_types(left_int, right_int, binary.span)?
                } else {
                    (left_int, right_int)
                };

                let result = match op {
                    BinaryOp::Add => self.builder.build_int_add(left_int, right_int, "add")?,
                    BinaryOp::Subtract => self.builder.build_int_sub(left_int, right_int, "sub")?,
                    BinaryOp::Multiply => self.builder.build_int_mul(left_int, right_int, "mul")?,
                    BinaryOp::Divide => {
                        // TODO: 符号付き/符号なしの区別
                        self.builder.build_int_signed_div(left_int, right_int, "div")?
                    }
                    BinaryOp::Modulo => self.builder.build_int_signed_rem(left_int, right_int, "rem")?,
                    BinaryOp::Lt => self.builder.build_int_compare(IntPredicate::SLT, left_int, right_int, "lt")?,
                    BinaryOp::Gt => self.builder.build_int_compare(IntPredicate::SGT, left_int, right_int, "gt")?,
                    BinaryOp::Le => self.builder.build_int_compare(IntPredicate::SLE, left_int, right_int, "le")?,
                    BinaryOp::Ge => self.builder.build_int_compare(IntPredicate::SGE, left_int, right_int, "ge")?,
                    BinaryOp::Eq => self.builder.build_int_compare(IntPredicate::EQ, left_int, right_int, "eq")?,
                    BinaryOp::Ne => self.builder.build_int_compare(IntPredicate::NE, left_int, right_int, "ne")?,
                    BinaryOp::And => self.builder.build_and(left_int, right_int, "and")?,
                    BinaryOp::Or => self.builder.build_or(left_int, right_int, "or")?,
                    // ビット演算子は現在定義されていない
                    _ => return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: format!("Unsupported binary operation: {:?}", op),
                        span: binary.span,
                    })),
                };
                Ok(result.into())
            }
            
            // 浮動小数点演算
            (op, BasicValueEnum::FloatValue(left_float), BasicValueEnum::FloatValue(right_float)) => {
                // 型が異なる場合は型変換を行う
                let (left_float, right_float) = if left_float.get_type() != right_float.get_type() {
                    self.coerce_float_types(left_float, right_float)?
                } else {
                    (left_float, right_float)
                };
                
                match op {
                    BinaryOp::Add => Ok(self.builder.build_float_add(left_float, right_float, "fadd")?.into()),
                    BinaryOp::Subtract => Ok(self.builder.build_float_sub(left_float, right_float, "fsub")?.into()),
                    BinaryOp::Multiply => Ok(self.builder.build_float_mul(left_float, right_float, "fmul")?.into()),
                    BinaryOp::Divide => Ok(self.builder.build_float_div(left_float, right_float, "fdiv")?.into()),
                    BinaryOp::Modulo => Ok(self.builder.build_float_rem(left_float, right_float, "frem")?.into()),
                    BinaryOp::Lt => Ok(self.builder.build_float_compare(FloatPredicate::OLT, left_float, right_float, "flt")?.into()),
                    BinaryOp::Gt => Ok(self.builder.build_float_compare(FloatPredicate::OGT, left_float, right_float, "fgt")?.into()),
                    BinaryOp::Le => Ok(self.builder.build_float_compare(FloatPredicate::OLE, left_float, right_float, "fle")?.into()),
                    BinaryOp::Ge => Ok(self.builder.build_float_compare(FloatPredicate::OGE, left_float, right_float, "fge")?.into()),
                    BinaryOp::Eq => Ok(self.builder.build_float_compare(FloatPredicate::OEQ, left_float, right_float, "feq")?.into()),
                    BinaryOp::Ne => Ok(self.builder.build_float_compare(FloatPredicate::ONE, left_float, right_float, "fne")?.into()),
                    _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: format!("Invalid operation {:?} for float types", op),
                        span: binary.span,
                    })),
                }
            }
            
            _ => Err(YuniError::Codegen(CodegenError::TypeError {
                expected: "numeric types".to_string(),
                actual: "non-numeric types".to_string(),
                span: binary.span,
            })),
        }
    }

    /// 単項演算式をコンパイル
    pub fn compile_unary_expr(&mut self, unary: &UnaryExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let operand = self.compile_expression(&unary.expr)?;

        match (&unary.op, operand) {
            (UnaryOp::Not, BasicValueEnum::IntValue(int_val)) => {
                Ok(self.builder.build_not(int_val, "not")?.into())
            }
            (UnaryOp::Negate, BasicValueEnum::IntValue(int_val)) => {
                Ok(self.builder.build_int_neg(int_val, "neg")?.into())
            }
            (UnaryOp::Negate, BasicValueEnum::FloatValue(float_val)) => {
                Ok(self.builder.build_float_neg(float_val, "fneg")?.into())
            }
            // ビット反転演算子は現在定義されていない
            _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                message: format!("Invalid unary operation {:?}", unary.op),
                span: unary.span,
            })),
        }
    }

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
        let call_result = self.builder.build_call(func, &args, "call_result")?;
        
        if let Some(value) = call_result.try_as_basic_value().left() {
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

        // 最初の引数をフォーマット文字列として使用
        let format_arg = self.compile_expression(&args[0])?;
        
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
            // 複数の引数がある場合（簡易実装）
            let printf_fn = self.runtime_manager.get_function("printf")
                .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                    message: "printf function not found".to_string(),
                }))?;

            let mut printf_args = vec![format_arg.into()];
            for arg in args.iter().skip(1) {
                let arg_value = self.compile_expression(arg)?;
                printf_args.push(arg_value.into());
            }

            self.builder.build_call(printf_fn, &printf_args, "println_call")?;
        }

        Ok(self.context.i32_type().const_zero().into())
    }

    /// メソッド呼び出し式をコンパイル
    pub fn compile_method_call(&mut self, method_call: &MethodCallExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Method calls not yet implemented".to_string(),
            span: method_call.span,
        }))
    }

    /// インデックスアクセス式をコンパイル
    pub fn compile_index_expr(&mut self, index: &IndexExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Index access not yet implemented".to_string(),
            span: index.span,
        }))
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

    /// 参照式をコンパイル
    pub fn compile_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Reference expressions not yet implemented".to_string(),
            span: ref_expr.span,
        }))
    }

    /// デリファレンス式をコンパイル
    pub fn compile_dereference_expr(&mut self, deref: &DereferenceExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Dereference expressions not yet implemented".to_string(),
            span: deref.span,
        }))
    }

    /// 構造体リテラルをコンパイル
    pub fn compile_struct_literal(&mut self, struct_lit: &StructLiteral) -> YuniResult<BasicValueEnum<'ctx>> {
        // 構造体型を取得
        let struct_type = self.type_manager.get_struct(&struct_lit.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: struct_lit.name.clone(),
                span: struct_lit.span,
            }))?;

        // 構造体情報を取得してクローン（借用チェッカーエラーを回避）
        let struct_info = self.struct_info.get(&struct_lit.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_lit.name),
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
        // データを持たないバリアントのみ現在サポート
        match &enum_var.fields {
            crate::ast::EnumVariantFields::Unit => {
                // バリアントのインデックスを取得
                let key = (enum_var.enum_name.clone(), enum_var.variant.clone());
                let variant_index = self.enum_variants.get(&key)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("{}::{}", enum_var.enum_name, enum_var.variant),
                        span: enum_var.span,
                    }))?;
                
                // i32の定数として返す
                Ok(self.context.i32_type().const_int(*variant_index as u64, false).into())
            }
            _ => {
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Enum variants with data not yet implemented".to_string(),
                    span: enum_var.span,
                }))
            }
        }
    }

    /// 配列式をコンパイル
    pub fn compile_array_expr(&mut self, array: &ArrayExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Array expressions not yet implemented".to_string(),
            span: array.span,
        }))
    }

    /// タプル式をコンパイル
    pub fn compile_tuple_expr(&mut self, tuple: &TupleExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Tuple expressions not yet implemented".to_string(),
            span: tuple.span,
        }))
    }

    /// キャスト式をコンパイル
    pub fn compile_cast_expr(&mut self, cast: &CastExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let value = self.compile_expression(&cast.expr)?;
        let target_type = self.type_manager.ast_type_to_llvm(&cast.ty)?;
        
        match (value, target_type) {
            // 整数から整数へのキャスト
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) => {
                let source_bits = int_val.get_type().get_bit_width();
                let target_bits = target_int_type.get_bit_width();
                
                use std::cmp::Ordering;
                match source_bits.cmp(&target_bits) {
                    Ordering::Equal => Ok(int_val.into()),
                    Ordering::Less => {
                        // 拡張
                        if self.is_signed_type(source_bits) {
                            Ok(self.builder.build_int_s_extend(int_val, target_int_type, "sext")?.into())
                        } else {
                            Ok(self.builder.build_int_z_extend(int_val, target_int_type, "zext")?.into())
                        }
                    }
                    Ordering::Greater => {
                        // 切り詰め
                        Ok(self.builder.build_int_truncate(int_val, target_int_type, "trunc")?.into())
                    }
                }
            }
            
            // 整数から浮動小数点へのキャスト
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::FloatType(target_float_type)) => {
                if self.is_signed_type(int_val.get_type().get_bit_width()) {
                    Ok(self.builder.build_signed_int_to_float(int_val, target_float_type, "sitofp")?.into())
                } else {
                    Ok(self.builder.build_unsigned_int_to_float(int_val, target_float_type, "uitofp")?.into())
                }
            }
            
            // 浮動小数点から整数へのキャスト
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::IntType(target_int_type)) => {
                if self.is_signed_type(target_int_type.get_bit_width()) {
                    Ok(self.builder.build_float_to_signed_int(float_val, target_int_type, "fptosi")?.into())
                } else {
                    Ok(self.builder.build_float_to_unsigned_int(float_val, target_int_type, "fptoui")?.into())
                }
            }
            
            // 浮動小数点から浮動小数点へのキャスト
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float_type)) => {
                let source_type = float_val.get_type();
                if source_type == target_float_type {
                    Ok(float_val.into())
                } else if source_type == self.context.f32_type() && target_float_type == self.context.f64_type() {
                    Ok(self.builder.build_float_ext(float_val, target_float_type, "fpext")?.into())
                } else if source_type == self.context.f64_type() && target_float_type == self.context.f32_type() {
                    Ok(self.builder.build_float_trunc(float_val, target_float_type, "fptrunc")?.into())
                } else {
                    Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: format!("Unsupported float cast from {:?} to {:?}", source_type, target_float_type),
                        span: cast.span,
                    }))
                }
            }
            
            _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                message: format!("Unsupported cast from {:?} to {:?}", value, target_type),
                span: cast.span,
            }))
        }
    }

    /// 代入式をコンパイル
    pub fn compile_assignment_expr(&mut self, assign: &AssignmentExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Assignment expressions not yet implemented".to_string(),
            span: assign.span,
        }))
    }

    /// match式をコンパイル
    pub fn compile_match_expr(&mut self, match_expr: &MatchExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Match expressions not yet implemented".to_string(),
            span: match_expr.span,
        }))
    }

    /// if式をコンパイル
    pub fn compile_if_expr(&mut self, if_expr: &IfExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        let condition = self.compile_expression(&if_expr.condition)?;
        
        // 条件を bool に変換
        let condition_bool = match condition {
            BasicValueEnum::IntValue(int_val) => {
                if int_val.get_type().get_bit_width() == 1 {
                    int_val
                } else {
                    // 非ゼロかどうかで判定
                    let zero = int_val.get_type().const_zero();
                    self.builder.build_int_compare(IntPredicate::NE, int_val, zero, "condition")?
                }
            }
            _ => return Err(YuniError::Codegen(CodegenError::TypeError {
                expected: "bool".to_string(),
                actual: "non-bool".to_string(),
                span: if_expr.span,
            })),
        };

        let function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
        let then_bb = self.context.append_basic_block(function, "then");
        let else_bb = self.context.append_basic_block(function, "else");
        let merge_bb = self.context.append_basic_block(function, "merge");

        // 条件分岐
        self.builder.build_conditional_branch(condition_bool, then_bb, else_bb)?;

        // then ブロック
        self.builder.position_at_end(then_bb);
        let then_value = self.compile_expression(&if_expr.then_branch)?;
        self.builder.build_unconditional_branch(merge_bb)?;
        let then_bb = self.builder.get_insert_block().unwrap();

        // else ブロック
        self.builder.position_at_end(else_bb);
        let else_value = if let Some(else_branch) = &if_expr.else_branch {
            self.compile_expression(else_branch)?
        } else {
            // else句がない場合はunit値
            self.context.i32_type().const_zero().into()
        };
        self.builder.build_unconditional_branch(merge_bb)?;
        let else_bb = self.builder.get_insert_block().unwrap();

        // merge ブロック
        self.builder.position_at_end(merge_bb);
        
        // 両方のブランチで同じ型の値を返す必要がある
        if then_value.get_type() == else_value.get_type() {
            let phi = self.builder.build_phi(then_value.get_type(), "if_result")?;
            phi.add_incoming(&[(&then_value, then_bb), (&else_value, else_bb)]);
            Ok(phi.as_basic_value())
        } else {
            // 型が異なる場合はunit値を返す
            Ok(self.context.i32_type().const_zero().into())
        }
    }

    /// ブロック式をコンパイル
    pub fn compile_block_expr(&mut self, block_expr: &BlockExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 新しいスコープを作成
        self.scope_manager.push_scope();
        
        let mut last_value: BasicValueEnum = self.context.i32_type().const_zero().into();
        
        // ブロック内の文を順次コンパイル
        for stmt in &block_expr.statements {
            self.compile_statement(stmt)?;
        }
        
        // 最後の式がある場合はその値を返す
        if let Some(last_expr) = &block_expr.last_expr {
            last_value = self.compile_expression(last_expr)?;
        }
        
        // スコープを終了
        self.scope_manager.pop_scope();
        
        Ok(last_value)
    }

    /// 値を文字列に変換
    pub fn value_to_string(&mut self, value: BasicValueEnum<'ctx>) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Ok(value)
    }

    /// 値を指定された型に変換
    fn coerce_to_type(
        &self, 
        value: BasicValueEnum<'ctx>, 
        target_type: BasicTypeEnum<'ctx>,
        span: Span
    ) -> YuniResult<BasicValueEnum<'ctx>> {
        match (value, target_type) {
            // 整数から整数への変換
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) => {
                let source_type = int_val.get_type();
                if source_type == target_int_type {
                    Ok(int_val.into())
                } else {
                    let source_bits = source_type.get_bit_width();
                    let target_bits = target_int_type.get_bit_width();
                    
                    if source_bits < target_bits {
                        // 拡張
                        if self.is_signed_type(source_bits) {
                            Ok(self.builder.build_int_s_extend(int_val, target_int_type, "sext")?.into())
                        } else {
                            Ok(self.builder.build_int_z_extend(int_val, target_int_type, "zext")?.into())
                        }
                    } else {
                        // 切り詰め
                        Ok(self.builder.build_int_truncate(int_val, target_int_type, "trunc")?.into())
                    }
                }
            }
            // 浮動小数点から浮動小数点への変換
            (BasicValueEnum::FloatValue(float_val), BasicTypeEnum::FloatType(target_float_type)) => {
                let source_type = float_val.get_type();
                if source_type == target_float_type {
                    Ok(float_val.into())
                } else if source_type == self.context.f32_type() && target_float_type == self.context.f64_type() {
                    Ok(self.builder.build_float_ext(float_val, target_float_type, "fpext")?.into())
                } else if source_type == self.context.f64_type() && target_float_type == self.context.f32_type() {
                    Ok(self.builder.build_float_trunc(float_val, target_float_type, "fptrunc")?.into())
                } else {
                    Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: format!("Unsupported float coercion from {:?} to {:?}", source_type, target_float_type),
                        span,
                    }))
                }
            }
            // 同じ型の場合はそのまま返す
            _ => {
                if value.get_type() == target_type {
                    Ok(value)
                } else {
                    Err(YuniError::Codegen(CodegenError::TypeError {
                        expected: format!("{:?}", target_type),
                        actual: format!("{:?}", value.get_type()),
                        span,
                    }))
                }
            }
        }
    }

    /// 整数型の強制変換を行う
    /// 異なるビット幅の整数型を同じ型に変換する
    fn coerce_int_types(
        &self, 
        left: inkwell::values::IntValue<'ctx>, 
        right: inkwell::values::IntValue<'ctx>,
        _span: Span
    ) -> YuniResult<(inkwell::values::IntValue<'ctx>, inkwell::values::IntValue<'ctx>)> {
        let left_bits = left.get_type().get_bit_width();
        let right_bits = right.get_type().get_bit_width();
        
        if left_bits == right_bits {
            return Ok((left, right));
        }
        
        // より大きい型に合わせる
        if left_bits > right_bits {
            // rightをleftの型に拡張
            let extended = if self.is_signed_type(right_bits) {
                self.builder.build_int_s_extend(right, left.get_type(), "sext")?
            } else {
                self.builder.build_int_z_extend(right, left.get_type(), "zext")?
            };
            Ok((left, extended))
        } else {
            // leftをrightの型に拡張
            let extended = if self.is_signed_type(left_bits) {
                self.builder.build_int_s_extend(left, right.get_type(), "sext")?
            } else {
                self.builder.build_int_z_extend(left, right.get_type(), "zext")?
            };
            Ok((extended, right))
        }
    }
    
    /// 整数型が符号付きかどうかを判定
    fn is_signed_type(&self, _bit_width: u32) -> bool {
        // TODO: 実際の型情報から符号の有無を判定すべき
        // 現在は簡易実装として、すべて符号付きとして扱う
        true
    }
    
    /// 浮動小数点型の強制変換を行う
    fn coerce_float_types(
        &self,
        left: inkwell::values::FloatValue<'ctx>,
        right: inkwell::values::FloatValue<'ctx>,
    ) -> YuniResult<(inkwell::values::FloatValue<'ctx>, inkwell::values::FloatValue<'ctx>)> {
        let left_type = left.get_type();
        let right_type = right.get_type();
        
        if left_type == right_type {
            return Ok((left, right));
        }
        
        // f64型を優先する（より精度が高い）
        if left_type == self.context.f64_type() {
            let extended = self.builder.build_float_ext(right, left_type, "fpext")?;
            Ok((left, extended))
        } else if right_type == self.context.f64_type() {
            let extended = self.builder.build_float_ext(left, right_type, "fpext")?;
            Ok((extended, right))
        } else {
            // どちらもf64でない場合はそのまま返す（エラーになるかもしれない）
            Ok((left, right))
        }
    }

    /// 式の型を推論する
    pub fn expression_type(&mut self, expr: &Expression) -> YuniResult<Type> {
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
                        _ => Ok(Type::I32), // デフォルト
                    }
                } else {
                    Ok(Type::I32) // デフォルトはi32（Rustと同じ）
                }
            }
            Expression::Float(lit) => {
                if let Some(suffix) = &lit.suffix {
                    match suffix.as_str() {
                        "f32" => Ok(Type::F32),
                        "f64" => Ok(Type::F64),
                        _ => Ok(Type::F64), // デフォルト
                    }
                } else {
                    Ok(Type::F64) // デフォルト
                }
            }
            Expression::String(_) => Ok(Type::String),
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::Identifier(id) => {
                if let Some(symbol) = self.scope_manager.lookup(&id.name) {
                    Ok(symbol.ty.clone())
                } else {
                    Err(YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))
                }
            }
            Expression::Path(path) => {
                if path.segments.len() == 1 {
                    let name = &path.segments[0];
                    
                    // 関数を探す
                    if self.functions.contains_key(name) {
                        // 関数ポインタ型として扱う（簡易実装）
                        return Ok(Type::UserDefined(format!("fn_{}", name)));
                    }
                    
                    // 変数として扱う
                    if let Some(symbol) = self.scope_manager.lookup(name) {
                        Ok(symbol.ty.clone())
                    } else {
                        Err(YuniError::Codegen(CodegenError::Undefined {
                            name: name.clone(),
                            span: path.span,
                        }))
                    }
                } else {
                    Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Multi-segment path type inference not implemented".to_string(),
                        span: path.span,
                    }))
                }
            }
            Expression::Binary(binary) => {
                let left_type = self.expression_type(&binary.left)?;
                let right_type = self.expression_type(&binary.right)?;
                
                match &binary.op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                        if left_type == right_type {
                            Ok(left_type)
                        } else {
                            // 型の自動昇格をサポート（簡易実装）
                            Ok(left_type)
                        }
                    }
                    BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge | BinaryOp::Eq | BinaryOp::Ne |
                    BinaryOp::And | BinaryOp::Or => Ok(Type::Bool),
                    _ => Ok(left_type),
                }
            }
            Expression::Unary(unary) => {
                let operand_type = self.expression_type(&unary.expr)?;
                match &unary.op {
                    UnaryOp::Not => Ok(Type::Bool),
                    UnaryOp::Negate => Ok(operand_type),
                    _ => Ok(operand_type),
                }
            }
            Expression::Call(call) => {
                let func_name = match call.callee.as_ref() {
                    Expression::Identifier(id) => &id.name,
                    Expression::Path(path) if path.segments.len() == 1 => &path.segments[0],
                    _ => return Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Complex function call type inference not implemented".to_string(),
                        span: call.span,
                    })),
                };
                
                // println の特別な処理
                if func_name == "println" {
                    return Ok(Type::I32); // printlnは実際にはi32(0)を返すので
                }
                
                // 関数の戻り値型を取得
                if let Some(return_type) = self.function_types.get(func_name) {
                    // Void型の関数は実際にはunit値（i32(0)）を返すため、
                    // 型推論ではI32として扱う
                    if matches!(return_type, Type::Void) {
                        Ok(Type::I32)
                    } else {
                        Ok(return_type.clone())
                    }
                } else {
                    // 関数が見つからない場合はエラー
                    Err(YuniError::Codegen(CodegenError::Undefined {
                        name: func_name.clone(),
                        span: call.span,
                    }))
                }
            }
            Expression::If(if_expr) => {
                // if式の場合、then/elseブランチの型から推論
                let then_type = self.expression_type(&if_expr.then_branch)?;
                if let Some(else_branch) = &if_expr.else_branch {
                    let else_type = self.expression_type(else_branch)?;
                    // 両方の型が同じならその型を返す
                    if then_type == else_type {
                        Ok(then_type)
                    } else {
                        // 型が異なる場合はunit型
                        Ok(Type::I32) // unit型の代わりにi32(0)を使用
                    }
                } else {
                    // elseブランチがない場合はunit型
                    Ok(Type::I32) // unit型の代わりにi32(0)を使用
                }
            }
            Expression::Block(block_expr) => {
                // ブロック式の場合、最後の式の型を返す
                if let Some(last_expr) = &block_expr.last_expr {
                    self.expression_type(last_expr)
                } else {
                    // 最後の式がない場合はunit型
                    Ok(Type::I32) // unit型の代わりにi32(0)を使用
                }
            }
            Expression::StructLit(struct_lit) => {
                // 構造体リテラルの型は構造体名から決まる
                Ok(Type::UserDefined(struct_lit.name.clone()))
            }
            Expression::Field(field_expr) => {
                // フィールドアクセスの型推論
                let object_type = self.expression_type(&field_expr.object)?;
                
                let struct_name = match &object_type {
                    Type::UserDefined(name) => name.clone(),
                    Type::Reference(inner, _) => {
                        if let Type::UserDefined(name) = inner.as_ref() {
                            name.clone()
                        } else {
                            return Err(YuniError::Codegen(CodegenError::InvalidType {
                                message: "Field access on non-struct type".to_string(),
                                span: field_expr.span,
                            }));
                        }
                    }
                    _ => {
                        return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "Field access on non-struct type".to_string(),
                            span: field_expr.span,
                        }));
                    }
                };
                
                let struct_info = self.struct_info.get(&struct_name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: format!("Struct info not found for {}", struct_name),
                    }))?;
                
                let field_index = struct_info.get_field_index(&field_expr.field)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("{}.{}", struct_name, field_expr.field),
                        span: field_expr.span,
                    }))?;
                
                let field_type = struct_info.get_field_type(field_index as usize)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: format!("Field type not found for index {}", field_index),
                    }))?;
                
                Ok(field_type.clone())
            }
            Expression::EnumVariant(enum_variant) => {
                // Enumバリアントの型はEnum自体の型
                Ok(Type::UserDefined(enum_variant.enum_name.clone()))
            }
            _ => Err(YuniError::Codegen(CodegenError::Unimplemented {
                feature: "Type inference not implemented for this expression".to_string(),
                span: expr.span(),
            })),
        }
    }
}