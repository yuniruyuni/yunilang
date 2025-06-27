//! コレクション型のセマンティック解析

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisResult, AnalysisError};
use crate::analyzer::semantic_analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// リストリテラルの解析
    pub(crate) fn analyze_list_literal(&mut self, list: &ListLiteral, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &list.type_name {
            if type_name != "Vec" {
                return Err(AnalysisError::UndefinedType {
                    name: type_name.clone(),
                    span: list.span,
                });
            }
            
            if type_args.len() != 1 {
                return Err(AnalysisError::InvalidType {
                    message: format!("Vec requires exactly one type argument, found {}", type_args.len()),
                    span: list.span,
                });
            }
            
            let element_type = &type_args[0];
            
            // 各要素の型をチェック
            for elem in &list.elements {
                let elem_type = self.analyze_expression(elem)?;
                self.type_checker.check_type_compatibility(&elem_type, element_type, elem.span())?;
            }
            
            Ok(Type::Generic("Vec".to_string(), type_args.clone()))
        } else {
            // 型推論
            if list.elements.is_empty() {
                // 期待される型から推論
                if let Some(Type::Generic(name, args)) = expected_type {
                    if name == "Vec" && args.len() == 1 {
                        return Ok(Type::Generic(name.clone(), args.clone()));
                    }
                }
                
                return Err(AnalysisError::TypeInference {
                    message: "Cannot infer type for empty list literal without type annotation".to_string(),
                    span: list.span,
                });
            }
            
            // 最初の要素から型を推論
            let first_elem_type = self.analyze_expression(&list.elements[0])?;
            
            // 残りの要素が同じ型であることを確認
            for elem in &list.elements[1..] {
                let elem_type = self.analyze_expression(elem)?;
                self.type_checker.check_type_compatibility(&elem_type, &first_elem_type, elem.span())?;
            }
            
            Ok(Type::Generic("Vec".to_string(), vec![first_elem_type]))
        }
    }
    
    /// マップリテラルの解析
    pub(crate) fn analyze_map_literal(&mut self, map: &MapLiteral, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        // 型名が指定されている場合
        if let Some((type_name, type_args)) = &map.type_name {
            if type_name != "HashMap" {
                return Err(AnalysisError::UndefinedType {
                    name: type_name.clone(),
                    span: map.span,
                });
            }
            
            if type_args.len() != 2 {
                return Err(AnalysisError::InvalidType {
                    message: format!("HashMap requires exactly two type arguments, found {}", type_args.len()),
                    span: map.span,
                });
            }
            
            let key_type = &type_args[0];
            let value_type = &type_args[1];
            
            // 各ペアの型をチェック
            for (key_expr, value_expr) in &map.pairs {
                let k_type = self.analyze_expression(key_expr)?;
                let v_type = self.analyze_expression(value_expr)?;
                
                self.type_checker.check_type_compatibility(&k_type, key_type, key_expr.span())?;
                self.type_checker.check_type_compatibility(&v_type, value_type, value_expr.span())?;
            }
            
            Ok(Type::Generic("HashMap".to_string(), type_args.clone()))
        } else {
            // 型推論
            if map.pairs.is_empty() {
                // 期待される型から推論
                if let Some(Type::Generic(name, args)) = expected_type {
                    if name == "HashMap" && args.len() == 2 {
                        return Ok(Type::Generic(name.clone(), args.clone()));
                    }
                }
                
                return Err(AnalysisError::TypeInference {
                    message: "Cannot infer type for empty map literal without type annotation".to_string(),
                    span: map.span,
                });
            }
            
            // 最初のペアから型を推論
            let (first_key, first_value) = &map.pairs[0];
            let key_type = self.analyze_expression(first_key)?;
            let value_type = self.analyze_expression(first_value)?;
            
            // 残りのペアが同じ型であることを確認
            for (key_expr, value_expr) in &map.pairs[1..] {
                let k_type = self.analyze_expression(key_expr)?;
                let v_type = self.analyze_expression(value_expr)?;
                
                self.type_checker.check_type_compatibility(&k_type, &key_type, key_expr.span())?;
                self.type_checker.check_type_compatibility(&v_type, &value_type, value_expr.span())?;
            }
            
            Ok(Type::Generic("HashMap".to_string(), vec![key_type, value_type]))
        }
    }

    /// 配列式の解析
    pub fn analyze_array_expression(&mut self, array: &ArrayExpr) -> AnalysisResult<Type> {
        if array.elements.is_empty() {
            return Err(AnalysisError::InvalidType {
                message: "Cannot infer type for empty array".to_string(),
                span: array.span,
            });
        }
        
        // 最初の要素の型を取得
        let first_type = self.analyze_expression(&array.elements[0])?;
        
        // すべての要素が同じ型であることを確認
        for (i, elem) in array.elements[1..].iter().enumerate() {
            let elem_type = self.analyze_expression(elem)?;
            if elem_type != first_type {
                return Err(AnalysisError::TypeMismatch {
                    expected: format!("{:?}", first_type),
                    found: format!("{:?}", elem_type),
                    span: elem.span(),
                });
            }
        }
        
        Ok(Type::Array(Box::new(first_type)))
    }

    /// タプル式の解析
    pub(crate) fn analyze_tuple_expression(&mut self, tuple_expr: &TupleExpr) -> AnalysisResult<Type> {
        let mut element_types = Vec::new();
        
        for element in &tuple_expr.elements {
            element_types.push(self.analyze_expression(element)?);
        }
        
        Ok(Type::Tuple(element_types))
    }
}