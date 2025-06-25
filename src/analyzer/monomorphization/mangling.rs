//! 名前マングリング処理

use crate::ast::Type;

/// 型を文字列に変換（マングリング用）
pub fn type_to_string(ty: &Type) -> String {
    match ty {
        // 基本型
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
        Type::String => "string".to_string(),
        Type::Void => "void".to_string(),
        
        // ユーザー定義型
        Type::UserDefined(name) => name.clone(),
        
        // 配列型
        Type::Array(element) => {
            format!("array_{}", type_to_string(element))
        }
        
        // タプル型
        Type::Tuple(types) => {
            let type_strs: Vec<String> = types.iter()
                .map(type_to_string)
                .collect();
            format!("tuple_{}", type_strs.join("_"))
        }
        
        // 参照型
        Type::Reference(inner, is_mut) => {
            if *is_mut {
                format!("ref_mut_{}", type_to_string(inner))
            } else {
                format!("ref_{}", type_to_string(inner))
            }
        }
        
        // 関数型
        Type::Function(func_type) => {
            let param_strs: Vec<String> = func_type.params.iter()
                .map(type_to_string)
                .collect();
            let ret_str = type_to_string(&func_type.return_type);
            format!("fn_{}_{}", param_strs.join("_"), ret_str)
        }
        
        // 型変数
        Type::Variable(name) => format!("var_{}", name),
        
        // ジェネリック型
        Type::Generic(name, args) => {
            let arg_strs: Vec<String> = args.iter()
                .map(type_to_string)
                .collect();
            format!("{}_{}", name, arg_strs.join("_"))
        }
    }
}

/// 関数名をマングル
pub fn mangle_function_name(name: &str, type_args: &[Type]) -> String {
    if type_args.is_empty() {
        name.to_string()
    } else {
        let type_strs: Vec<String> = type_args.iter()
            .map(type_to_string)
            .collect();
        format!("{}_{}", name, type_strs.join("_"))
    }
}

/// 構造体名をマングル
pub fn mangle_struct_name(name: &str, type_args: &[Type]) -> String {
    if type_args.is_empty() {
        name.to_string()
    } else {
        let type_strs: Vec<String> = type_args.iter()
            .map(type_to_string)
            .collect();
        format!("{}_{}", name, type_strs.join("_"))
    }
}