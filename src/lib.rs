//! Yuni Language Compiler Library
//!
//! This library provides the core functionality for the Yuni language compiler.

pub mod analyzer;
pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod runtime;

// Re-export commonly used types
pub use analyzer::SemanticAnalyzer;
pub use ast::{Expression, Program, Statement};
pub use codegen::CodeGenerator;
pub use lexer::{Lexer, Token, TokenWithPosition};
pub use parser::{ParseError, ParseResult, Parser};
