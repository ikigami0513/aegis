use std::collections::HashMap;
use std::sync::OnceLock;
use crate::ast::environment::NativeFn;

static REGISTRY: OnceLock<HashMap<String, NativeFn>> = OnceLock::new();

pub fn init_registry() {
    let mut map = HashMap::new();

    io::register(&mut map);
    time::register(&mut map);
    random::register(&mut map);
    system::register(&mut map);
    json::register(&mut map);
    http::register(&mut map);
    core::register(&mut map);

    REGISTRY.set(map).expect("Registry already initialized");
}

pub fn find(name: &str) -> Option<NativeFn> {
    REGISTRY.get()?.get(name).cloned()
}

mod io;
mod time;
mod random;
mod system;
mod json;
mod http;
mod core;