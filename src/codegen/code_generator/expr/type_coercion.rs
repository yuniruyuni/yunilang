//! 型変換と型強制のロジック

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::types::BasicTypeEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 値を文字列に変換
    pub fn value_to_string(&mut self, value: BasicValueEnum<'ctx>) -> YuniResult<BasicValueEnum<'ctx>> {
        match value {
            BasicValueEnum::IntValue(int_val) => {
                // 整数を文字列に変換
                let to_string_fn = self.runtime_manager.get_function("yuni_int_to_string")
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: "Runtime function yuni_int_to_string not found".to_string(),
                    }))?;
                
                // yuni_int_to_string は i64 を期待するので、必要に応じて拡張
                let i64_type = self.context.i64_type();
                let int_val_as_i64 = match int_val.get_type().get_bit_width().cmp(&64) {
                    std::cmp::Ordering::Less => {
                        // 符号付き拡張（i8, i16, i32 -> i64）
                        self.builder.build_int_s_extend(int_val, i64_type, "sext_to_i64")?
                    }
                    std::cmp::Ordering::Greater => {
                        // 切り詰め（i128 -> i64）
                        self.builder.build_int_truncate(int_val, i64_type, "trunc_to_i64")?
                    }
                    std::cmp::Ordering::Equal => {
                        // 既に i64
                        int_val
                    }
                };
                
                let result = self.builder.build_call(
                    to_string_fn,
                    &[int_val_as_i64.into()],
                    "int_to_string_result",
                )?.try_as_basic_value().left()
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: "yuni_int_to_string should return a value".to_string(),
                    }))?;
                
                Ok(result)
            }
            BasicValueEnum::FloatValue(float_val) => {
                // 浮動小数点を文字列に変換
                let to_string_fn = self.runtime_manager.get_function("yuni_float_to_string")
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: "Runtime function yuni_float_to_string not found".to_string(),
                    }))?;
                
                let result = self.builder.build_call(
                    to_string_fn,
                    &[float_val.into()],
                    "float_to_string_result",
                )?.try_as_basic_value().left()
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: "yuni_float_to_string should return a value".to_string(),
                    }))?;
                
                Ok(result)
            }
            BasicValueEnum::PointerValue(_) => {
                // ポインタ値（文字列を含む）はそのまま返す
                Ok(value)
            }
            _ => {
                // bool型をチェック
                if value.is_int_value() {
                    let int_val = value.into_int_value();
                    if int_val.get_type().get_bit_width() == 1 {
                        // bool型の場合
                        let to_string_fn = self.runtime_manager.get_function("yuni_bool_to_string")
                            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                                message: "Runtime function yuni_bool_to_string not found".to_string(),
                            }))?;
                        
                        let result = self.builder.build_call(
                            to_string_fn,
                            &[int_val.into()],
                            "bool_to_string_result",
                        )?.try_as_basic_value().left()
                            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                                message: "yuni_bool_to_string should return a value".to_string(),
                            }))?;
                        
                        return Ok(result);
                    }
                }
                
                // その他の型は未実装
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: format!("String conversion for type {:?} not implemented", value.get_type()),
                    span: Span::dummy(),
                }))
            }
        }
    }

    /// 値を指定された型に変換
    pub fn coerce_to_type(
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
    pub fn coerce_int_types(
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
    
    /// 浮動小数点型の強制変換を行う
    pub fn coerce_float_types(
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
}