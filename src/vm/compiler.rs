use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

use crate::ast::value::{ClassData, FunctionData};
use crate::ast::{Instruction, Expression, Value};
use crate::chunk::Chunk;
use crate::opcode::OpCode;

#[derive(Debug)]
pub enum LoopState {
    While { start_ip: usize },
    For { continue_patches: Vec<usize> }, // Liste des jumps à corriger
}

#[derive(Debug, Clone, Copy)]
pub struct LocalInfo {
    index: u8,
    is_const: bool
}

pub struct Compiler {
    pub chunk: Chunk,
    pub globals: Rc<RefCell<HashMap<String, u8>>>, 
    pub locals: HashMap<String, LocalInfo>,
    pub global_constants: Vec<String>,
    pub scope_depth: usize,
    pub current_return_type: Option<String>,
    pub current_line: usize,
    pub loop_stack: Vec<LoopState>,
    pub context_parent_name: Option<String>,
}

impl Compiler {
    pub fn new() -> Self {
        let globals = Rc::new(RefCell::new(HashMap::new()));
        let natives = crate::native::get_all_names();
        
        {
            let mut g = globals.borrow_mut();
            for (i, name) in natives.into_iter().enumerate() {
                // On assigne les ID 0, 1, 2... dans l'ordre alphabétique
                g.insert(name, i as u8);
            }
        }

        Self {
            chunk: Chunk::new(),
            globals,
            locals: HashMap::new(),
            global_constants: Vec::new(),
            scope_depth: 0,
            current_return_type: None,
            current_line: 1,
            loop_stack: Vec::new(),
            context_parent_name: None
        }
    }

    pub fn new_with_globals(globals: Rc<RefCell<HashMap<String, u8>>>) -> Self {
         Self {
            chunk: Chunk::new(),
            globals, 
            locals: HashMap::new(),
            global_constants: Vec::new(),
            scope_depth: 0,
            current_return_type: None,
            current_line: 1,
            loop_stack: Vec::new(),
            context_parent_name: None
        }
    }

    pub fn compile(mut self, statements: Vec<crate::ast::Statement>) -> (Chunk, Rc<RefCell<HashMap<String, u8>>>) {
        for stmt in statements {
            self.current_line = stmt.line;
            self.compile_instruction(stmt.kind);
        }
        (self.chunk, self.globals)
    } 

    fn emit_byte(&mut self, byte: u8) {
        self.chunk.write(byte, self.current_line);
    }
    
    fn emit_op(&mut self, op: OpCode) {
        self.emit_byte(op as u8);
    }

    fn emit_constant(&mut self, val: Value) {
        let idx = self.chunk.add_constant(val);
        self.emit_op(OpCode::LoadConst);
        self.emit_byte(idx);
    }

    fn resolve_global(&mut self, name: &str) -> u8 {
        let mut globals = self.globals.borrow_mut();
        if let Some(&id) = globals.get(name) {
            return id;
        }
        let id = globals.len() as u8;
        globals.insert(name.to_string(), id);
        id
    }

