use crate::ast::Value;
use std::{collections::HashMap, io::{self, Write}};

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("io_clear".to_string(), io_clear);
    map.insert("io_write".to_string(), io_write);
    map.insert("sys_env".to_string(), sys_env);
    map.insert("sys_fail".to_string(), sys_fail);
}

fn io_clear(_: Vec<Value>) -> Result<Value, String> {
    // Petit hack cross-platform pour nettoyer le terminal
    print!("\x1B[2J\x1B[1;1H"); 
    io::stdout().flush().unwrap();
    Ok(Value::Null)
}

fn io_write(args: Vec<Value>) -> Result<Value, String> {
    let s = args[0].as_str()?;
    print!("{}", s); // Pas de println!
    std::io::stdout().flush().unwrap();
    Ok(Value::Null)
}

fn sys_env(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("sys_env attend le nom de la variable".into());
    }

    let key = args[0].as_str()?;

    match std::env::var(key) {
        Ok(val) => Ok(Value::String(val)),
        Err(_) => Ok(Value::Null)
    }
}

fn sys_fail(args: Vec<Value>) -> Result<Value, String> {
    let msg = args.get(0).and_then(|v| v.as_str().ok()).unwrap_or("Assertion failed".to_string());
    return Err(msg);
}
