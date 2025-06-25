//! インスタンス化処理

use std::collections::HashMap;
use crate::ast::*;
use crate::error::YuniResult;
use super::{Monomorphizer, InstantiationType};

impl Monomorphizer {
    /// インスタンス化処理
    pub(super) fn process_instantiation(&mut self, name: &str, type_args: &[Type], inst_type: InstantiationType) -> YuniResult<()> {
        match inst_type {
            InstantiationType::Function => {
                if let Some(func) = self.generic_functions.get(name).cloned() {
                    let monomorphized = self.monomorphize_function(func, type_args)?;
                    self.generated_items.push(Item::Function(monomorphized));
                }
            }
            InstantiationType::Struct => {
                if let Some(struct_def) = self.generic_structs.get(name).cloned() {
                    let monomorphized = self.monomorphize_struct(struct_def, type_args)?;
                    self.generated_items.push(Item::TypeDef(TypeDef::Struct(monomorphized)));
                }
            }
            InstantiationType::Enum => {
                if let Some(enum_def) = self.generic_enums.get(name).cloned() {
                    let monomorphized = self.monomorphize_enum(enum_def, type_args)?;
                    self.generated_items.push(Item::TypeDef(TypeDef::Enum(monomorphized)));
                }
            }
        }
        Ok(())
    }
    
    /// 関数を単相化
    fn monomorphize_function(&mut self, mut func: FunctionDecl, type_args: &[Type]) -> YuniResult<FunctionDecl> {
        // 型パラメータマッピングを作成
        let mut type_map = HashMap::new();
        for (i, param) in func.type_params.iter().enumerate() {
            if let Some(ty) = type_args.get(i) {
                type_map.insert(param.name.clone(), ty.clone());
            }
        }
        
        // 関数名をマングル
        func.name = crate::analyzer::monomorphization::mangling::mangle_function_name(&func.name, type_args);
        
        // 型パラメータをクリア
        func.type_params.clear();
        
        // パラメータの型を置換
        for param in &mut func.params {
            param.ty = self.substitute_type(&param.ty, &type_map);
        }
        
        // 戻り値の型を置換
        if let Some(ret_ty) = &mut func.return_type {
            *ret_ty = Box::new(self.substitute_type(ret_ty, &type_map));
        }
        
        // 関数本体を置換
        func.body = self.substitute_block(&func.body, &type_map)?;
        
        Ok(func)
    }
    
    /// 構造体を単相化
    fn monomorphize_struct(&mut self, mut struct_def: StructDef, type_args: &[Type]) -> YuniResult<StructDef> {
        // 型パラメータマッピングを作成
        let mut type_map = HashMap::new();
        for (i, param) in struct_def.type_params.iter().enumerate() {
            if let Some(ty) = type_args.get(i) {
                type_map.insert(param.name.clone(), ty.clone());
            }
        }
        
        // 構造体名をマングル
        struct_def.name = crate::analyzer::monomorphization::mangling::mangle_struct_name(&struct_def.name, type_args);
        
        // 型パラメータをクリア
        struct_def.type_params.clear();
        
        // フィールドの型を置換
        for field in &mut struct_def.fields {
            field.ty = self.substitute_type(&field.ty, &type_map);
        }
        
        Ok(struct_def)
    }
    
    /// 列挙型を単相化
    fn monomorphize_enum(&mut self, mut enum_def: EnumDef, type_args: &[Type]) -> YuniResult<EnumDef> {
        // 型パラメータマッピングを作成
        let mut type_map = HashMap::new();
        for (i, param) in enum_def.type_params.iter().enumerate() {
            if let Some(ty) = type_args.get(i) {
                type_map.insert(param.name.clone(), ty.clone());
            }
        }
        
        // 列挙型名をマングル
        enum_def.name = crate::analyzer::monomorphization::mangling::mangle_struct_name(&enum_def.name, type_args);
        
        // 型パラメータをクリア
        enum_def.type_params.clear();
        
        // バリアントのフィールドの型を置換
        for variant in &mut enum_def.variants {
            for field in &mut variant.fields {
                field.ty = self.substitute_type(&field.ty, &type_map);
            }
        }
        
        Ok(enum_def)
    }
}