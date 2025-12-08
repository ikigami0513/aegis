pub mod compiler;
pub mod debug;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{InstanceData, Value};
use crate::chunk::Chunk;
use crate::opcode::OpCode;
use crate::ast::environment::Environment;

const STACK_MAX: usize = 256;

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
            Value::Function(_, _, chunk, _) => chunk,
            // Pour le script principal (ou classes), on devra gérer le cas
            // Astuce: On peut wrapper le main script dans une Value::Function fictive
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
    handlers: Vec<ExceptionHandler>,
}

impl VM {
    pub fn new(main_chunk: Chunk) -> Self {
        let main_func = Value::Function(vec![], None, main_chunk, None);

        // Le script principal est la première "fonction" exécutée
        let main_frame = CallFrame {
            closure: main_func, // Utilise la closure
            ip: 0,
            slot_offset: 0,
        };

        let mut vm = VM {
            frames: vec![main_frame],
            stack: Vec::with_capacity(STACK_MAX),
            // On prépare de la place (256 slots globaux)
            globals: vec![Value::Null; 256],
            handlers: Vec::new(),
        };

        let natives = crate::native::get_all_names();

        // Sécurité : On ne peut pas avoir plus de 256 globales avec des ID sur u8
        if natives.len() > 256 {
            panic!("Trop de fonctions natives pour la VM v2 (>256)");
        }

        for (i, name) in natives.into_iter().enumerate() {
            vm.globals[i] = Value::Native(name);
        }

        vm
    }

    // Helper pour récupérer la frame courante sans se battre avec le borrow checker
    fn current_frame(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("No code to execute")
    }

