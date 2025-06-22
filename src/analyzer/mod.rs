//! Semantic analysis module for the Yuni language.
//!
//! This module performs type checking, name resolution, lifetime analysis,
//! and other semantic validations.

use crate::ast::*;
use crate::error::{AnalyzerError, ErrorCollector, YuniError, YuniResult};
use std::collections::{HashMap, HashSet};

// 既存のAnalysisError型を互換性のために残す
pub type AnalysisError = AnalyzerError;


pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// Symbol information stored in symbol tables
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub is_mutable: bool,
    pub span: Span,
    /// 変数が借用されているかどうか
    pub borrow_info: Option<BorrowInfo>,
    /// 変数が移動されたかどうか
    pub is_moved: bool,
    /// 変数のライフタイム（参照の場合）
    pub lifetime: Option<LifetimeId>,
}

/// Function signature information
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub lives_clause: Option<LivesClause>,
    pub is_method: bool,
    pub receiver_type: Option<Type>,
    pub span: Span,
}

/// Type definition information
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub name: String,
    pub kind: TypeKind,
    pub methods: HashMap<String, FunctionSignature>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypeKind {
    Struct(Vec<Field>),
    Enum(Vec<Variant>),
    Builtin,
}

/// Scope for variable bindings
#[derive(Debug)]
struct Scope {
    symbols: HashMap<String, Symbol>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            parent: None,
        }
    }

    fn with_parent(parent: Scope) -> Self {
        Self {
            symbols: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    fn define(&mut self, symbol: Symbol) -> AnalysisResult<()> {
        if self.symbols.contains_key(&symbol.name) {
            return Err(AnalysisError::DuplicateVariable {
                name: symbol.name.clone(),
                span: symbol.span,
            });
        }
        self.symbols.insert(symbol.name.clone(), symbol);
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbols
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(name)))
    }

    fn _lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        if self.symbols.contains_key(name) {
            self.symbols.get_mut(name)
        } else {
            self.parent
                .as_mut()
                .and_then(|parent| parent._lookup_mut(name))
        }
    }
    
    /// 変数を移動済みとしてマーク
    fn mark_moved(&mut self, name: &str) -> bool {
        if let Some(symbol) = self.symbols.get_mut(name) {
            symbol.is_moved = true;
            true
        } else if let Some(parent) = &mut self.parent {
            parent.mark_moved(name)
        } else {
            false
        }
    }
    
    /// 変数の借用情報を更新
    fn update_borrow_info(&mut self, name: &str, borrow_info: BorrowInfo) -> bool {
        if let Some(symbol) = self.symbols.get_mut(name) {
            symbol.borrow_info = Some(borrow_info);
            true
        } else if let Some(parent) = &mut self.parent {
            parent.update_borrow_info(name, borrow_info)
        } else {
            false
        }
    }
}

/// ライフタイム識別子の種類
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LifetimeId {
    /// 名前付きライフタイム（'a, 'bなど）
    Named(String),
    /// 無名ライフタイム（自動生成）
    Anonymous(usize),
    /// 静的ライフタイム（'static）
    Static,
    /// 不明なライフタイム（推論中）
    Unknown,
}

/// ライフタイムの情報
#[derive(Debug, Clone)]
struct Lifetime {
    /// ライフタイムID
    id: LifetimeId,
    /// このライフタイムが依存するライフタイム（このライフタイムより長く生きる必要がある）
    outlives: HashSet<LifetimeId>,
    /// このライフタイムのスコープ開始位置
    start_scope: ScopeId,
    /// このライフタイムのスコープ終了位置
    end_scope: Option<ScopeId>,
    /// ライフタイムが定義された場所
    span: Span,
}

/// スコープID（ネストしたスコープを管理）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ScopeId(usize);

/// 変数の借用情報
#[derive(Debug, Clone)]
struct BorrowInfo {
    /// 借用の種類
    kind: BorrowKind,
    /// 借用されているライフタイム
    lifetime: LifetimeId,
    /// 借用が発生した場所
    span: Span,
}

/// 借用の種類
#[derive(Debug, Clone, PartialEq)]
enum BorrowKind {
    /// 不変借用
    Immutable,
    /// 可変借用
    Mutable,
    /// 所有権の移動
    Move,
}

/// 変数の使用情報
#[derive(Debug, Clone)]
struct VariableUsage {
    /// 使用の種類
    usage_kind: UsageKind,
    /// 使用された場所
    span: Span,
    /// 使用されたスコープ
    scope: ScopeId,
}

/// 変数の使用種類
#[derive(Debug, Clone, PartialEq)]
enum UsageKind {
    /// 読み取り
    Read,
    /// 書き込み
    Write,
    /// 借用
    Borrow(BorrowKind),
    /// 移動
    Move,
}

/// ライフタイム分析のコンテキスト
#[derive(Debug)]
struct LifetimeContext {
    /// 全てのライフタイム
    lifetimes: HashMap<LifetimeId, Lifetime>,
    /// ライフタイム制約
    constraints: Vec<LivesConstraint>,
    /// 現在のスコープID
    current_scope: ScopeId,
    /// スコープの階層構造（parent scope mapping）
    scope_hierarchy: HashMap<ScopeId, Option<ScopeId>>,
    /// 次のスコープID
    next_scope_id: usize,
    /// 次の無名ライフタイムID
    next_anonymous_id: usize,
    /// 変数の借用情報
    variable_borrows: HashMap<String, Vec<BorrowInfo>>,
    /// 変数の使用履歴
    variable_usage: HashMap<String, Vec<VariableUsage>>,
}

impl LifetimeContext {
    fn new() -> Self {
        let mut ctx = Self {
            lifetimes: HashMap::new(),
            constraints: Vec::new(),
            current_scope: ScopeId(0),
            scope_hierarchy: HashMap::new(),
            next_scope_id: 1,
            next_anonymous_id: 0,
            variable_borrows: HashMap::new(),
            variable_usage: HashMap::new(),
        };
        
        // 静的ライフタイムを登録
        ctx.register_static_lifetime();
        
        ctx
    }
    
    /// 静的ライフタイムを登録
    fn register_static_lifetime(&mut self) {
        let static_lifetime = Lifetime {
            id: LifetimeId::Static,
            outlives: HashSet::new(),
            start_scope: ScopeId(0),
            end_scope: None, // 静的ライフタイムは終了しない
            span: Span::dummy(),
        };
        self.lifetimes.insert(LifetimeId::Static, static_lifetime);
    }
    
    /// 新しいスコープを開始
    fn enter_scope(&mut self) -> ScopeId {
        let new_scope = ScopeId(self.next_scope_id);
        self.next_scope_id += 1;
        
        // 親スコープを記録
        self.scope_hierarchy.insert(new_scope, Some(self.current_scope));
        self.current_scope = new_scope;
        
        new_scope
    }
    
    /// スコープを終了
    fn exit_scope(&mut self) {
        if let Some(parent) = self.scope_hierarchy.get(&self.current_scope).cloned().flatten() {
            self.current_scope = parent;
        }
    }
    
    /// 新しい無名ライフタイムを生成
    fn create_anonymous_lifetime(&mut self, span: Span) -> LifetimeId {
        let id = LifetimeId::Anonymous(self.next_anonymous_id);
        self.next_anonymous_id += 1;
        
        let lifetime = Lifetime {
            id: id.clone(),
            outlives: HashSet::new(),
            start_scope: self.current_scope,
            end_scope: None,
            span,
        };
        
        self.lifetimes.insert(id.clone(), lifetime);
        id
    }
    
    /// 名前付きライフタイムを登録
    fn register_named_lifetime(&mut self, name: String, span: Span) -> AnalysisResult<LifetimeId> {
        let id = LifetimeId::Named(name.clone());
        
        if self.lifetimes.contains_key(&id) {
            return Err(AnalysisError::LifetimeError {
                message: format!("ライフタイム '{}' は既に定義されています", name),
                span,
            });
        }
        
        let lifetime = Lifetime {
            id: id.clone(),
            outlives: HashSet::new(),
            start_scope: self.current_scope,
            end_scope: None,
            span,
        };
        
        self.lifetimes.insert(id.clone(), lifetime);
        Ok(id)
    }
    
    /// ライフタイム制約を追加
    fn add_constraint(&mut self, constraint: LivesConstraint) {
        self.constraints.push(constraint);
    }
    
    /// ライフタイムの依存関係を追加（'a: 'b means 'a outlives 'b）
    fn add_outlives_constraint(&mut self, longer: LifetimeId, shorter: LifetimeId) {
        if let Some(lifetime) = self.lifetimes.get_mut(&shorter) {
            lifetime.outlives.insert(longer);
        }
    }
    
