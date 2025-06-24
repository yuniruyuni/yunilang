//! 型推論機能
//!
//! ジェネリック型の型パラメータを推論する

use std::collections::HashMap;
use crate::ast::Type;
use crate::error::YuniResult;
use super::symbol::AnalysisError;
use super::type_env::TypeEnvironment;

/// 型推論エンジン
pub struct TypeInference<'a> {
    /// 型環境への参照
    type_env: &'a mut TypeEnvironment,
}

impl<'a> TypeInference<'a> {
    /// 新しい型推論エンジンを作成
    pub fn new(type_env: &'a mut TypeEnvironment) -> Self {
        Self { type_env }
    }
    
    /// 2つの型を統一（unify）し、型変数のバインディングを更新
    /// 
    /// # Arguments
    /// * `expected` - 期待される型（型パラメータを含む可能性がある）
    /// * `actual` - 実際の型（具体的な型）
    /// * `span` - エラー報告用のソース位置
    pub fn unify(&mut self, expected: &Type, actual: &Type, span: crate::ast::Span) -> YuniResult<()> {
        match (expected, actual) {
            // 型変数の場合、バインディングを設定
            (Type::Variable(name), actual_type) => {
                if let Some(bound_type) = self.type_env.get_binding(name).cloned() {
                    // すでにバインディングがある場合、その型と統一
                    self.unify(&bound_type, actual_type, span)?;
                } else if self.type_env.is_type_param(name) {
                    // 型パラメータの場合、バインディングを設定
                    self.type_env.bind_type(name.clone(), actual_type.clone());
                } else {
                    // 未定義の型変数
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::UndefinedType {
                            name: name.clone(),
                            span,
                        }
                    ));
                }
                Ok(())
            }
            
            // 逆方向の型変数
            (actual_type, Type::Variable(name)) => {
                self.unify(&Type::Variable(name.clone()), actual_type, span)
            }
            
