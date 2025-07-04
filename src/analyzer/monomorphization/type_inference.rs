//! 型推論ヘルパー関数

use std::collections::HashMap;
use crate::ast::*;
use crate::error::YuniResult;
use super::Monomorphizer;

impl Monomorphizer {
    /// 関数呼び出しから型引数を推論（簡略版）
    pub(super) fn infer_type_args_from_call(&self, func_name: &str, args: &[Expression]) -> YuniResult<Vec<Type>> {
        // TODO: 実際の型推論実装
        // 現在は単純に引数の型から推論
        
        // ジェネリック関数の定義を取得
        if let Some(generic_func) = self.generic_functions.get(func_name) {
            let mut type_args = Vec::new();
            let mut type_param_map: HashMap<String, Type> = HashMap::new();
            
            // 各引数から型パラメータを推論
            for (i, arg) in args.iter().enumerate() {
                if i < generic_func.params.len() {
                    let param_type = &generic_func.params[i].ty;
                    if let Some(arg_type) = self.infer_expr_type(arg) {
                        self.unify_types(param_type, &arg_type, &mut type_param_map);
                    }
                }
            }
            
            // 型パラメータの順番で型引数を収集
            for type_param in &generic_func.type_params {
                if let Some(ty) = type_param_map.get(&type_param.name) {
                    type_args.push(ty.clone());
                } else {
                    // 推論できない場合はデフォルトでi32
                    type_args.push(Type::I32);
                }
            }
            
            Ok(type_args)
        } else {
            // ジェネリック関数でない場合は空のベクタを返す
            Ok(vec![])
        }
    }
    
    /// 型を統一（簡易版）
    #[allow(clippy::only_used_in_recursion)]
    pub(super) fn unify_types(&self, param_type: &Type, arg_type: &Type, type_map: &mut HashMap<String, Type>) {
        match (param_type, arg_type) {
            (Type::Variable(name), _) => {
                // 型変数の場合は型を記録
                type_map.insert(name.clone(), arg_type.clone());
            }
            _ => {
                // その他の場合は何もしない
            }
        }
    }
    
    /// 構造体リテラルから型引数を推論
    pub(super) fn infer_type_args_from_struct_lit(&self, struct_lit: &StructLiteral) -> YuniResult<Vec<Type>> {
        // TODO: 実際の型推論実装
        // 現在はフィールドの値から簡易的に推論
        let mut type_args = Vec::new();
        
        for field in &struct_lit.fields {
            if let Some(ty) = self.infer_expr_type(&field.value) {
                // 重複を避ける
                if !type_args.contains(&ty) {
                    type_args.push(ty);
                }
            }
        }
        
        Ok(type_args)
    }
    
    /// 式の型を推論（簡略版）
    pub(super) fn infer_expr_type(&self, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::Integer(int_lit) => {
                match int_lit.suffix.as_deref() {
                    Some("i8") => Some(Type::I8),
                    Some("i16") => Some(Type::I16),
                    Some("i64") => Some(Type::I64),
                    Some("u8") => Some(Type::U8),
                    Some("u16") => Some(Type::U16),
                    Some("u32") => Some(Type::U32),
                    Some("u64") => Some(Type::U64),
                    _ => Some(Type::I32), // デフォルト
                }
            }
            Expression::Float(float_lit) => {
                match float_lit.suffix.as_deref() {
                    Some("f32") => Some(Type::F32),
                    _ => Some(Type::F64), // デフォルト
                }
            }
            Expression::String(_) => Some(Type::String),
            Expression::Boolean(_) => Some(Type::Bool),
            Expression::StructLit(struct_lit) => {
                // 構造体リテラルの型を推論
                struct_lit.name.as_ref().map(|name| Type::UserDefined(name.clone()))
            }
            Expression::Field(field_expr) => {
                // フィールドアクセスの型推論
                if let Some(struct_type) = self.infer_expr_type(&field_expr.object) {
                    // 構造体定義からフィールドの型を取得
                    match &struct_type {
                        Type::UserDefined(_struct_name) => {
                            // TODO: 実際の構造体定義からフィールド型を取得
                            None
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            }
            Expression::Array(array_expr) => {
                // 配列の要素型を推論
                array_expr.elements.first()
                    .and_then(|first_elem| self.infer_expr_type(first_elem))
                    .map(|elem_type| Type::Array(Box::new(elem_type)))
            }
            Expression::ListLiteral(list) => {
                // リストリテラルの型を推論
                if let Some((name, type_args)) = &list.type_name {
                    Some(Type::Generic(name.clone(), type_args.clone()))
                } else if let Some(first_elem) = list.elements.first() {
                    self.infer_expr_type(first_elem)
                        .map(|elem_type| Type::Generic("Vec".to_string(), vec![elem_type]))
                } else {
                    None
                }
            }
            Expression::MapLiteral(map) => {
                // マップリテラルの型を推論
                if let Some((name, type_args)) = &map.type_name {
                    Some(Type::Generic(name.clone(), type_args.clone()))
                } else if let Some((key, value)) = map.pairs.first() {
                    let key_type = self.infer_expr_type(key)?;
                    let value_type = self.infer_expr_type(value)?;
                    Some(Type::Generic("HashMap".to_string(), vec![key_type, value_type]))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}