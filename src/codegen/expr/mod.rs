//! 式のコード生成モジュール

mod literal;
mod binary;
mod unary;
mod call;
mod struct_enum;
mod array_tuple;
mod control;
mod type_inference;
mod basic;

use crate::ast::*;
use crate::error::YuniResult;
use inkwell::values::BasicValueEnum;

use super::code_generator::CodeGenerator;

impl<'ctx> CodeGenerator<'ctx> {
    /// 式をコンパイル（期待される型のコンテキストなし）
    pub fn compile_expression(&mut self, expr: &Expression) -> YuniResult<BasicValueEnum<'ctx>> {
        self.compile_expression_with_type(expr, None)
    }

    /// 式をコンパイル（期待される型のコンテキスト付き）
    pub fn compile_expression_with_type(&mut self, expr: &Expression, expected_type: Option<&Type>) -> YuniResult<BasicValueEnum<'ctx>> {
        match expr {
            Expression::Integer(lit) => self.compile_integer_literal_with_type(lit, expected_type),
            Expression::Float(lit) => self.compile_float_literal(lit),
            Expression::String(lit) => self.compile_string_literal(lit),
            Expression::TemplateString(lit) => self.compile_template_string(lit),
            Expression::Boolean(lit) => self.compile_boolean_literal(lit),
            Expression::Identifier(id) => self.compile_identifier(id),
            Expression::Path(path) => self.compile_path(path),
            Expression::Binary(binary) => self.compile_binary_expr(binary),
            Expression::Unary(unary) => self.compile_unary_expr(unary),
            Expression::Call(call) => self.compile_call_expr(call),
            Expression::MethodCall(method_call) => self.compile_method_call(method_call),
            Expression::Index(index) => self.compile_index_expr(index),
            Expression::Field(field) => self.compile_field_expr(field),
            Expression::Reference(ref_expr) => self.compile_reference_expr(ref_expr),
            Expression::Dereference(deref) => self.compile_dereference_expr(deref),
            Expression::StructLit(struct_lit) => self.compile_struct_literal(struct_lit),
            Expression::EnumVariant(enum_var) => self.compile_enum_variant(enum_var),
            Expression::Array(array) => self.compile_array_expr(array),
            Expression::Tuple(tuple) => self.compile_tuple_expr(tuple),
            Expression::Cast(cast) => self.compile_cast_expr(cast),
            Expression::Assignment(assign) => self.compile_assignment_expr(assign),
            Expression::Match(match_expr) => self.compile_match_expr(match_expr),
            Expression::If(if_expr) => self.compile_if_expr(if_expr),
            Expression::Block(block_expr) => self.compile_block_expr(block_expr),
            Expression::ListLiteral(list) => self.compile_list_literal(list),
            Expression::MapLiteral(map) => self.compile_map_literal(map),
        }
    }
}