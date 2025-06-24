//! 単相化（Monomorphization）モジュール
//!
//! ジェネリック関数や型を具体的な型で特殊化する

use std::collections::{HashMap, HashSet};
use crate::ast::*;
use crate::error::YuniResult;

/// 単相化された関数の情報
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedFunction {
    /// 元の関数名
    pub original_name: String,
    /// 型引数の具体的な型へのマッピング
    pub type_args: Vec<Type>,
    /// マングルされた名前（例: Vec_i32_new）
    pub mangled_name: String,
}

/// 単相化された構造体の情報
#[derive(Debug, Clone, PartialEq)]
pub struct MonomorphizedStruct {
    /// 元の構造体名
    pub original_name: String,
    /// 型引数の具体的な型へのマッピング
    pub type_args: Vec<Type>,
    /// マングルされた名前（例: Vec_i32）
    pub mangled_name: String,
}

/// 単相化エンジン
pub struct Monomorphizer {
    /// 単相化された関数のセット（重複を避けるため）
    monomorphized_functions: HashSet<(String, Vec<Type>)>,
    /// 単相化された構造体のセット
    monomorphized_structs: HashSet<(String, Vec<Type>)>,
    /// 元のプログラム
    original_program: Program,
    /// ジェネリック関数の定義（名前 -> 関数宣言）
    generic_functions: HashMap<String, FunctionDecl>,
    /// ジェネリック構造体の定義（名前 -> 構造体定義）
    generic_structs: HashMap<String, StructDef>,
    /// ジェネリック列挙型の定義（名前 -> 列挙型定義）
    generic_enums: HashMap<String, EnumDef>,
    /// 処理すべきインスタンス化のキュー
    instantiation_queue: Vec<(String, Vec<Type>, InstantiationType)>,
    /// 生成された単相化アイテム
    generated_items: Vec<Item>,
}

/// インスタンス化の種類
#[derive(Debug, Clone, Copy, PartialEq)]
enum InstantiationType {
    Function,
    Struct,
    Enum,
}

impl Monomorphizer {
    /// 新しい単相化エンジンを作成
    pub fn new(program: Program) -> Self {
        Self {
            monomorphized_functions: HashSet::new(),
            monomorphized_structs: HashSet::new(),
            original_program: program,
            generic_functions: HashMap::new(),
            generic_structs: HashMap::new(),
            generic_enums: HashMap::new(),
            instantiation_queue: Vec::new(),
            generated_items: Vec::new(),
        }
    }
    
    /// プログラムを単相化
    pub fn monomorphize(mut self) -> YuniResult<Program> {
        // ステップ1: ジェネリック定義を収集
        self.collect_generic_definitions();
        
        // ステップ2: 最初のパスでジェネリックの使用箇所を検出
        self.collect_instantiations(&self.original_program.clone())?;
        
        // ステップ3: キューを処理して必要な単相化バージョンを生成
        while let Some((name, type_args, inst_type)) = self.instantiation_queue.pop() {
            self.process_instantiation(&name, &type_args, inst_type)?;
        }
        
        // ステップ4: 単相化されたプログラムを構築
        let mut result_program = self.original_program.clone();
        
        // ジェネリック定義を削除して、単相化されたバージョンを追加
        result_program.items.retain(|item| {
            match item {
                Item::Function(func) => func.type_params.is_empty(),
                Item::TypeDef(TypeDef::Struct(s)) => s.type_params.is_empty(),
                Item::TypeDef(TypeDef::Enum(e)) => e.type_params.is_empty(),
                Item::TypeDef(TypeDef::Alias(a)) => a.type_params.is_empty(),
                _ => true,
            }
        });
        
        // 生成された単相化アイテムを追加
        let generated_items = self.generated_items.clone();
        result_program.items.extend(generated_items);
        
        // 呼び出し箇所を単相化バージョンに置き換え
        self.replace_generic_calls(&mut result_program)?;
        
        Ok(result_program)
    }
    
    /// ジェネリック定義を収集
    fn collect_generic_definitions(&mut self) {
        for item in &self.original_program.items {
            match item {
                Item::Function(func) if !func.type_params.is_empty() => {
                    self.generic_functions.insert(func.name.clone(), func.clone());
                }
                Item::TypeDef(TypeDef::Struct(s)) if !s.type_params.is_empty() => {
                    self.generic_structs.insert(s.name.clone(), s.clone());
                }
                Item::TypeDef(TypeDef::Enum(e)) if !e.type_params.is_empty() => {
                    self.generic_enums.insert(e.name.clone(), e.clone());
                }
                Item::TypeDef(TypeDef::Alias(_)) => {
                    // ジェネリック型エイリアスの単相化は現時点では未対応
                }
                _ => {}
            }
        }
    }
    
    /// プログラム全体を走査してジェネリックの使用箇所を検出
    fn collect_instantiations(&mut self, program: &Program) -> YuniResult<()> {
        for item in &program.items {
            match item {
                Item::Function(func) => self.collect_instantiations_in_function(func)?,
                Item::Method(method) => self.collect_instantiations_in_method(method)?,
                _ => {}
            }
        }
        Ok(())
    }
    
    /// 関数内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_function(&mut self, func: &FunctionDecl) -> YuniResult<()> {
        // 型パラメータのマッピングを作成
        let type_params: HashMap<String, Type> = func.type_params.iter()
            .map(|p| (p.name.clone(), Type::Variable(p.name.clone())))
            .collect();
            
