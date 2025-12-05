use crate::ast::{Instruction, Expression, Value};
use crate::compiler;
use crate::environment::Environment;
use crate::loader::parse_block;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

// Type alias pour simplifier les signatures
type SharedEnv = Rc<RefCell<Environment>>;

/// Helper pour déterminer si une valeur est "Vraie" (Python-like)
fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Boolean(b) => *b,
        Value::Null => false,
        Value::Integer(i) => *i != 0,
        Value::Float(f) => *f != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::List(l) => !l.borrow().is_empty(),
        Value::Dict(d) => !d.borrow().is_empty(),
        Value::Instance(_) => true,
    }
}

/// Évalue une expression pour obtenir une valeur
pub fn evaluate(expr: &Expression, env: SharedEnv) -> Result<Value, String> {
    match expr {
        Expression::Literal(v) => Ok(v.clone()),
        Expression::Variable(name) => env.borrow().get_variable(name).ok_or_else(|| format!("Variable non définie : {}", name)),

        // --- ARITHMÉTIQUE ---
        Expression::Add(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            match (l, r) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + b as f64)),
                (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b))),
                (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                _ => Err("Types incompatibles pour +".into()),
            }
        },
        Expression::Sub(left, right) => {
            match (evaluate(left, env.clone())?, evaluate(right, env.clone())?) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - b as f64)),
                _ => Err("Types incompatibles pour -".into()),
            }
        },
        Expression::Mul(left, right) => {
             match (evaluate(left, env.clone())?, evaluate(right, env.clone())?) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * b as f64)),
                _ => Err("Types incompatibles pour *".into()),
            }
        },
        Expression::Div(left, right) => {
             match (evaluate(left, env.clone())?, evaluate(right, env.clone())?) {
                (Value::Integer(a), Value::Integer(b)) => if b==0 {Err("Div / 0".into())} else {Ok(Value::Integer(a/b))},
                (Value::Float(a), Value::Float(b)) => if b==0.0 {Err("Div / 0".into())} else {Ok(Value::Float(a/b))},
                (Value::Integer(a), Value::Float(b)) => if b==0.0 {Err("Div / 0".into())} else {Ok(Value::Float(a as f64 / b))},
                (Value::Float(a), Value::Integer(b)) => if b==0 {Err("Div / 0".into())} else {Ok(Value::Float(a / b as f64))},
                _ => Err("Types incompatibles pour /".into()),
            }
        },

        Expression::Modulo(left, right) => {
             match (evaluate(left, env.clone())?, evaluate(right, env.clone())?) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if b == 0 { Err("Modulo by 0".into()) } else { Ok(Value::Integer(a % b)) }
                },
                (Value::Float(a), Value::Float(b)) => {
                     if b == 0.0 { Err("Modulo by 0.0".into()) } else { Ok(Value::Float(a % b)) }
                },
                // Rust allows float % int, so let's handle mixed types
                (Value::Integer(a), Value::Float(b)) => {
                     if b == 0.0 { Err("Modulo by 0.0".into()) } else { Ok(Value::Float((a as f64) % b)) }
                },
                (Value::Float(a), Value::Integer(b)) => {
                     if b == 0 { Err("Modulo by 0".into()) } else { Ok(Value::Float(a % (b as f64))) }
                },
                _ => Err("Types incompatibles pour %".into()),
            }
        },

        // --- LOGIQUE ---
        Expression::Not(expr) => {
            let val = evaluate(expr, env)?;
            Ok(Value::Boolean(!is_truthy(&val)))
        },
        Expression::And(left, right) => {
            let l_val = evaluate(left, env.clone())?;
            // Court-circuit : Si gauche est faux, on retourne false immédiatement
            if !is_truthy(&l_val) {
                Ok(Value::Boolean(false))
            } else {
                let r_val = evaluate(right, env)?;
                Ok(Value::Boolean(is_truthy(&r_val)))
            }
        },
        Expression::Or(left, right) => {
            let l_val = evaluate(left, env.clone())?;
            // Court-circuit : Si gauche est vrai, on retourne true immédiatement
            if is_truthy(&l_val) {
                Ok(Value::Boolean(true))
            } else {
                let r_val = evaluate(right, env)?;
                Ok(Value::Boolean(is_truthy(&r_val)))
            }
        },

        // --- COMPARAISONS (MISE À JOUR) ---
        Expression::Equal(left, right) => Ok(Value::Boolean(evaluate(left, env.clone())? == evaluate(right, env)?)),
        Expression::NotEqual(left, right) => Ok(Value::Boolean(evaluate(left, env.clone())? != evaluate(right, env)?)),
        
        Expression::LessThan(left, right) => compare_nums(evaluate(left, env.clone())?, evaluate(right, env)?, |a,b| a < b),
        Expression::GreaterThan(left, right) => compare_nums(evaluate(left, env.clone())?, evaluate(right, env)?, |a,b| a > b),
        Expression::LessEqual(left, right) => compare_nums(evaluate(left, env.clone())?, evaluate(right, env)?, |a,b| a <= b),
        Expression::GreaterEqual(left, right) => compare_nums(evaluate(left, env.clone())?, evaluate(right, env)?, |a,b| a >= b),

        // --- POO & STRUCTURES ---
        Expression::New(class_name, args) => {
            let cls = env.borrow().get_class(class_name).ok_or("Classe inconnue")?;
            let mut resolved = Vec::new();
            for a in args { resolved.push(evaluate(a, env.clone())?); }
            if resolved.len() != cls.params.len() { return Err("Constructeur arity mismatch".into()); }
            let mut fields = HashMap::new();
            for (p, v) in cls.params.iter().zip(resolved) { fields.insert(p.clone(), v); }
            Ok(Value::Instance(Rc::new(RefCell::new(crate::ast::InstanceData { class_name: class_name.clone(), fields }))))
        },
        Expression::GetAttr(obj, attr) => {
            if let Value::Instance(i) = evaluate(obj, env)? {
                i.borrow().fields.get(attr).cloned().ok_or("Attribut introuvable".into())
            } else { Err("Pas une instance".into()) }
        },
        Expression::List(exprs) => {
            let mut vals = Vec::new();
            for e in exprs { vals.push(evaluate(e, env.clone())?); }
            Ok(Value::List(Rc::new(RefCell::new(vals))))
        },
        Expression::Dict(entries) => {
            let mut d = HashMap::new();
            for (k, e) in entries { d.insert(k.clone(), evaluate(e, env.clone())?); }
            Ok(Value::Dict(Rc::new(RefCell::new(d))))
        },

        // Appels de méthodes (Copie optimisée de ton code précédent)
        Expression::CallMethod(obj, method, args) => {
            let obj_val = evaluate(obj, env.clone())?;
            let mut resolved = Vec::new();
            for a in args { resolved.push(evaluate(a, env.clone())?); }

            match &obj_val {
                Value::List(l) => match method.as_str() {
                    "push" => { l.borrow_mut().push(resolved[0].clone()); Ok(Value::Null) },
                    "pop" => Ok(l.borrow_mut().pop().unwrap_or(Value::Null)),
                    "len" => Ok(Value::Integer(l.borrow().len() as i64)),
                    "at" => {
                        // 1. On récupère l'index (c'est l'argument 0, et non 1)
                        if resolved.is_empty() { return Err("List.at() expects 1 argument".into()); }
                        let index = resolved[0].as_int()? as usize;
                        
                        // 2. On sécurise l'accès pour éviter le panic Rust
                        let list = l.borrow();
                        if index >= list.len() {
                            return Err(format!("Index out of bounds: {} (len: {})", index, list.len()));
                        }
                        
                        Ok(list[index].clone())
                    },
                    _ => Err("Method list unknown".into())
                },
                Value::Dict(d) => match method.as_str() {
                    "insert" => { d.borrow_mut().insert(resolved[0].clone().as_str()?, resolved[1].clone()); Ok(Value::Null) },
                    "remove" => Ok(d.borrow_mut().remove(&resolved[0].clone().as_str()?).unwrap_or(Value::Null)),
                    "keys" => Ok(Value::List(Rc::new(RefCell::new(d.borrow().keys().map(|k| Value::String(k.clone())).collect())))),
                    "len" => Ok(Value::Integer(d.borrow().len() as i64)),
                    _ => Err("Method dict unknown".into())
                },
                Value::String(s) => match method.as_str() {
                    "trim" => {
                        // Usage: "  hello  ".trim() -> "hello"
                        Ok(Value::String(s.trim().to_string()))
                    },
                    "split" => {
                        // Usage: "a,b,c".split(",") -> ["a", "b", "c"]
                        if resolved.len() != 1 { return Err("split expects 1 argument (separator)".into()); }
                        
                        let separator = match &resolved[0] {
                            Value::String(sep) => sep,
                            _ => return Err("Separator must be a string".into())
                        };

                        let parts: Vec<Value> = s.split(separator)
                            .map(|p| Value::String(p.to_string()))
                            .collect();
                        
                        Ok(Value::List(Rc::new(RefCell::new(parts))))
                    },
                    "replace" => {
                        // Usage: "hello world".replace("world", "Aegis")
                        if resolved.len() != 2 { return Err("replace expects 2 arguments (old, new)".into()); }
                        
                        let old_s = match &resolved[0] {
                            Value::String(v) => v,
                            _ => return Err("Argument 1 must be string".into())
                        };
                        let new_s = match &resolved[1] {
                            Value::String(v) => v,
                            _ => return Err("Argument 2 must be string".into())
                        };

                        Ok(Value::String(s.replace(old_s, new_s)))
                    },
                    _ => Err(format!("Unknown method '{}' on String", method))
                }
                Value::Instance(inst) => {
                     let class_name = inst.borrow().class_name.clone();
                     let mut cur = Some(class_name);
                     let mut m_def = None;
                     while let Some(n) = cur {
                         let e = env.borrow();
                         let cls = e.get_class(&n).ok_or("Class lost")?;
                         if let Some(m) = cls.methods.get(method) { m_def = Some(m.clone()); break; }
                         cur = cls.parent.clone();
                     }
                     let (params, body) = m_def.ok_or("Method not found")?;
                     let child = Environment::new_child(env.clone());
                     child.borrow_mut().set_variable("this".into(), Value::Instance(inst.clone()));
                     for (p, v) in params.iter().zip(resolved) { child.borrow_mut().set_variable(p.clone(), v); }
                     for i in body { if let Some(r) = execute(&i, child.clone())? { return Ok(r); } }
                     Ok(Value::Null)
                },
                _ => Err("No method on this type".into())
            }
        },
        
        // Appels de fonctions (Simple wrapper)
        Expression::FunctionCall(name, args) => {
             let mut resolved = Vec::new();
             for a in args { resolved.push(evaluate(a, env.clone())?); }
             
             // Built-ins
             match name.as_str() {
                "str" => return Ok(Value::String(format!("{}", resolved[0]))),
                "to_int" => return Ok(Value::Integer(resolved[0].as_int()?)),
                "len" => { // Support générique len()
                     match &resolved[0] {
                         Value::String(s) => return Ok(Value::Integer(s.len() as i64)),
                         Value::List(l) => return Ok(Value::Integer(l.borrow().len() as i64)),
                         Value::Dict(d) => return Ok(Value::Integer(d.borrow().len() as i64)),
                         _ => return Err("Type not supported for len()".into())
                     }
                 },
                 // --- FILE I/O (Natif) ---
                "io_read" => {
                    if resolved.len() != 1 { return Err("io_read attend 1 argument (chemin)".into()); }
                    let path = resolved[0].as_str()?;
                    
                    match fs::read_to_string(&path) {
                        Ok(content) => return Ok(Value::String(content)),
                        Err(_) => return Ok(Value::Null), // Retourne null si fichier introuvable
                    }
                },
                "io_write" => {
                    if resolved.len() != 2 { return Err("io_write attend 2 arguments (chemin, contenu)".into()); }
                    let path = resolved[0].as_str()?;
                    let content = resolved[1].as_str()?; // Force la conversion en string
                    
                    fs::write(&path, content).map_err(|e| format!("Erreur écriture: {}", e))?;
                    return Ok(Value::Boolean(true));
                },
                "io_append" => {
                    if resolved.len() != 2 { return Err("io_append attend 2 arguments (chemin, contenu)".into()); }
                    let path = resolved[0].as_str()?;
                    let content = resolved[1].as_str()?;
                    
                    let mut file = OpenOptions::new()
                        .write(true)
                        .append(true)
                        .create(true) // Crée le fichier s'il n'existe pas
                        .open(&path)
                        .map_err(|e| format!("Erreur ouverture fichier: {}", e))?;

                    write!(file, "{}", content).map_err(|e| format!("Erreur append: {}", e))?;
                    return Ok(Value::Boolean(true));
                },
                "io_exists" => {
                    if resolved.len() != 1 { return Err("io_exists attend 1 argument (chemin)".into()); }
                    let path = resolved[0].as_str()?;
                    return Ok(Value::Boolean(Path::new(&path).exists()));
                },
                "io_delete" => {
                    if resolved.len() != 1 { return Err("io_delete attend 1 argument".into()); }
                    let path = resolved[0].as_str()?;
                    if Path::new(&path).exists() {
                        fs::remove_file(&path).map_err(|e| e.to_string())?;
                        return Ok(Value::Boolean(true));
                    }
                    return Ok(Value::Boolean(false));
                },
                // ------------------------

                // --- TIME ---
                "time_now" => {
                    let start = SystemTime::now();
                    let since_the_epoch = start
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards");
                    // On retourne des millisecondes pour être pratique
                    return Ok(Value::Integer(since_the_epoch.as_millis() as i64));
                },
                // ------------------------

                // --- RANDOM ---
                "rand_int" => {
                    if resolved.len() != 2 { return Err("rand_int attend 2 arguments (min, max)".into()); }
                    let min = resolved[0].as_int()?;
                    let max = resolved[1].as_int()?;
                    
                    if min >= max { return Err("min doit être inférieur à max".into()); }
                    
                    let mut rng = rand::thread_rng();
                    let val = rng.gen_range(min..max); // [min, max[
                    return Ok(Value::Integer(val));
                },
                "rand_float" => {
                    let mut rng = rand::thread_rng();
                    let val: f64 = rng.r#gen(); // 0.0 .. 1.0
                    return Ok(Value::Float(val));
                },
                // ------------------------

                 _ => {}
             }

             // Fonctions utilisateur
             let func = env.borrow().get_function(name).ok_or(format!("Fonction {} inconnue", name))?;
             let child = Environment::new_child(env.clone());
             for (p, v) in func.params.iter().zip(resolved) { child.borrow_mut().set_variable(p.clone(), v); }
             for i in func.body { if let Some(r) = execute(&i, child.clone())? { return Ok(r); } }
             Ok(Value::Null)
        }
    }
}

