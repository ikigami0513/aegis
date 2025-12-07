use crate::ast::Value;
use std::collections::HashMap;

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("str".to_string(), str);
    map.insert("to_int".to_string(), to_int);
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
    if args.len() != 1 { 
        return Err("typeof attend 1 argument".into()); 
    }
                                
    let type_name = match args[0] {
        Value::Integer(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Boolean(_) => "bool",
        Value::Null => "null",
        Value::List(_) => "list",
        Value::Dict(_) => "dict",
        Value::Function(..) => "function",
        Value::Class(_) => "class",
        Value::Instance(ref i) => return Ok(Value::String(i.borrow().class_def.name.clone())),
    };
    
    Ok(Value::String(type_name.to_string()))
}
