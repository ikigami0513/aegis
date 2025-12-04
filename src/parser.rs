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

                // Variables et Appels
                "get" => {
                    let name = array.get(1).and_then(|v| v.as_str()).ok_or("Get attend un nom de variable")?;
                    return Ok(Expression::Variable(name.to_string()));
                },
                // Appel de fonction (Expression)
                "call" => {
                    let func_name = array.get(1).and_then(|v| v.as_str()).ok_or("Call attend un nom")?;
                    // Les arguments sont le reste du tableau après "call" et "name"
                    // Si votre format est ["call", "name", arg1, arg2], on slice à partir de 2
                    let args_json = &array[2..];
                    let mut args = Vec::new();
                    for arg in args_json {
                        args.push(parse_expression(arg)?);
                    }
                    return Ok(Expression::FunctionCall(func_name.to_string(), args));
                },
                "new" => {
                    // ["new", "ClassName", arg1, arg2...]
                    let class_name = array.get(1).and_then(|v| v.as_str()).ok_or("New attend un nom de classe")?.to_string();
                    let args_json = &array[2..];
                    let mut args = Vec::new();
                    for arg in args_json {
                        args.push(parse_expression(arg)?);
                    }
                    return Ok(Expression::New(class_name, args));
                },
                "get_attr" => {
                    let obj = parse_expression(&array[1])?;
                    let attr = array.get(2).and_then(|v| v.as_str()).ok_or("Attr attend un nom")?.to_string();
                    return Ok(Expression::GetAttr(Box::new(obj), attr));
                },
                "call_method" => {
                    let obj = parse_expression(&array[1])?;
                    let method = array.get(2).and_then(|v| v.as_str()).ok_or("Method attend un nom")?.to_string();
                    let args_json = &array[3..];
                    let mut args = Vec::new();
                    for arg in args_json {
                        args.push(parse_expression(arg)?);
                    }
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
                    // Vérification : est-ce que c'est vraiment une fonction ou juste une liste de données ?
                    // Dans notre architecture, tout ce qui commence par une string dans un tableau
                    // est potentiellement du code.
                    
                    // On construit les arguments (tout ce qui est après l'index 0)
                    let args_json = &array[1..];
                    let mut args = Vec::new();
                    for arg in args_json {
                        args.push(parse_expression(arg)?);
                    }
                    
                    // On génère un FunctionCall
                    return Ok(Expression::FunctionCall(cmd_name.to_string(), args));
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
            // ["set", "name", expr]
            let name = array.get(1).and_then(|v| v.as_str()).ok_or("Set attend un nom de variable")?.to_string();
            let expr = parse_expression(&array[2])?;
            // Pas de Box ici selon votre ast.rs précédent : Set(String, Expression)
            Ok(Instruction::Set(name, expr)) 
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
            // ["function", "name", ["arg1"], [body]]
            let name = array[1].as_str().ok_or("Nom de fonction invalide")?.to_string();
            
            // Parsing des arguments
            let params_array = array[2].as_array().ok_or("Params doit être un tableau")?;
            let mut params = Vec::new();
            for p in params_array {
                params.push(p.as_str().ok_or("Param doit être string")?.to_string());
            }

            let body = parse_block(&array[3])?;
            
            Ok(Instruction::Function { name, params, body })
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
            // ["class", "Name", [params], {methods}, "Parent"?]
            let name = array[1].as_str().ok_or("Nom classe")?.to_string();
            
            // Params
            let params_arr = array[2].as_array().ok_or("Params array")?;
            let params: Vec<String> = params_arr.iter().map(|v| v.as_str().unwrap().to_string()).collect();
            
            // Methods: Un objet JSON où chaque clé est le nom, et la valeur est [params, body]
            let methods_map = array[3].as_object().ok_or("Methods object")?;
            let mut methods = HashMap::new();
            
            for (m_name, m_data) in methods_map {
                let m_arr = m_data.as_array().ok_or("Method data array")?;
                let m_params: Vec<String> = m_arr[0].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
                let m_body = parse_block(&m_arr[1])?;
                methods.insert(m_name.clone(), (m_params, m_body));
            }
            
            // Parent (Optionnel)
            let parent = if array.len() > 4 {
                array[4].as_str().map(|s| s.to_string())
            } else {
                None
            };

            Ok(Instruction::Class(crate::ast::ClassDefinition { name, parent, params, methods }))
        },
        
        "set_attr" => {
            // ["set_attr", obj, "attr", value]
            let obj = parse_expression(&array[1])?;
            let attr = array[2].as_str().ok_or("Attr name")?.to_string();
            let val = parse_expression(&array[3])?;
            Ok(Instruction::SetAttr(Box::new(obj), attr, val))
        },
        _ => Err(format!("Instruction inconnue: {}", command)),
    }
}
