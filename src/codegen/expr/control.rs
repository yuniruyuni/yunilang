//! if式、match式、ブロック式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::{BasicValueEnum, BasicValue};
use inkwell::IntPredicate;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// match式をコンパイル
    pub fn compile_match_expr(&mut self, match_expr: &MatchExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // マッチ対象の式をコンパイル
        let scrutinee = self.compile_expression(&match_expr.expr)?;
        
        // 現在の関数を取得
        let function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
        
        // アームごとのブロックを作成
        let mut arm_blocks = Vec::new();
        let mut result_blocks = Vec::new();
        for (i, _) in match_expr.arms.iter().enumerate() {
            let arm_block = self.context.append_basic_block(function, &format!("arm{}", i));
            let result_block = self.context.append_basic_block(function, &format!("arm{}_result", i));
            arm_blocks.push(arm_block);
            result_blocks.push(result_block);
        }
        
        // マージブロック
        let merge_block = self.context.append_basic_block(function, "match_merge");
        
        // エンドブロック（すべてのパターンが失敗した場合）
        let end_block = self.context.append_basic_block(function, "match_end");
        
        // エントリーポイントから最初のアームへジャンプ
        if !arm_blocks.is_empty() {
            self.builder.build_unconditional_branch(arm_blocks[0])?;
        } else {
            // アームがない場合はマージブロックへ直接ジャンプ
            self.builder.build_unconditional_branch(merge_block)?;
        }
        
        // 各アームの結果を格納
        let mut arm_results = Vec::new();
        let mut arm_result_blocks = Vec::new();
        
        // 各アームを処理
        for (i, arm) in match_expr.arms.iter().enumerate() {
            self.builder.position_at_end(arm_blocks[i]);
            
            // パターンマッチング
            let matches = self.compile_pattern_match(&arm.pattern, scrutinee, match_expr.span)?;
            
            // パターンで導入された変数をバインド（ガード評価のため）
            self.scope_manager.push_scope();
            self.bind_pattern_variables(&arm.pattern, scrutinee)?;
            
            // ガード条件がある場合
            let final_condition = if let Some(guard) = &arm.guard {
                let guard_value = self.compile_expression(guard)?;
                let guard_bool = match guard_value {
                    BasicValueEnum::IntValue(int_val) => {
                        if int_val.get_type().get_bit_width() == 1 {
                            int_val
                        } else {
                            let zero = int_val.get_type().const_zero();
                            self.builder.build_int_compare(IntPredicate::NE, int_val, zero, "guard_cond")?
                        }
                    }
                    _ => return Err(YuniError::Codegen(CodegenError::TypeError {
                        expected: "bool".to_string(),
                        actual: "non-bool".to_string(),
                        span: match_expr.span,
                    })),
                };
                
                // パターンマッチとガードの両方が真の場合のみマッチ
                self.builder.build_and(matches, guard_bool, "match_with_guard")?
            } else {
                matches
            };
            
            // ガード評価後、変数スコープを終了
            self.scope_manager.pop_scope();
            
            // 次のアームまたはエンドブロックを決定
            let next_block = if i + 1 < arm_blocks.len() {
                arm_blocks[i + 1]
            } else {
                end_block
            };
            
            // マッチした場合は結果ブロックへ、しなかった場合は次のアームへ
            self.builder.build_conditional_branch(final_condition, result_blocks[i], next_block)?;
            
            // 結果ブロックでアームの式を評価
            self.builder.position_at_end(result_blocks[i]);
            
            // パターンで導入された変数を再度スコープに追加（今度はアームの式評価のため）
            self.scope_manager.push_scope();
            self.bind_pattern_variables(&arm.pattern, scrutinee)?;
            
            // アームの式を評価
            let result = self.compile_expression(&arm.expr)?;
            arm_results.push(result);
            
            // スコープを終了
            self.scope_manager.pop_scope();
            
            // マージブロックへジャンプ
            self.builder.build_unconditional_branch(merge_block)?;
            arm_result_blocks.push(self.builder.get_insert_block().unwrap());
        }
        
        // エンドブロック（すべてのパターンがマッチしなかった場合）
        self.builder.position_at_end(end_block);
        // パニックまたはデフォルト値を返す（ここではunit値を返す）
        let unit_value = self.context.i32_type().const_zero();
        self.builder.build_unconditional_branch(merge_block)?;
        let end_block = self.builder.get_insert_block().unwrap();
        
        // マージブロックでPHIノードを作成
        self.builder.position_at_end(merge_block);
        
        if arm_results.is_empty() {
            // アームがない場合はunit値を返す
            Ok(unit_value.into())
        } else {
            // すべてのアームの結果が同じ型であることを確認
            let result_type = arm_results[0].get_type();
            let mut use_default = false;
            for result in &arm_results[1..] {
                if result.get_type() != result_type {
                    // 型が異なる場合はunit値を返す
                    use_default = true;
                    break;
                }
            }
            
            if use_default {
                Ok(unit_value.into())
            } else {
                // PHIノードを作成
                let phi = self.builder.build_phi(result_type, "match_result")?;
                
                // 各アームの結果を追加
                for (result, block) in arm_results.iter().zip(arm_result_blocks.iter()) {
                    phi.add_incoming(&[(result, *block)]);
                }
                
                // エンドブロックからの場合はunit値を追加
                phi.add_incoming(&[(&unit_value.as_basic_value_enum(), end_block)]);
                
                Ok(phi.as_basic_value())
            }
        }
    }
    
    /// パターンマッチングをコンパイル
    fn compile_pattern_match(
        &mut self,
        pattern: &Pattern,
        value: BasicValueEnum<'ctx>,
        span: Span,
    ) -> YuniResult<inkwell::values::IntValue<'ctx>> {
        match pattern {
            Pattern::Identifier(_, _) => {
                // 識別子パターンは常にマッチ
                Ok(self.context.bool_type().const_all_ones())
            }
            Pattern::EnumVariant { enum_name, variant, fields } => {
                // enum値の比較
                match fields {
                    crate::ast::EnumVariantPatternFields::Unit => {
                        // Unitバリアントの場合、値を比較
                        let key = (enum_name.clone(), variant.clone());
                        let expected_index = self.enum_variants.get(&key)
                            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                                name: format!("{}::{}", enum_name, variant),
                                span,
                            }))?;
                        
                        // 値がi32であることを確認
                        if let BasicValueEnum::IntValue(int_val) = value {
                            let expected = self.context.i32_type().const_int(*expected_index as u64, false);
                            Ok(self.builder.build_int_compare(IntPredicate::EQ, int_val, expected, "enum_match")?)
                        } else {
                            Err(YuniError::Codegen(CodegenError::TypeError {
                                expected: "enum value".to_string(),
                                actual: format!("{:?}", value.get_type()),
                                span,
                            }))
                        }
                    }
                    _ => {
                        // データを持つバリアントは未実装
                        Err(YuniError::Codegen(CodegenError::Unimplemented {
                            feature: "Enum variants with data in patterns not yet implemented".to_string(),
                            span,
                        }))
                    }
                }
            }
            Pattern::Tuple(_) => {
                // タプルパターンは未実装
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Tuple patterns not yet implemented".to_string(),
                    span,
                }))
            }
            Pattern::Struct(_, _) => {
                // 構造体パターンは未実装
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "Struct patterns not yet implemented".to_string(),
                    span,
                }))
            }
        }
    }
    
    /// パターンで導入された変数をバインド
    fn bind_pattern_variables(
        &mut self,
        pattern: &Pattern,
        value: BasicValueEnum<'ctx>,
    ) -> YuniResult<()> {
        match pattern {
            Pattern::Identifier(name, is_mut) => {
                // 値を変数にバインド
                let llvm_type = value.get_type();
                let ptr = self.builder.build_alloca(llvm_type, name)?;
                self.builder.build_store(ptr, value)?;
                
                // 型を推論
                let ty = match value {
                    BasicValueEnum::IntValue(int_val) => {
                        match int_val.get_type().get_bit_width() {
                            1 => Type::Bool,
                            8 => Type::I8,
                            16 => Type::I16,
                            32 => Type::I32,
                            64 => Type::I64,
                            128 => Type::I128,
                            _ => Type::I32,
                        }
                    }
                    BasicValueEnum::FloatValue(float_val) => {
                        if float_val.get_type() == self.context.f32_type() {
                            Type::F32
                        } else {
                            Type::F64
                        }
                    }
                    BasicValueEnum::PointerValue(_) => {
                        // ポインタ型の場合は適切に処理（簡易実装）
                        Type::String // 仮実装
                    }
                    _ => Type::I32, // デフォルト
                };
                
                // スコープに登録
                self.scope_manager.define_variable(name.clone(), ptr, ty, *is_mut);
                Ok(())
            }
            Pattern::EnumVariant { .. } => {
                // Enumバリアントパターンは変数をバインドしない（現在の実装では）
                Ok(())
            }
            _ => {
                // その他のパターンは未実装
                Ok(())
            }
        }
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