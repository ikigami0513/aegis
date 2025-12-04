use crate::ast::{Instruction, Expression, Value};
use crate::environment::Environment;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{self, Write};

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

        // --- LOGIQUE (NOUVEAU) ---
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
                    "at" => Ok(l.borrow()[resolved[1].clone().as_int()? as usize].clone()),
                    _ => Err("Method list unknown".into())
                },
                Value::Dict(d) => match method.as_str() {
                    "insert" => { d.borrow_mut().insert(resolved[0].clone().as_str()?, resolved[1].clone()); Ok(Value::Null) },
                    "remove" => Ok(d.borrow_mut().remove(&resolved[0].clone().as_str()?).unwrap_or(Value::Null)),
                    "keys" => Ok(Value::List(Rc::new(RefCell::new(d.borrow().keys().map(|k| Value::String(k.clone())).collect())))),
                    "len" => Ok(Value::Integer(d.borrow().len() as i64)),
                    _ => Err("Method dict unknown".into())
                },
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
    }
}