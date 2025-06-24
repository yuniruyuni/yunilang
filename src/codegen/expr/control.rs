//! if式、match式、ブロック式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
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
        
        // パニックメッセージを生成
        let panic_msg = "パターンマッチが網羅的ではありません";
        let panic_str = self.builder.build_global_string_ptr(panic_msg, "panic_msg")?.as_pointer_value();
        
        // yuni_panic関数を呼び出し
        let panic_fn = self.runtime_manager.get_function("yuni_panic")
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: "yuni_panic".to_string(),
                span: match_expr.span,
            }))?;
        
        self.builder.build_call(panic_fn, &[panic_str.into()], "panic_call")?;
        self.builder.build_unreachable()?;
        
        // unit値を返す（実際には到達しないが、型システムのために必要）
        let unit_value = self.context.i32_type().const_zero();
        
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
            Pattern::Wildcard => {
                // ワイルドカードパターンは常にマッチ
                Ok(self.context.bool_type().const_all_ones())
            }
            Pattern::Literal(lit) => {
                // リテラルパターンは値と比較
                match (lit, value) {
                    (LiteralPattern::Integer(expected), BasicValueEnum::IntValue(actual)) => {
                        let expected_val = self.context.i64_type().const_int(*expected as u64, expected < &0);
                        // 実際の値が異なるビット幅の場合、適切にキャストする
                        let actual_64 = match actual.get_type().get_bit_width().cmp(&64) {
                            std::cmp::Ordering::Less => {
                                if *expected < 0 {
                                    self.builder.build_int_s_extend(actual, self.context.i64_type(), "sext")?
                                } else {
                                    self.builder.build_int_z_extend(actual, self.context.i64_type(), "zext")?
                                }
                            }
                            std::cmp::Ordering::Greater => {
                                self.builder.build_int_truncate(actual, self.context.i64_type(), "trunc")?
                            }
                            std::cmp::Ordering::Equal => actual,
                        };
                        Ok(self.builder.build_int_compare(IntPredicate::EQ, actual_64, expected_val, "lit_match")?)
                    }
                    (LiteralPattern::Float(expected), BasicValueEnum::FloatValue(actual)) => {
                        let expected_val = self.context.f64_type().const_float(*expected);
                        Ok(self.builder.build_float_compare(inkwell::FloatPredicate::OEQ, actual, expected_val, "lit_match")?)
                    }
                    (LiteralPattern::Bool(expected), BasicValueEnum::IntValue(actual)) => {
                        let expected_val = self.context.bool_type().const_int(if *expected { 1 } else { 0 }, false);
                        Ok(self.builder.build_int_compare(IntPredicate::EQ, actual, expected_val, "lit_match")?)
                    }
                    (LiteralPattern::String(expected), BasicValueEnum::PointerValue(actual)) => {
                        // 文字列比較のランタイム関数を使用
                        let expected_str = self.builder.build_global_string_ptr(expected, "expected_str")?.as_pointer_value();
                        
                        // yuni_string_eq関数を取得
                        let string_eq_fn = self.runtime_manager.get_function("yuni_string_eq")
                            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                                name: "yuni_string_eq".to_string(),
                                span,
                            }))?;
                        
                        // 文字列を比較
                        let result = self.builder.build_call(
                            string_eq_fn,
                            &[expected_str.into(), actual.into()],
                            "string_eq_result"
                        )?;
                        
                        Ok(result.try_as_basic_value().left()
                            .ok_or_else(|| YuniError::Codegen(CodegenError::TypeError {
                                expected: "bool value".to_string(),
                                actual: "void".to_string(),
                                span,
                            }))?
                            .into_int_value())
                    }
                    _ => {
                        Err(YuniError::Codegen(CodegenError::TypeError {
                            expected: format!("{:?}", lit),
                            actual: format!("{:?}", value.get_type()),
                            span,
                        }))
                    }
                }
            }
            Pattern::EnumVariant { enum_name, variant, fields } => {
                // バリアントのインデックスを取得
                let key = (enum_name.clone(), variant.clone());
                let expected_index = self.enum_variants.get(&key)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("{}::{}", enum_name, variant),
                        span,
                    }))?;
                
                match fields {
                    crate::ast::EnumVariantPatternFields::Unit => {
                        // Unitバリアントの場合、値を直接比較
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
                    crate::ast::EnumVariantPatternFields::Tuple(patterns) => {
                        // タプル形式のデータを持つバリアント
                        if let BasicValueEnum::StructValue(struct_val) = value {
                            // discriminantを抽出して比較
                            let discriminant = self.builder.build_extract_value(struct_val, 0, "enum_discriminant")?
                                .into_int_value();
                            let expected = self.context.i32_type().const_int(*expected_index as u64, false);
                            let discriminant_match = self.builder.build_int_compare(IntPredicate::EQ, discriminant, expected, "discriminant_match")?;
                            
                            // discriminantがマッチしない場合は早期リターン
                            if patterns.is_empty() {
                                return Ok(discriminant_match);
                            }
                            
                            // データタプルを抽出
                            let data_tuple = self.builder.build_extract_value(struct_val, 1, "enum_data")?;
                            
                            // 各フィールドのパターンマッチング
                            let mut all_match = discriminant_match;
                            for (i, pattern) in patterns.iter().enumerate() {
                                let field_value = self.builder.build_extract_value(data_tuple.into_struct_value(), i as u32, &format!("field_{}", i))?;
                                let field_match = self.compile_pattern_match(pattern, field_value, span)?;
                                all_match = self.builder.build_and(all_match, field_match, &format!("field_match_{}", i))?;
                            }
                            
                            Ok(all_match)
                        } else {
                            Err(YuniError::Codegen(CodegenError::TypeError {
                                expected: "enum struct value".to_string(),
                                actual: format!("{:?}", value.get_type()),
                                span,
                            }))
                        }
                    }
                    crate::ast::EnumVariantPatternFields::Struct(field_patterns) => {
                        // 構造体形式のデータを持つバリアント
                        if let BasicValueEnum::StructValue(struct_val) = value {
                            // discriminantを抽出して比較
                            let discriminant = self.builder.build_extract_value(struct_val, 0, "enum_discriminant")?
                                .into_int_value();
                            let expected = self.context.i32_type().const_int(*expected_index as u64, false);
                            let discriminant_match = self.builder.build_int_compare(IntPredicate::EQ, discriminant, expected, "discriminant_match")?;
                            
                            // discriminantがマッチしない場合は早期リターン
                            if field_patterns.is_empty() {
                                return Ok(discriminant_match);
                            }
                            
                            // データ構造体を抽出
                            let data_struct = self.builder.build_extract_value(struct_val, 1, "enum_data")?;
                            
                            // 各フィールドのパターンマッチング
                            let mut all_match = discriminant_match;
                            for (i, (_, pattern)) in field_patterns.iter().enumerate() {
                                let field_value = self.builder.build_extract_value(data_struct.into_struct_value(), i as u32, &format!("field_{}", i))?;
                                let field_match = self.compile_pattern_match(pattern, field_value, span)?;
                                all_match = self.builder.build_and(all_match, field_match, &format!("field_match_{}", i))?;
                            }
                            
                            Ok(all_match)
                        } else {
                            Err(YuniError::Codegen(CodegenError::TypeError {
                                expected: "enum struct value".to_string(),
                                actual: format!("{:?}", value.get_type()),
                                span,
                            }))
                        }
                    }
                }
            }
            Pattern::Tuple(patterns) => {
                // タプル値であることを確認
                if let BasicValueEnum::StructValue(tuple_val) = value {
                    // すべての要素がマッチするかチェック
                    let mut all_match = self.context.bool_type().const_all_ones();
                    
                    for (i, pattern) in patterns.iter().enumerate() {
                        // タプルの要素を抽出
                        let element = self.builder.build_extract_value(tuple_val, i as u32, &format!("tuple_elem_{}", i))?;
                        
                        // 要素とパターンをマッチング
                        let element_match = self.compile_pattern_match(pattern, element, span)?;
                        
                        // AND演算で結果を結合
                        all_match = self.builder.build_and(all_match, element_match, &format!("tuple_match_{}", i))?;
                    }
                    
                    Ok(all_match)
                } else {
                    Err(YuniError::Codegen(CodegenError::TypeError {
                        expected: "tuple value".to_string(),
                        actual: format!("{:?}", value.get_type()),
                        span,
                    }))
                }
            }
            Pattern::Struct(struct_name, field_patterns) => {
                // 構造体値であることを確認
                if let BasicValueEnum::StructValue(struct_val) = value {
                    // 構造体情報を取得
                    let struct_info = self.struct_info.get(struct_name)
                        .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                            name: struct_name.clone(),
                            span,
                        }))?
                        .clone();
                    
                    // すべてのフィールドがマッチするかチェック
                    let mut all_match = self.context.bool_type().const_all_ones();
                    
                    for (field_name, pattern) in field_patterns {
                        // フィールドのインデックスを取得
                        let field_index = struct_info.get_field_index(field_name)
                            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                                name: format!("{}.{}", struct_name, field_name),
                                span,
                            }))?;
                        
                        // フィールドの値を抽出
                        let field_value = self.builder.build_extract_value(struct_val, field_index, &format!("{}_value", field_name))?;
                        
                        // フィールドとパターンをマッチング
                        let field_match = self.compile_pattern_match(pattern, field_value, span)?;
                        
                        // AND演算で結果を結合
                        all_match = self.builder.build_and(all_match, field_match, &format!("{}_match", field_name))?;
                    }
                    
                    Ok(all_match)
                } else {
                    Err(YuniError::Codegen(CodegenError::TypeError {
                        expected: "struct value".to_string(),
                        actual: format!("{:?}", value.get_type()),
                        span,
                    }))
                }
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
            Pattern::Literal(_) | Pattern::Wildcard => {
                // リテラルパターンとワイルドカードパターンは変数をバインドしない
                Ok(())
            }
            Pattern::EnumVariant { fields, .. } => {
                match fields {
                    crate::ast::EnumVariantPatternFields::Unit => {
                        // Unitバリアントは変数をバインドしない
                        Ok(())
                    }
                    crate::ast::EnumVariantPatternFields::Tuple(patterns) => {
                        // データを持つEnumバリアントの場合、データ部分を抽出
                        if let BasicValueEnum::StructValue(struct_val) = value {
                            let data_tuple = self.builder.build_extract_value(struct_val, 1, "enum_data")?;
                            
                            // 各フィールドパターンの変数をバインド
                            for (i, pattern) in patterns.iter().enumerate() {
                                let field_value = self.builder.build_extract_value(data_tuple.into_struct_value(), i as u32, &format!("field_{}", i))?;
                                self.bind_pattern_variables(pattern, field_value)?;
                            }
                        }
                        Ok(())
                    }
                    crate::ast::EnumVariantPatternFields::Struct(field_patterns) => {
                        // データを持つEnumバリアントの場合、データ部分を抽出
                        if let BasicValueEnum::StructValue(struct_val) = value {
                            let data_struct = self.builder.build_extract_value(struct_val, 1, "enum_data")?;
                            
                            // 各フィールドパターンの変数をバインド
                            for (i, (_, pattern)) in field_patterns.iter().enumerate() {
                                let field_value = self.builder.build_extract_value(data_struct.into_struct_value(), i as u32, &format!("field_{}", i))?;
                                self.bind_pattern_variables(pattern, field_value)?;
                            }
                        }
                        Ok(())
                    }
                }
            }
            Pattern::Tuple(patterns) => {
                // タプルの各要素の変数をバインド
                if let BasicValueEnum::StructValue(tuple_val) = value {
                    for (i, pattern) in patterns.iter().enumerate() {
                        let element = self.builder.build_extract_value(tuple_val, i as u32, &format!("tuple_elem_{}", i))?;
                        self.bind_pattern_variables(pattern, element)?;
                    }
                }
                Ok(())
            }
            Pattern::Struct(_, field_patterns) => {
                // 構造体の各フィールドの変数をバインド
                if let BasicValueEnum::StructValue(struct_val) = value {
                    for (i, (_, pattern)) in field_patterns.iter().enumerate() {
                        let field_value = self.builder.build_extract_value(struct_val, i as u32, &format!("field_{}", i))?;
                        self.bind_pattern_variables(pattern, field_value)?;
                    }
                }
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