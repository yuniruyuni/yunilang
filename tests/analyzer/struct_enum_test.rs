//! 構造体と列挙型のセマンティック解析テスト

use super::*;

#[test]
fn test_struct_type_checking() {
    // 構造体の型チェックのテスト
    let source = r#"
    package main
    
    struct Point {
        x: f64,
        y: f64,
    }
    
    fn distance(p1: Point, p2: Point): f64 {
        let dx = p1.x - p2.x;
        let dy = p1.y - p2.y;
        return sqrt(dx * dx + dy * dy);
    }
    
    fn main() {
        let origin = Point { x: 0.0, y: 0.0 };
        let point = Point { x: 3.0, y: 4.0 };
        let dist = distance(origin, point);
    }
    "#;
    
    assert_analysis_success(source);
}

#[test]
fn test_enum_type_checking() {
    // 列挙型の型チェックのテスト
    let source = r#"
    package main
    
    enum Color {
        Red,
        Green,
        Blue,
    }
    
    enum Maybe {
        Some { value: i32 },
        None,
    }
    
    fn get_color_name(color: Color): str {
        return match color {
            Color::Red => "red",
            Color::Green => "green",
            Color::Blue => "blue",
        };
    }
    
    fn main() {
        let color = Color::Red;
        let name = get_color_name(color);
        
        let maybe_value = Maybe::Some { value: 42 };
        let empty = Maybe::None;
    }
    "#;
    
    assert_analysis_success(source);
}