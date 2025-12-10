use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use crate::ast::environment::NativeFn;

static REGISTRY: OnceLock<RwLock<HashMap<String, NativeFn>>> = OnceLock::new();

pub fn init_registry() {
    let mut map = HashMap::new();

    io::register(&mut map);
    time::register(&mut map);
    random::register(&mut map);
    system::register(&mut map);
    json::register(&mut map);
    http::register(&mut map);
    core::register(&mut map);
    process::register(&mut map);
    path::register(&mut map);
    regex::register(&mut map);
    crypto::register(&mut map);
    date::register(&mut map);
    socket::register(&mut map);

    let _ = REGISTRY.set(RwLock::new(map));
}

pub fn find(name: &str) -> Option<NativeFn> {
    let register_lock = REGISTRY.get()?;

    let reader = register_lock.read().ok()?;

    reader.get(name).cloned()
}

pub fn extend_registry(new_funcs: HashMap<String, NativeFn>) {
    if let Some(registry_lock) = REGISTRY.get() {
        if let Ok(mut writer) = registry_lock.write() {
            println!("[Aegis] Chargement de {} nouvelles fonctions natives...", new_funcs.len());

            writer.extend(new_funcs);
        }
        else {
            eprintln!("[Aegis] Erreur : Impossible d'obtenir le verrou d'écriture sur le registre.");
        }
    }
    else {
        eprintln!("[Aegis] Erreur : Registre non initialisé avant le chargement des plugins.");
    }
}

pub fn get_all_names() -> Vec<String> {
    // On s'assure que le registre est initialisé, sinon on le fait
    if REGISTRY.get().is_none() {
        init_registry();
    }

    let lock = REGISTRY.get().expect("Registry not initialized");
    let reader = lock.read().expect("Registry lock poisoned");

    let mut names: Vec<String> = reader.keys().cloned().collect();
    
    // TRES IMPORTANT : On trie pour garantir le déterminisme entre Compiler et VM
    names.sort(); 
    
    names
}

mod io;
mod time;
mod random;
mod system;
mod json;
mod http;
mod core;
mod process;
mod path;
mod regex;
mod crypto;
mod date;
mod socket;