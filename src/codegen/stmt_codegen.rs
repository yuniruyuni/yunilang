//! 文のコード生成

use crate::ast::*;
use crate::error::{CodegenError, YuniError, YuniResult};
use inkwell::values::BasicValueEnum;

use super::codegen::CodeGenerator;
use super::symbol_table::Symbol;

impl<'ctx> CodeGenerator<'ctx> {
    /// ブロックをコンパイル
    pub fn compile_block(&mut self, block: &Block) -> YuniResult<()> {
        self.scope_manager.push_scope();

        for stmt in &block.statements {
            self.compile_statement(stmt)?;

            // ターミネータに到達したら停止
            if self.current_block_has_terminator() {
                break;
            }
        }

        self.scope_manager.pop_scope();
        Ok(())
    }

    /// 文をコンパイル
    pub fn compile_statement(&mut self, stmt: &Statement) -> YuniResult<()> {
        match stmt {
            Statement::Let(let_stmt) => self.compile_let_statement(let_stmt),
            Statement::Assignment(assign) => self.compile_assignment(assign),
            Statement::Expression(expr) => {
                self.compile_expression(expr)?;
                Ok(())
            }
            Statement::Return(ret) => self.compile_return(ret),
            Statement::If(if_stmt) => self.compile_if_statement(if_stmt),
            Statement::While(while_stmt) => self.compile_while_statement(while_stmt),
            Statement::For(for_stmt) => self.compile_for_statement(for_stmt),
            Statement::Block(block) => self.compile_block(block),
        }
    }

