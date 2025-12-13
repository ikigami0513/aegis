use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    True, False, Null,
    Try, Catch, Throw,
    Var, If, Else, While, Func, Return, Print, Input, 
    Class, New, Extends, Enum,
    Import, Break, Continue, Switch, Case, Default,
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
    Arrow,
    Super,
    Question,
    DoubleQuestion,
    Const,
    ForEach, In,
    DotDot,
    Public, Protected, Private,
    Static,
    Final,
    Prop,
    Interface, Implements
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
        self.handle_shebang();

        let mut tokens = Vec::new();
        while self.chars.peek().is_some() {
            // On utilise scan_token pour lire le prochain élément
            if let Err(e) = self.scan_token(&mut tokens) {
                // En cas d'erreur (ex: string non fermée), on panic pour l'instant
                // Idéalement, il faudrait retourner un Result<Vec<Token>, String>
                panic!("Lexer error: {}", e);
            }
        }
        self.add_token(&mut tokens, TokenKind::EOF);
        tokens
    }

    // Extrait la logique de lecture d'un token unique pour pouvoir la réutiliser
    fn scan_token(&mut self, tokens: &mut Vec<Token>) -> Result<(), String> {
        if let Some(&c) = self.chars.peek() {
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
                    else if let Some(&'*') = self.chars.peek() {
                        self.chars.next(); // Consomme '*'
                        self.skip_multiline_comment()?;
                    }
                    else if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::SlashEq);
                    } 
                    else { 
                        self.add_token(tokens, TokenKind::Slash);
                    }
                }
                '{' => {
                    self.add_token(tokens, TokenKind::LBrace);
                    self.chars.next(); 
                }
                '}' => { 
                    self.add_token(tokens, TokenKind::RBrace);
                    self.chars.next(); 
                }
                '(' => {
                    self.add_token(tokens, TokenKind::LParen);
                    self.chars.next(); 
                }
                ')' => {
                    self.add_token(tokens, TokenKind::RParen);
                    self.chars.next(); 
                }
                '[' => { 
                    self.add_token(tokens, TokenKind::LBracket);
                    self.chars.next(); 
                }
                ']' => { 
                    self.add_token(tokens, TokenKind::RBracket);
                    self.chars.next(); 
                }
                ',' => {
                    self.add_token(tokens, TokenKind::Comma);
                    self.chars.next(); 
                }
                '.' => { 
                    self.chars.next();
                    if let Some(&'.') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::DotDot);
                    }
                    else {
                        self.add_token(tokens, TokenKind::Dot);
                    }
                }
                ':' => {
                    self.add_token(tokens, TokenKind::Colon);
                    self.chars.next(); 
                }
                '?' => {
                    self.chars.next();
                    if let Some(&'?') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::DoubleQuestion);
                    }
                    else {
                        self.add_token(tokens, TokenKind::Question);
                    }
                },
                '+' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::PlusEq);
                    } 
                    else if let Some(&'+') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::PlusPlus);
                    } 
                    else {
                        self.add_token(tokens, TokenKind::Plus);
                    }
                }
                '-' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::MinusEq);
                    } 
                    else if let Some(&'-') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::MinusMinus);
                    } 
                    else if let Some(&'>') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::Arrow);
                    }
                    else {
                        self.add_token(tokens, TokenKind::Minus);
                    }
                }
                '*' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() {
                        self.chars.next();
                        self.add_token(tokens, TokenKind::StarEq);
                    } 
                    else {
                        self.add_token(tokens, TokenKind::Star);
                    }
                }
                '%' => { 
                    self.add_token(tokens, TokenKind::Percent);
                    self.chars.next(); 
                }
                '=' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(tokens, TokenKind::EqEq);
                    } 
                    else { 
                        self.add_token(tokens, TokenKind::Eq);
                    }
                }
                '<' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(tokens, TokenKind::LtEq);
                    }
                    else if let Some(&'<') = self.chars.peek() { 
                        self.chars.next();
                        self.add_token(tokens, TokenKind::ShiftLeft);
                    }
                    else { 
                        self.add_token(tokens, TokenKind::Lt);
                    }
                }
                '>' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(tokens, TokenKind::GtEq);
                    }
                    else if let Some(&'>') = self.chars.peek() { 
                        self.chars.next();
                        self.add_token(tokens, TokenKind::ShiftRight);
                    }
                    else { 
                        self.add_token(tokens, TokenKind::Gt);
                    }
                },
                '&' => {
                    self.chars.next();
                    if let Some(&'&') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(tokens, TokenKind::And);
                    }
                    else {
                        self.add_token(tokens, TokenKind::BitAnd);
                    }
                },
                '|' => {
                    self.chars.next();
                    if let Some(&'|') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(tokens, TokenKind::Or);
                    }
                    else {
                        self.add_token(tokens, TokenKind::BitOr);
                    }
                },
                '^' => {
                    self.chars.next();
                    self.add_token(tokens, TokenKind::BitXor);
                },
                '!' => {
                    self.chars.next();
                    if let Some(&'=') = self.chars.peek() { 
                        self.chars.next(); 
                        self.add_token(tokens, TokenKind::Neq);
                    }
                    else { 
                        self.add_token(tokens, TokenKind::Bang);
                    }
                },
                '@' => {
                    self.chars.next();
                    self.add_token(tokens, TokenKind::At);
                }
                '"' => {
                    let token = self.read_string();
                    tokens.push(token);
                },
                '`' => {
                    self.chars.next(); // On consomme le backtick d'ouverture
                    self.read_multiline_string(tokens)?;
                },
                c if c.is_digit(10) => {
                    let token = self.read_number();
                    tokens.push(token);
                },
                c if c.is_alphabetic() || c == '_' => {
                    let token = self.read_identifier();
                    tokens.push(token);
                },
                _ => return Err(format!("Unexpected char '{}' at line {}", c, self.line)),
            }
        }
        Ok(())
    }

    fn read_string(&mut self) -> Token {
        self.chars.next(); // On consomme le guillemet ouvrant "
        let mut s = String::new();
        
        while let Some(&c) = self.chars.peek() {
            match c {
                '"' => { 
                    self.chars.next(); // On consomme le guillemet fermant "
                    return Token {
                        kind: TokenKind::StringLiteral(s), 
                        line: self.line
                    };
                },
                '\\' => {
                    self.chars.next(); // On consomme le \
                    if let Some(escaped) = self.chars.next() {
                        match escaped {
                            'n' => s.push('\n'),
                            'r' => s.push('\r'),
                            't' => s.push('\t'),
                            '"' => s.push('"'),
                            '\\' => s.push('\\'),
                            _ => s.push(escaped),
                        }
                    }
                },
                _ => {
                    s.push(self.chars.next().unwrap());
                }
            }
        }
        panic!("Unterminated string at line {}", self.line);
    }

    fn read_number(&mut self) -> Token {
        let mut s = String::new();
        let mut has_dot = false;
        while let Some(&c) = self.chars.peek() {
            if c.is_digit(10) { 
                s.push(self.chars.next().unwrap()); 
            } 
            else if c == '.' && !has_dot {
                let mut lookahead = self.chars.clone();
                lookahead.next();

                if let Some(&'.') = lookahead.peek() {
                    // C'est un '..', donc ce n'est pas un nombre à virgule.
                    // On arrête la lecture du nombre ici (c'est un entier).
                    break; 
                }

                has_dot = true; 
                s.push(self.chars.next().unwrap()); 
            } 
            else { 
                break; 
            }
        }

        let kind = if has_dot { 
            TokenKind::Float(s.parse().unwrap_or(0.0))
        } 
        else {
            TokenKind::Integer(s.parse().unwrap_or(0))
        };

        Token { kind, line: self.line }
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
            "throw" => TokenKind::Throw,
            "namespace" => TokenKind::Namespace,
            "continue" => TokenKind::Continue,
            "super" => TokenKind::Super,
            "enum" => TokenKind::Enum,
            "const" => TokenKind::Const,
            "foreach" => TokenKind::ForEach,
            "in" => TokenKind::In,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "protected" => TokenKind::Protected,
            "static" => TokenKind::Static,
            "final" => TokenKind::Final,
            "prop" => TokenKind::Prop,
            "interface" => TokenKind::Interface,
            "implements" => TokenKind::Implements,
            _ => TokenKind::Identifier(s),
        };

        Token { kind, line: self.line }
    }

    fn handle_shebang(&mut self) {
        let mut lookahead = self.chars.clone();
        
        if let Some('#') = lookahead.next() {
            if let Some('!') = lookahead.next() {
                // C'est un shebang ! On consomme la vraie ligne.
                while let Some(&c) = self.chars.peek() {
                    if c == '\n' { break; } 
                    self.chars.next();
                }
            }
        }
    }

    fn skip_multiline_comment(&mut self) -> Result<(), String> {
        while let Some(c) = self.chars.next() {
            if c == '*' {
                if let Some('/') = self.chars.peek() {
                    self.chars.next(); // Consomme '/'
                    return Ok(()); // Fin du commentaire
                }
            } else if c == '\n' {
                self.line += 1;
            }
        }
        
        Err(format!("Unterminated block comment at line {}", self.line))
    }

    fn read_multiline_string(&mut self, tokens: &mut Vec<Token>) -> Result<(), String> {
        let mut string_content = String::new();
        
        while let Some(&c) = self.chars.peek() {
            match c {
                '`' => { // Fin de la chaîne
                    self.chars.next();
                    self.add_token(tokens, TokenKind::StringLiteral(string_content));
                    return Ok(());
                },
                '\n' => { // Saut de ligne autorisé
                    self.chars.next();
                    string_content.push('\n');
                    self.line += 1;
                },
                '$' => { 
                    self.chars.next();
                    if let Some('{') = self.chars.peek() {
                        // C'est une interpolation ${...}
                        self.chars.next(); // Mange '{'
                        
                        // 1. On push ce qu'on a lu jusqu'ici
                        self.add_token(tokens, TokenKind::StringLiteral(string_content.clone()));
                        string_content.clear();
                        
                        // 2. On ajoute un '+'
                        self.add_token(tokens, TokenKind::Plus);
                        
                        // 3. On lit l'expression intérieure
                        self.read_interpolated_expression(tokens)?;
                        
                        // 4. Au retour, on ajoute un autre '+'
                        self.add_token(tokens, TokenKind::Plus);
                    } else {
                        string_content.push('$');
                    }
                },
                '\\' => { 
                    self.chars.next();
                    if let Some(escaped) = self.chars.next() {
                        match escaped {
                            'n' => string_content.push('\n'),
                            't' => string_content.push('\t'),
                            'r' => string_content.push('\r'),
                            '`' => string_content.push('`'),
                            '\\' => string_content.push('\\'),
                            _ => string_content.push(escaped),
                        }
                    }
                },
                _ => {
                    self.chars.next();
                    string_content.push(c);
                }
            }
        }

        Err(format!("Unterminated string literal starting at line {}", self.line))
    }

    // NOUVELLE MÉTHODE : Lit une expression à l'intérieur de ${...}
    fn read_interpolated_expression(&mut self, tokens: &mut Vec<Token>) -> Result<(), String> {
        let mut balance = 1; // On a déjà consommé le '{' ouvrant

        while balance > 0 {
            if self.chars.peek().is_none() {
                return Err("Unclosed string interpolation".to_string());
            }

            // Gestion manuelle des accolades pour l'imbrication
            if let Some(&'}') = self.chars.peek() {
                self.chars.next();
                balance -= 1;
                if balance == 0 {
                    return Ok(()); // Fin de l'interpolation
                }
                self.add_token(tokens, TokenKind::RBrace);
                continue;
            }
            
            if let Some(&'{') = self.chars.peek() {
                self.chars.next();
                balance += 1;
                self.add_token(tokens, TokenKind::LBrace);
                continue;
            }

            // Pour tout le reste, on utilise le scanner standard
            self.scan_token(tokens)?;
        }
        Ok(())
    }
}
