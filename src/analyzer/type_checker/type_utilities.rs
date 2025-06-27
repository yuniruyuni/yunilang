//! 型に関するユーティリティ関数

use crate::ast::*;
use crate::analyzer::type_checker::TypeChecker;

impl TypeChecker {
    /// 整数型かどうか判定
    pub fn is_integer_type(&self, ty: &Type) -> bool {
        matches!(ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256
        )
    }

    /// 浮動小数点型かどうか判定
    pub fn is_float_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::F32 | Type::F64)
    }

    /// 数値型かどうか判定
    pub fn is_numeric_type(&self, ty: &Type) -> bool {
        self.is_integer_type(ty) || self.is_float_type(ty)
    }

    /// 文字列型かどうか判定
    pub fn is_string_type(&self, ty: &Type) -> bool {
        matches!(ty, Type::String)
    }
}