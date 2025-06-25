//! 単相化（Monomorphization）モジュール
//!
//! ジェネリック関数や型を具体的な型で特殊化する

use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::error::YuniResult;

// サブモジュール
mod types;
mod collector;
mod type_inference;
mod instantiator;
mod substitution;
mod replacement;
mod mangling;

// 再エクスポート
pub use types::*;


/// 単相化エンジン
pub struct Monomorphizer {
    /// 単相化された関数のセット（重複を避けるため）
    pub(crate) monomorphized_functions: HashSet<(String, Vec<Type>)>,
    /// 単相化された構造体のセット
    pub(crate) monomorphized_structs: HashSet<(String, Vec<Type>)>,
    /// 元のプログラム
    pub(crate) original_program: Program,
    /// ジェネリック関数の定義（名前 -> 関数宣言）
    pub(crate) generic_functions: HashMap<String, FunctionDecl>,
    /// ジェネリック構造体の定義（名前 -> 構造体定義）
    pub(crate) generic_structs: HashMap<String, StructDef>,
    /// ジェネリック列挙型の定義（名前 -> 列挙型定義）
    pub(crate) generic_enums: HashMap<String, EnumDef>,
    /// 処理すべきインスタンス化のキュー
    pub(crate) instantiation_queue: Vec<(String, Vec<Type>, InstantiationType)>,
    /// 生成された単相化アイテム
    pub(crate) generated_items: Vec<Item>,
}

impl Monomorphizer {
    /// 新しい単相化エンジンを作成
    pub fn new(program: Program) -> Self {
        Self {
            monomorphized_functions: HashSet::new(),
            monomorphized_structs: HashSet::new(),
            original_program: program,
            generic_functions: HashMap::new(),
            generic_structs: HashMap::new(),
            generic_enums: HashMap::new(),
            instantiation_queue: Vec::new(),
            generated_items: Vec::new(),
        }
    }
    
    /// プログラムを単相化
    pub fn monomorphize(mut self) -> YuniResult<Program> {
        // ステップ1: ジェネリック定義を収集
        self.collect_generic_definitions();
        
        // ステップ2: 最初のパスでジェネリックの使用箇所を検出
        self.collect_instantiations(&self.original_program.clone())?;
        
        // ステップ3: キューを処理して必要な単相化バージョンを生成
        while let Some((name, type_args, inst_type)) = self.instantiation_queue.pop() {
            self.process_instantiation(&name, &type_args, inst_type)?;
        }
        
        // ステップ4: 単相化されたプログラムを構築
        let mut result_program = self.original_program.clone();
        
        // ジェネリック定義を削除して、単相化されたバージョンを追加
        result_program.items.retain(|item| {
            match item {
                Item::Function(func) => func.type_params.is_empty(),
                Item::TypeDef(TypeDef::Struct(s)) => s.type_params.is_empty(),
                Item::TypeDef(TypeDef::Enum(e)) => e.type_params.is_empty(),
                Item::TypeDef(TypeDef::Alias(a)) => a.type_params.is_empty(),
                _ => true,
            }
        });
        
        // 単相化されたアイテムを追加
        result_program.items.extend(self.generated_items.clone());
        
        // ステップ5: すべてのジェネリック呼び出しを単相化バージョンに置き換え
        self.replace_generic_calls(&mut result_program)?;
        
        Ok(result_program)
    }
}

/// プログラムを単相化するエントリポイント
pub fn monomorphize_program(program: Program) -> YuniResult<Program> {
    let monomorphizer = Monomorphizer::new(program);
    monomorphizer.monomorphize()
}