pub mod compiler;
pub mod debug;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::value::FunctionData;
use crate::ast::{InstanceData, Value};
use crate::chunk::Chunk;
use crate::opcode::OpCode;
use crate::ast::environment::Environment;

const STACK_MAX: usize = 4096;

#[allow(dead_code)]
const FRAMES_MAX: usize = 64;

#[derive(Debug, Clone)]
struct CallFrame {
    closure: Value,       // Le code de la fonction
    ip: usize,          // Où on en est dans CETTE fonction
    slot_offset: usize, // Où commencent ses variables locales dans la pile globale (Base Pointer)
}

impl CallFrame {
    fn chunk(&self) -> &Chunk {
        match &self.closure {
            // On accède via le Rc
            Value::Function(rc_fn) => &rc_fn.chunk,
            _ => panic!("CallFrame closure is not a function"),
        }
    }
}

#[derive(Debug, Clone)]
struct ExceptionHandler {
    frame_index: usize, // L'index de la frame dans vm.frames
    catch_ip: usize,    // L'adresse du bloc catch
    stack_height: usize, // La hauteur de la pile de valeurs à restaurer
}

pub struct VM {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    globals: Vec<Value>,
    global_names: Rc<RefCell<HashMap<String, u8>>>,
    handlers: Vec<ExceptionHandler>,
    modules: HashMap<String, Value>,
}

impl VM {
    pub fn new(main_chunk: Chunk, global_names: Rc<RefCell<HashMap<String, u8>>>, args: Vec<String>) -> Self {
        let main_func = Value::Function(Rc::new(FunctionData {
            params: vec![],
            ret_type: None,
            chunk: main_chunk,
            env: None
        }));

        // Le script principal est la première "fonction" exécutée
        let main_frame = CallFrame {
            closure: main_func, // Utilise la closure
            ip: 0,
            slot_offset: 0,
        };

        let mut vm = VM {
            frames: Vec::with_capacity(64),
            stack: Vec::with_capacity(STACK_MAX),
            // On prépare de la place (256 slots globaux)
            globals: vec![Value::Null; 256],
            global_names,
            handlers: Vec::new(),
            modules: HashMap::new()
        };

        vm.frames.push(main_frame);

        let natives = crate::native::get_all_names();

        // Sécurité : On ne peut pas avoir plus de 256 globales avec des ID sur u8
        if natives.len() > 256 {
            panic!("Trop de fonctions natives pour la VM v2 (>256)");
        }

        for (i, name) in natives.into_iter().enumerate() {
            vm.globals[i] = Value::Native(name);
        }

        let args_values: Vec<Value> = args.iter().map(|s| Value::String(s.clone())).collect();
        let args_list = Value::List(Rc::new(RefCell::new(args_values)));

        // On doit trouver l'ID de "_ARGS" (ou un nom réservé)
        // Astuce : On l'ajoute manuellement à global_names et globals
        {
            let mut names = vm.global_names.borrow_mut();
            let id = names.len() as u8;
            names.insert("__ARGS__".to_string(), id);
            
            if id as usize >= vm.globals.len() {
                vm.globals.resize((id + 1) as usize, Value::Null);
            }
            vm.globals[id as usize] = args_list;
        }

        vm
    }

