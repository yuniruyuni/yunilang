//! 型チェッカー

use std::collections::HashMap;
use crate::ast::*;
use crate::analyzer::symbol::{TypeInfo, TypeKind, FunctionSignature};

// サブモジュール
mod type_operations;
mod type_registry;
mod type_compatibility;
mod type_utilities;

/// 型チェッカー
pub struct TypeChecker {
    /// 登録された型
    pub types: HashMap<String, TypeInfo>,
    /// 登録された関数
    pub functions: HashMap<String, FunctionSignature>,
    /// 登録されたメソッド（型名 -> メソッド名 -> シグネチャ）
    pub methods: HashMap<String, HashMap<String, FunctionSignature>>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            types: HashMap::new(),
            functions: HashMap::new(),
            methods: HashMap::new(),
        };
        
        // 組み込み型を登録
        checker.register_builtin_types();
        
        checker
    }

    /// 組み込み型を登録
    fn register_builtin_types(&mut self) {
        // Vec型を登録
        self.types.insert("Vec".to_string(), TypeInfo {
            name: "Vec".to_string(),
            type_params: vec![TypeParam { name: "T".to_string(), span: Span::dummy() }],
            kind: TypeKind::Builtin,
            methods: HashMap::new(),
            span: Span::dummy(),
        });
        
        // HashMap型を登録
        self.types.insert("HashMap".to_string(), TypeInfo {
            name: "HashMap".to_string(),
            type_params: vec![TypeParam { name: "K".to_string(), span: Span::dummy() }, TypeParam { name: "V".to_string(), span: Span::dummy() }],
            kind: TypeKind::Builtin,
            methods: HashMap::new(),
            span: Span::dummy(),
        });
        
        // Option型を登録
        self.types.insert("Option".to_string(), TypeInfo {
            name: "Option".to_string(),
            type_params: vec![TypeParam { name: "T".to_string(), span: Span::dummy() }],
            kind: TypeKind::Enum(vec![
                Variant {
                    name: "None".to_string(),
                    fields: vec![],
                    span: Span::dummy(),
                },
                Variant {
                    name: "Some".to_string(),
                    fields: vec![Field {
                        name: "0".to_string(),
                        ty: Type::Variable("T".to_string()),
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                },
            ]),
            methods: HashMap::new(),
            span: Span::dummy(),
        });
        
        // Result型を登録
        self.types.insert("Result".to_string(), TypeInfo {
            name: "Result".to_string(),
            type_params: vec![TypeParam { name: "T".to_string(), span: Span::dummy() }, TypeParam { name: "E".to_string(), span: Span::dummy() }],
            kind: TypeKind::Enum(vec![
                Variant {
                    name: "Ok".to_string(),
                    fields: vec![Field {
                        name: "0".to_string(),
                        ty: Type::Variable("T".to_string()),
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                },
                Variant {
                    name: "Err".to_string(),
                    fields: vec![Field {
                        name: "0".to_string(),
                        ty: Type::Variable("E".to_string()),
                        span: Span::dummy(),
                    }],
                    span: Span::dummy(),
                },
            ]),
            methods: HashMap::new(),
            span: Span::dummy(),
        });
    }
}