use serde_json::Value as JsonValue;
use crate::ast::{ClassDefinition, Expression, Instruction, Statement, Value, nodes::{ClassField, ClassProperty}, value::Visibility};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub fn parse_block(block_json: &JsonValue) -> Result<Vec<Statement>, String> {
    let array = block_json.as_array().ok_or("Block must be a JSON array")?;
    array.iter().map(|instr| parse_statement_json(instr)).collect()
}

fn parse_visibility(v: &str) -> Visibility {
    match v {
        "private" => Visibility::Private,
        "protected" => Visibility::Protected,
        _ => Visibility::Public,
    }
}

fn json_to_value(json: &JsonValue) -> Result<Value, String> {
    match json {
        JsonValue::Number(n) => {
            if n.is_i64() { Ok(Value::Integer(n.as_i64().unwrap())) }
            else if n.is_f64() { Ok(Value::Float(n.as_f64().unwrap())) }
            else { Ok(Value::Integer(n.as_i64().unwrap_or(0))) }
        },
        JsonValue::String(s) => Ok(Value::String(s.clone())),
        JsonValue::Bool(b) => Ok(Value::Boolean(*b)),
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Array(arr) => {
            let mut list = Vec::new();
            for v in arr { list.push(json_to_value(v)?); }
            Ok(Value::List(Rc::new(RefCell::new(list))))
        },
        JsonValue::Object(map) => {
            let mut dict = HashMap::new();
            for (k, v) in map { dict.insert(k.clone(), json_to_value(v)?); }
            Ok(Value::Dict(Rc::new(RefCell::new(dict))))
        }
    }
}

