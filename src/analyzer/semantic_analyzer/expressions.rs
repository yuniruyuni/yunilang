//! 式の解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisError, AnalysisResult, TypeKind};
use crate::analyzer::type_inference::TypeInference;
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
            Expression::ListLiteral(list) => self.analyze_list_literal(list, expected_type),
            Expression::MapLiteral(map) => self.analyze_map_literal(map, expected_type),
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
        
        // 配列型またはVec型の場合、要素型を返す
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
            Type::Generic(name, type_args) if name == "Vec" && type_args.len() == 1 => {
                // Vec型の場合
                // インデックスが整数型であることを確認
                if !self.type_checker.is_integer_type(&index_type) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "integer type".to_string(),
                        found: self.type_checker.type_to_string(&index_type),
                        span: index_expr.span,
                    });
                }
                Ok(type_args[0].clone())
            }
            _ => Err(AnalysisError::TypeMismatch {
                expected: "array or Vec type".to_string(),
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
                
                // ジェネリック関数の場合、型推論を行う
                if !func_sig.type_params.is_empty() {
                    // 型パラメータのスコープを開始
                    self.type_env.enter_scope();
                    
                    // 型パラメータを環境に登録
                    if let Err(e) = self.type_env.register_type_params(&func_sig.type_params) {
                        return match e {
                            crate::error::YuniError::Analyzer(ae) => Err(ae),
                            _ => Err(AnalysisError::InvalidOperation {
                                message: format!("Unexpected error in type parameter registration: {:?}", e),
                                span: call.span,
                            }),
                        };
                    }
                    
                    // 各引数の型を収集
                    let mut arg_types = Vec::new();
                    for arg in &call.args {
                        arg_types.push(self.analyze_expression(arg)?);
                    }
                    
                    // 型推論エンジンを作成して型パラメータを推論
                    let mut inference = TypeInference::new(&mut self.type_env);
                    for (i, arg_type) in arg_types.iter().enumerate() {
                        let expected_type = &func_sig.params[i].1;
                        
                        // 型を統一（型変数のバインディングを設定）
                        if let Err(e) = inference.unify(expected_type, arg_type, call.span) {
                            self.type_env.exit_scope(); // スコープをクリーンアップ
                            return match e {
                                crate::error::YuniError::Analyzer(ae) => Err(ae),
                                _ => Err(AnalysisError::InvalidOperation {
                                    message: format!("Type inference error: {:?}", e),
                                    span: call.span,
                                }),
                            };
                        }
                    }
                    
                    // 推論された型で戻り値型を具体化
                    let instantiated_return_type = self.type_env.instantiate_type(&func_sig.return_type);
                    
                    // 型パラメータのスコープを終了
                    self.type_env.exit_scope();
                    
                    Ok(instantiated_return_type)
                } else {
                    // 非ジェネリック関数の場合、従来通りの処理
                    for (i, arg) in call.args.iter().enumerate() {
                        let arg_type = self.analyze_expression(arg)?;
                        let expected_type = &func_sig.params[i].1;
                        self.type_checker.check_type_compatibility(expected_type, &arg_type, call.span)?;
                    }
                    
                    Ok(func_sig.return_type)
                }
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
        // 型名が指定されていない場合は、文脈から推論される必要がある
        let struct_name = match &struct_lit.name {
            Some(name) => name.clone(),
            None => {
                return Err(AnalysisError::InvalidOperation {
                    message: "Type inference for anonymous struct literals not yet implemented".to_string(),
                    span: struct_lit.span,
                });
            }
        };
        let struct_span = struct_lit.span;
        
        if let Some(type_info) = self.type_checker.get_type_info(&struct_name).cloned() {
            let fields = match &type_info.kind {
                TypeKind::Struct(fields) => fields.clone(),
                _ => return Err(AnalysisError::InvalidOperation {
                    message: format!("Type {} is not a struct", struct_name),
                    span: struct_span,
                }),
            };
            
            // ジェネリック構造体の場合、型推論を行う
            if !type_info.type_params.is_empty() {
                // 型パラメータのスコープを開始
                self.type_env.enter_scope();
                
                // 型パラメータを環境に登録
                if let Err(e) = self.type_env.register_type_params(&type_info.type_params) {
                    return match e {
                        crate::error::YuniError::Analyzer(ae) => Err(ae),
                        _ => Err(AnalysisError::InvalidOperation {
                            message: format!("Unexpected error in type parameter registration: {:?}", e),
                            span: struct_span,
                        }),
                    };
                }
                
                // 各フィールドの値の型を収集
                let mut field_value_types = Vec::new();
                for field_init in &struct_lit.fields {
                    if let Some(field_def) = fields.iter().find(|f| f.name == field_init.name) {
                        let value_type = self.analyze_expression(&field_init.value)?;
                        field_value_types.push((field_def.ty.clone(), value_type));
                    } else {
                        self.type_env.exit_scope(); // スコープをクリーンアップ
                        return Err(AnalysisError::UndefinedVariable {
                            name: format!("{}.{}", struct_name, field_init.name),
                            span: struct_span,
                        });
                    }
                }
                
                // 型推論エンジンを作成して型パラメータを推論
                let mut inference = TypeInference::new(&mut self.type_env);
                for (field_type, value_type) in &field_value_types {
                    // 型を統一（型変数のバインディングを設定）
                    if let Err(e) = inference.unify(field_type, value_type, struct_span) {
                        self.type_env.exit_scope(); // スコープをクリーンアップ
                        return match e {
                            crate::error::YuniError::Analyzer(ae) => Err(ae),
                            _ => Err(AnalysisError::InvalidOperation {
                                message: format!("Type inference error: {:?}", e),
                                span: struct_span,
                            }),
                        };
                    }
                }
                
                // 推論された型パラメータを収集
                let mut type_args = Vec::new();
                for type_param in &type_info.type_params {
                    if let Some(binding) = self.type_env.get_binding(&type_param.name) {
                        type_args.push(binding.clone());
                    } else {
                        self.type_env.exit_scope(); // スコープをクリーンアップ
                        return Err(AnalysisError::TypeInferenceError {
                            name: type_param.name.clone(),
                            span: struct_span,
                        });
                    }
                }
                
                // 型パラメータのスコープを終了
                self.type_env.exit_scope();
                
                // ジェネリック型を返す
                Ok(Type::Generic(struct_name, type_args))
            } else {
                // 非ジェネリック構造体の場合、従来通りの処理
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
            }
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

    /// 初期化式の解析
    fn analyze_list_literal(&mut self, list: &ListLiteral, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &list.type_name {
            if type_name == "Vec" && type_args.len() == 1 {
                let element_type = &type_args[0];
                
                // 各要素の型をチェック
                for elem in &list.elements {
                    let elem_type = self.analyze_expression(elem)?;
                    self.type_checker.check_type_compatibility(element_type, &elem_type, elem.span())?;
                }
                
                return Ok(Type::Generic("Vec".to_string(), type_args.clone()));
            } else {
                return Err(AnalysisError::InvalidOperation {
                    message: format!("Unknown list type: {}", type_name),
                    span: list.span,
                });
            }
        }
        
        // 型名が省略されている場合、期待される型または要素から推論
        if let Some(expected) = expected_type {
            if let Type::Generic(name, args) = expected {
                if name == "Vec" && args.len() == 1 {
                    let element_type = &args[0];
                    
                    for elem in &list.elements {
                        let elem_type = self.analyze_expression(elem)?;
                        self.type_checker.check_type_compatibility(element_type, &elem_type, elem.span())?;
                    }
                    
                    return Ok(expected.clone());
                }
            }
        }
        
        // 要素から型を推論
        if !list.elements.is_empty() {
            let first_type = self.analyze_expression(&list.elements[0])?;
            
            for elem in &list.elements[1..] {
                let elem_type = self.analyze_expression(elem)?;
                self.type_checker.check_type_compatibility(&first_type, &elem_type, elem.span())?;
            }
            
            return Ok(Type::Generic("Vec".to_string(), vec![first_type]));
        }
        
        // 空のリストの場合、エラー
        Err(AnalysisError::InvalidOperation {
            message: "Cannot infer type for empty list literal without type annotation".to_string(),
            span: list.span,
        })
    }
    
    fn analyze_map_literal(&mut self, map: &MapLiteral, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &map.type_name {
            if type_name == "HashMap" && type_args.len() == 2 {
                let key_type = &type_args[0];
                let value_type = &type_args[1];
                
                // 各ペアの型をチェック
                for (key, value) in &map.pairs {
                    let k_type = self.analyze_expression(key)?;
                    let v_type = self.analyze_expression(value)?;
                    self.type_checker.check_type_compatibility(key_type, &k_type, key.span())?;
                    self.type_checker.check_type_compatibility(value_type, &v_type, value.span())?;
                }
                
                return Ok(Type::Generic("HashMap".to_string(), type_args.clone()));
            } else {
                return Err(AnalysisError::InvalidOperation {
                    message: format!("Unknown map type: {}", type_name),
                    span: map.span,
                });
            }
        }
        
        // 型名が省略されている場合、期待される型または要素から推論
        if let Some(expected) = expected_type {
            if let Type::Generic(name, args) = expected {
                if name == "HashMap" && args.len() == 2 {
                    let key_type = &args[0];
                    let value_type = &args[1];
                    
                    for (key, value) in &map.pairs {
                        let k_type = self.analyze_expression(key)?;
                        let v_type = self.analyze_expression(value)?;
                        self.type_checker.check_type_compatibility(key_type, &k_type, key.span())?;
                        self.type_checker.check_type_compatibility(value_type, &v_type, value.span())?;
                    }
                    
                    return Ok(expected.clone());
                }
            }
        }
        
        // 要素から型を推論
        if !map.pairs.is_empty() {
            let (first_key, first_value) = &map.pairs[0];
            let key_type = self.analyze_expression(first_key)?;
            let value_type = self.analyze_expression(first_value)?;
            
            for (key, value) in &map.pairs[1..] {
                let k_type = self.analyze_expression(key)?;
                let v_type = self.analyze_expression(value)?;
                self.type_checker.check_type_compatibility(&key_type, &k_type, key.span())?;
                self.type_checker.check_type_compatibility(&value_type, &v_type, value.span())?;
            }
            
            return Ok(Type::Generic("HashMap".to_string(), vec![key_type, value_type]));
        }
        
        // 空のマップの場合、エラー
        Err(AnalysisError::InvalidOperation {
            message: "Cannot infer type for empty map literal without type annotation".to_string(),
            span: map.span,
        })
    }
    
//     // 以下は削除される古い関数
//     fn analyze_initializer_expression(&mut self, init_expr: &InitializerExpr, _expected_type: Option<&Type>) -> AnalysisResult<Type> {
//         match &init_expr.constructor {
//             InitializerConstructor::Type { name, type_args } => {
//                 // 構造体の初期化
//                 if self.type_checker.has_type(name) {
//                     // 既存の構造体リテラルとして変換
//                     let mut fields = Vec::new();
//                     for element in &init_expr.elements {
//                         match element {
//                             InitializerElement::Named { name, value } => {
//                                 fields.push(StructFieldInit {
//                                     name: name.clone(),
//                                     value: value.clone(),
//                                 });
//                             }
//                             _ => {
//                                 return Err(AnalysisError::InvalidOperation {
//                                     message: format!("Expected named field initialization for struct {}, found positional or key-value initialization", name),
//                                     span: init_expr.span,
//                                 });
//                             }
//                         }
//                     }
//                     
//                     let struct_lit = StructLiteral {
//                         name: name.clone(),
//                         fields,
//                         span: init_expr.span,
//                     };
//                     
//                     self.analyze_struct_literal(&struct_lit)
//                 } 
//                 // 標準ライブラリ型の初期化
//                 else if name == "Vec" {
//                     // Vec<T>の初期化
//                     if type_args.len() != 1 {
//                         return Err(AnalysisError::InvalidOperation {
//                             message: format!("Vec requires exactly one type argument, found {}", type_args.len()),
//                             span: init_expr.span,
//                         });
//                     }
//                     
//                     let element_type = &type_args[0];
//                     
//                     // すべての要素の型をチェック
//                     for element in &init_expr.elements {
//                         match element {
//                             InitializerElement::Positional(expr) => {
//                                 let elem_type = self.analyze_expression(expr)?;
//                                 self.type_checker.check_type_compatibility(element_type, &elem_type, expr.span())?;
//                             }
//                             _ => {
//                                 return Err(AnalysisError::InvalidOperation {
//                                     message: "Vec initialization requires positional elements".to_string(),
//                                     span: init_expr.span,
//                                 });
//                             }
//                         }
//                     }
//                     
//                     // Vec<T>型を返す
//                     Ok(Type::Generic("Vec".to_string(), type_args.clone()))
//                 }
//                 else if name == "HashMap" {
//                     // HashMap<K, V>の初期化
//                     if type_args.len() != 2 {
//                         return Err(AnalysisError::InvalidOperation {
//                             message: format!("HashMap requires exactly two type arguments, found {}", type_args.len()),
//                             span: init_expr.span,
//                         });
//                     }
//                     
//                     let key_type = &type_args[0];
//                     let value_type = &type_args[1];
//                     
//                     // すべての要素の型をチェック
//                     for element in &init_expr.elements {
//                         match element {
//                             InitializerElement::KeyValue { key, value } => {
//                                 let k_type = self.analyze_expression(key)?;
//                                 let v_type = self.analyze_expression(value)?;
//                                 self.type_checker.check_type_compatibility(key_type, &k_type, key.span())?;
//                                 self.type_checker.check_type_compatibility(value_type, &v_type, value.span())?;
//                             }
//                             _ => {
//                                 return Err(AnalysisError::InvalidOperation {
//                                     message: "HashMap initialization requires key-value pairs".to_string(),
//                                     span: init_expr.span,
//                                 });
//                             }
//                         }
//                     }
//                     
//                     // HashMap<K, V>型を返す
//                     Ok(Type::Generic("HashMap".to_string(), type_args.clone()))
//                 }
//                 else if name == "Some" || name == "Ok" || name == "Err" {
//                     // Option/Result型のバリアント
//                     if init_expr.elements.len() != 1 {
//                         return Err(AnalysisError::InvalidOperation {
//                             message: format!("{} requires exactly one argument", name),
//                             span: init_expr.span,
//                         });
//                     }
//                     
//                     match &init_expr.elements[0] {
//                         InitializerElement::Positional(expr) => {
//                             let inner_type = self.analyze_expression(expr)?;
//                             
//                             // 型を推論
//                             match name.as_str() {
//                                 "Some" => Ok(Type::Generic("Option".to_string(), vec![inner_type])),
//                                 "Ok" => {
//                                     // Result<T, E>の場合、エラー型は推論できないのでプレースホルダを使用
//                                     // TODO: より良い型推論
//                                     Ok(Type::Generic("Result".to_string(), vec![inner_type, Type::Variable("E".to_string())]))
//                                 }
//                                 "Err" => {
//                                     // Result<T, E>の場合、成功型は推論できないのでプレースホルダを使用
//                                     Ok(Type::Generic("Result".to_string(), vec![Type::Variable("T".to_string()), inner_type]))
//                                 }
//                                 _ => unreachable!()
//                             }
//                         }
//                         InitializerElement::Named { name: field_name, value } => {
//                             // Ok { value: 42 } のような名前付き初期化もサポート
//                             if field_name == "value" && name == "Ok" {
//                                 let inner_type = self.analyze_expression(value)?;
//                                 Ok(Type::Generic("Result".to_string(), vec![inner_type, Type::Variable("E".to_string())]))
//                             } else {
//                                 Err(AnalysisError::InvalidOperation {
//                                     message: format!("Invalid field name '{}' for {}", field_name, name),
//                                     span: init_expr.span,
//                                 })
//                             }
//                         }
//                         _ => {
//                             Err(AnalysisError::InvalidOperation {
//                                 message: format!("{} initialization requires a single positional argument", name),
//                                 span: init_expr.span,
//                             })
//                         }
//                     }
//                 }
//                 else if name == "None" {
//                     // Noneは引数なし
//                     if !init_expr.elements.is_empty() {
//                         return Err(AnalysisError::InvalidOperation {
//                             message: "None takes no arguments".to_string(),
//                             span: init_expr.span,
//                         });
//                     }
//                     
//                     // Option<T>型を返す（Tは推論される必要がある）
//                     Ok(Type::Generic("Option".to_string(), vec![Type::Variable("T".to_string())]))
//                 }
//                 else {
//                     // 未定義の型
//                     Err(AnalysisError::UndefinedType {
//                         name: name.clone(),
//                         span: init_expr.span,
//                     })
//                 }
//             }
//             InitializerConstructor::Expression(_) => {
//                 // TODO: 式コンストラクタの実装
//                 Err(AnalysisError::InvalidOperation {
//                     message: "Expression constructors are not yet implemented".to_string(),
//                     span: init_expr.span,
//                 })
//             }
//         }
//     }
}