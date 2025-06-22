//! 文の定義

use serde::{Deserialize, Serialize};

use super::{Expression, Pattern, Span, Type};

/// 文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Let(LetStatement),
    Assignment(AssignStatement),
    Return(ReturnStatement),
    If(IfStatement),
    While(WhileStatement),
    For(ForStatement),
    Expression(Expression),
    Block(Block),
}

/// let文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetStatement {
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub init: Option<Expression>,
    pub span: Span,
}

/// 代入文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssignStatement {
    pub target: Expression,
    pub value: Expression,
    pub span: Span,
}

/// return文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

/// if文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_branch: Block,
    pub else_branch: Option<ElseBranch>,
    pub span: Span,
}

/// elseブランチ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStatement>),
}

/// while文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: Block,
    pub span: Span,
}

/// for文
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForStatement {
    pub init: Option<Box<Statement>>,
    pub condition: Option<Expression>,
    pub update: Option<Expression>,
    pub body: Block,
    pub span: Span,
}

/// ブロック
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}