use serde_json::Value as JsonValue;
use crate::ast::{Instruction, Expression, Value};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

/// Transforme un bloc JSON (un tableau d'instructions) en Vec<Instruction>
pub fn parse_block(block_json: &JsonValue) -> Result<Vec<Instruction>, String> {
    let array = block_json.as_array()
        .ok_or("Un bloc de code doit être un tableau JSON")?;
    
    array.iter()
        .map(|instr| parse_instruction(instr))
        .collect()
}

/// Convertit une valeur JSON brute (serde) en notre type Value interne
fn json_to_value(json: &JsonValue) -> Result<Value, String> {
    match json {
        JsonValue::Number(n) => {
            if n.is_i64() {
                Ok(Value::Integer(n.as_i64().unwrap()))
            } else if n.is_f64() {
                Ok(Value::Float(n.as_f64().unwrap()))
            } else {
                // Cas rare (u64 très grand), on fallback sur float ou i64
                Ok(Value::Integer(n.as_i64().unwrap_or(0)))
            }
        },
        JsonValue::String(s) => Ok(Value::String(s.clone())),
        JsonValue::Bool(b) => Ok(Value::Boolean(*b)),
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Array(arr) => {
            let mut list = Vec::new();
            for v in arr {
                list.push(json_to_value(v)?);
            }
            Ok(Value::List(Rc::new(RefCell::new(list))))
        },
        JsonValue::Object(map) => {
            let mut dict = HashMap::new();
            for (k, v) in map {
                dict.insert(k.clone(), json_to_value(v)?);
            }
            Ok(Value::Dict(Rc::new(RefCell::new(dict))))
        }
    }
}

