//! 単項演算のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
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
}