    /// 変数の借用を記録
    fn record_borrow(&mut self, var_name: String, kind: BorrowKind, lifetime: LifetimeId, span: Span) {
        let borrow_info = BorrowInfo {
            kind,
            lifetime,
            span,
        };
        
        self.variable_borrows.entry(var_name).or_insert_with(Vec::new).push(borrow_info);
    }
    
    /// 変数の使用を記録
    fn record_usage(&mut self, var_name: String, usage_kind: UsageKind, span: Span) {
        let usage = VariableUsage {
            usage_kind,
            span,
            scope: self.current_scope,
        };
        
        self.variable_usage.entry(var_name).or_insert_with(Vec::new).push(usage);
    }
    
    /// 借用の競合をチェック
    fn check_borrow_conflicts(&self, var_name: &str) -> AnalysisResult<()> {
        if let Some(borrows) = self.variable_borrows.get(var_name) {
            // 可変借用と他の借用が同時に存在しないかチェック
            let mut mutable_borrows = Vec::new();
            let mut immutable_borrows = Vec::new();
            
            for borrow in borrows {
                match borrow.kind {
                    BorrowKind::Mutable => mutable_borrows.push(borrow),
                    BorrowKind::Immutable => immutable_borrows.push(borrow),
                    BorrowKind::Move => {} // 移動は別途チェック
                }
            }
            
            // 可変借用は他の借用と同時に存在できない
            if mutable_borrows.len() > 1 {
                let first_mut = &mutable_borrows[0];
                return Err(AnalysisError::MultipleMutableBorrows {
                    name: var_name.to_string(),
                    span: first_mut.span,
                });
            } else if !mutable_borrows.is_empty() && !immutable_borrows.is_empty() {
                let first_mut = &mutable_borrows[0];
                return Err(AnalysisError::MutableBorrowConflict {
                    name: var_name.to_string(),
                    span: first_mut.span,
                });
            }
        }
        
        Ok(())
    }
    
    /// ライフタイム制約の検証を実行
    fn validate(&self) -> AnalysisResult<()> {
        // 1. 全ての参照されるライフタイムが存在することを確認
        for constraint in &self.constraints {
            let target_id = LifetimeId::Named(constraint.target.clone());
            if !self.lifetimes.contains_key(&target_id) {
                return Err(AnalysisError::LifetimeError {
                    message: format!("未定義のライフタイム: '{}'", constraint.target),
                    span: constraint.span,
                });
            }
            
            for source in &constraint.sources {
                let source_id = LifetimeId::Named(source.clone());
                if !self.lifetimes.contains_key(&source_id) {
                    return Err(AnalysisError::LifetimeError {
                        message: format!("未定義のライフタイム: '{}'", source),
                        span: constraint.span,
                    });
                }
            }
        }
        
        // 2. ライフタイムの循環依存をチェック
        self.check_lifetime_cycles()?;
        
        // 3. 全ての変数に対して借用の競合をチェック
        for var_name in self.variable_borrows.keys() {
            self.check_borrow_conflicts(var_name)?;
        }
        
        // 4. ライフタイムの依存関係が満たされているかチェック
        self.validate_lifetime_dependencies()?;
        
        Ok(())
    }
    
    /// ライフタイムの循環依存をチェック
    fn check_lifetime_cycles(&self) -> AnalysisResult<()> {
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();
        
        for lifetime_id in self.lifetimes.keys() {
            if !visited.contains(lifetime_id) {
                self.check_cycle_dfs(lifetime_id, &mut visited, &mut recursion_stack)?;
            }
        }
        
        Ok(())
    }
    
    /// DFSによる循環検出
    fn check_cycle_dfs(
        &self,
        current: &LifetimeId,
        visited: &mut HashSet<LifetimeId>,
        recursion_stack: &mut HashSet<LifetimeId>,
    ) -> AnalysisResult<()> {
        visited.insert(current.clone());
        recursion_stack.insert(current.clone());
        
        if let Some(lifetime) = self.lifetimes.get(current) {
            for dependency in &lifetime.outlives {
                if !visited.contains(dependency) {
                    self.check_cycle_dfs(dependency, visited, recursion_stack)?;
                } else if recursion_stack.contains(dependency) {
                    return Err(AnalysisError::LifetimeError {
                        message: format!(
                            "ライフタイムの循環依存が検出されました: {:?} -> {:?}",
                            current, dependency
                        ),
                        span: lifetime.span,
                    });
                }
            }
        }
        
        recursion_stack.remove(current);
        Ok(())
    }
    
