//! match式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// match式をコンパイル
    pub fn compile_match_expr(&mut self, match_expr: &MatchExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // scrutinee（マッチ対象）をコンパイル
        let scrutinee_value = self.compile_expression(&match_expr.expr)?;
        
        // 現在の関数を取得
        let function = self.builder.get_insert_block()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "No insert block".to_string(),
            }))?
            .get_parent()
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "No parent function".to_string(),
            }))?;
        
        // match式の後のブロック
        let after_match_block = self.context.append_basic_block(function, "after_match");
        
        // match式の結果を格納するPHI命令の準備
        let result_type = self.expression_type(&match_expr.arms[0].expr)?;
        let llvm_result_type = self.type_manager.ast_type_to_llvm(&result_type)?;
        let result_alloca = self.builder.build_alloca(llvm_result_type, "match_result")?;
        
        // 各アームの処理
        let mut arm_blocks = Vec::new();
        for (i, arm) in match_expr.arms.iter().enumerate() {
            let arm_block = self.context.append_basic_block(function, &format!("match_arm_{}", i));
            arm_blocks.push(arm_block);
        }
        
        // デフォルトブロック（網羅性チェックがない場合のフォールバック）
        let default_block = self.context.append_basic_block(function, "match_default");
        
        // scrutineeの型によって処理を分岐
        let scrutinee_type = self.expression_type(&match_expr.expr)?;
        
        match &scrutinee_type {
            Type::UserDefined(enum_name) => {
                self.compile_enum_match(
                    scrutinee_value,
                    enum_name,
                    match_expr,
                    &arm_blocks,
                    default_block,
                    after_match_block,
                    result_alloca,
                )?;
            }
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 => {
                self.compile_integer_match(
                    scrutinee_value,
                    match_expr,
                    &arm_blocks,
                    default_block,
                    after_match_block,
                    result_alloca,
                )?;
            }
            Type::Bool => {
                self.compile_bool_match(
                    scrutinee_value,
                    match_expr,
                    &arm_blocks,
                    default_block,
                    after_match_block,
                    result_alloca,
                )?;
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: format!("Match on type {:?} not yet implemented", scrutinee_type),
                    span: match_expr.span,
                }));
            }
        }
        
        // match式の後のブロックに移動
        self.builder.position_at_end(after_match_block);
        
        // 結果を読み込んで返す
        let result = self.builder.build_load(llvm_result_type, result_alloca, "match_result_value")?;
        Ok(result)
    }

    /// Enum型のmatch式をコンパイル
    fn compile_enum_match(
        &mut self,
        scrutinee_value: BasicValueEnum<'ctx>,
        enum_name: &str,
        match_expr: &MatchExpr,
        arm_blocks: &[inkwell::basic_block::BasicBlock<'ctx>],
        default_block: inkwell::basic_block::BasicBlock<'ctx>,
        after_match_block: inkwell::basic_block::BasicBlock<'ctx>,
        result_alloca: inkwell::values::PointerValue<'ctx>,
    ) -> YuniResult<()> {
        // Option型の特別処理
        if enum_name == "Option" {
            return self.compile_option_match(
                scrutinee_value,
                match_expr,
                arm_blocks,
                default_block,
                after_match_block,
                result_alloca,
            );
        }
        
        // Result型の特別処理
        if enum_name == "Result" {
            return self.compile_result_match(
                scrutinee_value,
                match_expr,
                arm_blocks,
                default_block,
                after_match_block,
                result_alloca,
            );
        }
        
        // 通常のEnum型
        // タグ値（判別子）を取得
        let tag_value = scrutinee_value.into_int_value();
        
        // switch文を作成
        let cases: Vec<(inkwell::values::IntValue, inkwell::basic_block::BasicBlock)> = match_expr.arms.iter().enumerate()
            .filter_map(|(i, arm)| {
                match &arm.pattern {
                    Pattern::EnumVariant { enum_name: pattern_enum, variant, .. } => {
                        if pattern_enum != enum_name {
                            return None;
                        }
                        
                        // バリアントのタグ値を取得
                        let variant_tag = self.enum_variants.get(&(enum_name.to_string(), variant.clone()))?;
                        let case_value = self.context.i32_type().const_int(*variant_tag as u64, false);
                        Some((case_value, arm_blocks[i]))
                    }
                    _ => None,
                }
            })
            .collect();
        
        self.builder.build_switch(tag_value, default_block, &cases)?;
        
        // 各アームに対応するcaseを追加
        for (i, arm) in match_expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::EnumVariant { enum_name: pattern_enum, variant, .. } => {
                    if pattern_enum != enum_name {
                        return Err(YuniError::Codegen(CodegenError::TypeError {
                            expected: enum_name.to_string(),
                            actual: pattern_enum.clone(),
                            span: match_expr.span,
                        }));
                    }
                }
                _ => {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Expected enum variant pattern".to_string(),
                        span: match_expr.span,
                    }));
                }
            }
        }
        
        // 各アームのコードを生成
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.builder.position_at_end(arm_blocks[i]);
            
            // パターンマッチによる変数バインディング（TODO: 実装）
            
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
        
        // デフォルトブロック（パニック）
        self.builder.position_at_end(default_block);
        self.compile_panic("Unmatched pattern in match expression")?;
        
        Ok(())
    }
}

