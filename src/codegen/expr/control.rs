//! if式、match式、ブロック式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::IntPredicate;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
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
}