        self.collect_instantiations_in_block(&func.body, &type_params)
    }
    
    /// メソッド内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_method(&mut self, method: &MethodDecl) -> YuniResult<()> {
        // 型パラメータのマッピングを作成
        let type_params: HashMap<String, Type> = method.type_params.iter()
            .map(|p| (p.name.clone(), Type::Variable(p.name.clone())))
            .collect();
            
        self.collect_instantiations_in_block(&method.body, &type_params)
    }
    
    /// ブロック内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_block(&mut self, block: &Block, type_params: &HashMap<String, Type>) -> YuniResult<()> {
        for stmt in &block.statements {
            self.collect_instantiations_in_statement(stmt, type_params)?;
        }
        Ok(())
    }
    
    /// 文内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_statement(&mut self, stmt: &Statement, type_params: &HashMap<String, Type>) -> YuniResult<()> {
        match stmt {
            Statement::Let(let_stmt) => {
                if let Some(init) = &let_stmt.init {
                    self.collect_instantiations_in_expr(init, type_params)?;
                }
                if let Some(ty) = &let_stmt.ty {
                    self.collect_instantiations_in_type(ty, type_params)?;
                }
            }
            Statement::Assignment(assign) => {
                self.collect_instantiations_in_expr(&assign.target, type_params)?;
                self.collect_instantiations_in_expr(&assign.value, type_params)?;
            }
            Statement::Expression(expr) => {
                self.collect_instantiations_in_expr(expr, type_params)?;
            }
            Statement::Return(ret_stmt) => {
                if let Some(value) = &ret_stmt.value {
                    self.collect_instantiations_in_expr(value, type_params)?;
                }
            }
            Statement::If(if_stmt) => {
                self.collect_instantiations_in_expr(&if_stmt.condition, type_params)?;
                self.collect_instantiations_in_block(&if_stmt.then_branch, type_params)?;
                if let Some(ElseBranch::Block(else_block)) = &if_stmt.else_branch {
                    self.collect_instantiations_in_block(else_block, type_params)?;
                }
            }
            Statement::While(while_stmt) => {
                self.collect_instantiations_in_expr(&while_stmt.condition, type_params)?;
                self.collect_instantiations_in_block(&while_stmt.body, type_params)?;
            }
            Statement::For(for_stmt) => {
                if let Some(init) = &for_stmt.init {
                    self.collect_instantiations_in_statement(init, type_params)?;
                }
                if let Some(condition) = &for_stmt.condition {
                    self.collect_instantiations_in_expr(condition, type_params)?;
                }
                if let Some(update) = &for_stmt.update {
                    self.collect_instantiations_in_expr(update, type_params)?;
                }
                self.collect_instantiations_in_block(&for_stmt.body, type_params)?;
            }
            Statement::Block(block) => {
                self.collect_instantiations_in_block(block, type_params)?;
            }
        }
        Ok(())
    }
    
    /// 式内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_expr(&mut self, expr: &Expression, type_params: &HashMap<String, Type>) -> YuniResult<()> {
        match expr {
            Expression::Call(call) => {
                // 呼び出し先の式を確認
                if let Expression::Identifier(ident) = &*call.callee {
                    // ジェネリック関数の呼び出しかチェック
                    if self.generic_functions.contains_key(&ident.name) {
                        // TODO: 型推論結果から実際の型引数を取得
                        // 現在は単純化のため、引数から推論
                        let type_args = self.infer_type_args_from_call(&ident.name, &call.args)?;
                        if !type_args.is_empty() {
                            self.queue_instantiation(&ident.name, type_args, InstantiationType::Function);
                        }
                    }
                }
                
                // 引数も再帰的に処理
                for arg in &call.args {
                    self.collect_instantiations_in_expr(arg, type_params)?;
                }
            }
            Expression::StructLit(struct_lit) => {
                // ジェネリック構造体のインスタンス化かチェック
                if self.generic_structs.contains_key(&struct_lit.name) {
                    // TODO: フィールドの型から型引数を推論
                    let type_args = self.infer_type_args_from_struct_lit(struct_lit)?;
                    if !type_args.is_empty() {
                        self.queue_instantiation(&struct_lit.name, type_args, InstantiationType::Struct);
                    }
                }
                
                // フィールドの値も再帰的に処理
                for field in &struct_lit.fields {
                    self.collect_instantiations_in_expr(&field.value, type_params)?;
                }
            }
            Expression::Binary(binary) => {
                self.collect_instantiations_in_expr(&binary.left, type_params)?;
                self.collect_instantiations_in_expr(&binary.right, type_params)?;
            }
            Expression::Block(block) => {
                for stmt in &block.statements {
                    self.collect_instantiations_in_statement(stmt, type_params)?;
                }
                if let Some(last_expr) = &block.last_expr {
                    self.collect_instantiations_in_expr(last_expr, type_params)?;
                }
            }
            Expression::If(if_expr) => {
                self.collect_instantiations_in_expr(&if_expr.condition, type_params)?;
                self.collect_instantiations_in_expr(&if_expr.then_branch, type_params)?;
                if let Some(else_branch) = &if_expr.else_branch {
                    self.collect_instantiations_in_expr(else_branch, type_params)?;
                }
            }
            Expression::Match(match_expr) => {
                self.collect_instantiations_in_expr(&match_expr.expr, type_params)?;
                for arm in &match_expr.arms {
                    self.collect_instantiations_in_expr(&arm.expr, type_params)?;
                }
            }
            // 他の式タイプも必要に応じて処理
            _ => {}
        }
        Ok(())
    }
    
    /// 型内でのジェネリックの使用箇所を検出
    fn collect_instantiations_in_type(&mut self, ty: &Type, _type_params: &HashMap<String, Type>) -> YuniResult<()> {
        match ty {
            Type::Generic(name, args) => {
                // ジェネリック型の使用を検出
                if self.generic_structs.contains_key(name) {
                    self.queue_instantiation(name, args.clone(), InstantiationType::Struct);
                } else if self.generic_enums.contains_key(name) {
                    self.queue_instantiation(name, args.clone(), InstantiationType::Enum);
                }
                
                // 型引数も再帰的に処理
                for arg in args {
                    self.collect_instantiations_in_type(arg, _type_params)?;
                }
            }
            Type::Array(elem) => {
                self.collect_instantiations_in_type(elem, _type_params)?;
            }
            Type::Reference(inner, _) => {
                self.collect_instantiations_in_type(inner, _type_params)?;
            }
            Type::Tuple(elems) => {
                for elem in elems {
                    self.collect_instantiations_in_type(elem, _type_params)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    
    /// インスタンス化をキューに追加
    fn queue_instantiation(&mut self, name: &str, type_args: Vec<Type>, inst_type: InstantiationType) {
        let key = (name.to_string(), type_args.clone());
        
        // 既に単相化済みかチェック
        let already_monomorphized = match inst_type {
            InstantiationType::Function => self.monomorphized_functions.contains(&key),
            InstantiationType::Struct | InstantiationType::Enum => self.monomorphized_structs.contains(&key),
        };
        
        if !already_monomorphized {
            self.instantiation_queue.push((name.to_string(), type_args, inst_type));
            
            // キューに追加したことを記録
            match inst_type {
                InstantiationType::Function => {
                    self.monomorphized_functions.insert(key);
                }
                InstantiationType::Struct | InstantiationType::Enum => {
                    self.monomorphized_structs.insert(key);
                }
            }
        }
    }
    
    /// 関数呼び出しから型引数を推論（簡略版）
    fn infer_type_args_from_call(&self, func_name: &str, args: &[Expression]) -> YuniResult<Vec<Type>> {
        // TODO: 実際の型推論実装
        // 現在は単純に引数の型から推論
        
        // ジェネリック関数の定義を取得
        if let Some(generic_func) = self.generic_functions.get(func_name) {
            let mut type_args = Vec::new();
            let mut type_param_map: HashMap<String, Type> = HashMap::new();
            
            // 各引数から型パラメータを推論
            for (i, arg) in args.iter().enumerate() {
                if i < generic_func.params.len() {
                    let param_type = &generic_func.params[i].ty;
                    if let Some(arg_type) = self.infer_expr_type(arg) {
                        self.unify_types(param_type, &arg_type, &mut type_param_map);
                    }
                }
            }
            
            // 型パラメータの順番で型引数を収集
            for type_param in &generic_func.type_params {
                if let Some(ty) = type_param_map.get(&type_param.name) {
                    type_args.push(ty.clone());
                } else {
                    // 推論できない場合はデフォルトでi32
                    type_args.push(Type::I32);
                }
            }
            
            Ok(type_args)
        } else {
            // ジェネリック関数でない場合は空のベクタを返す
            Ok(vec![])
        }
    }
    
    /// 型を統一（簡易版）
    fn unify_types(&self, param_type: &Type, arg_type: &Type, type_map: &mut HashMap<String, Type>) {
        match (param_type, arg_type) {
            (Type::Variable(name), _) => {
                // 型変数の場合は型を記録
                type_map.insert(name.clone(), arg_type.clone());
            }
            (Type::Generic(name, param_args), Type::Generic(arg_name, arg_args)) if name == arg_name => {
                // ジェネリック型の場合は再帰的に統一
                for (p, a) in param_args.iter().zip(arg_args.iter()) {
                    self.unify_types(p, a, type_map);
                }
            }
            _ => {
                // その他の場合は何もしない
            }
        }
    }
    
    /// 構造体リテラルから型引数を推論
    fn infer_type_args_from_struct_lit(&self, struct_lit: &StructLiteral) -> YuniResult<Vec<Type>> {
        // TODO: 実際の型推論実装
        // 現在はフィールドの値から簡易的に推論
        let mut type_args = Vec::new();
        
        for field in &struct_lit.fields {
            if let Some(ty) = self.infer_expr_type(&field.value) {
                // 重複を避ける
                if !type_args.contains(&ty) {
                    type_args.push(ty);
                }
            }
        }
        
        Ok(type_args)
    }
    
    /// 式の型を推論（簡略版）
    fn infer_expr_type(&self, expr: &Expression) -> Option<Type> {
        match expr {
            Expression::Integer(int_lit) => {
                match int_lit.suffix.as_deref() {
                    Some("i8") => Some(Type::I8),
                    Some("i16") => Some(Type::I16),
                    Some("i64") => Some(Type::I64),
                    Some("u8") => Some(Type::U8),
                    Some("u16") => Some(Type::U16),
                    Some("u32") => Some(Type::U32),
                    Some("u64") => Some(Type::U64),
                    _ => Some(Type::I32), // デフォルト
                }
            }
            Expression::Float(float_lit) => {
                match float_lit.suffix.as_deref() {
                    Some("f32") => Some(Type::F32),
                    _ => Some(Type::F64), // デフォルト
                }
            }
            Expression::String(_) => Some(Type::String),
            Expression::Boolean(_) => Some(Type::Bool),
            Expression::StructLit(struct_lit) => {
                // 構造体リテラルの型を推論
                if let Ok(type_args) = self.infer_type_args_from_struct_lit(struct_lit) {
                    if type_args.is_empty() {
                        Some(Type::UserDefined(struct_lit.name.clone()))
                    } else {
                        Some(Type::Generic(struct_lit.name.clone(), type_args))
                    }
                } else {
                    Some(Type::UserDefined(struct_lit.name.clone()))
                }
            }
            Expression::Field(field_expr) => {
                // フィールドアクセスの型推論
                if let Some(object_type) = self.infer_expr_type(&field_expr.object) {
                    // 簡易的にフィールドの型を推論（実際はもっと複雑）
                    match &object_type {
                        Type::Generic(struct_name, type_args) => {
                            if let Some(struct_def) = self.generic_structs.get(struct_name) {
                                // フィールドを探す
                                for field in &struct_def.fields {
                                    if field.name == field_expr.field {
                                        // 型変数を置換
                                        return Some(self.substitute_type_vars(&field.ty, struct_def, type_args));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                None
            }
            _ => None, // TODO: 他の式タイプも実装
        }
    }
    
    /// 型変数を具体的な型に置換（簡易版）
    fn substitute_type_vars(&self, ty: &Type, struct_def: &StructDef, type_args: &[Type]) -> Type {
        match ty {
            Type::Variable(name) => {
                // 型パラメータのインデックスを探す
                for (i, param) in struct_def.type_params.iter().enumerate() {
                    if param.name == *name {
                        if let Some(arg) = type_args.get(i) {
                            return arg.clone();
                        }
                    }
                }
                ty.clone()
            }
            _ => ty.clone(),
        }
    }
    /// インスタンス化を処理
    fn process_instantiation(&mut self, name: &str, type_args: &[Type], inst_type: InstantiationType) -> YuniResult<()> {
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
        func.name = self.mangle_function_name(&func.name, type_args);
        
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
        struct_def.name = self.mangle_struct_name(&struct_def.name, type_args);
        
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
        enum_def.name = self.mangle_struct_name(&enum_def.name, type_args);
        
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
    
    /// 型を置換
    fn substitute_type(&self, ty: &Type, type_map: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Variable(name) => {
                type_map.get(name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::Generic(name, args) => {
                let substituted_args: Vec<Type> = args.iter()
                    .map(|arg| self.substitute_type(arg, type_map))
                    .collect();
                Type::Generic(name.clone(), substituted_args)
            }
            Type::Array(elem) => {
                Type::Array(Box::new(self.substitute_type(elem, type_map)))
            }
            Type::Reference(inner, is_mut) => {
                Type::Reference(Box::new(self.substitute_type(inner, type_map)), *is_mut)
            }
            Type::Tuple(elems) => {
                let substituted_elems: Vec<Type> = elems.iter()
                    .map(|elem| self.substitute_type(elem, type_map))
                    .collect();
                Type::Tuple(substituted_elems)
            }
            Type::Function(func_ty) => {
                let substituted_params: Vec<Type> = func_ty.params.iter()
                    .map(|param| self.substitute_type(param, type_map))
                    .collect();
                let substituted_ret = self.substitute_type(&func_ty.return_type, type_map);
                Type::Function(FunctionType {
                    params: substituted_params,
                    return_type: Box::new(substituted_ret),
                })
            }
            _ => ty.clone(),
        }
    }
    
    /// ブロックを置換
    fn substitute_block(&mut self, block: &Block, type_map: &HashMap<String, Type>) -> YuniResult<Block> {
        let mut new_statements = Vec::new();
        for stmt in &block.statements {
            new_statements.push(self.substitute_statement(stmt, type_map)?);
        }
        Ok(Block {
            statements: new_statements,
            span: block.span.clone(),
        })
    }
    
    /// 文を置換
    fn substitute_statement(&mut self, stmt: &Statement, type_map: &HashMap<String, Type>) -> YuniResult<Statement> {
        match stmt {
            Statement::Let(let_stmt) => {
                let new_ty = let_stmt.ty.as_ref().map(|ty| self.substitute_type(ty, type_map));
                let new_init = let_stmt.init.as_ref()
                    .map(|init| self.substitute_expr(init, type_map))
                    .transpose()?;
                Ok(Statement::Let(LetStatement {
                    pattern: let_stmt.pattern.clone(),
                    ty: new_ty,
                    init: new_init,
                    span: let_stmt.span.clone(),
                }))
            }
            Statement::Expression(expr) => {
                Ok(Statement::Expression(self.substitute_expr(expr, type_map)?))
            }
            Statement::Assignment(assign) => {
                let new_target = self.substitute_expr(&assign.target, type_map)?;
                let new_value = self.substitute_expr(&assign.value, type_map)?;
                Ok(Statement::Assignment(AssignStatement {
                    target: new_target,
                    value: new_value,
                    span: assign.span.clone(),
                }))
            }
            Statement::Return(ret_stmt) => {
                let new_value = ret_stmt.value.as_ref()
                    .map(|value| self.substitute_expr(value, type_map))
                    .transpose()?;
                Ok(Statement::Return(ReturnStatement {
                    value: new_value,
                    span: ret_stmt.span.clone(),
                }))
            }
            Statement::If(if_stmt) => {
                let new_condition = self.substitute_expr(&if_stmt.condition, type_map)?;
                let new_then = self.substitute_block(&if_stmt.then_branch, type_map)?;
                let new_else = match &if_stmt.else_branch {
                    Some(ElseBranch::Block(block)) => 
                        Some(ElseBranch::Block(self.substitute_block(block, type_map)?)),
                    Some(ElseBranch::If(if_stmt)) => {
                        if let Statement::If(new_if) = self.substitute_statement(
                            &Statement::If(*if_stmt.clone()), type_map)? {
                            Some(ElseBranch::If(Box::new(new_if)))
                        } else {
                            None
                        }
                    }
                    None => None,
                };
                Ok(Statement::If(IfStatement {
                    condition: new_condition,
                    then_branch: new_then,
                    else_branch: new_else,
                    span: if_stmt.span.clone(),
                }))
            }
            Statement::While(while_stmt) => {
                let new_condition = self.substitute_expr(&while_stmt.condition, type_map)?;
                let new_body = self.substitute_block(&while_stmt.body, type_map)?;
                Ok(Statement::While(WhileStatement {
                    condition: new_condition,
                    body: new_body,
                    span: while_stmt.span.clone(),
                }))
            }
            Statement::For(for_stmt) => {
                let new_init = match for_stmt.init.as_ref() {
                    Some(init) => Some(Box::new(self.substitute_statement(init, type_map)?)),
                    None => None,
                };
                let new_condition = for_stmt.condition.as_ref()
                    .map(|cond| self.substitute_expr(cond, type_map))
                    .transpose()?;
                let new_update = for_stmt.update.as_ref()
                    .map(|update| self.substitute_expr(update, type_map))
                    .transpose()?;
                let new_body = self.substitute_block(&for_stmt.body, type_map)?;
                Ok(Statement::For(ForStatement {
                    init: new_init,
                    condition: new_condition,
                    update: new_update,
                    body: new_body,
                    span: for_stmt.span.clone(),
                }))
            }
            Statement::Block(block) => {
                Ok(Statement::Block(self.substitute_block(block, type_map)?))
            }
        }
    }
    
    /// 式を置換
    fn substitute_expr(&mut self, expr: &Expression, type_map: &HashMap<String, Type>) -> YuniResult<Expression> {
        match expr {
            Expression::Call(call) => {
                let new_callee = self.substitute_expr(&call.callee, type_map)?;
                let mut new_args = Vec::new();
                for arg in &call.args {
                    new_args.push(self.substitute_expr(arg, type_map)?);
                }
                Ok(Expression::Call(CallExpr {
                    callee: Box::new(new_callee),
                    args: new_args,
                    span: call.span.clone(),
                    is_tail: call.is_tail,
                }))
            }
            Expression::StructLit(struct_lit) => {
                let mut new_fields = Vec::new();
                for field in &struct_lit.fields {
                    new_fields.push(StructFieldInit {
                        name: field.name.clone(),
                        value: self.substitute_expr(&field.value, type_map)?,
                    });
                }
                Ok(Expression::StructLit(StructLiteral {
                    name: struct_lit.name.clone(),
                    fields: new_fields,
                    span: struct_lit.span.clone(),
                }))
            }
            Expression::Binary(binary) => {
                let new_left = self.substitute_expr(&binary.left, type_map)?;
                let new_right = self.substitute_expr(&binary.right, type_map)?;
                Ok(Expression::Binary(BinaryExpr {
                    left: Box::new(new_left),
                    op: binary.op.clone(),
                    right: Box::new(new_right),
                    span: binary.span.clone(),
                }))
            }
            Expression::Unary(unary) => {
                let new_expr = self.substitute_expr(&unary.expr, type_map)?;
                Ok(Expression::Unary(UnaryExpr {
                    op: unary.op.clone(),
                    expr: Box::new(new_expr),
                    span: unary.span.clone(),
                }))
            }
            Expression::Field(field) => {
                let new_object = self.substitute_expr(&field.object, type_map)?;
                Ok(Expression::Field(FieldExpr {
                    object: Box::new(new_object),
                    field: field.field.clone(),
                    span: field.span.clone(),
                }))
            }
            Expression::Index(index) => {
                let new_object = self.substitute_expr(&index.object, type_map)?;
                let new_index = self.substitute_expr(&index.index, type_map)?;
                Ok(Expression::Index(IndexExpr {
                    object: Box::new(new_object),
                    index: Box::new(new_index),
                    span: index.span.clone(),
                }))
            }
            Expression::Block(block) => {
                let mut new_statements = Vec::new();
                for stmt in &block.statements {
                    new_statements.push(self.substitute_statement(stmt, type_map)?);
                }
                let new_last_expr = match block.last_expr.as_ref() {
                    Some(expr) => Some(Box::new(self.substitute_expr(expr, type_map)?)),
                    None => None,
                };
                Ok(Expression::Block(BlockExpr {
                    statements: new_statements,
                    last_expr: new_last_expr,
                    span: block.span.clone(),
                }))
            }
            Expression::If(if_expr) => {
                let new_condition = Box::new(self.substitute_expr(&if_expr.condition, type_map)?);
                let new_then = Box::new(self.substitute_expr(&if_expr.then_branch, type_map)?);
                let new_else = match if_expr.else_branch.as_ref() {
                    Some(else_branch) => Some(Box::new(self.substitute_expr(else_branch, type_map)?)),
                    None => None,
                };
                Ok(Expression::If(IfExpr {
                    condition: new_condition,
                    then_branch: new_then,
                    else_branch: new_else,
                    span: if_expr.span.clone(),
                }))
            }
            Expression::Match(match_expr) => {
                let new_expr = Box::new(self.substitute_expr(&match_expr.expr, type_map)?);
                let mut new_arms = Vec::new();
                for arm in &match_expr.arms {
                    new_arms.push(MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref()
                            .map(|guard| self.substitute_expr(guard, type_map))
                            .transpose()?,
                        expr: self.substitute_expr(&arm.expr, type_map)?,
                    });
                }
                Ok(Expression::Match(MatchExpr {
                    expr: new_expr,
                    arms: new_arms,
                    span: match_expr.span.clone(),
                }))
            }
            Expression::Reference(ref_expr) => {
                let new_expr = Box::new(self.substitute_expr(&ref_expr.expr, type_map)?);
                Ok(Expression::Reference(ReferenceExpr {
                    expr: new_expr,
                    is_mut: ref_expr.is_mut,
                    span: ref_expr.span.clone(),
                }))
            }
            Expression::Dereference(deref) => {
                let new_expr = Box::new(self.substitute_expr(&deref.expr, type_map)?);
                Ok(Expression::Dereference(DereferenceExpr {
                    expr: new_expr,
                    span: deref.span.clone(),
                }))
            }
            Expression::Cast(cast) => {
                let new_expr = Box::new(self.substitute_expr(&cast.expr, type_map)?);
                let new_ty = self.substitute_type(&cast.ty, type_map);
                Ok(Expression::Cast(CastExpr {
                    expr: new_expr,
                    ty: new_ty,
                    span: cast.span.clone(),
                }))
            }
            Expression::MethodCall(method_call) => {
                let new_object = Box::new(self.substitute_expr(&method_call.object, type_map)?);
                let mut new_args = Vec::new();
                for arg in &method_call.args {
                    new_args.push(self.substitute_expr(arg, type_map)?);
                }
                Ok(Expression::MethodCall(MethodCallExpr {
                    object: new_object,
                    method: method_call.method.clone(),
                    args: new_args,
                    span: method_call.span.clone(),
                }))
            }
            Expression::Array(array) => {
                let mut new_elements = Vec::new();
                for elem in &array.elements {
                    new_elements.push(self.substitute_expr(elem, type_map)?);
                }
                Ok(Expression::Array(ArrayExpr {
                    elements: new_elements,
                    span: array.span.clone(),
                }))
            }
            Expression::Tuple(tuple) => {
                let mut new_elements = Vec::new();
                for elem in &tuple.elements {
                    new_elements.push(self.substitute_expr(elem, type_map)?);
                }
                Ok(Expression::Tuple(TupleExpr {
                    elements: new_elements,
                    span: tuple.span.clone(),
                }))
            }
            Expression::Assignment(assign) => {
                let new_target = Box::new(self.substitute_expr(&assign.target, type_map)?);
                let new_value = Box::new(self.substitute_expr(&assign.value, type_map)?);
                Ok(Expression::Assignment(AssignmentExpr {
                    target: new_target,
                    value: new_value,
                    span: assign.span.clone(),
                }))
            }
            Expression::EnumVariant(variant) => {
                let new_fields = match &variant.fields {
                    EnumVariantFields::Tuple(exprs) => {
                        let mut new_exprs = Vec::new();
                        for expr in exprs {
                            new_exprs.push(self.substitute_expr(expr, type_map)?);
                        }
                        EnumVariantFields::Tuple(new_exprs)
                    }
                    EnumVariantFields::Struct(fields) => {
                        let mut new_fields = Vec::new();
                        for field in fields {
                            new_fields.push(StructFieldInit {
                                name: field.name.clone(),
                                value: self.substitute_expr(&field.value, type_map)?,
                            });
                        }
                        EnumVariantFields::Struct(new_fields)
                    }
                    EnumVariantFields::Unit => EnumVariantFields::Unit,
                };
                Ok(Expression::EnumVariant(EnumVariantExpr {
                    enum_name: variant.enum_name.clone(),
                    variant: variant.variant.clone(),
                    fields: new_fields,
                    span: variant.span.clone(),
                }))
            }
            // リテラルはそのまま返す
            Expression::Integer(_) | Expression::Float(_) | Expression::String(_) | 
            Expression::Boolean(_) | Expression::Identifier(_) | Expression::Path(_) |
            Expression::TemplateString(_) => Ok(expr.clone()),
        }
    }
    
    /// 呼び出し箇所を単相化バージョンに置き換え
    fn replace_generic_calls(&self, program: &mut Program) -> YuniResult<()> {
        for item in &mut program.items {
            match item {
                Item::Function(func) => {
                    func.body = self.replace_calls_in_block(&func.body)?;
                }
                Item::Method(method) => {
                    method.body = self.replace_calls_in_block(&method.body)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    /// ブロック内の呼び出しを置き換え
    fn replace_calls_in_block(&self, block: &Block) -> YuniResult<Block> {
        let mut new_statements = Vec::new();
        for stmt in &block.statements {
            new_statements.push(self.replace_calls_in_statement(stmt)?);
        }
        Ok(Block {
            statements: new_statements,
            span: block.span.clone(),
        })
    }
    
    /// 文内の呼び出しを置き換え
    fn replace_calls_in_statement(&self, stmt: &Statement) -> YuniResult<Statement> {
        match stmt {
            Statement::Let(let_stmt) => {
                let new_init = let_stmt.init.as_ref()
                    .map(|init| self.replace_calls_in_expr(init))
                    .transpose()?;
                Ok(Statement::Let(LetStatement {
                    pattern: let_stmt.pattern.clone(),
                    ty: let_stmt.ty.clone(),
                    init: new_init,
                    span: let_stmt.span.clone(),
                }))
            }
            Statement::Expression(expr) => {
                Ok(Statement::Expression(self.replace_calls_in_expr(expr)?))
            }
            Statement::Assignment(assign) => {
                Ok(Statement::Assignment(AssignStatement {
                    target: self.replace_calls_in_expr(&assign.target)?,
                    value: self.replace_calls_in_expr(&assign.value)?,
                    span: assign.span.clone(),
                }))
            }
            Statement::Return(ret_stmt) => {
                let new_value = ret_stmt.value.as_ref()
                    .map(|value| self.replace_calls_in_expr(value))
                    .transpose()?;
                Ok(Statement::Return(ReturnStatement {
                    value: new_value,
                    span: ret_stmt.span.clone(),
                }))
            }
            Statement::If(if_stmt) => {
                let new_condition = self.replace_calls_in_expr(&if_stmt.condition)?;
                let new_then = self.replace_calls_in_block(&if_stmt.then_branch)?;
                let new_else = match &if_stmt.else_branch {
                    Some(ElseBranch::Block(block)) => 
                        Some(ElseBranch::Block(self.replace_calls_in_block(block)?)),
                    Some(ElseBranch::If(if_stmt)) => {
                        if let Statement::If(new_if) = self.replace_calls_in_statement(
                            &Statement::If(*if_stmt.clone()))? {
                            Some(ElseBranch::If(Box::new(new_if)))
                        } else {
                            None
                        }
                    }
                    None => None,
                };
                Ok(Statement::If(IfStatement {
                    condition: new_condition,
                    then_branch: new_then,
                    else_branch: new_else,
                    span: if_stmt.span.clone(),
                }))
            }
            Statement::While(while_stmt) => {
                Ok(Statement::While(WhileStatement {
                    condition: self.replace_calls_in_expr(&while_stmt.condition)?,
                    body: self.replace_calls_in_block(&while_stmt.body)?,
                    span: while_stmt.span.clone(),
                }))
            }
            Statement::For(for_stmt) => {
                let new_init = match for_stmt.init.as_ref() {
                    Some(init) => Some(Box::new(self.replace_calls_in_statement(init)?)),
                    None => None,
                };
                let new_condition = for_stmt.condition.as_ref()
                    .map(|cond| self.replace_calls_in_expr(cond))
                    .transpose()?;
                let new_update = for_stmt.update.as_ref()
                    .map(|update| self.replace_calls_in_expr(update))
                    .transpose()?;
                Ok(Statement::For(ForStatement {
                    init: new_init,
                    condition: new_condition,
                    update: new_update,
                    body: self.replace_calls_in_block(&for_stmt.body)?,
                    span: for_stmt.span.clone(),
                }))
            }
            Statement::Block(block) => {
                Ok(Statement::Block(self.replace_calls_in_block(block)?))
            }
        }
    }
    
    /// 式内の呼び出しを置き換え
    fn replace_calls_in_expr(&self, expr: &Expression) -> YuniResult<Expression> {
        match expr {
            Expression::Call(call) => {
                if let Expression::Identifier(ident) = &*call.callee {
                    // ジェネリック関数の呼び出しかチェック
                    if self.generic_functions.contains_key(&ident.name) {
                        // 型引数を推論
                        let type_args = self.infer_type_args_from_call(&ident.name, &call.args)?;
                        if !type_args.is_empty() {
                            // マングルされた名前に置き換え
                            let mangled_name = self.mangle_function_name(&ident.name, &type_args);
                            return Ok(Expression::Call(CallExpr {
                                callee: Box::new(Expression::Identifier(Identifier {
                                    name: mangled_name,
                                    span: ident.span.clone(),
                                })),
                                args: call.args.clone(),
                                span: call.span.clone(),
                                is_tail: call.is_tail,
                            }));
                        }
                    }
                }
                
                // 引数も再帰的に処理
                let mut new_args = Vec::new();
                for arg in &call.args {
                    new_args.push(self.replace_calls_in_expr(arg)?);
                }
                Ok(Expression::Call(CallExpr {
                    callee: call.callee.clone(),
                    args: new_args,
                    span: call.span.clone(),
                    is_tail: call.is_tail,
                }))
            }
            Expression::StructLit(struct_lit) => {
                // ジェネリック構造体のインスタンス化かチェック
                if self.generic_structs.contains_key(&struct_lit.name) {
                    // 型引数を推論
                    let type_args = self.infer_type_args_from_struct_lit(struct_lit)?;
                    if !type_args.is_empty() {
                        // マングルされた名前に置き換え
                        let mangled_name = self.mangle_struct_name(&struct_lit.name, &type_args);
                        let mut new_fields = Vec::new();
                        for field in &struct_lit.fields {
                            new_fields.push(StructFieldInit {
                                name: field.name.clone(),
                                value: self.replace_calls_in_expr(&field.value)?,
                            });
                        }
                        return Ok(Expression::StructLit(StructLiteral {
                            name: mangled_name,
                            fields: new_fields,
                            span: struct_lit.span.clone(),
                        }));
                    }
                }
                
                // フィールドの値も再帰的に処理
                let mut new_fields = Vec::new();
                for field in &struct_lit.fields {
                    new_fields.push(StructFieldInit {
                        name: field.name.clone(),
                        value: self.replace_calls_in_expr(&field.value)?,
                    });
                }
                Ok(Expression::StructLit(StructLiteral {
                    name: struct_lit.name.clone(),
                    fields: new_fields,
                    span: struct_lit.span.clone(),
                }))
            }
            Expression::Binary(binary) => {
                let new_left = self.replace_calls_in_expr(&binary.left)?;
                let new_right = self.replace_calls_in_expr(&binary.right)?;
                Ok(Expression::Binary(BinaryExpr {
                    left: Box::new(new_left),
                    op: binary.op.clone(),
                    right: Box::new(new_right),
                    span: binary.span.clone(),
                }))
            }
            Expression::Block(block) => {
                let mut new_statements = Vec::new();
                for stmt in &block.statements {
                    new_statements.push(self.replace_calls_in_statement(stmt)?);
                }
                let new_last_expr = match block.last_expr.as_ref() {
                    Some(expr) => Some(Box::new(self.replace_calls_in_expr(expr)?)),
                    None => None,
                };
                Ok(Expression::Block(BlockExpr {
                    statements: new_statements,
                    last_expr: new_last_expr,
                    span: block.span.clone(),
                }))
            }
            Expression::Field(field) => {
                let new_object = self.replace_calls_in_expr(&field.object)?;
                Ok(Expression::Field(FieldExpr {
                    object: Box::new(new_object),
                    field: field.field.clone(),
                    span: field.span.clone(),
                }))
            }
            Expression::If(if_expr) => {
                let new_condition = Box::new(self.replace_calls_in_expr(&if_expr.condition)?);
                let new_then = Box::new(self.replace_calls_in_expr(&if_expr.then_branch)?);
                let new_else = match if_expr.else_branch.as_ref() {
                    Some(else_branch) => Some(Box::new(self.replace_calls_in_expr(else_branch)?)),
                    None => None,
                };
                Ok(Expression::If(IfExpr {
                    condition: new_condition,
                    then_branch: new_then,
                    else_branch: new_else,
                    span: if_expr.span.clone(),
                }))
            }
            Expression::Match(match_expr) => {
                let new_expr = Box::new(self.replace_calls_in_expr(&match_expr.expr)?);
                let mut new_arms = Vec::new();
                for arm in &match_expr.arms {
                    new_arms.push(MatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref()
                            .map(|guard| self.replace_calls_in_expr(guard))
                            .transpose()?,
                        expr: self.replace_calls_in_expr(&arm.expr)?,
                    });
                }
                Ok(Expression::Match(MatchExpr {
                    expr: new_expr,
                    arms: new_arms,
                    span: match_expr.span.clone(),
                }))
            }
            // リテラルなどはそのまま
            _ => Ok(expr.clone()),
        }
    }
    
    /// 型を文字列に変換（マングリング用）
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::I8 => "i8".to_string(),
            Type::I16 => "i16".to_string(),
            Type::I32 => "i32".to_string(),
            Type::I64 => "i64".to_string(),
            Type::U8 => "u8".to_string(),
            Type::U16 => "u16".to_string(),
            Type::U32 => "u32".to_string(),
            Type::U64 => "u64".to_string(),
            Type::F32 => "f32".to_string(),
            Type::F64 => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "string".to_string(),
            Type::Str => "str".to_string(),
            Type::UserDefined(name) => name.clone(),
            Type::Array(elem) => format!("array_{}", self.type_to_string(elem)),
            Type::Reference(inner, is_mut) => {
                if *is_mut {
                    format!("mut_ref_{}", self.type_to_string(inner))
                } else {
                    format!("ref_{}", self.type_to_string(inner))
                }
            }
            Type::Generic(name, args) => {
                let arg_strs: Vec<String> = args.iter()
                    .map(|arg| self.type_to_string(arg))
                    .collect();
                format!("{}_{}", name, arg_strs.join("_"))
            }
            _ => "unknown".to_string(),
        }
    }
    
    /// 関数名をマングル
    fn mangle_function_name(&self, name: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            name.to_string()
        } else {
            let type_names: Vec<String> = type_args.iter()
                .map(|ty| self.type_to_string(ty))
                .collect();
            format!("{}_{}", name, type_names.join("_"))
        }
    }
    
    /// 構造体名をマングル
    fn mangle_struct_name(&self, name: &str, type_args: &[Type]) -> String {
        if type_args.is_empty() {
            name.to_string()
        } else {
            let type_names: Vec<String> = type_args.iter()
                .map(|ty| self.type_to_string(ty))
                .collect();
            format!("{}_{}", name, type_names.join("_"))
        }
    }
}

/// プログラムを単相化する
pub fn monomorphize_program(program: Program) -> YuniResult<Program> {
    let monomorphizer = Monomorphizer::new(program);
    monomorphizer.monomorphize()
}