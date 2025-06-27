//! プリミティブ型のパターンマッチング

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 整数型のmatch式をコンパイル
    pub fn compile_integer_match(
        &mut self,
        scrutinee_value: BasicValueEnum<'ctx>,
        match_expr: &MatchExpr,
        arm_blocks: &[inkwell::basic_block::BasicBlock<'ctx>],
        default_block: inkwell::basic_block::BasicBlock<'ctx>,
        after_match_block: inkwell::basic_block::BasicBlock<'ctx>,
        result_alloca: inkwell::values::PointerValue<'ctx>,
    ) -> YuniResult<()> {
        let int_value = scrutinee_value.into_int_value();
        
        // switch文を作成
        let cases: Vec<(inkwell::values::IntValue, inkwell::basic_block::BasicBlock)> = match_expr.arms.iter().enumerate()
            .filter_map(|(i, arm)| {
                match &arm.pattern {
                    Pattern::Literal(LiteralPattern::Integer(value)) => {
                        let case_value = int_value.get_type().const_int(*value as u64, *value < 0);
                        Some((case_value, arm_blocks[i]))
                    }
                    _ => None,
                }
            })
            .collect();
        
        self.builder.build_switch(int_value, default_block, &cases)?;
        
        // 各アームのパターンを処理
        for (i, arm) in match_expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Wildcard => {
                    // ワイルドカードは最後のアームとして扱う
                    self.builder.position_at_end(default_block);
                    self.builder.build_unconditional_branch(arm_blocks[i])?;
                }
                _ => {}
            }
        }
        
        // 各アームのコードを生成
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.builder.position_at_end(arm_blocks[i]);
            
            // ガード条件がある場合はチェック
            if let Some(guard) = &arm.guard {
                let guard_value = self.compile_expression(guard)?;
                let guard_bool = guard_value.into_int_value();
                
                let guard_true_block = self.context.append_basic_block(
                    arm_blocks[i].get_parent().unwrap(),
                    &format!("guard_true_{}", i)
                );
                
                self.builder.build_conditional_branch(guard_bool, guard_true_block, default_block)?;
                self.builder.position_at_end(guard_true_block);
            }
            
            // アームの式をコンパイル
            let arm_value = self.compile_expression(&arm.expr)?;
            self.builder.build_store(result_alloca, arm_value)?;
            self.builder.build_unconditional_branch(after_match_block)?;
        }
        
        // デフォルトブロック
        self.builder.position_at_end(default_block);
        self.compile_panic("Unmatched pattern in match expression")?;
        
        Ok(())
    }

    /// Bool型のmatch式をコンパイル
    pub fn compile_bool_match(
        &mut self,
        scrutinee_value: BasicValueEnum<'ctx>,
        match_expr: &MatchExpr,
        arm_blocks: &[inkwell::basic_block::BasicBlock<'ctx>],
        default_block: inkwell::basic_block::BasicBlock<'ctx>,
        after_match_block: inkwell::basic_block::BasicBlock<'ctx>,
        result_alloca: inkwell::values::PointerValue<'ctx>,
    ) -> YuniResult<()> {
        let bool_value = scrutinee_value.into_int_value();
        
        // true用とfalse用のブロックを用意
        let mut true_block = None;
        let mut false_block = None;
        
        for (i, arm) in match_expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::Literal(LiteralPattern::Bool(value)) => {
                    if *value {
                        true_block = Some(arm_blocks[i]);
                    } else {
                        false_block = Some(arm_blocks[i]);
                    }
                }
                Pattern::Wildcard => {
                    // ワイルドカードは両方のケースに使用
                    if true_block.is_none() {
                        true_block = Some(arm_blocks[i]);
                    }
                    if false_block.is_none() {
                        false_block = Some(arm_blocks[i]);
                    }
                }
                _ => {}
            }
        }
        
        // 分岐
        if let (Some(true_b), Some(false_b)) = (true_block, false_block) {
            self.builder.build_conditional_branch(bool_value, true_b, false_b)?;
        } else {
            return Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Incomplete bool match".to_string(),
                span: match_expr.span,
            }));
        }
        
        // 各アームのコードを生成
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.builder.position_at_end(arm_blocks[i]);
            
            // ガード条件がある場合はチェック
            if let Some(guard) = &arm.guard {
                let guard_value = self.compile_expression(guard)?;
                let guard_bool = guard_value.into_int_value();
                
                let guard_true_block = self.context.append_basic_block(
                    arm_blocks[i].get_parent().unwrap(),
                    &format!("guard_true_{}", i)
                );
                
                self.builder.build_conditional_branch(guard_bool, guard_true_block, default_block)?;
                self.builder.position_at_end(guard_true_block);
            }
            
            // アームの式をコンパイル
            let arm_value = self.compile_expression(&arm.expr)?;
            self.builder.build_store(result_alloca, arm_value)?;
            self.builder.build_unconditional_branch(after_match_block)?;
        }
        
        // デフォルトブロック
        self.builder.position_at_end(default_block);
        self.compile_panic("Unmatched pattern in match expression")?;
        
        Ok(())
    }

    /// パニックを生成
    pub fn compile_panic(&mut self, message: &str) -> YuniResult<()> {
        let panic_msg = self.builder.build_global_string_ptr(message, "panic_msg")?.as_pointer_value();
        
        // yuni_panic関数を呼び出し
        let panic_fn = self.runtime_manager.get_function("yuni_panic")
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: "yuni_panic".to_string(),
                span: Span::dummy(),
            }))?;
        
        self.builder.build_call(panic_fn, &[panic_msg.into()], "panic_call")?;
        self.builder.build_unreachable()?;
        
        Ok(())
    }
}