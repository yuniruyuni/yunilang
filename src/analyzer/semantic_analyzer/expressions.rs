//! 式の解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisError, AnalysisResult, TypeKind};
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 式の解析と型推論（期待される型のコンテキストなし）
    pub fn analyze_expression(&mut self, expr: &Expression) -> AnalysisResult<Type> {
        self.analyze_expression_with_type(expr, None)
    }

    /// 式の解析と型推論（期待される型のコンテキスト付き）
    pub fn analyze_expression_with_type(&mut self, expr: &Expression, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        match expr {
            Expression::Integer(int_lit) => self.analyze_integer_literal(int_lit, expected_type),
            Expression::Float(float_lit) => self.analyze_float_literal(float_lit),
            Expression::String(_) => Ok(Type::String),
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::Identifier(ident) => self.analyze_identifier(ident),
            Expression::Binary(binary) => self.analyze_binary_expression(binary),
            Expression::Unary(unary) => self.analyze_unary_expression(unary),
            Expression::Call(call) => self.analyze_call_expression(call),
            Expression::Field(field) => self.analyze_field_expression(field),
            Expression::StructLit(struct_lit) => self.analyze_struct_literal(struct_lit),
            Expression::Array(array) => self.analyze_array_expression(array),
            Expression::Cast(cast) => self.analyze_cast_expression(cast),
            Expression::Match(match_expr) => self.analyze_match_expression(match_expr),
            Expression::EnumVariant(enum_variant) => self.analyze_enum_variant_expression(enum_variant),
            Expression::MethodCall(method_call) => self.analyze_method_call_expression(method_call),
            Expression::If(if_expr) => self.analyze_if_expression(if_expr),
            Expression::Block(block_expr) => self.analyze_block_expression(block_expr),
            Expression::TemplateString(template) => self.analyze_template_string(template),
            Expression::Path(path_expr) => self.analyze_path_expression(path_expr),
            Expression::Index(index_expr) => self.analyze_index_expression(index_expr),
            Expression::Reference(ref_expr) => self.analyze_reference_expression(ref_expr),
            Expression::Dereference(deref_expr) => self.analyze_dereference_expression(deref_expr),
            Expression::Assignment(assign_expr) => self.analyze_assignment_expression(assign_expr),
            Expression::Tuple(tuple_expr) => self.analyze_tuple_expression(tuple_expr),
        }
    }

    fn analyze_integer_literal(&self, int_lit: &IntegerLit, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        if let Some(suffix) = &int_lit.suffix {
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
            // 期待される型が指定されている場合はそれを使用
            if let Some(expected) = expected_type {
                match expected {
                    Type::I8 => Ok(Type::I8),
                    Type::I16 => Ok(Type::I16),
                    Type::I32 => Ok(Type::I32),
                    Type::I64 => Ok(Type::I64),
                    Type::I128 => Ok(Type::I128),
                    Type::U8 => Ok(Type::U8),
                    Type::U16 => Ok(Type::U16),
                    Type::U32 => Ok(Type::U32),
                    Type::U64 => Ok(Type::U64),
                    Type::U128 => Ok(Type::U128),
                    _ => Ok(Type::I32), // 整数型でない場合はデフォルト
                }
            } else {
                Ok(Type::I32) // サフィックスがない場合のデフォルトはi32
            }
        }
    }

    fn analyze_float_literal(&self, float_lit: &FloatLit) -> AnalysisResult<Type> {
        if let Some(suffix) = &float_lit.suffix {
            match suffix.as_str() {
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                _ => Ok(Type::F64), // fallback
            }
        } else {
            Ok(Type::F64) // default when no suffix
        }
    }

    fn analyze_identifier(&self, ident: &Identifier) -> AnalysisResult<Type> {
        if let Some(symbol) = self.lookup_variable(&ident.name) {
            Ok(symbol.ty.clone())
        } else {
            Err(AnalysisError::UndefinedVariable {
                name: ident.name.clone(),
                span: ident.span,
            })
        }
    }

    /// 二項演算式の解析
    pub fn analyze_binary_expression(&mut self, binary: &BinaryExpr) -> AnalysisResult<Type> {
        let left_type = self.analyze_expression(&binary.left)?;
        let right_type = self.analyze_expression(&binary.right)?;
        
        self.type_checker.binary_op_result_type(&binary.op, &left_type, &right_type, binary.span)
    }

    /// 単項演算式の解析
    pub fn analyze_unary_expression(&mut self, unary: &UnaryExpr) -> AnalysisResult<Type> {
        let operand_type = self.analyze_expression(&unary.expr)?;
        
        self.type_checker.unary_op_result_type(&unary.op, &operand_type, unary.span)
    }

    fn analyze_path_expression(&self, path_expr: &PathExpr) -> AnalysisResult<Type> {
        // パス式（Enum::Variantなど）の解析
        // 2つのセグメントの場合、Enum variantとして処理
        if path_expr.segments.len() == 2 {
            // これはパーサーのバグで、本来はEnumVariantExprとして解析されるべき
            // しかし、とりあえずEnum型として処理
            return Ok(Type::UserDefined(path_expr.segments[0].clone()));
        }
        
        // その他のパス式は未実装
        Err(AnalysisError::UndefinedVariable {
            name: path_expr.segments.join("::"),
            span: path_expr.span,
        })
    }

    fn analyze_index_expression(&mut self, index_expr: &IndexExpr) -> AnalysisResult<Type> {
        // インデックスアクセスの解析
        let object_type = self.analyze_expression(&index_expr.object)?;
        let index_type = self.analyze_expression(&index_expr.index)?;
        
        // 配列型の場合、要素型を返す
        match object_type {
            Type::Array(elem_type) => {
                // インデックスが整数型であることを確認
                if !self.type_checker.is_integer_type(&index_type) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "integer type".to_string(),
                        found: self.type_checker.type_to_string(&index_type),
                        span: index_expr.span,
                    });
                }
                Ok(*elem_type)
            }
            _ => Err(AnalysisError::TypeMismatch {
                expected: "array type".to_string(),
                found: self.type_checker.type_to_string(&object_type),
                span: index_expr.span,
            }),
        }
    }

    fn analyze_reference_expression(&mut self, ref_expr: &ReferenceExpr) -> AnalysisResult<Type> {
        // 参照式の解析
        let inner_type = self.analyze_expression(&ref_expr.expr)?;
        Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
    }

    fn analyze_dereference_expression(&mut self, deref_expr: &DereferenceExpr) -> AnalysisResult<Type> {
        // 参照外し式の解析
        let ref_type = self.analyze_expression(&deref_expr.expr)?;
        match ref_type {
            Type::Reference(inner_type, _) => Ok(*inner_type),
            _ => Err(AnalysisError::TypeMismatch {
                expected: "reference type".to_string(),
                found: self.type_checker.type_to_string(&ref_type),
                span: deref_expr.span,
            }),
        }
    }

    fn analyze_assignment_expression(&mut self, assign_expr: &AssignmentExpr) -> AnalysisResult<Type> {
        // 代入式の解析
        let target_type = self.analyze_expression(&assign_expr.target)?;
        let value_type = self.analyze_expression(&assign_expr.value)?;
        
        // 型の互換性チェック
        self.type_checker.check_type_compatibility(&target_type, &value_type, assign_expr.span)?;
        
        // 代入式の値はunit型
        Ok(Type::Void)
    }

    fn analyze_tuple_expression(&mut self, tuple_expr: &TupleExpr) -> AnalysisResult<Type> {
        // タプル式の解析
        let mut element_types = Vec::new();
        for elem in &tuple_expr.elements {
            element_types.push(self.analyze_expression(elem)?);
        }
        Ok(Type::Tuple(element_types))
    }

    fn analyze_cast_expression(&mut self, cast: &CastExpr) -> AnalysisResult<Type> {
        self.analyze_expression(&cast.expr)?;
        self.type_checker.validate_type(&cast.ty, cast.span)?;
        Ok(cast.ty.clone())
    }

    /// 関数呼び出し式の解析
    pub fn analyze_call_expression(&mut self, call: &CallExpr) -> AnalysisResult<Type> {
        if let Expression::Identifier(ident) = call.callee.as_ref() {
            // println関数の特別な処理（任意の数の引数と型を受け入れる）
            if ident.name == "println" {
                // 引数がない場合は改行のみを出力
                // 全ての引数の型を解析するが、型チェックはしない（任意の型を受け入れる）
                for arg in &call.args {
                    self.analyze_expression(arg)?;
                }
                return Ok(Type::Void);
            }
            
            if let Some(func_sig) = self.type_checker.get_function_signature(&ident.name).cloned() {
                // 引数数のチェック
                if call.args.len() != func_sig.params.len() {
                    return Err(AnalysisError::ArgumentCountMismatch {
                        expected: func_sig.params.len(),
                        found: call.args.len(),
                        span: call.span,
                    });
                }
                
                // 各引数の型チェック
                for (i, arg) in call.args.iter().enumerate() {
                    let arg_type = self.analyze_expression(arg)?;
                    let expected_type = &func_sig.params[i].1;
                    self.type_checker.check_type_compatibility(expected_type, &arg_type, call.span)?;
                }
                
                Ok(func_sig.return_type)
            } else {
                Err(AnalysisError::UndefinedFunction {
                    name: ident.name.clone(),
                    span: call.span,
                })
            }
        } else {
            // 関数ポインタ呼び出しなど、将来の拡張
            Ok(Type::Void)
        }
    }

    /// フィールドアクセス式の解析
    pub fn analyze_field_expression(&mut self, field: &FieldExpr) -> AnalysisResult<Type> {
        let object_type = self.analyze_expression(&field.object)?;
        self.type_checker.get_field_type(&object_type, &field.field, field.span)
    }

    /// 構造体リテラル式の解析
    pub fn analyze_struct_literal(&mut self, struct_lit: &StructLiteral) -> AnalysisResult<Type> {
        // 構造体型の検証
        let struct_name = struct_lit.name.clone();
        let struct_span = struct_lit.span;
        
        if let Some(type_info) = self.type_checker.get_type_info(&struct_name) {
            let fields = match &type_info.kind {
                TypeKind::Struct(fields) => fields.clone(),
                _ => return Err(AnalysisError::InvalidOperation {
                    message: format!("Type {} is not a struct", struct_name),
                    span: struct_span,
                }),
            };
            
            // 各フィールドの型チェック
            for field_init in &struct_lit.fields {
                if let Some(field_def) = fields.iter().find(|f| f.name == field_init.name) {
                    let value_type = self.analyze_expression(&field_init.value)?;
                    self.type_checker.check_type_compatibility(&field_def.ty, &value_type, struct_span)?;
                } else {
                    return Err(AnalysisError::UndefinedVariable {
                        name: format!("{}.{}", struct_name, field_init.name),
                        span: struct_span,
                    });
                }
            }
            
            Ok(Type::UserDefined(struct_name))
        } else {
            Err(AnalysisError::UndefinedType {
                name: struct_name,
                span: struct_span,
            })
        }
    }

    /// 配列式の解析
    pub fn analyze_array_expression(&mut self, array: &ArrayExpr) -> AnalysisResult<Type> {
        if array.elements.is_empty() {
            // 空配列の場合、型を推論できない
            return Err(AnalysisError::TypeInferenceError {
                name: "array".to_string(),
                span: array.span,
            });
        }
        
        // 最初の要素の型を基準とする
        let first_element_type = self.analyze_expression(&array.elements[0])?;
        
        // 残りの要素の型が一致するかチェック
        for element in array.elements.iter().skip(1) {
            let element_type = self.analyze_expression(element)?;
            if !self.type_checker.types_compatible(&first_element_type, &element_type) {
                return Err(AnalysisError::TypeMismatch {
                    expected: self.type_checker.type_to_string(&first_element_type),
                    found: self.type_checker.type_to_string(&element_type),
                    span: self.get_expression_span(element),
                });
            }
        }
        
        Ok(Type::Array(Box::new(first_element_type)))
    }

    /// テンプレート文字列の解析
    fn analyze_template_string(&mut self, template: &TemplateStringLit) -> AnalysisResult<Type> {
        // 各補間式の型を解析
        for part in &template.parts {
            if let TemplateStringPart::Interpolation(expr) = part {
                // 補間式の型を解析（任意の型を許可）
                self.analyze_expression(expr)?;
            }
        }
        
        // テンプレート文字列の結果型は常にString
        Ok(Type::String)
    }
}