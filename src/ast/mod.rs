//! Abstract Syntax Tree (AST) definitions for the Yuni language.

use serde::{Deserialize, Serialize};

/// Span information for source location tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

/// Root node of the AST representing a complete Yuni program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub package: PackageDecl,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
    pub span: Span,
}

/// Package declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageDecl {
    pub name: String,
    pub span: Span,
}

/// Import statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
    pub span: Span,
}

/// Top-level items in a program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    Function(FunctionDecl),
    Method(MethodDecl),
    TypeDef(TypeDef),
}

/// Type definition (struct or enum)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
}

/// Struct definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// Field in a struct
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// Enum definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

/// Variant in an enum
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

/// Function declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub lives_clause: Option<LivesClause>,
    pub body: Block,
    pub is_public: bool,
    pub span: Span,
}

/// Method declaration (function with receiver)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodDecl {
    pub receiver: Receiver,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub lives_clause: Option<LivesClause>,
    pub body: Block,
    pub is_public: bool,
    pub span: Span,
}

/// Method receiver
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receiver {
    pub ty: Type,
    pub name: Option<String>,
    pub span: Span,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// Lives clause for lifetime constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivesClause {
    pub constraints: Vec<LivesConstraint>,
    pub span: Span,
}

/// Single lifetime constraint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LivesConstraint {
    pub target: String,
    pub sources: Vec<String>,
    pub span: Span,
}

/// Statement types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Let(LetStatement),
    Assignment(AssignStatement),
    Expression(Expression),
    Return(ReturnStatement),
    If(IfStatement),
    While(WhileStatement),
    For(ForStatement),
    Block(Block),
}

/// Let statement for variable declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetStatement {
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub init: Option<Expression>,
    pub span: Span,
}

/// Pattern for destructuring in let statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Identifier(String, bool), // name, is_mut
    Tuple(Vec<Pattern>),
    Struct(String, Vec<(String, Pattern)>),
}

/// Assignment statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignStatement {
    pub target: Expression,
    pub value: Expression,
    pub span: Span,
}

/// Return statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

/// If statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_branch: Block,
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

/// Else branch (can be another if or a block)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStatement>),
}

/// While statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: Block,
    pub span: Span,
}

/// For statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForStatement {
    pub init: Option<Box<Statement>>,
    pub condition: Option<Expression>,
    pub update: Option<Expression>,
    pub body: Block,
    pub span: Span,
}

/// Block statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Expression types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    // Literals
    Integer(IntegerLit),
    Float(FloatLit),
    String(StringLit),
    TemplateString(TemplateStringLit),
    Boolean(BooleanLit),

    // Identifiers and paths
    Identifier(Identifier),
    Path(PathExpr),

    // Operations
    Binary(BinaryExpr),
    Unary(UnaryExpr),

    // Function and method calls
    Call(CallExpr),
    MethodCall(MethodCallExpr),

    // Access expressions
    Index(IndexExpr),
    Field(FieldExpr),

    // Reference and dereference
    Reference(ReferenceExpr),
    Dereference(DereferenceExpr),

    // Struct and enum construction
    StructLit(StructLit),
    EnumVariant(EnumVariantExpr),

    // Array and tuple
    Array(ArrayExpr),
    Tuple(TupleExpr),

    // Type cast
    Cast(CastExpr),

    // Assignment (for expressions like in for loop update)
    Assignment(AssignmentExpr),
}

/// Integer literal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegerLit {
    pub value: i64,
    pub suffix: Option<String>, // e.g., "i32", "u64"
    pub span: Span,
}

/// Float literal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloatLit {
    pub value: f64,
    pub suffix: Option<String>, // e.g., "f32", "f64"
    pub span: Span,
}

/// String literal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringLit {
    pub value: String,
    pub span: Span,
}

/// Template string literal with interpolation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateStringLit {
    pub parts: Vec<TemplateStringPart>,
    pub span: Span,
}

/// Part of a template string
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateStringPart {
    Text(String),
    Interpolation(Box<Expression>),
}

/// Boolean literal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanLit {
    pub value: bool,
    pub span: Span,
}

/// Identifier
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

