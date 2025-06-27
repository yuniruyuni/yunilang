//! パターンマッチングのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::IntPredicate;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// Option型のmatch式をコンパイル
    pub fn compile_option_match(
        &mut self,
        scrutinee_value: BasicValueEnum<'ctx>,
        match_expr: &MatchExpr,
        arm_blocks: &[inkwell::basic_block::BasicBlock<'ctx>],
        default_block: inkwell::basic_block::BasicBlock<'ctx>,
        after_match_block: inkwell::basic_block::BasicBlock<'ctx>,
        result_alloca: inkwell::values::PointerValue<'ctx>,
    ) -> YuniResult<()> {
        // Optionはポインタとして表現される（nullがNone）
        let option_ptr = scrutinee_value.into_pointer_value();
        let is_null = self.builder.build_is_null(option_ptr, "is_none")?;
        
        // None用とSome用のブロックを用意
        let mut none_block = None;
        let mut some_block = None;
        
        for (i, arm) in match_expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::EnumVariant { variant, .. } => {
                    match variant.as_str() {
                        "None" => none_block = Some(arm_blocks[i]),
                        "Some" => some_block = Some(arm_blocks[i]),
                        _ => return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: format!("Unknown Option variant: {}", variant),
                            span: match_expr.span,
                        })),
                    }
                }
                _ => {}
            }
        }
        
        // 分岐
        if let (Some(none_b), Some(some_b)) = (none_block, some_block) {
            self.builder.build_conditional_branch(is_null, none_b, some_b)?;
        } else {
            return Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Incomplete Option match".to_string(),
                span: match_expr.span,
            }));
        }
        
        // 各アームのコードを生成
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.builder.position_at_end(arm_blocks[i]);
            
            match &arm.pattern {
                Pattern::EnumVariant { variant, fields, .. } => {
                    if variant == "Some" {
                        // Someの場合、値を抽出
                        if let EnumVariantPatternFields::Tuple(patterns) = fields {
                            if patterns.len() == 1 {
                                if let Pattern::Identifier(var_name, _) = &patterns[0] {
                                    // 値をロード
                                    let value_type = match self.expression_type(&match_expr.expr)? {
                                        Type::Generic(_, args) if args.len() == 1 => args[0].clone(),
                                        _ => return Err(YuniError::Codegen(CodegenError::Internal {
                                            message: "Invalid Option type".to_string(),
                                        })),
                                    };
                                    let llvm_value_type = self.type_manager.ast_type_to_llvm(&value_type)?;
                                    let value = self.builder.build_load(llvm_value_type, option_ptr, "some_value")?;
                                    
                                    // 変数として登録
                                    let ptr = self.builder.build_alloca(llvm_value_type, var_name)?;
                                    self.builder.build_store(ptr, value)?;
                                    self.scope_manager.define_variable(var_name.clone(), ptr, value_type, false);
                                }
                            }
                        }
                    }
                    // Noneの場合は何もしない
                }
                _ => {}
            }
            
            // アームの式をコンパイル
            let arm_value = self.compile_expression(&arm.expr)?;
            self.builder.build_store(result_alloca, arm_value)?;
            self.builder.build_unconditional_branch(after_match_block)?;
            
            // Someパターンの変数をスコープから削除
            if let Pattern::EnumVariant { variant, fields, .. } = &arm.pattern {
                if variant == "Some" {
                    if let EnumVariantPatternFields::Tuple(patterns) = fields {
                        if patterns.len() == 1 {
                            if let Pattern::Identifier(var_name, _) = &patterns[0] {
                                // スコープ管理はブロックを抜ける際に自動的に処理される
                            }
                        }
                    }
                }
            }
        }
        
        // デフォルトブロック
        self.builder.position_at_end(default_block);
        self.compile_panic("Unmatched pattern in match expression")?;
        
        Ok(())
    }

    /// Result型のmatch式をコンパイル
    pub fn compile_result_match(
        &mut self,
        scrutinee_value: BasicValueEnum<'ctx>,
        match_expr: &MatchExpr,
        arm_blocks: &[inkwell::basic_block::BasicBlock<'ctx>],
        default_block: inkwell::basic_block::BasicBlock<'ctx>,
        after_match_block: inkwell::basic_block::BasicBlock<'ctx>,
        result_alloca: inkwell::values::PointerValue<'ctx>,
    ) -> YuniResult<()> {
        // Resultは構造体として表現される（タグ + データ）
        let result_struct = scrutinee_value.into_struct_value();
        let tag_value = self.builder.build_extract_value(result_struct, 0, "result_tag")?.into_int_value();
        
        // Ok用とErr用のブロックを用意
        let mut ok_block = None;
        let mut err_block = None;
        
        for (i, arm) in match_expr.arms.iter().enumerate() {
            match &arm.pattern {
                Pattern::EnumVariant { variant, .. } => {
                    match variant.as_str() {
                        "Ok" => ok_block = Some(arm_blocks[i]),
                        "Err" => err_block = Some(arm_blocks[i]),
                        _ => return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: format!("Unknown Result variant: {}", variant),
                            span: match_expr.span,
                        })),
                    }
                }
                _ => {}
            }
        }
        
        // switch文を作成
        let mut cases = Vec::new();
        
        // Ok (tag = 0) とErr (tag = 1) のケースを追加
        if let Some(ok_b) = ok_block {
            cases.push((self.context.i32_type().const_int(0, false), ok_b));
        }
        if let Some(err_b) = err_block {
            cases.push((self.context.i32_type().const_int(1, false), err_b));
        }
        
        self.builder.build_switch(tag_value, default_block, &cases)?;
        
        // 各アームのコードを生成
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.builder.position_at_end(arm_blocks[i]);
            
            match &arm.pattern {
                Pattern::EnumVariant { variant, fields, .. } => {
                    if let EnumVariantPatternFields::Tuple(patterns) = fields {
                        if patterns.len() == 1 {
                            if let Pattern::Identifier(var_name, _) = &patterns[0] {
                                // データを抽出
                                let data_value = self.builder.build_extract_value(result_struct, 1, "result_data")?;
                                
                                let value_type = match self.expression_type(&match_expr.expr)? {
                                    Type::Generic(_, args) if args.len() == 2 => {
                                        if variant == "Ok" {
                                            args[0].clone()
                                        } else {
                                            args[1].clone()
                                        }
                                    }
                                    _ => return Err(YuniError::Codegen(CodegenError::Internal {
                                        message: "Invalid Result type".to_string(),
                                    })),
                                };
                                let llvm_value_type = self.type_manager.ast_type_to_llvm(&value_type)?;
                                
                                // データ値はすでにdata_valueとして抽出されている
                                // 変数として登録
                                let ptr = self.builder.build_alloca(llvm_value_type, var_name)?;
                                self.builder.build_store(ptr, data_value)?;
                                self.scope_manager.define_variable(var_name.clone(), ptr, value_type, false);
                            }
                        }
                    }
                }
                _ => {}
            }
            
            // アームの式をコンパイル
            let arm_value = self.compile_expression(&arm.expr)?;
            self.builder.build_store(result_alloca, arm_value)?;
            self.builder.build_unconditional_branch(after_match_block)?;
            
            // パターンの変数のスコープはブロックを抜ける際に自動的に管理される
        }
        
        // デフォルトブロック
        self.builder.position_at_end(default_block);
        self.compile_panic("Unmatched pattern in match expression")?;
        
        Ok(())
    }
}