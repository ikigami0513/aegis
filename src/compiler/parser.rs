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
                if let Token::Identifier(p) = self.advance() { 
                    let p_name = p.clone();
                    let p_type = self.parse_type_annotation()?; // Parse ": type"
                    params.push(json!([p_name, p_type]));
                }
                if !self.match_token(Token::Comma) { break; }
            }
        }
        self.consume(Token::RParen, "Expect ')'")?;
        Ok(json!(params))
    }

    // --- Statements ---

    fn parse_statement(&mut self) -> Result<Value, String> {
        match self.peek() {
            Token::At => self.parse_decorated_function(),
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
            Token::Namespace => self.parse_namespace(),
            
            // Cas générique pour Identifiants (Variables, Appels, Attributs)
            Token::Identifier(_) => {
                // 1. On parse l'expression complète (ex: "x", "this.nom", "obj.method()")
                // Cela consomme les tokens correctement, y compris les points.
                let expr = self.parse_expression()?;

                match self.peek() {
                    Token::Eq => {
                        self.advance();
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
                        
                        return self.convert_to_assignment(expr, value);
                    },
                    // Case: i++  =>  i = i + 1
                    Token::PlusPlus => {
                        self.advance();
                        let one = json!(1);
                        // We construct: expr = expr + 1
                        let new_val = json!(["+", expr.clone(), one]);
                        return self.convert_to_assignment(expr, new_val);
                    },
                    // Case: i--  =>  i = i - 1
                    Token::MinusMinus => {
                        self.advance();
                        let one = json!(1);
                        let new_val = json!(["-", expr.clone(), one]);
                        return self.convert_to_assignment(expr, new_val);
                    },
                    // Case: x += 10  =>  x = x + 10
                    Token::PlusEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["+", expr.clone(), val]);
                        return self.convert_to_assignment(expr, new_val);
                    },
                    Token::MinusEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["-", expr.clone(), val]);
                        return self.convert_to_assignment(expr, new_val);
                    },
                    Token::StarEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["*", expr.clone(), val]);
                        return self.convert_to_assignment(expr, new_val);
                    },
                    Token::SlashEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["/", expr.clone(), val]);
                        return self.convert_to_assignment(expr, new_val);
                    },
                    
                    // -----------------------------------------------
                    
                    _ => return Ok(expr) // Just an expression statement (like a function call)
                }
            },
            
            _ => Err(format!("Unexpected token at start of statement: {:?}", self.peek())),
        }
    }

    // Helper to transform a getter expression into a setter instruction
    fn convert_to_assignment(&self, target: Value, value: Value) -> Result<Value, String> {
        if let Some(arr) = target.as_array() {
            let cmd = arr[0].as_str().unwrap_or("");
            
            if cmd == "get" {
                let name = &arr[1];
                return Ok(json!(["set", name, value]));
            }
            if cmd == "get_attr" {
                let obj = &arr[1];
                let attr = &arr[2];
                return Ok(json!(["set_attr", obj, attr, value]));
            }
        }
        Err("Invalid assignment target".to_string())
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
        self.advance(); // Mange 'var'

        // --- CAS 1 : Déstructuration de Liste : var [x, y] = ... ---
        if self.match_token(Token::LBracket) {
            let mut vars = Vec::new();
            
            // 1. Lire la liste des variables cibles [a, b, c]
            if !self.check(&Token::RBracket) {
                loop {
                    if let Token::Identifier(n) = self.advance() {
                        vars.push(n.clone());
                    } else {
                        return Err("Expect variable name in destructuring".into());
                    }
                    if !self.match_token(Token::Comma) { break; }
                }
            }
            self.consume(Token::RBracket, "Expect ']' after var list")?;
            self.consume(Token::Eq, "Expect '=' after destructuring list")?;
            
            // 2. Lire l'expression source (la liste à déballer)
            let expr = self.parse_expression()?;
            
            // 3. Génération du code (Desugaring)
            let mut instructions = Vec::new();
            
            // On utilise un nom unique pour stocker la liste temporairement
            // (pour éviter d'évaluer l'expression plusieurs fois)
            let temp_name = format!("__destruct_temp_{}", vars.len()); 
            
            // Instruction A: var __temp = expr
            instructions.push(json!(["set", temp_name, null, expr]));
            
            // Instruction B...N: var x = __temp.at(0), var y = __temp.at(1)...
            for (i, var_name) in vars.iter().enumerate() {
                // Construction de l'appel : __temp.at(i)
                let access = json!([
                    "call_method", 
                    ["get", temp_name], 
                    "at", 
                    [json!(i as i64)]
                ]);
                
                // Instruction: set var_name = __temp.at(i)
                instructions.push(json!(["set", var_name, null, access]));
            }
            
            // ASTUCE : On retourne un bloc "if (true)" contenant nos instructions
            // Cela permet de retourner une seule "Value" au parser principal
            // tout en exécutant plusieurs lignes.
            return Ok(json!(["if", json!(true), instructions]));
        }

        // --- CAS 2 : Variable Classique : var x: type = ... ---
        let name = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect var name".into()); };
        
        let type_annot = self.parse_type_annotation()?; 

        let expr = if self.match_token(Token::Eq) {
            self.parse_expression()?
        } else {
            json!(null)
        };
        
        Ok(json!(["set", name, type_annot, expr]))
    }

    fn parse_type_annotation(&mut self) -> Result<Option<String>, String> {
        if self.match_token(Token::Colon) {
            if let Token::Identifier(t) = self.advance() {
                Ok(Some(t.clone()))
            } else {
                Err("Expect type name after ':'".into())
            }
        } else {
            Ok(None)
        }
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

    fn parse_namespace(&mut self) -> Result<Value, String> {
        self.advance(); // Eat 'namespace'
        
        // 1. Get the namespace name
        let name = if let Token::Identifier(n) = self.advance() {
            n.clone()
        } else {
            return Err("Expect namespace name".into());
        };
        
        // 2. Parse the block content
        let body = self.parse_block()?;
        
        // JSON: ["namespace", "Name", [instructions]]
        Ok(json!(["namespace", name, body]))
    }

    fn parse_decorated_function(&mut self) -> Result<Value, String> {
        // 1. On mange le '@'
        self.advance();
        
        // 2. On récupère le nom du décorateur (ex: "log")
        let decorator_name = if let Token::Identifier(n) = self.advance() {
            n.clone()
        } else {
            return Err("Expect decorator name after '@'".into());
        };

        // 3. On s'attend à trouver une fonction juste après
        self.consume(Token::Func, "Expect 'func' after decorator")?;
        
        // 4. On parse le nom de la fonction cible (ex: "dire_bonjour")
        let func_name = if let Token::Identifier(n) = self.advance() {
            n.clone()
        } else {
            return Err("Expect function name".into());
        };

        // 5. On parse les paramètres et le corps (comme une lambda)
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

        // 6. Construction de l'AST équivalent à :
        // var func_name = decorator_name( func(params){ body } )
        
        let lambda = json!(["lambda", params, body]);
        let decorator_var = json!(["get", decorator_name]);
        
        // L'appel : decorator(lambda)
        let call_expr = json!(["call", decorator_var, [lambda]]);
        
        // L'assignation : var func_name = ...
        Ok(json!(["set", func_name, call_expr]))
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
        
        // Params contient maintenant [[name, type], [name, type]...]
        let params = self.parse_params_list()?;

        let mut ret_type = Value::Null;
        if self.match_token(Token::Arrow) {
             if let Token::Identifier(t) = self.advance() {
                 ret_type = json!(t);
             } else {
                 return Err("Expect return type after '->'".into());
             }
        }

        let body = self.parse_block()?;

        Ok(json!(["function", name, params, ret_type, body]))
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
        let mut left = self.parse_bitwise()?;
        while let Token::Lt | Token::Gt | Token::LtEq | Token::GtEq = self.peek() {
             let op = match self.advance() {
                Token::Lt => "<",
                Token::Gt => ">",
                Token::LtEq => "<=",
                Token::GtEq => ">=",
                _ => unreachable!(),
            };
            let right = self.parse_bitwise()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_bitwise(&mut self) -> Result<Value, String> {
        let mut left = self.parse_additive()?;
        
        while let Token::BitAnd | Token::BitOr | Token::BitXor | Token::ShiftLeft | Token::ShiftRight = self.peek() {
            let op = match self.advance() {
                Token::BitAnd => "&",
                Token::BitOr => "|",
                Token::BitXor => "^",
                Token::ShiftLeft => "<<",
                Token::ShiftRight => ">>",
                _ => unreachable!()
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

    fn parse_interpolated_string(&self, source: &str) -> Result<Value, String> {
        let mut parts = Vec::new();
        let mut current_text = String::new();
        let mut chars = source.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '$' {
                if let Some(&'{') = chars.peek() {
                    chars.next(); // Eat '{'
                    
                    if !current_text.is_empty() {
                        parts.push(json!(current_text.clone()));
                        current_text.clear();
                    }

                    // Extraction intelligente
                    let mut code_snippet = String::new();
                    let mut format_specifier = String::new();
                    let mut brace_count = 1;
                    let mut found_colon = false;
                    
                    while let Some(code_char) = chars.next() {
                        if code_char == '}' {
                            brace_count -= 1;
                            if brace_count == 0 { break; }
                        } else if code_char == '{' {
                            brace_count += 1;
                        }

                        // Si on trouve ':' et qu'on est au niveau 1 (pas dans un dict imbriqué)
                        if code_char == ':' && brace_count == 1 && !found_colon {
                            found_colon = true;
                            continue; // On ne l'ajoute pas au code, on switch de mode
                        }

                        if found_colon {
                            format_specifier.push(code_char);
                        } else {
                            code_snippet.push(code_char);
                        }
                    }
                    
                    if brace_count > 0 { return Err("Unterminated interpolation".into()); }

                    // Compilation du snippet
                    let mut sub_lexer = super::lexer::Lexer::new(&code_snippet);
                    let sub_tokens = sub_lexer.tokenize();
                    let mut sub_parser = Parser::new(sub_tokens);
                    let expr = sub_parser.parse_expression()?;
                    
                    if !format_specifier.is_empty() {
                        // Si format détecté -> Appelle fmt(expr, format)
                        // AST: ["call", ["get", "fmt"], [expr, format_string]]
                        let fmt_call = json!(["call", ["get", "fmt"], [expr, json!(format_specifier)]]);
                        parts.push(fmt_call);
                    } else {
                        parts.push(expr);
                    }
                    
                    continue;
                }
            }
            current_text.push(c);
        }
        
        if !current_text.is_empty() { parts.push(json!(current_text)); }
        if parts.is_empty() { return Ok(json!("")); }
        
        let mut final_expr = parts[0].clone();
        for i in 1..parts.len() {
            final_expr = json!(["+", final_expr, parts[i]]);
        }

        Ok(final_expr)
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
        let mut expr =  match self.peek() {
            Token::Integer(n) => { let v = *n; self.advance(); json!(v) },
            Token::Float(f) => { let v = *f; self.advance(); json!(v) },
            Token::StringLiteral(s) => { 
                let raw_string = s.clone();
                self.advance(); 
                
                // Check for interpolation marker "${"
                if raw_string.contains("${") {
                    return self.parse_interpolated_string(&raw_string);
                }

                return Ok(json!(raw_string));
             },
            
            Token::True => { self.advance(); json!(true) },
            Token::False => { self.advance(); json!(false) },

            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                json!(["get", name])
            },

            Token::Func => {
                self.advance(); // Eat 'func'
                self.consume(Token::LParen, "Expect '('")?;
                // Réutilisation de ta logique de parsing de params (astuce: extraire parse_params en helper si besoin, ou copier la logique ici)
                let mut params = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        if let Token::Identifier(p) = self.advance() { params.push(p.clone()); }
                        if !self.match_token(Token::Comma) { break; }
                    }
                }
                self.consume(Token::RParen, "Expect ')'")?;
                let body = self.parse_block()?;
                
                // On retourne une expression de type Function
                // JSON: ["lambda", [params], [body]]
                json!(["lambda", params, body])
            },

            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume(Token::RParen, "Expect ')'")?;
                expr
            },
            
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
                json!(ast)
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
                json!(ast)
            },
            Token::New => {
                self.advance(); // Mange 'new'
                
                // 1. On parse le nom de la classe.
                // Attention : ça peut être "Chien" ou "Math.Vector2".
                // On utilise parse_identifier_chain (logique simplifiée ici)
                
                let mut expr = if let Token::Identifier(n) = self.advance() {
                    json!(["get", n.clone()])
                } else {
                    return Err("Expect class name after new".into());
                };
                
                // On gère les points (Math.Vector2.SubClass...)
                while self.match_token(Token::Dot) {
                    if let Token::Identifier(member) = self.advance() {
                        expr = json!(["get_attr", expr, member.clone()]);
                    } else {
                        return Err("Expect member name after dot".into());
                    }
                }

                // 2. Les arguments ( ... )
                self.consume(Token::LParen, "Expect '(' after class ref")?;
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(Token::Comma) { break; }
                    }
                }
                self.consume(Token::RParen, "Expect ')'")?;
                
                // JSON: ["new", expression_cible, [args]]
                let mut new_cmd = vec![json!("new"), expr];
                new_cmd.extend(args);
                json!(new_cmd)
            },
            _ => return Err(format!("Unexpected token in expression: {:?}", self.peek()))
        };

        loop {
            if self.match_token(Token::LParen) {
                // C'est un APPEL : expr(...)
                let mut args = Vec::new();
                if !self.check(&Token::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(Token::Comma) { break; }
                    }
                }
                self.consume(Token::RParen, "Expect ')'")?;
                // JSON: ["call", target_expr, [args]]
                expr = json!(["call", expr, args]);
                
            } else if self.match_token(Token::Dot) {
                // C'est un ACCÈS : expr.prop
                let member = if let Token::Identifier(n) = self.advance() { n.clone() } else { return Err("Expect member name".into()); };
                
                // Petite subtilité pour les méthodes : obj.method()
                // Ton ancien code gérait "call_method". 
                // Avec les fonctions first-class, obj.method() est techniquement (obj.method)()
                // Mais pour garder le support des méthodes natives (split, trim) qui ne sont pas des variables,
                // gardons la détection "si suivi de ( alors call_method".
                
                if self.match_token(Token::LParen) {
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if !self.match_token(Token::Comma) { break; }
                        }
                    }
                    self.consume(Token::RParen, "Expect ')'")?;
                    expr = json!(["call_method", expr, member, args]);
                } else {
                    expr = json!(["get_attr", expr, member]);
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }
}
