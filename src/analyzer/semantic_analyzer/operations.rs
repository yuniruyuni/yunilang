//! 演算子と代入式のセマンティック解析

use crate::ast::*;
use crate::analyzer::symbol::AnalysisResult;
use crate::analyzer::semantic_analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 二項演算式の解析
    pub fn analyze_binary_expression(&mut self, binary: &BinaryExpr) -> AnalysisResult<Type> {
        let left_type = self.analyze_expression(&binary.left)?;
        let right_type = self.analyze_expression(&binary.right)?;
        
        self.type_checker.check_binary_operation(&binary.op, &left_type, &right_type, binary.span)
    }
    
    /// 単項演算式の解析
    pub fn analyze_unary_expression(&mut self, unary: &UnaryExpr) -> AnalysisResult<Type> {
        let expr_type = self.analyze_expression(&unary.expr)?;
        self.type_checker.check_unary_operation(&unary.op, &expr_type, unary.span)
    }

    /// 代入式の解析
    pub(crate) fn analyze_assignment_expression(&mut self, assign_expr: &AssignmentExpr) -> AnalysisResult<Type> {
        let target_type = self.analyze_expression(&assign_expr.target)?;
        let value_type = self.analyze_expression(&assign_expr.value)?;
        
        // 型の互換性をチェック
        self.type_checker.check_type_compatibility(&value_type, &target_type, assign_expr.span)?;
        
        // TODO: ミュータビリティのチェック
        
        Ok(target_type)
    }

    /// キャスト式の解析
    pub(crate) fn analyze_cast_expression(&mut self, cast: &CastExpr) -> AnalysisResult<Type> {
        let _expr_type = self.analyze_expression(&cast.expr)?;
        // TODO: キャストの妥当性をチェック
        Ok(cast.ty.clone())
    }
}