/// Parse une expression (qui retourne une valeur)
pub fn parse_expression(json_expr: &JsonValue) -> Result<Expression, String> {
    // 1. Si c'est un tableau, ça peut être une commande ou une opération
    if let Some(array) = json_expr.as_array() {
        if array.is_empty() {
            // Liste vide [] -> Littéral
            return Ok(Expression::Literal(Value::List(Rc::new(RefCell::new(vec![])))));
        }

        // On regarde le premier élément pour voir si c'est une commande connue
        if let Some(command) = array[0].as_str() {
            match command {
                // Logique et Comparaisons
                "&" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::BitAnd(Box::new(left), Box::new(right)));
                }
                "|" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::BitOr(Box::new(left), Box::new(right)));
                }
                "^" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::BitXor(Box::new(left), Box::new(right)));
                }
                "<<" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::ShiftLeft(Box::new(left), Box::new(right)));
                }
                ">>" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::ShiftRight(Box::new(left), Box::new(right)));
                }
                "&&" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::And(Box::new(left), Box::new(right)));
                },
                "||" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::Or(Box::new(left), Box::new(right)));
                },
                "!" => {
                    let expr = parse_expression(&array[1])?;
                    return Ok(Expression::Not(Box::new(expr)));
                }
                "==" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    // Tu devras ajouter Equal(Box<Expr>, Box<Expr>) dans ast.rs
                    return Ok(Expression::Equal(Box::new(left), Box::new(right)));
                },
                "!=" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::NotEqual(Box::new(left), Box::new(right)));
                },
                "<" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::LessThan(Box::new(left), Box::new(right)));
                },
                ">" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::GreaterThan(Box::new(left), Box::new(right)));
                },
                "<=" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::LessEqual(Box::new(left), Box::new(right)));
                },
                ">=" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::GreaterEqual(Box::new(left), Box::new(right)));
                }
                
                // Arithmétique
                "+" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::Add(Box::new(left), Box::new(right)));
                },
                "-" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::Sub(Box::new(left), Box::new(right)));
                },
                "*" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::Mul(Box::new(left), Box::new(right)));
                },
                "/" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::Div(Box::new(left), Box::new(right)));
                },
                "%" => {
                    let left = parse_expression(&array[1])?;
                    let right = parse_expression(&array[2])?;
                    return Ok(Expression::Modulo(Box::new(left), Box::new(right)));
                }

                // Variables et Appels
                "get" => {
                    let name = array.get(1).and_then(|v| v.as_str()).ok_or("Get attend un nom de variable")?;
                    return Ok(Expression::Variable(name.to_string()));
                },
                "lambda" => {
                    let params_json = array[1].as_array().ok_or("Params array")?;
                    let mut params = Vec::new();
                    
                    // On gère la rétro-compatibilité (si ancien format string) ou nouveau format [name, type]
                    for p in params_json {
                        if let Some(name) = p.as_str() {
                            // Ancien format sans type
                            params.push((name.to_string(), None));
                        } else if let Some(pair) = p.as_array() {
                            // Nouveau format avec type
                            let name = pair[0].as_str().unwrap().to_string();
                            let typ = pair[1].as_str().map(|s| s.to_string());
                            params.push((name, typ));
                        }
                    }

                    let body = parse_block(&array[2])?;
                    
                    // On retourne Expression::Function
                    return Ok(Expression::Function { params, ret_type: None, body });
                },
                "call" => {
                    // ["call", target_expr, [args]]
                    let target = parse_expression(&array[1])?;
                    // args est un tableau dans array[2]
                    let args_arr = array[2].as_array().ok_or("Call args must be array")?;
                    let args = args_arr.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    return Ok(Expression::Call(Box::new(target), args));
                },
                "new" => {
                    // ["new", target_expr, arg1, arg2...]
                    let target = parse_expression(&array[1])?;
                    let args_json = &array[2..];
                    let args = args_json.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    return Ok(Expression::New(Box::new(target), args));
                },
                "get_attr" => {
                    let obj = parse_expression(&array[1])?;
                    let attr = array.get(2).and_then(|v| v.as_str()).ok_or("Attr attend un nom")?.to_string();
                    return Ok(Expression::GetAttr(Box::new(obj), attr));
                },
                "call_method" => {
                    let obj = parse_expression(&array[1])?;
                    let method = array[2].as_str().ok_or("Method")?.to_string();
                    
                    // ["call_method", obj, "method", [arg1, arg2]]
                    let args_arr = array[3].as_array().ok_or("Method args must be array")?;
                    let args = args_arr.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    
                    return Ok(Expression::CallMethod(Box::new(obj), method, args));
                },

                // Listes et Dicts
                "make_list" => {
                    // ["make_list", expr1, expr2...]
                    let args_json = &array[1..];
                    let mut items = Vec::new();
                    for arg in args_json {
                        items.push(parse_expression(arg)?);
                    }
                    return Ok(Expression::List(items));
                },
                "make_dict" => {
                    // ["make_dict", [k, v], [k, v]...]
                    let args_json = &array[1..];
                    let mut entries = Vec::new();
                    for entry in args_json {
                        let entry_arr = entry.as_array().ok_or("Dict entry must be array")?;
                        let key = entry_arr[0].as_str().ok_or("Key must be string")?.to_string();
                        let val = parse_expression(&entry_arr[1])?;
                        entries.push((key, val));
                    }
                    return Ok(Expression::Dict(entries));
                },

                // Fallback pour les appels implicite (ex: ["print", ...])
                cmd_name => {
                    // Fallback implicit call: print("hello") -> Call(Var("print"), ["hello"])
                    let args_json = &array[1..];
                    let args = args_json.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    
                    // On crée une Variable pour le nom de la commande
                    let target = Expression::Variable(cmd_name.to_string());
                    
                    return Ok(Expression::Call(Box::new(target), args));
                }
            }
        }
    }

    // 2. Sinon, c'est un littéral simple (String, Number, Bool)
    let val = json_to_value(json_expr)?;
    Ok(Expression::Literal(val))
}

