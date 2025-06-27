//! 式のセマンティック解析

use crate::ast::*;
use crate::analyzer::symbol::AnalysisResult;
use crate::analyzer::semantic_analyzer::SemanticAnalyzer;

impl SemanticAnalyzer {
    /// 式を解析
    pub fn analyze_expression(&mut self, expr: &Expression) -> AnalysisResult<Type> {
        self.analyze_expression_with_type(expr, None)
    }
    
    /// 式を解析（期待される型のコンテキスト付き）
    pub fn analyze_expression_with_type(&mut self, expr: &Expression, expected_type: Option<&Type>) -> AnalysisResult<Type> {
        match expr {
            Expression::Integer(lit) => self.analyze_integer_literal(lit, expected_type),
            Expression::Float(lit) => self.analyze_float_literal(lit),
            Expression::String(lit) => self.analyze_string_literal(lit),
            Expression::Boolean(lit) => self.analyze_boolean_literal(lit),
            Expression::TemplateString(lit) => self.analyze_template_string(lit),
            Expression::Identifier(ident) => self.analyze_identifier(ident),
            Expression::Path(path) => self.analyze_path_expression(path),
            Expression::Binary(binary) => self.analyze_binary_expression(binary),
            Expression::Unary(unary) => self.analyze_unary_expression(unary),
            Expression::Call(call) => self.analyze_call_expression(call),
            Expression::MethodCall(method_call) => self.analyze_method_call(method_call),
            Expression::Field(field) => self.analyze_field_expression(field),
            Expression::Index(index) => self.analyze_index_expression(index),
            Expression::Reference(ref_expr) => self.analyze_reference_expression(ref_expr),
            Expression::Dereference(deref_expr) => self.analyze_dereference_expression(deref_expr),
            Expression::Assignment(assign) => self.analyze_assignment_expression(assign),
            Expression::StructLit(struct_lit) => self.analyze_struct_literal(struct_lit),
            Expression::Array(array) => self.analyze_array_expression(array),
            Expression::Tuple(tuple) => self.analyze_tuple_expression(tuple),
            Expression::Cast(cast) => self.analyze_cast_expression(cast),
            Expression::EnumVariant(enum_variant) => self.analyze_enum_variant(enum_variant),
            Expression::If(if_expr) => self.analyze_if_expression(if_expr),
            Expression::Match(match_expr) => self.analyze_match_expression(match_expr),
            Expression::Block(block_expr) => self.analyze_block_expression(block_expr),
            Expression::ListLiteral(list) => self.analyze_list_literal(list, expected_type),
            Expression::MapLiteral(map) => self.analyze_map_literal(map, expected_type),
        }
    }
}