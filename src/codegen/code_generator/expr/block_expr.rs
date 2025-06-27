//! ブロック式のコード生成

use crate::ast::*;
use crate::error::YuniResult;
use inkwell::values::BasicValueEnum;

use crate::codegen::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// ブロック式をコンパイル
    pub fn compile_block_expr(&mut self, block_expr: &BlockExpr) -> YuniResult<BasicValueEnum<'ctx>> {
        // 新しいスコープを作成
        self.scope_manager.push_scope();
        
        let mut last_value: BasicValueEnum = self.context.i32_type().const_zero().into();
        
        // ブロック内の文を順次コンパイル
        for stmt in &block_expr.statements {
            self.compile_statement(stmt)?;
        }
        
        // 最後の式がある場合はその値を返す
        if let Some(last_expr) = &block_expr.last_expr {
            last_value = self.compile_expression(last_expr)?;
        }
        
        // スコープを終了
        self.scope_manager.pop_scope();
        
        Ok(last_value)
    }
}