//! 型パラメータ環境の管理
//!
//! ジェネリック型の型パラメータのスコープを管理する

use std::collections::{HashMap, HashSet};
use crate::ast::{Type, TypeParam};
use crate::error::YuniResult;
use super::symbol::AnalysisError;

/// 型パラメータ環境
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// 現在のスコープの型パラメータ
    type_params: Vec<HashSet<String>>,
    /// 型変数の具体的な型へのマッピング（型推論後）
    type_bindings: HashMap<String, Type>,
}

impl TypeEnvironment {
    /// 新しい型環境を作成
    pub fn new() -> Self {
        Self {
            type_params: vec![HashSet::new()],
            type_bindings: HashMap::new(),
        }
    }
    
    /// 新しいスコープを開始
    pub fn enter_scope(&mut self) {
        self.type_params.push(HashSet::new());
    }
    
    /// スコープを終了
    pub fn exit_scope(&mut self) {
        if let Some(params) = self.type_params.pop() {
            // スコープを抜けるときに、そのスコープの型パラメータのバインディングを削除
            for param in params {
                self.type_bindings.remove(&param);
            }
        }
    }
    
    /// 型パラメータを登録
    pub fn register_type_params(&mut self, params: &[TypeParam]) -> YuniResult<()> {
        if let Some(current_scope) = self.type_params.last_mut() {
            for param in params {
                if !current_scope.insert(param.name.clone()) {
                    return Err(crate::error::YuniError::Analyzer(
                        AnalysisError::TypeMismatch {
                            expected: "unique type parameter".to_string(),
                            found: format!("duplicate type parameter '{}'", param.name),
                            span: param.span,
                        }
                    ));
                }
            }
        }
        Ok(())
    }
    
    /// 型パラメータが定義されているかチェック
    pub fn is_type_param(&self, name: &str) -> bool {
        self.type_params.iter().any(|scope| scope.contains(name))
    }
    
    /// 型変数にバインディングを設定
    pub fn bind_type(&mut self, type_var: String, ty: Type) {
        self.type_bindings.insert(type_var, ty);
    }
    
    /// 型変数のバインディングを取得
    pub fn get_binding(&self, type_var: &str) -> Option<&Type> {
        self.type_bindings.get(type_var)
    }
    
    /// 型を具体化（型変数を具体的な型に置き換え）
    pub fn instantiate_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Variable(name) => {
                // バインディングがあれば具体的な型に置き換え
                if let Some(concrete_type) = self.get_binding(name) {
                    self.instantiate_type(concrete_type)
                } else {
                    ty.clone()
                }
            }
            Type::Generic(name, args) => {
                // 型引数も再帰的に具体化
                let instantiated_args: Vec<Type> = args
                    .iter()
                    .map(|arg| self.instantiate_type(arg))
                    .collect();
                Type::Generic(name.clone(), instantiated_args)
            }
            Type::Array(elem_ty) => {
                Type::Array(Box::new(self.instantiate_type(elem_ty)))
            }
            Type::Reference(inner_ty, is_mut) => {
                Type::Reference(Box::new(self.instantiate_type(inner_ty)), *is_mut)
            }
            Type::Tuple(types) => {
                let instantiated_types: Vec<Type> = types
                    .iter()
                    .map(|t| self.instantiate_type(t))
                    .collect();
                Type::Tuple(instantiated_types)
            }
            Type::Function(fn_type) => {
                let instantiated_params: Vec<Type> = fn_type.params
                    .iter()
                    .map(|p| self.instantiate_type(p))
                    .collect();
                let instantiated_return = self.instantiate_type(&fn_type.return_type);
                Type::Function(crate::ast::FunctionType {
                    params: instantiated_params,
                    return_type: Box::new(instantiated_return),
                })
            }
            _ => ty.clone(),
        }
    }
    
    /// 型変数を収集
    #[allow(dead_code)]
    pub fn collect_type_variables(&self, ty: &Type) -> HashSet<String> {
        let mut vars = HashSet::new();
        self.collect_type_variables_impl(ty, &mut vars);
        vars
    }
    
    #[allow(dead_code)]
    fn collect_type_variables_impl(&self, ty: &Type, vars: &mut HashSet<String>) {
        match ty {
            Type::Variable(name) => {
                vars.insert(name.clone());
            }
            Type::Generic(_, args) => {
                for arg in args {
                    self.collect_type_variables_impl(arg, vars);
                }
            }
            Type::Array(elem_ty) => {
                self.collect_type_variables_impl(elem_ty, vars);
            }
            Type::Reference(inner_ty, _) => {
                self.collect_type_variables_impl(inner_ty, vars);
            }
            Type::Tuple(types) => {
                for t in types {
                    self.collect_type_variables_impl(t, vars);
                }
            }
            Type::Function(fn_type) => {
                for param in &fn_type.params {
                    self.collect_type_variables_impl(param, vars);
                }
                self.collect_type_variables_impl(&fn_type.return_type, vars);
            }
            _ => {}
        }
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;
    
    #[test]
    fn test_type_param_registration() {
        let mut env = TypeEnvironment::new();
        
        let params = vec![
            TypeParam { name: "T".to_string(), span: Span::dummy() },
            TypeParam { name: "U".to_string(), span: Span::dummy() },
        ];
        
        assert!(env.register_type_params(&params).is_ok());
        assert!(env.is_type_param("T"));
        assert!(env.is_type_param("U"));
        assert!(!env.is_type_param("V"));
    }
    
    #[test]
    fn test_type_instantiation() {
        let mut env = TypeEnvironment::new();
        
        // T -> i32 のバインディングを設定
        env.bind_type("T".to_string(), Type::I32);
        
        // Variable(T) -> I32
        let type_var = Type::Variable("T".to_string());
        let instantiated = env.instantiate_type(&type_var);
        assert_eq!(instantiated, Type::I32);
        
        // Array<T> -> Array<i32>
        let array_type = Type::Array(Box::new(Type::Variable("T".to_string())));
        let instantiated = env.instantiate_type(&array_type);
        assert_eq!(instantiated, Type::Array(Box::new(Type::I32)));
    }
}