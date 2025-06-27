//! 式の型推論

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
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
            Expression::Path(path) => self.path_expression_type(path),
            Expression::Binary(binary) => self.binary_expression_type(binary),
            Expression::Unary(unary) => self.unary_expression_type(unary),
            Expression::Call(call) => self.call_expression_type(call),
            Expression::MethodCall(method_call) => self.method_call_type(method_call),
            Expression::Index(index) => self.index_expression_type(index),
            Expression::Field(field) => self.field_expression_type(field),
            Expression::Reference(ref_expr) => {
                let inner_type = self.expression_type(&ref_expr.expr)?;
                Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
            }
            Expression::Dereference(deref_expr) => {
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
            Expression::StructLit(struct_lit) => {
                struct_lit.name.as_ref()
                    .map(|name| Type::UserDefined(name.clone()))
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                        message: "Anonymous struct literal type inference not implemented".to_string(),
                    }))
            }
            Expression::EnumVariant(enum_variant) => {
                Ok(Type::UserDefined(enum_variant.enum_name.clone()))
            }
            Expression::Array(array_expr) => self.array_expression_type(array_expr),
            Expression::Tuple(tuple_expr) => self.tuple_expression_type(tuple_expr),
            Expression::Cast(cast) => Ok(cast.ty.clone()),
            Expression::Assignment(assign_expr) => self.expression_type(&assign_expr.value),
            Expression::Match(match_expr) => self.match_expression_type(match_expr),
            Expression::If(if_expr) => self.if_expression_type(if_expr),
            Expression::Block(block_expr) => self.block_expression_type(block_expr),
            Expression::TemplateString(_) => Ok(Type::String),
            Expression::ListLiteral(list) => self.list_literal_type(list),
            Expression::MapLiteral(map) => self.map_literal_type(map),
        }
    }

    fn path_expression_type(&mut self, path: &PathExpr) -> YuniResult<Type> {
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

    fn binary_expression_type(&mut self, binary: &BinaryExpr) -> YuniResult<Type> {
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

    fn unary_expression_type(&mut self, unary: &UnaryExpr) -> YuniResult<Type> {
        let operand_type = self.expression_type(&unary.expr)?;
        match &unary.op {
            UnaryOp::Not => Ok(Type::Bool),
            UnaryOp::Negate => Ok(operand_type),
            _ => Ok(operand_type),
        }
    }

    fn call_expression_type(&mut self, call: &CallExpr) -> YuniResult<Type> {
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

    fn method_call_type(&mut self, method_call: &MethodCallExpr) -> YuniResult<Type> {
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

    fn index_expression_type(&mut self, index: &IndexExpr) -> YuniResult<Type> {
        let object_type = self.expression_type(&index.object)?;
        match object_type {
            Type::Array(element_type) => Ok(*element_type),
            Type::String => Ok(Type::U8), // 文字列の要素はu8（バイト）
            Type::Generic(ref name, ref type_args) if name == "Vec" && type_args.len() == 1 => {
                Ok(type_args[0].clone())
            }
            _ => Err(YuniError::Codegen(CodegenError::InvalidType {
                message: format!("Cannot index into type: {:?}", object_type),
                span: index.span,
            })),
        }
    }

    fn field_expression_type(&mut self, field: &FieldExpr) -> YuniResult<Type> {
        let object_type = self.expression_type(&field.object)?;
        
        let struct_name = match &object_type {
            Type::UserDefined(name) => name.clone(),
            Type::Reference(inner, _) => {
                if let Type::UserDefined(name) = inner.as_ref() {
                    name.clone()
                } else {
                    return Err(YuniError::Codegen(CodegenError::InvalidType {
                        message: "Field access on non-struct type".to_string(),
                        span: field.span,
                    }));
                }
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Field access on non-struct type".to_string(),
                    span: field.span,
                }));
            }
        };
        
        let struct_info = self.struct_info.get(&struct_name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Struct info not found for {}", struct_name),
            }))?;
        
        let field_index = struct_info.get_field_index(&field.field)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: format!("{}.{}", struct_name, field.field),
                span: field.span,
            }))?;
        
        let field_type = struct_info.get_field_type(field_index as usize)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: format!("Field type not found for index {}", field_index),
            }))?;
        
        Ok(field_type.clone())
    }

    fn array_expression_type(&mut self, array: &ArrayExpr) -> YuniResult<Type> {
        if array.elements.is_empty() {
            Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "空の配列の型を推論できません".to_string(),
                span: array.span,
            }))
        } else {
            // 最初の要素の型を配列の要素型とする
            let element_type = self.expression_type(&array.elements[0])?;
            Ok(Type::Array(Box::new(element_type)))
        }
    }

    fn tuple_expression_type(&mut self, tuple: &TupleExpr) -> YuniResult<Type> {
        let element_types: Vec<Type> = tuple.elements
            .iter()
            .map(|elem| self.expression_type(elem))
            .collect::<YuniResult<Vec<_>>>()?;
        Ok(Type::Tuple(element_types))
    }

    fn match_expression_type(&mut self, match_expr: &MatchExpr) -> YuniResult<Type> {
        if match_expr.arms.is_empty() {
            return Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Empty match expression".to_string(),
                span: match_expr.span,
            }));
        }
        
        // 最初のアームの型を取得
        let first_arm_type = self.expression_type(&match_expr.arms[0].expr)?;
        
        // すべてのアームが同じ型を返すことを確認
        for arm in &match_expr.arms[1..] {
            let arm_type = self.expression_type(&arm.expr)?;
            if arm_type != first_arm_type {
                // 型が異なる場合はunit型として扱う
                return Ok(Type::I32); // unit型の代わりにi32(0)を使用
            }
        }
        
        Ok(first_arm_type)
    }

    fn if_expression_type(&mut self, if_expr: &IfExpr) -> YuniResult<Type> {
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

    fn block_expression_type(&mut self, block: &BlockExpr) -> YuniResult<Type> {
        if let Some(last_expr) = &block.last_expr {
            self.expression_type(last_expr)
        } else {
            // 最後の式がない場合はunit型
            Ok(Type::I32) // unit型の代わりにi32(0)を使用
        }
    }

    fn list_literal_type(&mut self, list: &ListLiteral) -> YuniResult<Type> {
        if let Some((name, type_args)) = &list.type_name {
            Ok(Type::Generic(name.clone(), type_args.clone()))
        } else if !list.elements.is_empty() {
            // 要素から型を推論
            let elem_type = self.expression_type(&list.elements[0])?;
            Ok(Type::Generic("Vec".to_string(), vec![elem_type]))
        } else {
            Err(YuniError::Codegen(CodegenError::Internal {
                message: "Cannot infer type for empty list literal".to_string(),
            }))
        }
    }

    fn map_literal_type(&mut self, map: &MapLiteral) -> YuniResult<Type> {
        if let Some((name, type_args)) = &map.type_name {
            Ok(Type::Generic(name.clone(), type_args.clone()))
        } else if !map.pairs.is_empty() {
            // 要素から型を推論
            let (key, value) = &map.pairs[0];
            let key_type = self.expression_type(key)?;
            let value_type = self.expression_type(value)?;
            Ok(Type::Generic("HashMap".to_string(), vec![key_type, value_type]))
        } else {
            Err(YuniError::Codegen(CodegenError::Internal {
                message: "Cannot infer type for empty map literal".to_string(),
            }))
        }
    }
}