            // ジェネリック型の統一
            (Type::Generic(expected_name, expected_args), Type::Generic(actual_name, actual_args)) => {
                if expected_name != actual_name {
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::TypeMismatch {
                            expected: expected_name.clone(),
                            found: actual_name.clone(),
                            span,
                        }
                    ));
                }
                
                if expected_args.len() != actual_args.len() {
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::TypeMismatch {
                            expected: format!("{}<{} args>", expected_name, expected_args.len()),
                            found: format!("{}<{} args>", actual_name, actual_args.len()),
                            span,
                        }
                    ));
                }
                
                // 各型引数を再帰的に統一
                for (expected_arg, actual_arg) in expected_args.iter().zip(actual_args.iter()) {
                    self.unify(expected_arg, actual_arg, span)?;
                }
                Ok(())
            }
            
            // 配列型の統一
            (Type::Array(expected_elem), Type::Array(actual_elem)) => {
                self.unify(expected_elem, actual_elem, span)
            }
            
            // 参照型の統一
            (Type::Reference(expected_inner, expected_mut), Type::Reference(actual_inner, actual_mut)) => {
                if expected_mut != actual_mut {
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::TypeMismatch {
                            expected: if *expected_mut { "&mut _" } else { "&_" }.to_string(),
                            found: if *actual_mut { "&mut _" } else { "&_" }.to_string(),
                            span,
                        }
                    ));
                }
                self.unify(expected_inner, actual_inner, span)
            }
            
            // タプル型の統一
            (Type::Tuple(expected_types), Type::Tuple(actual_types)) => {
                if expected_types.len() != actual_types.len() {
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::TypeMismatch {
                            expected: format!("tuple of {} elements", expected_types.len()),
                            found: format!("tuple of {} elements", actual_types.len()),
                            span,
                        }
                    ));
                }
                
                for (expected_elem, actual_elem) in expected_types.iter().zip(actual_types.iter()) {
                    self.unify(expected_elem, actual_elem, span)?;
                }
                Ok(())
            }
            
            // 関数型の統一
            (Type::Function(expected_fn), Type::Function(actual_fn)) => {
                if expected_fn.params.len() != actual_fn.params.len() {
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::TypeMismatch {
                            expected: format!("function with {} parameters", expected_fn.params.len()),
                            found: format!("function with {} parameters", actual_fn.params.len()),
                            span,
                        }
                    ));
                }
                
                // パラメータ型を統一
                for (expected_param, actual_param) in expected_fn.params.iter().zip(actual_fn.params.iter()) {
                    self.unify(expected_param, actual_param, span)?;
                }
                
                // 戻り値型を統一
                self.unify(&expected_fn.return_type, &actual_fn.return_type, span)
            }
            
            // 同じ型の場合は成功
            (expected, actual) if expected == actual => Ok(()),
            
            // その他は型不一致
            _ => Err(crate::error::YuniError::Analyzer(
                AnalysisError::TypeMismatch {
                    expected: format!("{:?}", expected),
                    found: format!("{:?}", actual),
                    span,
                }
            )),
        }
    }
    
    /// 型から型変数への置換マッピングを推論
    /// 
    /// # Arguments
    /// * `generic_type` - ジェネリック型（型変数を含む）
    /// * `concrete_type` - 具体的な型
    /// 
    /// # Returns
    /// 型変数名から具体的な型へのマッピング
    #[allow(dead_code)]
    pub fn infer_type_substitutions(
        &self,
        generic_type: &Type,
        concrete_type: &Type,
    ) -> YuniResult<HashMap<String, Type>> {
        let mut substitutions = HashMap::new();
        self.collect_substitutions(generic_type, concrete_type, &mut substitutions)?;
        Ok(substitutions)
    }
    
    /// 型変数の置換を収集（内部実装）
    #[allow(dead_code)]
    fn collect_substitutions(
        &self,
        generic_type: &Type,
        concrete_type: &Type,
        substitutions: &mut HashMap<String, Type>,
    ) -> YuniResult<()> {
        match (generic_type, concrete_type) {
            (Type::Variable(name), concrete) => {
                if let Some(existing) = substitutions.get(name) {
                    // すでに置換が存在する場合、一致することを確認
                    if existing != concrete {
                        return Err(crate::error::YuniError::Analyzer(
                            AnalysisError::TypeMismatch {
                                expected: format!("{:?}", existing),
                                found: format!("{:?}", concrete),
                                span: crate::ast::Span::dummy(),
                            }
                        ));
                    }
                } else {
                    substitutions.insert(name.clone(), concrete.clone());
                }
                Ok(())
            }
            
            (Type::Generic(g_name, g_args), Type::Generic(c_name, c_args)) if g_name == c_name => {
                for (g_arg, c_arg) in g_args.iter().zip(c_args.iter()) {
                    self.collect_substitutions(g_arg, c_arg, substitutions)?;
                }
                Ok(())
            }
            
            (Type::Array(g_elem), Type::Array(c_elem)) => {
                self.collect_substitutions(g_elem, c_elem, substitutions)
            }
            
            (Type::Reference(g_inner, g_mut), Type::Reference(c_inner, c_mut)) if g_mut == c_mut => {
                self.collect_substitutions(g_inner, c_inner, substitutions)
            }
            
            (Type::Tuple(g_types), Type::Tuple(c_types)) if g_types.len() == c_types.len() => {
                for (g_type, c_type) in g_types.iter().zip(c_types.iter()) {
                    self.collect_substitutions(g_type, c_type, substitutions)?;
                }
                Ok(())
            }
            
            (Type::Function(g_fn), Type::Function(c_fn)) if g_fn.params.len() == c_fn.params.len() => {
                for (g_param, c_param) in g_fn.params.iter().zip(c_fn.params.iter()) {
                    self.collect_substitutions(g_param, c_param, substitutions)?;
                }
                self.collect_substitutions(&g_fn.return_type, &c_fn.return_type, substitutions)
            }
            
            (g, c) if g == c => Ok(()),
            
            _ => Err(crate::error::YuniError::Analyzer(
                AnalysisError::TypeMismatch {
                    expected: format!("{:?}", generic_type),
                    found: format!("{:?}", concrete_type),
                    span: crate::ast::Span::dummy(),
                }
            )),
        }
    }
    
    /// 型に置換を適用
    #[allow(dead_code)]
    pub fn apply_substitutions(&self, ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Variable(name) => {
                substitutions.get(name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::Generic(name, args) => {
                let substituted_args: Vec<Type> = args
                    .iter()
                    .map(|arg| self.apply_substitutions(arg, substitutions))
                    .collect();
                Type::Generic(name.clone(), substituted_args)
            }
            Type::Array(elem_ty) => {
                Type::Array(Box::new(self.apply_substitutions(elem_ty, substitutions)))
            }
            Type::Reference(inner_ty, is_mut) => {
                Type::Reference(Box::new(self.apply_substitutions(inner_ty, substitutions)), *is_mut)
            }
            Type::Tuple(types) => {
                let substituted_types: Vec<Type> = types
                    .iter()
                    .map(|t| self.apply_substitutions(t, substitutions))
                    .collect();
                Type::Tuple(substituted_types)
            }
            Type::Function(fn_type) => {
                let substituted_params: Vec<Type> = fn_type.params
                    .iter()
                    .map(|p| self.apply_substitutions(p, substitutions))
                    .collect();
                let substituted_return = self.apply_substitutions(&fn_type.return_type, substitutions);
                Type::Function(crate::ast::FunctionType {
                    params: substituted_params,
                    return_type: Box::new(substituted_return),
                })
            }
            _ => ty.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Span, TypeParam};
    
    #[test]
    fn test_unify_simple_types() {
        let mut env = TypeEnvironment::new();
        let params = vec![TypeParam { name: "T".to_string(), span: Span::dummy() }];
        env.register_type_params(&params).unwrap();
        
        let mut inference = TypeInference::new(&mut env);
        
        // T を i32 に統一
        inference.unify(&Type::Variable("T".to_string()), &Type::I32, Span::dummy()).unwrap();
        
        // バインディングが設定されたことを確認
        assert_eq!(env.get_binding("T"), Some(&Type::I32));
    }
    
    #[test]
    fn test_unify_generic_types() {
        let mut env = TypeEnvironment::new();
        let params = vec![TypeParam { name: "T".to_string(), span: Span::dummy() }];
        env.register_type_params(&params).unwrap();
        
        let mut inference = TypeInference::new(&mut env);
        
        // Vec<T> を Vec<i32> に統一
        let expected = Type::Generic("Vec".to_string(), vec![Type::Variable("T".to_string())]);
        let actual = Type::Generic("Vec".to_string(), vec![Type::I32]);
        
        inference.unify(&expected, &actual, Span::dummy()).unwrap();
        
        // T が i32 にバインドされたことを確認
        assert_eq!(env.get_binding("T"), Some(&Type::I32));
    }
    
    #[test]
    fn test_infer_substitutions() {
        let mut env = TypeEnvironment::new();
        let inference = TypeInference::new(&mut env);
        
        // identity<T>(x: T) -> T に対して identity(42) を呼び出す場合
        let generic_type = Type::Function(crate::ast::FunctionType {
            params: vec![Type::Variable("T".to_string())],
            return_type: Box::new(Type::Variable("T".to_string())),
        });
        
        let concrete_type = Type::Function(crate::ast::FunctionType {
            params: vec![Type::I32],
            return_type: Box::new(Type::I32),
        });
        
        let substitutions = inference.infer_type_substitutions(&generic_type, &concrete_type).unwrap();
        
        assert_eq!(substitutions.len(), 1);
        assert_eq!(substitutions.get("T"), Some(&Type::I32));
    }
}