/// Path expression (e.g., math.sqrt, Point::new)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathExpr {
    pub segments: Vec<String>,
    pub span: Span,
}

/// Binary expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub op: BinaryOp,
    pub right: Box<Expression>,
    pub span: Span,
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // Logical
    And,
    Or,

    // Assignment operators
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModuloAssign,
}

/// Unary expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub operand: Box<Expression>,
    pub span: Span,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Negate,
}

/// Function call expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
    pub span: Span,
}

/// Method call expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodCallExpr {
    pub receiver: Box<Expression>,
    pub method: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

/// Index expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexExpr {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
    pub span: Span,
}

/// Field access expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldExpr {
    pub object: Box<Expression>,
    pub field: String,
    pub span: Span,
}

/// Reference expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceExpr {
    pub is_mut: bool,
    pub expr: Box<Expression>,
    pub span: Span,
}

/// Dereference expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DereferenceExpr {
    pub expr: Box<Expression>,
    pub span: Span,
}

/// Struct literal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructLit {
    pub ty: String,
    pub fields: Vec<FieldInit>,
    pub span: Span,
}

/// Field initialization in struct literal
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldInit {
    pub name: String,
    pub value: Expression,
    pub span: Span,
}

/// Enum variant expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumVariantExpr {
    pub enum_name: String,
    pub variant: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

/// Array expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

/// Tuple expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

/// Type cast expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastExpr {
    pub expr: Box<Expression>,
    pub ty: Type,
    pub span: Span,
}

/// Assignment expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignmentExpr {
    pub target: Box<Expression>,
    pub value: Box<Expression>,
    pub span: Span,
}

/// Type representations
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    // Basic types
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    F8,
    F16,
    F32,
    F64,
    Bool,
    String,
    Void,

    // Reference types
    Reference(Box<Type>, bool), // type, is_mut

    // Compound types
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Function(FunctionType),

    // User-defined types
    UserDefined(String),
}

/// Function type
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

// Visitor pattern for AST traversal
pub trait Visitor<T> {
    fn visit_program(&mut self, program: &Program) -> T;
    fn visit_item(&mut self, item: &Item) -> T;
    fn visit_function(&mut self, func: &FunctionDecl) -> T;
    fn visit_method(&mut self, method: &MethodDecl) -> T;
    fn visit_type_def(&mut self, type_def: &TypeDef) -> T;
    fn visit_struct_def(&mut self, struct_def: &StructDef) -> T;
    fn visit_enum_def(&mut self, enum_def: &EnumDef) -> T;
    fn visit_statement(&mut self, stmt: &Statement) -> T;
    fn visit_expression(&mut self, expr: &Expression) -> T;
    fn visit_type(&mut self, ty: &Type) -> T;
}

// Default visitor implementation
pub trait DefaultVisitor<T: Default> {
    fn default_result() -> T {
        T::default()
    }
}

// Pretty printing implementations
impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "package {}", self.package.name)?;

        if !self.imports.is_empty() {
            writeln!(f)?;
            writeln!(f, "import (")?;
            for import in &self.imports {
                writeln!(f, "    \"{}\"", import.path)?;
            }
            writeln!(f, ")")?;
        }

        for item in &self.items {
            writeln!(f)?;
            write!(f, "{}", item)?;
        }

        Ok(())
    }
}

impl std::fmt::Display for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Item::Function(func) => write!(f, "{}", func),
            Item::Method(method) => write!(f, "{}", method),
            Item::TypeDef(type_def) => write!(f, "{}", type_def),
        }
    }
}

impl std::fmt::Display for FunctionDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fn {}(", self.name)?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", param.name, param.ty)?;
        }
        write!(f, ")")?;
        if let Some(ret_ty) = &self.return_type {
            write!(f, ": {}", ret_ty)?;
        }
        writeln!(f, " {{ ... }}")
    }
}

impl std::fmt::Display for MethodDecl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fn ({}: {}) {}(",
            self.receiver.name.as_ref().unwrap_or(&"self".to_string()),
            self.receiver.ty,
            self.name
        )?;
        for (i, param) in self.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}: {}", param.name, param.ty)?;
        }
        write!(f, ")")?;
        if let Some(ret_ty) = &self.return_type {
            write!(f, ": {}", ret_ty)?;
        }
        writeln!(f, " {{ ... }}")
    }
}

