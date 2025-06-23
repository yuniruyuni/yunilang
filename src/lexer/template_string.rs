//! テンプレート文字列の処理

use crate::ast::{TemplateStringLit, TemplateStringPart, Expression, Span};
use crate::error::ParserError;

/// テンプレート文字列をパース
pub fn parse_template_string(input: &str, span: Span) -> Result<TemplateStringLit, ParserError> {
    let mut parts = Vec::new();
    let mut current_text = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            // 補間式の開始
            chars.next(); // '{'をスキップ
            
            // 現在のテキストを保存
            if !current_text.is_empty() {
                parts.push(TemplateStringPart::Text(current_text.clone()));
                current_text.clear();
            }
            
            // 補間式を収集
            let mut expr_str = String::new();
            let mut brace_count = 1;
            
            while brace_count > 0 {
                match chars.next() {
                    Some('{') => {
                        brace_count += 1;
                        expr_str.push('{');
                    }
                    Some('}') => {
                        brace_count -= 1;
                        if brace_count > 0 {
                            expr_str.push('}');
                        }
                    }
                    Some(c) => expr_str.push(c),
                    None => {
                        return Err(ParserError::SyntaxError {
                            message: "Unterminated interpolation in template string".to_string(),
                            span,
                        });
                    }
                }
            }
            
            // 補間式をプレースホルダとして保存
            // 実際の式の解析はパーサーで行う
            parts.push(TemplateStringPart::Interpolation(
                Expression::Identifier(crate::ast::Identifier {
                    name: format!("${{{}}}", expr_str),
                    span,
                })
            ));
        } else if ch == '\\' {
            // エスケープシーケンス
            match chars.next() {
                Some('n') => current_text.push('\n'),
                Some('r') => current_text.push('\r'),
                Some('t') => current_text.push('\t'),
                Some('\\') => current_text.push('\\'),
                Some('`') => current_text.push('`'),
                Some('$') => current_text.push('$'),
                Some(c) => {
                    current_text.push('\\');
                    current_text.push(c);
                }
                None => current_text.push('\\'),
            }
        } else {
            current_text.push(ch);
        }
    }
    
    // 残りのテキストを保存
    if !current_text.is_empty() {
        parts.push(TemplateStringPart::Text(current_text));
    }
    
    Ok(TemplateStringLit { parts, span })
}

/// テンプレート文字列の補間部分を識別するためのヘルパー関数
pub fn find_interpolations(input: &str) -> Vec<(usize, usize)> {
    let mut positions = Vec::new();
    let mut chars = input.char_indices().peekable();
    
    while let Some((i, ch)) = chars.next() {
        if ch == '$' && chars.peek().map(|(_, c)| *c) == Some('{') {
            let start = i;
            chars.next(); // '{'をスキップ
            
            let mut brace_count = 1;
            let mut end = start + 2;
            
            while brace_count > 0 {
                match chars.next() {
                    Some((j, '{')) => {
                        brace_count += 1;
                        end = j + 1;
                    }
                    Some((j, '}')) => {
                        brace_count -= 1;
                        end = j + 1;
                    }
                    Some((j, _)) => {
                        end = j + 1;
                    }
                    None => break,
                }
            }
            
            if brace_count == 0 {
                positions.push((start, end));
            }
        }
    }
    
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_interpolations() {
        assert_eq!(find_interpolations("hello world"), vec![]);
        assert_eq!(find_interpolations("hello ${name}"), vec![(6, 13)]);
        assert_eq!(find_interpolations("${a} and ${b}"), vec![(0, 4), (9, 13)]);
        assert_eq!(find_interpolations("nested ${x + ${y}}"), vec![(7, 18)]);
    }
}