pub fn parse_expression(json_expr: &JsonValue) -> Result<Expression, String> {
    if let Some(array) = json_expr.as_array() {
        if array.is_empty() { return Ok(Expression::Literal(Value::List(Rc::new(RefCell::new(vec![]))))); }
        
        if let Some(cmd_name) = array[0].as_str() {
            match cmd_name {
                // --- Variables ---
                "get" => {
                    let name = array[1].as_str().ok_or("Var name missing")?;
                    Ok(Expression::Variable(name.to_string()))
                },

                // --- Logique ---
                "&&" => Ok(Expression::And(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "||" => Ok(Expression::Or(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "!" => Ok(Expression::Not(Box::new(parse_expression(&array[1])?))),
                "?" => {
                    // ["?", cond, true, false]
                    let cond = parse_expression(&array[1])?;
                    let then_branch = parse_expression(&array[2])?;
                    let else_branch = parse_expression(&array[3])?;
                    
                    Ok(Expression::Ternary(
                        Box::new(cond),
                        Box::new(then_branch),
                        Box::new(else_branch)
                    ))
                },
                "??" => {
                    let left = parse_expression(&array[2])?;
                    let right = parse_expression(&array[3])?;
                    Ok(Expression::NullCoalescing(Box::new(left), Box::new(right)))
                },
                
                // --- Comparaison ---
                "==" => Ok(Expression::Equal(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "!=" => Ok(Expression::NotEqual(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "<" => Ok(Expression::LessThan(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                ">" => Ok(Expression::GreaterThan(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "<=" => Ok(Expression::LessEqual(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                ">=" => Ok(Expression::GreaterEqual(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                
                // --- Arithmétique ---
                "+" => Ok(Expression::Add(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "-" => {
                     Ok(Expression::Sub(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?)))
                },
                "*" => Ok(Expression::Mul(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "/" => Ok(Expression::Div(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "%" => Ok(Expression::Modulo(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                
                // --- Bitwise ---
                "&" => Ok(Expression::BitAnd(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "|" => Ok(Expression::BitOr(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "^" => Ok(Expression::BitXor(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                "<<" => Ok(Expression::ShiftLeft(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),
                ">>" => Ok(Expression::ShiftRight(Box::new(parse_expression(&array[1])?), Box::new(parse_expression(&array[2])?))),

                // --- Structures & OOP ---
                "make_list" => Ok(Expression::List(array[1..].iter().map(parse_expression).collect::<Result<_,_>>()?)),
                "make_dict" => {
                    let mut entries = Vec::new();
                    for entry in &array[1..] {
                        let arr = entry.as_array().ok_or("Dict entry array")?;
                        let k = arr[0].as_str().ok_or("Key string")?.to_string();
                        let v = parse_expression(&arr[1])?;
                        entries.push((k, v));
                    }
                    Ok(Expression::Dict(entries))
                },
                "new" => {
                    let class_name_expr = parse_expression(&array[1])?;
                    let args_json = &array[2..];
                    let args = args_json.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    Ok(Expression::New(Box::new(class_name_expr), args))
                },
                "get_attr" => Ok(Expression::GetAttr(Box::new(parse_expression(&array[1])?), array[2].as_str().ok_or("Attr")?.to_string())),
                
                // --- Fonctions ---
                "lambda" => {
                    let params_json = array[1].as_array().ok_or("Params array")?;
                    let mut params = Vec::new();
                    for p in params_json {
                        if let Some(name) = p.as_str() {
                            params.push((name.to_string(), None));
                        } else if let Some(pair) = p.as_array() {
                            let name = pair[0].as_str().unwrap().to_string();
                            let typ = pair[1].as_str().map(|s| s.to_string());
                            params.push((name, typ));
                        }
                    }
                    let body = parse_block(&array[2])?;
                    Ok(Expression::Function { params, ret_type: None, body })
                },

                // --- GESTION ROBUSTE DES APPELS (AVEC OU SANS LIGNE) ---
                
                "call" => {
                    // Avec Ligne: ["call", LINE, TARGET, ARGS] -> Len 4
                    // Sans Ligne: ["call", TARGET, ARGS]       -> Len 3
                    let (target_idx, args_idx) = if array.len() == 4 { (2, 3) } else { (1, 2) };
                    
                    let target = parse_expression(&array[target_idx])?;
                    let args_arr = array[args_idx].as_array().ok_or("Call: Args array missing")?;
                    let args = args_arr.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    
                    Ok(Expression::Call(Box::new(target), args))
                },

                "call_method" => {
                    // Avec Ligne: ["call_method", LINE, OBJ, METHOD, ARGS] -> Len 5
                    // Sans Ligne: ["call_method", OBJ, METHOD, ARGS]       -> Len 4
                    let (obj_idx, method_idx, args_idx) = if array.len() == 5 { (2, 3, 4) } else { (1, 2, 3) };

                    let obj = parse_expression(&array[obj_idx])?;
                    let method = array[method_idx].as_str().ok_or("CallMethod: Method name missing")?.to_string();
                    let args_arr = array[args_idx].as_array().ok_or("CallMethod: Args array missing")?;
                    let args = args_arr.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    
                    Ok(Expression::CallMethod(Box::new(obj), method, args))
                },

                "super_call" => {
                    // Avec Ligne: ["super_call", LINE, METHOD, ARGS] -> Len 4
                    // Sans Ligne: ["super_call", METHOD, ARGS]       -> Len 3
                    let (method_idx, args_idx) = if array.len() == 4 { (2, 3) } else { (1, 2) };

                    let method = array[method_idx].as_str().ok_or("SuperCall: Method name missing")?.to_string();
                    let args_arr = array[args_idx].as_array().ok_or("SuperCall: Args array missing")?;
                    let args = args_arr.iter().map(parse_expression).collect::<Result<_,_>>()?;
                    
                    Ok(Expression::SuperCall(method, args))
                },

                "range" => {
                    let start = parse_expression(&array[2])?;
                    let end = parse_expression(&array[3])?;
                    // On peut créer un OpCode spécifique ou une Expression dédiée.
                    // Créons une Expression::Range dans ast/mod.rs d'abord si ce n'est pas fait.
                    Ok(Expression::Range(Box::new(start), Box::new(end)))
                },
                // -----------------------------------------------------

                // Fallback (pour les expressions génériques)
                _ => {
                     // Si ce n'est pas un mot-clé connu, est-ce un appel implicite ?
                     // Ex: ["ma_fonction", arg1] -> Call
                     if array.len() > 1 {
                         let args = array[1..].iter().map(parse_expression).collect::<Result<_,_>>()?;
                         let target = Expression::Variable(cmd_name.to_string());
                         Ok(Expression::Call(Box::new(target), args))
                     } else {
                         let val = json_to_value(json_expr)?;
                         Ok(Expression::Literal(val))
                     }
                }
            }
        } else {
             // Tableau de données [1, 2]
             let val = json_to_value(json_expr)?;
             Ok(Expression::Literal(val))
        }
    } else {
        // Littéral simple
        let val = json_to_value(json_expr)?;
        Ok(Expression::Literal(val))
    }
}

pub fn parse_statement_json(json_instr: &JsonValue) -> Result<Statement, String> {
    let array = json_instr.as_array().ok_or("Instruction must be array")?;
    let command = array[0].as_str().ok_or("Command must be string")?;
    
    // Le 2ème élément est la ligne
    let line = array[1].as_u64().ok_or("Line number missing (Check Parser)")? as usize;

    let instruction = match command {
        "set" => {
            let name = array[2].as_str().unwrap().to_string();
            let type_annot = array[3].as_str().map(|s| s.to_string());
            let expr = parse_expression(&array[4])?;
            Ok(Instruction::Set(name, type_annot, expr)) 
        },
        "set_attr" => {
            let obj = parse_expression(&array[2])?;
            let attr = array[3].as_str().unwrap().to_string();
            let val = parse_expression(&array[4])?;
            Ok(Instruction::SetAttr(Box::new(obj), attr, val))
        },
        "print" => Ok(Instruction::Print(parse_expression(&array[2])?)),
        "input" => {
            let var = array[2].as_str().unwrap().to_string();
            let prompt = parse_expression(&array[3])?;
            Ok(Instruction::Input(var, prompt))
        },
        "if" => {
            Ok(Instruction::If { 
                condition: parse_expression(&array[2])?, 
                body: parse_block(&array[3])?, 
                else_body: if array.len() > 4 { parse_block(&array[4])? } else { vec![] }
            })
        },
        "while" => Ok(Instruction::While { condition: parse_expression(&array[2])?, body: parse_block(&array[3])? }),
        
        "return" => Ok(Instruction::Return(parse_expression(&array[2])?)),
        
        "call" | "call_method" | "super_call" => {
            // Ici, parse_expression va gérer le format imbriqué
            Ok(Instruction::ExpressionStatement(parse_expression(json_instr)?))
        },
        
        "function" => {
            let name = array[2].as_str().unwrap().to_string();
            let params_json = array[3].as_array().unwrap();
            let mut params = Vec::new();
            for p in params_json {
                if let Some(s) = p.as_str() {
                    params.push((s.to_string(), None));
                } else if let Some(pair) = p.as_array() {
                    let n = pair[0].as_str().unwrap().to_string();
                    let t = pair[1].as_str().map(|s| s.to_string());
                    params.push((n, t));
                }
            }
            let ret_type = array[4].as_str().map(|s| s.to_string());
            let body = parse_block(&array[5])?;
            Ok(Instruction::Function { name, params, ret_type, body })
        },
        
        "class" => {
            // ["class", line, name, methods, parent, fields, visibilities]
            
            let name = array[2].as_str().ok_or("Invalid class name")?.to_string();
            
            // 1. Parsing des Méthodes (Adapté à ta HashMap)
            let methods_map_json = array[3].as_object().ok_or("Invalid methods object")?;
            let mut methods = std::collections::HashMap::new();
            
            for (m_name, m_data) in methods_map_json {
                let m_arr = m_data.as_array().ok_or("Invalid method array")?;
                
                // JSON attendu : [params, body, is_static]
                
                // A. Params
                let params_arr = m_arr[0].as_array().ok_or("Invalid params array")?;
                let mut params = Vec::new();
                for p in params_arr {
                    if let Some(p_row) = p.as_array() {
                         let p_name = p_row[0].as_str().unwrap().to_string();
                         let p_type = if p_row.len() > 1 && !p_row[1].is_null() {
                             Some(p_row[1].as_str().unwrap().to_string())
                         } else { None };
                         params.push((p_name, p_type));
                    }
                }
                
                // B. Body
                let body = parse_block(&m_arr[1])?;

                // C. Static (NOUVEAU)
                // Si l'élément 2 existe et est true, c'est statique.
                let is_static = if m_arr.len() > 2 {
                    m_arr[2].as_bool().unwrap_or(false)
                } else { false };

                let is_final = if m_arr.len() > 3 {
                    m_arr[3].as_bool().unwrap_or(false)
                } else { false };
                
                // On insère le tuple (params, body, is_static)
                methods.insert(m_name.clone(), (params, body, is_static, is_final));
            }

            // 2. Parsing du Parent
            let parent = if array[4].is_null() {
                None
            } else {
                Some(array[4].as_str().unwrap().to_string())
            };

            // 3. Parsing des Champs (Fields)
            let mut fields = Vec::new();
            let mut properties = Vec::new();

            if array.len() > 5 {
                if let Some(members_arr) = array[5].as_array() {
                    for m in members_arr {
                        let m_data = m.as_array().ok_or("Invalid member struct")?;
                        let kind = m_data[0].as_str().ok_or("Invalid member kind")?;
                        // JSON: ["field", name, vis_str, default_val]
                        
                        if kind == "field" {
                            let f_name = m_data[1].as_str().unwrap().to_string();
                            let f_vis_str = m_data[2].as_str().unwrap();
                            let default_expr = parse_expression(&m_data[3])?;

                            let is_static = if m_data.len() > 4 {
                                m_data[4].as_bool().unwrap_or(false)
                            } else { false };
                            
                            fields.push(ClassField {
                                name: f_name,
                                visibility: parse_visibility(f_vis_str),
                                default_value: default_expr,
                                is_static
                            });
                        }
                        else if kind == "prop" {
                            // ["prop", name, vis, is_static, getter, setter]
                            let p_name = m_data[1].as_str().unwrap().to_string();
                            let p_vis_str = m_data[2].as_str().unwrap();
                            let is_static = m_data[3].as_bool().unwrap_or(false);
                            
                            // Parsing Getter
                            let getter_data = if !m_data[4].is_null() {
                                let g_arr = m_data[4].as_array().unwrap();
                                // g_arr[0] est params (vide), g_arr[1] est body
                                let body = parse_block(&g_arr[1])?;
                                Some((vec![], body))
                            } else { None };
                            
                            // Parsing Setter
                            let setter_data = if !m_data[5].is_null() {
                                let s_arr = m_data[5].as_array().unwrap();
                                let params_json = s_arr[0].as_array().unwrap();
                                
                                // On parse les params du setter (ex: [val])
                                let mut params = Vec::new();
                                for p in params_json {
                                    if let Some(p_row) = p.as_array() {
                                         let p_name = p_row[0].as_str().unwrap().to_string();
                                         let p_type = if p_row.len() > 1 && !p_row[1].is_null() { Some(p_row[1].as_str().unwrap().to_string()) } else { None };
                                         params.push((p_name, p_type));
                                    }
                                }
                                let body = parse_block(&s_arr[1])?;
                                Some((params, body))
                            } else { None };

                            properties.push(ClassProperty {
                                name: p_name,
                                visibility: parse_visibility(p_vis_str),
                                is_static,
                                getter: getter_data,
                                setter: setter_data,
                            });
                        }
                    }
                }
            }

            // 4. Parsing des Visibilités (Visibilities) - NOUVEAU
            let mut visibilities = std::collections::HashMap::new();
            if array.len() > 6 {
                if let Some(vis_obj) = array[6].as_object() {
                    for (key, val) in vis_obj {
                        let vis_str = val.as_str().unwrap_or("public");
                        visibilities.insert(key.clone(), parse_visibility(vis_str));
                    }
                }
            }

            let is_class_final = if array.len() > 7 {
                array[7].as_bool().unwrap_or(false)
            } else { false };

            Ok(Instruction::Class(ClassDefinition {
                name,
                parent,
                methods,
                fields,
                properties,
                visibilities,
                is_final: is_class_final
            }))
        },

        "enum" => {
            let name = array[2].as_str().unwrap().to_string();
            let variants_arr = array[3].as_array().unwrap();
            
            let variants: Vec<String> = variants_arr.iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
                
            Ok(Instruction::Enum(name, variants))
        },
        
        "import" => Ok(Instruction::Import(array[2].as_str().unwrap().to_string())),
        
        "switch" => {
            let val = parse_expression(&array[2])?;
            let cases_json = array[3].as_array().unwrap();
            let mut cases = Vec::new();
            for c in cases_json {
                let c_arr = c.as_array().unwrap();
                cases.push((parse_expression(&c_arr[0])?, parse_block(&c_arr[1])?));
            }
            let def = parse_block(&array[4])?;
            Ok(Instruction::Switch { value: val, cases, default: def })
        },
        
        "try" => {
            Ok(Instruction::TryCatch { 
                try_body: parse_block(&array[2])?, 
                error_var: array[3].as_str().unwrap().to_string(), 
                catch_body: parse_block(&array[4])? 
            })
        },

        "throw" => Ok(Instruction::Throw(parse_expression(&array[2])?)),
        
        "namespace" => {
            Ok(Instruction::Namespace {
                name: array[2].as_str().unwrap().to_string(),
                body: parse_block(&array[3])?
            })
        },
        
        "break" => Ok(Instruction::ExpressionStatement(Expression::Literal(Value::Null))),

        "continue" => Ok(Instruction::Continue),

        "const" => {
            let name = array[2].as_str().unwrap().to_string();
            let expr = parse_expression(&array[3])?;
            Ok(Instruction::Const(name, expr))
        },

        "foreach" => {
            let var_name = array[2].as_str().unwrap().to_string();
            let iterable = parse_expression(&array[3])?;
            let body = parse_block(&array[4])?;
                    
            Ok(Instruction::ForEach(var_name, iterable, body))
        },
        
        _ => Err(format!("Instruction inconnue: {}", command)),
    }?;

    Ok(Statement { kind: instruction, line })
}
