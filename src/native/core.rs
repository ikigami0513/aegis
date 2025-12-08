use crate::ast::Value;
use std::collections::HashMap;

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("str".to_string(), str);
    map.insert("to_int".to_string(), to_int);
    map.insert("to_float".to_string(), to_float);
    map.insert("len".to_string(), len);
    map.insert("fmt".to_string(), fmt);
    map.insert("typeof".to_string(), type_of);
}

fn str(args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::String(format!("{}", args[0])))
}

fn to_int(args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Integer(args[0].as_int()?))
}

fn to_float(args: Vec<Value>) -> Result<Value, String> {
    Ok(Value::Float(args[0].as_float()?))
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
                                    
        Value::Function(..) => "function".to_string(),
        Value::Class { .. } => "class".to_string(),
                                    
        // Pour l'instance, on récupère le nom dynamiquement
        Value::Instance(i) => {
            let borrow = i.borrow();
            if let Value::Class { ref name, .. } = *borrow.class {
                name.clone()
            } 
            else {
                "instance".to_string()
            }
        },
        Value::Native(_) => "function".to_string()
    };

    Ok(Value::String(type_name))
}
