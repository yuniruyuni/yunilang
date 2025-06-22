//! Demonstrates parsing Yuni language code without LLVM dependencies

use yunilang::lexer::Lexer;
use yunilang::parser::Parser;

fn main() {
    let examples = vec![
        (
            "Simple function",
            r#"package main

fn add(a: i32, b: i32): i32 {
    return a + b;
}"#,
        ),
        (
            "Struct and method",
            r#"package main

type Point struct {
    x: f32,
    y: f32
}

fn (p: &Point) Length(): f32 {
    return p.x;
}"#,
        ),
        (
            "Template strings",
            r#"package main

fn main() {
    let name = "World";
    let msg = `Hello, ${name}!`;
}"#,
        ),
        (
            "Lives clause",
            r#"package main

fn new(message: &String): (ret: Messenger)
lives
    ret = message
{
    return Messenger { message };
}"#,
        ),
    ];

    for (name, code) in examples {
        println!("\n=== {} ===", name);
        println!("Code:\n{}\n", code);

        let lexer = Lexer::new(code);
        let tokens: Vec<_> = lexer.collect_tokens();

        let mut parser = Parser::new(tokens);
        match parser.parse() {
            Ok(program) => {
                println!("✓ Successfully parsed!");
                println!("  Package: {}", program.package.name);
                println!("  Imports: {}", program.imports.len());
                println!("  Items: {}", program.items.len());
            }
            Err(e) => {
                println!("✗ Parse error: {}", e);
            }
        }
    }
}
