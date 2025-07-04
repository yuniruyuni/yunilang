//! 式の定義

use serde::{Deserialize, Serialize};

use super::{Pattern, Span, Type};

/// 式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Integer(IntegerLit),
    Float(FloatLit),
    String(StringLit),
    TemplateString(TemplateStringLit),
    Boolean(BooleanLit),
    Identifier(Identifier),
    Path(PathExpr),
    Binary(BinaryExpr),
    Unary(UnaryExpr),
    Call(CallExpr),
    MethodCall(MethodCallExpr),
    Index(IndexExpr),
    Field(FieldExpr),
    Reference(ReferenceExpr),
    Dereference(DereferenceExpr),
    StructLit(StructLiteral),
    EnumVariant(EnumVariantExpr),
    Array(ArrayExpr),
    Tuple(TupleExpr),
    Cast(CastExpr),
    Assignment(AssignmentExpr),
    Match(MatchExpr),
    If(IfExpr),
    Block(BlockExpr),
    ListLiteral(ListLiteral),
    MapLiteral(MapLiteral),
}

/// 整数リテラル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegerLit {
    pub value: i128,
    pub suffix: Option<String>,
    pub span: Span,
}

/// 浮動小数点リテラル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FloatLit {
    pub value: f64,
    pub suffix: Option<String>,
    pub span: Span,
}

/// 文字列リテラル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StringLit {
    pub value: String,
    pub span: Span,
}

/// テンプレート文字列リテラル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateStringLit {
    pub parts: Vec<TemplateStringPart>,
    pub span: Span,
}

/// テンプレート文字列の部分
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateStringPart {
    Text(String),
    Interpolation(Expression),
}

/// 真偽値リテラル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanLit {
    pub value: bool,
    pub span: Span,
}

/// 識別子
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

/// パス式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathExpr {
    pub segments: Vec<String>,
    pub span: Span,
}

/// 二項演算式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryExpr {
    pub left: Box<Expression>,
    pub op: BinaryOp,
    pub right: Box<Expression>,
    pub span: Span,
}

/// 二項演算子
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// 単項演算式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnaryExpr {
    pub op: UnaryOp,
    pub expr: Box<Expression>,
    pub span: Span,
}

/// 単項演算子
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Negate,
    BitNot,
}

/// 関数呼び出し式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallExpr {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
    pub span: Span,
    #[serde(default)]
    pub is_tail: bool,  // 末尾呼び出しかどうか
}

/// メソッド呼び出し式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodCallExpr {
    pub object: Box<Expression>,
    pub method: String,
    pub args: Vec<Expression>,
    pub span: Span,
}

/// インデックスアクセス式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexExpr {
    pub object: Box<Expression>,
    pub index: Box<Expression>,
    pub span: Span,
}

/// フィールドアクセス式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldExpr {
    pub object: Box<Expression>,
    pub field: String,
    pub span: Span,
}

/// 参照式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceExpr {
    pub expr: Box<Expression>,
    pub is_mut: bool,
    pub span: Span,
}

/// 参照外し式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DereferenceExpr {
    pub expr: Box<Expression>,
    pub span: Span,
}

/// 構造体リテラル
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructLiteral {
    /// 型名。暗黙的変換の場合はNone
    pub name: Option<String>,
    pub fields: Vec<StructFieldInit>,
    pub span: Span,
}

/// 構造体フィールド初期化
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructFieldInit {
    pub name: String,
    pub value: Expression,
}

/// 列挙型バリアント式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumVariantExpr {
    pub enum_name: String,
    pub variant: String,
    pub fields: EnumVariantFields,
    pub span: Span,
}

/// 列挙型バリアントのフィールド定義
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EnumVariantFields {
    /// タプルライク（位置指定）フィールド: Some(42)
    Tuple(Vec<Expression>),
    /// 構造体ライク（名前付き）フィールド: Some { value: 42 }
    Struct(Vec<StructFieldInit>),
    /// フィールドなし: None
    Unit,
}

/// 配列式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

/// タプル式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleExpr {
    pub elements: Vec<Expression>,
    pub span: Span,
}

/// キャスト式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastExpr {
    pub expr: Box<Expression>,
    pub ty: Type,
    pub span: Span,
}

/// 代入式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignmentExpr {
    pub target: Box<Expression>,
    pub value: Box<Expression>,
    pub span: Span,
}

/// match式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchExpr {
    pub expr: Box<Expression>,
    pub arms: Vec<MatchArm>,
    pub span: Span,
}

/// matchアーム
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>,
    pub expr: Expression,
}

/// if式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfExpr {
    pub condition: Box<Expression>,
    pub then_branch: Box<Expression>,
    pub else_branch: Option<Box<Expression>>,
    pub span: Span,
}

/// ブロック式
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockExpr {
    pub statements: Vec<super::statements::Statement>,
    pub last_expr: Option<Box<Expression>>,
    pub span: Span,
}

/// リストリテラル（[1, 2, 3] または Vec<T>[1, 2, 3]）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListLiteral {
    /// 型名（Vec<T> など）。省略可能
    pub type_name: Option<(String, Vec<Type>)>,
    /// 要素
    pub elements: Vec<Expression>,
    pub span: Span,
}

/// マップリテラル（{"key": value} または HashMap<K,V>{...}）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapLiteral {
    /// 型名（HashMap<K,V> など）。省略可能
    pub type_name: Option<(String, Vec<Type>)>,
    /// キー・バリューペア
    pub pairs: Vec<(Expression, Expression)>,
    pub span: Span,
}

impl Expression {
    /// 式のSpanを取得する
    pub fn span(&self) -> Span {
        match self {
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
            Expression::Match(match_expr) => match_expr.span,
            Expression::If(if_expr) => if_expr.span,
            Expression::Block(block_expr) => block_expr.span,
            Expression::ListLiteral(list) => list.span,
            Expression::MapLiteral(map) => map.span,
        }
    }
}