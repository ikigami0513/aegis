pub mod lexer;
pub mod parser;

use serde_json::Value as JsonValue;
use lexer::Lexer;
use parser::Parser;

pub fn compile(source: &str) -> Result<JsonValue, String> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse()
}
