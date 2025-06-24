//! 型チェック機能

use crate::ast::*;
use std::collections::HashMap;

use super::symbol::{AnalysisError, AnalysisResult, FunctionSignature, TypeInfo, TypeKind};

/// 型チェッカー
pub struct TypeChecker {
    /// 型定義のレジストリ
    types: HashMap<String, TypeInfo>,
    /// 関数シグネチャのレジストリ
    functions: HashMap<String, FunctionSignature>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        let mut checker = Self {
            types: HashMap::new(),
            functions: HashMap::new(),
        };
        
        // ビルトイン型を登録
        checker.register_builtin_types();
        
        // ビルトイン関数を登録
        checker.register_builtin_functions();
        
        checker
    }
}

impl TypeChecker {
    pub fn new() -> Self {
        Self::default()
    }
    
    /// ビルトイン型を登録
    fn register_builtin_types(&mut self) {
        let builtin_types = vec![
            "i8", "i16", "i32", "i64", "i128", "i256", 
            "u8", "u16", "u32", "u64", "u128", "u256",
            "f8", "f16", "f32", "f64", 
            "bool", "str", "String", "void",
        ];

        for type_name in builtin_types {
            self.types.insert(
                type_name.to_string(),
                TypeInfo {
                    name: type_name.to_string(),
                    type_params: Vec::new(),
                    kind: TypeKind::Builtin,
                    methods: HashMap::new(),
                    span: Span::dummy(),
                },
            );
        }
    }
    
    /// ビルトイン関数を登録
    fn register_builtin_functions(&mut self) {
        // println関数
        let println_sig = FunctionSignature {
            name: "println".to_string(),
            type_params: Vec::new(),
            params: vec![("value".to_string(), Type::String)],
            return_type: Type::Void,
            lives_clause: None,
            is_method: false,
            receiver_type: None,
            span: Span::dummy(),
        };
        self.functions.insert("println".to_string(), println_sig);

        // sqrt関数
        let sqrt_sig = FunctionSignature {
            name: "sqrt".to_string(),
            type_params: Vec::new(),
            params: vec![("value".to_string(), Type::F64)],
            return_type: Type::F64,
            lives_clause: None,
            is_method: false,
            receiver_type: None,
            span: Span::dummy(),
        };
        self.functions.insert("sqrt".to_string(), sqrt_sig);
    }
    
    /// 型定義を登録
    pub fn register_type(&mut self, type_info: TypeInfo) -> AnalysisResult<()> {
        if self.types.contains_key(&type_info.name) {
            return Err(AnalysisError::DuplicateType {
                name: type_info.name.clone(),
                span: type_info.span,
            });
        }
        self.types.insert(type_info.name.clone(), type_info);
        Ok(())
    }
    
    /// 関数シグネチャを登録
    pub fn register_function(&mut self, func_sig: FunctionSignature) -> AnalysisResult<()> {
        if self.functions.contains_key(&func_sig.name) {
            return Err(AnalysisError::DuplicateFunction {
                name: func_sig.name.clone(),
                span: func_sig.span,
            });
        }
        self.functions.insert(func_sig.name.clone(), func_sig);
        Ok(())
    }
    
    /// 型の互換性をチェック
    pub fn check_type_compatibility(&self, expected: &Type, actual: &Type, span: Span) -> AnalysisResult<()> {
        // 型エイリアスを解決してから比較
        let resolved_expected = self.resolve_type_alias(expected);
        let resolved_actual = self.resolve_type_alias(actual);
        
        if !self.types_compatible(&resolved_expected, &resolved_actual) {
            return Err(AnalysisError::TypeMismatch {
                expected: self.type_to_string(expected),
                found: self.type_to_string(actual),
                span,
            });
        }
        Ok(())
    }
    
