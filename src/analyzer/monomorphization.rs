//! 単相化（Monomorphization）モジュール
//!
//! ジェネリック関数や型を具体的な型で特殊化する
//! 
//! 現在は基本的な構造のみを実装しており、実際の単相化処理は
//! 段階的に実装していく予定。

use std::collections::HashSet;
use crate::ast::{Program, Type};
use crate::error::YuniResult;

/// 単相化された関数の情報
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedFunction {
    /// 元の関数名
    pub original_name: String,
    /// 型引数の具体的な型へのマッピング
    pub type_args: Vec<Type>,
    /// マングルされた名前（例: Vec_i32_new）
    pub mangled_name: String,
}

/// 単相化された構造体の情報
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedStruct {
    /// 元の構造体名
    pub original_name: String,
    /// 型引数の具体的な型へのマッピング
    pub type_args: Vec<Type>,
    /// マングルされた名前（例: Vec_i32）
    pub mangled_name: String,
}

/// 単相化エンジン
pub struct Monomorphizer {
    /// 単相化された関数のセット（重複を避けるため）
    monomorphized_functions: HashSet<(String, Vec<Type>)>,
    /// 単相化された構造体のセット
    monomorphized_structs: HashSet<(String, Vec<Type>)>,
    /// 元のプログラム
    original_program: Program,
}

impl Monomorphizer {
    /// 新しい単相化エンジンを作成
    pub fn new(program: Program) -> Self {
        Self {
            monomorphized_functions: HashSet::new(),
            monomorphized_structs: HashSet::new(),
            original_program: program,
        }
    }
    
    /// プログラムを単相化
    pub fn monomorphize(self) -> YuniResult<Program> {
        // 現在の実装では、ジェネリクスを使用していないプログラムを
        // そのまま返す。将来的にはここで実際の単相化を行う。
        
        // TODO: 以下の処理を実装
        // 1. プログラム全体を走査してジェネリック関数/型の使用を検出
        // 2. 使用されている具体的な型引数を収集
        // 3. 各ジェネリック関数/型に対して具体的なバージョンを生成
        // 4. 呼び出し箇所を単相化されたバージョンに置き換え
        
        Ok(self.original_program)
    }
    
    /// 型を文字列に変換（マングリング用）
    #[allow(dead_code)]
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::UserDefined(name) => name.clone(),
            Type::Array(elem) => format!("array_{}", self.type_to_string(elem)),
            Type::Reference(inner, is_mut) => {
                if *is_mut {
                    format!("mut_ref_{}", self.type_to_string(inner))
                } else {
                    format!("ref_{}", self.type_to_string(inner))
                }
            }
            Type::Generic(name, args) => {
                let arg_strs: Vec<String> = args.iter()
                    .map(|arg| self.type_to_string(arg))
                    .collect();
                format!("{}_{}", name, arg_strs.join("_"))
            }
            _ => "unknown".to_string(),
        }
    }
    
    /// 関数名をマングル
    #[allow(dead_code)]
    fn mangle_function_name(&self, name: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            name.to_string()
        } else {
            let type_names: Vec<String> = type_args.iter()
                .map(|ty| self.type_to_string(ty))
                .collect();
            format!("{}_{}", name, type_names.join("_"))
        }
    }
    
    /// 構造体名をマングル
    #[allow(dead_code)]
    fn mangle_struct_name(&self, name: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            name.to_string()
        } else {
            let type_names: Vec<String> = type_args.iter()
                .map(|ty| self.type_to_string(ty))
                .collect();
            format!("{}_{}", name, type_names.join("_"))
        }
    }
}

/// プログラムを単相化する
pub fn monomorphize_program(program: Program) -> YuniResult<Program> {
    let monomorphizer = Monomorphizer::new(program);
    monomorphizer.monomorphize()
}