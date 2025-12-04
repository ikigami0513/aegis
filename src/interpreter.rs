use crate::ast::{Instruction, Expression, Value};
use crate::environment::Environment;
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
        Value::List(l) => !l.is_empty(),
        Value::Dict(d) => !d.is_empty(),
    }
}

/// Évalue une expression pour obtenir une valeur
pub fn evaluate(expr: &Expression, env: SharedEnv) -> Result<Value, String> {
    match expr {
        Expression::Literal(v) => Ok(v.clone()),
        
        Expression::Variable(name) => {
            env.borrow().get_variable(name)
                .ok_or_else(|| format!("Variable non définie : {}", name))
        },

        // --- ARITHMÉTIQUE ---
        
        Expression::Add(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            match (l, r) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + b as f64)),
                // Concaténation
                (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b))),
                (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
                _ => Err("Types incompatibles pour l'addition".to_string()),
            }
        },

        Expression::Sub(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            match (l, r) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - b as f64)),
                _ => Err("Types incompatibles pour la soustraction".to_string()),
            }
        },

        Expression::Mul(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            match (l, r) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * b as f64)),
                _ => Err("Types incompatibles pour la multiplication".to_string()),
            }
        },

        Expression::Div(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            match (l, r) {
                (Value::Integer(a), Value::Integer(b)) => {
                    if b == 0 { return Err("Division par zéro".to_string()); }
                    // Division entière par défaut pour Int/Int
                    Ok(Value::Integer(a / b)) 
                },
                (Value::Float(a), Value::Float(b)) => {
                    if b == 0.0 { return Err("Division par zéro".to_string()); }
                    Ok(Value::Float(a / b))
                },
                (Value::Integer(a), Value::Float(b)) => {
                    if b == 0.0 { return Err("Division par zéro".to_string()); }
                    Ok(Value::Float(a as f64 / b))
                },
                (Value::Float(a), Value::Integer(b)) => {
                    if b == 0 { return Err("Division par zéro".to_string()); }
                    Ok(Value::Float(a / b as f64))
                },
                _ => Err("Types incompatibles pour la division".to_string()),
            }
        },

        // --- COMPARAISONS ---

        Expression::Equal(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            // Rust gère l'égalité profonde via PartialEq dérivé sur l'enum Value
            Ok(Value::Boolean(l == r))
        },

        Expression::LessThan(left, right) => {
            let l = evaluate(left, env.clone())?;
            let r = evaluate(right, env.clone())?;
            match (l, r) {
                (Value::Integer(a), Value::Integer(b)) => Ok(Value::Boolean(a < b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Boolean(a < b)),
                (Value::Integer(a), Value::Float(b)) => Ok(Value::Boolean((a as f64) < b)),
                (Value::Float(a), Value::Integer(b)) => Ok(Value::Boolean(a < (b as f64))),
                _ => Err("Comparaison impossible pour ces types".to_string()),
            }
        },

        // --- FONCTIONS ---

        Expression::FunctionCall(name, args_exprs) => {
            // Note: Pour implémenter cela complètement, l'Environnement doit stocker les fonctions.
            // Pour l'instant, on gère juste les appels natifs basiques ou on renvoie une erreur.
            
            // Évaluation des arguments
            let mut resolved_args = Vec::new();
            for arg_expr in args_exprs {
                resolved_args.push(evaluate(arg_expr, env.clone())?);
            }

            match name.as_str() {
                "len" => {
                    if resolved_args.len() != 1 { return Err("len() attend 1 argument".to_string()); }
                    match &resolved_args[0] {
                        Value::String(s) => Ok(Value::Integer(s.len() as i64)),
                        Value::List(l) => Ok(Value::Integer(l.len() as i64)),
                        Value::Dict(d) => Ok(Value::Integer(d.len() as i64)),
                        _ => Err("len() ne supporte pas ce type".to_string()),
                    }
                },
                "str" => {
                    if resolved_args.len() != 1 { return Err("str() attend 1 argument".to_string()); }
                    Ok(Value::String(format!("{}", resolved_args[0])))
                },
                // Fonction utilisateur
                _ => {
                    let func_def_opt = env.borrow().get_function(name);

                    if let Some(func_def) = func_def_opt {
                        if resolved_args.len() != func_def.params.len() {
                            return Err(format!("Arity mismatch: {} attend {} args", name, func_def.params.len()));
                        }

                        // 4. Créer le scope enfant
                        let child_env = Environment::new_child(env.clone());

                        // 5. Injecter les arguments
                        for (param_name, arg_val) in func_def.params.iter().zip(resolved_args) {
                            child_env.borrow_mut().set_variable(param_name.clone(), arg_val);
                        }

                        // 6. Exécuter le corps
                        // Note : il faut importer execute dans evaluate ou le rendre dispo
                        for instr in func_def.body {
                            // Appel récursif de execute
                            if let Some(ret_val) = execute(&instr, child_env.clone())? {
                                return Ok(ret_val);
                            }
                        }
                        return Ok(Value::Null); // Pas de return explicite
                    }

                    Err(format!("Fonction inconnue : {}", name))
                }
            }
        },
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
    }
}