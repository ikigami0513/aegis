use crate::ast::Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("json_parse".to_string(), json_parse);
    map.insert("json_stringify".to_string(), json_stringify);
}

// Conversion : serde_json::Value (Externe) -> crate::ast::Value (Interne Aegis)
fn serde_to_aegis(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(b),
        serde_json::Value::Number(n) => {
            if n.is_i64() { Value::Integer(n.as_i64().unwrap()) }
            else { Value::Float(n.as_f64().unwrap()) }
        },
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => {
            let list = arr.into_iter().map(serde_to_aegis).collect();
            Value::List(Rc::new(RefCell::new(list)))
        },
        serde_json::Value::Object(map) => {
            let mut dict = HashMap::new();
            for (k, v) in map {
                dict.insert(k, serde_to_aegis(v));
            }
            Value::Dict(Rc::new(RefCell::new(dict)))
        }
    }
}

// Conversion inverse (pour envoyer du JSON ou stringify) : Aegis -> Serde
// (Version simplifiée qui retourne string direct pour l'instant)
#[allow(dead_code)]
fn aegis_to_json_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::String(s) => format!("\"{}\"", s), // Ajout des quotes pour JSON valide
        Value::List(l) => {
            let items: Vec<String> = l.borrow().iter().map(|i| aegis_to_json_string(i)).collect();
            format!("[{}]", items.join(", "))
        },
        Value::Dict(d) => {
            let items: Vec<String> = d.borrow().iter().map(|(k, v)| {
                format!("\"{}\": {}", k, aegis_to_json_string(v))
            }).collect();
            format!("{{{}}}", items.join(", "))
        },
        _ => "\"unsupported\"".to_string()
    }
}


fn json_parse(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("json_parse attend une chaine".into());
    }

    let json_str = args[0].as_str()?;

    let serde_val: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Erreur Parsing JSON: {}", e))?;

    Ok(serde_to_aegis(serde_val))
}

fn json_stringify(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("json_parse attend une chaine".into());
    }
    let json_str = args[0].as_str()?;

    let serde_val: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| format!("Erreur Parsing JSON: {}", e))?;

    Ok(serde_to_aegis(serde_val))
}