impl std::fmt::Display for TypeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeDef::Struct(s) => write!(f, "type {} struct {{ ... }}", s.name),
            TypeDef::Enum(e) => write!(f, "type {} enum {{ ... }}", e.name),
        }
    }
}

impl std::fmt::Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::Let(let_stmt) => write!(
                f,
                "let {} = ...",
                match &let_stmt.pattern {
                    Pattern::Identifier(name, _) => name,
                    _ => "...",
                }
            ),
            Statement::Assignment(_) => write!(f, "... = ..."),
            Statement::Expression(expr) => write!(f, "{};", expr),
            Statement::Return(_) => write!(f, "return ..."),
            Statement::If(_) => write!(f, "if (...) {{ ... }}"),
            Statement::While(_) => write!(f, "while (...) {{ ... }}"),
            Statement::For(_) => write!(f, "for (...) {{ ... }}"),
            Statement::Block(_) => write!(f, "{{ ... }}"),
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Integer(lit) => write!(f, "{}", lit.value),
            Expression::Float(lit) => write!(f, "{}", lit.value),
            Expression::String(lit) => write!(f, "\"{}\"", lit.value),
            Expression::TemplateString(_) => write!(f, "`...`"),
            Expression::Boolean(lit) => write!(f, "{}", lit.value),
            Expression::Identifier(id) => write!(f, "{}", id.name),
            Expression::Path(path) => write!(f, "{}", path.segments.join("::")),
            Expression::Binary(expr) => write!(f, "({} {:?} {})", expr.left, expr.op, expr.right),
            Expression::Unary(expr) => write!(f, "({:?} {})", expr.op, expr.operand),
            Expression::Call(expr) => write!(f, "{}(...)", expr.callee),
            Expression::MethodCall(expr) => write!(f, "{}.{}(...)", expr.receiver, expr.method),
            Expression::Index(expr) => write!(f, "{}[{}]", expr.object, expr.index),
            Expression::Field(expr) => write!(f, "{}.{}", expr.object, expr.field),
            Expression::Reference(expr) => {
                write!(f, "&{}{}", if expr.is_mut { "mut " } else { "" }, expr.expr)
            }
            Expression::Dereference(expr) => write!(f, "*{}", expr.expr),
            Expression::StructLit(lit) => write!(f, "{} {{ ... }}", lit.ty),
            Expression::EnumVariant(expr) => write!(f, "{}::{}", expr.enum_name, expr.variant),
            Expression::Array(_) => write!(f, "[...]"),
            Expression::Tuple(_) => write!(f, "(...)"),
            Expression::Cast(expr) => write!(f, "{} as {}", expr.expr, expr.ty),
            Expression::Assignment(expr) => write!(f, "{} = {}", expr.target, expr.value),
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I8 => write!(f, "i8"),
            Type::I16 => write!(f, "i16"),
            Type::I32 => write!(f, "i32"),
            Type::I64 => write!(f, "i64"),
            Type::I128 => write!(f, "i128"),
            Type::I256 => write!(f, "i256"),
            Type::U8 => write!(f, "u8"),
            Type::U16 => write!(f, "u16"),
            Type::U32 => write!(f, "u32"),
            Type::U64 => write!(f, "u64"),
            Type::U128 => write!(f, "u128"),
            Type::U256 => write!(f, "u256"),
            Type::F8 => write!(f, "f8"),
            Type::F16 => write!(f, "f16"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Bool => write!(f, "bool"),
            Type::String => write!(f, "String"),
            Type::Void => write!(f, "void"),
            Type::Reference(ty, is_mut) => {
                write!(f, "&{}{}", if *is_mut { "mut " } else { "" }, ty)
            }
            Type::Array(ty) => write!(f, "[{}]", ty),
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            Type::Function(func_ty) => {
                write!(f, "fn(")?;
                for (i, ty) in func_ty.params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", ty)?;
                }
                write!(f, ") -> {}", func_ty.return_type)
            }
            Type::UserDefined(name) => write!(f, "{}", name),
        }
    }
}
