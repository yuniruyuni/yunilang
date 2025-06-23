//! 複雑な式（match、enum variant、メソッド呼び出しなど）の解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisError, AnalysisResult, Symbol, TypeInfo, TypeKind};
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// match式の解析
    pub fn analyze_match_expression(&mut self, match_expr: &MatchExpr) -> AnalysisResult<Type> {
        // match対象の式を解析
        let expr_type = self.analyze_expression(&match_expr.expr)?;
        
        if match_expr.arms.is_empty() {
            return Err(AnalysisError::TypeMismatch {
                expected: "non-empty match".to_string(),
                found: "empty match".to_string(),
                span: match_expr.span,
            });
        }
        
        // 最初のarmの型を基準とする
        let first_arm = &match_expr.arms[0];
        self.analyze_pattern(&first_arm.pattern, &expr_type)?;
        let expected_type = self.analyze_expression(&first_arm.expr)?;
        
        // 残りのarmの型を確認
        for arm in &match_expr.arms[1..] {
            self.analyze_pattern(&arm.pattern, &expr_type)?;
            let arm_type = self.analyze_expression(&arm.expr)?;
            if !self.type_checker.types_compatible(&expected_type, &arm_type) {
                return Err(AnalysisError::TypeMismatch {
                    expected: format!("{:?}", expected_type),
                    found: format!("{:?}", arm_type),
                    span: match_expr.span,
                });
            }
        }
        
        Ok(expected_type)
    }
    
    /// enum variant式の解析
    pub fn analyze_enum_variant_expression(&mut self, enum_variant: &EnumVariantExpr) -> AnalysisResult<Type> {
        // enum型が定義されているかチェック（借用を避けるためにクローンする）
        let enum_def = if let Some(enum_def) = self.lookup_type(&enum_variant.enum_name) {
            enum_def.clone()
        } else {
            return Err(AnalysisError::UndefinedType {
                name: enum_variant.enum_name.clone(),
                span: enum_variant.span,
            });
        };

        // variant が存在するかチェック
        if let TypeKind::Enum(variants) = &enum_def.kind {
            for variant in variants {
                if variant.name == enum_variant.variant {
                    // フィールドの型チェック
                    match (&enum_variant.fields, &variant.fields) {
                        (crate::ast::EnumVariantFields::Unit, fields) if fields.is_empty() => {
                            return Ok(Type::UserDefined(enum_variant.enum_name.clone()));
                        }
                        (crate::ast::EnumVariantFields::Tuple(args), fields) => {
                            if args.len() != fields.len() {
                                return Err(AnalysisError::ArgumentCountMismatch {
                                    expected: fields.len(),
                                    found: args.len(),
                                    span: enum_variant.span,
                                });
                            }
                            for (arg, field) in args.iter().zip(fields.iter()) {
                                let arg_type = self.analyze_expression(arg)?;
                                if !self.type_checker.types_compatible(&field.ty, &arg_type) {
                                    return Err(AnalysisError::TypeMismatch {
                                        expected: format!("{:?}", field.ty),
                                        found: format!("{:?}", arg_type),
                                        span: enum_variant.span,
                                    });
                                }
                            }
                            return Ok(Type::UserDefined(enum_variant.enum_name.clone()));
                        }
                        (crate::ast::EnumVariantFields::Struct(field_inits), fields) => {
                            for field_init in field_inits {
                                if let Some(field) = fields.iter().find(|f| f.name == field_init.name) {
                                    let value_type = self.analyze_expression(&field_init.value)?;
                                    if !self.type_checker.types_compatible(&field.ty, &value_type) {
                                        return Err(AnalysisError::TypeMismatch {
                                            expected: format!("{:?}", field.ty),
                                            found: format!("{:?}", value_type),
                                            span: enum_variant.span,
                                        });
                                    }
                                } else {
                                    return Err(AnalysisError::UndefinedVariable {
                                        name: field_init.name.clone(),
                                        span: enum_variant.span,
                                    });
                                }
                            }
                            return Ok(Type::UserDefined(enum_variant.enum_name.clone()));
                        }
                        _ => {
                            return Err(AnalysisError::TypeMismatch {
                                expected: "matching variant fields".to_string(),
                                found: "mismatched variant fields".to_string(),
                                span: enum_variant.span,
                            });
                        }
                    }
                }
            }
            Err(AnalysisError::UndefinedVariable {
                name: format!("{}::{}", enum_variant.enum_name, enum_variant.variant),
                span: enum_variant.span,
            })
        } else {
            Err(AnalysisError::TypeMismatch {
                expected: "enum type".to_string(),
                found: format!("{:?}", enum_def.kind),
                span: enum_variant.span,
            })
        }
    }
    
    /// パターンの解析
    pub fn analyze_pattern(&mut self, pattern: &Pattern, expected_type: &Type) -> AnalysisResult<()> {
        match pattern {
            Pattern::Identifier(name, _is_mut) => {
                // パターン変数をスコープに追加
                let symbol = Symbol {
                    name: name.clone(),
                    ty: expected_type.clone(),
                    is_mutable: false,
                    span: crate::ast::Span::dummy(), // TODO: 適切なspan
                    borrow_info: None,
                    is_moved: false,
                    lifetime: None,
                };
                self.scope_stack.last_mut().unwrap().define(symbol)?;
                Ok(())
            }
            Pattern::EnumVariant { enum_name, variant: _, fields: _ } => {
                // enum型が存在することを確認
                if let Type::UserDefined(type_name) = expected_type {
                    if type_name != enum_name {
                        return Err(AnalysisError::TypeMismatch {
                            expected: enum_name.clone(),
                            found: type_name.clone(),
                            span: crate::ast::Span::dummy(), // TODO: 適切なspan
                        });
                    }
                    // TODO: variant とフィールドの詳細チェック
                    Ok(())
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: enum_name.clone(),
                        found: format!("{:?}", expected_type),
                        span: crate::ast::Span::dummy(), // TODO: 適切なspan
                    })
                }
            }
            _ => {
                // 他のパターンは後で実装
                Ok(())
            }
        }
    }
    
    /// メソッド呼び出し式の解析
    pub fn analyze_method_call_expression(&mut self, method_call: &MethodCallExpr) -> AnalysisResult<Type> {
        // オブジェクトの型を取得
        let object_type = self.analyze_expression(&method_call.object)?;
        
        // メソッドが定義されているかチェック（借用を避けるためにクローンする）
        let type_info = if let Some(type_info) = self.lookup_type_info(&object_type) {
            type_info.clone()
        } else {
            return Err(AnalysisError::MethodNotFound {
                method: method_call.method.clone(),
                ty: format!("{:?}", object_type),
                span: method_call.span,
            });
        };
        
        if let Some(method_sig) = type_info.methods.get(&method_call.method) {
            // 引数数のチェック
            if method_call.args.len() != method_sig.params.len() {
                return Err(AnalysisError::ArgumentCountMismatch {
                    expected: method_sig.params.len(),
                    found: method_call.args.len(),
                    span: method_call.span,
                });
            }
            
            // 各引数の型チェック
            for (i, arg) in method_call.args.iter().enumerate() {
                let arg_type = self.analyze_expression(arg)?;
                let expected_type = &method_sig.params[i].1;
                self.type_checker.check_type_compatibility(expected_type, &arg_type, method_call.span)?;
            }
            
            Ok(method_sig.return_type.clone())
        } else {
            Err(AnalysisError::MethodNotFound {
                method: method_call.method.clone(),
                ty: format!("{:?}", object_type),
                span: method_call.span,
            })
        }
    }
    
    /// 型情報を取得（型名から）
    pub fn lookup_type_info(&self, ty: &Type) -> Option<&TypeInfo> {
        match ty {
            Type::UserDefined(name) => self.lookup_type(name),
            _ => None,
        }
    }
    
    /// if式の解析
    pub fn analyze_if_expression(&mut self, if_expr: &IfExpr) -> AnalysisResult<Type> {
        // 条件式をbool型として解析
        let condition_type = self.analyze_expression(&if_expr.condition)?;
        if !matches!(condition_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_checker.type_to_string(&condition_type),
                span: self.get_expression_span(&if_expr.condition),
            });
        }
        
        // then節の解析
        let then_type = self.analyze_expression(&if_expr.then_branch)?;
        
        // else節の解析（存在する場合）
        if let Some(else_branch) = &if_expr.else_branch {
            let else_type = self.analyze_expression(else_branch)?;
            // 両方のブランチの型が一致するかチェック
            if !self.type_checker.types_compatible(&then_type, &else_type) {
                return Err(AnalysisError::TypeMismatch {
                    expected: self.type_checker.type_to_string(&then_type),
                    found: self.type_checker.type_to_string(&else_type),
                    span: self.get_expression_span(else_branch),
                });
            }
            Ok(then_type)
        } else {
            // else節がない場合、then節はunit型である必要がある
            if !matches!(then_type, Type::Void) {
                return Err(AnalysisError::TypeMismatch {
                    expected: "()".to_string(),
                    found: self.type_checker.type_to_string(&then_type),
                    span: self.get_expression_span(&if_expr.then_branch),
                });
            }
            Ok(Type::Void)
        }
    }
    
    /// ブロック式の解析
    pub fn analyze_block_expression(&mut self, block_expr: &BlockExpr) -> AnalysisResult<Type> {
        self.enter_scope();
        
        // 文を順次解析
        for stmt in &block_expr.statements {
            self.analyze_statement(stmt)?;
        }
        
        // 最後の式の型を返す
        let result_type = if let Some(last_expr) = &block_expr.last_expr {
            self.analyze_expression(last_expr)?
        } else {
            Type::Void
        };
        
        self.exit_scope();
        Ok(result_type)
    }
}