//! リテラル式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 整数リテラルをコンパイル（期待される型のコンテキストなし）
    #[allow(dead_code)]
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

    /// ブール値リテラルをコンパイル
    pub fn compile_boolean_literal(&self, lit: &BooleanLit) -> YuniResult<BasicValueEnum<'ctx>> {
        Ok(self.context.bool_type().const_int(lit.value as u64, false).into())
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
}