/// Parse une instruction (Action)
pub fn parse_instruction(json_instr: &JsonValue) -> Result<Instruction, String> {
    let array = json_instr.as_array().ok_or("L'instruction doit être un tableau JSON")?;
    if array.is_empty() {
        return Err("Instruction vide".to_string());
    }

    let command = array[0].as_str().ok_or("La commande doit être une chaîne de caractères")?;

    match command {
        "set" => {
            let name = array[1].as_str().unwrap().to_string();
            // Le type est à l'index 2 (String ou Null)
            let type_annot = array[2].as_str().map(|s| s.to_string());
            let expr = parse_expression(&array[3])?;
            Ok(Instruction::Set(name, type_annot, expr))
        },
        "print" => {
            // ["print", expr]
            let expr = parse_expression(&array[1])?;
            Ok(Instruction::Print(expr))
        },
        "if" => {
            // ["if", cond, true_block, false_block?]
            let cond = parse_expression(&array[1])?;
            let true_block = parse_block(&array[2])?;
            
            let false_block = if array.len() > 3 {
                parse_block(&array[3])?
            } else {
                Vec::new()
            };

            Ok(Instruction::If { 
                condition: cond, 
                body: true_block, 
                else_body: false_block 
            })
        },
        "while" => {
            // ["while", cond, body]
            let cond = parse_expression(&array[1])?;
            let body = parse_block(&array[2])?;
            Ok(Instruction::While { 
                condition: cond, 
                body 
            })
        },
        "for_range" => {
            // ["for_range", "i", 0, 10, 1, [body]]
            let var_name = array[1].as_str().ok_or("Nom de variable for invalide")?.to_string();
            let start = parse_expression(&array[2])?;
            let end = parse_expression(&array[3])?;
            let step = parse_expression(&array[4])?;
            let body = parse_block(&array[5])?;

            Ok(Instruction::ForRange { var_name, start, end, step, body })
        },
        "return" => {
            // ["return", expr]
            let expr = parse_expression(&array[1])?;
            Ok(Instruction::Return(expr))
        },
        "call" => {
            // ["call", "func_name", args...] utilisé comme instruction
            // On réutilise la logique d'expression mais on l'enveloppe
            let expr = parse_expression(json_instr)?;
            Ok(Instruction::ExpressionStatement(expr))
        },
        "call_method" => {
            // ["call_method", obj, "method", args...]
            // On délègue le parsing à parse_expression qui sait gérer "call_method"
            // Et on l'enveloppe dans une Instruction (pour l'exécuter sans attendre de retour)
            let expr = parse_expression(json_instr)?;
            Ok(Instruction::ExpressionStatement(expr))
        },
        "function" => {
            let name = array[1].as_str().unwrap().to_string();
            
            // Params est un tableau. Ses éléments peuvent être :
            // 1. Une string simple "x" (Ancien format ou parser simplifié)
            // 2. Un tableau ["x", "int"] (Nouveau format typé)
            // 3. Un tableau ["x", null] (Nouveau format non typé)
            
            let params_json = array[2].as_array().unwrap();
            let mut params = Vec::new();
            
            for p in params_json {
                if let Some(p_str) = p.as_str() {
                    // Cas 1 : Juste une string (ex: "x") -> Type None
                    params.push((p_str.to_string(), None));
                } else if let Some(pair) = p.as_array() {
                    // Cas 2 & 3 : Tableau [nom, type]
                    if pair.len() >= 1 {
                        let p_name = pair[0].as_str().unwrap().to_string();
                        
                        let p_type = if pair.len() > 1 {
                            pair[1].as_str().map(|s| s.to_string())
                        } else {
                            None
                        };
                        
                        params.push((p_name, p_type));
                    }
                }
            }

            // Index 3 pour le type de retour (peut être null ou absent si vieux JSON)
            let ret_type = if array.len() > 3 {
                array[3].as_str().map(|s| s.to_string())
            } else {
                None
            };

            // Index 4 pour le body
            // Attention : Si ret_type était absent dans l'ancien format, body était à l'index 3 ?
            // Il vaut mieux se fier à ton Parser actuel.
            // Ton parser actuel génère TOUJOURS 5 éléments : ["function", name, params, ret_type, body]
            // Donc array[4] est correct SI le parser est utilisé.
            
            let body = parse_block(&array[4])?;
            
            Ok(Instruction::Function { name, params, ret_type, body })
        },
        "input" => {
            // ["input", "nom_var", "Texte du prompt"]
            let var_name = array.get(1)
                .and_then(|v| v.as_str())
                .ok_or("Input attend un nom de variable (string)")?
                .to_string();
            
            let prompt = parse_expression(&array[2])?;
            
            Ok(Instruction::Input(var_name, prompt))
        },
        "class" => {
            let name = array[1].as_str().unwrap().to_string();
            
            // Params constructeur
            let params_json = array[2].as_array().unwrap();
            let mut params = Vec::new();
            for p in params_json {
                // Gestion robuste comme pour Function
                if let Some(s) = p.as_str() {
                    params.push((s.to_string(), None));
                } else if let Some(pair) = p.as_array() {
                    let n = pair[0].as_str().unwrap().to_string();
                    let t = pair[1].as_str().map(|s| s.to_string());
                    params.push((n, t));
                }
            }
            
            // Methods
            let methods_map = array[3].as_object().unwrap();
            let mut methods = HashMap::new();
            
            for (k, v) in methods_map {
                let m_arr = v.as_array().unwrap();
                
                // Params méthode
                let m_params_json = m_arr[0].as_array().unwrap();
                let mut m_params = Vec::new();
                for p in m_params_json {
                    if let Some(s) = p.as_str() {
                        m_params.push((s.to_string(), None));
                    } else if let Some(pair) = p.as_array() {
                        let n = pair[0].as_str().unwrap().to_string();
                        let t = pair[1].as_str().map(|s| s.to_string());
                        m_params.push((n, t));
                    }
                }
                
                let m_body = parse_block(&m_arr[1])?;
                methods.insert(k.clone(), (m_params, m_body));
            }
            
            let parent = if array.len() > 4 { array[4].as_str().map(|s| s.to_string()) } else { None };
            Ok(Instruction::Class(crate::ast::ClassDefinition { name, parent, params, methods }))
        },
        
        "set_attr" => {
            // ["set_attr", obj, "attr", value]
            let obj = parse_expression(&array[1])?;
            let attr = array[2].as_str().ok_or("Attr name")?.to_string();
            let val = parse_expression(&array[3])?;
            Ok(Instruction::SetAttr(Box::new(obj), attr, val))
        },
        "import" => {
            // ["import", "filename"]
            let path = array.get(1)
                .and_then(|v| v.as_str())
                .ok_or("Import expects a file path string")?
                .to_string();
            Ok(Instruction::Import(path))
        },
        "try" => {
            // ["try", [body], "var", [catch]]
            let try_body = parse_block(&array[1])?;
            let error_var = array[2].as_str().ok_or("Error var must be string")?.to_string();
            let catch_body = parse_block(&array[3])?;
            
            Ok(Instruction::TryCatch { 
                try_body, 
                error_var, 
                catch_body 
            })
        },
        "switch" => {
            // ["switch", value, cases_array, default_body]
            let value = parse_expression(&array[1])?;
            
            let cases_json = array[2].as_array().ok_or("Cases must be array")?;
            let mut cases = Vec::new();
            
            for c in cases_json {
                let c_arr = c.as_array().unwrap();
                let c_val = parse_expression(&c_arr[0])?;
                let c_body = parse_block(&c_arr[1])?;
                cases.push((c_val, c_body));
            }
            
            let default_body = parse_block(&array[3])?;
            
            Ok(Instruction::Switch { 
                value, 
                cases, 
                default: default_body 
            })
        },
        "namespace" => {
            let name = array[1].as_str().ok_or("Namespace name must be string")?.to_string();
            let body = parse_block(&array[2])?;
            Ok(Instruction::Namespace { name, body })
        },
        _ => Err(format!("Instruction inconnue: {}", command)),
    }
}
