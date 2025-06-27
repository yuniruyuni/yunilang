//! 演算子の型チェック

use crate::ast::*;
use crate::analyzer::symbol::{AnalysisResult, AnalysisError};
use crate::analyzer::type_checker::TypeChecker;

impl TypeChecker {
    /// 二項演算の結果型を取得
    pub fn check_binary_operation(&self, op: &BinaryOp, left: &Type, right: &Type, span: Span) -> AnalysisResult<Type> {
        // まず型の互換性をチェック
        match op {
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                // 算術演算は数値型でなければならない
                if !self.is_numeric_type(left) || !self.is_numeric_type(right) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "numeric type".to_string(),
                        found: format!("{:?} and {:?}", left, right),
                        span,
                    });
                }
                
                // 同じ型である必要がある
                if left != right {
                    return Err(AnalysisError::TypeMismatch {
                        expected: format!("{:?}", left),
                        found: format!("{:?}", right),
                        span,
                    });
                }
                
                Ok(left.clone())
            }
            
            BinaryOp::Eq | BinaryOp::Ne => {
                // 等価比較は同じ型である必要がある
                if left != right {
                    return Err(AnalysisError::TypeMismatch {
                        expected: format!("{:?}", left),
                        found: format!("{:?}", right),
                        span,
                    });
                }
                Ok(Type::Bool)
            }
            
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Le | BinaryOp::Ge => {
                // 大小比較は数値型でなければならない
                if !self.is_numeric_type(left) || !self.is_numeric_type(right) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "numeric type".to_string(),
                        found: format!("{:?} and {:?}", left, right),
                        span,
                    });
                }
                
                // 同じ型である必要がある
                if left != right {
                    return Err(AnalysisError::TypeMismatch {
                        expected: format!("{:?}", left),
                        found: format!("{:?}", right),
                        span,
                    });
                }
                
                Ok(Type::Bool)
            }
            
            BinaryOp::And | BinaryOp::Or => {
                // 論理演算はブール型でなければならない
                if left != &Type::Bool || right != &Type::Bool {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: format!("{:?} and {:?}", left, right),
                        span,
                    });
                }
                Ok(Type::Bool)
            }
            
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
                // ビット演算は整数型でなければならない
                if !self.is_integer_type(left) || !self.is_integer_type(right) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "integer type".to_string(),
                        found: format!("{:?} and {:?}", left, right),
                        span,
                    });
                }
                
                // 同じ型である必要がある
                if left != right {
                    return Err(AnalysisError::TypeMismatch {
                        expected: format!("{:?}", left),
                        found: format!("{:?}", right),
                        span,
                    });
                }
                
                Ok(left.clone())
            }
            
            _ => {
                Err(AnalysisError::Internal {
                    message: format!("Unhandled binary operator: {:?}", op),
                })
            }
        }
    }

    /// 単項演算の結果型を取得
    pub fn check_unary_operation(&self, op: &UnaryOp, operand: &Type, span: Span) -> AnalysisResult<Type> {
        match op {
            UnaryOp::Negate => {
                // 否定演算は数値型でなければならない
                if !self.is_numeric_type(operand) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "numeric type".to_string(),
                        found: format!("{:?}", operand),
                        span,
                    });
                }
                Ok(operand.clone())
            }
            
            UnaryOp::Not => {
                // 論理否定はブール型でなければならない
                if operand != &Type::Bool {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "bool".to_string(),
                        found: format!("{:?}", operand),
                        span,
                    });
                }
                Ok(Type::Bool)
            }
            
            UnaryOp::BitNot => {
                // ビット否定は整数型でなければならない
                if !self.is_integer_type(operand) {
                    return Err(AnalysisError::TypeMismatch {
                        expected: "integer type".to_string(),
                        found: format!("{:?}", operand),
                        span,
                    });
                }
                Ok(operand.clone())
            }
            
            _ => {
                Err(AnalysisError::Internal {
                    message: format!("Unhandled unary operator: {:?}", op),
                })
            }
        }
    }
}