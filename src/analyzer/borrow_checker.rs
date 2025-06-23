//! 借用チェッカー

use crate::ast::{Expression, Pattern, Span, Statement, Type};

use super::lifetime::{LifetimeContext, UsageKind};
use super::symbol::{AnalysisError, AnalysisResult, BorrowInfo, BorrowKind, Scope};

/// 借用チェッカー
#[allow(dead_code)]
pub struct BorrowChecker<'a> {
    /// ライフタイムコンテキスト
    lifetime_ctx: &'a mut LifetimeContext,
    /// 現在のスコープ
    #[allow(dead_code)]
    current_scope: &'a mut Scope,
    /// エラーコレクタ
    errors: Vec<AnalysisError>,
}

#[allow(dead_code)]
impl<'a> BorrowChecker<'a> {
    pub fn new(lifetime_ctx: &'a mut LifetimeContext, current_scope: &'a mut Scope) -> Self {
        Self {
            lifetime_ctx,
            current_scope,
            errors: Vec::new(),
        }
    }

    /// 借用チェックを実行
    pub fn check(&mut self) -> AnalysisResult<()> {
        // ライフタイムコンテキストの借用チェック
        self.lifetime_ctx.check_borrows()?;
        
        // ライフタイム制約の検証
        self.lifetime_ctx.verify_constraints()?;
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors[0].clone())
        }
    }

    /// 式の借用チェック
    pub fn check_expr(&mut self, expr: &Expression) -> AnalysisResult<()> {
        match expr {
            Expression::Identifier(id) => {
                self.check_variable_access(&id.name, &id.span)?;
            }
            Expression::Reference(ref_expr) => {
                self.check_borrow(&ref_expr.expr, ref_expr.is_mut, &ref_expr.span)?;
            }
            Expression::Field(field_expr) => {
                self.check_field_access(&field_expr.object, &field_expr.field, &field_expr.span)?;
            }
            Expression::Assignment(assign_expr) => {
                self.check_assignment(&assign_expr.target, &assign_expr.value, &assign_expr.span)?;
            }
            Expression::Call(call_expr) => {
                self.check_call(&call_expr.callee, &call_expr.args, &call_expr.span)?;
            }
            _ => {
                // その他の式は再帰的にチェック
                self.visit_expr_children(expr)?;
            }
        }
        Ok(())
    }

    /// 変数アクセスのチェック
    fn check_variable_access(&mut self, name: &str, span: &Span) -> AnalysisResult<()> {
        if let Some(symbol) = self.current_scope.lookup(name) {
            // 移動済みの変数へのアクセスをチェック
            if symbol.is_moved {
                return Err(AnalysisError::UseAfterMove {
                    name: name.to_string(),
                    span: *span,
                });
            }
            
            // 借用中の変数へのアクセスを記録
            self.lifetime_ctx.record_usage(
                name.to_string(),
                UsageKind::Read,
                *span,
            );
        }
        Ok(())
    }

    /// 借用のチェック
    fn check_borrow(&mut self, expr: &Expression, is_mutable: bool, span: &Span) -> AnalysisResult<()> {
        if let Expression::Identifier(id) = expr {
            if let Some(symbol) = self.current_scope.lookup(&id.name) {
                // 移動済みの変数を借用できない
                if symbol.is_moved {
                    return Err(AnalysisError::MoveWhileBorrowed {
                        name: id.name.to_string(),
                        span: *span,
                    });
                }
                
                // 既存の借用との競合をチェック
                if let Some(existing_borrow) = &symbol.borrow_info {
                    // 可変借用は常に排他的
                    if is_mutable && existing_borrow.kind == BorrowKind::Mutable {
                        return Err(AnalysisError::MultipleMutableBorrows {
                            name: id.name.to_string(),
                            span: *span,
                        });
                    }
                    // 既存が可変借用の場合、新しい借用（共有・可変問わず）は不可
                    if existing_borrow.kind == BorrowKind::Mutable {
                        return Err(AnalysisError::MultipleMutableBorrows {
                            name: id.name.to_string(),
                            span: *span,
                        });
                    }
                    // 新しい借用が可変の場合、既存の共有借用があってもエラー
                    if is_mutable {
                        return Err(AnalysisError::MultipleMutableBorrows {
                            name: id.name.to_string(),
                            span: *span,
                        });
                    }
                    // 共有借用同士は許可（何もしない）
                }
                
                // 新しい借用を記録
                let lifetime = self.lifetime_ctx.create_anonymous_lifetime(*span);
                let borrow_kind = if is_mutable { BorrowKind::Mutable } else { BorrowKind::Shared };
                
                self.lifetime_ctx.record_borrow(
                    id.name.to_string(),
                    borrow_kind,
                    lifetime,
                    *span,
                );
                
                self.current_scope.mark_borrowed(
                    &id.name,
                    BorrowInfo {
                        kind: borrow_kind,
                        lifetime,
                    },
                )?;
            }
        }
        Ok(())
    }

    /// フィールドアクセスのチェック
    fn check_field_access(&mut self, object: &Expression, field: &str, span: &Span) -> AnalysisResult<()> {
        // オブジェクトの借用状態をチェック
        self.check_expr(object)?;
        
        // フィールドアクセスは読み取りとして記録
        if let Expression::Identifier(id) = object {
            self.lifetime_ctx.record_usage(
                format!("{}.{}", id.name, field),
                UsageKind::Read,
                *span,
            );
        }
        Ok(())
    }

    /// 代入のチェック
    fn check_assignment(&mut self, target: &Expression, value: &Expression, span: &Span) -> AnalysisResult<()> {
        // 値の評価
        self.check_expr(value)?;
        
        // ターゲットへの書き込み権限をチェック
        match target {
            Expression::Identifier(id) => {
                if let Some(symbol) = self.current_scope.lookup(&id.name) {
                    // 不変変数への代入をチェック
                    if !symbol.is_mutable {
                        return Err(AnalysisError::ImmutableVariable {
                            name: id.name.to_string(),
                            span: *span,
                        });
                    }
                    
                    // 借用中の変数への代入をチェック
                    if symbol.borrow_info.is_some() {
                        return Err(AnalysisError::MoveWhileBorrowed {
                            name: id.name.to_string(),
                            span: *span,
                        });
                    }
                    
                    self.lifetime_ctx.record_usage(
                        id.name.to_string(),
                        UsageKind::Write,
                        *span,
                    );
                }
            }
            Expression::Field(field_expr) => {
                // オブジェクトが可変でアクセス可能かチェック
                self.check_field_assignment(&field_expr.object, &field_expr.field, span)?;
            }
            _ => {}
        }
        
        // 値が移動を伴うかチェック
        if self.is_move_expr(value) {
            self.handle_move(value)?;
        }
        
        Ok(())
    }

    /// 関数呼び出しのチェック
    fn check_call(&mut self, callee: &Expression, args: &[Expression], _span: &Span) -> AnalysisResult<()> {
        // 関数の借用チェック
        self.check_expr(callee)?;
        
        // 各引数の借用チェック
        for arg in args {
            self.check_expr(arg)?;
            
            // 引数が移動を伴うかチェック
            if self.is_move_expr(arg) {
                self.handle_move(arg)?;
            }
        }
        
        Ok(())
    }

    /// フィールドへの代入チェック
    fn check_field_assignment(&mut self, object: &Expression, field: &str, span: &Span) -> AnalysisResult<()> {
        if let Expression::Identifier(id) = object {
            if let Some(symbol) = self.current_scope.lookup(&id.name) {
                if !symbol.is_mutable {
                    return Err(AnalysisError::ImmutableVariable {
                        name: format!("{}.{}", id.name, field),
                        span: *span,
                    });
                }
                
                self.lifetime_ctx.record_usage(
                    format!("{}.{}", id.name, field),
                    UsageKind::Write,
                    *span,
                );
            }
        }
        Ok(())
    }

    /// 式が移動を伴うかチェック
    fn is_move_expr(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(id) => {
                if let Some(symbol) = self.current_scope.lookup(&id.name) {
                    // 参照型は移動しない
                    if matches!(symbol.ty, Type::Reference(_, _)) {
                        return false;
                    }
                    // コピー可能な型は移動しない
                    !self.is_copy_type(&symbol.ty)
                } else {
                    false
                }
            }
            _ => false,
        }
    }
    
    /// 型がコピー可能かチェック
    fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            // プリミティブ型はコピー可能
            Type::I8 | Type::I16 | Type::I32 | Type::I64 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 |
            Type::F32 | Type::F64 |
            Type::Bool => true,
            // 参照型もコピー可能（参照自体がコピーされる）
            Type::Reference(_, _) => true,
            // その他の型（文字列、配列、構造体など）は移動
            _ => false,
        }
    }

    /// 移動の処理
    fn handle_move(&mut self, expr: &Expression) -> AnalysisResult<()> {
        if let Expression::Identifier(id) = expr {
            if let Some(symbol) = self.current_scope.lookup(&id.name) {
                if symbol.is_moved {
                    return Err(AnalysisError::UseAfterMove {
                        name: id.name.to_string(),
                        span: id.span,
                    });
                }
                
                // 借用中の変数は移動できない
                if symbol.borrow_info.is_some() {
                    return Err(AnalysisError::MoveWhileBorrowed {
                        name: id.name.to_string(),
                        span: id.span,
                    });
                }
                
                self.current_scope.mark_moved(&id.name)?;
                
                self.lifetime_ctx.record_usage(
                    id.name.to_string(),
                    UsageKind::Move,
                    id.span,
                );
            }
        }
        Ok(())
    }

    /// 式の子要素を再帰的に訪問
    fn visit_expr_children(&mut self, expr: &Expression) -> AnalysisResult<()> {
        match expr {
            Expression::Binary(binary) => {
                self.check_expr(&binary.left)?;
                self.check_expr(&binary.right)?;
            }
            Expression::Unary(unary) => {
                self.check_expr(&unary.expr)?;
            }
            Expression::Index(index) => {
                self.check_expr(&index.object)?;
                self.check_expr(&index.index)?;
            }
            Expression::MethodCall(method_call) => {
                self.check_expr(&method_call.object)?;
                for arg in &method_call.args {
                    self.check_expr(arg)?;
                }
            }
            Expression::Array(array) => {
                for elem in &array.elements {
                    self.check_expr(elem)?;
                }
            }
            Expression::Tuple(tuple) => {
                for elem in &tuple.elements {
                    self.check_expr(elem)?;
                }
            }
            Expression::StructLit(struct_lit) => {
                for field in &struct_lit.fields {
                    self.check_expr(&field.value)?;
                }
            }
            Expression::Cast(cast) => {
                self.check_expr(&cast.expr)?;
            }
            Expression::Dereference(deref) => {
                self.check_expr(&deref.expr)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// 文の借用チェック
    pub fn check_statement(&mut self, stmt: &Statement) -> AnalysisResult<()> {
        match stmt {
            Statement::Let(let_stmt) => {
                // 初期値の借用チェック
                if let Some(ref init) = let_stmt.init {
                    self.check_expr(init)?;
                    
                    // 値が移動を伴うかチェック
                    if self.is_move_expr(init) {
                        self.handle_move(init)?;
                    }
                }
                
                // パターンに含まれる変数を記録
                self.register_pattern(&let_stmt.pattern)?;
            }
            Statement::Assignment(assign) => {
                self.check_assignment(&assign.target, &assign.value, &assign.span)?;
            }
            Statement::Expression(expr) => {
                self.check_expr(expr)?;
            }
            Statement::Return(ret) => {
                if let Some(ref value) = ret.value {
                    self.check_expr(value)?;
                    
                    // 返り値が移動を伴うかチェック
                    if self.is_move_expr(value) {
                        self.handle_move(value)?;
                    }
                }
            }
            Statement::If(if_stmt) => {
                self.check_expr(&if_stmt.condition)?;
                
                // then分岐
                for stmt in &if_stmt.then_branch.statements {
                    self.check_statement(stmt)?;
                }
                
                // else分岐
                if let Some(ref else_branch) = if_stmt.else_branch {
                    match else_branch {
                        crate::ast::ElseBranch::Block(block) => {
                            for stmt in &block.statements {
                                self.check_statement(stmt)?;
                            }
                        }
                        crate::ast::ElseBranch::If(if_stmt) => {
                            self.check_statement(&Statement::If(*if_stmt.clone()))?;
                        }
                    }
                }
            }
            Statement::While(while_stmt) => {
                self.check_expr(&while_stmt.condition)?;
                
                for stmt in &while_stmt.body.statements {
                    self.check_statement(stmt)?;
                }
            }
            Statement::For(for_stmt) => {
                if let Some(ref init) = for_stmt.init {
                    self.check_statement(init)?;
                }
                
                if let Some(ref cond) = for_stmt.condition {
                    self.check_expr(cond)?;
                }
                
                if let Some(ref update) = for_stmt.update {
                    self.check_expr(update)?;
                }
                
                for stmt in &for_stmt.body.statements {
                    self.check_statement(stmt)?;
                }
            }
            Statement::Block(block) => {
                for stmt in &block.statements {
                    self.check_statement(stmt)?;
                }
            }
        }
        Ok(())
    }

    /// パターンに含まれる変数を登録
    #[allow(clippy::only_used_in_recursion)]
    fn register_pattern(&mut self, pattern: &Pattern) -> AnalysisResult<()> {
        match pattern {
            Pattern::Identifier(_name, _) => {
                // 新しい変数の登録は別の場所で行われる
                Ok(())
            }
            Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.register_pattern(p)?;
                }
                Ok(())
            }
            Pattern::Struct(_, fields) => {
                for (_, p) in fields {
                    self.register_pattern(p)?;
                }
                Ok(())
            }
            Pattern::EnumVariant { fields, .. } => {
                match fields {
                    crate::ast::EnumVariantPatternFields::Tuple(patterns) => {
                        for pattern in patterns {
                            self.register_pattern(pattern)?;
                        }
                    }
                    crate::ast::EnumVariantPatternFields::Struct(fields) => {
                        for (_, pattern) in fields {
                            self.register_pattern(pattern)?;
                        }
                    }
                    crate::ast::EnumVariantPatternFields::Unit => {
                        // フィールドなし
                    }
                }
                Ok(())
            }
        }
    }
}