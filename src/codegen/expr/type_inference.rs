//! 型推論と型変換のロジック

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;
use inkwell::types::BasicTypeEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 値を文字列に変換
    pub fn value_to_string(&mut self, value: BasicValueEnum<'ctx>) -> YuniResult<BasicValueEnum<'ctx>> {
        // TODO: 実装
        Ok(value)
    }

    /// 値を指定された型に変換
    pub fn coerce_to_type(
        &self, 
        value: BasicValueEnum<'ctx>, 
        target_type: BasicTypeEnum<'ctx>,
        span: Span
    ) -> YuniResult<BasicValueEnum<'ctx>> {
        match (value, target_type) {
            // 整数から整数への変換
            (BasicValueEnum::IntValue(int_val), BasicTypeEnum::IntType(target_int_type)) => {
                let source_type = int_val.get_type();
                if source_type == target_int_type {
                    Ok(int_val.into())
                } else {
                    let source_bits = source_type.get_bit_width();
                    let target_bits = target_int_type.get_bit_width();
                    
                    if source_bits < target_bits {
                        // 拡張
                        if self.is_signed_type(source_bits) {
                            Ok(self.builder.build_int_s_extend(int_val, target_int_type, "sext")?.into())
                        } else {
                            Ok(self.builder.build_int_z_extend(int_val, target_int_type, "zext")?.into())
                        }
                    } else {
                        // 切り詰め
                        Ok(self.builder.build_int_truncate(int_val, target_int_type, "trunc")?.into())
                    }
                }
            }
            // 浮動小数点から浮動小数点への変換
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
                        message: format!("Unsupported float coercion from {:?} to {:?}", source_type, target_float_type),
                        span,
                    }))
                }
            }
            // 同じ型の場合はそのまま返す
            _ => {
                if value.get_type() == target_type {
                    Ok(value)
                } else {
                    Err(YuniError::Codegen(CodegenError::TypeError {
                        expected: format!("{:?}", target_type),
                        actual: format!("{:?}", value.get_type()),
                        span,
                    }))
                }
            }
        }
    }

    /// 整数型の強制変換を行う
    /// 異なるビット幅の整数型を同じ型に変換する
    pub fn coerce_int_types(
        &self, 
        left: inkwell::values::IntValue<'ctx>, 
        right: inkwell::values::IntValue<'ctx>,
        _span: Span
    ) -> YuniResult<(inkwell::values::IntValue<'ctx>, inkwell::values::IntValue<'ctx>)> {
        let left_bits = left.get_type().get_bit_width();
        let right_bits = right.get_type().get_bit_width();
        
        if left_bits == right_bits {
            return Ok((left, right));
        }
        
        // より大きい型に合わせる
        if left_bits > right_bits {
            // rightをleftの型に拡張
            let extended = if self.is_signed_type(right_bits) {
                self.builder.build_int_s_extend(right, left.get_type(), "sext")?
            } else {
                self.builder.build_int_z_extend(right, left.get_type(), "zext")?
            };
            Ok((left, extended))
        } else {
            // leftをrightの型に拡張
            let extended = if self.is_signed_type(left_bits) {
                self.builder.build_int_s_extend(left, right.get_type(), "sext")?
            } else {
                self.builder.build_int_z_extend(left, right.get_type(), "zext")?
            };
            Ok((extended, right))
        }
    }
    
    /// 整数型が符号付きかどうかを判定
    /// ビット幅から型を推測して判定する（非推奨）
    pub fn is_signed_type(&self, bit_width: u32) -> bool {
        // ビット幅だけでは正確な判定ができないため、デフォルトで符号付きとする
        // より正確な判定にはType enumを使用すること
        match bit_width {
            8 | 16 | 32 | 64 | 128 => true, // デフォルトで符号付き
            _ => true,
        }
    }
    
    /// 型が符号付き整数かどうかを判定（推奨）
    pub fn is_signed_integer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256
        )
    }
    
    /// 型が符号なし整数かどうかを判定
    #[allow(dead_code)]
    pub fn is_unsigned_integer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256
        )
    }
    
    /// 浮動小数点型の強制変換を行う
    pub fn coerce_float_types(
        &self,
        left: inkwell::values::FloatValue<'ctx>,
        right: inkwell::values::FloatValue<'ctx>,
    ) -> YuniResult<(inkwell::values::FloatValue<'ctx>, inkwell::values::FloatValue<'ctx>)> {
        let left_type = left.get_type();
        let right_type = right.get_type();
        
        if left_type == right_type {
            return Ok((left, right));
        }
        
        // f64型を優先する（より精度が高い）
        if left_type == self.context.f64_type() {
            let extended = self.builder.build_float_ext(right, left_type, "fpext")?;
            Ok((left, extended))
        } else if right_type == self.context.f64_type() {
            let extended = self.builder.build_float_ext(left, right_type, "fpext")?;
            Ok((extended, right))
        } else {
            // どちらもf64でない場合はそのまま返す（エラーになるかもしれない）
            Ok((left, right))
        }
    }

    /// 式の型を推論する
    pub fn expression_type(&mut self, expr: &Expression) -> YuniResult<Type> {
        match expr {
            Expression::Integer(lit) => {
                if let Some(suffix) = &lit.suffix {
                    match suffix.as_str() {
                        "i8" => Ok(Type::I8),
                        "i16" => Ok(Type::I16),
                        "i32" => Ok(Type::I32),
                        "i64" => Ok(Type::I64),
                        "i128" => Ok(Type::I128),
                        "u8" => Ok(Type::U8),
                        "u16" => Ok(Type::U16),
                        "u32" => Ok(Type::U32),
                        "u64" => Ok(Type::U64),
                        "u128" => Ok(Type::U128),
                        _ => Ok(Type::I32), // デフォルト
                    }
                } else {
                    Ok(Type::I32) // デフォルトはi32（Rustと同じ）
                }
            }
            Expression::Float(lit) => {
                if let Some(suffix) = &lit.suffix {
                    match suffix.as_str() {
                        "f32" => Ok(Type::F32),
                        "f64" => Ok(Type::F64),
                        _ => Ok(Type::F64), // デフォルト
                    }
                } else {
                    Ok(Type::F64) // デフォルト
                }
            }
            Expression::String(_) => Ok(Type::String),
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::Identifier(id) => {
                if let Some(symbol) = self.scope_manager.lookup(&id.name) {
                    Ok(symbol.ty.clone())
                } else {
                    Err(YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))
                }
            }
            Expression::Path(path) => {
                if path.segments.len() == 1 {
                    let name = &path.segments[0];
                    
                    // 関数を探す
                    if self.functions.contains_key(name) {
                        // 関数ポインタ型として扱う（簡易実装）
                        return Ok(Type::UserDefined(format!("fn_{}", name)));
                    }
                    
                    // 変数として扱う
                    if let Some(symbol) = self.scope_manager.lookup(name) {
                        Ok(symbol.ty.clone())
                    } else {
                        Err(YuniError::Codegen(CodegenError::Undefined {
                            name: name.clone(),
                            span: path.span,
                        }))
                    }
                } else {
                    Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Multi-segment path type inference not implemented".to_string(),
                        span: path.span,
                    }))
                }
            }
            Expression::Binary(binary) => {
                let left_type = self.expression_type(&binary.left)?;
                let right_type = self.expression_type(&binary.right)?;
                
                match &binary.op {
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                        if left_type == right_type {
                            Ok(left_type)
                        } else {
                            // 型の自動昇格をサポート（簡易実装）
                            Ok(left_type)
                        }
                    }
                    BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge | BinaryOp::Eq | BinaryOp::Ne |
                    BinaryOp::And | BinaryOp::Or => Ok(Type::Bool),
                    _ => Ok(left_type),
                }
            }
            Expression::Unary(unary) => {
                let operand_type = self.expression_type(&unary.expr)?;
                match &unary.op {
                    UnaryOp::Not => Ok(Type::Bool),
                    UnaryOp::Negate => Ok(operand_type),
                    _ => Ok(operand_type),
                }
            }
            Expression::Call(call) => {
                let func_name = match call.callee.as_ref() {
                    Expression::Identifier(id) => &id.name,
                    Expression::Path(path) if path.segments.len() == 1 => &path.segments[0],
                    _ => return Err(YuniError::Codegen(CodegenError::Unimplemented {
                        feature: "Complex function call type inference not implemented".to_string(),
                        span: call.span,
                    })),
                };
                
                // println の特別な処理
                if func_name == "println" {
                    return Ok(Type::I32); // printlnは実際にはi32(0)を返すので
                }
                
                // 関数の戻り値型を取得
                if let Some(return_type) = self.function_types.get(func_name) {
                    // Void型の関数は実際にはunit値（i32(0)）を返すため、
                    // 型推論ではI32として扱う
                    if matches!(return_type, Type::Void) {
                        Ok(Type::I32)
                    } else {
                        Ok(return_type.clone())
                    }
                } else {
                    // 関数が見つからない場合はエラー
                    Err(YuniError::Codegen(CodegenError::Undefined {
                        name: func_name.clone(),
                        span: call.span,
                    }))
                }
            }
            Expression::If(if_expr) => {
                // if式の場合、then/elseブランチの型から推論
                let then_type = self.expression_type(&if_expr.then_branch)?;
                if let Some(else_branch) = &if_expr.else_branch {
                    let else_type = self.expression_type(else_branch)?;
                    // 両方の型が同じならその型を返す
                    if then_type == else_type {
                        Ok(then_type)
                    } else {
                        // 型が異なる場合はunit型
                        Ok(Type::I32) // unit型の代わりにi32(0)を使用
                    }
                } else {
                    // elseブランチがない場合はunit型
                    Ok(Type::I32) // unit型の代わりにi32(0)を使用
                }
            }
            Expression::Block(block_expr) => {
                // ブロック式の場合、最後の式の型を返す
                if let Some(last_expr) = &block_expr.last_expr {
                    self.expression_type(last_expr)
                } else {
                    // 最後の式がない場合はunit型
                    Ok(Type::I32) // unit型の代わりにi32(0)を使用
                }
            }
            Expression::StructLit(struct_lit) => {
                // 構造体リテラルの型は構造体名から決まる
                Ok(Type::UserDefined(struct_lit.name.clone()))
            }
            Expression::Field(field_expr) => {
                // フィールドアクセスの型推論
                let object_type = self.expression_type(&field_expr.object)?;
                
                let struct_name = match &object_type {
                    Type::UserDefined(name) => name.clone(),
                    Type::Reference(inner, _) => {
                        if let Type::UserDefined(name) = inner.as_ref() {
                            name.clone()
                        } else {
                            return Err(YuniError::Codegen(CodegenError::InvalidType {
                                message: "Field access on non-struct type".to_string(),
                                span: field_expr.span,
                            }));
                        }
                    }
                    _ => {
                        return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "Field access on non-struct type".to_string(),
                            span: field_expr.span,
                        }));
                    }
                };
                
                let struct_info = self.struct_info.get(&struct_name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: format!("Struct info not found for {}", struct_name),
                    }))?;
                
                let field_index = struct_info.get_field_index(&field_expr.field)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("{}.{}", struct_name, field_expr.field),
                        span: field_expr.span,
                    }))?;
                
                let field_type = struct_info.get_field_type(field_index as usize)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: format!("Field type not found for index {}", field_index),
                    }))?;
                
                Ok(field_type.clone())
            }
            Expression::EnumVariant(enum_variant) => {
                // Enumバリアントの型はEnum自体の型
                Ok(Type::UserDefined(enum_variant.enum_name.clone()))
            }
            Expression::Array(array_expr) => {
                if array_expr.elements.is_empty() {
                    Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "空の配列の型を推論できません".to_string(),
                        span: array_expr.span,
                    }))
                } else {
                    // 最初の要素の型を配列の要素型とする
                    let element_type = self.expression_type(&array_expr.elements[0])?;
                    Ok(Type::Array(Box::new(element_type)))
                }
            }
            Expression::Tuple(tuple_expr) => {
                let element_types: Vec<Type> = tuple_expr.elements
                    .iter()
                    .map(|elem| self.expression_type(elem))
                    .collect::<YuniResult<Vec<_>>>()?;
                Ok(Type::Tuple(element_types))
            }
            Expression::Reference(ref_expr) => {
                // 参照式の型は、内部式の型をReference型でラップしたもの
                let inner_type = self.expression_type(&ref_expr.expr)?;
                Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
            }
            Expression::Dereference(deref_expr) => {
                // 参照外し式の型は、参照型の内部型
                let expr_type = self.expression_type(&deref_expr.expr)?;
                match expr_type {
                    Type::Reference(inner, _is_mut) => Ok(*inner),
                    _ => Err(YuniError::Codegen(CodegenError::TypeError {
                        expected: "reference type".to_string(),
                        actual: format!("{:?}", expr_type),
                        span: deref_expr.span,
                    })),
                }
            }
            Expression::Index(index_expr) => {
                // インデックスアクセスの型は配列の要素型
                let object_type = self.expression_type(&index_expr.object)?;
                match object_type {
                    Type::Array(element_type) => Ok(*element_type),
                    Type::String => Ok(Type::U8), // 文字列の要素はu8（バイト）
                    _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: format!("Cannot index into type: {:?}", object_type),
                        span: index_expr.span,
                    })),
                }
            }
            Expression::MethodCall(method_call) => {
                // メソッド呼び出しの型は、メソッドの戻り値型
                let object_type = self.expression_type(&method_call.object)?;
                
                // 構造体名を取得
                let struct_name = match &object_type {
                    Type::UserDefined(name) => name.clone(),
                    Type::Reference(inner, _is_mut) => {
                        if let Type::UserDefined(name) = inner.as_ref() {
                            name.clone()
                        } else {
                            return Err(YuniError::Codegen(CodegenError::InvalidType {
                                message: "Method call on non-struct type".to_string(),
                                span: method_call.span,
                            }));
                        }
                    }
                    _ => {
                        return Err(YuniError::Codegen(CodegenError::InvalidType {
                            message: "Method call on non-struct type".to_string(),
                            span: method_call.span,
                        }));
                    }
                };
                
                // メソッドの関数名を取得
                let methods = self.struct_methods.get(&struct_name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("No methods found for type {}", struct_name),
                        span: method_call.span,
                    }))?;
                
                let (_, mangled_name) = methods.iter()
                    .find(|(method_name, _)| method_name == &method_call.method)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: format!("Method '{}' not found for type '{}'", method_call.method, struct_name),
                        span: method_call.span,
                    }))?;
                
                // メソッドの戻り値型を取得
                if let Some(return_type) = self.function_types.get(mangled_name) {
                    Ok(return_type.clone())
                } else {
                    Err(YuniError::Codegen(CodegenError::Internal {
                        message: format!("Method return type not found for '{}'", mangled_name),
                    }))
                }
            }
            _ => Err(YuniError::Codegen(CodegenError::Unimplemented {
                feature: "Type inference not implemented for this expression".to_string(),
                span: expr.span(),
            })),
        }
    }
}