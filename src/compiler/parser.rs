use super::lexer::{ Token, TokenKind };
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

    // --- Helpers ---

    fn peek(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }

    fn current_line(&self) -> usize {
        if self.is_at_end() {
            if !self.tokens.is_empty() {
                self.tokens[self.tokens.len() - 1].line
            } else {
                1
            }
        } else {
            self.tokens[self.pos].line
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.pos += 1;
        }
        &self.tokens[self.pos - 1]
    }

    fn match_token(&mut self, kind: TokenKind) -> bool {
        if self.check(&kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() { return false; }
        std::mem::discriminant(self.peek()) == std::mem::discriminant(kind)
    }

    fn is_at_end(&self) -> bool {
        self.peek() == &TokenKind::EOF
    }

    fn consume(&mut self, expected: TokenKind, msg: &str) -> Result<&Token, String> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(format!("{} (Line {})", msg, self.current_line()))
        }
    }

    // --- Statements ---

    fn parse_statement(&mut self) -> Result<Value, String> {
        match self.peek() {
            TokenKind::At => self.parse_decorated_function(),
            TokenKind::Var => self.parse_var(),
            TokenKind::Print => self.parse_print(),
            TokenKind::If => self.parse_if(),
            TokenKind::While => self.parse_while(),
            TokenKind::For => self.parse_for(),
            TokenKind::Func => self.parse_func(),
            TokenKind::Class => self.parse_class(),
            TokenKind::Enum => self.parse_enum(),
            TokenKind::Return => self.parse_return(),
            TokenKind::Input => self.parse_input(),
            TokenKind::Break => { 
                let line = self.current_line();
                self.advance(); 
                Ok(json!(["break", line])) 
            },
            TokenKind::Import => self.parse_import(),
            TokenKind::Try => self.parse_try(),
            TokenKind::Throw => self.parse_throw(),
            TokenKind::Switch => self.parse_switch(),
            TokenKind::Namespace => self.parse_namespace(),
            TokenKind::Const => self.parse_const(),
            TokenKind::ForEach => self.parse_foreach(),
            
            TokenKind::Identifier(_) | TokenKind::Super => {
                let line = self.current_line();
                let expr = self.parse_expression()?;

                match self.peek() {
                    TokenKind::Eq => {
                        self.advance();
                        let value = self.parse_expression()?;
                        return self.convert_to_assignment(line, expr, value);
                    },
                    TokenKind::PlusPlus => {
                        self.advance();
                        let one = json!(1);
                        let new_val = json!(["+", expr.clone(), one]);
                        return self.convert_to_assignment(line, expr, new_val);
                    },
                    TokenKind::MinusMinus => {
                        self.advance();
                        let one = json!(1);
                        let new_val = json!(["-", expr.clone(), one]);
                        return self.convert_to_assignment(line, expr, new_val);
                    },
                    TokenKind::PlusEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["+", expr.clone(), val]);
                        return self.convert_to_assignment(line, expr, new_val);
                    },
                    TokenKind::MinusEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["-", expr.clone(), val]);
                        return self.convert_to_assignment(line, expr, new_val);
                    },
                    TokenKind::StarEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["*", expr.clone(), val]);
                        return self.convert_to_assignment(line, expr, new_val);
                    },
                    TokenKind::SlashEq => {
                        self.advance();
                        let val = self.parse_expression()?;
                        let new_val = json!(["/", expr.clone(), val]);
                        return self.convert_to_assignment(line, expr, new_val);
                    },
                    _ => {
                        if let Some(arr) = expr.as_array() {
                            let mut new_arr = arr.clone();
                            if !new_arr.is_empty() {
                                if let Some(cmd) = new_arr[0].as_str() {
                                    if cmd == "call" || cmd == "call_method" || cmd == "super_call" {
                                        new_arr.insert(1, json!(line));
                                        return Ok(Value::Array(new_arr));
                                    }
                                }
                            }
                        }
                        Ok(expr) 
                    }
                }
            },

            TokenKind::Continue => {
                let line = self.current_line();
                self.advance(); 
                Ok(json!(["continue", line])) 
            },
            
            _ => Err(format!("Unexpected token at start of statement: {:?} (Line {})", self.peek(), self.current_line())),
        }
    }

    fn convert_to_assignment(&self, line: usize, target: Value, value: Value) -> Result<Value, String> {
        if let Some(arr) = target.as_array() {
            let cmd = arr[0].as_str().unwrap_or("");
            
            if cmd == "get" {
                let name = &arr[1];
                return Ok(json!(["set", line, name, null, value]));
            }
            if cmd == "get_attr" {
                let obj = &arr[1];
                let attr = &arr[2];
                return Ok(json!(["set_attr", line, obj, attr, value]));
            }
        }
        Err(format!("Invalid assignment target (Line {})", line))
    }

    fn parse_block(&mut self) -> Result<Value, String> {
        self.consume(TokenKind::LBrace, "Expect '{' before block")?;
        let mut block = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            block.push(self.parse_statement()?);
        }
        self.consume(TokenKind::RBrace, "Expect '}' after block")?;
        Ok(json!(block))
    }

    fn parse_var(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); 

        if self.match_token(TokenKind::LBracket) {
            let mut vars = Vec::new();
            if !self.check(&TokenKind::RBracket) {
                loop {
                    if let TokenKind::Identifier(n) = &self.advance().kind {
                        vars.push(n.clone());
                    } else {
                        return Err(format!("Expect variable name in destructuring (Line {})", line));
                    }
                    if !self.match_token(TokenKind::Comma) { break; }
                }
            }
            self.consume(TokenKind::RBracket, "Expect ']'")?;
            self.consume(TokenKind::Eq, "Expect '='")?;
            
            let expr = self.parse_expression()?;
            
            let mut instructions = Vec::new();
            let temp_name = format!("__destruct_temp_{}", vars.len()); 
            
            instructions.push(json!(["set", line, temp_name, null, expr]));
            
            for (i, var_name) in vars.iter().enumerate() {
                let access = json!([
                    "call_method", 
                    ["get", temp_name], 
                    "at", 
                    [json!(i as i64)]
                ]);
                instructions.push(json!(["set", line, var_name, null, access]));
            }
            
            return Ok(json!(["if", line, json!(true), instructions]));
        }

        let name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Expect var name".into()); };
        let type_annot = self.parse_type_annotation()?; 
        let expr = if self.match_token(TokenKind::Eq) { self.parse_expression()? } else { json!(null) };
        
        Ok(json!(["set", line, name, type_annot, expr]))
    }

    fn parse_type_annotation(&mut self) -> Result<Option<String>, String> {
        if self.match_token(TokenKind::Colon) {
            if let TokenKind::Identifier(t) = &self.advance().kind {
                Ok(Some(t.clone()))
            } else {
                Err(format!("Expect type name after ':' (Line {})", self.current_line()))
            }
        } else {
            Ok(None)
        }
    }

    fn parse_print(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let expr = self.parse_expression()?;
        Ok(json!(["print", line, expr]))
    }

    fn parse_return(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let expr = self.parse_expression()?;
        Ok(json!(["return", line, expr]))
    }

    fn parse_input(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Expect name".into()); };
        let prompt = self.parse_expression()?;
        Ok(json!(["input", line, name, prompt]))
    }

    fn parse_import(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let path = match &self.advance().kind {
            TokenKind::StringLiteral(s) => s.clone(),
            _ => return Err("Expect path".into()),
        };
        Ok(json!(["import", line, path]))
    }

    fn parse_try(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let try_body = self.parse_block()?;
        self.consume(TokenKind::Catch, "Expect catch")?;
        self.consume(TokenKind::LParen, "(")?;
        let err_var = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Expect error var".into()); };
        self.consume(TokenKind::RParen, ")")?;
        let catch_body = self.parse_block()?;
        Ok(json!(["try", line, try_body, err_var, catch_body]))
    }

    fn parse_throw(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); // Consomme 'throw'
        let expr = self.parse_expression()?;
        Ok(json!(["throw", line, expr]))
    }

    fn parse_switch(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        self.consume(TokenKind::LParen, "(")?;
        let val = self.parse_expression()?;
        self.consume(TokenKind::RParen, ")")?;
        self.consume(TokenKind::LBrace, "{")?;
        
        let mut cases = Vec::new();
        let mut default = Vec::new();
        
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            if self.match_token(TokenKind::Case) {
                let c_val = self.parse_expression()?;
                self.consume(TokenKind::Colon, ":")?;
                let mut body = Vec::new();
                while !self.check(&TokenKind::Case) && !self.check(&TokenKind::Default) && !self.check(&TokenKind::RBrace) {
                    body.push(self.parse_statement()?);
                }
                cases.push(json!([c_val, body]));
            } else if self.match_token(TokenKind::Default) {
                self.consume(TokenKind::Colon, ":")?;
                while !self.check(&TokenKind::Case) && !self.check(&TokenKind::Default) && !self.check(&TokenKind::RBrace) {
                    default.push(self.parse_statement()?);
                }
            } else {
                return Err("Unexpected in switch".into());
            }
        }
        self.consume(TokenKind::RBrace, "}")?;
        
        Ok(json!(["switch", line, val, cases, default]))
    }

    fn parse_namespace(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Ns Name".into()); };
        let body = self.parse_block()?;
        Ok(json!(["namespace", line, name, body]))
    }

    fn parse_const(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); // Eat 'const'
        
        let name = if let TokenKind::Identifier(n) = &self.advance().kind { 
            n.clone() 
        } else { 
            return Err("Expect constant name".into()); 
        };

        // Typage graduel optionnel (const PI: float = ...)
        // On consomme le type mais on l'ignore pour l'instant (ou on l'utilise pour check)
        let _type_annot = self.parse_type_annotation()?; 

        self.consume(TokenKind::Eq, "Expect '=' after constant name")?;
        
        let expr = self.parse_expression()?;
        
        // JSON: ["const", line, name, expr]
        Ok(json!(["const", line, name, expr]))
    }

    fn parse_foreach(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); // Eat 'foreach'
        
        self.consume(TokenKind::LParen, "Expect '(' after 'foreach'")?;
        
        // Nom de la variable (ex: "elem")
        let var_name = if let TokenKind::Identifier(n) = &self.advance().kind {
            n.clone()
        } else {
            return Err("Expect variable name in foreach".into());
        };
        
        self.consume(TokenKind::In, "Expect 'in' after variable name")?;
        
        // L'expression itérable (ex: "mylist" ou "[1, 2]")
        let iterable = self.parse_expression()?;
        
        self.consume(TokenKind::RParen, "Expect ')' after loop header")?;
        
        // Le corps
        let body = self.parse_block()?;
        
        // JSON: ["foreach", line, var_name, iterable, body]
        Ok(json!(["foreach", line, var_name, iterable, body]))
    }

    fn parse_decorated_function(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); // @
        let deco_name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Deco Name".into()); };
        self.consume(TokenKind::Func, "Func")?;
        let func_name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Func Name".into()); };
        
        self.consume(TokenKind::LParen, "(")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if let TokenKind::Identifier(p) = &self.advance().kind { params.push(p.clone()); }
                if !self.match_token(TokenKind::Comma) { break; }
            }
        }
        self.consume(TokenKind::RParen, ")")?;
        let body = self.parse_block()?;
        
        let lambda = json!(["lambda", params, body]);
        let deco_var = json!(["get", deco_name]);
        let call = json!(["call", deco_var, [lambda]]);
        
        Ok(json!(["set", line, func_name, null, call]))
    }

    fn parse_params_list(&mut self) -> Result<Value, String> {
        self.consume(TokenKind::LParen, "(")?;
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                if let TokenKind::Identifier(p) = &self.advance().kind {
                    let p_name = p.clone();
                    let p_type = self.parse_type_annotation()?;
                    params.push(json!([p_name, p_type]));
                }
                if !self.match_token(TokenKind::Comma) { break; }
            }
        }
        self.consume(TokenKind::RParen, ")")?;
        Ok(json!(params))
    }

    fn parse_if(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        self.consume(TokenKind::LParen, "(")?;
        let cond = self.parse_expression()?;
        self.consume(TokenKind::RParen, ")")?;
        let true_blk = self.parse_block()?;
        let mut false_blk = json!([]);
        
        if self.match_token(TokenKind::Else) {
            if self.check(&TokenKind::If) {
                false_blk = json!([self.parse_if()?]);
            } else {
                false_blk = self.parse_block()?;
            }
        }
        
        if false_blk.as_array().unwrap().is_empty() {
            Ok(json!(["if", line, cond, true_blk]))
        } else {
            Ok(json!(["if", line, cond, true_blk, false_blk]))
        }
    }

    fn parse_while(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        self.consume(TokenKind::LParen, "(")?;
        let cond = self.parse_expression()?;
        self.consume(TokenKind::RParen, ")")?;
        let body = self.parse_block()?;
        Ok(json!(["while", line, cond, body]))
    }

    fn parse_for(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        self.consume(TokenKind::LParen, "(")?;
        let var = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("For var".into()); };
        self.consume(TokenKind::Comma, ",")?;
        let start = self.parse_expression()?;
        self.consume(TokenKind::Comma, ",")?;
        let end = self.parse_expression()?;
        self.consume(TokenKind::Comma, ",")?;
        let step = self.parse_expression()?;
        self.consume(TokenKind::RParen, ")")?;
        let body = self.parse_block()?;
        
        Ok(json!(["for_range", line, var, start, end, step, body]))
    }

    fn parse_class(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); // Eat 'class'
        let name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Class Name".into()); };
        
        let mut parent = Value::Null;
        if self.match_token(TokenKind::Extends) {
            if let TokenKind::Identifier(n) = &self.advance().kind { parent = json!(n); }
        }
        
        self.consume(TokenKind::LBrace, "{")?;
        let mut methods = serde_json::Map::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            let m_name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Method Name".into()); };
            let m_params = self.parse_params_list()?;
            let m_body = self.parse_block()?;
            methods.insert(m_name, json!([m_params, m_body]));
        }
        self.consume(TokenKind::RBrace, "}")?;
        
        if parent.is_null() {
            Ok(json!(["class", line, name, methods]))
        } else {
            Ok(json!(["class", line, name, methods, parent]))
        }
    }

    fn parse_enum(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance(); // Eat 'enum'
        
        let name = if let TokenKind::Identifier(n) = &self.advance().kind { 
            n.clone() 
        } else { 
            return Err("Expect Enum Name".into()); 
        };

        self.consume(TokenKind::LBrace, "Expect '{'")?;
        
        let mut variants = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                if let TokenKind::Identifier(v) = &self.advance().kind {
                    variants.push(json!(v));
                } else {
                    return Err("Expect enum variant name".into());
                }
                
                // Virgule optionnelle pour le dernier élément ?
                if !self.match_token(TokenKind::Comma) { 
                    break; 
                }
            }
        }
        
        self.consume(TokenKind::RBrace, "Expect '}'")?;
        
        // JSON: ["enum", line, name, [variants...]]
        Ok(json!(["enum", line, name, variants]))
    }

    fn parse_func(&mut self) -> Result<Value, String> {
        let line = self.current_line();
        self.advance();
        let name = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Func Name".into()); };
        
        let params = self.parse_params_list()?;
        
        let mut ret_type = Value::Null;
        if self.match_token(TokenKind::Arrow) {
             if let TokenKind::Identifier(t) = &self.advance().kind {
                 ret_type = json!(t);
             }
        }
        let body = self.parse_block()?;
        
        Ok(json!(["function", line, name, params, ret_type, body]))
    }

    // --- Expression Parsing ---

    fn parse_expression(&mut self) -> Result<Value, String> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Value, String> {
        // On commence par parser le niveau inférieur (OR, AND...)
        let mut expr = self.parse_null_coalescing()?;

        // Si on rencontre '?', c'est un ternaire
        if self.match_token(TokenKind::Question) {
            let true_branch = self.parse_expression()?; // Récursif pour permettre l'imbrication
            self.consume(TokenKind::Colon, "Expect ':' in ternary operator")?;
            let false_branch = self.parse_ternary()?;   // Associativité à droite

            // Format JSON : ["?", condition, true_expr, false_expr]
            expr = json!(["?", expr, true_branch, false_branch]);
        }

        Ok(expr)
    }

    fn parse_null_coalescing(&mut self) -> Result<Value, String> {
        let mut expr = self.parse_logical_or()?;

        while self.match_token(TokenKind::DoubleQuestion) {
            let right = self.parse_logical_or()?;

            let line = self.current_line();
            expr = json!(["??", line, expr, right]);
        }

        Ok(expr)
    }

    fn parse_logical_or(&mut self) -> Result<Value, String> {
        let mut left = self.parse_logical_and()?;
        while self.match_token(TokenKind::Or) {
            let right = self.parse_logical_and()?;
            left = json!(["||", left, right]);
        }
        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Value, String> {
        let mut left = self.parse_equality()?;
        while self.match_token(TokenKind::And) {
            let right = self.parse_equality()?;
            left = json!(["&&", left, right]);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Value, String> {
        let mut left = self.parse_relational()?;
        while let TokenKind::EqEq | TokenKind::Neq = self.peek() {
            let op = match self.advance().kind {
                TokenKind::EqEq => "==",
                TokenKind::Neq => "!=",
                _ => unreachable!()
            };
            let right = self.parse_relational()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Value, String> {
        let mut left = self.parse_bitwise()?;
        while let TokenKind::Lt | TokenKind::Gt | TokenKind::LtEq | TokenKind::GtEq = self.peek() {
             let op = match self.advance().kind {
                TokenKind::Lt => "<",
                TokenKind::Gt => ">",
                TokenKind::LtEq => "<=",
                TokenKind::GtEq => ">=",
                _ => unreachable!(),
            };
            let right = self.parse_bitwise()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_bitwise(&mut self) -> Result<Value, String> {
        let mut left = self.parse_additive()?;
        while let TokenKind::BitAnd | TokenKind::BitOr | TokenKind::BitXor | TokenKind::ShiftLeft | TokenKind::ShiftRight = self.peek() {
            let op = match self.advance().kind {
                TokenKind::BitAnd => "&",
                TokenKind::BitOr => "|",
                TokenKind::BitXor => "^",
                TokenKind::ShiftLeft => "<<",
                TokenKind::ShiftRight => ">>",
                _ => unreachable!()
            };
            let right = self.parse_additive()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Value, String> {
        let mut left = self.parse_multiplicative()?;
        while let TokenKind::Plus | TokenKind::Minus = self.peek() {
            let op = match self.advance().kind {
                TokenKind::Plus => "+",
                TokenKind::Minus => "-",
                _ => unreachable!()
            };
            let right = self.parse_multiplicative()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Value, String> {
        let mut left = self.parse_unary()?;
        while let TokenKind::Star | TokenKind::Slash | TokenKind::Percent = self.peek() {
            let op = match self.advance().kind {
                TokenKind::Star => "*",
                TokenKind::Slash => "/",
                TokenKind::Percent => "%",
                _ => unreachable!()
            };
            let right = self.parse_unary()?;
            left = json!([op, left, right]);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Value, String> {
        if self.match_token(TokenKind::Bang) {
            let right = self.parse_unary()?;
            return Ok(json!(["!", right]));
        }
        if self.match_token(TokenKind::Minus) {
            let right = self.parse_unary()?;
            return Ok(json!(["-", json!(0), right]));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Value, String> {
        let mut expr = match self.peek() {
            TokenKind::Integer(n) => { let v = *n; self.advance(); json!(v) },
            TokenKind::Float(f) => { let v = *f; self.advance(); json!(v) },
            TokenKind::StringLiteral(s) => { 
                let raw = s.clone(); 
                self.advance(); 
                if raw.contains("${") { return self.parse_interpolated_string(&raw); }
                json!(raw) 
            },
            TokenKind::True => { self.advance(); json!(true) },
            TokenKind::False => { self.advance(); json!(false) },
            TokenKind::Null => { self.advance(); json!(null) },
            TokenKind::Identifier(name) => { let n = name.clone(); self.advance(); json!(["get", n]) },
            TokenKind::Func => {
                self.advance();
                self.consume(TokenKind::LParen, "(")?;
                let mut params = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        if let TokenKind::Identifier(p) = &self.advance().kind { params.push(p.clone()); }
                        if !self.match_token(TokenKind::Comma) { break; }
                    }
                }
                self.consume(TokenKind::RParen, ")")?;
                let body = self.parse_block()?;
                json!(["lambda", params, body])
            },
            TokenKind::LParen => {
                self.advance();
                let e = self.parse_expression()?;
                self.consume(TokenKind::RParen, ")")?;
                e
            },
            TokenKind::LBracket => {
                self.advance();
                let mut els = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop { els.push(self.parse_expression()?); if !self.match_token(TokenKind::Comma) { break; } }
                }
                self.consume(TokenKind::RBracket, "]")?;
                let mut ast = vec![json!("make_list")];
                ast.extend(els);
                json!(ast)
            },
            TokenKind::LBrace => {
                self.advance();
                let mut entries = Vec::new();
                if !self.check(&TokenKind::RBrace) {
                    loop {
                        let key = match &self.advance().kind {
                            TokenKind::StringLiteral(s) => s.clone(),
                            TokenKind::Identifier(s) => s.clone(),
                            _ => return Err("Dict Key".into())
                        };
                        self.consume(TokenKind::Colon, ":")?;
                        let val = self.parse_expression()?;
                        entries.push(json!([key, val]));
                        if !self.match_token(TokenKind::Comma) { break; }
                    }
                }
                self.consume(TokenKind::RBrace, "}")?;
                let mut ast = vec![json!("make_dict")];
                ast.extend(entries);
                json!(ast)
            },
            TokenKind::New => {
                self.advance();
                let mut expr = if let TokenKind::Identifier(n) = &self.advance().kind { json!(["get", n.clone()]) } else { return Err("Class".into()); };
                while self.match_token(TokenKind::Dot) {
                    if let TokenKind::Identifier(m) = &self.advance().kind { expr = json!(["get_attr", expr, m.clone()]); }
                }
                self.consume(TokenKind::LParen, "(")?;
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop { args.push(self.parse_expression()?); if !self.match_token(TokenKind::Comma) { break; } }
                }
                self.consume(TokenKind::RParen, ")")?;
                let mut new_cmd = vec![json!("new"), expr];
                new_cmd.extend(args);
                json!(new_cmd)
            },
            TokenKind::Super => {
                self.advance(); // Consomme 'super'
                self.consume(TokenKind::Dot, "Expect '.' after super")?;
                
                let method_name = if let TokenKind::Identifier(n) = &self.advance().kind {
                    n.clone()
                } else {
                    return Err("Expect superclass method name".into());
                };

                self.consume(TokenKind::LParen, "Expect '(' after method name")?;
                
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop {
                        args.push(self.parse_expression()?);
                        if !self.match_token(TokenKind::Comma) { break; }
                    }
                }
                self.consume(TokenKind::RParen, "Expect ')' after arguments")?;

                // On génère le format JSON attendu par le Loader
                json!(["super_call", method_name, args])
            },
            _ => return Err(format!("Unexpected token: {:?}", self.peek()))
        };

        loop {
            if self.match_token(TokenKind::LParen) {
                let mut args = Vec::new();
                if !self.check(&TokenKind::RParen) {
                    loop { args.push(self.parse_expression()?); if !self.match_token(TokenKind::Comma) { break; } }
                }
                self.consume(TokenKind::RParen, ")")?;
                expr = json!(["call", expr, args]);
            } else if self.match_token(TokenKind::Dot) {
                let member = if let TokenKind::Identifier(n) = &self.advance().kind { n.clone() } else { return Err("Member".into()); };
                if self.match_token(TokenKind::LParen) {
                    let mut args = Vec::new();
                    if !self.check(&TokenKind::RParen) {
                        loop { args.push(self.parse_expression()?); if !self.match_token(TokenKind::Comma) { break; } }
                    }
                    self.consume(TokenKind::RParen, ")")?;
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

                        // Si on trouve ':' et qu'on est au niveau 1
                        if code_char == ':' && brace_count == 1 && !found_colon {
                            found_colon = true;
                            continue;
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
}
