use crate::ast::Value;
use std::collections::HashMap;

// Enregistrement des fonctions dans la VM
pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("math_abs".to_string(), abs);
    map.insert("math_ceil".to_string(), ceil);
    map.insert("math_floor".to_string(), floor);
    map.insert("math_round".to_string(), round);
    map.insert("math_sqrt".to_string(), sqrt);
    map.insert("math_pow".to_string(), pow);
    map.insert("math_sin".to_string(), sin);
    map.insert("math_cos".to_string(), cos);
    map.insert("math_tan".to_string(), tan);
    map.insert("math_acos".to_string(), acos);
    map.insert("math_asin".to_string(), asin);
    map.insert("math_atan".to_string(), atan);
}

// Helper pour convertir Value (Int ou Float) en f64
fn get_number(val: &Value) -> Result<f64, String> {
    match val {
        Value::Integer(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(format!("Expected number, got {}", val)),
    }
}

// --- Implémentations ---

fn abs(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_abs attend 1 argument".into()); }
    match &args[0] {
        Value::Integer(i) => Ok(Value::Integer(i.abs())),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err("math_abs attend un nombre".into()),
    }
}

fn ceil(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_ceil attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Integer(n.ceil() as i64))
}

fn floor(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_floor attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Integer(n.floor() as i64))
}

fn round(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_round attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Integer(n.round() as i64))
}

fn sqrt(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_sqrt attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    if n < 0.0 { return Ok(Value::Null); } // Ou erreur, au choix
    Ok(Value::Float(n.sqrt()))
}

fn pow(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 { return Err("math_pow attend 2 arguments".into()); }
    let base = get_number(&args[0])?;
    let exp = get_number(&args[1])?;
    Ok(Value::Float(base.powf(exp)))
}

fn sin(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_sin attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Float(n.sin()))
}

fn cos(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_cos attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Float(n.cos()))
}

fn tan(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_tan attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Float(n.tan()))
}

fn acos(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_acos attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Float(n.acos()))
}

fn asin(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_asin attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Float(n.asin()))
}

fn atan(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 { return Err("math_atan attend 1 argument".into()); }
    let n = get_number(&args[0])?;
    Ok(Value::Float(n.atan()))
}