    /// ライフタイムの依存関係の検証
    fn validate_lifetime_dependencies(&self) -> AnalysisResult<()> {
        for (lifetime_id, lifetime) in &self.lifetimes {
            for dependency in &lifetime.outlives {
                if !self.lifetime_outlives(dependency, lifetime_id) {
                    return Err(AnalysisError::LifetimeError {
                        message: format!(
                            "ライフタイム制約違反: {:?} は {:?} より長く生きる必要があります",
                            dependency, lifetime_id
                        ),
                        span: lifetime.span,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    /// ライフタイムAがライフタイムBより長く生きるかチェック
    fn lifetime_outlives(&self, a: &LifetimeId, b: &LifetimeId) -> bool {
        // 静的ライフタイムは全てより長い
        if a == &LifetimeId::Static {
            return true;
        }
        
        // 同じライフタイムなら満たされる
        if a == b {
            return true;
        }
        
        // スコープベースの比較（簡単な実装）
        if let (Some(lifetime_a), Some(lifetime_b)) = (
            self.lifetimes.get(a),
            self.lifetimes.get(b),
        ) {
            // より外側のスコープ（小さいID）が長く生きる
            return lifetime_a.start_scope.0 <= lifetime_b.start_scope.0;
        }
        
        false
    }
}

/// Main semantic analyzer
pub struct SemanticAnalyzer {
    /// Current variable scope
    current_scope: Scope,
    /// Global function signatures
    functions: HashMap<String, FunctionSignature>,
    /// Global type definitions
    types: HashMap<String, TypeInfo>,
    /// Import aliases
    imports: HashMap<String, String>,
    /// Current function return type (for checking return statements)
    current_return_type: Option<Type>,
    /// Lifetime context for the current function
    lifetime_context: LifetimeContext,
    /// Collected errors
    errors: Vec<AnalysisError>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            current_scope: Scope::new(),
            functions: HashMap::new(),
            types: HashMap::new(),
            imports: HashMap::new(),
            current_return_type: None,
            lifetime_context: LifetimeContext::new(),
            errors: Vec::new(),
        };

        // Register built-in types
        analyzer.register_builtin_types();

        analyzer
    }

    fn register_builtin_types(&mut self) {
        let builtin_types = vec![
            "i8", "i16", "i32", "i64", "i128", "i256", "u8", "u16", "u32", "u64", "u128", "u256",
            "f8", "f16", "f32", "f64", "bool", "String", "void",
        ];

        for type_name in builtin_types {
            self.types.insert(
                type_name.to_string(),
                TypeInfo {
                    name: type_name.to_string(),
                    kind: TypeKind::Builtin,
                    methods: HashMap::new(),
                    span: Span::dummy(),
                },
            );
        }
    }

    pub fn analyze(&mut self, program: &Program) -> AnalysisResult<()> {
        // Process imports
        for import in &program.imports {
            self.process_import(import);
        }

        // First pass: collect all type definitions and function signatures
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

        // Second pass: analyze function and method bodies
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
        if self.types.contains_key(&struct_def.name) {
            return Err(AnalysisError::DuplicateType {
                name: struct_def.name.clone(),
                span: struct_def.span,
            });
        }

        // Validate field types
        for field in &struct_def.fields {
            self.validate_type(&field.ty, field.span)?;
        }

        self.types.insert(
            struct_def.name.clone(),
            TypeInfo {
                name: struct_def.name.clone(),
                kind: TypeKind::Struct(struct_def.fields.clone()),
                methods: HashMap::new(),
                span: struct_def.span,
            },
        );

        Ok(())
    }

    fn collect_enum_definition(&mut self, enum_def: &EnumDef) -> AnalysisResult<()> {
        if self.types.contains_key(&enum_def.name) {
            return Err(AnalysisError::DuplicateType {
                name: enum_def.name.clone(),
                span: enum_def.span,
            });
        }

        // Validate variant field types
        for variant in &enum_def.variants {
            for field in &variant.fields {
                self.validate_type(&field.ty, field.span)?;
            }
        }

        self.types.insert(
            enum_def.name.clone(),
            TypeInfo {
                name: enum_def.name.clone(),
                kind: TypeKind::Enum(enum_def.variants.clone()),
                methods: HashMap::new(),
                span: enum_def.span,
            },
        );

        Ok(())
    }

    fn collect_function_signature(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        if self.functions.contains_key(&func.name) {
            return Err(AnalysisError::DuplicateFunction {
                name: func.name.clone(),
                span: func.span,
            });
        }

        // Validate parameter types
        let mut params = Vec::new();
        for param in &func.params {
            self.validate_type(&param.ty, param.span)?;
            params.push((param.name.clone(), param.ty.clone()));
        }

        // Validate return type
        let return_type = if let Some(ty) = &func.return_type {
            self.validate_type(ty, func.span)?;
            ty.clone()
        } else {
            Type::Void
        };

        self.functions.insert(
            func.name.clone(),
            FunctionSignature {
                name: func.name.clone(),
                params,
                return_type,
                lives_clause: func.lives_clause.clone(),
                is_method: false,
                receiver_type: None,
                span: func.span,
            },
        );

        Ok(())
    }

    fn collect_method_signature(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        // Extract receiver type
        let receiver_type = self.extract_base_type(&method.receiver.ty);
        let type_name = match &receiver_type {
            Type::UserDefined(name) => name.clone(),
            _ => {
                return Err(AnalysisError::InvalidOperation {
                    message: "Method receiver must be a user-defined type".to_string(),
                    span: method.span,
                });
            }
        };

        // Check that the type exists
        if !self.types.contains_key(&type_name) {
            return Err(AnalysisError::UndefinedType {
                name: type_name.clone(),
                span: method.span,
            });
        }

        // Check for duplicate method
        if let Some(type_info) = self.types.get(&type_name) {
            if type_info.methods.contains_key(&method.name) {
                return Err(AnalysisError::DuplicateFunction {
                    name: format!("{}::{}", type_name, method.name),
                    span: method.span,
                });
            }
        }

        // Validate parameter types
        let mut params = Vec::new();
        for param in &method.params {
            self.validate_type(&param.ty, param.span)?;
            params.push((param.name.clone(), param.ty.clone()));
        }

        // Validate return type
        let return_type = if let Some(ty) = &method.return_type {
            self.validate_type(ty, method.span)?;
            ty.clone()
        } else {
            Type::Void
        };

        // Add method to type
        if let Some(type_info) = self.types.get_mut(&type_name) {
            type_info.methods.insert(
                method.name.clone(),
                FunctionSignature {
                    name: method.name.clone(),
                    params,
                    return_type,
                    lives_clause: method.lives_clause.clone(),
                    is_method: true,
                    receiver_type: Some(method.receiver.ty.clone()),
                    span: method.span,
                },
            );
        }

        Ok(())
    }

    fn analyze_function(&mut self, func: &FunctionDecl) -> AnalysisResult<()> {
        self.enter_scope();

        // Set current return type
        self.current_return_type = func.return_type.clone().or(Some(Type::Void));

        // Add parameters to scope
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
            self.current_scope.define(symbol)?;
        }

        // Setup lifetime context
        if let Some(lives_clause) = &func.lives_clause {
            for constraint in &lives_clause.constraints {
                self.lifetime_context.add_constraint(constraint.clone());
            }
        }

        // Analyze function body
        let has_return = self.analyze_block(&func.body)?;

        // Check if function needs a return statement
        if !matches!(self.current_return_type, Some(Type::Void)) && !has_return {
            return Err(AnalysisError::MissingReturn {
                name: func.name.clone(),
                span: func.span,
            });
        }

        // Validate lifetime constraints
        self.lifetime_context.validate()?;

        self.current_return_type = None;
        self.lifetime_context = LifetimeContext::new();
        self.exit_scope();

        Ok(())
    }

    fn analyze_method(&mut self, method: &MethodDecl) -> AnalysisResult<()> {
        self.enter_scope();

        // Set current return type
        self.current_return_type = method.return_type.clone().or(Some(Type::Void));

        // Add receiver to scope
        let default_name = "self".to_string();
        let receiver_name = method.receiver.name.as_ref().unwrap_or(&default_name);
        let receiver_symbol = Symbol {
            name: receiver_name.clone(),
            ty: method.receiver.ty.clone(),
            is_mutable: matches!(&method.receiver.ty, Type::Reference(_, true)),
            span: method.receiver.span,
            borrow_info: None,
            is_moved: false,
            lifetime: None,
        };
        self.current_scope.define(receiver_symbol)?;

        // Add parameters to scope
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
            self.current_scope.define(symbol)?;
        }

        // Setup lifetime context
        if let Some(lives_clause) = &method.lives_clause {
            for constraint in &lives_clause.constraints {
                self.lifetime_context.add_constraint(constraint.clone());
            }
        }

        // Analyze method body
        let has_return = self.analyze_block(&method.body)?;

        // Check if method needs a return statement
        if !matches!(self.current_return_type, Some(Type::Void)) && !has_return {
            return Err(AnalysisError::MissingReturn {
                name: method.name.clone(),
                span: method.span,
            });
        }

        // Validate lifetime constraints
        self.lifetime_context.validate()?;

        self.current_return_type = None;
        self.lifetime_context = LifetimeContext::new();
        self.exit_scope();

        Ok(())
    }

    fn analyze_block(&mut self, block: &Block) -> AnalysisResult<bool> {
        let mut has_return = false;

        for stmt in &block.statements {
            if has_return {
                // Unreachable code after return
                // We could warn about this
            }

            has_return |= self.analyze_statement(stmt)?;
        }

        Ok(has_return)
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> AnalysisResult<bool> {
        match stmt {
            Statement::Let(let_stmt) => {
                self.analyze_let_statement(let_stmt)?;
                Ok(false)
            }
            Statement::Assignment(assign_stmt) => {
                self.analyze_assignment(assign_stmt)?;
                Ok(false)
            }
            Statement::Expression(expr) => {
                self.analyze_expression(expr)?;
                Ok(false)
            }
            Statement::Return(return_stmt) => {
                self.analyze_return_statement(return_stmt)?;
                Ok(true)
            }
            Statement::If(if_stmt) => self.analyze_if_statement(if_stmt),
            Statement::While(while_stmt) => {
                self.analyze_while_statement(while_stmt)?;
                Ok(false)
            }
            Statement::For(for_stmt) => {
                self.analyze_for_statement(for_stmt)?;
                Ok(false)
            }
            Statement::Block(block) => {
                self.enter_scope();
                let has_return = self.analyze_block(block)?;
                self.exit_scope();
                Ok(has_return)
            }
        }
    }

    fn analyze_let_statement(&mut self, let_stmt: &LetStatement) -> AnalysisResult<()> {
        // Analyze initializer expression if present
        let init_type = if let Some(init) = &let_stmt.init {
            let ty = self.analyze_expression(init)?;
            
            // 初期化での移動セマンティクスをチェック
            if let Expression::Identifier(id) = init {
                if !self.is_copy_type(&ty) {
                    self.check_move_semantics(&id.name, id.span)?;
                }
            }
            
            Some(ty)
        } else {
            None
        };

        // Determine the type
        let var_type = if let Some(declared_type) = &let_stmt.ty {
            self.validate_type(declared_type, let_stmt.span)?;

            // Check type compatibility if initializer is present
            if let Some(init_type) = &init_type {
                self.check_type_compatibility(declared_type, init_type, let_stmt.span)?;
            }

            declared_type.clone()
        } else if let Some(init_type) = init_type {
            // Infer type from initializer
            init_type
        } else {
            return Err(AnalysisError::TypeInferenceError {
                name: "variable".to_string(),
                span: let_stmt.span,
            });
        };

        // Process pattern and add variables to scope
        self.process_pattern(&let_stmt.pattern, &var_type, let_stmt.span)?;

        Ok(())
    }

    fn analyze_assignment(&mut self, assign_stmt: &AssignStatement) -> AnalysisResult<()> {
        // 借用チェックを実行
        self.check_assignment_borrow(&assign_stmt.target, &assign_stmt.value, assign_stmt.span)?;
        
        // Check that target is assignable (mutable)
        let target_type = self.analyze_assignable_expression(&assign_stmt.target)?;

        // Analyze value expression
        let value_type = self.analyze_expression(&assign_stmt.value)?;

        // Check type compatibility
        self.check_type_compatibility(&target_type, &value_type, assign_stmt.span)?;

        Ok(())
    }

    fn analyze_return_statement(&mut self, return_stmt: &ReturnStatement) -> AnalysisResult<()> {
        let return_type = if let Some(value) = &return_stmt.value {
            self.analyze_expression(value)?
        } else {
            Type::Void
        };

        let expected_type = self.current_return_type.as_ref().unwrap_or(&Type::Void);
        self.check_type_compatibility(expected_type, &return_type, return_stmt.span)?;

        Ok(())
    }

    fn analyze_if_statement(&mut self, if_stmt: &IfStatement) -> AnalysisResult<bool> {
        // Analyze condition
        let cond_type = self.analyze_expression(&if_stmt.condition)?;
        if !matches!(cond_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_to_string(&cond_type),
                span: if_stmt.span,
            });
        }

        // Analyze then branch
        self.enter_scope();
        let then_returns = self.analyze_block(&if_stmt.then_branch)?;
        self.exit_scope();

        // Analyze else branch if present
        let else_returns = if let Some(else_branch) = &if_stmt.else_branch {
            match else_branch {
                ElseBranch::Block(block) => {
                    self.enter_scope();
                    let returns = self.analyze_block(block)?;
                    self.exit_scope();
                    returns
                }
                ElseBranch::If(if_stmt) => self.analyze_if_statement(if_stmt)?,
            }
        } else {
            false
        };

        // Both branches must return for the if statement to guarantee a return
        Ok(then_returns && else_returns)
    }

    fn analyze_while_statement(&mut self, while_stmt: &WhileStatement) -> AnalysisResult<()> {
        // Analyze condition
        let cond_type = self.analyze_expression(&while_stmt.condition)?;
        if !matches!(cond_type, Type::Bool) {
            return Err(AnalysisError::TypeMismatch {
                expected: "bool".to_string(),
                found: self.type_to_string(&cond_type),
                span: while_stmt.span,
            });
        }

        // Analyze body
        self.enter_scope();
        self.analyze_block(&while_stmt.body)?;
        self.exit_scope();

        Ok(())
    }

    fn analyze_for_statement(&mut self, for_stmt: &ForStatement) -> AnalysisResult<()> {
        self.enter_scope();

        // Analyze init statement if present
        if let Some(init) = &for_stmt.init {
            self.analyze_statement(init)?;
        }

        // Analyze condition if present
        if let Some(condition) = &for_stmt.condition {
            let cond_type = self.analyze_expression(condition)?;
            if !matches!(cond_type, Type::Bool) {
                return Err(AnalysisError::TypeMismatch {
                    expected: "bool".to_string(),
                    found: self.type_to_string(&cond_type),
                    span: for_stmt.span,
                });
            }
        }

        // Analyze update expression if present
        if let Some(update) = &for_stmt.update {
            self.analyze_expression(update)?;
        }

        // Analyze body
        self.analyze_block(&for_stmt.body)?;

        self.exit_scope();
        Ok(())
    }

    fn analyze_expression(&mut self, expr: &Expression) -> AnalysisResult<Type> {
        match expr {
            Expression::Integer(lit) => self.analyze_integer_literal(lit),
            Expression::Float(lit) => self.analyze_float_literal(lit),
            Expression::String(_) => Ok(Type::String),
            Expression::TemplateString(lit) => self.analyze_template_string(lit),
            Expression::Boolean(_) => Ok(Type::Bool),
            Expression::Identifier(id) => self.analyze_identifier(id),
            Expression::Path(path) => self.analyze_path(path),
            Expression::Binary(binary) => self.analyze_binary_expr(binary),
            Expression::Unary(unary) => self.analyze_unary_expr(unary),
            Expression::Call(call) => self.analyze_call_expr(call),
            Expression::MethodCall(method_call) => self.analyze_method_call(method_call),
            Expression::Index(index) => self.analyze_index_expr(index),
            Expression::Field(field) => self.analyze_field_expr(field),
            Expression::Reference(ref_expr) => self.analyze_reference_expr(ref_expr),
            Expression::Dereference(deref) => self.analyze_dereference_expr(deref),
            Expression::StructLit(struct_lit) => self.analyze_struct_literal(struct_lit),
            Expression::EnumVariant(enum_var) => self.analyze_enum_variant(enum_var),
            Expression::Array(array) => self.analyze_array_expr(array),
            Expression::Tuple(tuple) => self.analyze_tuple_expr(tuple),
            Expression::Cast(cast) => self.analyze_cast_expr(cast),
            Expression::Assignment(assign) => self.analyze_assignment_expr(assign),
        }
    }

    fn analyze_assignable_expression(&mut self, expr: &Expression) -> AnalysisResult<Type> {
        match expr {
            Expression::Identifier(id) => {
                let symbol = self.current_scope.lookup(&id.name).ok_or_else(|| {
                    AnalysisError::UndefinedVariable {
                        name: id.name.clone(),
                        span: id.span,
                    }
                })?;

                if !symbol.is_mutable {
                    return Err(AnalysisError::ImmutableVariable {
                        name: id.name.clone(),
                        span: id.span,
                    });
                }

                Ok(symbol.ty.clone())
            }
            Expression::Field(field) => {
                let obj_type = self.analyze_expression(&field.object)?;
                self.get_field_type(&obj_type, &field.field, field.span)
            }
            Expression::Index(index) => {
                let array_type = self.analyze_expression(&index.object)?;
                let index_type = self.analyze_expression(&index.index)?;

                // Check index is integer type
                if !self.is_integer_type(&index_type) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "integer".to_string(),
                        found: self.type_to_string(&index_type),
                        span: index.span,
                    });
                }

                // Get element type
                match array_type {
                    Type::Array(elem_type) => Ok(*elem_type),
                    _ => Err(AnalysisError::InvalidOperation {
                        message: "Cannot index non-array type".to_string(),
                        span: index.span,
                    }),
                }
            }
            Expression::Dereference(deref) => {
                let ref_type = self.analyze_expression(&deref.expr)?;
                match ref_type {
                    Type::Reference(inner, _) => Ok(*inner),
                    _ => Err(AnalysisError::InvalidOperation {
                        message: "Cannot dereference non-reference type".to_string(),
                        span: deref.span,
                    }),
                }
            }
            _ => Err(AnalysisError::InvalidOperation {
                message: "Expression is not assignable".to_string(),
                span: self.get_expression_span(expr),
            }),
        }
    }

    fn analyze_integer_literal(&self, lit: &IntegerLit) -> AnalysisResult<Type> {
        if let Some(suffix) = &lit.suffix {
            match suffix.as_str() {
                "i8" => Ok(Type::I8),
                "i16" => Ok(Type::I16),
                "i32" => Ok(Type::I32),
                "i64" => Ok(Type::I64),
                "i128" => Ok(Type::I128),
                "i256" => Ok(Type::I256),
                "u8" => Ok(Type::U8),
                "u16" => Ok(Type::U16),
                "u32" => Ok(Type::U32),
                "u64" => Ok(Type::U64),
                "u128" => Ok(Type::U128),
                "u256" => Ok(Type::U256),
                _ => Err(AnalysisError::InvalidOperation {
                    message: format!("Unknown integer suffix: {}", suffix),
                    span: lit.span,
                }),
            }
        } else {
            // Default to i64 if no suffix
            Ok(Type::I64)
        }
    }

    fn analyze_float_literal(&self, lit: &FloatLit) -> AnalysisResult<Type> {
        if let Some(suffix) = &lit.suffix {
            match suffix.as_str() {
                "f8" => Ok(Type::F8),
                "f16" => Ok(Type::F16),
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                _ => Err(AnalysisError::InvalidOperation {
                    message: format!("Unknown float suffix: {}", suffix),
                    span: lit.span,
                }),
            }
        } else {
            // Default to f64 if no suffix
            Ok(Type::F64)
        }
    }

    fn analyze_template_string(&mut self, lit: &TemplateStringLit) -> AnalysisResult<Type> {
        // Analyze all interpolated expressions
        for part in &lit.parts {
            if let TemplateStringPart::Interpolation(expr) = part {
                self.analyze_expression(expr)?;
            }
        }
        Ok(Type::String)
    }

    fn analyze_identifier(&mut self, id: &Identifier) -> AnalysisResult<Type> {
        // Check for builtin functions
        if id.name == "println" {
            // println is a variadic builtin function
            // We return a special function type that won't be used for normal type checking
            return Ok(Type::Function(FunctionType {
                params: vec![], // Empty params to indicate variadic
                return_type: Box::new(Type::Void),
            }));
        }

        let (symbol_type, is_moved) = {
            let symbol = self.current_scope.lookup(&id.name).ok_or_else(|| {
                AnalysisError::UndefinedVariable {
                    name: id.name.clone(),
                    span: id.span,
                }
            })?;
            (symbol.ty.clone(), symbol.is_moved)
        };
        
        // 変数の読み取り使用を記録
        self.record_variable_usage(&id.name, UsageKind::Read, id.span);
        
        // 移動されていないかチェック
        if is_moved {
            return Err(AnalysisError::LifetimeError {
                message: format!("変数 '{}' は既に移動されているため使用できません", id.name),
                span: id.span,
            });
        }
        
        Ok(symbol_type)
    }

    fn analyze_path(&mut self, path: &PathExpr) -> AnalysisResult<Type> {
        // For now, we'll treat paths as function references
        // TODO: Support module paths, type paths, etc.
        if path.segments.len() == 1 {
            let name = &path.segments[0];
            if let Some(func_sig) = self.functions.get(name) {
                Ok(Type::Function(FunctionType {
                    params: func_sig.params.iter().map(|(_, ty)| ty.clone()).collect(),
                    return_type: Box::new(func_sig.return_type.clone()),
                }))
            } else {
                Err(AnalysisError::UndefinedFunction {
                    name: name.clone(),
                    span: path.span,
                })
            }
        } else {
            // TODO: Handle module paths
            Err(AnalysisError::InvalidOperation {
                message: "Module paths not yet supported".to_string(),
                span: path.span,
            })
        }
    }

    fn analyze_binary_expr(&mut self, binary: &BinaryExpr) -> AnalysisResult<Type> {
        let left_type = self.analyze_expression(&binary.left)?;
        let right_type = self.analyze_expression(&binary.right)?;

        match binary.op {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Modulo => {
                // Arithmetic operations
                if self.is_numeric_type(&left_type) && self.is_numeric_type(&right_type) {
                    // TODO: Implement numeric type promotion
                    self.check_type_compatibility(&left_type, &right_type, binary.span)?;
                    Ok(left_type)
                } else {
                    Err(AnalysisError::InvalidOperation {
                        message: format!("Cannot apply {:?} to non-numeric types", binary.op),
                        span: binary.span,
                    })
                }
            }
            BinaryOp::Equal | BinaryOp::NotEqual => {
                // Equality comparison
                self.check_type_compatibility(&left_type, &right_type, binary.span)?;
                Ok(Type::Bool)
            }
            BinaryOp::Less | BinaryOp::Greater | BinaryOp::LessEqual | BinaryOp::GreaterEqual => {
                // Ordering comparison
                if self.is_numeric_type(&left_type) && self.is_numeric_type(&right_type) {
                    self.check_type_compatibility(&left_type, &right_type, binary.span)?;
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::InvalidOperation {
                        message: format!("Cannot compare non-numeric types with {:?}", binary.op),
                        span: binary.span,
                    })
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                // Logical operations
                if matches!(left_type, Type::Bool) && matches!(right_type, Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::InvalidOperation {
                        message: format!("Cannot apply {:?} to non-boolean types", binary.op),
                        span: binary.span,
                    })
                }
            }
            BinaryOp::AddAssign
            | BinaryOp::SubtractAssign
            | BinaryOp::MultiplyAssign
            | BinaryOp::DivideAssign
            | BinaryOp::ModuloAssign => {
                // Compound assignment
                let left_type = self.analyze_assignable_expression(&binary.left)?;
                if self.is_numeric_type(&left_type) && self.is_numeric_type(&right_type) {
                    self.check_type_compatibility(&left_type, &right_type, binary.span)?;
                    Ok(Type::Void)
                } else {
                    Err(AnalysisError::InvalidOperation {
                        message: format!("Cannot apply {:?} to non-numeric types", binary.op),
                        span: binary.span,
                    })
                }
            }
        }
    }

    fn analyze_unary_expr(&mut self, unary: &UnaryExpr) -> AnalysisResult<Type> {
        let operand_type = self.analyze_expression(&unary.operand)?;

        match unary.op {
            UnaryOp::Not => {
                if matches!(operand_type, Type::Bool) {
                    Ok(Type::Bool)
                } else {
                    Err(AnalysisError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: self.type_to_string(&operand_type),
                        span: unary.span,
                    })
                }
            }
            UnaryOp::Negate => {
                if self.is_numeric_type(&operand_type) {
                    Ok(operand_type)
                } else {
                    Err(AnalysisError::InvalidOperation {
                        message: "Cannot negate non-numeric type".to_string(),
                        span: unary.span,
                    })
                }
            }
        }
    }

    fn analyze_call_expr(&mut self, call: &CallExpr) -> AnalysisResult<Type> {
        // Check for builtin functions first
        if let Expression::Identifier(id) = &*call.callee {
            if id.name == "println" {
                // println is a builtin that accepts any number of arguments
                // All arguments are automatically converted to strings
                for arg in &call.args {
                    self.analyze_expression(arg)?;
                }
                return Ok(Type::Void);
            }
        }

        // Analyze callee
        let callee_type = self.analyze_expression(&call.callee)?;

        match callee_type {
            Type::Function(func_type) => {
                // Check argument count
                if call.args.len() != func_type.params.len() {
                    return Err(AnalysisError::ArgumentCountMismatch {
                        expected: func_type.params.len(),
                        found: call.args.len(),
                        span: call.span,
                    });
                }

                // Check argument types
                for (_i, (arg, param_type)) in call.args.iter().zip(&func_type.params).enumerate() {
                    let arg_type = self.analyze_expression(arg)?;

                    // Handle automatic reference taking
                    let arg_type = self.auto_ref(&arg_type, param_type);

                    self.check_type_compatibility(param_type, &arg_type, call.span)?;
                    
                    // 引数が値で渡される場合、移動をチェック
                    if !matches!(param_type, Type::Reference(_, _)) {
                        if let Expression::Identifier(id) = arg {
                            // 値渡しの場合、変数が移動される
                            self.check_move_semantics(&id.name, id.span)?;
                        }
                    }
                }

                Ok(*func_type.return_type)
            }
            _ => Err(AnalysisError::InvalidOperation {
                message: "Cannot call non-function type".to_string(),
                span: call.span,
            }),
        }
    }

    fn analyze_method_call(&mut self, method_call: &MethodCallExpr) -> AnalysisResult<Type> {
        // Analyze receiver
        let receiver_type = self.analyze_expression(&method_call.receiver)?;
        let base_type = self.extract_base_type(&receiver_type);

        // Look up method
        let type_name = match &base_type {
            Type::UserDefined(name) => name.clone(),
            _ => {
                return Err(AnalysisError::MethodNotFound {
                    method: method_call.method.clone(),
                    ty: self.type_to_string(&base_type),
                    span: method_call.span,
                });
            }
        };

        // Get method signature (clone to avoid borrow issues)
        let method_sig = {
            let type_info =
                self.types
                    .get(&type_name)
                    .ok_or_else(|| AnalysisError::UndefinedType {
                        name: type_name.clone(),
                        span: method_call.span,
                    })?;

            type_info
                .methods
                .get(&method_call.method)
                .ok_or_else(|| AnalysisError::MethodNotFound {
                    method: method_call.method.clone(),
                    ty: type_name.clone(),
                    span: method_call.span,
                })?
                .clone()
        };

        // Check receiver type compatibility
        if let Some(expected_receiver) = &method_sig.receiver_type {
            let actual_receiver = if self.needs_auto_ref(&receiver_type, expected_receiver) {
                self.auto_ref(&receiver_type, expected_receiver)
            } else {
                receiver_type.clone()
            };

            self.check_type_compatibility(expected_receiver, &actual_receiver, method_call.span)?;
        }

        // Check arguments
        if method_call.args.len() != method_sig.params.len() {
            return Err(AnalysisError::ArgumentCountMismatch {
                expected: method_sig.params.len(),
                found: method_call.args.len(),
                span: method_call.span,
            });
        }

        for (arg, (_, param_type)) in method_call.args.iter().zip(&method_sig.params) {
            let arg_type = self.analyze_expression(arg)?;
            let arg_type = self.auto_ref(&arg_type, param_type);
            self.check_type_compatibility(param_type, &arg_type, method_call.span)?;
        }

        Ok(method_sig.return_type.clone())
    }

    fn analyze_index_expr(&mut self, index: &IndexExpr) -> AnalysisResult<Type> {
        let array_type = self.analyze_expression(&index.object)?;
        let index_type = self.analyze_expression(&index.index)?;

        // Check index is integer type
        if !self.is_integer_type(&index_type) {
            return Err(AnalysisError::TypeMismatch {
                expected: "integer".to_string(),
                found: self.type_to_string(&index_type),
                span: index.span,
            });
        }

        // Get element type
        match array_type {
            Type::Array(elem_type) => Ok(*elem_type),
            _ => Err(AnalysisError::InvalidOperation {
                message: "Cannot index non-array type".to_string(),
                span: index.span,
            }),
        }
    }

    fn analyze_field_expr(&mut self, field: &FieldExpr) -> AnalysisResult<Type> {
        let obj_type = self.analyze_expression(&field.object)?;
        self.get_field_type(&obj_type, &field.field, field.span)
    }

    fn analyze_reference_expr(&mut self, ref_expr: &ReferenceExpr) -> AnalysisResult<Type> {
        let inner_type = self.analyze_expression(&ref_expr.expr)?;

        // Check if we can take a reference
        match &*ref_expr.expr {
            Expression::Identifier(id) => {
                // 借用可能性をチェック
                self.check_borrowability(&id.name, ref_expr.is_mut, ref_expr.span)?;
                
                // 新しいライフタイムを作成
                let lifetime = self.lifetime_context.create_anonymous_lifetime(ref_expr.span);
                
                // 借用を記録
                let borrow_kind = if ref_expr.is_mut {
                    BorrowKind::Mutable
                } else {
                    BorrowKind::Immutable
                };
                
                self.lifetime_context.record_borrow(
                    id.name.clone(),
                    borrow_kind,
                    lifetime.clone(),
                    ref_expr.span,
                );
                
                Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
            }
            Expression::Field(_) | Expression::Index(_) => {
                // フィールドアクセスやインデックスアクセスの借用
                // より詳細な実装が必要だが、基本的な形を提供
                Ok(Type::Reference(Box::new(inner_type), ref_expr.is_mut))
            }
            _ => Err(AnalysisError::TemporaryReference {
                span: ref_expr.span,
            }),
        }
    }

    fn analyze_dereference_expr(&mut self, deref: &DereferenceExpr) -> AnalysisResult<Type> {
        let ref_type = self.analyze_expression(&deref.expr)?;
        match ref_type {
            Type::Reference(inner, _) => Ok(*inner),
            _ => Err(AnalysisError::InvalidOperation {
                message: "Cannot dereference non-reference type".to_string(),
                span: deref.span,
            }),
        }
    }

    fn analyze_struct_literal(&mut self, struct_lit: &StructLit) -> AnalysisResult<Type> {
        // Look up struct type and clone fields to avoid borrow issues
        let fields = {
            let type_info =
                self.types
                    .get(&struct_lit.ty)
                    .ok_or_else(|| AnalysisError::UndefinedType {
                        name: struct_lit.ty.clone(),
                        span: struct_lit.span,
                    })?;

            // Check that it's a struct
            match &type_info.kind {
                TypeKind::Struct(fields) => fields.clone(),
                _ => {
                    return Err(AnalysisError::InvalidOperation {
                        message: format!("{} is not a struct type", struct_lit.ty),
                        span: struct_lit.span,
                    });
                }
            }
        };

        // Check fields
        let mut initialized_fields = HashSet::new();

        for field_init in &struct_lit.fields {
            // Check for duplicate initialization
            if !initialized_fields.insert(&field_init.name) {
                return Err(AnalysisError::InvalidOperation {
                    message: format!("Field {} initialized multiple times", field_init.name),
                    span: field_init.span,
                });
            }

            // Find field definition
            let field_def = fields
                .iter()
                .find(|f| f.name == field_init.name)
                .ok_or_else(|| AnalysisError::InvalidOperation {
                    message: format!("Unknown field: {}", field_init.name),
                    span: field_init.span,
                })?;

            // Check field type
            let init_type = self.analyze_expression(&field_init.value)?;
            self.check_type_compatibility(&field_def.ty, &init_type, field_init.span)?;
        }

        // Check all fields are initialized
        for field in &fields {
            if !initialized_fields.contains(&field.name) {
                return Err(AnalysisError::InvalidOperation {
                    message: format!("Field {} not initialized", field.name),
                    span: struct_lit.span,
                });
            }
        }

        Ok(Type::UserDefined(struct_lit.ty.clone()))
    }

    fn analyze_enum_variant(&mut self, enum_var: &EnumVariantExpr) -> AnalysisResult<Type> {
        // Look up enum type and clone variant to avoid borrow issues
        let variant = {
            let type_info = self.types.get(&enum_var.enum_name).ok_or_else(|| {
                AnalysisError::UndefinedType {
                    name: enum_var.enum_name.clone(),
                    span: enum_var.span,
                }
            })?;

            // Check that it's an enum
            let variants = match &type_info.kind {
                TypeKind::Enum(variants) => variants,
                _ => {
                    return Err(AnalysisError::InvalidOperation {
                        message: format!("{} is not an enum type", enum_var.enum_name),
                        span: enum_var.span,
                    });
                }
            };

            // Find variant
            variants
                .iter()
                .find(|v| v.name == enum_var.variant)
                .ok_or_else(|| AnalysisError::InvalidOperation {
                    message: format!(
                        "Unknown variant: {}::{}",
                        enum_var.enum_name, enum_var.variant
                    ),
                    span: enum_var.span,
                })?
                .clone()
        };

        // Check argument count and types
        if enum_var.args.len() != variant.fields.len() {
            return Err(AnalysisError::ArgumentCountMismatch {
                expected: variant.fields.len(),
                found: enum_var.args.len(),
                span: enum_var.span,
            });
        }

        for (arg, field) in enum_var.args.iter().zip(&variant.fields) {
            let arg_type = self.analyze_expression(arg)?;
            self.check_type_compatibility(&field.ty, &arg_type, enum_var.span)?;
        }

        Ok(Type::UserDefined(enum_var.enum_name.clone()))
    }

    fn analyze_array_expr(&mut self, array: &ArrayExpr) -> AnalysisResult<Type> {
        if array.elements.is_empty() {
            return Err(AnalysisError::TypeInferenceError {
                name: "empty array".to_string(),
                span: array.span,
            });
        }

        // Analyze first element to determine array type
        let elem_type = self.analyze_expression(&array.elements[0])?;

        // Check all elements have the same type
        for elem in &array.elements[1..] {
            let ty = self.analyze_expression(elem)?;
            self.check_type_compatibility(&elem_type, &ty, array.span)?;
        }

        Ok(Type::Array(Box::new(elem_type)))
    }

    fn analyze_tuple_expr(&mut self, tuple: &TupleExpr) -> AnalysisResult<Type> {
        let mut types = Vec::new();

        for elem in &tuple.elements {
            types.push(self.analyze_expression(elem)?);
        }

        Ok(Type::Tuple(types))
    }

    fn analyze_cast_expr(&mut self, cast: &CastExpr) -> AnalysisResult<Type> {
        let expr_type = self.analyze_expression(&cast.expr)?;
        self.validate_type(&cast.ty, cast.span)?;

        // Check if cast is valid
        if !self.is_valid_cast(&expr_type, &cast.ty) {
            return Err(AnalysisError::InvalidOperation {
                message: format!(
                    "Cannot cast {} to {}",
                    self.type_to_string(&expr_type),
                    self.type_to_string(&cast.ty)
                ),
                span: cast.span,
            });
        }

        Ok(cast.ty.clone())
    }

    fn analyze_assignment_expr(&mut self, assign: &AssignmentExpr) -> AnalysisResult<Type> {
        // Analyze the target (must be assignable)
        let target_type = self.analyze_assignable_expression(&assign.target)?;

        // Analyze the value
        let value_type = self.analyze_expression(&assign.value)?;

        // Check type compatibility
        if !self.types_equal(&target_type, &value_type) {
            return Err(AnalysisError::TypeMismatch {
                expected: self.type_to_string(&target_type),
                found: self.type_to_string(&value_type),
                span: self.get_expression_span(&assign.value),
            });
        }

        Ok(target_type)
    }

    fn process_pattern(&mut self, pattern: &Pattern, ty: &Type, span: Span) -> AnalysisResult<()> {
        match pattern {
            Pattern::Identifier(name, is_mut) => {
                let symbol = Symbol {
                    name: name.clone(),
                    ty: ty.clone(),
                    is_mutable: *is_mut,
                    span,
                    borrow_info: None,
                    is_moved: false,
                    lifetime: None,
                };
                self.current_scope.define(symbol)?;
            }
            Pattern::Tuple(patterns) => match ty {
                Type::Tuple(types) => {
                    if patterns.len() != types.len() {
                        return Err(AnalysisError::InvalidOperation {
                            message: "Tuple pattern size mismatch".to_string(),
                            span,
                        });
                    }
                    for (pattern, ty) in patterns.iter().zip(types) {
                        self.process_pattern(pattern, ty, span)?;
                    }
                }
                _ => {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "tuple".to_string(),
                        found: self.type_to_string(ty),
                        span,
                    });
                }
            },
            Pattern::Struct(_struct_name, _field_patterns) => {
                // TODO: Implement struct pattern matching
                return Err(AnalysisError::InvalidOperation {
                    message: "Struct patterns not yet implemented".to_string(),
                    span,
                });
            }
        }
        Ok(())
    }

    fn get_field_type(
        &self,
        obj_type: &Type,
        field_name: &str,
        span: Span,
    ) -> AnalysisResult<Type> {
        let base_type = self.extract_base_type(obj_type);

        match &base_type {
            Type::UserDefined(type_name) => {
                let type_info =
                    self.types
                        .get(type_name)
                        .ok_or_else(|| AnalysisError::UndefinedType {
                            name: type_name.clone(),
                            span,
                        })?;

                match &type_info.kind {
                    TypeKind::Struct(fields) => {
                        let field =
                            fields
                                .iter()
                                .find(|f| f.name == field_name)
                                .ok_or_else(|| AnalysisError::InvalidOperation {
                                    message: format!("Unknown field: {}", field_name),
                                    span,
                                })?;
                        Ok(field.ty.clone())
                    }
                    _ => Err(AnalysisError::InvalidOperation {
                        message: format!("Cannot access field on non-struct type"),
                        span,
                    }),
                }
            }
            _ => Err(AnalysisError::InvalidOperation {
                message: format!("Cannot access field on non-user-defined type"),
                span,
            }),
        }
    }

    fn validate_type(&self, ty: &Type, span: Span) -> AnalysisResult<()> {
        match ty {
            Type::UserDefined(name) => {
                if !self.types.contains_key(name) {
                    return Err(AnalysisError::UndefinedType {
                        name: name.clone(),
                        span,
                    });
                }
            }
            Type::Reference(inner, _) => self.validate_type(inner, span)?,
            Type::Array(elem) => self.validate_type(elem, span)?,
            Type::Tuple(types) => {
                for ty in types {
                    self.validate_type(ty, span)?;
                }
            }
            Type::Function(func_ty) => {
                for param in &func_ty.params {
                    self.validate_type(param, span)?;
                }
                self.validate_type(&func_ty.return_type, span)?;
            }
            _ => {} // Built-in types are always valid
        }
        Ok(())
    }

    fn check_type_compatibility(
        &self,
        expected: &Type,
        actual: &Type,
        span: Span,
    ) -> AnalysisResult<()> {
        if !self.types_equal(expected, actual) {
            return Err(AnalysisError::TypeMismatch {
                expected: self.type_to_string(expected),
                found: self.type_to_string(actual),
                span,
            });
        }
        Ok(())
    }

    fn types_equal(&self, a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::I8, Type::I8)
            | (Type::I16, Type::I16)
            | (Type::I32, Type::I32)
            | (Type::I64, Type::I64)
            | (Type::I128, Type::I128)
            | (Type::I256, Type::I256)
            | (Type::U8, Type::U8)
            | (Type::U16, Type::U16)
            | (Type::U32, Type::U32)
            | (Type::U64, Type::U64)
            | (Type::U128, Type::U128)
            | (Type::U256, Type::U256)
            | (Type::F8, Type::F8)
            | (Type::F16, Type::F16)
            | (Type::F32, Type::F32)
            | (Type::F64, Type::F64)
            | (Type::Bool, Type::Bool)
            | (Type::String, Type::String)
            | (Type::Void, Type::Void) => true,

            (Type::Reference(a_inner, a_mut), Type::Reference(b_inner, b_mut)) => {
                a_mut == b_mut && self.types_equal(a_inner, b_inner)
            }

            (Type::Array(a_elem), Type::Array(b_elem)) => self.types_equal(a_elem, b_elem),

            (Type::Tuple(a_types), Type::Tuple(b_types)) => {
                a_types.len() == b_types.len()
                    && a_types
                        .iter()
                        .zip(b_types)
                        .all(|(a, b)| self.types_equal(a, b))
            }

            (Type::Function(a_func), Type::Function(b_func)) => {
                self.types_equal(&a_func.return_type, &b_func.return_type)
                    && a_func.params.len() == b_func.params.len()
                    && a_func
                        .params
                        .iter()
                        .zip(&b_func.params)
                        .all(|(a, b)| self.types_equal(a, b))
            }

            (Type::UserDefined(a_name), Type::UserDefined(b_name)) => a_name == b_name,

            _ => false,
        }
    }

    fn auto_ref(&self, from: &Type, to: &Type) -> Type {
        match to {
            Type::Reference(_, is_mut) => match from {
                Type::Reference(_, _) => from.clone(),
                _ => Type::Reference(Box::new(from.clone()), *is_mut),
            },
            _ => from.clone(),
        }
    }

    fn needs_auto_ref(&self, from: &Type, to: &Type) -> bool {
        matches!(to, Type::Reference(_, _)) && !matches!(from, Type::Reference(_, _))
    }

    fn extract_base_type(&self, ty: &Type) -> Type {
        match ty {
            Type::Reference(inner, _) => self.extract_base_type(inner),
            _ => ty.clone(),
        }
    }

    fn is_numeric_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::I128
                | Type::I256
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::U128
                | Type::U256
                | Type::F8
                | Type::F16
                | Type::F32
                | Type::F64
        )
    }

    fn is_integer_type(&self, ty: &Type) -> bool {
        matches!(
            ty,
            Type::I8
                | Type::I16
                | Type::I32
                | Type::I64
                | Type::I128
                | Type::I256
                | Type::U8
                | Type::U16
                | Type::U32
                | Type::U64
                | Type::U128
                | Type::U256
        )
    }

    fn is_valid_cast(&self, from: &Type, to: &Type) -> bool {
        // Allow numeric casts
        if self.is_numeric_type(from) && self.is_numeric_type(to) {
            return true;
        }

        // TODO: Add more cast rules
        false
    }

    fn type_to_string(&self, ty: &Type) -> String {
        ty.to_string()
    }

    fn get_expression_span(&self, expr: &Expression) -> Span {
        match expr {
            Expression::Integer(lit) => lit.span,
            Expression::Float(lit) => lit.span,
            Expression::String(lit) => lit.span,
            Expression::TemplateString(lit) => lit.span,
            Expression::Boolean(lit) => lit.span,
            Expression::Identifier(id) => id.span,
            Expression::Path(path) => path.span,
            Expression::Binary(binary) => binary.span,
            Expression::Unary(unary) => unary.span,
            Expression::Call(call) => call.span,
            Expression::MethodCall(method_call) => method_call.span,
            Expression::Index(index) => index.span,
            Expression::Field(field) => field.span,
            Expression::Reference(ref_expr) => ref_expr.span,
            Expression::Dereference(deref) => deref.span,
            Expression::StructLit(struct_lit) => struct_lit.span,
            Expression::EnumVariant(enum_var) => enum_var.span,
            Expression::Array(array) => array.span,
            Expression::Tuple(tuple) => tuple.span,
            Expression::Cast(cast) => cast.span,
            Expression::Assignment(assign) => assign.span,
        }
    }

    fn enter_scope(&mut self) {
        let new_scope =
            Scope::with_parent(std::mem::replace(&mut self.current_scope, Scope::new()));
        self.current_scope = new_scope;
        // ライフタイムコンテキストでもスコープを管理
        self.lifetime_context.enter_scope();
    }

    fn exit_scope(&mut self) {
        if let Some(parent) = self.current_scope.parent.take() {
            self.current_scope = *parent;
        }
        // ライフタイムコンテキストでもスコープを管理
        self.lifetime_context.exit_scope();
    }
    
    /// 借用可能性をチェック
    fn check_borrowability(&mut self, var_name: &str, is_mut: bool, span: Span) -> AnalysisResult<()> {
        // 変数が存在するかチェックし、必要な情報を取得
        let (is_moved, is_mutable) = {
            let symbol = self.current_scope.lookup(var_name).ok_or_else(|| {
                AnalysisError::UndefinedVariable {
                    name: var_name.to_string(),
                    span,
                }
            })?;
            (symbol.is_moved, symbol.is_mutable)
        };
        
        // 既に移動されている変数は借用できない
        if is_moved {
            return Err(AnalysisError::LifetimeError {
                message: format!("変数 '{}' は既に移動されているため借用できません", var_name),
                span,
            });
        }
        
        // 可変借用の場合、変数も可変である必要がある
        if is_mut && !is_mutable {
            return Err(AnalysisError::ImmutableVariable {
                name: var_name.to_string(),
                span,
            });
        }
        
        // 既存の借用との競合をチェック
        self.lifetime_context.check_borrow_conflicts(var_name)?;
        
        Ok(())
    }
    
    /// 変数の使用を記録
    fn record_variable_usage(&mut self, var_name: &str, usage_kind: UsageKind, span: Span) {
        self.lifetime_context.record_usage(var_name.to_string(), usage_kind, span);
    }
    
    /// 変数が移動されたことを記録
    fn mark_variable_moved(&mut self, var_name: &str, span: Span) -> AnalysisResult<()> {
        // 変数が存在するかチェック
        if self.current_scope.lookup(var_name).is_none() {
            return Err(AnalysisError::UndefinedVariable {
                name: var_name.to_string(),
                span,
            });
        }
        
        // 移動の使用を記録
        self.record_variable_usage(var_name, UsageKind::Move, span);
        
        // 変数を移動済みとしてマーク
        self.current_scope.mark_moved(var_name);
        
        Ok(())
    }
    
    /// 移動セマンティクスをチェック
    fn check_move_semantics(&mut self, var_name: &str, span: Span) -> AnalysisResult<()> {
        let (is_moved, symbol_type) = {
            let symbol = self.current_scope.lookup(var_name).ok_or_else(|| {
                AnalysisError::UndefinedVariable {
                    name: var_name.to_string(),
                    span,
                }
            })?;
            (symbol.is_moved, symbol.ty.clone())
        };
        
        // 既に移動されている場合はエラー
        if is_moved {
            return Err(AnalysisError::LifetimeError {
                message: format!("変数 '{}' は既に移動されているため再度移動できません", var_name),
                span,
            });
        }
        
        // Copyトレイトを持つ型かどうかをチェック（簡単な実装）
        if self.is_copy_type(&symbol_type) {
            // Copyできる型は移動ではなくコピーされる
            self.record_variable_usage(var_name, UsageKind::Read, span);
        } else {
            // 非Copyの型は移動される
            self.mark_variable_moved(var_name, span)?;
        }
        
        Ok(())
    }
    
    /// 型がCopyトレイトを持つかどうかを判定
    fn is_copy_type(&self, ty: &Type) -> bool {
        match ty {
            // プリミティブ型はCopy
            Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::I256 |
            Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 |
            Type::F8 | Type::F16 | Type::F32 | Type::F64 | Type::Bool => true,
            
            // 参照はCopy（ただし、参照先ではない）
            Type::Reference(_, _) => true,
            
            // タプルは全ての要素がCopyならCopy
            Type::Tuple(types) => types.iter().all(|t| self.is_copy_type(t)),
            
            // 配列は要素がCopyならCopy（ただし、動的サイズ配列は除く）
            Type::Array(elem_type) => self.is_copy_type(elem_type),
            
            // StringやVoidや関数型、ユーザー定義型は通常非Copy
            _ => false,
        }
    }
    
    /// 代入時の借用チェック
    fn check_assignment_borrow(&mut self, target: &Expression, value: &Expression, span: Span) -> AnalysisResult<()> {
        // 代入先が可変であることを確認
        if let Expression::Identifier(id) = target {
            let is_mutable = {
                let symbol = self.current_scope.lookup(&id.name).ok_or_else(|| {
                    AnalysisError::UndefinedVariable {
                        name: id.name.clone(),
                        span,
                    }
                })?;
                symbol.is_mutable
            };
            
            if !is_mutable {
                return Err(AnalysisError::ImmutableVariable {
                    name: id.name.clone(),
                    span,
                });
            }
            
            // 書き込み使用を記録
            self.record_variable_usage(&id.name, UsageKind::Write, span);
        }
        
        // 代入する値が移動される場合をチェック
        if let Expression::Identifier(id) = value {
            // 値の型を取得して移動セマンティクスをチェック
            let value_type = self.analyze_expression(value)?;
            if !self.is_copy_type(&value_type) {
                self.check_move_semantics(&id.name, span)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_program() {
        let mut analyzer = SemanticAnalyzer::new();
        let program = Program {
            package: PackageDecl {
                name: "test".to_string(),
                span: Span::dummy(),
            },
            imports: vec![],
            items: vec![],
            span: Span::dummy(),
        };
        assert!(analyzer.analyze(&program).is_ok());
    }

    #[test]
    fn test_undefined_variable() {
        let mut analyzer = SemanticAnalyzer::new();
        let program = Program {
            package: PackageDecl {
                name: "test".to_string(),
                span: Span::dummy(),
            },
            imports: vec![],
            items: vec![Item::Function(FunctionDecl {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                lives_clause: None,
                body: Block {
                    statements: vec![Statement::Expression(Expression::Identifier(Identifier {
                        name: "undefined".to_string(),
                        span: Span::new(0, 9),
                    }))],
                    span: Span::dummy(),
                },
                is_public: false,
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };

        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        match result.unwrap_err() {
            AnalysisError::UndefinedVariable { name, .. } => {
                assert_eq!(name, "undefined");
            }
            _ => panic!("Expected UndefinedVariable error"),
        }
    }
    
    #[test]
    fn test_move_semantics() {
        use crate::ast::*;
        
        let mut analyzer = SemanticAnalyzer::new();
        let program = Program {
            package: PackageDecl {
                name: "test".to_string(),
                span: Span::dummy(),
            },
            imports: vec![],
            items: vec![Item::Function(FunctionDecl {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                lives_clause: None,
                body: Block {
                    statements: vec![
                        // let x: String;
                        Statement::Let(LetStatement {
                            pattern: Pattern::Identifier("x".to_string(), false),
                            ty: Some(Type::String),
                            init: Some(Expression::String(StringLit {
                                value: "hello".to_string(),
                                span: Span::dummy(),
                            })),
                            span: Span::dummy(),
                        }),
                        // let y = x; // xを移動
                        Statement::Let(LetStatement {
                            pattern: Pattern::Identifier("y".to_string(), false),
                            ty: None,
                            init: Some(Expression::Identifier(Identifier {
                                name: "x".to_string(),
                                span: Span::new(10, 11),
                            })),
                            span: Span::dummy(),
                        }),
                        // println!(x); // xは既に移動されているのでエラーになるはず
                        Statement::Expression(Expression::Call(CallExpr {
                            callee: Box::new(Expression::Identifier(Identifier {
                                name: "println".to_string(),
                                span: Span::dummy(),
                            })),
                            args: vec![Expression::Identifier(Identifier {
                                name: "x".to_string(),
                                span: Span::new(20, 21),
                            })],
                            span: Span::dummy(),
                        })),
                    ],
                    span: Span::dummy(),
                },
                is_public: false,
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };

        let result = analyzer.analyze(&program);
        assert!(result.is_err());
        match result.unwrap_err() {
            AnalysisError::LifetimeError { message, .. } => {
                assert!(message.contains("移動されているため使用できません"));
            }
            _ => panic!("Expected LifetimeError for use after move"),
        }
    }
    
    #[test]
    fn test_copy_types_no_move() {
        use crate::ast::*;
        
        let mut analyzer = SemanticAnalyzer::new();
        let program = Program {
            package: PackageDecl {
                name: "test".to_string(),
                span: Span::dummy(),
            },
            imports: vec![],
            items: vec![Item::Function(FunctionDecl {
                name: "main".to_string(),
                params: vec![],
                return_type: None,
                lives_clause: None,
                body: Block {
                    statements: vec![
                        // let x = 42; // i64はCopyなので移動しない
                        Statement::Let(LetStatement {
                            pattern: Pattern::Identifier("x".to_string(), false),
                            ty: Some(Type::I64),
                            init: Some(Expression::Integer(IntegerLit {
                                value: 42,
                                suffix: None,
                                span: Span::dummy(),
                            })),
                            span: Span::dummy(),
                        }),
                        // let y = x; // xをコピー
                        Statement::Let(LetStatement {
                            pattern: Pattern::Identifier("y".to_string(), false),
                            ty: None,
                            init: Some(Expression::Identifier(Identifier {
                                name: "x".to_string(),
                                span: Span::dummy(),
                            })),
                            span: Span::dummy(),
                        }),
                        // println!(x); // xはまだ使える
                        Statement::Expression(Expression::Call(CallExpr {
                            callee: Box::new(Expression::Identifier(Identifier {
                                name: "println".to_string(),
                                span: Span::dummy(),
                            })),
                            args: vec![Expression::Identifier(Identifier {
                                name: "x".to_string(),
                                span: Span::dummy(),
                            })],
                            span: Span::dummy(),
                        })),
                    ],
                    span: Span::dummy(),
                },
                is_public: false,
                span: Span::dummy(),
            })],
            span: Span::dummy(),
        };

        let result = analyzer.analyze(&program);
        // Copyできる型の場合、エラーにならないはず
        assert!(result.is_ok(), "Copy types should not cause move errors");
    }
}
