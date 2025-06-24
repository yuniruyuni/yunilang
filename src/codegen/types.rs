//! LLVM型変換とType関連のユーティリティ

use crate::ast::Type;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::context::Context;
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, StructType};
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use std::collections::HashMap;

/// 型変換マネージャー
pub struct TypeManager<'ctx> {
    context: &'ctx Context,
    /// 名前付き型のキャッシュ
    types: HashMap<String, StructType<'ctx>>,
    /// Enum型のキャッシュ（Enumはi32として表現）
    enum_types: HashMap<String, BasicTypeEnum<'ctx>>,
    /// 型エイリアス（型名 -> 基底型）
    type_aliases: HashMap<String, Type>,
}

impl<'ctx> TypeManager<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        Self {
            context,
            types: HashMap::new(),
            enum_types: HashMap::new(),
            type_aliases: HashMap::new(),
        }
    }
    
    /// 構造体型を登録
    pub fn register_struct(&mut self, name: String, struct_type: StructType<'ctx>) {
        self.types.insert(name, struct_type);
    }
    
    /// 構造体型を取得
    pub fn get_struct(&self, name: &str) -> Option<StructType<'ctx>> {
        self.types.get(name).copied()
    }
    
    /// Enum型を登録（i32として表現）
    pub fn register_enum(&mut self, name: String, enum_type: inkwell::types::IntType<'ctx>) {
        self.enum_types.insert(name, enum_type.into());
    }
    
    /// 型エイリアスを登録
    pub fn register_type_alias(&mut self, name: String, underlying_type: Type) {
        self.type_aliases.insert(name, underlying_type);
    }
    
    /// AST型からLLVM型への変換
    pub fn ast_type_to_llvm(&self, ty: &Type) -> YuniResult<BasicTypeEnum<'ctx>> {
        match ty {
            Type::I8 => Ok(self.context.i8_type().into()),
            Type::I16 => Ok(self.context.i16_type().into()),
            Type::I32 => Ok(self.context.i32_type().into()),
            Type::I64 => Ok(self.context.i64_type().into()),
            Type::I128 => Ok(self.context.i128_type().into()),
            Type::U8 => Ok(self.context.i8_type().into()),
            Type::U16 => Ok(self.context.i16_type().into()),
            Type::U32 => Ok(self.context.i32_type().into()),
            Type::U64 => Ok(self.context.i64_type().into()),
            Type::U128 => Ok(self.context.i128_type().into()),
            Type::F32 => Ok(self.context.f32_type().into()),
            Type::F64 => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            Type::Str => Ok(self.context.ptr_type(AddressSpace::default()).into()),
            Type::String => Ok(self.context.ptr_type(AddressSpace::default()).into()),
            Type::Array(elem_ty) => {
                let _elem_type = self.ast_type_to_llvm(elem_ty)?;
                Ok(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Tuple(types) => {
                let field_types: Vec<BasicTypeEnum> = types
                    .iter()
                    .map(|t| self.ast_type_to_llvm(t))
                    .collect::<YuniResult<Vec<_>>>()?;
                Ok(self.context.struct_type(&field_types, false).into())
            }
            Type::UserDefined(name) => {
                // まず型エイリアスを解決
                if let Some(underlying_type) = self.type_aliases.get(name) {
                    return self.ast_type_to_llvm(underlying_type);
                }
                
                // 構造体型を探す
                if let Some(struct_type) = self.types.get(name).copied() {
                    Ok(struct_type.into())
                }
                // 次にEnum型を探す
                else if let Some(enum_type) = self.enum_types.get(name).copied() {
                    Ok(enum_type)
                }
                else {
                    Err(YuniError::Codegen(CodegenError::Undefined {
                        name: name.clone(),
                        span: crate::ast::Span::dummy(),
                    }))
                }
            }
            Type::Reference(referent, _is_mut) => {
                let _inner_type = self.ast_type_to_llvm(referent)?;
                Ok(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Function(fn_type) => {
                // 関数ポインタ型として扱う
                let param_types: Vec<BasicMetadataTypeEnum> = fn_type.params
                    .iter()
                    .map(|t| self.ast_type_to_llvm(t).map(|bt| bt.into()))
                    .collect::<YuniResult<Vec<_>>>()?;
                    
                let ret_type = if matches!(fn_type.return_type.as_ref(), Type::Void) {
                    None
                } else {
                    Some(self.ast_type_to_llvm(&fn_type.return_type)?)
                };
                
                let _llvm_fn_type = match ret_type {
                    Some(ret) => ret.fn_type(&param_types, false),
                    None => self.context.void_type().fn_type(&param_types, false),
                };
                
                Ok(self.context.ptr_type(AddressSpace::default()).into())
            }
            Type::Void => Err(YuniError::Codegen(CodegenError::InvalidType {
                message: "Void type cannot be used as a value".to_string(),
                span: crate::ast::Span::dummy(),
            })),
            Type::I256 | Type::U256 | Type::F8 | Type::F16 => {
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: format!("{:?} type not yet implemented", ty),
                    span: crate::ast::Span::dummy(),
                }))
            }
            Type::Variable(_) | Type::Generic(_, _) => {
                // ジェネリック型は具体化されるまでコード生成できない
                Err(YuniError::Codegen(CodegenError::Unimplemented {
                    feature: format!("Generic types must be instantiated before code generation: {:?}", ty),
                    span: crate::ast::Span::dummy(),
                }))
            }
        }
    }
    
    /// AST型からメタデータ型への変換（関数パラメータ用）
    pub fn ast_type_to_metadata(&self, ty: &Type) -> YuniResult<BasicMetadataTypeEnum<'ctx>> {
        Ok(self.ast_type_to_llvm(ty)?.into())
    }
    
    /// 関数型を作成
    pub fn create_function_type(
        &self,
        params: &[Type],
        return_type: &Type,
        is_varargs: bool,
    ) -> YuniResult<FunctionType<'ctx>> {
        let param_types: Vec<BasicMetadataTypeEnum> = params
            .iter()
            .map(|t| self.ast_type_to_metadata(t))
            .collect::<YuniResult<Vec<_>>>()?;
            
        let fn_type = if matches!(return_type, Type::Void) {
            self.context.void_type().fn_type(&param_types, is_varargs)
        } else {
            let ret_type = self.ast_type_to_llvm(return_type)?;
            ret_type.fn_type(&param_types, is_varargs)
        };
        
        Ok(fn_type)
    }
    
    /// 型がプリミティブ型かチェック
    #[allow(dead_code)]
    pub fn is_primitive_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 |
            Type::F8 | Type::F16 | Type::F32 | Type::F64 |
            Type::Bool
        )
    }
    
    /// 型が整数型かチェック
    #[allow(dead_code)]
    pub fn is_integer_type(ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128
        )
    }
    
    /// 型が符号付き整数型かチェック
    #[allow(dead_code)]
    pub fn is_signed_integer(ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128
        )
    }
    
    /// 型が浮動小数点型かチェック
    #[allow(dead_code)]
    pub fn is_float_type(ty: &Type) -> bool {
        matches!(ty, Type::F8 | Type::F16 | Type::F32 | Type::F64)
    }
    
    /// デフォルトのゼロ値を作成
    pub fn create_default_value(&self, ty: &Type) -> YuniResult<BasicValueEnum<'ctx>> {
        match ty {
            Type::I8 | Type::U8 => Ok(self.context.i8_type().const_zero().into()),
            Type::I16 | Type::U16 => Ok(self.context.i16_type().const_zero().into()),
            Type::I32 | Type::U32 => Ok(self.context.i32_type().const_zero().into()),
            Type::I64 | Type::U64 => Ok(self.context.i64_type().const_zero().into()),
            Type::I128 | Type::U128 => Ok(self.context.i128_type().const_zero().into()),
            Type::F32 => Ok(self.context.f32_type().const_zero().into()),
            Type::F64 => Ok(self.context.f64_type().const_zero().into()),
            Type::Bool => Ok(self.context.bool_type().const_zero().into()),
            Type::Str => Ok(self.context.ptr_type(AddressSpace::default()).const_null().into()),
            Type::String => Ok(self.context.ptr_type(AddressSpace::default()).const_null().into()),
            Type::Array(_) | Type::Reference(_, _) => {
                Ok(self.context.ptr_type(AddressSpace::default()).const_null().into())
            }
            Type::UserDefined(_) => {
                let llvm_type = self.ast_type_to_llvm(ty)?;
                match llvm_type {
                    BasicTypeEnum::StructType(st) => Ok(st.const_zero().into()),
                    BasicTypeEnum::IntType(it) => Ok(it.const_zero().into()), // Enum型の場合
                    _ => Err(YuniError::Codegen(CodegenError::Internal {
                        message: format!("Cannot create default value for type {:?}", ty),
                    })),
                }
            }
            Type::Tuple(types) => {
                let values: Vec<BasicValueEnum> = types
                    .iter()
                    .map(|t| self.create_default_value(t))
                    .collect::<YuniResult<Vec<_>>>()?;
                let _field_types: Vec<BasicTypeEnum> = values
                    .iter()
                    .map(|v| v.get_type())
                    .collect();
                Ok(self.context.const_struct(&values, false).into())
            }
            _ => Err(YuniError::Codegen(CodegenError::Internal {
                message: format!("Cannot create default value for type {:?}", ty),
            })),
        }
    }
}