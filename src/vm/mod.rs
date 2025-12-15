pub mod compiler;
pub mod debug;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::value::{ClassData, FunctionData, Visibility};
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
    class_context: Option<Rc<ClassData>>, // La classe dans laquelle on s'exécute (pour private/protected)
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
            class_context: None
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
                if let Some(handler) = self.handlers.pop() {
                    // 1. Unwind frames
                    while self.frames.len() > handler.frame_index + 1 {
                        self.frames.pop();
                    }
                    
                    // 2. Restore Stack - C'EST LA CLÉ
                    // On coupe brutalement la pile à la hauteur enregistrée lors du 'try'
                    if handler.stack_height <= self.stack.len() {
                        self.stack.truncate(handler.stack_height);
                    } else {
                        // Corruption grave : la pile est plus petite qu'au début du try !
                        return Err("Critical VM Error: Stack corrupted during unwind".into());
                    }
                    
                    // 3. Push Error
                    self.push(Value::String(msg));
                    
                    // 4. Jump
                    self.current_frame().ip = handler.catch_ip;
                    Ok(true) 
                } else {
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
    fn run_callable_sync(&mut self, callable: Value, args: Vec<Value>, context: Option<Rc<ClassData>>) -> Result<Value, String> {
        // 1. On empile la fonction et les arguments comme un appel normal
        self.push(callable.clone());
        for arg in args.iter() {
            self.push(arg.clone());
        }

        // 2. On prépare la Frame (comme OpCode::Call)
        // Note: call_value empile la nouvelle frame
        self.call_value(callable, args.len(), context)?;

        // 3. On note la profondeur actuelle de la pile de frames
        let start_depth = self.frames.len();

        // 4. BOUCLE SECONDAIRE : On exécute tant qu'on n'est pas revenu au niveau d'avant
        // C'est ici la magie : on fait tourner la VM "manuellement" pour ce callback
        while self.frames.len() >= start_depth {
            if self.frames.is_empty() {
                return Err("VM Panic: Call stack exhausted during sync execution".into());
            }

            match self.step() {
                Ok(true) => continue,
                Ok(false) => break, // Fin normale du programme (ne devrait pas arriver ici)
                Err(e) => {
                    // Si une erreur survient et n'est pas attrapée par un try/catch interne,
                    // elle remonte ici. On doit propager l'erreur et arrêter la mini-VM.
                    return Err(self.runtime_error(e));
                }
            }
        }

        // 5. Le résultat est sur la pile (la valeur de retour du callback)
        // Normalement, `OpCode::Return` a laissé la valeur de retour sur la pile
        if self.stack.is_empty() {
             return Ok(Value::Null); // Sécurité
        }

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
                
                // SÉCURITÉ : Vérifier qu'on a assez d'éléments sur la pile
                if self.stack.len() < 1 + arg_count {
                    return Err(format!("Stack underflow during Call (args: {})", arg_count));
                }

                let func_idx = self.stack.len() - 1 - arg_count;
                
                // VERSION SAFE
                let target = self.stack[func_idx].clone();
                
                self.call_value(target, arg_count, None)?;
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
    
                // 1. On récupère la valeur brute. 
                // Si l'index est hors limite (ne devrait pas arriver si le compilateur est bon), on met Null.
                let mut val = if idx < self.globals.len() {
                    self.globals[idx].clone()
                } else {
                    Value::Null
                };

                // 2. Si la valeur est Null, on vérifie si c'est une fonction Native "paresseuse" (Lazy)
                if matches!(val, Value::Null) {
                    if let Some(native_val) = self.resolve_lazy_native(idx) {
                        val = native_val;
                    }
                    // SINON : C'est juste une variable qui contient Null. Ce n'est PAS une erreur.
                    // On laisse val à Value::Null.
                }

                self.push(val);
            },
            OpCode::GetLocal => {
                let slot_idx = self.read_byte() as usize;
                let abs_index = self.current_frame().slot_offset + slot_idx;
                
                // VERSION SAFE
                if let Some(val) = self.stack.get(abs_index) {
                    self.push(val.clone());
                } else {
                    return Err(format!("Stack access out of bounds (local: {}, abs: {}, stack_len: {})", 
                        slot_idx, abs_index, self.stack.len()));
                }
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
                        let class_rc = inst.borrow().class.clone();
                        self.check_access(&class_rc, &attr_name)?;

                        // 1. Check Properties (Instance)
                        // On doit chercher dans toute la hiérarchie
                        let mut lookup_class = Some(class_rc.clone());
                        let mut found_prop = None;
                        
                        while let Some(c) = lookup_class {
                            if let Some(prop) = c.properties.get(&attr_name) {
                                found_prop = Some((prop.clone(), c.clone()));
                                break;
                            }
                            lookup_class = c.parent_ref.clone();
                        }

                        if let Some((prop, owner_class)) = found_prop {
                            if let Some(getter) = &prop.getter {
                                // Appel du getter : On remet 'this' sur la pile
                                self.push(getter.clone());
                                self.push(Value::Instance(inst.clone())); 
                                self.call_value(getter.clone(), 1, Some(owner_class))?; 
                                return Ok(true); // On laisse la VM exécuter le getter
                            } else {
                                return Err(format!("Property '{}' is write-only", attr_name));
                            }
                        }

                        // 2. Champs classiques
                        let val = inst.borrow().fields.get(&attr_name).cloned().unwrap_or(Value::Null);
                        self.push(val);
                    }
                    Value::Class(class_rc) => {
                        self.check_access(&class_rc, &attr_name)?;

                        // 1. Check Static Properties
                        // Pour l'instant on cherche juste dans la classe elle-même (pas d'héritage statique complexe)
                        if let Some(prop) = class_rc.static_properties.get(&attr_name) {
                            if let Some(getter) = &prop.getter {
                                // 'this' pour un statique est la Classe elle-même
                                self.push(getter.clone());
                                self.push(Value::Class(class_rc.clone()));
                                self.call_value(getter.clone(), 1, Some(class_rc.clone()))?;
                                return Ok(true);
                            } else {
                                return Err(format!("Static Property '{}' is write-only", attr_name));
                            }
                        }

                        // 2. Static Fields
                        if let Some(val) = class_rc.static_fields.borrow().get(&attr_name) {
                            self.push(val.clone());
                        } 
                        // 3. Static Methods
                        else if let Some(method) = class_rc.static_methods.get(&attr_name) {
                            self.push(method.clone());
                        } else {
                            return Err(format!("Unknown static member '{}'", attr_name));
                        }
                    }
                    Value::Dict(d) => {
                        let val = d.borrow().get(&attr_name).cloned().unwrap_or(Value::Null);
                        self.push(val);
                    }
                    Value::Enum(e) => {
                        // Accès direct sans borrow() car pas de RefCell
                        let val = e.get(&attr_name).cloned().unwrap_or(Value::Null);
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
                        let class_rc = inst.borrow().class.clone();
                        self.check_access(&class_rc, &attr_name)?;

                        // 1. Check Properties (Instance)
                        let mut lookup_class = Some(class_rc.clone());
                        let mut found_prop = None;
                        while let Some(c) = lookup_class {
                            if let Some(prop) = c.properties.get(&attr_name) {
                                found_prop = Some((prop.clone(), c.clone()));
                                break;
                            }
                            lookup_class = c.parent_ref.clone();
                        }

                        if let Some((prop, owner_class)) = found_prop {
                            if let Some(setter) = &prop.setter {
                                // Appel Setter
                                // On remet les arguments pour call_value
                                self.push(setter.clone());
                                self.push(Value::Instance(inst.clone())); // arg 0: this
                                self.push(val.clone());                   // arg 1: value
                                
                                self.call_value(setter.clone(), 2, Some(owner_class))?;
                                return Ok(true);
                            } else {
                                return Err(format!("Property '{}' is read-only", attr_name));
                            }
                        }

                        // 2. Champs classiques
                        inst.borrow_mut().fields.insert(attr_name, val.clone());
                        self.push(val);
                    }
                    Value::Class(class_rc) => {
                        self.check_access(&class_rc, &attr_name)?;

                        // 1. Check Static Properties
                        if let Some(prop) = class_rc.static_properties.get(&attr_name) {
                            if let Some(setter) = &prop.setter {
                                self.push(setter.clone());
                                self.push(Value::Class(class_rc.clone())); // arg 0: this (Class)
                                self.push(val.clone());                    // arg 1: value
                                self.call_value(setter.clone(), 2, Some(class_rc.clone()))?;
                                return Ok(true);
                            } else {
                                return Err(format!("Static Property '{}' is read-only", attr_name));
                            }
                        }

                        // 2. Static Fields
                        class_rc.static_fields.borrow_mut().insert(attr_name, val.clone());
                        self.push(val);
                    }
                    Value::Dict(d) => {
                        d.borrow_mut().insert(attr_name, val.clone());
                        self.push(val);
                    }
                    Value::Enum(_) => {
                        return Err("Cannot modify an Enum member (Enums are immutable)".into());
                    },
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
                let template_val = self.current_frame().chunk().constants[idx as usize].clone();
                
                if let Value::Class(template_data) = template_val {
                    // ---------------------------------------------------------
                    // 1. RESOLUTION DU PARENT
                    // ---------------------------------------------------------
                    let mut final_parent_ref = None;
                    if let Some(parent_name) = &template_data.parent {
                        if let Some(parent_val) = self.get_global_by_name(parent_name) {
                            if let Value::Class(parent_rc) = parent_val {
                                final_parent_ref = Some(parent_rc.clone());
                            } else {
                                return Err(format!("Parent '{}' is not a class", parent_name));
                            }
                        } else {
                            return Err(format!("Parent class '{}' not found", parent_name));
                        }
                    }

                    // Check Final Class (Parent)
                    if let Some(parent_rc) = &final_parent_ref {
                        if parent_rc.is_final {
                            return Err(format!("Erreur: La classe '{}' ne peut pas hériter de '{}' car elle est marquée 'final'.", template_data.name, parent_rc.name));
                        }
                    }

                    // ---------------------------------------------------------
                    // 2. RESOLUTION DES INTERFACES (AVANT de créer la classe)
                    // ---------------------------------------------------------
                    let mut resolved_interfaces = Vec::new();
                    for iface_name in &template_data.interfaces_names {
                        if let Some(val) = self.get_global_by_name(iface_name) {
                            if let Value::Interface(iface_rc) = val {
                                resolved_interfaces.push(iface_rc.clone());
                            } else {
                                return Err(format!("'{}' is not an interface", iface_name));
                            }
                        } else {
                            return Err(format!("Interface '{}' not found", iface_name));
                        }
                    }

                    // ---------------------------------------------------------
                    // 3. CREATION DE LA CLASSE (MAINTENANT !)
                    // ---------------------------------------------------------
                    let final_class_rc = Rc::new(ClassData {
                        name: template_data.name.clone(),
                        parent: template_data.parent.clone(),
                        parent_ref: final_parent_ref.clone(),
                        methods: template_data.methods.clone(),
                        visibilities: template_data.visibilities.clone(),
                        fields: template_data.fields.clone(),
                        field_types: template_data.field_types.clone(),
                        properties: template_data.properties.clone(),
                        
                        static_methods: template_data.static_methods.clone(),
                        static_fields: RefCell::new(HashMap::new()),
                        static_field_types: template_data.static_field_types.clone(),
                        static_properties: template_data.static_properties.clone(),

                        is_final: template_data.is_final,
                        final_methods: template_data.final_methods.clone(),

                        // On injecte les interfaces résolues
                        interfaces: resolved_interfaces.clone(),
                        interfaces_names: template_data.interfaces_names.clone(),
                    });

                    // ---------------------------------------------------------
                    // 4. VERIFICATIONS DE CONFORMITÉ
                    // ---------------------------------------------------------

                    // A. Vérification des Méthodes Finales (Surcharge interdite)
                    if let Some(parent_rc) = &final_parent_ref {
                        let check_override = |methods_map: &HashMap<String, Value>| -> Result<(), String> {
                            for method_name in methods_map.keys() {
                                let mut curr = Some(parent_rc.clone());
                                while let Some(p) = curr {
                                    if p.final_methods.contains(method_name) {
                                        return Err(format!("Erreur: Impossible de surcharger la méthode finale '{}' de la classe '{}'.", method_name, p.name));
                                    }
                                    curr = p.parent_ref.clone();
                                }
                            }
                            Ok(())
                        };
                        check_override(&template_data.methods)?;
                        check_override(&template_data.static_methods)?;
                    }

                    // B. Vérification des Interfaces (Contrat)
                    // Maintenant que final_class_rc existe, on peut utiliser find_method dessus !
                    for iface_rc in &resolved_interfaces {
                        for (method_name, expected_arity) in &iface_rc.methods {
                            let found_method = self.find_method(&final_class_rc, method_name);
                            
                            if let Some(m_val) = found_method {
                                if let Value::Function(f) = m_val {
                                    // Arity check (params.len() inclut 'this', donc -1)
                                    let actual_arity = if f.params.len() > 0 { f.params.len() - 1 } else { 0 };
                                    
                                    if actual_arity != *expected_arity {
                                        return Err(format!(
                                            "Class '{}' implements interface '{}' incorrectly: Method '{}' expects {} arguments, got {}.",
                                            template_data.name, iface_rc.name, method_name, expected_arity, actual_arity
                                        ));
                                    }
                                }
                            } else {
                                return Err(format!(
                                    "Class '{}' must implement method '{}' from interface '{}'.",
                                    template_data.name, method_name, iface_rc.name
                                ));
                            }
                        }
                    }

                    // ---------------------------------------------------------
                    // 5. INITIALISATION STATIQUE
                    // ---------------------------------------------------------
                    let static_inits = template_data.static_fields.borrow().clone();
                    for (name, init_val_or_func) in static_inits {
                        let final_val = if let Value::Function(func) = init_val_or_func {
                            self.run_callable_sync(
                                Value::Function(func), 
                                vec![], 
                                Some(final_class_rc.clone())
                            )?
                        } else {
                            init_val_or_func 
                        };
                        final_class_rc.static_fields.borrow_mut().insert(name, final_val);
                    }

                    self.push(Value::Class(final_class_rc));
                }
            },

            OpCode::MakeEnum => {
                let count = self.read_byte() as usize; // Nombre total d'éléments sur la pile (clés + valeurs)
                let num_pairs = count / 2;
                let mut map = HashMap::new();

                // On dépile dans l'ordre inverse
                for _ in 0..num_pairs {
                    let val = self.pop();
                    let key_val = self.pop();
                    let key = key_val.as_str().unwrap_or("?".to_string());
                    map.insert(key, val);
                }

                // On crée un Value::Enum SANS RefCell
                self.push(Value::Enum(Rc::new(map)));
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
                    let module_result = self.run_callable_sync(module_func, vec![], None)?;

                    // 6. UPDATE CACHE
                    self.modules.insert(path.clone(), Value::Boolean(true));
                    
                    // 7. RETURN
                    self.push(module_result);
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
                    
                    // 1. DÉBALLAGE IMMÉDIAT
                    // On convertit Value::Class -> Rc<ClassData> tout de suite
                    let mut current_class_rc = match parent_class_val {
                        Value::Class(c) => c,
                        _ => return Err(format!("'{}' n'est pas une classe", parent_name)),
                    };

                    loop {
                        // current_class_rc est maintenant bien un Rc<ClassData>
                        // On a donc accès à .methods et .parent_ref
                        if let Some(method_val) = current_class_rc.methods.get(&method_name) {
                            self.check_access(&current_class_rc, &method_name)?;
                            self.stack[obj_idx] = method_val.clone();
                            self.stack.insert(obj_idx + 1, obj.clone());
                            self.call_value(method_val.clone(), arg_count + 1, Some(current_class_rc.clone()))?;
                            return Ok(true);
                        }

                        // Remontée via référence forte (Type correct !)
                        if let Some(p) = &current_class_rc.parent_ref {
                            current_class_rc = p.clone(); // Rc<ClassData> -> Rc<ClassData>
                            continue;
                        }

                        return Err(format!("Méthode '{}' introuvable dans super", method_name));
                    }
                } else {
                    return Err(format!("Classe parente '{}' introuvable", parent_name));
                }
            },
            OpCode::MakeRange => {
                let end_val = self.pop();
                let start_val = self.pop();
                
                let start = start_val.as_int().unwrap_or(0);
                let end = end_val.as_int().unwrap_or(0);
                
                // Par défaut, le step est 1
                self.push(Value::Range(start, end, 1));
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
            // inst.borrow().class est maintenant directement Rc<ClassData>
            // Plus besoin de déballer un Value::Class !
            let mut current_class_rc = inst.borrow().class.clone();
            
            loop {
                // A. Méthode présente ?
                if let Some(method_val) = current_class_rc.methods.get(&method_name) {
                    self.check_access(&current_class_rc, &method_name)?;
                    self.stack[obj_idx] = method_val.clone();
                    self.stack.insert(obj_idx + 1, obj.clone()); 
                    self.call_value(
                        method_val.clone(), 
                        arg_count + 1, 
                        Some(current_class_rc.clone())
                    )?; 
                    return Ok(()); 
                }
                
                // B. Parent ? (Via référence forte)
                if let Some(parent_rc) = &current_class_rc.parent_ref {
                    current_class_rc = parent_rc.clone();
                    continue;
                }
                
                break; // Non trouvé
            }
        }

        if let Value::Class(class_rc) = &obj {
            // Search in static methods of the class itself
            // Note: Inheritance of static methods is possible in some languages.
            // For now, let's look in the class itself (simplification).
            // To support static inheritance: Loop on parent_ref like in Instance.
            
            let mut current_lookup = class_rc.clone();
            loop {
                if let Some(method_val) = current_lookup.static_methods.get(&method_name) {
                    // A. Security Check
                    self.check_access(&current_lookup, &method_name)?;

                    // B. Setup Stack
                    self.stack[obj_idx] = method_val.clone();
                    // We reinject the Class Object as 'this' (argument 0)
                    self.stack.insert(obj_idx + 1, obj.clone()); 

                    // C. Call with Context
                    self.call_value(
                        method_val.clone(), 
                        arg_count + 1, 
                        Some(current_lookup.clone())
                    )?;
                    return Ok(());
                }
                
                // Static Inheritance (Optional but nice)
                if let Some(parent) = &current_lookup.parent_ref {
                    current_lookup = parent.clone();
                    continue;
                }
                break;
            }
            return Err(format!("Static method '{}' not found on class '{}'", method_name, class_rc.name).into());
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
                    
                    self.call_value(val, arg_count, None)?;
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

                "is_empty" => Value::Boolean(l.borrow().is_empty()),

                "first" => l.borrow().first().cloned().unwrap_or(Value::Null),

                "last" => l.borrow().last().cloned().unwrap_or(Value::Null),

                "clear" => {
                    l.borrow_mut().clear();
                    Value::Null
                },

                "reduce" => {
                    // Usage: list.reduce(func(acc, val), initial_value)
                    if args.len() < 2 { return Err("reduce expects (callback, initial)".into()); }
                    
                    let callback = args[0].clone();
                    let mut accumulator = args[1].clone();
                    let list_data = l.borrow().clone();

                    for item in list_data {
                        // Le callback prend (acc, item) et retourne le nouvel acc
                        accumulator = self.run_callable_sync(callback.clone(), vec![accumulator, item], None)?;
                    }
                    
                    accumulator
                },

                "index_of" => {
                    // Usage: list.index_of(value) -> int (ou -1)
                    if args.len() < 1 { return Err("index_of attend 1 argument".into()); }
                    let target = &args[0];
                    
                    let list = l.borrow();
                    let index = list.iter().position(|x| x == target); // PartialEq fait le travail
                    
                    match index {
                        Some(i) => Value::Integer(i as i64),
                        None => Value::Integer(-1),
                    }
                },

                "find" => {
                    if args.len() < 1 { return Err("find attend 1 callback".into()); }
                    let callback = args[0].clone();
                    
                    let list_data = l.borrow().clone();
                    
                    // 1. Variable pour stocker le résultat (par défaut Null)
                    let mut found_item = Value::Null;
                    
                    for item in list_data {
                        let res = self.run_callable_sync(callback.clone(), vec![item.clone()], None)?;
                        
                        let is_found = match res {
                            Value::Boolean(b) => b,
                            Value::Null => false,
                            Value::Integer(i) => i != 0,
                            _ => true,
                        };
                        
                        if is_found {
                            found_item = item; // On stocke
                            break;             // On arrête la recherche
                        }
                    }
                    
                    // 2. On retourne la valeur trouvée (ou Null) comme résultat du match
                    found_item 
                },

                "sort" => {
                    // 1. On clone le vecteur pour pouvoir le trier sans violer les règles d'emprunt (RefCell)
                    // Cela permet aussi au callback de comparaison de lire la liste si nécessaire sans crash.
                    let mut data = l.borrow().clone();
                    
                    // 2. Récupération du comparateur optionnel (fonction de callback)
                    let comparator = if args.len() > 0 { Some(args[0].clone()) } else { None };
                    
                    // 3. Logique de Tri
                    if let Some(comp_fn) = comparator {
                        // --- CAS A : TRI PERSONNALISÉ ---
                        // On utilise une variable pour capturer une erreur éventuelle survenue dans le callback Aegis
                        let mut sort_error = None;
                        
                        data.sort_by(|a, b| {
                            // Si une erreur a déjà eu lieu, on arrête de calculer (on renvoie Equal)
                            if sort_error.is_some() { return std::cmp::Ordering::Equal; }
                            
                            // Appel Synchrone de la VM : on exécute la fonction Aegis
                            match self.run_callable_sync(comp_fn.clone(), vec![a.clone(), b.clone()], None) {
                                Ok(res) => {
                                    // La convention standard : négatif = Less, positif = Greater, 0 = Equal
                                    // On essaie de convertir en entier, sinon en float
                                    let n = if let Ok(i) = res.as_int() { i as f64 } 
                                            else { res.as_float().unwrap_or(0.0) };

                                    if n < 0.0 { std::cmp::Ordering::Less }
                                    else if n > 0.0 { std::cmp::Ordering::Greater }
                                    else { std::cmp::Ordering::Equal }
                                },
                                Err(e) => {
                                    // On capture l'erreur pour la remonter après le sort
                                    sort_error = Some(e);
                                    std::cmp::Ordering::Equal
                                }
                            }
                        });
                        
                        // Si le tri a échoué à cause d'une erreur script, on la propage
                        if let Some(e) = sort_error { return Err(e); }
                        
                    } else {
                        // --- CAS B : TRI PAR DÉFAUT ---
                        // Rust ne sait pas trier nativement nos Values sans implémenter Ord.
                        // On implémente une logique "best effort".
                        data.sort_by(|a, b| {
                             match (a, b) {
                                 // Comparaison d'entiers
                                 (Value::Integer(i1), Value::Integer(i2)) => i1.cmp(i2),
                                 // Comparaison de floats (partial_cmp peut renvoyer None pour NaN, on gère)
                                 (Value::Float(f1), Value::Float(f2)) => f1.partial_cmp(f2).unwrap_or(std::cmp::Ordering::Equal),
                                 // Mixte Int/Float
                                 (Value::Integer(i), Value::Float(f)) => (*i as f64).partial_cmp(f).unwrap_or(std::cmp::Ordering::Equal),
                                 (Value::Float(f), Value::Integer(i)) => f.partial_cmp(&(*i as f64)).unwrap_or(std::cmp::Ordering::Equal),
                                 // Chaînes de caractères
                                 (Value::String(s1), Value::String(s2)) => s1.cmp(s2),
                                 // Fallback : Comparaison via représentation string (ex: "true" > "false")
                                 (v1, v2) => v1.to_string().cmp(&v2.to_string())
                             }
                        });
                    }
                    
                    // 4. On remplace le contenu de la liste originale par la version triée
                    *l.borrow_mut() = data;
                    
                    Value::Null
                },

                // --- Utility ---

                "slice" => {
                    // Usage: list.slice(start, end_exclusive)
                    let len = l.borrow().len();
                    let start = args.get(0).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
                    let end = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(len as i64) as usize;

                    // Clamping pour éviter les crashs
                    let start = start.min(len);
                    let end = end.min(len).max(start);

                    let list_borrow = l.borrow();
                    // On crée une nouvelle liste avec la tranche
                    let new_vec = list_borrow[start..end].to_vec();
                    
                    Value::List(Rc::new(RefCell::new(new_vec)))
                },
                
                // --- FUNCTIONAL PROGRAMMING ---
                
                "map" => {
                    let callback = args[0].clone();
                    let list_data = l.borrow().clone(); // Clone to avoid RefCell borrow conflict during callback
                    let mut new_list = Vec::new();

                    for item in list_data {
                        // On appelle la VM récursivement pour chaque élément !
                        let res = self.run_callable_sync(callback.clone(), vec![item], None)?;
                        new_list.push(res);
                    }
                    Value::List(Rc::new(RefCell::new(new_list)))
                },

                "filter" => {
                    let callback = args[0].clone();
                    let list_data = l.borrow().clone();
                    let mut new_list = Vec::new();

                    for item in list_data {
                        let res = self.run_callable_sync(callback.clone(), vec![item.clone()], None)?;
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
                        self.run_callable_sync(callback.clone(), vec![item], None)?;
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

                "is_empty" => Value::Boolean(d.borrow().is_empty()),

                "remove" => {
                    let key = args[0].as_str().unwrap_or_default();
                    // Retourne la valeur supprimée ou Null
                    d.borrow_mut().remove(&key).unwrap_or(Value::Null)
                },

                "values" => {
                    // Retourne une liste des valeurs
                    let vals: Vec<Value> = d.borrow().values().cloned().collect();
                    Value::List(Rc::new(RefCell::new(vals)))
                },
                _ => return Err(format!("Unknown dict method '{}'", method_name).into())
            },

            Value::Range(start, end, step) => match method_name.as_str() {
                // Pour que foreach sache combien de tours faire
                "len" => {
                    if step == 0 { return Err("Step cannot be zero".into()); }
                    
                    // Calcul mathématique du nombre d'éléments
                    let diff = end - start;
                    let count = if (diff > 0 && step > 0) || (diff < 0 && step < 0) {
                        (diff as f64 / step as f64).ceil() as i64
                    } else {
                        0
                    };
                    Value::Integer(count.max(0))
                },
                
                // Pour que foreach récupère l'élément courant
                "at" => {
                    let idx = args[0].as_int().unwrap_or(0);
                    let val = start + (idx * step);
                    Value::Integer(val)
                },
                
                // Méthode fluide pour changer le pas : (0..10).step(2)
                "step" => {
                    let new_step = args[0].as_int().unwrap_or(1);
                    if new_step == 0 { return Err("Step cannot be 0".into()); }
                    Value::Range(start, end, new_step)
                },
                
                // Bonus : Conversion en liste réelle
                "to_list" => {
                    let mut list = Vec::new();
                    let mut current = start;
                    // Logique simplifiée (attention aux boucles infinies si step est mauvais)
                    if step > 0 {
                        while current < end {
                            list.push(Value::Integer(current));
                            current += step;
                        }
                    } else {
                        while current > end {
                            list.push(Value::Integer(current));
                            current += step;
                        }
                    }
                    Value::List(Rc::new(RefCell::new(list)))
                },
                
                _ => return Err(format!("Unknown range method '{}'", method_name).into())
            },

            Value::String(s) => match method_name.as_str() {
                "len" => Value::Integer(s.chars().count() as i64),
                "at" => {
                    // Récupération de l'index
                    let idx = args[0].as_int().unwrap_or(0);
                    
                    if idx < 0 {
                        Value::Null
                    } else {
                        // On utilise chars().nth() pour gérer correctement l'UTF-8 (accents, emojis)
                        match s.chars().nth(idx as usize) {
                            Some(c) => Value::String(c.to_string()),
                            None => Value::Null,
                        }
                    }
                },
                "index_of" => {
                    // Récupère la sous-chaîne à chercher
                    let sub = args[0].as_str().unwrap_or_default();
                    
                    // s.find retourne un Option<usize> (l'index en octets)
                    match s.find(&sub) {
                        Some(idx) => Value::Integer(idx as i64),
                        None => Value::Integer(-1), // Retourne -1 si non trouvé
                    }
                }

                "slice" => {
                    // Usage: string.slice(start, end)
                    let len = s.chars().count();
                    let start = args.get(0).and_then(|v| v.as_int().ok()).unwrap_or(0) as usize;
                    // Par défaut jusqu'à la fin
                    let end = args.get(1).and_then(|v| v.as_int().ok()).unwrap_or(len as i64) as usize;

                    // Clamping
                    let start = start.min(len);
                    let end = end.min(len).max(start);

                    // Attention : le slicing en Rust se fait sur les octets, mais ici on veut logique "caractères"
                    // Pour supporter l'UTF-8 correctement, on itère sur chars()
                    let sub: String = s.chars()
                        .skip(start)
                        .take(end - start)
                        .collect();
                    
                    Value::String(sub)
                },
                
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

                "is_empty" => Value::Boolean(s.is_empty()),

                "pad_start" => {
                    // Args: width, char (optionnel, defaut ' ')
                    let width = args[0].as_int().unwrap_or(0) as usize;
                    let pad_char = if args.len() > 1 { 
                        args[1].as_str().unwrap_or(" ".to_string()).chars().next().unwrap_or(' ') 
                    } else { ' ' };

                    if s.len() >= width {
                        Value::String(s.clone())
                    } else {
                        let padding = pad_char.to_string().repeat(width - s.len());
                        Value::String(format!("{}{}", padding, s))
                    }
                },

                "pad_end" => {
                    let width = args[0].as_int().unwrap_or(0) as usize;
                    let pad_char = if args.len() > 1 { 
                        args[1].as_str().unwrap_or(" ".to_string()).chars().next().unwrap_or(' ') 
                    } else { ' ' };

                    if s.len() >= width {
                        Value::String(s.clone())
                    } else {
                        let padding = pad_char.to_string().repeat(width - s.len());
                        Value::String(format!("{}{}", s, padding))
                    }
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
        // VERSION SAFE : On vérifie les bornes
        if frame.ip >= frame.chunk().code.len() {
            panic!("VM Error: Instruction Pointer out of bounds!");
        }
        let b = frame.chunk().code[frame.ip];
        frame.ip += 1;
        b
    }

    fn read_short(&mut self) -> u16 {
        let frame = self.current_frame();
        let ip = frame.ip;
        frame.ip += 2;
        ((frame.chunk().code[ip] as u16) << 8) | frame.chunk().code[ip + 1] as u16
    }

    fn call_value(&mut self, target: Value, arg_count: usize, context: Option<Rc<ClassData>>) -> Result<(), String> {
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
                    class_context: context
                 };
                 
                 self.frames.push(frame);
                 Ok(())
            },

            // CAS 2 : Classe
            Value::Class(rc_class) => {
                // 1. Création de l'instance vide (avec le bon type Rc<ClassData>)
                let instance_rc = Rc::new(RefCell::new(InstanceData {
                    class: rc_class.clone(),
                    fields: HashMap::new()
                }));

                // 2. On crée la Value pour la VM
                let instance = Value::Instance(instance_rc.clone());

                // --- INITIALISATION DES CHAMPS ---
                // On doit remonter toute la chaîne de prototypes (parents d'abord)
                // pour initialiser les champs dans le bon ordre (optionnel, mais propre).
                // Ici, on fait simple : on initialise les champs de la classe courante.
                // Note : Si tu veux supporter les champs hérités, il faut iterer sur les parents.
                
                // On collecte la chaîne d'héritage (du plus lointain parent à l'enfant)
                let mut hierarchy = Vec::new();
                let mut curr = Some(rc_class.clone());
                while let Some(c) = curr {
                    hierarchy.push(c.clone());
                    curr = c.parent_ref.clone();
                }
                hierarchy.reverse(); // On commence par le Grand-Père

                // On exécute les initialiseurs
                for class_def in hierarchy {
                    for (field_name, init_val_or_func) in &class_def.fields {
                        // Si l'initialiseur est une fonction (cas compilé par nous), on l'exécute
                        if let Value::Function(init_func) = init_val_or_func {
                            // Appel synchrone de la fonction d'initialisation (sans arguments)
                            // Elle retourne la valeur par défaut (ex: 100)
                            match self.run_callable_sync(
                                Value::Function(init_func.clone()), 
                                vec![], 
                                Some(class_def.clone())
                            ) {
                                Ok(val) => {
                                    // On insère dans l'instance
                                    instance_rc.borrow_mut().fields.insert(field_name.clone(), val);
                                },
                                Err(e) => return Err(format!("Erreur initialisation champ '{}': {}", field_name, e)),
                            }
                        } else {
                            // Cas théorique (si on stockait des constantes brutes)
                            instance_rc.borrow_mut().fields.insert(field_name.clone(), init_val_or_func.clone());
                        }
                    }
                }
                // -------------------------------------------

                // 2. Recherche du constructeur "init" (Logique existante)
                let mut init_method = None;
                let mut current_class_lookup = rc_class.clone();

                loop {
                    if let Some(m) = current_class_lookup.methods.get("init") {
                        init_method = Some(m.clone());
                        break;
                    }
                    if let Some(parent_rc) = &current_class_lookup.parent_ref {
                        current_class_lookup = parent_rc.clone();
                        continue;
                    }
                    break;
                }

                // 3. Appel du constructeur
                if let Some(method_val) = init_method {
                    let args_start = func_idx + 1;
                    let args: Vec<Value> = self.stack.drain(args_start..).collect();
                    
                    let mut call_args = vec![instance.clone()];
                    call_args.extend(args);

                    self.run_callable_sync(method_val, call_args, Some(rc_class.clone()))?;
                } else {
                    if arg_count > 0 {
                        return Err(format!("Classe '{}' n'a pas de constructeur 'init'", rc_class.name));
                    }
                    self.stack.truncate(func_idx + 1);
                }
                
                self.stack[func_idx] = instance;
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
            class_context: None,
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

    fn check_access(&mut self, target_class: &Rc<ClassData>, member_name: &str) -> Result<(), String> {
        // 1. Récupérer la visibilité (Public par défaut)
        let visibility = target_class.visibilities.get(member_name).unwrap_or(&Visibility::Public);

        if matches!(visibility, Visibility::Public) {
            return Ok(());
        }

        // 2. Qui appelle ?
        let current_context = &self.current_frame().class_context;

        // Si on n'est pas dans une classe, on n'a accès qu'au public
        let ctx = match current_context {
            Some(c) => c,
            None => return Err(format!("Accès refusé : '{}' est {:?} (Appel hors classe)", member_name, visibility)),
        };

        match visibility {
            Visibility::Public => Ok(()), // Déjà géré mais au cas où
            
            Visibility::Private => {
                // Règle : Seule la classe elle-même a accès (Identity Check)
                if Rc::ptr_eq(ctx, target_class) {
                    Ok(())
                } else {
                    Err(format!("Accès refusé : '{}' est privé à la classe '{}'", member_name, target_class.name))
                }
            },
            
            Visibility::Protected => {
                // Règle : La classe elle-même OU ses enfants ont accès.
                
                // A. Même classe ?
                if Rc::ptr_eq(ctx, target_class) { return Ok(()); }

                // B. Est-ce que 'ctx' (l'appelant) hérite de 'target_class' (le propriétaire) ?
                // On remonte la chaîne des parents de l'appelant
                let mut curr = Some(ctx.clone());
                while let Some(c) = curr {
                    if Rc::ptr_eq(&c, target_class) {
                        return Ok(());
                    }
                    curr = c.parent_ref.clone();
                }
                
                Err(format!("Accès refusé : '{}' est protégé dans '{}'", member_name, target_class.name))
            }
        }
    }

    fn find_method(&self, class: &Rc<ClassData>, name: &str) -> Option<Value> {
        // 1. Chercher dans la classe courante
        if let Some(m) = class.methods.get(name) {
            return Some(m.clone());
        }
        
        // 2. Remonter au parent
        if let Some(parent) = &class.parent_ref {
            return self.find_method(parent, name);
        }
        
        None
    }
}
