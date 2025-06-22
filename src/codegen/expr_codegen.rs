//! 式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
use std::collections::HashMap;

use super::codegen::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 式をコンパイル
    pub fn compile_expression(&mut self, expr: &Expression) -> YuniResult<BasicValueEnum<'ctx>> {
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
            Expression::Match(match_expr) => self.compile_match_expr(match_expr),
            Expression::If(if_expr) => self.compile_if_expr(if_expr),
            Expression::Block(block_expr) => self.compile_block_expr(block_expr),
        }
    }

    /// 整数リテラルをコンパイル
    pub fn compile_integer_literal(&self, lit: &IntegerLit) -> YuniResult<BasicValueEnum<'ctx>> {
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
                _ => self.context.i64_type(), // デフォルト
            }
        } else {
            self.context.i64_type() // デフォルトはi64
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
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Function calls not yet implemented".to_string(),
            span: call.span,
        }))
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
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Field access not yet implemented".to_string(),
            span: field.span,
        }))
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
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Struct literals not yet implemented".to_string(),
            span: struct_lit.span,
        }))
    }

    /// 列挙型バリアントをコンパイル
    pub fn compile_enum_variant(&mut self, enum_var: &EnumVariantExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Enum variants not yet implemented".to_string(),
            span: enum_var.span,
        }))
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
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Cast expressions not yet implemented".to_string(),
            span: cast.span,
        }))
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
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "If expressions not yet implemented".to_string(),
            span: if_expr.span,
        }))
    }

    /// ブロック式をコンパイル
    pub fn compile_block_expr(&mut self, block_expr: &BlockExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Block expressions not yet implemented".to_string(),
            span: block_expr.span,
        }))
    }

    /// 値を文字列に変換
    pub fn value_to_string(&mut self, value: BasicValueEnum<'ctx>) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Ok(value)
    }
}