    // Helper pour récupérer la frame courante sans se battre avec le borrow checker
    fn current_frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("No code to execute")
    }

    // Helper pour la pile
    #[inline(always)]
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    #[inline(always)]
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

    #[inline(always)]
    fn step(&mut self) -> Result<bool, String> {
        // 1. Gestion des fins de Frames (Return implicite)
        // On vérifie d'abord si l'IP est au bout du code de la frame actuelle
        if self.current_frame().ip >= self.current_frame().chunk().code.len() {
            if self.frames.len() > 1 {
                self.frames.pop();
                return Ok(true); // On continue sur la frame parente
            } else {
                return Ok(false); // Plus de frames, fin du programme
            }
        }

        // 2. FETCH
        let byte = self.read_byte();
        let op: OpCode = byte.into();

        // EXECUTE WITH INTERCEPTION
        let result = self.execute_op(op);

        match result {
            Ok(keep_going) => Ok(keep_going),
            Err(msg) => {
                // --- ERROR HANDLING LOGIC ---
                // Une erreur est survenue (Result::Err).
                // Au lieu de crasher, on cherche un handler.
                
                if let Some(handler) = self.handlers.pop() {
                    // 1. Unwind frames : On remonte jusqu'à la frame qui a le try
                    while self.frames.len() > handler.frame_index + 1 {
                        self.frames.pop();
                    }
                    
                    // 2. Restore Stack : On nettoie la pile des calculs incomplets
                    self.stack.truncate(handler.stack_height);
                    
                    // 3. Push Error : On met le message d'erreur sur la pile
                    // (Ainsi le code 'catch' pourra le mettre dans une variable)
                    self.push(Value::String(msg));
                    
                    // 4. Jump : On déplace l'IP au début du bloc catch
                    self.current_frame().ip = handler.catch_ip;
                    
                    // On continue l'exécution !
                    Ok(true) 
                } else {
                    // Pas de handler ? On propage l'erreur (Crash)
                    Err(msg)
                }
            }
        }
    }

    pub fn run(&mut self) -> Result<(), String> {
        loop {
            match self.step() {
                Ok(true) => continue, // Continue loop
                Ok(false) => break,   // End of program
                Err(e) => {
                    // C'est ici qu'on enrichit l'erreur !
                    return Err(self.runtime_error(e));
                }
            }
        }
        Ok(())
    }

    // --- NOUVEAU : Helper pour MAP/FILTER ---
    // Cette fonction exécute une fonction Aegis (callback) de façon synchrone
    // C'est une "mini-vm" à l'intérieur de l'instruction
    fn run_callable_sync(&mut self, callable: Value, args: Vec<Value>) -> Result<Value, String> {
        // 1. On empile la fonction et les arguments comme un appel normal
        self.push(callable.clone());
        for arg in args.iter() {
            self.push(arg.clone());
        }

        // 2. On prépare la Frame (comme OpCode::Call)
        // Note: call_value empile la nouvelle frame
        self.call_value(callable, args.len())?;

        // 3. On note la profondeur actuelle de la pile de frames
        let start_depth = self.frames.len();

        // 4. BOUCLE SECONDAIRE : On exécute tant qu'on n'est pas revenu au niveau d'avant
        // C'est ici la magie : on fait tourner la VM "manuellement" pour ce callback
        while self.frames.len() >= start_depth {
            if !self.step()? {
                break; // Fin du programme inattendue
            }
        }

        // 5. Le résultat est sur la pile (la valeur de retour du callback)
        // Normalement, `OpCode::Return` a laissé la valeur de retour sur la pile
        Ok(self.pop())
    }

    #[inline(always)]
    fn execute_op(&mut self, op: OpCode) -> Result<bool, String> {
        // 2. EXECUTE
        match op {
            OpCode::Return => {
                let result = self.pop(); // La valeur de retour

                // On détruit la frame
                let frame = self.frames.pop().expect("No frame to return from");

                if self.frames.is_empty() {
                    // Fin du script principal
                    return Ok(true);
                }

                // Nettoyage de la pile : on enlève les arguments et les variables locales de la fonction
                // On remet la pile à l'état "avant l'appel" + le résultat
                self.stack.truncate(frame.slot_offset - 1);
                self.push(result);
            }
            OpCode::Call => {
                let arg_count = self.read_byte() as usize;
                let func_idx = self.stack.len() - 1 - arg_count;
                
                // VERSION UNSAFE : On récupère la fonction sans vérifier les bornes
                let target = unsafe { 
                    self.stack.get_unchecked(func_idx).clone() 
                };
                
                self.call_value(target, arg_count)?;
            },
            OpCode::Print => {
                let val = self.pop();
                println!("{}", val);
            }
            OpCode::LoadConst => {
                let idx = self.read_byte();
                let val = self.current_frame().chunk().constants[idx as usize].clone();
                self.push(val);
            }
            OpCode::Add => {
                // ASTUCE : On regarde les deux derniers éléments SANS les poper (peek)
                // Cela évite de déplacer la mémoire si on doit juste remplacer le résultat
                let len = self.stack.len();
                let right = &self.stack[len - 1];
                let left = &self.stack[len - 2];

                // FAST PATH : Si ce sont deux entiers, on calcule et on écrase
                if let (Value::Integer(b), Value::Integer(a)) = (right, left) {
                    let res = a + b;
                    // On retire virtuellement un élément (pop)
                    self.stack.truncate(len - 1);
                    // On écrase le dernier élément restant par le résultat
                    self.stack[len - 2] = Value::Integer(res);
                }
                // SLOW PATH : Le reste (String, Float...)
                else {
                    let b = self.pop();
                    let a = self.pop();

                    match (a, b) {
                        // Float + Float
                        (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 + v2)),
                        // Float + Int (Coercition)
                        (Value::Float(v1), Value::Integer(v2)) => {
                            self.push(Value::Float(v1 + v2 as f64))
                        }
                        (Value::Integer(v1), Value::Float(v2)) => {
                            self.push(Value::Float(v1 as f64 + v2))
                        }

                        // String + N'importe quoi
                        (Value::String(s1), val2) => {
                            self.push(Value::String(format!("{}{}", s1, val2)));
                        }
                        (val1, Value::String(s2)) => {
                            self.push(Value::String(format!("{}{}", val1, s2)));
                        }

                        _ => return Err("Type error in ADD".into()),
                    }
                }
            }
            OpCode::Sub => {
                let len = self.stack.len();
                let b_ref = &self.stack[len - 1];
                let a_ref = &self.stack[len - 2];

                // FAST PATH : Integer - Integer
                if let (Value::Integer(b), Value::Integer(a)) = (b_ref, a_ref) {
                    let res = a - b;
                    // On supprime le dernier élément (b)
                    self.stack.truncate(len - 1);
                    // On remplace l'avant-dernier (a) par le résultat
                    self.stack[len - 2] = Value::Integer(res);
                }
                // SLOW PATH : Le reste (Float...)
                else {
                    let b = self.pop();
                    let a = self.pop();
                    match (a, b) {
                        (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 - v2)),
                        (Value::Integer(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 - v2)),
                        (Value::Float(v1), Value::Integer(v2)) => self.push(Value::Float(v1 - v2 as f64)),
                        _ => return Err("Type error in SUB".into())
                    }
                }
            },
            OpCode::Mul => {
                let b = self.pop();
                let a = self.pop();
                match (a, b) {
                    (Value::Integer(v1), Value::Integer(v2)) => self.push(Value::Integer(v1 * v2)),
                    (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 * v2)),
                    (Value::Integer(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 * v2)),
                    (Value::Float(v1), Value::Integer(v2)) => self.push(Value::Float(v1 * v2 as f64)),
                    _ => return Err("Type error in MUL".into())
                }
            },
            OpCode::Div => {
                let b = self.pop();
                let a = self.pop();
                match (a, b) {
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        if v2 == 0 { return Err("Division by zero".into()); }
                        self.push(Value::Integer(v1 / v2))
                    },
                    (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 / v2)),
                    (Value::Integer(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 / v2)),
                    (Value::Float(v1), Value::Integer(v2)) => self.push(Value::Float(v1 / v2 as f64)),
                    _ => return Err("Type error in DIV".into())
                }
            },
            OpCode::SetGlobal => {
                let idx = self.read_byte() as usize;
                let val = self.pop();

                // Si l'index est plus grand que le tableau, on agrandit (sécurité)
                if idx >= self.globals.len() {
                    self.globals.resize(idx + 1, Value::Null);
                }

                self.globals[idx] = val;
            }
            OpCode::GetGlobal => {
                let idx = self.read_byte() as usize;
    
                // Logique de récupération avec Fallback
                let val = if idx < self.globals.len() && !matches!(self.globals[idx], Value::Null) {
                    // Cas nominal : La valeur est déjà là
                    self.globals[idx].clone()
                } else {
                    // Cas "Lazy" : On vérifie si c'est une nouvelle native
                    self.resolve_lazy_native(idx)
                        .ok_or_else(|| format!("Variable globale non définie (ID {})", idx))?
                };

                self.push(val);
            }
            OpCode::GetLocal => {
                let slot_idx = self.read_byte() as usize;
                let abs_index = self.current_frame().slot_offset + slot_idx;
                
                // VERSION UNSAFE
                // On évite le "Bounds Check" du vecteur stack
                let val = unsafe { 
                    self.stack.get_unchecked(abs_index).clone() 
                };
                self.push(val);
            }
            OpCode::SetLocal => {
                let slot_idx = self.read_byte() as usize;
                let abs_index = self.current_frame().slot_offset + slot_idx;

                let val = self.stack.last().expect("Stack empty").clone(); // Peek
                self.stack[abs_index] = val;
                // Note : SetLocal ne pop pas forcément la valeur (expression),
                // mais pour simplifier ici on peut dire qu'elle reste sur la pile.
            }
            OpCode::Jump => {
                let offset = self.read_short();
                self.current_frame().ip += offset as usize;
            }
            OpCode::JumpIfFalse => {
                let offset = self.read_short();
                // On peek la valeur (on ne la pop pas tout de suite pour la logique,
                // mais dans un if simple, le compilateur a émis un POP après)
                let condition = self.stack.last().expect("Empty stack");

                // Logique is_truthy simplifiée pour la VM
                let is_false = match condition {
                    Value::Boolean(b) => !(*b),
                    Value::Null => true,
                    Value::Integer(i) => *i == 0,
                    _ => false, // Tout le reste est vrai
                };

                if is_false {
                    self.current_frame().ip += offset as usize;
                }
            }
            OpCode::Loop => {
                let offset = self.read_short();
                // On soustrait l'offset à l'IP (on recule)
                self.current_frame().ip -= offset as usize;
            }
            OpCode::Pop => {
                self.pop();
            }
            OpCode::Modulo => {
                let b = self.pop();
                let a = self.pop();
                match (a, b) {
                    (Value::Integer(v1), Value::Integer(v2)) => self.push(Value::Integer(v1 % v2)),
                    _ => return Err("Type error %".into()),
                }
            }
            OpCode::Equal => {
                let b = self.pop();
                let a = self.pop();
                self.push(Value::Boolean(a == b));
            }
            OpCode::NotEqual => {
                let b = self.pop();
                let a = self.pop();
                self.push(Value::Boolean(a != b));
            }
            OpCode::Greater => {
                let b = self.pop();
                let a = self.pop();
                if let (Value::Integer(v1), Value::Integer(v2)) = (a, b) {
                    self.push(Value::Boolean(v1 > v2));
                } else {
                    self.push(Value::Boolean(false));
                }
            }
            OpCode::GreaterEqual => {
                let b = self.pop();
                let a = self.pop();
                if let (Value::Integer(v1), Value::Integer(v2)) = (a, b) {
                    self.push(Value::Boolean(v1 >= v2));
                } else {
                    self.push(Value::Boolean(false));
                }
            }
            OpCode::Less => {let len = self.stack.len();
                if len < 2 { return Err("Stack underflow in LESS".into()); }

                let b_ref = &self.stack[len - 1];
                let a_ref = &self.stack[len - 2];

                // FAST PATH : Integer < Integer
                if let (Value::Integer(b), Value::Integer(a)) = (b_ref, a_ref) {
                    let res = a < b;
                    self.stack.truncate(len - 1);
                    // On remplace l'Integer 'a' par un Boolean
                    self.stack[len - 2] = Value::Boolean(res);
                } 
                // SLOW PATH
                else {
                    let b = self.pop();
                    let a = self.pop();
                    if let (Value::Integer(v1), Value::Integer(v2)) = (&a, &b) {
                        self.push(Value::Boolean(v1 < v2));
                    } else if let (Value::Float(v1), Value::Float(v2)) = (&a, &b) {
                        self.push(Value::Boolean(v1 < v2));
                    } else {
                        // Comparaison mixte ou autre
                        // Note: Pour être rigoureux, il faudrait gérer Float vs Int ici aussi
                        self.push(Value::Boolean(false));
                    }
                }
            }
            OpCode::LessEqual => {
                let b = self.pop();
                let a = self.pop();
                if let (Value::Integer(v1), Value::Integer(v2)) = (a, b) {
                    self.push(Value::Boolean(v1 <= v2));
                } else {
                    self.push(Value::Boolean(false));
                }
            }
            OpCode::Not => {
                let val = self.pop();
                // is_truthy simplifié
                let b = match val {
                    Value::Boolean(v) => v,
                    Value::Null => false,
                    _ => true,
                };
                self.push(Value::Boolean(!b));
            }
            OpCode::BitAnd => {
                let b = self.pop().as_int().unwrap_or(0);
                let a = self.pop().as_int().unwrap_or(0);
                self.push(Value::Integer(a & b));
            }
            OpCode::BitOr => {
                let b = self.pop().as_int().unwrap_or(0);
                let a = self.pop().as_int().unwrap_or(0);
                self.push(Value::Integer(a | b));
            }
            OpCode::BitXor => {
                let b = self.pop().as_int().unwrap_or(0);
                let a = self.pop().as_int().unwrap_or(0);
                self.push(Value::Integer(a ^ b));
            }
            OpCode::ShiftLeft => {
                let b = self.pop().as_int().unwrap_or(0);
                let a = self.pop().as_int().unwrap_or(0);
                self.push(Value::Integer(a << b));
            }
            OpCode::ShiftRight => {
                let b = self.pop().as_int().unwrap_or(0);
                let a = self.pop().as_int().unwrap_or(0);
                self.push(Value::Integer(a >> b));
            }
            OpCode::MakeList => {
                let count = self.read_byte() as usize;
                let mut items = Vec::new();
                // On dépile dans l'ordre inverse pour retrouver l'ordre initial
                for _ in 0..count {
                    items.push(self.pop());
                }
                items.reverse();
                self.push(Value::List(std::rc::Rc::new(std::cell::RefCell::new(
                    items,
                ))));
            }
            OpCode::Method => self.op_method()?,
            OpCode::MakeDict => {
                let count = self.read_byte() as usize; // Nombre d'éléments total sur la pile (clés + valeurs)
                let num_pairs = count / 2;
                let mut dict = HashMap::new();

                // Pile : [k1, v1, k2, v2...]
                // Pop : v2, k2, v1, k1...

                for _ in 0..num_pairs {
                    let val = self.pop();
                    let key_val = self.pop();
                    let key = key_val.as_str().unwrap_or("unknown".to_string());
                    dict.insert(key, val);
                }

                self.push(Value::Dict(Rc::new(RefCell::new(dict))));
            }
            OpCode::GetAttr => {
                let name_idx = self.read_byte();
                let attr_name = self.current_frame().chunk().constants[name_idx as usize].to_string();
                let obj = self.pop();

                match obj {
                    Value::Instance(inst) => {
                        let val = inst
                            .borrow()
                            .fields
                            .get(&attr_name)
                            .cloned()
                            .unwrap_or(Value::Null);
                        self.push(val);
                    }
                    Value::Dict(d) => {
                        let val = d.borrow().get(&attr_name).cloned().unwrap_or(Value::Null);
                        self.push(val);
                    }
                    // On pourrait ajouter d'autres types (ex: Module)
                    _ => {
                        return Err(format!(
                            "Impossible de lire l'attribut '{}' sur ce type",
                            attr_name
                        )
                        .into());
                    }
                }
            }
            OpCode::SetAttr => {
                let name_idx = self.read_byte();
                let attr_name = self.current_frame().chunk().constants[name_idx as usize].to_string();

                let val = self.pop(); // La valeur à assigner
                let obj = self.pop(); // L'objet

                match obj {
                    Value::Instance(inst) => {
                        inst.borrow_mut().fields.insert(attr_name, val.clone());
                        self.push(val); // L'expression d'assignation retourne la valeur
                    }
                    Value::Dict(d) => {
                        d.borrow_mut().insert(attr_name, val.clone());
                        self.push(val);
                    }
                    _ => return Err("Impossible d'assigner un attribut sur ce type".into()),
                }
            }

            OpCode::Input => {
                let prompt = self.pop();
                print!("{}", prompt);

                // Force l'affichage immédiat
                use std::io::Write;
                std::io::stdout().flush().unwrap();

                let mut buffer = String::new();
                std::io::stdin().read_line(&mut buffer).unwrap();
                let input = buffer.trim().to_string();

                self.push(Value::String(input));
            }

            OpCode::Class => {
                let idx = self.read_byte();
                let class_val = self.current_frame().chunk().constants[idx as usize].clone();
                self.push(class_val);
            }

            OpCode::MakeClosure => {
                let function_val = self.pop();
                
                if let Value::Function(rc_fn) = function_val {
                    let env_rc = Environment::new_global();
                    
                    // 1. Extraction (Attention : il faut accéder aux champs du Rc)
                    let (parent_params, parent_locals_map, slot_offset) = {
                        let frame = self.current_frame();
                        
                        let pp = if let Value::Function(parent_rc) = &frame.closure {
                            Some(parent_rc.params.clone()) // On clone le Vec<Params>
                        } else {
                            None
                        };
                        
                        let locals = frame.chunk().locals_map.clone();
                        (pp, locals, frame.slot_offset)
                    };

                    // 2. Population Phase (Fill the environment)
                    // SCOPE START: We create a block to contain the mutable borrow
                    {
                        let mut env_inner = env_rc.borrow_mut();

                        // A. Capture Arguments
                        if let Some(parent_params) = parent_params {
                            for (i, (name, _)) in parent_params.iter().enumerate() {
                                if slot_offset + i < self.stack.len() {
                                    let val = self.stack[slot_offset + i].clone();
                                    env_inner.variables.insert(name.clone(), val);
                                }
                            }
                        }

                        // B. Capture Locals (The fix for your "line" variable)
                        for (idx, name) in parent_locals_map {
                            let abs_index = slot_offset + (idx as usize);
                            if abs_index < self.stack.len() {
                                let val = self.stack[abs_index].clone();
                                // We insert into the closure environment
                                env_inner.variables.insert(name, val);
                            }
                        }
                    } 
                    // SCOPE END: 'env_inner' is dropped here. 'env_rc' is now free!

                    // 3. Creation (On doit créer un NOUVEAU FunctionData)
                    // Note: rc_fn.chunk est un clone couteux ici ? 
                    // Non, Chunk contient des Vec. Idéalement Chunk devrait être dans un Rc aussi,
                    // mais FunctionData est déjà un gros progrès.
                    
                    let new_data = FunctionData {
                        params: rc_fn.params.clone(),
                        ret_type: rc_fn.ret_type.clone(),
                        chunk: rc_fn.chunk.clone(), // On clone le chunk (lourd, mais nécessaire pour l'instant)
                        env: Some(env_rc)
                    };

                    let closure = Value::Function(Rc::new(new_data));
                    self.push(closure);
                } else {
                    panic!("MakeClosure on non-function value");
                }
            },

            OpCode::GetFreeVar => {
                let name_idx = self.read_byte();
                // Récupération du nom
                let name = {
                    let frame = self.current_frame();
                    if let Value::Function(rc_fn) = &frame.closure {
                        rc_fn.chunk.constants[name_idx as usize].to_string()
                    } else {
                        panic!("Frame sans closure fonctionnelle ?");
                    }
                };

                let mut val_to_push = None;

                // 1. Essai : Closure Environment
                {
                    let frame = self.current_frame();
                    // On match le Rc
                    if let Value::Function(rc_fn) = &frame.closure {
                        if let Some(env) = &rc_fn.env { // on accède au champ .env du struct
                            if let Some(val) = env.borrow().variables.get(&name) {
                                val_to_push = Some(val.clone());
                            }
                        }
                    }
                }

                // 2. Essai : Global Environment (Fallback)
                if val_to_push.is_none() {
                    let global_id_opt = self.global_names.borrow().get(&name).cloned();
                    
                    if let Some(id) = global_id_opt {
                        let idx = id as usize;
                        
                        // Même logique que GetGlobal
                        if idx < self.globals.len() && !matches!(self.globals[idx], Value::Null) {
                            val_to_push = Some(self.globals[idx].clone());
                        } else {
                            // Tentative de résolution native tardive
                            val_to_push = self.resolve_lazy_native(idx);
                        }
                    }
                }

                // 3. Résultat
                if let Some(val) = val_to_push {
                    self.push(val);
                } else {
                    return Err(format!("Variable introuvable (ni locale, ni globale) : '{}'", name));
                }
            },

            OpCode::Dup => {
                // On regarde le dernier élément sans le poper
                let val = self.stack.last().expect("Stack underflow in DUP").clone();
                self.push(val);
            },

            OpCode::SetupExcept => {
                let offset = self.read_short();
                let handler = ExceptionHandler {
                    frame_index: self.frames.len() - 1,
                    catch_ip: self.current_frame().ip + (offset as usize),
                    stack_height: self.stack.len(),
                };
                self.handlers.push(handler);
            },
            OpCode::PopExcept => {
                self.handlers.pop();
            },
            OpCode::Throw => {
                let msg = self.pop();
                return Err(format!("{}", msg)); // On utilise le mécanisme standard d'erreur Rust
            },

            OpCode::Import => {
                let path_idx = self.read_byte();
                let path = self.current_frame().chunk().constants[path_idx as usize].to_string();

                // 1. CACHE CHECK
                // If module is already loaded, we don't re-execute it (prevents side-effect duplication)
                if self.modules.contains_key(&path) {
                    self.push(Value::Null); // Import returns Null
                } else {
                    // 2. LOAD FILE
                    // Reads relative to CWD. You might want to handle absolute paths or include paths later.
                    let source = std::fs::read_to_string(&path)
                        .map_err(|e| format!("Failed to import '{}': {}", path, e))?;

                    // 3. FRONTEND (Source -> AST)
                    // We reuse the v1 compiler pipeline to get instructions
                    let json_ast = crate::compiler::compile(&source)?;
                    let statements = crate::loader::parse_block(&json_ast)?;
                    let instructions: Vec<crate::ast::Instruction> = statements.into_iter().map(|s| s.kind).collect();

                    // 4. BACKEND (AST -> Bytecode)
                    // CRITICAL: We create a compiler that SHARES the global_names with the main VM.
                    // This ensures that 'namespace System' in the module gets the same Global ID 
                    // as 'System' in the main script.
                    let mut module_compiler = crate::vm::compiler::Compiler::new_with_globals(self.global_names.clone());
                    
                    // CRITICAL: We force GLOBAL scope (0) so 'var' and 'func' become SET_GLOBAL
                    module_compiler.scope_depth = 0; 

                    for instr in instructions {
                        module_compiler.compile_instruction(instr);
                    }
                    
                    // 5. EXECUTION
                    let module_chunk = module_compiler.chunk;
                    
                    // Wrap module code in a function to execute it
                    let module_func = Value::Function(Rc::new(FunctionData {
                        params: vec![],
                        ret_type: None,
                        chunk: module_chunk,
                        env: None
                    }));
                    
                    // Run the module synchronously.
                    // Its instructions (SET_GLOBAL) will write directly to 'self.globals'.
                    self.run_callable_sync(module_func, vec![])?;

                    // 6. UPDATE CACHE
                    self.modules.insert(path.clone(), Value::Boolean(true));
                    
                    // 7. RETURN
                    self.push(Value::Null);
                }
            },
            OpCode::CheckType => {
                let type_name_idx = self.read_byte();
                let expected_type = self.current_frame().chunk().constants[type_name_idx as usize].to_string();
                
                // On regarde la valeur sur le sommet de la pile (sans la pop)
                let val = self.stack.last().expect("Stack underflow in CheckType");
                
                // Vérification
                let is_valid = match (val, expected_type.as_str()) {
                    (Value::Integer(_), "int") => true,
                    (Value::Float(_), "float") => true,
                    (Value::String(_), "string") => true,
                    (Value::Boolean(_), "bool") => true,
                    (Value::List(_), "list") => true,
                    (Value::Dict(_), "dict") => true,
                    (Value::Function(_), "func") => true, // Ou "function"
                    (Value::Null, _) => false, // Null n'est généralement pas le type attendu (sauf "any" ?)
                    (_, "any") => true,
                    _ => false,
                };

                if !is_valid {
                    return Err(format!(
                        "Erreur de Type: Attendu '{}', recu '{}'", 
                        expected_type, val
                    ));
                }
            },

            OpCode::Super => {
                let method_idx = self.read_byte();
                let arg_count = self.read_byte() as usize;
                let parent_idx = self.read_byte(); // Le 3ème argument

                let chunk = self.current_frame().chunk();
                let method_name = chunk.constants[method_idx as usize].to_string();
                let parent_name = chunk.constants[parent_idx as usize].to_string();

                // L'objet 'this' est sur la pile, juste avant les args
                let obj_idx = self.stack.len() - 1 - arg_count;
                let obj = self.stack[obj_idx].clone(); // On garde 'this' pour l'appel

                // On résout la classe parente DEPUIS LE NOM GRAVÉ DANS LE BYTECODE
                // C'est ça qui évite la récursion infinie.
                // Si Animal.speak appelle super, le bytecode contient "LivingBeing".
                // Si Dog.speak appelle super, le bytecode contient "Animal".
                
                if let Some(parent_class_val) = self.get_global_by_name(&parent_name) {
                    // On commence la recherche directement à ce niveau
                    let mut current_class_val = Rc::new(parent_class_val);
                    
                    // Logique identique à op_method, mais on commence au parent
                    loop {
                        if let Value::Class(rc_class) = &*current_class_val {
                            // A. Trouvé ?
                            if let Some(method_val) = rc_class.methods.get(&method_name) {
                                // On remplace 'this' par la méthode sur la pile
                                // ET on réinsère 'this' (comme op_method)
                                self.stack[obj_idx] = method_val.clone();
                                self.stack.insert(obj_idx + 1, obj.clone());
                                
                                self.call_value(method_val.clone(), arg_count + 1)?;
                                return Ok(true); // Continue VM
                            }
                            
                            // B. Remonter encore ?
                            if let Some(grand_parent) = &rc_class.parent {
                                if let Some(gp_val) = self.get_global_by_name(grand_parent) {
                                    current_class_val = Rc::new(gp_val);
                                    continue;
                                }
                            }
                        }
                        return Err(format!("Méthode '{}' introuvable dans le parent '{}' (ou ses ancêtres)", method_name, parent_name));
                    }
                } else {
                    return Err(format!("Classe parente '{}' introuvable au runtime", parent_name));
                }
            },
        }

        Ok(true)
    }

    fn op_method(&mut self) -> Result<(), String> {
        let name_idx = self.read_byte();
        let arg_count = self.read_byte() as usize;

        // Name resolution
        let method_name_val = &self.current_frame().chunk().constants[name_idx as usize];
        let method_name = match method_name_val {
            Value::String(s) => s.clone(),
            _ => method_name_val.to_string(),
        };

        let obj_idx = self.stack.len() - 1 - arg_count;
        let obj = self.stack[obj_idx].clone();

        // 1. Instance Methods (POO)
        if let Value::Instance(inst) = &obj {
             // On commence par la classe de l'instance
             let mut current_class_val = inst.borrow().class.clone();
             
             // Boucle de recherche (Prototype Chain)
             loop {
                 if let Value::Class(rc_class) = &*current_class_val {
                     // A. Est-ce que la méthode est ici ?
                     if let Some(method_val) = rc_class.methods.get(&method_name) {
                         self.stack[obj_idx] = method_val.clone();
                         self.stack.insert(obj_idx + 1, obj.clone()); 
                         self.call_value(method_val.clone(), arg_count + 1)?; 
                         return Ok(()); // Trouvé et appelé !
                     }
                     
                     // B. Sinon, a-t-on un parent ?
                     if let Some(parent_name) = &rc_class.parent {
                         // On cherche la classe parente dans les globales
                         if let Some(parent_class) = self.get_global_by_name(parent_name) {
                             // On remonte d'un cran
                             current_class_val = Rc::new(parent_class);
                             continue;
                         } else {
                             return Err(format!("Classe parente introuvable : '{}'", parent_name).into());
                         }
                     }
                 }
                 // Si on arrive ici, c'est qu'on n'a pas trouvé et qu'il n'y a plus de parent
                 break; 
             }
        }

        if let Value::Dict(d) = &obj {
            // On regarde si la clé existe dans le dictionnaire
            let field_val = d.borrow().get(&method_name).cloned();

            if let Some(val) = field_val {
                // Si la valeur trouvée est une fonction (ou native), on l'exécute
                if matches!(val, Value::Function(..) | Value::Native(..)) {
                    
                    // On remplace le Dictionnaire sur la pile par la Fonction trouvée
                    // Stack avant : [Dict, Arg1, Arg2...]
                    // Stack après : [Func, Arg1, Arg2...]
                    self.stack[obj_idx] = val.clone();
                    
                    // Note : Contrairement aux Instances, on n'injecte PAS 'this'.
                    // Les fonctions de namespace sont considérées comme statiques.
                    
                    self.call_value(val, arg_count)?;
                    return Ok(()); // L'appel est géré, on rend la main à la boucle principale
                }
            }
        }

        // 2. Native Methods
        let args: Vec<Value> = self.stack.drain((obj_idx + 1)..).collect();
        let _obj_popped = self.pop(); // Pop object

        let result = match obj {
            Value::List(l) => match method_name.as_str() {
                "push" => { l.borrow_mut().push(args[0].clone()); Value::Null },
                "pop" => l.borrow_mut().pop().unwrap_or(Value::Null),
                "at" => { 
                    let idx = args[0].as_int().unwrap_or(0) as usize;
                    l.borrow().get(idx).cloned().unwrap_or(Value::Null) 
                },
                "len" => Value::Integer(l.borrow().len() as i64),

                "reverse" => {
                    l.borrow_mut().reverse();
                    Value::List(l.clone())
                },

                "contains" => {
                    let target = &args[0];
                    let exists = l.borrow().contains(target); // Nécessite que Value implémente PartialEq (c'est le cas)
                    Value::Boolean(exists)
                },

                "join" => {
                    let sep = if args.is_empty() { "".to_string() } else { args[0].as_str().unwrap_or_default() };
                    
                    let list_borrow = l.borrow();
                    // On convertit tout en string et on joint
                    let strings: Vec<String> = list_borrow.iter().map(|v| v.to_string()).collect();
                    
                    Value::String(strings.join(&sep))
                },
                
                // --- FUNCTIONAL PROGRAMMING ---
                
                "map" => {
                    let callback = args[0].clone();
                    let list_data = l.borrow().clone(); // Clone to avoid RefCell borrow conflict during callback
                    let mut new_list = Vec::new();

                    for item in list_data {
                        // On appelle la VM récursivement pour chaque élément !
                        let res = self.run_callable_sync(callback.clone(), vec![item])?;
                        new_list.push(res);
                    }
                    Value::List(Rc::new(RefCell::new(new_list)))
                },

                "filter" => {
                    let callback = args[0].clone();
                    let list_data = l.borrow().clone();
                    let mut new_list = Vec::new();

                    for item in list_data {
                        let res = self.run_callable_sync(callback.clone(), vec![item.clone()])?;
                        // On garde si le résultat est "truthy"
                        if matches!(res, Value::Boolean(true)) || (res.as_int().unwrap_or(0) != 0 && !matches!(res, Value::Null)) {
                            new_list.push(item);
                        }
                    }
                    Value::List(Rc::new(RefCell::new(new_list)))
                },

                "for_each" => {
                    let callback = args[0].clone();
                    let list_data = l.borrow().clone();
                    
                    for item in list_data {
                        // On exécute juste pour l'effet de bord, on ignore le résultat
                        self.run_callable_sync(callback.clone(), vec![item])?;
                    }
                    Value::Null
                },

                _ => return Err(format!("Unknown list method '{}'", method_name).into())
            },
            
            // ... Dict methods (insert, keys, get...) inchangés ...
            Value::Dict(d) => match method_name.as_str() {
                "insert" => {
                    if args.len() < 2 { return Err("insert needs 2 args".into()); }
                    let key = args[0].as_str().unwrap_or("?".to_string());
                    d.borrow_mut().insert(key, args[1].clone());
                    Value::Null
                },
                "keys" => {
                    let keys: Vec<Value> = d.borrow().keys().map(|k| Value::String(k.clone())).collect();
                    Value::List(Rc::new(RefCell::new(keys)))
                },
                "get" => {
                     let key = args[0].as_str().unwrap_or("?".to_string());
                     d.borrow().get(&key).cloned().unwrap_or(Value::Null)
                },
                _ => return Err(format!("Unknown dict method '{}'", method_name).into())
            },

            Value::String(s) => match method_name.as_str() {
                "len" => Value::Integer(s.len() as i64),
                
                // --- Transformation ---
                "trim" => {
                    // Rust fait ça très bien nativement
                    Value::String(s.trim().to_string())
                },
                "upper" => Value::String(s.to_uppercase()),
                "lower" => Value::String(s.to_lowercase()),

                // --- Analyse ---
                "contains" => { // NOUVEAU
                    let sub = args[0].as_str().unwrap_or_default();
                    Value::Boolean(s.contains(&sub))
                },
                "starts_with" => { // NOUVEAU
                    let sub = args[0].as_str().unwrap_or_default();
                    Value::Boolean(s.starts_with(&sub))
                },
                "ends_with" => { // NOUVEAU
                    let sub = args[0].as_str().unwrap_or_default();
                    Value::Boolean(s.ends_with(&sub))
                },

                // --- Modification ---
                "replace" => {
                    if args.len() < 2 { return Err("String.replace attend 2 arguments (old, new)".into()); }
                    
                    // On sécurise la récupération des chaînes
                    let old_part = args[0].as_str().unwrap_or("".to_string());
                    let new_part = args[1].as_str().unwrap_or("".to_string());
                    
                    Value::String(s.replace(&old_part, &new_part))
                },

                "split" => {
                    let delim = if args.is_empty() { 
                        " ".to_string() 
                    } else { 
                        args[0].as_str().unwrap_or(" ".to_string()) 
                    };

                    // On découpe et on convertit chaque morceau en Value::String
                    let parts: Vec<Value> = s.split(&delim)
                        .map(|sub| Value::String(sub.to_string()))
                        .collect();
                    
                    // On retourne une Value::List
                    Value::List(Rc::new(RefCell::new(parts)))
                },

                _ => return Err(format!("Méthode string inconnue '{}'", method_name).into())
            },

            Value::Instance(_) => return Err(format!("Instance has no method '{}'", method_name).into()),
            _ => return Err(format!("Method '{}' not supported on {:?}", method_name, obj).into())
        };

        self.push(result);
        Ok(())
    }

    // Helper pour lire l'octet suivant et avancer IP
    #[inline(always)]
    fn read_byte(&mut self) -> u8 {
        let frame = self.current_frame();
        // VERSION UNSAFE (Plus rapide)
        unsafe {
            // On suppose que le compilateur n'a jamais généré un saut hors du code
            let b = *frame.chunk().code.get_unchecked(frame.ip);
            frame.ip += 1;
            b
        }
    }

    fn read_short(&mut self) -> u16 {
        let frame = self.current_frame();
        let ip = frame.ip;
        frame.ip += 2;
        ((frame.chunk().code[ip] as u16) << 8) | frame.chunk().code[ip + 1] as u16
    }

    fn call_value(&mut self, target: Value, arg_count: usize) -> Result<(), String> {
        let func_idx = self.stack.len() - 1 - arg_count;

        match &target {
            // CAS 1 : Fonction Aegis
            Value::Function(rc_fn) => { 
                 // On accède aux champs via rc_fn
                 if arg_count != rc_fn.params.len() { 
                    return Err(format!("Arity mismatch: attendu {}, reçu {}", rc_fn.params.len(), arg_count)); 
                 }
                 
                 let frame = CallFrame {
                     closure: target.clone(), // Clone le Rc (rapide !)
                     ip: 0,
                     slot_offset: func_idx + 1,
                 };
                 
                 self.frames.push(frame);
                 Ok(())
            },

            // CAS 2 : Classe
            Value::Class(rc_class) => { // Déstructuration newtype
                if arg_count != rc_class.params.len() { 
                    return Err(format!("Constructeur {}: attendu {} args", rc_class.name, rc_class.params.len())); 
                }
                
                // Ici target contient déjà un Rc<ClassData>, le cloner est rapide
                let class_val_rc = Rc::new(target.clone());
                
                let mut fields = HashMap::new();
                for (i, (param_name, _)) in rc_class.params.iter().enumerate() {
                    let arg_val = self.stack[func_idx + 1 + i].clone();
                    fields.insert(param_name.clone(), arg_val);
                }
                
                let instance = Value::Instance(Rc::new(RefCell::new(InstanceData {
                    class: class_val_rc,
                    fields
                })));
                
                self.stack[func_idx] = instance;
                self.stack.truncate(func_idx + 1);
                Ok(())
            },

            // CAS 3 : Fonction Native
            Value::Native(name) => {
                let func_ptr = crate::native::find(&name)
                    .ok_or(format!("Fonction native '{}' introuvable", name))?;

                let args_start = func_idx + 1;
                let args: Vec<Value> = self.stack.drain(args_start..).collect();

                let result = func_ptr(args)?;

                self.stack.pop(); // Pop la fonction native
                self.push(result);
                Ok(())
            }

            _ => Err(format!(
                "Tentative d'appel sur {:?} qui n'est pas une fonction",
                target
            )),
        }
    }

    fn resolve_lazy_native(&mut self, global_id: usize) -> Option<Value> {
        // 1. Retrouver le nom à partir de l'ID
        let name = {
            let names = self.global_names.borrow();
            names.iter()
                // CORRECTION ICI : On déstructure explicitement la référence externe
                .find(|&(_, &id)| id as usize == global_id)
                .map(|(k, _)| k.clone())
        }?; 

        // 2. Chercher dans le registre natif
        // on veut juste savoir si 'find' retourne Some(...)
        if let Some(_) = crate::native::find(&name) {
            let val = Value::Native(name);
            
            // 3. Mettre en cache dans les globales
            if global_id >= self.globals.len() {
                self.globals.resize(global_id + 1, Value::Null);
            }
            self.globals[global_id] = val.clone();
            
            return Some(val);
        }

        None
    }

    /// Injecte et exécute un nouveau Chunk dans la VM existante (pour le REPL)
    pub fn execute_chunk(&mut self, chunk: Chunk) -> Result<(), String> {
        // On crée une fonction fictive pour emballer ce chunk
        let script_func = Value::Function(Rc::new(crate::ast::value::FunctionData {
            params: vec![],
            ret_type: None,
            chunk,
            env: None
        }));

        // On crée une nouvelle Frame au niveau 0 (comme le main)
        let frame = CallFrame {
            closure: script_func,
            ip: 0,
            slot_offset: 0,
        };

        // On l'ajoute à la pile d'appels
        self.frames.push(frame);

        // Et on lance l'exécution !
        self.run()
    }

    fn runtime_error(&self, message: String) -> String {
        let frame = self.frames.last().expect("No frame for error");
        let chunk = frame.chunk();
        
        // On récupère l'IP précédent (l'instruction qui a causé l'erreur)
        let ip = if frame.ip > 0 { frame.ip - 1 } else { 0 };
        
        // On récupère la ligne
        let line = if ip < chunk.lines.len() {
            chunk.lines[ip]
        } else {
            0
        };

        format!("[Line {}] Error: {}", line, message)
    }

    fn get_global_by_name(&self, name: &str) -> Option<Value> {
        let global_id = self.global_names.borrow().get(name).cloned()?;
        let val = self.globals.get(global_id as usize)?;
        if matches!(val, Value::Null) { None } else { Some(val.clone()) }
    }
}
