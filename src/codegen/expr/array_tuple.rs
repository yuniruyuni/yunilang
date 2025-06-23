//! 配列・タプルのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::types::BasicTypeEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
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
}