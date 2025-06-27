//! 基本的な式のセマンティック解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisResult, AnalysisError, TypeKind};
use crate::analyzer::semantic_analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 関数呼び出し式の解析
    pub fn analyze_call_expression(&mut self, call: &CallExpr) -> AnalysisResult<Type> {
        // 関数名を取得
        let func_name = match &*call.callee {
            Expression::Identifier(id) => &id.name,
            Expression::Path(path) if path.segments.len() == 1 => &path.segments[0],
            _ => return Err(AnalysisError::InvalidType {
                message: "Invalid function call".to_string(),
                span: call.span,
            }),
        };
        
        // 関数が定義されているか確認
        let func_sig = self.type_checker.get_function_signature(func_name)
            .ok_or_else(|| AnalysisError::UndefinedFunction {
                name: func_name.clone(),
                span: call.span,
            })?.clone();
        
        // 引数の型チェック
        if call.args.len() != func_sig.params.len() {
            return Err(AnalysisError::ArgCountMismatch {
                expected: func_sig.params.len(),
                found: call.args.len(),
                span: call.span,
            });
        }
        
        for (_i, (arg, (_, expected_type))) in call.args.iter().zip(func_sig.params.iter()).enumerate() {
            let arg_type = self.analyze_expression(arg)?;
            self.type_checker.check_type_compatibility(expected_type, &arg_type, arg.span())?;
        }
        
        Ok(func_sig.return_type)
    }
    
    /// 識別子の解析
    pub(crate) fn analyze_identifier(&self, ident: &Identifier) -> AnalysisResult<Type> {
        // 現在のスコープから変数を検索
        for scope in self.scope_stack.iter().rev() {
            if let Some(var_info) = scope.lookup(&ident.name) {
                return Ok(var_info.ty.clone());
            }
        }
        
        Err(AnalysisError::UndefinedVariable {
            name: ident.name.clone(),
            span: ident.span,
        })
    }

    /// パス式の解析
    pub(crate) fn analyze_path_expression(&self, path_expr: &PathExpr) -> AnalysisResult<Type> {
        if path_expr.segments.len() == 1 {
            // 単一セグメントのパスは識別子として扱う
            self.analyze_identifier(&Identifier {
                name: path_expr.segments[0].clone(),
                span: path_expr.span,
            })
        } else {
            // TODO: モジュールパスの解決
            Err(AnalysisError::Internal {
                message: "Multi-segment paths not yet implemented".to_string(),
            })
        }
    }

    /// インデックス式の解析
    pub(crate) fn analyze_index_expression(&mut self, index_expr: &IndexExpr) -> AnalysisResult<Type> {
        let object_type = self.analyze_expression(&index_expr.object)?;
        let index_type = self.analyze_expression(&index_expr.index)?;
        
        // インデックスが整数型であることを確認
        match &index_type {
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 => {},
            _ => {
                return Err(AnalysisError::TypeMismatch {
                    expected: "integer type".to_string(),
                    found: format!("{:?}", index_type),
                    span: index_expr.index.span(),
                });
            }
        }
        
        // インデックス可能な型かチェック
        match &object_type {
            Type::Array(element_type) => Ok(*element_type.clone()),
            Type::String => Ok(Type::U8), // 文字列のインデックスはバイト（u8）を返す
            Type::Generic(name, type_args) if name == "Vec" && type_args.len() == 1 => {
                Ok(type_args[0].clone())
            }
            _ => Err(AnalysisError::InvalidType {
                message: format!("Cannot index type {:?}", object_type),
                span: index_expr.object.span(),
            }),
        }
    }

    /// 参照式の解析
    pub(crate) fn analyze_reference_expression(&mut self, ref_expr: &ReferenceExpr) -> AnalysisResult<Type> {
        let inner_type = self.analyze_expression(&ref_expr.expr)?;
        Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
    }

    /// 参照外し式の解析
    pub(crate) fn analyze_dereference_expression(&mut self, deref_expr: &DereferenceExpr) -> AnalysisResult<Type> {
        let expr_type = self.analyze_expression(&deref_expr.expr)?;
        
        match expr_type {
            Type::Reference(inner_type, _is_mut) => Ok(*inner_type),
            _ => Err(AnalysisError::TypeMismatch {
                expected: "reference type".to_string(),
                found: format!("{:?}", expr_type),
                span: deref_expr.span,
            }),
        }
    }

    /// メソッド呼び出し式の解析
    pub fn analyze_method_call(&mut self, method_call: &MethodCallExpr) -> AnalysisResult<Type> {
        // レシーバーの型を取得
        let receiver_type = self.analyze_expression(&method_call.object)?;
        
        // メソッドが定義されているか確認
        let type_name = match &receiver_type {
            Type::UserDefined(name) => name.clone(),
            Type::Generic(name, _) => name.clone(),
            _ => return Err(AnalysisError::InvalidType {
                message: format!("Cannot call method on type {:?}", receiver_type),
                span: method_call.span,
            }),
        };
        
        let method_sig = self.type_checker.get_method_signature(&type_name, &method_call.method)
            .ok_or_else(|| AnalysisError::UndefinedMethod {
                type_name: type_name.clone(),
                method_name: method_call.method.clone(),
                span: method_call.span,
            })?.clone();
        
        // 引数の型チェック
        if method_call.args.len() != method_sig.params.len() {
            return Err(AnalysisError::ArgCountMismatch {
                expected: method_sig.params.len(),
                found: method_call.args.len(),
                span: method_call.span,
            });
        }
        
        for (arg, (_, expected_type)) in method_call.args.iter().zip(method_sig.params.iter()) {
            let arg_type = self.analyze_expression(arg)?;
            self.type_checker.check_type_compatibility(expected_type, &arg_type, arg.span())?;
        }
        
        Ok(method_sig.return_type)
    }
    
    /// フィールドアクセス式の解析
    pub fn analyze_field_expression(&mut self, field: &FieldExpr) -> AnalysisResult<Type> {
        let object_type = self.analyze_expression(&field.object)?;
        
        // フィールドの型を取得
        self.type_checker.get_field_type(&object_type, &field.field, field.span)
    }
    
    /// 構造体リテラルの解析
    pub fn analyze_struct_literal(&mut self, struct_lit: &StructLiteral) -> AnalysisResult<Type> {
        // 型名が指定されていない場合はエラー
        let struct_name = struct_lit.name.as_ref()
            .ok_or_else(|| AnalysisError::InvalidType {
                message: "Struct literal must have a type name".to_string(),
                span: struct_lit.span,
            })?;
        
        // 構造体が定義されているか確認
        let struct_info = self.type_checker.get_type_info(struct_name)
            .ok_or_else(|| AnalysisError::UndefinedType {
                name: struct_name.clone(),
                span: struct_lit.span,
            })?.clone();
        
        // 各フィールドの型チェック
        match &struct_info.kind {
            TypeKind::Struct(fields) => {
                let fields = fields.clone();
                for field_init in &struct_lit.fields {
                    let expected_field = fields.iter()
                        .find(|f| f.name == field_init.name)
                        .ok_or_else(|| AnalysisError::UndefinedField {
                            struct_name: struct_name.clone(),
                            field_name: field_init.name.clone(),
                            span: struct_lit.span,
                        })?;
                    
                    let field_type = self.analyze_expression(&field_init.value)?;
                    self.type_checker.check_type_compatibility(&expected_field.ty, &field_type, struct_lit.span)?;
                }
                
                Ok(Type::UserDefined(struct_name.clone()))
            }
            _ => Err(AnalysisError::NotAStruct {
                name: struct_name.clone(),
                span: struct_lit.span,
            }),
        }
    }
    
    /// Enumバリアント式の解析
    pub fn analyze_enum_variant(&mut self, enum_variant: &EnumVariantExpr) -> AnalysisResult<Type> {
        self.analyze_enum_variant_expression(enum_variant)
    }
}