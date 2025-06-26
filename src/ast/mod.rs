//! 抽象構文木（AST）定義
//!
//! このモジュールはYuni言語のASTノードを定義します。

mod declarations;
mod expressions;
mod patterns;
mod program;
mod span;
mod statements;
mod types;

// ソース位置情報を再エクスポート
pub use span::Span;

// プログラム構造を再エクスポート
pub use program::{Import, PackageDecl, Program};

// 型定義を再エクスポート
pub use types::{FunctionType, LivesClause, LivesConstraint, Type, TypeParam};

// 宣言を再エクスポート
pub use declarations::{
    EnumDef, Field, FunctionDecl, Item, MethodDecl, Param, Receiver, StructDef, TypeAlias, TypeDef, Variant,
};

// 式を再エクスポート
pub use expressions::{
    ArrayExpr, AssignmentExpr, BinaryExpr, BinaryOp, BlockExpr, BooleanLit, CallExpr, CastExpr,
    DereferenceExpr, EnumVariantExpr, EnumVariantFields, Expression, FieldExpr, FloatLit, Identifier, IfExpr, IndexExpr,
    IntegerLit, ListLiteral, MapLiteral, MatchArm, MatchExpr, MethodCallExpr, PathExpr, ReferenceExpr, StringLit,
    StructFieldInit, StructLiteral, TemplateStringLit, TemplateStringPart, TupleExpr, UnaryExpr,
    UnaryOp,
};

// 文を再エクスポート
pub use statements::{
    AssignStatement, Block, ElseBranch, ForStatement, IfStatement, LetStatement, ReturnStatement,
    Statement, WhileStatement,
};

// パターンを再エクスポート
pub use patterns::{Pattern, EnumVariantPatternFields, LiteralPattern};