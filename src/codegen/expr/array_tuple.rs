//! 配列・タプルのコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::AddressSpace;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 配列式をコンパイル
    pub fn compile_array_expr(&mut self, array: &ArrayExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        if array.elements.is_empty() {
            return Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "空の配列は型推論できません".to_string(),
                span: array.span,
            }));
        }

        // 各要素をコンパイル
        let mut compiled_elements = Vec::new();
        for element in &array.elements {
            compiled_elements.push(self.compile_expression(element)?);
        }

        // 最初の要素の型を基準とする
        let element_type = compiled_elements[0].get_type();
        
        // 配列をヒープに割り当て（動的配列として実装）
        let array_size = self.context.i64_type().const_int(array.elements.len() as u64, false);
        let element_size = element_type.size_of().unwrap();
        let total_size = self.builder.build_int_mul(array_size, element_size, "array_total_size")?;
        
        // メモリ割り当て（mallocを使用）
        let alloc_fn = self.runtime_manager.get_function("malloc")
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "malloc function not found".to_string(),
            }))?;
        
        let array_ptr = self.builder.build_call(
            alloc_fn,
            &[total_size.into()],
            "array_alloc"
        )?.try_as_basic_value().left()
            .ok_or_else(|| YuniError::Codegen(CodegenError::InvalidType {
                message: "メモリ割り当て関数が値を返しませんでした".to_string(),
                span: array.span,
            }))?;

        // 配列ポインタを適切な型にキャスト
        let array_ptr = array_ptr.into_pointer_value();
        let typed_array_ptr = self.builder.build_pointer_cast(
            array_ptr,
            self.context.ptr_type(AddressSpace::default()),
            "typed_array_ptr"
        )?;

        // 各要素を配列にコピー
        for (i, element_value) in compiled_elements.into_iter().enumerate() {
            let index = self.context.i64_type().const_int(i as u64, false);
            let element_ptr = unsafe {
                self.builder.build_gep(
                    element_type,
                    typed_array_ptr,
                    &[index],
                    &format!("array_element_{}", i)
                )?
            };
            self.builder.build_store(element_ptr, element_value)?;
        }

        Ok(typed_array_ptr.into())
    }

    /// タプル式をコンパイル
    pub fn compile_tuple_expr(&mut self, tuple: &TupleExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 各要素をコンパイル
        let mut compiled_elements = Vec::new();
        let mut element_types = Vec::new();
        
        for element in &tuple.elements {
            let value = self.compile_expression(element)?;
            element_types.push(value.get_type());
            compiled_elements.push(value);
        }
        
        // タプル構造体の型を作成
        let tuple_type = self.context.struct_type(&element_types, false);
        
        // スタック上にタプルを割り当て
        let tuple_alloca = self.builder.build_alloca(tuple_type, "tuple")?;
        
        // 各要素を構造体に格納
        for (i, value) in compiled_elements.into_iter().enumerate() {
            let field_ptr = self.builder.build_struct_gep(
                tuple_type,
                tuple_alloca,
                i as u32,
                &format!("tuple_field_{}", i)
            )?;
            self.builder.build_store(field_ptr, value)?;
        }
        
        // タプル全体を値として返す
        let tuple_value = self.builder.build_load(tuple_type, tuple_alloca, "tuple_value")?;
        Ok(tuple_value)
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
        // 値を評価
        let value = self.compile_expression(&assign.value)?;
        
        // ターゲットに応じて代入処理
        match &assign.target.as_ref() {
            Expression::Identifier(id) => {
                let symbol = self.scope_manager.lookup(&id.name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))?;
                    
                if !symbol.is_mutable {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: format!("変数 {} は不変です", id.name),
                        span: assign.span,
                    }));
                }
                
                self.builder.build_store(symbol.ptr, value)?;
            }
            Expression::Field(field_expr) => {
                // フィールドアクセスへの代入
                self.compile_field_assignment_expr(field_expr, value)?;
            }
            Expression::Index(index_expr) => {
                // インデックスアクセスへの代入
                self.compile_index_assignment_expr(index_expr, value)?;
            }
            Expression::Dereference(deref_expr) => {
                // 参照外しへの代入
                self.compile_deref_assignment_expr(deref_expr, value)?;
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "無効な代入先です".to_string(),
                    span: assign.span,
                }));
            }
        }
        
        // 代入式は代入された値を返す（Rustと同様）
        Ok(value)
    }
    
    /// フィールドへの代入式をコンパイル（ヘルパー）
    fn compile_field_assignment_expr(&mut self, field_expr: &FieldExpr, value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        // オブジェクトのアドレスを取得
        let object_ptr = match field_expr.object.as_ref() {
            Expression::Identifier(id) => {
                let symbol = self.scope_manager.lookup(&id.name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))?;
                symbol.ptr
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: "ネストしたフィールドアクセスへの代入はまだ実装されていません".to_string(),
                    span: field_expr.span,
                }));
            }
        };
        
        // 構造体情報を取得
        let struct_name = match field_expr.object.as_ref() {
            Expression::Identifier(id) => {
                let symbol = self.scope_manager.lookup(&id.name).unwrap();
                match &symbol.ty {
                    Type::UserDefined(name) => name.clone(),
                    _ => return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "構造体型ではありません".to_string(),
                        span: field_expr.span,
                    })),
                }
            }
            _ => unreachable!(),
        };
        
        let struct_info = self.struct_info.get(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: struct_name.clone(),
                span: field_expr.span,
            }))?;
        
        let field_index = struct_info.field_indices.get(&field_expr.field)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("{}.{}", struct_name, field_expr.field),
                span: field_expr.span,
            }))?;
        
        // フィールドポインタを取得
        let struct_ty = Type::UserDefined(struct_name.clone());
        let llvm_struct_type = self.type_manager.ast_type_to_llvm(&struct_ty)?;
        let struct_type = match llvm_struct_type {
            BasicTypeEnum::StructType(st) => st,
            _ => return Err(YuniError::Codegen(CodegenError::Internal {
                message: format!("構造体型 {} が見つかりません", struct_name),
            })),
        };
        
        let field_ptr = self.builder.build_struct_gep(
            struct_type,
            object_ptr,
            *field_index,
            &format!("{}_field_{}", struct_name, field_expr.field)
        )?;
        
        self.builder.build_store(field_ptr, value)?;
        Ok(())
    }
    
    /// インデックスへの代入式をコンパイル（ヘルパー）
    fn compile_index_assignment_expr(&mut self, _index_expr: &IndexExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "インデックスアクセスへの代入はまだ実装されていません".to_string(),
            span: Span::dummy(),
        }))
    }
    
    /// 参照外しへの代入式をコンパイル（ヘルパー）
    fn compile_deref_assignment_expr(&mut self, _deref_expr: &DereferenceExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "参照外しへの代入はまだ実装されていません".to_string(),
            span: Span::dummy(),
        }))
    }
}