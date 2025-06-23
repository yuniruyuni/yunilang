//! セマンティック解析器のメイン実装

use crate::ast::*;
use crate::error::ErrorCollector;
use std::collections::HashMap;

use super::borrow_checker::BorrowChecker;
use super::lifetime::LifetimeContext;
use super::symbol::{AnalysisError, AnalysisResult, FunctionSignature, Scope, Symbol, TypeInfo, TypeKind};
use super::type_checker::TypeChecker;

/// セマンティック解析器
pub struct SemanticAnalyzer {
    /// スコープスタック
    scope_stack: Vec<Scope>,
    /// 型チェッカー
    type_checker: TypeChecker,
    /// インポートエイリアス
    imports: HashMap<String, String>,
    /// 現在の関数の戻り値型（return文のチェック用）
    current_return_type: Option<Type>,
    /// 現在の関数のライフタイムコンテキスト
    lifetime_context: LifetimeContext,
    /// 収集されたエラー
    errors: Vec<AnalysisError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scope_stack: vec![Scope::new()],
            type_checker: TypeChecker::new(),
            imports: HashMap::new(),
            current_return_type: None,
            lifetime_context: LifetimeContext::new(),
            errors: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &Program) -> AnalysisResult<()> {
        // インポートを処理
        for import in &program.imports {
            self.process_import(import);
        }

        // 第一パス: 型定義と関数シグネチャを収集
        for item in &program.items {
            match item {
                Item::TypeDef(type_def) => {
                    if let Err(e) = self.collect_type_definition(type_def) {
                        self.errors.push(e);
                    }
                }
                Item::Function(func) => {
                    if let Err(e) = self.collect_function_signature(func) {
                        self.errors.push(e);
                    }
                }
                Item::Method(method) => {
                    if let Err(e) = self.collect_method_signature(method) {
                        self.errors.push(e);
                    }
                }
            }
        }

        // 第二パス: 関数とメソッドの本体を解析
        for item in &program.items {
            match item {
                Item::Function(func) => {
                    if let Err(e) = self.analyze_function(func) {
                        self.errors.push(e);
                    }
                }
                Item::Method(method) => {
                    if let Err(e) = self.analyze_method(method) {
                        self.errors.push(e);
                    }
                }
                _ => {}
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            // 最初のエラーを返すが、Spanがdummyの場合は実際のエラー箇所が分からないので
            // より詳細なエラーメッセージを構築
            Err(self.errors[0].clone())
        }
    }

    fn process_import(&mut self, import: &Import) {
        let alias = import.alias.as_ref().unwrap_or(&import.path);
        self.imports.insert(alias.clone(), import.path.clone());
    }

    fn collect_type_definition(&mut self, type_def: &TypeDef) -> AnalysisResult<()> {
        match type_def {
            TypeDef::Struct(struct_def) => self.collect_struct_definition(struct_def),
            TypeDef::Enum(enum_def) => self.collect_enum_definition(enum_def),
        }
    }

    fn collect_struct_definition(&mut self, struct_def: &StructDef) -> AnalysisResult<()> {
        // フィールドの型を検証
        for field in &struct_def.fields {
            self.type_checker.validate_type(&field.ty, field.span)?;
        }

        let type_info = TypeInfo {
            name: struct_def.name.clone(),
            kind: TypeKind::Struct(struct_def.fields.clone()),
            methods: HashMap::new(),
            span: struct_def.span,
        };

        // type_checkerとscopeの両方に登録
        self.type_checker.register_type(type_info.clone())?;
        self.scope_stack.last_mut().unwrap().define_type(type_info)?;
        Ok(())
    }

    fn collect_enum_definition(&mut self, enum_def: &EnumDef) -> AnalysisResult<()> {
        // バリアントのフィールド型を検証
        for variant in &enum_def.variants {
            for field in &variant.fields {
                self.type_checker.validate_type(&field.ty, field.span)?;
            }
        }

        let type_info = TypeInfo {
            name: enum_def.name.clone(),
            kind: TypeKind::Enum(enum_def.variants.clone()),
            methods: HashMap::new(),
            span: enum_def.span,
        };

        // type_checkerとscopeの両方に登録
        self.type_checker.register_type(type_info.clone())?;
        self.scope_stack.last_mut().unwrap().define_type(type_info)?;
        Ok(())
    }

    fn collect_function_signature(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        // パラメータの型を検証
        for param in &func.params {
            self.type_checker.validate_type(&param.ty, param.span)?;
        }

        // 戻り値型を検証
        if let Some(ref return_type) = func.return_type {
            self.type_checker.validate_type(return_type, func.span)?;
        }

        let func_sig = FunctionSignature {
            name: func.name.clone(),
            params: func.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
            return_type: func.return_type.as_ref().map(|t| (**t).clone()).unwrap_or(Type::Void),
            lives_clause: func.lives_clause.clone(),
            is_method: false,
            receiver_type: None,
            span: func.span,
        };

        self.type_checker.register_function(func_sig)
    }

    fn collect_method_signature(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // レシーバの型を検証
        self.type_checker.validate_type(&method.receiver.ty, method.receiver.span)?;

        // パラメータの型を検証
        for param in &method.params {
            self.type_checker.validate_type(&param.ty, param.span)?;
        }

        // 戻り値型を検証
        if let Some(ref return_type) = method.return_type {
            self.type_checker.validate_type(return_type, method.span)?;
        }

        // メソッドを型のメソッドテーブルに追加
        if let Type::UserDefined(name) = &method.receiver.ty {
            let func_sig = FunctionSignature {
                name: method.name.clone(),
                params: method.params.iter().map(|p| (p.name.clone(), p.ty.clone())).collect(),
                return_type: method.return_type.as_ref().map(|t| (**t).clone()).unwrap_or(Type::Void),
                lives_clause: method.lives_clause.clone(),
                is_method: true,
                receiver_type: Some(method.receiver.ty.clone()),
                span: method.span,
            };

            // TODO: メソッドを型情報に追加
        }

        Ok(())
    }

    fn analyze_function(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        // 新しいスコープを開始
        self.enter_scope();
        self.current_return_type = func.return_type.as_ref().map(|t| (**t).clone());

        // パラメータをスコープに追加
        for param in &func.params {
            let symbol = Symbol {
                name: param.name.clone(),
                ty: param.ty.clone(),
                is_mutable: false,
                span: param.span,
                borrow_info: None,
                is_moved: false,
                lifetime: None,
            };
            self.scope_stack.last_mut().unwrap().define(symbol)?;
        }

        // 関数本体を解析
        let returns = self.analyze_block(&func.body)?;

        // 戻り値の確認
        if !returns && func.return_type.is_some() {
            self.errors.push(AnalysisError::MissingReturn {
                name: func.name.clone(),
                span: func.span,
            });
        }

        // 借用チェック
        let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, self.scope_stack.last_mut().unwrap());
        if let Err(e) = borrow_checker.check() {
            self.errors.push(e);
        }

        self.current_return_type = None;
        self.exit_scope();

        Ok(())
    }

    fn analyze_method(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // 新しいスコープを開始
        self.enter_scope();
        self.current_return_type = method.return_type.as_ref().map(|t| (**t).clone());

        // レシーバをスコープに追加
        let receiver_name = method.receiver.name.as_deref().unwrap_or("self");
        let is_mut_ref = matches!(&method.receiver.ty, Type::Reference(_, true));
        let symbol = Symbol {
            name: receiver_name.to_string(),
            ty: method.receiver.ty.clone(),
            is_mutable: is_mut_ref,
            span: method.receiver.span,
            borrow_info: None,
            is_moved: false,
            lifetime: None,
        };
        self.scope_stack.last_mut().unwrap().define(symbol)?;

        // パラメータをスコープに追加
        for param in &method.params {
            let symbol = Symbol {
                name: param.name.clone(),
                ty: param.ty.clone(),
                is_mutable: false,
                span: param.span,
                borrow_info: None,
                is_moved: false,
                lifetime: None,
            };
            self.scope_stack.last_mut().unwrap().define(symbol)?;
        }

        // メソッド本体を解析
        let returns = self.analyze_block(&method.body)?;

        // 戻り値の確認
        if !returns && method.return_type.is_some() {
            self.errors.push(AnalysisError::MissingReturn {
                name: method.name.clone(),
                span: method.span,
            });
        }

        // 借用チェック
        let mut borrow_checker = BorrowChecker::new(&mut self.lifetime_context, self.scope_stack.last_mut().unwrap());
        if let Err(e) = borrow_checker.check() {
            self.errors.push(e);
        }

        self.current_return_type = None;
        self.exit_scope();

        Ok(())
    }

    fn analyze_block(&mut self, block: &Block) -> AnalysisResult<bool> {
        let mut returns = false;
        
        for stmt in &block.statements {
            if returns {
                // TODO: Implement unreachable code detection
            }
            
            match self.analyze_statement(stmt) {
                Ok(stmt_returns) => returns = stmt_returns,
                Err(e) => self.errors.push(e),
            }
        }
        
        Ok(returns)
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> AnalysisResult<bool> {
        match stmt {
            Statement::Let(let_stmt) => self.analyze_let_statement(let_stmt),
            Statement::Assignment(assign) => self.analyze_assignment(assign),
            Statement::Return(ret) => self.analyze_return_statement(ret),
            Statement::If(if_stmt) => self.analyze_if_statement(if_stmt),
            Statement::While(while_stmt) => self.analyze_while_statement(while_stmt),
            Statement::For(for_stmt) => self.analyze_for_statement(for_stmt),
            Statement::Expression(expr) => {
                self.analyze_expression(expr)?;
                Ok(false)
            }
            Statement::Block(block) => {
                self.enter_scope();
                let returns = self.analyze_block(block)?;
                self.exit_scope();
                Ok(returns)
            }
        }
    }

    /// let文の解析
    fn analyze_let_statement(&mut self, let_stmt: &LetStatement) -> AnalysisResult<bool> {
        // 初期化式がある場合は型チェック
        let inferred_type = if let Some(ref init_expr) = let_stmt.init {
            // 型注釈がある場合はそれを期待される型として使用
            let expr_type = if let Some(ref annotated_type) = let_stmt.ty {
                self.type_checker.validate_type(annotated_type, let_stmt.span)?;
                let expr_type = self.analyze_expression_with_type(init_expr, Some(annotated_type))?;
                self.type_checker.check_type_compatibility(annotated_type, &expr_type, let_stmt.span)?;
                annotated_type.clone()
            } else {
                self.analyze_expression(init_expr)?
            };
            
            expr_type
        } else if let Some(ref annotated_type) = let_stmt.ty {
            self.type_checker.validate_type(annotated_type, let_stmt.span)?;
            annotated_type.clone()
        } else {
            return Err(AnalysisError::TypeInferenceError {
                name: "variable".to_string(), // パターンの名前は後で取得
                span: let_stmt.span,
            });
        };

        // パターンの解析（今は簡単な識別子のみ対応）
        if let Pattern::Identifier(name, is_mutable) = &let_stmt.pattern {
            let symbol = Symbol {
                name: name.clone(),
                ty: inferred_type,
                is_mutable: *is_mutable,
                span: let_stmt.span,
                borrow_info: None,
                is_moved: false,
                lifetime: None,
            };
            
            self.scope_stack.last_mut().unwrap().define(symbol)?;
        }
        
        Ok(false)
    }

    /// 代入文の解析
    fn analyze_assignment(&mut self, assign: &AssignStatement) -> AnalysisResult<bool> {
        // 左辺の解析
        let target_type = self.analyze_expression(&assign.target)?;
        
        // 右辺の解析
        let value_type = self.analyze_expression(&assign.value)?;
        
        // 型の互換性チェック
        self.type_checker.check_type_compatibility(&target_type, &value_type, assign.span)?;
        
        // 変更可能性のチェック
        if let Expression::Identifier(ident) = &assign.target {
            if let Some(symbol) = self.lookup_variable(&ident.name) {
                if !symbol.is_mutable {
                    return Err(AnalysisError::ImmutableVariable {
                        name: ident.name.clone(),
                        span: assign.span,
                    });
                }
            }
        }
        
        Ok(false)
    }

    /// return文の解析
    fn analyze_return_statement(&mut self, ret: &ReturnStatement) -> AnalysisResult<bool> {
        let expected_type = self.current_return_type.clone();
        let return_type = if let Some(ref expr) = ret.value {
            // 現在の関数の戻り値型を期待される型として渡す
            self.analyze_expression_with_type(expr, expected_type.as_ref())?
        } else {
            Type::Void
        };
        
        // 関数の戻り値型と一致するかチェック
        if let Some(ref expected_type) = self.current_return_type {
            self.type_checker.check_type_compatibility(expected_type, &return_type, ret.span)?;
        }
        
        Ok(true)
    }

    /// if文の解析
    fn analyze_if_statement(&mut self, if_stmt: &IfStatement) -> AnalysisResult<bool> {
        // 条件式の型チェック
        let condition_type = self.analyze_expression(&if_stmt.condition)?;
        if !matches!(condition_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_checker.type_to_string(&condition_type),
                span: self.get_expression_span(&if_stmt.condition),
            });
        }
        
        // then節の解析
        let then_returns = self.analyze_block(&if_stmt.then_branch)?;
        
        // else節の解析（存在する場合）
        let else_returns = if let Some(ref else_branch) = if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => self.analyze_block(block)?,
                ElseBranch::If(if_stmt) => self.analyze_if_statement(if_stmt)?,
            }
        } else {
            false
        };
        
        // 両方の分岐でreturnする場合のみ、このif文がreturnする
        Ok(then_returns && else_returns)
    }

    /// while文の解析
    fn analyze_while_statement(&mut self, while_stmt: &WhileStatement) -> AnalysisResult<bool> {
        // 条件式の型チェック
        let condition_type = self.analyze_expression(&while_stmt.condition)?;
        if !matches!(condition_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_checker.type_to_string(&condition_type),
                span: self.get_expression_span(&while_stmt.condition),
            });
        }
        
        // ループ本体の解析
        self.analyze_block(&while_stmt.body)?;
        
        // while文は必ずしもreturnしない（条件がfalseの場合実行されない可能性）
        Ok(false)
    }

    /// for文の解析
    fn analyze_for_statement(&mut self, for_stmt: &ForStatement) -> AnalysisResult<bool> {
        // 新しいスコープを作成（ループ変数用）
        self.enter_scope();
        
        // init文の解析（存在する場合）
        if let Some(ref init) = for_stmt.init {
            self.analyze_statement(init)?;
        }
        
        // 条件式の解析（存在する場合）
        if let Some(ref condition) = for_stmt.condition {
            let condition_type = self.analyze_expression(condition)?;
            if !matches!(condition_type, Type::Bool) {
                return Err(AnalysisError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: self.type_checker.type_to_string(&condition_type),
                    span: self.get_expression_span(condition),
                });
            }
        }
        
        // ループ本体の解析
        self.analyze_block(&for_stmt.body)?;
        
        // update式の解析（存在する場合）
        if let Some(ref update) = for_stmt.update {
            self.analyze_expression(update)?;
        }
        
        self.exit_scope();
        
        // for文は必ずしもreturnしない（条件がfalseの場合実行されない可能性）
        Ok(false)
    }

    /// 式の解析と型推論（期待される型のコンテキストなし）
    fn analyze_expression(&mut self, expr: &Expression) -> AnalysisResult<Type> {
        self.analyze_expression_with_type(expr, None)
    }

    /// 式の解析と型推論（期待される型のコンテキスト付き）
    fn analyze_expression_with_type(&mut self, expr: &Expression, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        match expr {
            Expression::Integer(int_lit) => {
                if let Some(suffix) = &int_lit.suffix {
                    match suffix.as_str() {
                        "i8" => Ok(Type::I8),
                        "i16" => Ok(Type::I16),
                        "i32" => Ok(Type::I32),
                        "i64" => Ok(Type::I64),
                        "i128" => Ok(Type::I128),
                        "u8" => Ok(Type::U8),
                        "u16" => Ok(Type::U16),
                        "u32" => Ok(Type::U32),
                        "u64" => Ok(Type::U64),
                        "u128" => Ok(Type::U128),
                        _ => Ok(Type::I32), // デフォルト
                    }
                } else {
                    // 期待される型が指定されている場合はそれを使用
                    if let Some(expected) = expected_type {
                        match expected {
                            Type::I8 => Ok(Type::I8),
                            Type::I16 => Ok(Type::I16),
                            Type::I32 => Ok(Type::I32),
                            Type::I64 => Ok(Type::I64),
                            Type::I128 => Ok(Type::I128),
                            Type::U8 => Ok(Type::U8),
                            Type::U16 => Ok(Type::U16),
                            Type::U32 => Ok(Type::U32),
                            Type::U64 => Ok(Type::U64),
                            Type::U128 => Ok(Type::U128),
                            _ => Ok(Type::I32), // 整数型でない場合はデフォルト
                        }
                    } else {
                        Ok(Type::I32) // サフィックスがない場合のデフォルトはi32
                    }
                }
            },
            Expression::Float(float_lit) => {
                if let Some(suffix) = &float_lit.suffix {
                    match suffix.as_str() {
                        "f32" => Ok(Type::F32),
                        "f64" => Ok(Type::F64),
                        _ => Ok(Type::F64), // fallback
                    }
                } else {
                    Ok(Type::F64) // default when no suffix
                }
            },
            Expression::String(_) => Ok(Type::String),
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::Identifier(ident) => {
                if let Some(symbol) = self.lookup_variable(&ident.name) {
                    Ok(symbol.ty.clone())
                } else {
                    Err(AnalysisError::UndefinedVariable {
                        name: ident.name.clone(),
                        span: ident.span,
                    })
                }
            }
            Expression::Binary(binary) => self.analyze_binary_expression(binary),
            Expression::Unary(unary) => self.analyze_unary_expression(unary),
            Expression::Call(call) => self.analyze_call_expression(call),
            Expression::Field(field) => self.analyze_field_expression(field),
            Expression::StructLit(struct_lit) => self.analyze_struct_literal(struct_lit),
            Expression::Array(array) => self.analyze_array_expression(array),
            Expression::Cast(cast) => {
                self.analyze_expression(&cast.expr)?;
                self.type_checker.validate_type(&cast.ty, cast.span)?;
                Ok(cast.ty.clone())
            }
            Expression::Match(match_expr) => self.analyze_match_expression(match_expr),
            Expression::EnumVariant(enum_variant) => self.analyze_enum_variant_expression(enum_variant),
            Expression::MethodCall(method_call) => self.analyze_method_call_expression(method_call),
            Expression::If(if_expr) => self.analyze_if_expression(if_expr),
            Expression::Block(block_expr) => self.analyze_block_expression(block_expr),
            Expression::TemplateString(_template_str) => {
                // テンプレート文字列は今のところString型として扱う
                Ok(Type::String)
            }
            Expression::Path(path_expr) => {
                // パス式（Enum::Variantなど）の解析
                // 2つのセグメントの場合、Enum variantとして処理
                if path_expr.segments.len() == 2 {
                    // これはパーサーのバグで、本来はEnumVariantExprとして解析されるべき
                    // しかし、とりあえずEnum型として処理
                    return Ok(Type::UserDefined(path_expr.segments[0].clone()));
                }
                
                // その他のパス式は未実装
                Err(AnalysisError::UndefinedVariable {
                    name: path_expr.segments.join("::"),
                    span: path_expr.span,
                })
            }
            Expression::Index(index_expr) => {
                // インデックスアクセスの解析
                let object_type = self.analyze_expression(&index_expr.object)?;
                let index_type = self.analyze_expression(&index_expr.index)?;
                
                // 配列型の場合、要素型を返す
                match object_type {
                    Type::Array(elem_type) => {
                        // インデックスが整数型であることを確認
                        if !self.type_checker.is_integer_type(&index_type) {
                            return Err(AnalysisError::TypeMismatch {
                                expected: "integer type".to_string(),
                                found: self.type_checker.type_to_string(&index_type),
                                span: index_expr.span,
                            });
                        }
                        Ok(*elem_type)
                    }
                    _ => Err(AnalysisError::TypeMismatch {
                        expected: "array type".to_string(),
                        found: self.type_checker.type_to_string(&object_type),
                        span: index_expr.span,
                    }),
                }
            }
            Expression::Reference(ref_expr) => {
                // 参照式の解析
                let inner_type = self.analyze_expression(&ref_expr.expr)?;
                Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
            }
            Expression::Dereference(deref_expr) => {
                // 参照外し式の解析
                let ref_type = self.analyze_expression(&deref_expr.expr)?;
                match ref_type {
                    Type::Reference(inner_type, _) => Ok(*inner_type),
                    _ => Err(AnalysisError::TypeMismatch {
                        expected: "reference type".to_string(),
                        found: self.type_checker.type_to_string(&ref_type),
                        span: deref_expr.span,
                    }),
                }
            }
            Expression::Assignment(assign_expr) => {
                // 代入式の解析
                let target_type = self.analyze_expression(&assign_expr.target)?;
                let value_type = self.analyze_expression(&assign_expr.value)?;
                
                // 型の互換性チェック
                self.type_checker.check_type_compatibility(&target_type, &value_type, assign_expr.span)?;
                
                // 代入式の値はunit型
                Ok(Type::Void)
            }
            Expression::Tuple(tuple_expr) => {
                // タプル式の解析
                let mut element_types = Vec::new();
                for elem in &tuple_expr.elements {
                    element_types.push(self.analyze_expression(elem)?);
                }
                Ok(Type::Tuple(element_types))
            }
        }
    }

    /// 二項演算式の解析
    fn analyze_binary_expression(&mut self, binary: &BinaryExpr) -> AnalysisResult<Type> {
        let left_type = self.analyze_expression(&binary.left)?;
        let right_type = self.analyze_expression(&binary.right)?;
        
        self.type_checker.binary_op_result_type(&binary.op, &left_type, &right_type, binary.span)
    }

    /// 単項演算式の解析
    fn analyze_unary_expression(&mut self, unary: &UnaryExpr) -> AnalysisResult<Type> {
        let operand_type = self.analyze_expression(&unary.expr)?;
        
        self.type_checker.unary_op_result_type(&unary.op, &operand_type, unary.span)
    }

    /// 関数呼び出し式の解析
    fn analyze_call_expression(&mut self, call: &CallExpr) -> AnalysisResult<Type> {
        if let Expression::Identifier(ident) = call.callee.as_ref() {
            // println関数の特別な処理（任意の数の引数と型を受け入れる）
            if ident.name == "println" {
                // 最低1つの引数が必要
                if call.args.is_empty() {
                    return Err(AnalysisError::ArgumentCountMismatch {
                        expected: 1,
                        found: 0,
                        span: call.span,
                    });
                }
                // 全ての引数の型を解析するが、型チェックはしない（任意の型を受け入れる）
                for arg in &call.args {
                    self.analyze_expression(arg)?;
                }
                return Ok(Type::Void);
            }
            
            if let Some(func_sig) = self.type_checker.get_function_signature(&ident.name).cloned() {
                // 引数数のチェック
                if call.args.len() != func_sig.params.len() {
                    return Err(AnalysisError::ArgumentCountMismatch {
                        expected: func_sig.params.len(),
                        found: call.args.len(),
                        span: call.span,
                    });
                }
                
                // 各引数の型チェック
                for (i, arg) in call.args.iter().enumerate() {
                    let arg_type = self.analyze_expression(arg)?;
                    let expected_type = &func_sig.params[i].1;
                    self.type_checker.check_type_compatibility(expected_type, &arg_type, call.span)?;
                }
                
                Ok(func_sig.return_type)
            } else {
                Err(AnalysisError::UndefinedFunction {
                    name: ident.name.clone(),
                    span: call.span,
                })
            }
        } else {
            // 関数ポインタ呼び出しなど、将来の拡張
            Ok(Type::Void)
        }
    }

    /// フィールドアクセス式の解析
    fn analyze_field_expression(&mut self, field: &FieldExpr) -> AnalysisResult<Type> {
        let object_type = self.analyze_expression(&field.object)?;
        self.type_checker.get_field_type(&object_type, &field.field, field.span)
    }

    /// 構造体リテラル式の解析
    fn analyze_struct_literal(&mut self, struct_lit: &StructLiteral) -> AnalysisResult<Type> {
        // 構造体型の検証
        let struct_name = struct_lit.name.clone();
        let struct_span = struct_lit.span;
        
        if let Some(type_info) = self.type_checker.get_type_info(&struct_name) {
            let fields = match &type_info.kind {
                TypeKind::Struct(fields) => fields.clone(),
                _ => return Err(AnalysisError::InvalidOperation {
                    message: format!("Type {} is not a struct", struct_name),
                    span: struct_span,
                }),
            };
            
            // 各フィールドの型チェック
            for field_init in &struct_lit.fields {
                if let Some(field_def) = fields.iter().find(|f| f.name == field_init.name) {
                    let value_type = self.analyze_expression(&field_init.value)?;
                    self.type_checker.check_type_compatibility(&field_def.ty, &value_type, struct_span)?;
                } else {
                    return Err(AnalysisError::UndefinedVariable {
                        name: format!("{}.{}", struct_name, field_init.name),
                        span: struct_span,
                    });
                }
            }
            
            Ok(Type::UserDefined(struct_name))
        } else {
            Err(AnalysisError::UndefinedType {
                name: struct_name,
                span: struct_span,
            })
        }
    }

    /// 配列式の解析
    fn analyze_array_expression(&mut self, array: &ArrayExpr) -> AnalysisResult<Type> {
        if array.elements.is_empty() {
            // 空配列の場合、型を推論できない
            return Err(AnalysisError::TypeInferenceError {
                name: "array".to_string(),
                span: array.span,
            });
        }
        
        // 最初の要素の型を基準とする
        let first_element_type = self.analyze_expression(&array.elements[0])?;
        
        // 残りの要素の型が一致するかチェック
        for (_i, element) in array.elements.iter().skip(1).enumerate() {
            let element_type = self.analyze_expression(element)?;
            if !self.type_checker.types_compatible(&first_element_type, &element_type) {
                return Err(AnalysisError::TypeMismatch {
                    expected: self.type_checker.type_to_string(&first_element_type),
                    found: self.type_checker.type_to_string(&element_type),
                    span: self.get_expression_span(element),
                });
            }
        }
        
        Ok(Type::Array(Box::new(first_element_type)))
    }

    /// 変数の検索
    fn lookup_variable(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(symbol) = scope.lookup(name) {
                return Some(symbol);
            }
        }
        None
    }
    
    /// 型の検索
    fn lookup_type(&self, name: &str) -> Option<&TypeInfo> {
        for scope in self.scope_stack.iter().rev() {
            if let Some(type_info) = scope.lookup_type(name) {
                return Some(type_info);
            }
        }
        None
    }
    
    fn enter_scope(&mut self) {
        let new_scope = Scope::new();
        self.scope_stack.push(new_scope);
        self.lifetime_context.enter_scope();
    }
    
    fn exit_scope(&mut self) {
        if self.scope_stack.len() > 1 {
            self.scope_stack.pop();
        }
        self.lifetime_context.exit_scope();
    }
    
    fn get_statement_span(&self, stmt: &Statement) -> Span {
        match stmt {
            Statement::Let(s) => s.span,
            Statement::Assignment(s) => s.span,
            Statement::Return(s) => s.span,
            Statement::If(s) => s.span,
            Statement::While(s) => s.span,
            Statement::For(s) => s.span,
            Statement::Expression(e) => self.get_expression_span(e),
            Statement::Block(b) => b.span,
        }
    }
    
    fn get_expression_span(&self, expr: &Expression) -> Span {
        match expr {
            Expression::Integer(i) => i.span,
            Expression::Float(f) => f.span,
            Expression::String(s) => s.span,
            Expression::TemplateString(t) => t.span,
            Expression::Boolean(b) => b.span,
            Expression::Identifier(i) => i.span,
            Expression::Path(p) => p.span,
            Expression::Binary(b) => b.span,
            Expression::Unary(u) => u.span,
            Expression::Call(c) => c.span,
            Expression::MethodCall(m) => m.span,
            Expression::Index(i) => i.span,
            Expression::Field(f) => f.span,
            Expression::Reference(r) => r.span,
            Expression::Dereference(d) => d.span,
            Expression::StructLit(s) => s.span,
            Expression::EnumVariant(e) => e.span,
            Expression::Array(a) => a.span,
            Expression::Tuple(t) => t.span,
            Expression::Cast(c) => c.span,
            Expression::Assignment(a) => a.span,
            Expression::Match(m) => m.span,
            Expression::If(i) => i.span,
            Expression::Block(b) => b.span,
        }
    }
    
    /// match式の解析
    fn analyze_match_expression(&mut self, match_expr: &MatchExpr) -> AnalysisResult<Type> {
        // match対象の式を解析
        let expr_type = self.analyze_expression(&match_expr.expr)?;
        
        if match_expr.arms.is_empty() {
            return Err(AnalysisError::TypeMismatch {
                expected: "non-empty match".to_string(),
                found: "empty match".to_string(),
                span: match_expr.span,
            });
        }
        
        // 最初のarmの型を基準とする
        let first_arm = &match_expr.arms[0];
        self.analyze_pattern(&first_arm.pattern, &expr_type)?;
        let expected_type = self.analyze_expression(&first_arm.expr)?;
        
        // 残りのarmの型を確認
        for arm in &match_expr.arms[1..] {
            self.analyze_pattern(&arm.pattern, &expr_type)?;
            let arm_type = self.analyze_expression(&arm.expr)?;
            if !self.type_checker.types_compatible(&expected_type, &arm_type) {
                return Err(AnalysisError::TypeMismatch {
                    expected: format!("{:?}", expected_type),
                    found: format!("{:?}", arm_type),
                    span: match_expr.span,
                });
            }
        }
        
        Ok(expected_type)
    }
    
    /// enum variant式の解析
    fn analyze_enum_variant_expression(&mut self, enum_variant: &EnumVariantExpr) -> AnalysisResult<Type> {
        // enum型が定義されているかチェック（借用を避けるためにクローンする）
        let enum_def = if let Some(enum_def) = self.lookup_type(&enum_variant.enum_name) {
            enum_def.clone()
        } else {
            return Err(AnalysisError::UndefinedType {
                name: enum_variant.enum_name.clone(),
                span: enum_variant.span,
            });
        };

        // variant が存在するかチェック
        if let TypeKind::Enum(variants) = &enum_def.kind {
            for variant in variants {
                if variant.name == enum_variant.variant {
                    // フィールドの型チェック
                    match (&enum_variant.fields, &variant.fields) {
                        (crate::ast::EnumVariantFields::Unit, fields) if fields.is_empty() => {
                            return Ok(Type::UserDefined(enum_variant.enum_name.clone()));
                        }
                        (crate::ast::EnumVariantFields::Tuple(args), fields) => {
                            if args.len() != fields.len() {
                                return Err(AnalysisError::ArgumentCountMismatch {
                                    expected: fields.len(),
                                    found: args.len(),
                                    span: enum_variant.span,
                                });
                            }
                            for (arg, field) in args.iter().zip(fields.iter()) {
                                let arg_type = self.analyze_expression(arg)?;
                                if !self.type_checker.types_compatible(&field.ty, &arg_type) {
                                    return Err(AnalysisError::TypeMismatch {
                                        expected: format!("{:?}", field.ty),
                                        found: format!("{:?}", arg_type),
                                        span: enum_variant.span,
                                    });
                                }
                            }
                            return Ok(Type::UserDefined(enum_variant.enum_name.clone()));
                        }
                        (crate::ast::EnumVariantFields::Struct(field_inits), fields) => {
                            for field_init in field_inits {
                                if let Some(field) = fields.iter().find(|f| f.name == field_init.name) {
                                    let value_type = self.analyze_expression(&field_init.value)?;
                                    if !self.type_checker.types_compatible(&field.ty, &value_type) {
                                        return Err(AnalysisError::TypeMismatch {
                                            expected: format!("{:?}", field.ty),
                                            found: format!("{:?}", value_type),
                                            span: enum_variant.span,
                                        });
                                    }
                                } else {
                                    return Err(AnalysisError::UndefinedVariable {
                                        name: field_init.name.clone(),
                                        span: enum_variant.span,
                                    });
                                }
                            }
                            return Ok(Type::UserDefined(enum_variant.enum_name.clone()));
                        }
                        _ => {
                            return Err(AnalysisError::TypeMismatch {
                                expected: "matching variant fields".to_string(),
                                found: "mismatched variant fields".to_string(),
                                span: enum_variant.span,
                            });
                        }
                    }
                }
            }
            Err(AnalysisError::UndefinedVariable {
                name: format!("{}::{}", enum_variant.enum_name, enum_variant.variant),
                span: enum_variant.span,
            })
        } else {
            Err(AnalysisError::TypeMismatch {
                expected: "enum type".to_string(),
                found: format!("{:?}", enum_def.kind),
                span: enum_variant.span,
            })
        }
    }
    
    /// パターンの解析
    fn analyze_pattern(&mut self, pattern: &Pattern, expected_type: &Type) -> AnalysisResult<()> {
        match pattern {
            Pattern::Identifier(name, _is_mut) => {
                // パターン変数をスコープに追加
                let symbol = Symbol {
                    name: name.clone(),
                    ty: expected_type.clone(),
                    is_mutable: false,
                    span: crate::ast::Span::dummy(), // TODO: 適切なspan
                    borrow_info: None,
                    is_moved: false,
                    lifetime: None,
                };
                self.scope_stack.last_mut().unwrap().define(symbol)?;
                Ok(())
            }
            Pattern::EnumVariant { enum_name, variant, fields } => {
                // enum型が存在することを確認
                if let Type::UserDefined(type_name) = expected_type {
                    if type_name != enum_name {
                        return Err(AnalysisError::TypeMismatch {
                            expected: enum_name.clone(),
                            found: type_name.clone(),
                            span: crate::ast::Span::dummy(), // TODO: 適切なspan
                        });
                    }
                    // TODO: variant とフィールドの詳細チェック
                    Ok(())
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: enum_name.clone(),
                        found: format!("{:?}", expected_type),
                        span: crate::ast::Span::dummy(), // TODO: 適切なspan
                    })
                }
            }
            _ => {
                // 他のパターンは後で実装
                Ok(())
            }
        }
    }
    
    /// メソッド呼び出し式の解析
    fn analyze_method_call_expression(&mut self, method_call: &MethodCallExpr) -> AnalysisResult<Type> {
        // オブジェクトの型を取得
        let object_type = self.analyze_expression(&method_call.object)?;
        
        // メソッドが定義されているかチェック（借用を避けるためにクローンする）
        let type_info = if let Some(type_info) = self.lookup_type_info(&object_type) {
            type_info.clone()
        } else {
            return Err(AnalysisError::MethodNotFound {
                method: method_call.method.clone(),
                ty: format!("{:?}", object_type),
                span: method_call.span,
            });
        };
        
        if let Some(method_sig) = type_info.methods.get(&method_call.method) {
            // 引数数のチェック
            if method_call.args.len() != method_sig.params.len() {
                return Err(AnalysisError::ArgumentCountMismatch {
                    expected: method_sig.params.len(),
                    found: method_call.args.len(),
                    span: method_call.span,
                });
            }
            
            // 各引数の型チェック
            for (i, arg) in method_call.args.iter().enumerate() {
                let arg_type = self.analyze_expression(arg)?;
                let expected_type = &method_sig.params[i].1;
                self.type_checker.check_type_compatibility(expected_type, &arg_type, method_call.span)?;
            }
            
            Ok(method_sig.return_type.clone())
        } else {
            Err(AnalysisError::MethodNotFound {
                method: method_call.method.clone(),
                ty: format!("{:?}", object_type),
                span: method_call.span,
            })
        }
    }
    
    /// 型情報を取得（型名から）
    fn lookup_type_info(&self, ty: &Type) -> Option<&TypeInfo> {
        match ty {
            Type::UserDefined(name) => self.lookup_type(name),
            _ => None,
        }
    }
    
    /// if式の解析
    fn analyze_if_expression(&mut self, if_expr: &IfExpr) -> AnalysisResult<Type> {
        // 条件式の型チェック
        let condition_type = self.analyze_expression(&if_expr.condition)?;
        if !matches!(condition_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_checker.type_to_string(&condition_type),
                span: self.get_expression_span(&if_expr.condition),
            });
        }

        // then ブランチの型を取得
        let then_type = self.analyze_expression(&if_expr.then_branch)?;

        // else ブランチがある場合はその型もチェック
        if let Some(else_branch) = &if_expr.else_branch {
            let else_type = self.analyze_expression(else_branch)?;
            
            // then と else の型が一致する必要がある
            self.type_checker.check_type_compatibility(&then_type, &else_type, if_expr.span)?;
            Ok(then_type)
        } else {
            // else節がない場合、if式の値はunit型（値を返さない）
            Ok(Type::Void)
        }
    }

    /// ブロック式の解析
    fn analyze_block_expression(&mut self, block_expr: &BlockExpr) -> AnalysisResult<Type> {
        // 新しいスコープを作成
        self.scope_stack.push(Scope::new());

        let mut last_type = Type::Void;

        // ブロック内の文を順番に解析
        for stmt in &block_expr.statements {
            match stmt {
                Statement::Expression(expr) => {
                    last_type = self.analyze_expression(expr)?;
                }
                _ => {
                    self.analyze_statement(stmt)?;
                    last_type = Type::Void;
                }
            }
        }
        
        // 最後の式がある場合はその型を返す
        if let Some(last_expr) = &block_expr.last_expr {
            last_type = self.analyze_expression(last_expr)?;
        }

        self.scope_stack.pop();
        Ok(last_type)
    }

    // TODO: 残りのanalyze_*メソッドの実装
}