use crate::ast::Value;
use std::{collections::HashMap, thread, time::{self, SystemTime, UNIX_EPOCH}};

pub fn register(map: &mut HashMap<String, super::NativeFn>) {
    map.insert("time_now".to_string(), time_now);
    map.insert("time_sleep".to_string(), time_sleep);
}

fn time_now(_: Vec<Value>) -> Result<Value, String> {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Ok(Value::Integer(since_the_epoch.as_millis() as i64))
}

fn time_sleep(args: Vec<Value>) -> Result<Value, String> {
    let ms = args[0].as_int()?;
    thread::sleep(time::Duration::from_millis(ms as u64));
    Ok(Value::Null)
}