fn compare_nums<F>(l: Value, r: Value, op: F) -> Result<Value, String> 
where F: Fn(f64, f64) -> bool {
    match (l, r) {
        (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(op(a as f64, b as f64))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(op(a, b))),
        (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean(op(a as f64, b))),
        (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(op(a, b as f64))),
        _ => Err("Comparaison impossible".into())
    }
}

/// Exécute une instruction.
/// Retourne `Ok(Some(Value))` si un `return` a été rencontré, `Ok(None)` sinon.
pub fn execute(instr: &Instruction, env: SharedEnv) -> Result<Option<Value>, String> {
    match instr {
        Instruction::Set(name, expr) => {
            let val = evaluate(expr, env.clone())?;
            env.borrow_mut().set_variable(name.clone(), val);
            Ok(None)
        },
        
        Instruction::Print(expr) => {
            let val = evaluate(expr, env.clone())?;
            println!("{}", val);
            Ok(None)
        },
        
        Instruction::If { condition, body, else_body } => {
            let cond_val = evaluate(condition, env.clone())?;
            if is_truthy(&cond_val) {
                for i in body {
                    // Si une instruction retourne une valeur (return), on propage immédiatement
                    if let Some(ret_val) = execute(i, env.clone())? {
                        return Ok(Some(ret_val));
                    }
                }
            } else {
                for i in else_body {
                    if let Some(ret_val) = execute(i, env.clone())? {
                        return Ok(Some(ret_val));
                    }
                }
            }
            Ok(None)
        },
        
        Instruction::While { condition, body } => {
            while is_truthy(&evaluate(condition, env.clone())?) {
                for i in body {
                    if let Some(ret_val) = execute(i, env.clone())? {
                        return Ok(Some(ret_val));
                    }
                }
            }
            Ok(None)
        },

        Instruction::ForRange { var_name, start, end, step, body } => {
            // 1. Évaluer les bornes (On cast en i64 pour l'itération)
            let start_val = match evaluate(start, env.clone())? {
                Value::Integer(i) => i,
                _ => return Err("Start doit être un entier".to_string()),
            };
            let end_val = match evaluate(end, env.clone())? {
                Value::Integer(i) => i,
                _ => return Err("End doit être un entier".to_string()),
            };
            let step_val = match evaluate(step, env.clone())? {
                Value::Integer(i) => i,
                _ => return Err("Step doit être un entier".to_string()),
            };

            // 2. Boucle native Rust
            // Note: range de Rust ne supporte pas nativement un step variable facilement dans un for simple
            // On utilise une boucle while manuelle en Rust pour gérer tous les cas de step (positif/négatif)
            
            let mut i = start_val;
            // Logique simplifiée pour step positif. 
            // Pour être robuste, il faudrait gérer le sens (i < end si step > 0, i > end si step < 0)
            while (step_val > 0 && i < end_val) || (step_val < 0 && i > end_val) {
                
                // Mettre à jour la variable de boucle dans l'environnement
                env.borrow_mut().set_variable(var_name.clone(), Value::Integer(i));
                
                // Exécuter le corps
                for instr in body {
                    if let Some(ret) = execute(instr, env.clone())? {
                        return Ok(Some(ret));
                    }
                }
                
                i += step_val;
            }
            Ok(None)
        },
        
        Instruction::Return(expr) => {
            let val = evaluate(expr, env.clone())?;
            Ok(Some(val)) // On signale qu'on retourne une valeur
        },
        
        Instruction::ExpressionStatement(expr) => {
            evaluate(expr, env.clone())?;
            Ok(None)
        },

        Instruction::Function { name, params, body } => {
            env.borrow_mut().define_function(name.clone(), params.clone(), body.clone());
            Ok(None)
        },

        Instruction::Input(var_name, prompt_expr) => {
            // 1. Évaluer et afficher le prompt
            let prompt_val = evaluate(prompt_expr, env.clone())?;
            print!("{}", prompt_val);
            
            // Force l'affichage immédiat du prompt (sinon il peut rester dans le buffer)
            io::stdout().flush().map_err(|e| e.to_string())?;

            // 2. Lire l'entrée utilisateur
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).map_err(|e| e.to_string())?;
            
            // Retirer le saut de ligne à la fin (\n)
            let input = buffer.trim();

            // 3. Inférence de type (Int -> Float -> String)
            let val = if let Ok(i) = input.parse::<i64>() {
                Value::Integer(i)
            } else if let Ok(f) = input.parse::<f64>() {
                Value::Float(f)
            } else {
                Value::String(input.to_string())
            };

            // 4. Stocker dans la variable
            env.borrow_mut().set_variable(var_name.clone(), val);
            Ok(None)
        },

        Instruction::Class(def) => {
            env.borrow_mut().define_class(def.clone());
            Ok(None)
        },

        Instruction::SetAttr(obj_expr, attr, val_expr) => {
            let obj_val = evaluate(obj_expr, env.clone())?;
            let new_val = evaluate(val_expr, env.clone())?;
            
            if let Value::Instance(inst_rc) = obj_val {
                // Ici, on a besoin de la mutabilité intérieure via borrow_mut !
                inst_rc.borrow_mut().fields.insert(attr.clone(), new_val);
                Ok(None)
            } else {
                Err("SetAttr sur non-instance".to_string())
            }
        },

        Instruction::Import(path) => {
            // 1. Read the file content
            let source_code = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read file '{}': {}", path, e))?;

            // 2. Compile the source code using the existing compiler logic
            // We get a JSON Value (AST) back
            let ast_json = compiler::compile(&source_code)?;

            // 3. Parse the JSON AST into executable Instructions
            let instructions = parse_block(&ast_json)?;

            // 4. Execute the new instructions in the CURRENT environment.
            // This acts like an "include", meaning variables/functions defined 
            // in the imported file are added to the current scope.
            for i in instructions {
                // We ignore return values from top-level imports usually, 
                // but we propagate errors.
                if let Some(ret) = execute(&i, env.clone())? {
                    // If an import contains a return at top level, it stops the import execution
                    // logic here depends on desired behavior. Usually imports don't return values.
                    return Ok(Some(ret)); 
                }
            }

            Ok(None)
        },
        Instruction::TryCatch { try_body, error_var, catch_body } => {
            // 1. On essaie d'exécuter le bloc TRY instruction par instruction
            let mut error_occurred = None;

            // Note: On utilise un scope enfant pour le try si tu veux isoler les variables, 
            // mais généralement try partage le scope parent. Restons simples pour l'instant (scope partagé).
            for instr in try_body {
                // L'astuce est ici : on utilise match au lieu de ? pour ne pas planter l'interpréteur
                match execute(instr, env.clone()) {
                    Ok(Some(ret)) => return Ok(Some(ret)), // Gestion du return dans un try
                    Ok(None) => continue, // Tout va bien, instruction suivante
                    Err(msg) => {
                        // OUPS ! Une erreur. On la capture et on sort de la boucle du try
                        error_occurred = Some(msg);
                        break;
                    }
                }
            }

            // 2. Si une erreur a eu lieu, on exécute le CATCH
            if let Some(msg) = error_occurred {
                let catch_env = Environment::new_child(env.clone());
                // On injecte le message d'erreur dans la variable définie (ex: "e")
                catch_env.borrow_mut().set_variable(error_var.clone(), Value::String(msg));
                
                for instr in catch_body {
                    if let Some(ret) = execute(instr, catch_env.clone())? {
                        return Ok(Some(ret));
                    }
                }
            }
            
            Ok(None)
        },
        Instruction::Switch { value, cases, default } => {
            let val_to_match = evaluate(value, env.clone())?;
            let mut match_found = false;

            for (case_expr, case_body) in cases {
                let case_val = evaluate(case_expr, env.clone())?;
                
                // On compare les valeurs
                if val_to_match == case_val {
                    match_found = true;
                    // Exécuter le corps du case
                    // Note: On pourrait créer un scope enfant ici si on voulait isoler les variables
                    for instr in case_body {
                         if let Some(ret) = execute(instr, env.clone())? {
                            return Ok(Some(ret));
                        }
                    }
                    // Implicit break: on sort du switch dès qu'un cas est trouvé
                    break; 
                }
            }

            if !match_found {
                for instr in default {
                    if let Some(ret) = execute(instr, env.clone())? {
                        return Ok(Some(ret));
                    }
                }
            }
            
            Ok(None)
        },
    }
}