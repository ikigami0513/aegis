use rand::Rng;
use crate::ast::Value;
use std::collections::HashMap;

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("rand_int".to_string(), rand_int);
    map.insert("rand_float".to_string(), rand_float);
}

fn rand_int(args: Vec<Value>) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("rand_int attend 2 arguments (min, max)".into());
    }

    let min = args[0].as_int()?;
    let max = args[1].as_int()?;

    if min >= max {
        return Err("min doit être inférieur à max".into());
    }

    let mut rng = rand::thread_rng();
    let val = rng.gen_range(min..max);
    Ok(Value::Integer(val))
}

fn rand_float(_: Vec<Value>) -> Result<Value, String> {
    let mut rng = rand::thread_rng();
    let val: f64 = rng.r#gen();
    Ok(Value::Float(val))
}