    /// let文をコンパイル
    pub fn compile_let_statement(&mut self, let_stmt: &LetStatement) -> YuniResult<()> {
        match &let_stmt.pattern {
            Pattern::Identifier(name, is_mut) => {
                let ty = if let Some(ty) = &let_stmt.ty {
                    ty.clone()
                } else if let Some(init) = &let_stmt.init {
                    // 型推論
                    self.infer_type(init)?
                } else {
                    return Err(YuniError::Codegen(CodegenError::Internal {
                        message: format!("Cannot infer type for variable {} without initializer", name)
                    }));
                };

                let alloca = self.create_entry_block_alloca(name, &ty)?;

                if let Some(init) = &let_stmt.init {
                    let value = self.compile_expression(init)?;
                    self.builder.build_store(alloca, value)?;
                }

                self.add_variable(name, alloca, ty, *is_mut)?;
            }
            Pattern::Tuple(_patterns) => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented { 
                    feature: "Tuple patterns not yet implemented".to_string(), 
                    span: Span::dummy() 
                }));
            }
            Pattern::Struct(_name, _fields) => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented { 
                    feature: "Struct patterns not yet implemented".to_string(), 
                    span: Span::dummy() 
                }));
            }
            Pattern::EnumVariant { .. } => {
                return Err(YuniError::Codegen(CodegenError::Unimplemented { 
                    feature: "Enum variant patterns not yet implemented".to_string(), 
                    span: Span::dummy() 
                }));
            }
        }

        Ok(())
    }

    /// 代入文をコンパイル
    pub fn compile_assignment(&mut self, assign: &AssignStatement) -> YuniResult<()> {
        let value = self.compile_expression(&assign.value)?;

        match &assign.target {
            Expression::Identifier(id) => {
                let symbol = self.scope_manager.lookup(&id.name)
                    .ok_or_else(|| YuniError::Codegen(CodegenError::Undefined {
                        name: id.name.clone(),
                        span: id.span,
                    }))?;
                    
                if !symbol.is_mutable {
                    return Err(YuniError::Codegen(CodegenError::Internal {
                        message: format!("Cannot assign to immutable variable {}", id.name)
                    }));
                }
                self.builder.build_store(symbol.ptr, value)?;
            }
            Expression::Field(field_expr) => {
                self.compile_field_assignment(field_expr, value)?;
            }
            Expression::Index(index_expr) => {
                self.compile_index_assignment(index_expr, value)?;
            }
            Expression::Dereference(deref_expr) => {
                self.compile_deref_assignment(deref_expr, value)?;
            }
            _ => {
                return Err(YuniError::Codegen(CodegenError::InvalidType {
                    message: "Invalid assignment target".to_string(),
                    span: assign.span,
                }));
            }
        }

        Ok(())
    }

    /// return文をコンパイル
    pub fn compile_return(&mut self, ret: &ReturnStatement) -> YuniResult<()> {
        if let Some(value) = &ret.value {
            let return_value = self.compile_expression(value)?;
            self.builder.build_return(Some(&return_value))?;
        } else {
            self.builder.build_return(None)?;
        }
        Ok(())
    }

    /// if文をコンパイル
    pub fn compile_if_statement(&mut self, if_stmt: &IfStatement) -> YuniResult<()> {
        let condition = self.compile_expression(&if_stmt.condition)?;

        let function = self.current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: "No current function".to_string() 
            }))?;

        let then_block = self.context.append_basic_block(function, "if.then");
        let else_block = self.context.append_basic_block(function, "if.else");
        let merge_block = self.context.append_basic_block(function, "if.merge");

        match condition {
            BasicValueEnum::IntValue(int_val) => {
                self.builder.build_conditional_branch(int_val, then_block, else_block)?;
            }
            _ => return Err(YuniError::Codegen(CodegenError::TypeError {
                expected: "bool".to_string(),
                actual: "non-bool".to_string(),
                span: if_stmt.span,
            })),
        }

        // Then ブランチをコンパイル
        self.builder.position_at_end(then_block);
        self.compile_block(&if_stmt.then_branch)?;
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(merge_block)?;
        }

        // Else ブランチをコンパイル
        self.builder.position_at_end(else_block);
        if let Some(else_branch) = &if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => self.compile_block(block)?,
                ElseBranch::If(nested_if) => self.compile_if_statement(nested_if)?,
            }
        }
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(merge_block)?;
        }

        // Merge ブロックで継続
        self.builder.position_at_end(merge_block);

        Ok(())
    }

    /// while文をコンパイル
    pub fn compile_while_statement(&mut self, while_stmt: &WhileStatement) -> YuniResult<()> {
        let function = self.current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: "No current function".to_string() 
            }))?;

        let cond_block = self.context.append_basic_block(function, "while.cond");
        let body_block = self.context.append_basic_block(function, "while.body");
        let exit_block = self.context.append_basic_block(function, "while.exit");

        // 条件ブロックへジャンプ
        self.builder.build_unconditional_branch(cond_block)?;

        // 条件をコンパイル
        self.builder.position_at_end(cond_block);
        let condition = self.compile_expression(&while_stmt.condition)?;

        match condition {
            BasicValueEnum::IntValue(int_val) => {
                self.builder.build_conditional_branch(int_val, body_block, exit_block)?;
            }
            _ => return Err(YuniError::Codegen(CodegenError::TypeError {
                expected: "bool".to_string(),
                actual: "non-bool".to_string(),
                span: while_stmt.span,
            })),
        }

        // ボディをコンパイル
        self.builder.position_at_end(body_block);
        self.compile_block(&while_stmt.body)?;
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(cond_block)?;
        }

        // 終了ブロックで継続
        self.builder.position_at_end(exit_block);

        Ok(())
    }

    /// for文をコンパイル
    pub fn compile_for_statement(&mut self, for_stmt: &ForStatement) -> YuniResult<()> {
        // ループ変数用の新しいスコープを作成
        self.scope_manager.push_scope();

        // 初期化をコンパイル
        if let Some(init) = &for_stmt.init {
            self.compile_statement(init)?;
        }

        let function = self.current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal { 
                message: "No current function".to_string() 
            }))?;

        let cond_block = self.context.append_basic_block(function, "for.cond");
        let body_block = self.context.append_basic_block(function, "for.body");
        let update_block = self.context.append_basic_block(function, "for.update");
        let exit_block = self.context.append_basic_block(function, "for.exit");

        // 条件ブロックへジャンプ
        self.builder.build_unconditional_branch(cond_block)?;

        // 条件をコンパイル
        self.builder.position_at_end(cond_block);
        if let Some(condition) = &for_stmt.condition {
            let cond_value = self.compile_expression(condition)?;
            match cond_value {
                BasicValueEnum::IntValue(int_val) => {
                    self.builder.build_conditional_branch(int_val, body_block, exit_block)?;
                }
                _ => return Err(YuniError::Codegen(CodegenError::TypeError {
                    expected: "bool".to_string(),
                    actual: "non-bool".to_string(),
                    span: for_stmt.span,
                })),
            }
        } else {
            // 条件がない場合は無限ループ
            self.builder.build_unconditional_branch(body_block)?;
        }

        // ボディをコンパイル
        self.builder.position_at_end(body_block);
        self.compile_block(&for_stmt.body)?;
        if !self.current_block_has_terminator() {
            self.builder.build_unconditional_branch(update_block)?;
        }

        // 更新をコンパイル
        self.builder.position_at_end(update_block);
        if let Some(update) = &for_stmt.update {
            self.compile_expression(update)?;
        }
        self.builder.build_unconditional_branch(cond_block)?;

        // 終了ブロックで継続
        self.builder.position_at_end(exit_block);

        // ループスコープを終了
        self.scope_manager.pop_scope();

        Ok(())
    }

    /// 現在のブロックがターミネータを持っているかチェック
    pub fn current_block_has_terminator(&self) -> bool {
        let current_block = self.builder.get_insert_block().unwrap();
        current_block.get_terminator().is_some()
    }

    /// 式から型を推論
    pub fn infer_type(&mut self, expr: &Expression) -> YuniResult<Type> {
        self.expression_type(expr)
    }

    /// エントリブロックにallocaを作成
    pub fn create_entry_block_alloca(&self, name: &str, ty: &Type) -> YuniResult<inkwell::values::PointerValue<'ctx>> {
        let builder = self.context.create_builder();
        let function = self.current_function
            .ok_or_else(|| YuniError::Codegen(CodegenError::Internal {
                message: "No current function".to_string()
            }))?;
        
        let entry = function.get_first_basic_block().unwrap();
        match entry.get_first_instruction() {
            Some(first_instr) => builder.position_before(&first_instr),
            None => builder.position_at_end(entry),
        }

        let llvm_type = self.type_manager.ast_type_to_llvm(ty)?;
        Ok(builder.build_alloca(llvm_type, name)?)
    }

    /// 変数をスコープに追加
    pub fn add_variable(&mut self, name: &str, ptr: inkwell::values::PointerValue<'ctx>, ty: Type, is_mutable: bool) -> YuniResult<()> {
        let symbol = Symbol {
            ptr,
            ty,
            is_mutable,
        };
        self.scope_manager.define(name.to_string(), symbol);
        Ok(())
    }

    /// フィールド代入をコンパイル
    pub fn compile_field_assignment(&mut self, _field_expr: &FieldExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Field assignment not yet implemented".to_string(),
            span: Span::dummy(),
        }))
    }

    /// インデックス代入をコンパイル
    pub fn compile_index_assignment(&mut self, _index_expr: &IndexExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Index assignment not yet implemented".to_string(),
            span: Span::dummy(),
        }))
    }

    /// デリファレンス代入をコンパイル
    pub fn compile_deref_assignment(&mut self, _deref_expr: &DereferenceExpr, _value: BasicValueEnum<'ctx>) -> YuniResult<()> {
        Err(YuniError::Codegen(CodegenError::Unimplemented {
            feature: "Dereference assignment not yet implemented".to_string(),
            span: Span::dummy(),
        }))
    }
}