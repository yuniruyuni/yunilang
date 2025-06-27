//! 型に関するユーティリティ関数

use crate::ast::*;
use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 整数型のビット幅が符号付きかどうかを判定
    /// ビット幅から型を推測して判定する（非推奨）
    pub fn is_signed_type(&self, bit_width: u32) -> bool {
        // ビット幅だけでは正確な判定ができないため、デフォルトで符号付きとする
        // より正確な判定にはType enumを使用すること
        match bit_width {
            8 | 16 | 32 | 64 | 128 => true, // デフォルトで符号付き
            _ => true,
        }
    }
    
    /// 型が符号付き整数かどうかを判定（推奨）
    pub fn is_signed_integer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256
        )
    }
    
    /// 型が符号なし整数かどうかを判定
    #[allow(dead_code)]
    pub fn is_unsigned_integer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256
        )
    }
}