    fn compile_expression(&mut self, expr: Expression) {
        if let Some(val) = self.evaluate_constant(&expr) {
            self.emit_constant(val);
            return;
        }

        match expr {
            Expression::Literal(val) => self.emit_constant(val),
            Expression::Add(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Add);
            },
            Expression::Sub(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Sub);
            },
            Expression::Mul(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Mul);
            },
            Expression::Div(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Div);
            },
            Expression::Variable(name) => {
                // 1. On cherche d'abord dans les locales (si on est dans une fonction)
                if let Some(info) = self.locals.get(&name) {
                    let idx = info.index;
                    self.emit_op(OpCode::GetLocal);
                    self.emit_byte(idx);
                } else {
                    if self.scope_depth > 0 {
                        let name_idx = self.chunk.add_constant(Value::String(name.clone()));
                        self.emit_op(OpCode::GetFreeVar);
                        self.emit_byte(name_idx);
                    } else {
                        let id = self.resolve_global(&name);
                        self.emit_op(OpCode::GetGlobal);
                        self.emit_byte(id);
                    }
                }
            },
            Expression::LessThan(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Less);
            },
            Expression::GreaterThan(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Greater);
            },
            Expression::Equal(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Equal);
            },
            Expression::Call(target, args) => {
                // 1. On sauvegarde la taille (nécessaire pour le borrow checker)
                let arg_count = args.len(); 
    
                // A. D'abord on compile la fonction (pour qu'elle soit au fond de la pile)
                self.compile_expression(*target);

                // B. Ensuite on compile les arguments (qui s'empilent par-dessus)
                for arg in args {
                    self.compile_expression(arg);
                }
    
                // ----------------------------------
    
                // 4. Émettre CALL
                self.emit_op(OpCode::Call);
                self.emit_byte(arg_count as u8);
            }

            Expression::Modulo(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::Modulo);
            },
            Expression::NotEqual(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::NotEqual);
            },
            Expression::LessEqual(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::LessEqual);
            },
            Expression::GreaterEqual(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::GreaterEqual);
            },
            // Bitwise
            Expression::BitAnd(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::BitAnd);
            },
             Expression::BitOr(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::BitOr);
            },
            Expression::BitXor(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::BitXor);
            },
            Expression::ShiftLeft(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::ShiftLeft);
            },
            Expression::ShiftRight(left, right) => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                self.emit_op(OpCode::ShiftRight);
            },
            Expression::Not(expr) => {
                self.compile_expression(*expr);
                self.emit_op(OpCode::Not);
            },

            Expression::And(left, right) => {
                self.compile_expression(*left);
                // Si gauche est Faux, on saute tout de suite à la fin (résultat = Faux)
                let end_jump = self.emit_jump(OpCode::JumpIfFalse);
                self.emit_op(OpCode::Pop); // On pop le résultat de gauche
                self.compile_expression(*right);
                self.patch_jump(end_jump);
            },
            Expression::Or(left, right) => {
                self.compile_expression(*left);
                // Si gauche est Faux, on saute au "else" (qui évalue droite)
                let else_jump = self.emit_jump(OpCode::JumpIfFalse);
                let end_jump = self.emit_jump(OpCode::Jump); // Si Vrai, on saute à la fin
                
                self.patch_jump(else_jump);
                self.emit_op(OpCode::Pop); // Pop le faux
                self.compile_expression(*right);
                self.patch_jump(end_jump);
            },

            Expression::Ternary(cond, then_expr, else_expr) => {
                // 1. Condition
                self.compile_expression(*cond);
                
                // 2. Saut vers le Else si Faux
                let else_jump = self.emit_jump(OpCode::JumpIfFalse);
                
                // 3. Si Vrai : On pop la condition (true) et on évalue le Then
                self.emit_op(OpCode::Pop);
                self.compile_expression(*then_expr);
                
                // 4. Saut vers la fin (pour ne pas faire le Else)
                let end_jump = self.emit_jump(OpCode::Jump);
                
                // 5. Label Else
                self.patch_jump(else_jump);
                self.emit_op(OpCode::Pop); // On pop la condition (false)
                
                // 6. Si Faux : On évalue le Else
                self.compile_expression(*else_expr);
                
                // 7. Label Fin
                self.patch_jump(end_jump);
            },

            Expression::NullCoalescing(left, right) => {
                // 1. Evaluer Gauche
                self.compile_expression(*left); // Pile: [val]
                
                // 2. Dupliquer pour le test
                self.emit_op(OpCode::Dup);      // Pile: [val, val]
                
                // 3. Charger Null et Comparer
                let null_idx = self.chunk.add_constant(Value::Null);
                self.emit_op(OpCode::LoadConst);
                self.emit_byte(null_idx);       // Pile: [val, val, null]
                self.emit_op(OpCode::Equal);    // Pile: [val, is_null]
                
                // 4. Si c'est FAUX (donc pas null), on saute le bloc "Remplacement"
                let jump_over = self.emit_jump(OpCode::JumpIfFalse);
                
                // --- CHEMIN : C'EST NULL (is_null était Vrai) ---
                // Le JumpIfFalse n'a pas sauté. 
                // IMPORTANT : Dans ta VM, JumpIfFalse fait un PEEK sur la condition.
                // La pile est donc : [val (null), is_null (true)]
                
                self.emit_op(OpCode::Pop); // On retire le booléen 'true'
                self.emit_op(OpCode::Pop); // On retire la valeur 'null'
                
                // On évalue la partie droite
                self.compile_expression(*right); // Pile: [res_droite]
                
                // On doit sauter par-dessus le code de nettoyage de l'autre branche
                let jump_end = self.emit_jump(OpCode::Jump);
                
                // --- CHEMIN : CE N'EST PAS NULL (is_null était Faux) ---
                self.patch_jump(jump_over); // On atterrit ici si le jump a été pris
                
                // Pile : [val, is_null (false)]
                self.emit_op(OpCode::Pop); // On retire le booléen 'false'
                // Pile : [val] -> C'est ce qu'on veut !
                
                // --- FIN ---
                self.patch_jump(jump_end);
            },

            Expression::List(exprs) => {
                for expr in exprs.iter() {
                    self.compile_expression(expr.clone());
                }
                self.emit_op(OpCode::MakeList);
                self.emit_byte(exprs.len() as u8);
            },
            Expression::Dict(items) => {
                let count = items.len(); // Sauvegarde avant consommation

                for (key, val) in items {
                    let key_idx = self.chunk.add_constant(Value::String(key.clone()));
                    self.emit_op(OpCode::LoadConst);
                    self.emit_byte(key_idx);
                    self.compile_expression(val.clone());
                }
                self.emit_op(OpCode::MakeDict);
                self.emit_byte((count * 2) as u8); // Utilisation de la variable sauvegardée
            },

            Expression::GetAttr(obj, name) => {
                self.compile_expression(*obj);
                let name_idx = self.chunk.add_constant(Value::String(name));
                self.emit_op(OpCode::GetAttr);
                self.emit_byte(name_idx);
            },
            Expression::CallMethod(obj, name, args) => {
                let arg_count = args.len(); // Sauvegarde

                // 1. Compiler l'objet
                self.compile_expression(*obj);
                
                // 2. Compiler les arguments
                for arg in args {
                    self.compile_expression(arg.clone());
                }
                
                // 3. Émettre l'instruction
                let name_idx = self.chunk.add_constant(Value::String(name));
                self.emit_op(OpCode::Method);
                self.emit_byte(name_idx);
                self.emit_byte(arg_count as u8); // Utilisation
            },
            Expression::New(class_expr, args) => {
                let arg_count = args.len(); // Sauvegarde

                self.compile_expression(*class_expr);
                
                for arg in args {
                    self.compile_expression(arg.clone());
                }
                
                self.emit_op(OpCode::Call); // Ou OpCode::New si tu en as créé un
                self.emit_byte(arg_count as u8); // Utilisation
            },

            Expression::SuperCall(method, args) => {
                // 1. Vérification : Est-on dans une classe enfant ?
                let parent_name = if let Some(p) = &self.context_parent_name {
                    p.clone()
                } else {
                    panic!("'super' utilisé hors d'une classe avec héritage.");
                };

                // 2. On empile 'this' (toujours l'argument 0 d'une méthode)
                self.emit_op(OpCode::GetLocal);
                self.emit_byte(0);

                // 3. On empile les arguments
                let arg_count = args.len();
                for arg in args {
                    self.compile_expression(arg);
                }

                // 4. On émet l'instruction SUPER
                let name_idx = self.chunk.add_constant(Value::String(method));
                let parent_idx = self.chunk.add_constant(Value::String(parent_name));

                self.emit_op(OpCode::Super);
                self.emit_byte(name_idx);
                self.emit_byte(arg_count as u8);
                self.emit_byte(parent_idx);
            },

            Expression::Function { params, ret_type, body } => {
                let mut func_compiler = Compiler::new_with_globals(self.globals.clone());
                func_compiler.scope_depth = 1;

                for (i, (param_name, _)) in params.iter().enumerate() {
                    func_compiler.locals.insert(param_name.clone(), LocalInfo {
                        index: i as u8,
                        is_const: false
                    });
                }
                for stmt in body {
                    func_compiler.compile_instruction(stmt.kind);
                }
                func_compiler.emit_op(OpCode::LoadConst);
                let null_idx = func_compiler.chunk.add_constant(Value::Null);
                func_compiler.emit_byte(null_idx);
                func_compiler.emit_op(OpCode::Return);

                for (name, info) in &func_compiler.locals {
                    func_compiler.chunk.locals_map.insert(info.index, name.clone());
                }

                let func_chunk = func_compiler.chunk;
                let compiled_val = Value::Function(Rc::new(FunctionData {
                    params: params.clone(),
                    ret_type: ret_type.clone(),
                    chunk: func_chunk,
                    env: None
                }));
                let const_idx = self.chunk.add_constant(compiled_val);

                self.emit_op(OpCode::LoadConst);
                self.emit_byte(const_idx);

                self.emit_op(OpCode::MakeClosure);
            },
        }
    }

    pub fn compile_instruction(&mut self, instr: Instruction) {
        match instr {
            Instruction::Print(expr) => {
                self.compile_expression(expr);
                self.emit_op(OpCode::Print);
            },
            Instruction::Return(expr) => {
                self.compile_expression(expr); // 1. Calcule la valeur de retour

                if let Some(ret_type) = &self.current_return_type {
                    let type_idx = self.chunk.add_constant(Value::String(ret_type.clone()));
                    self.emit_op(OpCode::CheckType);
                    self.emit_byte(type_idx);
                }

                self.emit_op(OpCode::Return);  // 2. Quitte la fonction
            },
            Instruction::Set(var_name, type_annot, expr) => {
                // A. Check Locals
                if let Some(info) = self.locals.get(&var_name) {
                    if info.is_const {
                        panic!("Erreur: Impossible de modifier la constante locale '{}'", var_name);
                    }
                }
                
                // B. Check Globals (Scope courant)
                if self.global_constants.contains(&var_name) {
                    panic!("Erreur: Impossible de modifier la constante globale '{}'", var_name);
                }

                self.compile_expression(expr); // La valeur calculée est maintenant sur la pile [val]

                if let Some(type_name) = type_annot {
                    let type_idx = self.chunk.add_constant(Value::String(type_name));
                    self.emit_op(OpCode::CheckType);
                    self.emit_byte(type_idx);
                }

                // CAS 1 : C'est une variable locale DÉJÀ connue (Assignation : x = 5)
                if let Some(info) = self.locals.get(&var_name) {
                    let idx = info.index;
                    self.emit_op(OpCode::SetLocal);
                    self.emit_byte(idx);
                    self.emit_op(OpCode::Pop); // Nettoyage : On retire la valeur car c'est une instruction (statement)
                } 
                // CAS 2 : On est dans une fonction, c'est une NOUVELLE variable (Déclaration : var res = ...)
                else if self.scope_depth > 0 {
                    let idx = self.locals.len() as u8; // Le prochain slot libre sur la pile
                    self.locals.insert(var_name.clone(), LocalInfo {
                        index: idx,
                        is_const: false
                    });
                    
                    // ASTUCE MAGIQUE DE LA PILE :
                    // On ne fait RIEN d'autre. La valeur [val] est déjà au sommet de la pile.
                    // En l'enregistrant dans 'self.locals' à l'index 'idx', on dit au compilateur :
                    // "La valeur qui est actuellement sur la pile est maintenant la variable 'res'".
                    // Elle y restera jusqu'à la fin de la fonction.
                } 
                // CAS 3 : C'est une Globale (Assignation ou Déclaration globale)
                else {
                    let id = self.resolve_global(&var_name);
                    self.emit_op(OpCode::SetGlobal); // SetGlobal fait déjà un Pop dans la VM
                    self.emit_byte(id);
                }
            },

            Instruction::If { condition, body, else_body } => {
                self.compile_if(condition, body, else_body);
            },

            Instruction::While { condition, body } => {
                self.compile_while(condition, body);
            },
            
            Instruction::Function { name, params, ret_type, body } => {
                // 1. Compilation du corps de la fonction (Inchangé)
                let mut func_compiler = Compiler::new_with_globals(self.globals.clone());
                func_compiler.scope_depth = 1;

                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    func_compiler.locals.insert(param_name.clone(), LocalInfo {
                        index: i as u8,
                        is_const: false
                    });

                    if let Some(t) = param_type {
                        // Au début de la fonction, les arguments sont déjà sur la pile (locales).
                        // On doit les charger, les checker, et les poper (juste pour le check).
                        
                        // 1. Lire la variable locale
                        func_compiler.emit_op(OpCode::GetLocal);
                        func_compiler.emit_byte(i as u8);
                        
                        // 2. Checker
                        let type_idx = func_compiler.chunk.add_constant(Value::String(t.clone()));
                        func_compiler.emit_op(OpCode::CheckType);
                        func_compiler.emit_byte(type_idx);
                        
                        // 3. Nettoyer la pile (on a dupliqué via GetLocal)
                        func_compiler.emit_op(OpCode::Pop);
                    }
                }

                for stmt in body {
                    func_compiler.compile_instruction(stmt.kind);
                }

                func_compiler.emit_op(OpCode::LoadConst);
                let null_idx = func_compiler.chunk.add_constant(Value::Null);
                func_compiler.emit_byte(null_idx);
                func_compiler.emit_op(OpCode::Return);

                for (name, info) in &func_compiler.locals {
                    func_compiler.chunk.locals_map.insert(info.index, name.clone());
                }

                let func_chunk = func_compiler.chunk;
                let compiled_val = Value::Function(Rc::new(FunctionData {
                    params: params.clone(),
                    ret_type: ret_type.clone(),
                    chunk: func_chunk,
                    env: None
                }));

                // 2. Chargement de la fonction sur la pile (Inchangé)
                let const_idx = self.chunk.add_constant(compiled_val);
                self.emit_op(OpCode::LoadConst);
                self.emit_byte(const_idx);
                
                // On la transforme en closure (pour capturer l'env si besoin)
                self.emit_op(OpCode::MakeClosure);

                // 3. --- MODIFICATION : Stockage (Global ou Local) ---
                if self.scope_depth > 0 {
                    // Cas Namespace ou Fonction imbriquée : C'est une locale
                    let idx = self.locals.len() as u8;
                    self.locals.insert(name.clone(), LocalInfo {
                        index: idx,
                        is_const: false
                    });
                    // La fonction est déjà sur la pile, elle devient la variable locale 'name'.
                    // On ne fait rien d'autre (comme pour SetLocal implicite).
                } else {
                    // Cas Script Principal : C'est une globale
                    let global_id = self.resolve_global(&name);
                    self.emit_op(OpCode::SetGlobal);
                    self.emit_byte(global_id);
                }
            },

            Instruction::ForRange { var_name, start, end, step, body } => {
                // 1. Initialisation : On calcule la valeur de départ
                self.compile_expression(start); // Pile : [..., start_val]

                // 2. Déclaration de la variable de boucle
                let loop_var_idx = if self.scope_depth > 0 {
                    // C'est une variable LOCALE
                    // L'index est le sommet actuel de la pile (là où est start_val)
                    // ATTENTION : On utilise locals.len() AVANT d'insérer, ce qui correspond
                    // à l'index de la valeur qu'on vient de pousser (car len a augmenté implicitement via la stack).
                    let idx = self.locals.len() as u8;
                    self.locals.insert(var_name.clone(), LocalInfo {
                        index: idx,
                        is_const: false
                    });
                    idx
                } else {
                    let idx = self.resolve_global(&var_name);
                    self.emit_op(OpCode::SetGlobal);
                    self.emit_byte(idx);
                    idx
                };
                let is_local = self.scope_depth > 0;

                let loop_start = self.chunk.code.len();

                // 3. Condition : i < end
                if is_local {
                    self.emit_op(OpCode::GetLocal); self.emit_byte(loop_var_idx);
                } else {
                    self.emit_op(OpCode::GetGlobal); self.emit_byte(loop_var_idx);
                }
                
                self.compile_expression(end);
                self.emit_op(OpCode::Less); 

                let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
                self.emit_op(OpCode::Pop);

                self.loop_stack.push(LoopState::For { continue_patches: Vec::new() });

                // 4. Corps
                self.compile_scope(body);

                if let Some(LoopState::For { continue_patches }) = self.loop_stack.pop() {
                    for patch_offset in continue_patches {
                        self.patch_jump(patch_offset); // On redirige les sauts ici (début incrément)
                    }
                }

                // 5. Incrément : i = i + step
                if is_local {
                    self.emit_op(OpCode::GetLocal); self.emit_byte(loop_var_idx);
                } else {
                    self.emit_op(OpCode::GetGlobal); self.emit_byte(loop_var_idx);
                }
                
                self.compile_expression(step);
                self.emit_op(OpCode::Add);
                
                if is_local {
                    self.emit_op(OpCode::SetLocal); // Ici c'est OK car la variable existe déjà
                    self.emit_byte(loop_var_idx);
                    self.emit_op(OpCode::Pop);
                } else {
                    self.emit_op(OpCode::SetGlobal); 
                    self.emit_byte(loop_var_idx);
                }

                // 6. Loop
                self.emit_loop(loop_start);
                self.patch_jump(exit_jump);
                self.emit_op(OpCode::Pop);
                
                // 7. Nettoyage du scope local (Important !)
                // Si c'était une locale, à la fin du for, la variable 'j' doit être retirée de la pile
                if is_local {
                    // On enlève la variable de la pile d'exécution
                    self.emit_op(OpCode::Pop);
                    // On l'enlève de la table des symboles du compilateur pour les instructions suivantes
                    self.locals.remove(&var_name);
                }
            },

            Instruction::Switch { value, cases, default } => {
                self.compile_expression(value); // La valeur à tester est sur la pile

                let mut end_jumps = Vec::new();

                for (case_val, case_body) in cases {
                    self.emit_op(OpCode::Dup);
                    
                    self.compile_expression(case_val);
                    self.emit_op(OpCode::Equal);
                    
                    let next_case_jump = self.emit_jump(OpCode::JumpIfFalse);
                    self.emit_op(OpCode::Pop); // Pop le booléen true
                    
                    // Body
                    self.compile_scope(case_body);
                    
                    // Si on a exécuté un cas, on saute à la fin (break implicite)
                    end_jumps.push(self.emit_jump(OpCode::Jump));
                    
                    self.patch_jump(next_case_jump);
                    self.emit_op(OpCode::Pop); // Pop le booléen false
                }

                // Default
                self.compile_scope(default);

                // Patch de toutes les sorties
                for jump in end_jumps { self.patch_jump(jump); }
                
                self.emit_op(OpCode::Pop); // On nettoie la valeur testée originale
            },

            Instruction::ExpressionStatement(expr) => {
                self.compile_expression(expr);
                self.emit_op(OpCode::Pop); // On jette le résultat
            },
            
            Instruction::Input(var_name, prompt) => {
                self.compile_expression(prompt);
                self.emit_op(OpCode::Input); // VM devra gérer l'affichage + lecture
                // Le résultat de Input est sur la pile, on le stocke
                let id = self.resolve_global(&var_name); // Ou local
                self.emit_op(OpCode::SetGlobal);
                self.emit_byte(id);
            },

            Instruction::Class(def) => {
                let mut compiled_methods = HashMap::new();

                for (m_name, (m_params, m_body)) in def.methods {
                    let mut method_compiler = Compiler::new_with_globals(self.globals.clone());
                    method_compiler.scope_depth = 1;
                    method_compiler.context_parent_name = def.parent.clone();
                    
                    let mut actual_params = vec![("this".to_string(), None)];
                    actual_params.extend(m_params.clone());

                    for (i, (param_name, _)) in actual_params.iter().enumerate() {
                        method_compiler.locals.insert(param_name.clone(), LocalInfo {
                            index: i as u8,
                            is_const: false
                        });
                    }
                    for stmt in m_body {
                        method_compiler.compile_instruction(stmt.kind);
                    }
                    method_compiler.emit_op(OpCode::LoadConst);
                    let null_idx = method_compiler.chunk.add_constant(Value::Null);
                    method_compiler.emit_byte(null_idx);
                    method_compiler.emit_op(OpCode::Return);

                    let method_val = Value::Function(Rc::new(FunctionData {
                        params: actual_params,
                        ret_type: None,
                        chunk: method_compiler.chunk,
                        env: None
                    }));

                    compiled_methods.insert(m_name, method_val);
                }

                let class_val = Value::Class(Rc::new(ClassData {
                    name: def.name.clone(),
                    parent: def.parent.clone(),
                    methods: compiled_methods,
                }));

                let const_idx = self.chunk.add_constant(class_val);
                self.emit_op(OpCode::LoadConst);
                self.emit_byte(const_idx);
                
                let global_id = self.resolve_global(&def.name);
                self.emit_op(OpCode::SetGlobal);
                self.emit_byte(global_id);
            },

            Instruction::SetAttr(obj, attr, val) => {
                self.compile_expression(*obj); // 1. L'objet
                self.compile_expression(val);  // 2. La valeur
                
                let name_idx = self.chunk.add_constant(Value::String(attr));
                self.emit_op(OpCode::SetAttr);
                self.emit_byte(name_idx);
                // SetAttr laisse généralement la valeur sur la pile (comme une assignation),
                // mais comme c'est une instruction ici, on POP pour nettoyer.
                self.emit_op(OpCode::Pop); 
            },

            Instruction::TryCatch { try_body, error_var, catch_body } => {
                // 1. Setup Exception Handler
                let catch_jump = self.emit_jump(OpCode::SetupExcept);

                // 2. Compile Try Block
                self.compile_scope(try_body);

                // 3. Pop Exception (Success Path)
                self.emit_op(OpCode::PopExcept);
                let end_jump = self.emit_jump(OpCode::Jump);

                // 4. Start of Catch
                self.patch_jump(catch_jump);

                // 5. Variable Binding (CORRIGÉ)
                self.scope_depth += 1;
                
                // On déclare que la variable 'e' existe et qu'elle est située au sommet actuel de la pile.
                let catch_var_idx = self.locals.len() as u8;
                self.locals.insert(error_var.clone(), LocalInfo {
                    index: catch_var_idx,
                    is_const: true
                });
                
                // --- MODIFICATION ICI ---
                // On ne fait NI SetLocal, NI Pop. 
                // La valeur est déjà sur la pile, c'est notre variable locale.
                // ------------------------

                self.compile_scope(catch_body);
                
                // 6. Cleanup (OPTIONNEL MAIS RECOMMANDÉ)
                // À la fin du catch, on retire la variable 'e' de la pile pour revenir à l'état propre
                self.emit_op(OpCode::Pop); 
                
                self.locals.remove(&error_var);
                self.scope_depth -= 1;

                // 7. End
                self.patch_jump(end_jump);
            },
            Instruction::Throw(expr) => {
                // 1. On compile l'expression (l'erreur) pour la mettre sur la pile
                self.compile_expression(expr);
                
                // 2. On émet l'OpCode qui va déclencher la panique contrôlée dans la VM
                self.emit_op(OpCode::Throw);
            },

            Instruction::Namespace { name, body } => {
                // 1. RÉSERVATION DU NOM (Crucial pour l'auto-référence "Maths.square")
                // On définit où sera stocké le namespace final AVANT de compiler son contenu.
                let global_id = if self.scope_depth == 0 {
                    Some(self.resolve_global(&name))
                } else {
                    None
                };

                let local_idx = if self.scope_depth > 0 {
                    let idx = self.locals.len() as u8;
                    // On "réserve" le slot local. Attention: la valeur n'y est pas encore !
                    // Mais cela permet à 'resolve_local' de savoir que la variable existe.
                    self.locals.insert(name.clone(), LocalInfo {
                        index: idx,
                        is_const: false
                    });
                    Some(idx)
                } else {
                    None
                };

                // 2. COMPILATION DU CORPS (IIFE Pattern)
                let mut ns_compiler = Compiler::new_with_globals(self.globals.clone());
                ns_compiler.scope_depth = 1; 

                for stmt in body {
                    ns_compiler.compile_instruction(stmt.kind);
                }

                // 3. CONSTRUCTION DU DICTIONNAIRE (Exports)
                let exports: Vec<(String, u8)> = ns_compiler.locals.iter()
                    .map(|(k, info)| (k.clone(), info.index))
                    .collect();
                
                let count = exports.len();

                for (var_name, slot_idx) in exports {
                    let key_idx = ns_compiler.chunk.add_constant(Value::String(var_name));
                    ns_compiler.emit_op(OpCode::LoadConst);
                    ns_compiler.emit_byte(key_idx);
                    ns_compiler.emit_op(OpCode::GetLocal);
                    ns_compiler.emit_byte(slot_idx);
                }

                ns_compiler.emit_op(OpCode::MakeDict);
                ns_compiler.emit_byte((count * 2) as u8);
                ns_compiler.emit_op(OpCode::Return);

                for (name, info) in &ns_compiler.locals {
                    ns_compiler.chunk.locals_map.insert(info.index, name.clone());
                }

                // 4. EMBALLAGE (Closure)
                let ns_chunk = ns_compiler.chunk;
                let ns_func = Value::Function(Rc::new(FunctionData {
                    params: vec![],
                    ret_type: None,
                    chunk: ns_chunk,
                    env: None
                }));
                
                let const_idx = self.chunk.add_constant(ns_func);
                self.emit_op(OpCode::LoadConst);
                self.emit_byte(const_idx);
                self.emit_op(OpCode::MakeClosure);

                self.emit_op(OpCode::Call);
                self.emit_byte(0);

                // 5. STOCKAGE FINAL
                // On utilise les ID calculés à l'étape 1
                if let Some(id) = global_id {
                    self.emit_op(OpCode::SetGlobal);
                    self.emit_byte(id);
                } else if let Some(idx) = local_idx {
                    // Pour une locale, la valeur est maintenant sur le sommet de la pile.
                    // SetLocal la copie dans le slot réservé.
                    self.emit_op(OpCode::SetLocal);
                    self.emit_byte(idx);
                    // Namespace est une instruction, pas une expression, donc on pop le résultat de la pile
                    // (La valeur est maintenant en sécurité dans la variable locale)
                    self.emit_op(OpCode::Pop); 
                }
            },

            Instruction::Import(path) => {
                // Store the path as a constant string
                let path_idx = self.chunk.add_constant(Value::String(path));
                
                // Emit the IMPORT opcode
                self.emit_op(OpCode::Import);
                self.emit_byte(path_idx);
            },
            Instruction::Continue => {
                // Étape 1 : On détermine l'action à faire (Lecture seule ou copie simple)
                // On utilise un enum temporaire ou juste des variables pour sortir l'info du scope
                enum LoopAction {
                    JumpToStart(usize),
                    RecordPatch,
                    Error
                }

                let action = match self.loop_stack.last() { // .last() suffit (lecture seule)
                    Some(LoopState::While { start_ip }) => LoopAction::JumpToStart(*start_ip),
                    Some(LoopState::For { .. }) => LoopAction::RecordPatch,
                    None => LoopAction::Error,
                }; // Ici, l'emprunt sur self.loop_stack est terminé !

                // Étape 2 : On agit (self est libre)
                match action {
                    LoopAction::JumpToStart(ip) => {
                        self.emit_loop(ip);
                    },
                    LoopAction::RecordPatch => {
                        // 1. On émet le saut (besoin de self)
                        let offset = self.emit_jump(OpCode::Jump);
                        
                        // 2. On ré-emprunte juste ce qu'il faut pour stocker l'offset
                        if let Some(LoopState::For { continue_patches }) = self.loop_stack.last_mut() {
                            continue_patches.push(offset);
                        }
                    },
                    LoopAction::Error => panic!("'continue' utilisé hors d'une boucle."),
                }
            },
            Instruction::Enum(name, variants) => {
                for (i, variant_name) in variants.iter().enumerate() {
                    // Clé
                    let key_idx = self.chunk.add_constant(Value::String(variant_name.clone()));
                    self.emit_op(OpCode::LoadConst);
                    self.emit_byte(key_idx);
                    
                    // Valeur (i)
                    let val_idx = self.chunk.add_constant(Value::Integer(i as i64));
                    self.emit_op(OpCode::LoadConst);
                    self.emit_byte(val_idx);
                }
                
                // On crée l'enum
                self.emit_op(OpCode::MakeEnum);
                self.emit_byte((variants.len() * 2) as u8);
                
                // On le stocke dans la variable (Globale ou Locale selon le scope)
                if self.scope_depth > 0 {
                    let idx = self.locals.len() as u8;
                    self.locals.insert(name.clone(), LocalInfo {
                        index: idx,
                        is_const: false
                    });
                    self.emit_op(OpCode::SetLocal);
                    self.emit_byte(idx);
                } else {
                    let id = self.resolve_global(&name);
                    self.emit_op(OpCode::SetGlobal);
                    self.emit_byte(id);
                }
                // SetGlobal/SetLocal ne popent pas toujours selon ton implémentation.
                // Si SetGlobal consomme la valeur (ce qui est le cas dans ta VM v2), c'est bon.
                // Sinon, ajoute un Pop. (Dans ta v2, SetGlobal fait un pop implicite via l'assignation du tableau, non ?)
                // Vérification VM v2 : OpCode::SetGlobal => let val = self.pop(); ...
                // C'est bon, la pile est propre.
            },
            Instruction::Const(name, expr) => {
                self.compile_expression(expr); // Valeur sur la pile
                
                if self.scope_depth > 0 {
                    // --- LOCALE ---
                    let idx = self.locals.len() as u8;
                    self.locals.insert(name.clone(), LocalInfo { 
                        index: idx, 
                        is_const: true 
                    });
                    // La valeur est sur la pile, elle devient la variable.
                } else {
                    // --- GLOBALE ---
                    let id = self.resolve_global(&name);
                    self.emit_op(OpCode::SetGlobal);
                    self.emit_byte(id);
                    
                    // On la marque comme constante pour empêcher la modif dans ce fichier
                    self.global_constants.push(name);
                }
            },
        }
    }

    // Emits a jump instruction with a placeholder operand.
    // Returns the offset of the placeholder so we can patch it later.
    fn emit_jump(&mut self, instruction: OpCode) -> usize {
        self.emit_op(instruction);
        self.emit_byte(0xff); // Placeholder high
        self.emit_byte(0xff); // Placeholder low
        self.chunk.code.len() - 2
    }

    // Goes back to 'offset' and writes the current distance
    fn patch_jump(&mut self, offset: usize) {
        // -2 to adjust for the jump offset itself
        let jump = self.chunk.code.len() - offset - 2;

        if jump > u16::MAX as usize {
            panic!("Too much code to jump over!");
        }

        self.chunk.code[offset] = ((jump >> 8) & 0xff) as u8;
        self.chunk.code[offset + 1] = (jump & 0xff) as u8;
    }

    // Compile an IF statement
    // if (cond) { then } else { else }
    fn compile_if(&mut self, condition: Expression, then_body: Vec<crate::ast::Statement>, else_body: Vec<crate::ast::Statement>) {
        // 1. Compile condition
        self.compile_expression(condition);

        // 2. Jump over 'then' if false
        let then_jump = self.emit_jump(OpCode::JumpIfFalse);

        // 3. Compile 'then' block
        self.emit_op(OpCode::Pop); // Clean up condition result (optional optimization)

        self.compile_scope(then_body);

        // 4. Jump over 'else'
        let else_jump = self.emit_jump(OpCode::Jump);

        // 5. Patch the first jump (target is here, start of else)
        self.patch_jump(then_jump);
        
        self.emit_op(OpCode::Pop); // Clean up condition for the else path

        // 6. Compile 'else' block
        self.compile_scope(else_body);

        // 7. Patch the second jump (target is end)
        self.patch_jump(else_jump);
    }

    // Émet une instruction de saut en arrière
    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_op(OpCode::Loop);

        // Calcul du saut : position actuelle - début de la boucle + 2 (taille des opérandes)
        let offset = self.chunk.code.len() - loop_start + 2;
        
        if offset > u16::MAX as usize {
            panic!("Loop body too large!");
        }

        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn compile_while(&mut self, condition: Expression, body: Vec<crate::ast::Statement>) {
        // 1. Marquer le début de la boucle (pour y revenir après)
        let loop_start = self.chunk.code.len();

        self.loop_stack.push(LoopState::While { start_ip: loop_start });

        // 2. Compiler la condition
        self.compile_expression(condition);

        // 3. Sauter à la fin si la condition est fausse
        let exit_jump = self.emit_jump(OpCode::JumpIfFalse);
        self.emit_op(OpCode::Pop); // Nettoyer la condition de la pile

        // 4. Compiler le corps
        self.compile_scope(body);

        // 5. Remonter au début !
        self.emit_loop(loop_start);

        // 6. Patcher le saut de sortie
        self.patch_jump(exit_jump);
        self.emit_op(OpCode::Pop); // Nettoyer la condition finale

        self.loop_stack.pop();
    }

    // Compile une liste d'instructions en gérant le nettoyage des variables locales (Scope)
    fn compile_scope(&mut self, statements: Vec<crate::ast::Statement>) {
        let initial_locals_count = self.locals.len();
        
        for stmt in statements {
            self.compile_instruction(stmt.kind);
        }
        
        let final_locals_count = self.locals.len();
        let vars_created = final_locals_count - initial_locals_count;
        
        // 1. On nettoie la pile d'exécution (Runtime)
        for _ in 0..vars_created {
            self.emit_op(OpCode::Pop);
        }
        
        // 2. On nettoie la table des symboles (Compile-time)
        // On retire toutes les variables qui ont un index >= initial_locals_count
        self.locals.retain(|_, &mut info| info.index < initial_locals_count as u8);
    }

    // Tente de réduire une expression constante
    fn evaluate_constant(&self, expr: &Expression) -> Option<Value> {
        match expr {
            // 1. Valeurs littérales (Feuilles de l'arbre)
            Expression::Literal(v) => Some(v.clone()),
            
            // 2. Arithmétique de base
            Expression::Add(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(Value::Integer(a + b)),
                    (Some(Value::Float(a)), Some(Value::Float(b))) => Some(Value::Float(a + b)),
                    (Some(Value::String(a)), Some(Value::String(b))) => Some(Value::String(format!("{}{}", a, b))),
                    _ => None
                }
            },
            
            Expression::Sub(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(Value::Integer(a - b)),
                    (Some(Value::Float(a)), Some(Value::Float(b))) => Some(Value::Float(a - b)),
                    _ => None
                }
            },

            Expression::Mul(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(Value::Integer(a * b)),
                    (Some(Value::Float(a)), Some(Value::Float(b))) => Some(Value::Float(a * b)),
                    _ => None
                }
            },

            Expression::Div(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => {
                        if b == 0 { None } else { Some(Value::Integer(a / b)) }
                    },
                    (Some(Value::Float(a)), Some(Value::Float(b))) => Some(Value::Float(a / b)),
                    _ => None
                }
            },

            // 3. Modulo
            Expression::Modulo(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => {
                        if b == 0 { None } else { Some(Value::Integer(a % b)) }
                    },
                    (Some(Value::Float(a)), Some(Value::Float(b))) => Some(Value::Float(a % b)),
                    _ => None
                }
            },

            // 4. Opérateurs Bitwise (Entiers uniquement)
            Expression::BitAnd(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(Value::Integer(a & b)),
                    _ => None
                }
            },
            Expression::BitOr(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(Value::Integer(a | b)),
                    _ => None
                }
            },
            Expression::BitXor(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => Some(Value::Integer(a ^ b)),
                    _ => None
                }
            },

            // 5. Shifts (Entiers uniquement, avec conversion safe vers u32)
            Expression::ShiftLeft(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => {
                        // Rust panic si shift < 0 ou shift >= bits du type.
                        // On ne fold que si le shift est sûr.
                        if let Ok(shift) = u32::try_from(b) {
                            if shift < 64 { return Some(Value::Integer(a << shift)); }
                        }
                        None
                    },
                    _ => None
                }
            },
            Expression::ShiftRight(left, right) => {
                match (self.evaluate_constant(left), self.evaluate_constant(right)) {
                    (Some(Value::Integer(a)), Some(Value::Integer(b))) => {
                        if let Ok(shift) = u32::try_from(b) {
                            if shift < 64 { return Some(Value::Integer(a >> shift)); }
                        }
                        None
                    },
                    _ => None
                }
            },

            // 6. Unaire (Not)
            Expression::Not(expr) => {
                match self.evaluate_constant(expr) {
                    Some(Value::Boolean(b)) => Some(Value::Boolean(!b)),
                    // En Aegis, !null est souvent true, mais restons stricts pour le folding :
                    Some(Value::Null) => Some(Value::Boolean(true)), 
                    _ => None
                }
            },

            // Tout ce qui contient une variable, un appel de fonction, etc. n'est pas constant
            _ => None,
        }
    }
}