    // Helper pour la pile
    fn push(&mut self, value: Value) {
        self.stack.push(value);
    }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }

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
            if !self.step()? {
                break;
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

                // On clone la cible pour pouvoir la passer à call_value
                let target = self.stack[func_idx].clone();

                self.call_value(target, arg_count)?;
            }
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
                let b = self.pop();
                let a = self.pop();

                match (a, b) {
                    // Entier + Entier
                    (Value::Integer(v1), Value::Integer(v2)) => self.push(Value::Integer(v1 + v2)),
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
            OpCode::Sub => {
                let b = self.pop();
                let a = self.pop();
                match (a, b) {
                    (Value::Integer(v1), Value::Integer(v2)) => self.push(Value::Integer(v1 - v2)),
                    (Value::Float(v1), Value::Float(v2)) => self.push(Value::Float(v1 - v2)),
                    (Value::Integer(v1), Value::Float(v2)) => self.push(Value::Float(v1 as f64 - v2)),
                    (Value::Float(v1), Value::Integer(v2)) => self.push(Value::Float(v1 - v2 as f64)),
                    _ => return Err("Type error in SUB".into())
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
                let val = self.globals[idx].clone();
                self.push(val);
            }
            OpCode::GetLocal => {
                let slot_idx = self.read_byte() as usize;
                // On calcule la position absolue dans la pile
                let abs_index = self.current_frame().slot_offset + slot_idx;

                let val = self.stack[abs_index].clone();
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
            OpCode::Less => {
                let b = self.pop();
                let a = self.pop();
                if let (Value::Integer(v1), Value::Integer(v2)) = (a, b) {
                    self.push(Value::Boolean(v1 < v2));
                } else {
                    self.push(Value::Boolean(false));
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
                
                if let Value::Function(params, ret, chunk, _) = function_val {
                    let env_rc = Environment::new_global();
                    
                    // --- DEBUT DE LA PHASE D'EXTRACTION (SCOPE LIMITE) ---
                    // On récupère les infos nécessaires et on CLONE les noms pour libérer la frame ensuite
                    let (parent_params, slot_offset) = {
                        let frame = self.current_frame();
                        if let Value::Function(pp, _, _, _) = &frame.closure {
                            (Some(pp.clone()), frame.slot_offset)
                        } else {
                            (None, 0)
                        }
                    }; 
                    // --- FIN DE L'EMPRUNT (frame est drop ici) ---

                    // Maintenant self est libre, on peut accéder à self.stack
                    if let Some(parent_params) = parent_params {
                        let mut env_inner = env_rc.borrow_mut();
                        for (i, (name, _)) in parent_params.iter().enumerate() {
                            let val = self.stack[slot_offset + i].clone();
                            env_inner.variables.insert(name.clone(), val);
                        }
                    }
                    
                    let closure = Value::Function(
                        params, 
                        ret, 
                        chunk, 
                        Some(env_rc)
                    );
                    self.push(closure);
                } else {
                    panic!("MakeClosure sur non-fonction");
                }
            },

            OpCode::GetFreeVar => {
                let name_idx = self.read_byte();
                // Note: read_byte utilise current_frame() en interne, mais l'emprunt se termine 
                // dès que la fonction retourne le byte. C'est safe.
                
                // On doit ré-emprunter pour lire la constante string.
                // Pour éviter de garder la frame, on clone la string tout de suite.
                let name = {
                    let frame = self.current_frame();
                    frame.chunk().constants[name_idx as usize].to_string()
                };
                
                let mut val_to_push = None;

                // --- PHASE DE LECTURE ---
                {
                    let frame = self.current_frame();
                    if let Value::Function(_, _, _, Some(env)) = &frame.closure {
                        // On récupère la valeur et on la clone immédiatement
                        // Cela permet de relâcher le borrow sur 'env' et 'frame' juste après
                        if let Some(val) = env.borrow().variables.get(&name) {
                            val_to_push = Some(val.clone());
                        }
                    }
                }
                // --- FIN DE L'EMPRUNT ---

                // --- PHASE D'ECRITURE ---
                if let Some(val) = val_to_push {
                    self.push(val);
                } else {
                    // Fallback global ou null
                    return Err(format!("Variable de closure introuvable : '{}'", name));
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

        // 1. Instance Methods (POO) - Inchangé
        if let Value::Instance(inst) = &obj {
             let class_val = inst.borrow().class.clone();
             if let Value::Class { methods, .. } = &*class_val {
                 if let Some(method_val) = methods.get(&method_name) {
                     self.stack[obj_idx] = method_val.clone();
                     self.stack.insert(obj_idx + 1, obj.clone()); 
                     self.call_value(method_val.clone(), arg_count + 1)?; 
                     return Ok(()); // On laisse la boucle principale continuer
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
                
                "trim" => {
                    // Rust fait ça très bien nativement
                    Value::String(s.trim().to_string())
                },

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
    fn read_byte(&mut self) -> u8 {
        let frame = self.current_frame();
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

    fn call_value(&mut self, target: Value, arg_count: usize) -> Result<(), String> {
        let func_idx = self.stack.len() - 1 - arg_count;

        match &target {
            // CAS 1 : Fonction Aegis (Lambda ou Nommée)
            Value::Function(params, _, _, _closure_env) => {
                if arg_count != params.len() {
                    return Err(format!(
                        "Arity mismatch: attendu {}, reçu {}",
                        params.len(),
                        arg_count
                    ));
                }

                // Le parent du scope est soit la closure capturée, soit le scope global actuel (si on veut, mais ici on isole)
                // Pour la VM, on n'a pas besoin de créer l'environnement tout de suite,
                // c'est l'OpCode::GetLocal qui fera le lien avec la pile.
                // MAIS pour les Globales/Closures, on garde l'env.

                let frame = CallFrame {
                    closure: target.clone(),
                    ip: 0,
                    slot_offset: func_idx + 1, // Les locales commencent après la fonction
                };

                self.frames.push(frame);
                Ok(())
            }

            // CAS 2 : Instanciation de Classe
            Value::Class {
                name,
                params,
                methods: _,
            } => {
                if arg_count != params.len() {
                    return Err(format!(
                        "Constructeur {}: attendu {} args, reçu {}",
                        name,
                        params.len(),
                        arg_count
                    ));
                }

                // On clone la définition pour l'instance
                // (On utilise target.clone() ici, mais comme target est déplacé dans le match,
                // on doit le cloner avant ou le reconstruire.
                // Astuce : On recrée une Value::Class à partir des refs qu'on a, ou on clone target au début)

                // Pour simplifier (et éviter les soucis de borrow), on réutilise target
                // mais il faut ruser car 'ref name' l'emprunte.
                // Le plus simple : cloner 'target' AVANT le match dans l'appelant, ou ici :

                // On va reconstruire l'objet Value::Class pour le mettre dans le Rc
                // (C'est un peu coûteux mais sûr pour le borrow checker)
                let class_val_rc = Rc::new(target.clone());

                let mut fields = HashMap::new();
                for (i, (param_name, _)) in params.iter().enumerate() {
                    let arg_val = self.stack[func_idx + 1 + i].clone();
                    fields.insert(param_name.clone(), arg_val);
                }

                let instance = Value::Instance(Rc::new(RefCell::new(InstanceData {
                    class: class_val_rc,
                    fields,
                })));

                self.stack[func_idx] = instance;
                self.stack.truncate(func_idx + 1);
                Ok(())
            }

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
}
