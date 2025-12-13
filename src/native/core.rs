use crate::ast::Value;
use std::{collections::HashMap, rc::Rc};

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("to_str".to_string(), to_str);
    map.insert("to_int".to_string(), to_int);
    map.insert("to_float".to_string(), to_float);
    map.insert("chr".to_string(), chr);
    map.insert("ord".to_string(), ord);
    map.insert("len".to_string(), len);
    map.insert("fmt".to_string(), fmt);
    map.insert("typeof".to_string(), type_of);
    map.insert("is_instance".to_string(), is_instance);
}

fn to_str(args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::String(format!("{}", args[0])))
}

fn to_int(args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Integer(args[0].as_int()?))
}

fn to_float(args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Float(args[0].as_float()?))
}

fn chr(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("chr attend 1 argument (int)".into()); }
    
    let code = args[0].as_int()?;
    // Conversion sécurisée u32 -> char
    if let Some(c) = std::char::from_u32(code as u32) {
        Ok(Value::String(c.to_string()))
    } else {
        Err(format!("Code caractère invalide : {}", code))
    }
}

fn ord(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("ord attend 1 argument (string)".into()); }
    
    let s = args[0].as_str()?;
    // On prend le premier caractère
    if let Some(c) = s.chars().next() {
        Ok(Value::Integer(c as i64))
    } else {
        Ok(Value::Integer(0)) // Chaîne vide
    }
}

fn len(args: Vec<Value>) -> Result<Value, String> {
    match &args[0] {
        Value::String(s) => return Ok(Value::Integer(s.len() as i64)),
        Value::List(l) => return Ok(Value::Integer(l.borrow().len() as i64)),
        Value::Dict(d) => return Ok(Value::Integer(d.borrow().len() as i64)),
        _ => return Err("Type not supported for len()".into())
    }
}

fn fmt(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 { return Err("fmt attend 2 arguments (valeur, format)".into()); }
    
    let val = &args[0];
    let format_str = args[1].as_str()?;
                                
    // Parsing basique du format (ex: ".2f")
    if format_str.ends_with("f") {
        // Gestion des Floats
        let precision = format_str.trim_start_matches('.').trim_end_matches('f')
            .parse::<usize>().unwrap_or(2); // defaut 2
                                    
        let num = match val {
            Value::Integer(i) => *i as f64,
            Value::Float(f) => *f,
            _ => return Ok(Value::String(format!("{}", val))) // Fallback
        };
                                    
        // Astuce Rust pour précision dynamique
        return Ok(Value::String(format!("{:.1$}", num, precision)));
    } 
                                
    // Tu peux ajouter d'autres formats ici (ex: "b" pour binaire, "x" pour hexa...)
    Ok(Value::String(format!("{}", val)))
}

fn type_of(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("typeof attend 1 argument".into()); }
                                
    // On détermine le nom du type (String)
    let type_name = match &args[0] {
        Value::Integer(_) => "int".to_string(),
        Value::Float(_) => "float".to_string(),
        Value::String(_) => "string".to_string(),
        Value::Boolean(_) => "bool".to_string(),
        Value::Null => "null".to_string(),
        Value::List(_) => "list".to_string(),
        Value::Dict(_) => "dict".to_string(),
        Value::Enum(_) => "enum".to_string(),
        Value::Range(_, _, _) => "range".to_string(),
                                    
        Value::Function(..) => "function".to_string(),
        Value::Class { .. } => "class".to_string(),
        Value::Interface(_) => "interface".to_string(),
                                    
        // Pour l'instance, on récupère le nom dynamiquement
        Value::Instance(i) => {
            let borrow = i.borrow();
            borrow.class.name.clone()
        },
        Value::Native(_) => "function".to_string()
    };

    Ok(Value::String(type_name))
}

fn is_instance(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 { return Err("is_instance(obj, class)".into()); }

    let instance = &args[0];
    let target_class = &args[1];

    // 1. Récupération "tolérante" de la classe cible
    // Si le 2ème argument n'est pas une classe (ex: "String" qui est une fonction), 
    // on retourne 'false' au lieu de crasher. C'est le comportement de 'instanceof' en JS.
    let target_rc = match target_class {
        Value::Class(c) => c,
        _ => return Ok(Value::Boolean(false)), 
    };

    if let Value::Instance(inst) = instance {
        // On récupère le pointeur vers la classe de l'instance
        let mut current_class = inst.borrow().class.clone();

        loop {
            // 2. Comparaison par POINTEUR (Reference Equality)
            // Beaucoup plus rapide et fiable que '=='
            if Rc::ptr_eq(&current_class, target_rc) {
                return Ok(Value::Boolean(true));
            }

            // 3. Remontée via la référence forte
            if let Some(parent) = &current_class.parent_ref {
                current_class = parent.clone();
            } else {
                break;
            }
        }
    }

    Ok(Value::Boolean(false))
}