    /// 型が存在するか検証
    pub fn validate_type(&self, ty: &Type, span: Span) -> AnalysisResult<()> {
        match ty {
            Type::UserDefined(name) => {
                if !self.types.contains_key(name) {
                    return Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    });
                }
            }
            Type::Reference(referent, _) => {
                self.validate_type(referent, span)?;
            }
            Type::Array(element) => {
                self.validate_type(element, span)?;
            }
            Type::Tuple(elements) => {
                for elem in elements {
                    self.validate_type(elem, span)?;
                }
            }
            Type::Function(fn_type) => {
                for param in &fn_type.params {
                    self.validate_type(param, span)?;
                }
                self.validate_type(&fn_type.return_type, span)?;
            }
            Type::Variable(_name) => {
                // 型変数は型パラメータとして定義されている必要がある
                // 注: この検証はSemanticAnalyzerのtype_envで行われる
                // ここでは単にOKとする
            }
            Type::Generic(name, args) => {
                // ジェネリック型の基本型が存在するかチェック
                if !self.types.contains_key(name) {
                    return Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    });
                }
                // 型引数もチェック
                for arg in args {
                    self.validate_type(arg, span)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// 型の文字列表現を取得
    #[allow(clippy::only_used_in_recursion)]
    pub fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::I128 => "i128".to_string(),
            Type::I256 => "i256".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::U128 => "u128".to_string(),
            Type::U256 => "u256".to_string(),
            Type::F8 => "f8".to_string(),
            Type::F16 => "f16".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::Str => "str".to_string(),
            Type::String => "String".to_string(),
            Type::Void => "void".to_string(),
            Type::UserDefined(name) => name.clone(),
            Type::Reference(referent, is_mutable) => {
                if *is_mutable {
                    format!("&mut {}", self.type_to_string(referent))
                } else {
                    format!("&{}", self.type_to_string(referent))
                }
            }
            Type::Array(element) => format!("[{}]", self.type_to_string(element)),
            Type::Tuple(elements) => {
                let elems: Vec<String> = elements.iter().map(|e| self.type_to_string(e)).collect();
                format!("({})", elems.join(", "))
            }
            Type::Function(fn_type) => {
                let param_strs: Vec<String> = fn_type.params.iter().map(|p| self.type_to_string(p)).collect();
                format!("fn({}) -> {}", param_strs.join(", "), self.type_to_string(&fn_type.return_type))
            }
            Type::Variable(name) => name.clone(),
            Type::Generic(name, args) => {
                let arg_strs: Vec<String> = args.iter().map(|a| self.type_to_string(a)).collect();
                format!("{}<{}>", name, arg_strs.join(", "))
            }
        }
    }
    
    /// 型が整数型かチェック
    pub fn is_integer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256
        )
    }
    
    /// 型が浮動小数点型かチェック
    pub fn is_float_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::F8 | Type::F16 | Type::F32 | Type::F64)
    }
    
    /// 型が数値型かチェック
    pub fn is_numeric_type(&self, ty: &Type) -> bool {
        self.is_integer_type(ty) || self.is_float_type(ty)
    }
    
    pub fn is_string_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::String | Type::Str)
    }
    
    /// 型エイリアスを解決
    pub fn resolve_type_alias(&self, ty: &Type) -> Type {
        match ty {
            Type::UserDefined(name) => {
                // 型情報を取得
                if let Some(type_info) = self.types.get(name) {
                    if let TypeKind::Alias(underlying) = &type_info.kind {
                        // エイリアスの場合は基底型を再帰的に解決
                        return self.resolve_type_alias(underlying);
                    }
                }
                // エイリアスでない場合はそのまま返す
                ty.clone()
            }
            Type::Reference(inner, is_mut) => {
                Type::Reference(Box::new(self.resolve_type_alias(inner)), *is_mut)
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.resolve_type_alias(elem)))
            }
            Type::Tuple(elems) => {
                Type::Tuple(elems.iter().map(|e| self.resolve_type_alias(e)).collect())
            }
            Type::Generic(name, args) => {
                Type::Generic(
                    name.clone(),
                    args.iter().map(|a| self.resolve_type_alias(a)).collect()
                )
            }
            Type::Function(fn_type) => {
                Type::Function(FunctionType {
                    params: fn_type.params.iter().map(|p| self.resolve_type_alias(p)).collect(),
                    return_type: Box::new(self.resolve_type_alias(&fn_type.return_type)),
                })
            }
            // プリミティブ型はそのまま返す
            _ => ty.clone()
        }
    }
    
    /// フィールドの型を取得
    pub fn get_field_type(&self, struct_type: &Type, field_name: &str, span: Span) -> AnalysisResult<Type> {
        match struct_type {
            Type::UserDefined(name) => {
                if let Some(type_info) = self.types.get(name) {
                    match &type_info.kind {
                        TypeKind::Struct(fields) => {
                            for field in fields {
                                if field.name == field_name {
                                    return Ok(field.ty.clone());
                                }
                            }
                            Err(AnalysisError::UndefinedVariable {
                                name: format!("{}.{}", name, field_name),
                                span,
                            })
                        }
                        _ => Err(AnalysisError::InvalidOperation {
                            message: format!("Type {} is not a struct", name),
                            span,
                        }),
                    }
                } else {
                    Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    })
                }
            }
            Type::Generic(name, type_args) => {
                // ジェネリック型の場合、型パラメータを置換してフィールド型を取得
                if let Some(type_info) = self.types.get(name) {
                    match &type_info.kind {
                        TypeKind::Struct(fields) => {
                            // 型パラメータから型引数へのマッピングを作成
                            let mut substitutions = std::collections::HashMap::new();
                            for (i, type_param) in type_info.type_params.iter().enumerate() {
                                if i < type_args.len() {
                                    substitutions.insert(type_param.name.clone(), type_args[i].clone());
                                }
                            }
                            
                            // フィールドを検索
                            for field in fields {
                                if field.name == field_name {
                                    // フィールドの型に置換を適用
                                    let field_type = self.substitute_type(&field.ty, &substitutions);
                                    return Ok(field_type);
                                }
                            }
                            Err(AnalysisError::UndefinedVariable {
                                name: format!("{}.{}", name, field_name),
                                span,
                            })
                        }
                        _ => Err(AnalysisError::InvalidOperation {
                            message: format!("Type {} is not a struct", name),
                            span,
                        }),
                    }
                } else {
                    Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    })
                }
            }
            Type::Reference(inner, _) => {
                // 参照型の場合、内部の型でフィールドアクセス
                self.get_field_type(inner, field_name, span)
            }
            _ => Err(AnalysisError::InvalidOperation {
                message: "Field access on non-struct type".to_string(),
                span,
            }),
        }
    }
    
    /// 二項演算子の結果型を取得
    pub fn binary_op_result_type(&self, op: &BinaryOp, left: &Type, right: &Type, span: Span) -> AnalysisResult<Type> {
        match op {
            BinaryOp::Add => {
                // 数値の加算
                if self.types_compatible(left, right) && self.is_numeric_type(left) {
                    Ok(left.clone())
                }
                // 文字列の連結
                else if self.is_string_type(left) && self.is_string_type(right) {
                    Ok(Type::String) // 文字列連結の結果は常にString型
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: self.type_to_string(left),
                        found: self.type_to_string(right),
                        span,
                    })
                }
            }
            BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                if self.types_compatible(left, right) && self.is_numeric_type(left) {
                    Ok(left.clone())
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: self.type_to_string(left),
                        found: self.type_to_string(right),
                        span,
                    })
                }
            }
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
                if self.types_compatible(left, right) && self.is_numeric_type(left) {
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "numeric type".to_string(),
                        found: self.type_to_string(left),
                        span,
                    })
                }
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                if self.types_compatible(left, right) {
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: self.type_to_string(left),
                        found: self.type_to_string(right),
                        span,
                    })
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if matches!(left, Type::Bool) && matches!(right, Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: if !matches!(left, Type::Bool) {
                            self.type_to_string(left)
                        } else {
                            self.type_to_string(right)
                        },
                        span,
                    })
                }
            }
            // ビット演算子は現在定義されていない
            _ => {
                if self.is_integer_type(left) && self.is_integer_type(right) {
                    Ok(left.clone())
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "integer type".to_string(),
                        found: if !self.is_integer_type(left) {
                            self.type_to_string(left)
                        } else {
                            self.type_to_string(right)
                        },
                        span,
                    })
                }
            }
        }
    }
    
    /// 単項演算子の結果型を取得
    pub fn unary_op_result_type(&self, op: &UnaryOp, operand: &Type, span: Span) -> AnalysisResult<Type> {
        match op {
            UnaryOp::Not => {
                if matches!(operand, Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: self.type_to_string(operand),
                        span,
                    })
                }
            }
            UnaryOp::Negate => {
                if self.is_numeric_type(operand) {
                    Ok(operand.clone())
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "numeric type".to_string(),
                        found: self.type_to_string(operand),
                        span,
                    })
                }
            }
            // ビット反転演算子は現在定義されていない
            _ => {
                if self.is_integer_type(operand) {
                    Ok(operand.clone())
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "integer type".to_string(),
                        found: self.type_to_string(operand),
                        span,
                    })
                }
            }
        }
    }
    
    /// 型の互換性をチェック（公開メソッド）
    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        self.types_compatible_internal(expected, actual)
    }
    
    /// 型の互換性をチェック（内部実装）
    #[allow(clippy::only_used_in_recursion)]
    fn types_compatible_internal(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            // 同じ型は互換
            (a, b) if a == b => true,
            
            // 文字列型の互換性（Stringとstrは相互変換可能）
            (Type::String, Type::Str) => true,
            (Type::Str, Type::String) => true,
            
            // 参照型の互換性
            (Type::Reference(ref_a, mut_a),
             Type::Reference(ref_b, mut_b)) => {
                // 可変参照から不変参照への変換は可能
                (*mut_a == *mut_b || (!*mut_a && *mut_b)) && self.types_compatible_internal(ref_a, ref_b)
            }
            
            // ジェネリック型の互換性
            (Type::Generic(name_a, args_a), Type::Generic(name_b, args_b)) => {
                name_a == name_b && 
                args_a.len() == args_b.len() &&
                args_a.iter().zip(args_b.iter()).all(|(a, b)| self.types_compatible_internal(a, b))
            }
            
            // 配列型の互換性
            (Type::Array(elem_a), Type::Array(elem_b)) => {
                self.types_compatible_internal(elem_a, elem_b)
            }
            
            // タプル型の互換性
            (Type::Tuple(types_a), Type::Tuple(types_b)) => {
                types_a.len() == types_b.len() &&
                types_a.iter().zip(types_b.iter()).all(|(a, b)| self.types_compatible_internal(a, b))
            }
            
            // その他は非互換
            _ => false,
        }
    }
    
    /// 関数シグネチャを取得
    pub fn get_function_signature(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.get(name)
    }
    
    /// 型情報を取得
    pub fn get_type_info(&self, name: &str) -> Option<&TypeInfo> {
        self.types.get(name)
    }
    
    /// メソッドを型に登録
    pub fn register_method(&mut self, type_name: &str, method_sig: FunctionSignature) -> AnalysisResult<()> {
        // 型が存在するか確認
        if let Some(type_info) = self.types.get_mut(type_name) {
            // メソッド名の重複をチェック
            if type_info.methods.contains_key(&method_sig.name) {
                return Err(AnalysisError::DuplicateFunction {
                    name: format!("{}::{}", type_name, method_sig.name),
                    span: method_sig.span,
                });
            }
            
            // メソッドを登録
            type_info.methods.insert(method_sig.name.clone(), method_sig);
            Ok(())
        } else {
            Err(AnalysisError::UndefinedType {
                name: type_name.to_string(),
                span: method_sig.span,
            })
        }
    }
    
    /// メソッドシグネチャを取得
    #[allow(dead_code)]
    pub fn get_method_signature(&self, type_name: &str, method_name: &str) -> Option<&FunctionSignature> {
        self.types.get(type_name)
            .and_then(|type_info| type_info.methods.get(method_name))
    }
    
    /// 型に型パラメータの置換を適用
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type(&self, ty: &Type, substitutions: &std::collections::HashMap<String, Type>) -> Type {
        match ty {
            Type::Variable(name) => {
                substitutions.get(name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::Generic(name, args) => {
                let substituted_args: Vec<Type> = args
                    .iter()
                    .map(|arg| self.substitute_type(arg, substitutions))
                    .collect();
                Type::Generic(name.clone(), substituted_args)
            }
            Type::Array(elem_ty) => {
                Type::Array(Box::new(self.substitute_type(elem_ty, substitutions)))
            }
            Type::Reference(inner_ty, is_mut) => {
                Type::Reference(Box::new(self.substitute_type(inner_ty, substitutions)), *is_mut)
            }
            Type::Tuple(types) => {
                let substituted_types: Vec<Type> = types
                    .iter()
                    .map(|t| self.substitute_type(t, substitutions))
                    .collect();
                Type::Tuple(substituted_types)
            }
            Type::Function(fn_type) => {
                let substituted_params: Vec<Type> = fn_type.params
                    .iter()
                    .map(|p| self.substitute_type(p, substitutions))
                    .collect();
                let substituted_return = self.substitute_type(&fn_type.return_type, substitutions);
                Type::Function(FunctionType {
                    params: substituted_params,
                    return_type: Box::new(substituted_return),
                })
            }
            _ => ty.clone(),
        }
    }
}