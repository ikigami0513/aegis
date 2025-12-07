use crate::ast::Value;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("io_read".to_string(), io_read);
    map.insert("io_write".to_string(), io_write);
    map.insert("io_append".to_string(), io_append);
    map.insert("io_exists".to_string(), io_exists);
    map.insert("io_delete".to_string(), io_delete);
}

fn io_read(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("io_read attend 1 argument".into());
    }

    let path = args[0].as_str()?;

    match fs::read_to_string(&path) {
        Ok(content) => Ok(Value::String(content)),
        Err(_) => Ok(Value::Null)
    }
}

fn io_write(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("io_write attend 2 arguments".into());
    }

    let path = args[0].as_str()?;
    let content = args[1].as_str()?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(Value::Boolean(true))
}

fn io_append(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("io_append attend 2 arguments.".into());
    }

    let path = args[0].as_str()?;
    let content = args[1].as_str()?;

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|e| format!("Erreur ouverture fichier: {}", e))?;

    write!(file, "{}", content).map_err(|e| format!("Erreur append: {}", e))?;
    Ok(Value::Boolean(true))
}

fn io_exists(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("io_exists attend 1 argument (path).".into());
    }

    let path = args[0].as_str()?;

    Ok(Value::Boolean(Path::new(&path).exists()))
}

fn io_delete(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("io_delete attend 1 argument (path).".into());
    }

    let path = args[0].as_str()?;
    if Path::new(&path).exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
        return Ok(Value::Boolean(true));
    }
    return Ok(Value::Boolean(false));
}