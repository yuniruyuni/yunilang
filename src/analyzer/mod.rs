//! Semantic analysis module for the Yuni language.
//!
//! This module performs type checking, name resolution, lifetime analysis,
//! and other semantic validations.

use crate::ast::*;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum AnalysisError {
    #[error("Undefined variable: {name} at {span:?}")]
    UndefinedVariable { name: String, span: Span },

    #[error("Undefined type: {name} at {span:?}")]
    UndefinedType { name: String, span: Span },

    #[error("Undefined function: {name} at {span:?}")]
    UndefinedFunction { name: String, span: Span },

    #[error("Type mismatch: expected {expected}, found {found} at {span:?}")]
    TypeMismatch {
        expected: String,
        found: String,
        span: Span,
    },

    #[error("Function {name} already defined at {span:?}")]
    DuplicateFunction { name: String, span: Span },

    #[error("Type {name} already defined at {span:?}")]
    DuplicateType { name: String, span: Span },

    #[error("Variable {name} already defined in this scope at {span:?}")]
    DuplicateVariable { name: String, span: Span },

    #[error("Cannot infer type for {name} at {span:?}")]
    TypeInferenceError { name: String, span: Span },

    #[error("Invalid operation: {message} at {span:?}")]
    InvalidOperation { message: String, span: Span },

    #[error("Cannot mutate immutable variable {name} at {span:?}")]
    ImmutableVariable { name: String, span: Span },

    #[error("Missing return statement in function {name} at {span:?}")]
    MissingReturn { name: String, span: Span },

    #[error("Lifetime constraint violation: {message} at {span:?}")]
    LifetimeError { message: String, span: Span },

    #[error("Wrong number of arguments: expected {expected}, found {found} at {span:?}")]
    ArgumentCountMismatch {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("Method {method} not found for type {ty} at {span:?}")]
    MethodNotFound {
        method: String,
        ty: String,
        span: Span,
    },

    #[error("Cannot take reference of temporary value at {span:?}")]
    TemporaryReference { span: Span },

    #[error("Pattern matching not exhaustive at {span:?}")]
    NonExhaustivePattern { span: Span },
}

pub type AnalysisResult<T> = Result<T, AnalysisError>;

/// Symbol information stored in symbol tables
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub is_mutable: bool,
    pub span: Span,
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
}

/// Lifetime information for a reference
#[derive(Debug, Clone)]
struct _Lifetime {
    _name: String,
    _outlives: HashSet<String>,
}

/// Context for lifetime analysis
#[derive(Debug)]
struct LifetimeContext {
    _lifetimes: HashMap<String, _Lifetime>,
    constraints: Vec<LivesConstraint>,
}

impl LifetimeContext {
    fn new() -> Self {
        Self {
            _lifetimes: HashMap::new(),
            constraints: Vec::new(),
        }
    }

    fn add_constraint(&mut self, constraint: LivesConstraint) {
        self.constraints.push(constraint);
    }

    fn validate(&self) -> AnalysisResult<()> {
        // TODO: Implement full lifetime validation
        // For now, we'll do basic checks
        for constraint in &self.constraints {
            // Check that all referenced lifetimes exist
            if !self._lifetimes.contains_key(&constraint.target) {
                return Err(AnalysisError::LifetimeError {
                    message: format!("Unknown lifetime: {}", constraint.target),
                    span: constraint.span,
                });
            }
            for source in &constraint.sources {
                if !self._lifetimes.contains_key(source) {
                    return Err(AnalysisError::LifetimeError {
                        message: format!("Unknown lifetime: {}", source),
                        span: constraint.span,
                    });
                }
            }
        }
        Ok(())
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
        };
        self.current_scope.define(receiver_symbol)?;

        // Add parameters to scope
        for param in &method.params {
            let symbol = Symbol {
                name: param.name.clone(),
                ty: param.ty.clone(),
                is_mutable: false,
                span: param.span,
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
            Some(self.analyze_expression(init)?)
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

    fn analyze_identifier(&self, id: &Identifier) -> AnalysisResult<Type> {
        let symbol = self.current_scope.lookup(&id.name).ok_or_else(|| {
            AnalysisError::UndefinedVariable {
                name: id.name.clone(),
                span: id.span,
            }
        })?;
        Ok(symbol.ty.clone())
    }

    fn analyze_path(&self, path: &PathExpr) -> AnalysisResult<Type> {
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
            Expression::Identifier(_) | Expression::Field(_) | Expression::Index(_) => {
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
    }

    fn exit_scope(&mut self) {
        if let Some(parent) = self.current_scope.parent.take() {
            self.current_scope = *parent;
        }
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
}
