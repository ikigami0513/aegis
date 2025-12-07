// src/ast/env.rs
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
// Note : on utilise super:: pour remonter dans le module AST
use super::{ClassDefinition, Value}; 

pub type SharedEnv = Rc<RefCell<Environment>>;

pub type NativeFn = fn(Vec<Value>) -> Result<Value, String>;

#[derive(Debug, PartialEq)]
pub struct Environment {
    pub parent: Option<SharedEnv>, // Doit être pub pour l'accès
    pub variables: HashMap<String, Value>,
    pub classes: HashMap<String, ClassDefinition>,
    pub natives: HashMap<String, NativeFn>
}

impl Environment {
    pub fn new_global() -> SharedEnv {
        Rc::new(RefCell::new(Environment {
            parent: None,
            variables: HashMap::new(),
            classes: HashMap::new(),
            natives: HashMap::new()
        }))
    }

    pub fn new_child(parent: SharedEnv) -> SharedEnv {
        Rc::new(RefCell::new(Environment {
            parent: Some(parent),
            variables: HashMap::new(),
            classes: HashMap::new(),
            natives: HashMap::new()
        }))
    }

    pub fn set_variable(&mut self, name: String, value: Value) {
        if self.variables.contains_key(&name) {
            self.variables.insert(name, value);
            return;
        }

        if let Some(parent) = &self.parent {
            let exists_in_parent = parent.borrow().get_variable(&name).is_some();
            if exists_in_parent {
                parent.borrow_mut().set_variable(name, value);
                return;
            }
        }

        self.variables.insert(name, value);
    }

    pub fn get_variable(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.variables.get(name) {
            return Some(val.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.borrow().get_variable(name);
        }
        None
    }
}
