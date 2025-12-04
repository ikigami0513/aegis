use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Var, If, Else, While, For, Func, Return, Print, Input, Class, New, Extends, Import, Break, Switch, Case, Default,
    Identifier(String), StringLiteral(String), Integer(i64), Float(f64),
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, Neq, Lt, Gt, LtEq, GtEq,
    And, Or, Bang,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Dot, Colon, EOF,
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer { chars: input.chars().peekable() }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(&c) = self.chars.peek() {
            match c {
                ' ' | '\t' | '\n' | '\r' => { self.chars.next(); }
                '/' => {
                    self.chars.next();
                    if let Some(&'/') = self.chars.peek() {
                        while let Some(&c) = self.chars.peek() {
                            if c == '\n' { break; }
                            self.chars.next();
                        }
                    } else { tokens.push(Token::Slash); }
                }
                '{' => { tokens.push(Token::LBrace); self.chars.next(); }
                '}' => { tokens.push(Token::RBrace); self.chars.next(); }
                '(' => { tokens.push(Token::LParen); self.chars.next(); }
                ')' => { tokens.push(Token::RParen); self.chars.next(); }
                '[' => { tokens.push(Token::LBracket); self.chars.next(); }
                ']' => { tokens.push(Token::RBracket); self.chars.next(); }
                ',' => { tokens.push(Token::Comma); self.chars.next(); }
                '.' => { tokens.push(Token::Dot); self.chars.next(); }
                ':' => { tokens.push(Token::Colon); self.chars.next(); }
                '+' => { tokens.push(Token::Plus); self.chars.next(); }
                '-' => { tokens.push(Token::Minus); self.chars.next(); }
                '*' => { tokens.push(Token::Star); self.chars.next(); }
                '%' => { tokens.push(Token::Percent); self.chars.next(); }
                '=' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { self.chars.next(); tokens.push(Token::EqEq); } 
                    else { tokens.push(Token::Eq); }
                }
                '<' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { self.chars.next(); tokens.push(Token::LtEq); } 
                    else { tokens.push(Token::Lt); }
                }
                '>' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { self.chars.next(); tokens.push(Token::GtEq); } 
                    else { tokens.push(Token::Gt); }
                },
                '&' => {
                    self.chars.next();
                    if let Some(&'&') = self.chars.peek() { self.chars.next(); tokens.push(Token::And); }
                    else { panic!("Unsupported char '&' (use '&&')"); }
                },
                '|' => {
                    self.chars.next();
                    if let Some(&'|') = self.chars.peek() { self.chars.next(); tokens.push(Token::Or); }
                    else { panic!("Unsupported char '|' (use '||')"); }
                },
                '!' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { self.chars.next(); tokens.push(Token::Neq); }
                    else { tokens.push(Token::Bang); }
                },
                '"' => tokens.push(self.read_string()),
                c if c.is_digit(10) => tokens.push(self.read_number()),
                c if c.is_alphabetic() || c == '_' => tokens.push(self.read_identifier()),
                _ => panic!("Unexpected char '{}'", c),
            }
        }
        tokens.push(Token::EOF);
        tokens
    }

    fn read_string(&mut self) -> Token {
        self.chars.next(); 
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c == '"' { self.chars.next(); return Token::StringLiteral(s); }
            s.push(self.chars.next().unwrap());
        }
        panic!("Unterminated string");
    }

    fn read_number(&mut self) -> Token {
        let mut s = String::new();
        let mut has_dot = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_digit(10) { s.push(self.chars.next().unwrap()); } 
            else if c == '.' && !has_dot { has_dot = true; s.push(self.chars.next().unwrap()); } 
            else { break; }
        }
        if has_dot { Token::Float(s.parse().unwrap()) } else { Token::Integer(s.parse().unwrap()) }
    }

    fn read_identifier(&mut self) -> Token {
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' { s.push(self.chars.next().unwrap()); } else { break; }
        }
        match s.as_str() {
            "var" => Token::Var, "if" => Token::If, "else" => Token::Else, "while" => Token::While,
            "for" => Token::For, "func" => Token::Func, "return" => Token::Return, "print" => Token::Print,
            "input" => Token::Input, "class" => Token::Class, "new" => Token::New, "extends" => Token::Extends,
            "import" => Token::Import, "break" => Token::Break, "switch" => Token::Switch, "case" => Token::Case, "default" => Token::Default,
            _ => Token::Identifier(s),
        }
    }
}
