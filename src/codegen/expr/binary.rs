//! バイナリ演算のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::{FloatPredicate, IntPredicate};

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
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
}