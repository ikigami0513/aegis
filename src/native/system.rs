use crate::ast::Value;
use std::{cell::RefCell, collections::HashMap, io::{self, Write}, rc::Rc};

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("io_clear".to_string(), io_clear);
    map.insert("sys_env".to_string(), sys_env);
    map.insert("sys_args".to_string(), sys_args);
}

fn io_clear(_: Vec<Value>) -> Result<Value, String> {
    // Petit hack cross-platform pour nettoyer le terminal
    print!("\x1B[2J\x1B[1;1H"); 
    io::stdout().flush().unwrap();
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

fn sys_args(_: Vec<Value>) -> Result<Value, String> {
    // On récupère tous les arguments
    let all_args: Vec<String> = std::env::args().collect();

    // On cherche à partir d'où commencent les arguments du script
    // Généralement, c'est après le nom du script .aeg
    let mut script_args = Vec::new();
    let mut found_script = false;

    for arg in all_args {
        if found_script {
            script_args.push(Value::String(arg));
        }
        else if arg.ends_with(".aeg") {
            found_script = true;
        }
    }

    // Si on est en mode REPL (pas de .aeg), on renvoie tout sauf l'exécutable
    if !found_script && std::env::args().len() > 1 {
        // Fallback simple
        script_args = std::env::args().skip(1).map(Value::String).collect();
    }

    Ok(Value::List(Rc::new(RefCell::new(script_args))))
}
