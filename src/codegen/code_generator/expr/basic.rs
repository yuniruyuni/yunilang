//! 識別子とパス式のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 識別子をコンパイル
    pub fn compile_identifier(&mut self, id: &Identifier) -> YuniResult<BasicValueEnum<'ctx>> {
        let symbol = self.scope_manager.lookup(&id.name)
            .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                name: id.name.clone(),
                span: id.span,
            }))?;

        let value = self.builder.build_load(
            self.type_manager.ast_type_to_llvm(&symbol.ty)?,
            symbol.ptr,
            &id.name,
        )?;

        Ok(value)
    }

    /// パス式をコンパイル
    pub fn compile_path(&mut self, path: &PathExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        if path.segments.len() == 1 {
            let name = &path.segments[0];
            
            // 関数を探す
            if let Some(func) = self.functions.get(name) {
                return Ok(func.as_global_value().as_pointer_value().into());
            }
            
            // 変数として扱う
            return self.compile_identifier(&Identifier {
                name: name.clone(),
                span: path.span,
            });
        } else if path.segments.len() == 2 {
            // Enum::Variant のパターンを処理
            // これはEnumVariantExprとして処理されるべきだが、
            // 現在は単純にその形式でコンパイル
            let enum_variant = EnumVariantExpr {
                enum_name: path.segments[0].clone(),
                variant: path.segments[1].clone(),
                fields: crate::ast::EnumVariantFields::Unit,
                span: path.span,
            };
            return self.compile_enum_variant(&enum_variant);
        }
        
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Multi-segment paths not yet implemented".to_string(),
            span: path.span,
        }))
    }
}