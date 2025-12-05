use super::lexer::Token;
use serde_json::{json, Value};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> Result<Value, String> {
        let mut instructions = Vec::new();
        while !self.is_at_end() {
            instructions.push(self.parse_statement()?);
        }
        Ok(json!(instructions))
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        &self.tokens[self.pos - 1]
    }

    fn match_token(&mut self, token: Token) -> bool {
        if self.check(&token) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, token: &Token) -> bool {
        if self.is_at_end() { return false; }
        // Comparaison approximative car Token::Identifier porte une donnée
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }

    fn is_at_end(&self) -> bool {
        self.peek() == &Token::EOF
    }

    fn consume(&mut self, expected: Token, msg: &str) -> Result<&Token, String> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(format!("{} (Got {:?})", msg, self.peek()))
        }
    }

    fn parse_params_list(&mut self) -> Result<Value, String> {
        self.consume(Token::LParen, "Expect '('")?;
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                if let Token::Identifier(p) = self.advance() { params.push(p.clone()); }
                if !self.match_token(Token::Comma) { break; }
            }
        }
        self.consume(Token::RParen, "Expect ')'")?;
        Ok(json!(params))
    }

    // --- Statements ---

    fn parse_statement(&mut self) -> Result<Value, String> {
        match self.peek() {
            Token::Var => self.parse_var(),
            Token::Print => self.parse_print(),
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Func => self.parse_func(),
            Token::Class => self.parse_class(),
            Token::Return => self.parse_return(),
            Token::Input => self.parse_input(),
            Token::Break => { self.advance(); Ok(json!(["break"])) },
            Token::Import => self.parse_import(),
            Token::Try => self.parse_try(),
            Token::Switch => self.parse_switch(),
            
            // Cas générique pour Identifiants (Variables, Appels, Attributs)
            Token::Identifier(_) => {
                // 1. On parse l'expression complète (ex: "x", "this.nom", "obj.method()")
                // Cela consomme les tokens correctement, y compris les points.
                let expr = self.parse_expression()?;
                
                // 2. On regarde si c'est suivi d'un signe égal "=" (Assignation)
                if self.match_token(Token::Eq) {
                    let value = self.parse_expression()?;
                    
                    // 3. On transforme l'expression de lecture en instruction d'écriture
                    // On inspecte le JSON généré par parse_expression
                    if let Some(arr) = expr.as_array() {
                        let cmd = arr[0].as_str().unwrap_or("");
                        
                        // Cas 1: Variable simple ["get", "x"] -> ["set", "x", val]
                        if cmd == "get" {
                            let name = &arr[1];
                            return Ok(json!(["set", name, value]));
                        }
                        
                        // Cas 2: Attribut ["get_attr", obj, "attr"] -> ["set_attr", obj, "attr", val]
                        if cmd == "get_attr" {
                            let obj = &arr[1];
                            let attr = &arr[2];
                            return Ok(json!(["set_attr", obj, attr, value]));
                        }
                    }
                    
                    return Err("Cible d'assignation invalide (doit être une variable ou un attribut)".to_string());
                }
                
                // Si pas de "=", c'était juste une expression (ex: appel de fonction)
                Ok(expr)
            },
            
            _ => Err(format!("Unexpected token at start of statement: {:?}", self.peek())),
        }
    }

    fn parse_block(&mut self) -> Result<Value, String> {
        self.consume(Token::LBrace, "Expect '{' before block")?;
        let mut block = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            block.push(self.parse_statement()?);
        }
        self.consume(Token::RBrace, "Expect '}' after block")?;
        Ok(json!(block))
    }

    fn parse_var(&mut self) -> Result<Value, String> {
        self.advance(); // Eat 'var'
        let name = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect var name".into()); };
        
        let expr = if self.match_token(Token::Eq) {
            self.parse_expression()?
        } else {
            json!(null)
        };
        Ok(json!(["set", name, expr]))
    }

    fn parse_print(&mut self) -> Result<Value, String> {
        self.advance();
        let expr = self.parse_expression()?;
        Ok(json!(["print", expr]))
    }

    fn parse_input(&mut self) -> Result<Value, String> {
        self.advance();
        let name = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect var name".into()); };
        let prompt = self.parse_expression()?;
        Ok(json!(["input", name, prompt]))
    }

    fn parse_import(&mut self) -> Result<Value, String> {
        self.advance(); // Eat 'import' keyword

        // We expect a StringLiteral containing the file path
        let path = match self.advance() {
            Token::StringLiteral(s) => s.clone(),
            _ => return Err("Expect file path (string) after 'import'".into()),
        };

        // Output JSON: ["import", "path/to/file.ext"]
        Ok(json!(["import", path]))
    }

    fn parse_try(&mut self) -> Result<Value, String> {
        self.advance(); // Mange 'try'
        
        let try_body = self.parse_block()?;
        
        self.consume(Token::Catch, "Expect 'catch' after try block")?;
        self.consume(Token::LParen, "Expect '(' after catch")?;
        
        let error_var = if let Token::Identifier(n) = self.advance() {
            n.clone()
        } else {
            return Err("Expect error variable name".into());
        };
        
        self.consume(Token::RParen, "Expect ')' after catch variable")?;
        
        let catch_body = self.parse_block()?;
        
        // Format JSON: ["try", [try_body], "err_var_name", [catch_body]]
        Ok(json!(["try", try_body, error_var, catch_body]))
    }

    fn parse_switch(&mut self) -> Result<Value, String> {
        self.advance(); // Eat 'switch'
        self.consume(Token::LParen, "Expect '(' after switch")?;
        let value = self.parse_expression()?;
        self.consume(Token::RParen, "Expect ')' after value")?;
        self.consume(Token::LBrace, "Expect '{' to start switch block")?;

        let mut cases = Vec::new();
        let mut default_body = Vec::new();

        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if self.match_token(Token::Case) {
                // Case definition: case expr:
                let case_val = self.parse_expression()?;
                self.consume(Token::Colon, "Expect ':' after case value")?;
                
                // Read instructions until the next 'case', 'default', or '}'
                let mut body = Vec::new();
                while !self.check(&Token::Case) && !self.check(&Token::Default) && !self.check(&Token::RBrace) {
                    body.push(self.parse_statement()?);
                }
                
                // Add to cases list
                // Format: [case_val, [instructions]]
                cases.push(json!([case_val, body]));

            } else if self.match_token(Token::Default) {
                self.consume(Token::Colon, "Expect ':' after default")?;
                
                while !self.check(&Token::Case) && !self.check(&Token::Default) && !self.check(&Token::RBrace) {
                    default_body.push(self.parse_statement()?);
                }
            } else {
                return Err(format!("Unexpected token inside switch: {:?}", self.peek()));
            }
        }

        self.consume(Token::RBrace, "Expect '}' to end switch block")?;

        // JSON AST: ["switch", value_expr, [[case1_val, body], ...], [default_body]]
        Ok(json!(["switch", value, cases, default_body]))
    }

    fn parse_if(&mut self) -> Result<Value, String> {
        self.advance();
        self.consume(Token::LParen, "Expect '('")?;
        let condition = self.parse_expression()?;
        self.consume(Token::RParen, "Expect ')'")?;
        
        let true_block = self.parse_block()?;
        let mut false_block = json!([]);

        if self.match_token(Token::Else) {
            if self.check(&Token::If) {
                false_block = json!([self.parse_if()?]);
            } else {
                false_block = self.parse_block()?;
            }
        }
        
        // Si false_block est vide, on renvoie une liste à 3 éléments, sinon 4
        if false_block.as_array().unwrap().is_empty() {
             Ok(json!(["if", condition, true_block]))
        } else {
             Ok(json!(["if", condition, true_block, false_block]))
        }
    }

    fn parse_while(&mut self) -> Result<Value, String> {
        self.advance();
        self.consume(Token::LParen, "Expect '('")?;
        let cond = self.parse_expression()?;
        self.consume(Token::RParen, "Expect ')'")?;
        let body = self.parse_block()?;
        Ok(json!(["while", cond, body]))
    }

    fn parse_for(&mut self) -> Result<Value, String> {
        self.advance(); // Mange le mot-clé 'for'
        self.consume(Token::LParen, "Expect '(' after for")?;
        
        // 1. Variable d'itération
        let var_name = if let Token::Identifier(n) = self.advance() { 
            n.clone() 
        } else { 
            return Err("Expect variable name in for loop".into()); 
        };
        
        self.consume(Token::Comma, "Expect ',' after variable")?;
        
        // 2. Start
        let start = self.parse_expression()?;
        self.consume(Token::Comma, "Expect ',' after start")?;
        
        // 3. End
        let end = self.parse_expression()?;
        self.consume(Token::Comma, "Expect ',' after end")?;
        
        // 4. Step
        let step = self.parse_expression()?;
        
        self.consume(Token::RParen, "Expect ')' after for arguments")?;
        
        // 5. Body
        let body = self.parse_block()?;
        
        // Génère l'instruction JSON que le runtime Rust attend :
        // ["for_range", "var_name", start, end, step, [body]]
        Ok(json!(["for_range", var_name, start, end, step, body]))
    }

    fn parse_class(&mut self) -> Result<Value, String> {
        self.advance(); // Eat 'class'
        let name = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect class name".into()); };
        
        let params = self.parse_params_list()?;
        
        let mut parent = Value::Null;
        if self.match_token(Token::Extends) {
            if let Token::Identifier(n) = self.advance() { parent = json!(n); }
        }

        self.consume(Token::LBrace, "{")?;
        let mut methods = serde_json::Map::new();
        
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            // Method definition: name(params) { body }
            let m_name = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect method name".into()); };
            let m_params = self.parse_params_list()?;
            let m_body = self.parse_block()?;
            
            // Format for AST: [params, body]
            methods.insert(m_name, json!([m_params, m_body]));
        }
        self.consume(Token::RBrace, "}")?;

        if parent.is_null() {
            Ok(json!(["class", name, params, methods]))
        } else {
            Ok(json!(["class", name, params, methods, parent]))
        }
    }

    fn parse_return(&mut self) -> Result<Value, String> {
        self.advance();
        let expr = self.parse_expression()?;
        Ok(json!(["return", expr]))
    }

    fn parse_func(&mut self) -> Result<Value, String> {
        self.advance();
        let name = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect func name".into()); };
        
        self.consume(Token::LParen, "Expect '('")?;
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                if let Token::Identifier(p) = self.advance() { params.push(p.clone()); }
                if !self.match_token(Token::Comma) { break; }
            }
        }
        self.consume(Token::RParen, "Expect ')'")?;
        let body = self.parse_block()?;
        
        Ok(json!(["function", name, params, body]))
    }

    // --- Expressions (Pratt Parsing simplifié ou Recursive Descent) ---
    // Pour simplifier, on fait: Logic > Additive > Multiplicative > Primary

    fn parse_expression(&mut self) -> Result<Value, String> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Value, String> {
        let mut left = self.parse_logical_and()?;
        while self.match_token(Token::Or) {
            let right = self.parse_logical_and()?;
            left = json!(["||", left, right]);
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Value, String> {
        let mut left = self.parse_equality()?;
        while self.match_token(Token::And) {
            let right = self.parse_equality()?;
            left = json!(["&&", left, right]);
        }
        Ok(left)
    }

    // Renomme ton ancien "parse_logic" en "parse_equality" et ajoute !=
    fn parse_equality(&mut self) -> Result<Value, String> {
        let mut left = self.parse_relational()?; // Appel vers comparaison
        while let Token::EqEq | Token::Neq = self.peek() {
            let op = match self.advance() {
                Token::EqEq => "==",
                Token::Neq => "!=",
                _ => unreachable!(),
            };
            let right = self.parse_relational()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    // Nouvelle fonction pour <, >, <=, >=
    fn parse_relational(&mut self) -> Result<Value, String> {
        let mut left = self.parse_additive()?;
        while let Token::Lt | Token::Gt | Token::LtEq | Token::GtEq = self.peek() {
             let op = match self.advance() {
                Token::Lt => "<",
                Token::Gt => ">",
                Token::LtEq => "<=",
                Token::GtEq => ">=",
                _ => unreachable!(),
            };
            let right = self.parse_additive()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Value, String> {
        let mut left = self.parse_multiplicative()?;
        while let Token::Plus | Token::Minus = self.peek() {
            let op = match self.advance() {
                Token::Plus => "+",
                Token::Minus => "-",
                _ => unreachable!()
            };
            let right = self.parse_multiplicative()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Value, String> {
        let mut left = self.parse_unary()?;
        while let Token::Star | Token::Slash  | Token::Percent = self.peek() {
            let op = match self.advance() {
                Token::Star => "*",
                Token::Slash => "/",
                Token::Percent => "%",
                _ => unreachable!()
            };
            let right = self.parse_unary()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Value, String> {
        if self.match_token(Token::Bang) {
            let right = self.parse_unary()?;
            return Ok(json!(["!", right]));
        }

        if self.match_token(Token::Minus) {
            // On a trouvé un '-' unaire (ex: -5 ou -x)
            // On parse récursivement ce qui suit (pour gérer --5 par exemple)
            let right = self.parse_unary()?;
            
            // ASTuce : On transforme "-x" en "0 - x"
            // Comme ça, l'interpréteur utilise la soustraction qu'il connait déjà.
            return Ok(json!(["-", json!(0), right]));
        }

        // Si ce n'est pas un opérateur unaire, c'est une expression primaire standard
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Value, String> {
        match self.peek() {
            Token::Integer(n) => { let v = *n; self.advance(); Ok(json!(v)) },
            Token::Float(f) => { let v = *f; self.advance(); Ok(json!(v)) },
            Token::StringLiteral(s) => { let v = s.clone(); self.advance(); Ok(json!(v)) },
            
            Token::True => { self.advance(); Ok(json!(true)) },
            Token::False => { self.advance(); Ok(json!(false)) },
            
            Token::LBracket => {
                self.advance(); // Mange le '['
                let mut elements = Vec::new();
                
                if !self.check(&Token::RBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(Token::Comma) { break; }
                    }
                }
                
                self.consume(Token::RBracket, "Expect ']' after list")?;
                
                // IMPORTANT : On utilise un mot-clé spécial pour le runtime
                let mut ast = vec![json!("make_list")];
                ast.extend(elements);
                Ok(json!(ast))
            },

            Token::LBrace => {
                self.advance(); // Mange le '{'
                let mut entries = Vec::new(); // Sera une liste de paires [key, value]
                
                if !self.check(&Token::RBrace) {
                    loop {
                        // Clé (String ou Identifiant)
                        let key = match self.advance() {
                            Token::StringLiteral(s) => s.clone(),
                            Token::Identifier(s) => s.clone(),
                            _ => return Err("Dict key must be string or identifier".into())
                        };
                        
                        self.consume(Token::Colon, "Expect ':' after dict key")?;
                        let value = self.parse_expression()?;
                        
                        entries.push(json!([key, value])); // On stocke la paire
                        
                        if !self.match_token(Token::Comma) { break; }
                    }
                }
                
                self.consume(Token::RBrace, "Expect '}' after dict")?;
                
                // Structure JSON intermédiaire : ["make_dict", [k, v], [k, v]...]
                let mut ast = vec![json!("make_dict")];
                ast.extend(entries);
                Ok(json!(ast))
            },
            Token::New => {
                self.advance(); // Mange le mot-clé 'new'
                
                // 1. On attend le nom de la classe (Identifier)
                let class_name = if let Token::Identifier(n) = self.advance() {
                    n.clone()
                } else {
                    return Err("Expect class name after 'new'".to_string());
                };

                // 2. On attend les parenthèses et les arguments
                self.consume(Token::LParen, "Expect '(' after class name")?;
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(Token::Comma) { break; }
                    }
                }
                self.consume(Token::RParen, "Expect ')' after arguments")?;

                // 3. On construit l'AST JSON: ["new", "ClassName", arg1, arg2...]
                let mut new_expr = vec![json!("new"), json!(class_name)];
                new_expr.extend(args);
                Ok(json!(new_expr))
            },
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance(); // Consume the identifier (e.g., "x", "this", "console")
                
                // 1. Initial Expression: Is it a variable or a direct function call?
                let mut expr = if self.match_token(Token::LParen) {
                    // It's a function call: func(...)
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.match_token(Token::Comma) { break; }
                        }
                    }
                    self.consume(Token::RParen, "Expect ')'")?;
                    
                    // Native Allowlist logic
                    let native_commands = vec!["to_int", "len", "str"];
                    if native_commands.contains(&name.as_str()) {
                        let mut call = vec![json!(name)];
                        call.extend(args);
                        json!(call)
                    } else {
                        let mut call = vec![json!("call"), json!(name)];
                        call.extend(args);
                        json!(call)
                    }
                } else {
                    // It's a simple variable access: var
                    json!(["get", name])
                };

                // 2. Member Access Loop: Handle chains like `obj.prop` or `obj.method()`
                while self.match_token(Token::Dot) {
                    // We found a dot, we expect a property name next
                    let member_name = if let Token::Identifier(n) = self.advance() {
                        n.clone()
                    } else {
                        return Err("Expect property name after '.'".to_string());
                    };

                    if self.match_token(Token::LParen) {
                        // It is a method call: obj.method(...)
                        let mut args = Vec::new();
                        if !self.check(&Token::RParen) {
                            loop {
                                args.push(self.parse_expression()?);
                                if !self.match_token(Token::Comma) { break; }
                            }
                        }
                        self.consume(Token::RParen, "Expect ')'")?;

                        // Construct AST for CallMethod: ["call_method", obj_expr, "method_name", arg1, arg2...]
                        let mut call = vec![json!("call_method"), expr, json!(member_name)];
                        call.extend(args);
                        expr = json!(call);
                    } else {
                        // It is a property access: obj.field
                        // Construct AST for GetAttr: ["get_attr", obj_expr, "field_name"]
                        expr = json!(["get_attr", expr, member_name]);
                    }
                }

                Ok(expr)
            },
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(Token::RParen, "Expect ')'")?;
                Ok(expr)
            },
            _ => Err(format!("Unexpected token in expression: {:?}", self.peek())),
        }
    }
}
