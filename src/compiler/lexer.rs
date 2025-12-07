use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    True, False, Null,
    Try, Catch,
    Var, If, Else, While, For, Func, Return, Print, Input, Class, New, Extends, Import, Break, Switch, Case, Default,
    Identifier(String), StringLiteral(String), Integer(i64), Float(f64),
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, Neq, Lt, Gt, LtEq, GtEq,
    And, Or, Bang,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Comma, Dot, Colon, EOF,
    PlusEq,   // +=
    MinusEq,  // -=
    StarEq,   // *=
    SlashEq,  // /=
    PlusPlus, // ++
    MinusMinus, // --
    Namespace,
    BitAnd, BitOr, BitXor, ShiftLeft, ShiftRight,
    At,
    Arrow
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    line: usize
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Lexer { 
            chars: input.chars().peekable(),
            line: 1 
        }
    }
    fn add_token(&self, tokens: &mut Vec<Token>, kind: TokenKind) {
        tokens.push(Token { kind, line: self.line });
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(&c) = self.chars.peek() {
            match c {
                '\n' => {
                    self.line += 1;
                    self.chars.next();
                }
                ' ' | '\t' | '\r' => { self.chars.next(); }
                '/' => {
                    self.chars.next();
                    if let Some(&'/') = self.chars.peek() {
                        while let Some(&c) = self.chars.peek() {
                            if c == '\n' { break; }
                            self.chars.next();
                        }
                    }
                    else if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::SlashEq);
                    } 
                    else { 
                        self.add_token(&mut tokens, TokenKind::Slash);
                    }
                }
                '{' => {
                    self.add_token(&mut tokens, TokenKind::LBrace);
                    self.chars.next(); 
                }
                '}' => { 
                    self.add_token(&mut tokens, TokenKind::RBrace);
                    self.chars.next(); 
                }
                '(' => {
                    self.add_token(&mut tokens, TokenKind::LParen);
                    self.chars.next(); 
                }
                ')' => {
                    self.add_token(&mut tokens, TokenKind::RParen);
                    self.chars.next(); 
                }
                '[' => { 
                    self.add_token(&mut tokens, TokenKind::LBracket);
                    self.chars.next(); 
                }
                ']' => { 
                    self.add_token(&mut tokens, TokenKind::RBracket);
                    self.chars.next(); 
                }
                ',' => {
                    self.add_token(&mut tokens, TokenKind::Comma);
                    self.chars.next(); 
                }
                '.' => { 
                    self.add_token(&mut tokens, TokenKind::Dot);
                    self.chars.next(); 
                }
                ':' => {
                    self.add_token(&mut tokens, TokenKind::Colon);
                    self.chars.next(); 
                }
                '+' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::PlusEq);
                    } 
                    else if let Some(&'+') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::PlusPlus);
                    } 
                    else {
                        self.add_token(&mut tokens, TokenKind::Plus);
                    }
                }
                '-' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::MinusEq);
                    } 
                    else if let Some(&'-') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::MinusMinus);
                    } 
                    else if let Some(&'>') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::Arrow);
                    }
                    else {
                        self.add_token(&mut tokens, TokenKind::Minus);
                    }
                }
                '*' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::StarEq);
                    } 
                    else {
                        self.add_token(&mut tokens, TokenKind::Star);
                    }
                }
                '%' => { 
                    self.add_token(&mut tokens, TokenKind::Percent);
                    self.chars.next(); 
                }
                '=' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(&mut tokens, TokenKind::EqEq);
                    } 
                    else { 
                        self.add_token(&mut tokens, TokenKind::Eq);
                    }
                }
                '<' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(&mut tokens, TokenKind::LtEq);
                    }
                    else if let Some(&'<') = self.chars.peek() { // <<
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::ShiftLeft);
                    }
                    else { 
                        self.add_token(&mut tokens, TokenKind::Lt);
                    }
                }
                '>' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(&mut tokens, TokenKind::GtEq);
                    }
                    else if let Some(&'>') = self.chars.peek() { // >>
                        self.chars.next();
                        self.add_token(&mut tokens, TokenKind::ShiftRight);
                    }
                    else { 
                        self.add_token(&mut tokens, TokenKind::Gt);
                    }
                },
                '&' => {
                    self.chars.next();
                    if let Some(&'&') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(&mut tokens, TokenKind::And);
                    }
                    else {
                        self.add_token(&mut tokens, TokenKind::BitAnd);
                    }
                },
                '|' => {
                    self.chars.next();
                    if let Some(&'|') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(&mut tokens, TokenKind::Or);
                    }
                    else {
                        self.add_token(&mut tokens, TokenKind::BitOr);
                    }
                },
                '^' => {
                    self.chars.next();
                    self.add_token(&mut tokens, TokenKind::BitXor);
                },
                '!' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(&mut tokens, TokenKind::Neq);
                    }
                    else { 
                        self.add_token(&mut tokens, TokenKind::Bang);
                    }
                },
                '@' => {
                    self.chars.next();
                    self.add_token(&mut tokens, TokenKind::At);
                }
                '"' => tokens.push(self.read_string()),
                c if c.is_digit(10) => tokens.push(self.read_number()),
                c if c.is_alphabetic() || c == '_' => tokens.push(self.read_identifier()),
                _ => panic!("Unexpected char '{}'", c),
            }
        }
        self.add_token(&mut tokens, TokenKind::EOF);
        tokens
    }

    fn read_string(&mut self) -> Token {
        self.chars.next(); 
        let mut s = String::new();
        while let Some(&c) = self.chars.peek() {
            if c == '"' { 
                self.chars.next(); 
                return Token {
                    kind: TokenKind::StringLiteral(s), line: self.line
                };
            }
            s.push(self.chars.next().unwrap());
        }
        panic!("Unterminated string");
    }

    fn read_number(&mut self) -> Token {
        let mut s = String::new();
        let mut has_dot = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_digit(10) { 
                s.push(self.chars.next().unwrap()); 
            } 
            else if c == '.' && !has_dot { 
                has_dot = true; 
                s.push(self.chars.next().unwrap()); 
            } 
            else { 
                break; 
            }
        }

        let kind = if has_dot { 
            TokenKind::Float(s.parse().unwrap())
        } 
        else {
            TokenKind::Integer(s.parse().unwrap())
        };

        Token {
            kind,
            line: self.line 
        }
    }

    fn read_identifier(&mut self) -> Token {
        let mut s = String::new();

        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' { 
                s.push(self.chars.next().unwrap()); 
            } 
            else { 
                break; 
            }
        }

        let kind = match s.as_str() {
            "var" => TokenKind::Var, 
            "if" => TokenKind::If, 
            "else" => TokenKind::Else, 
            "while" => TokenKind::While,
            "for" => TokenKind::For, 
            "func" => TokenKind::Func, 
            "return" => TokenKind::Return, 
            "print" => TokenKind::Print,
            "input" => TokenKind::Input, 
            "class" => TokenKind::Class, 
            "new" => TokenKind::New, 
            "extends" => TokenKind::Extends,
            "import" => TokenKind::Import, 
            "break" => TokenKind::Break, 
            "switch" => TokenKind::Switch, 
            "case" => TokenKind::Case, 
            "default" => TokenKind::Default,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "null" => TokenKind::Null,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "namespace" => TokenKind::Namespace,
            _ => TokenKind::Identifier(s),
        };

        Token {
            kind,
            line: self.line
        }
    }
}
