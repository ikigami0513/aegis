use std::fmt;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::cell::RefCell;
use serde::{Deserialize, Serialize};

use crate::ast::Environment;
use crate::chunk::Chunk;

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionData {
    pub params: Vec<(String, Option<String>)>,
    pub ret_type: Option<String>,
    pub chunk: Chunk,
    pub env: Option<Rc<RefCell<Environment>>>, // SharedEnv
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropertyData {
    pub getter: Option<Value>,
    pub setter: Option<Value>
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceData {
    pub name: String,
    pub methods: HashMap<String, usize>
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassData {
    pub name: String,
    pub parent: Option<String>,
    pub parent_ref: Option<Rc<ClassData>>,

    pub methods: HashMap<String, Value>,
    pub fields: HashMap<String, Value>,
    pub field_types: HashMap<String, String>,
    pub properties: HashMap<String, PropertyData>,

    pub static_methods: HashMap<String, Value>,
    pub static_fields: RefCell<HashMap<String, Value>>,
    pub static_field_types: HashMap<String, String>,
    pub static_properties: HashMap<String, PropertyData>,

    pub is_final: bool,
    pub final_methods: HashSet<String>,

    pub interfaces: Vec<Rc<InterfaceData>>,
    pub interfaces_names: Vec<String>,

    pub visibilities: HashMap<String, Visibility>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceData {
    pub class: Rc<ClassData>, 
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    Enum(Rc<HashMap<String, Value>>),
    Function(Rc<FunctionData>), 
    Class(Rc<ClassData>),
    Instance(Rc<RefCell<InstanceData>>),
    Interface(Rc<InterfaceData>),
    Native(String),
    Range(i64, i64, i64),
    Null
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Integer(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.borrow().iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            },
            Value::Dict(d) => {
                write!(f, "{{")?;
                for (i, (k, v)) in d.borrow().iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            },
            Value::Enum(e) => {
                write!(f, "<Enum ({} variants)>", e.len())
            },
            Value::Function(rc_fn) => {
                 let p_str: Vec<String> = rc_fn.params.iter().map(|p| p.0.clone()).collect();
                 write!(f, "<Function({})>", p_str.join(", "))
            },
            Value::Class { 0: rc_class } => write!(f, "<Class {}>", rc_class.name),
            Value::Instance(inst) => {
                let borrow = inst.borrow();
                // Accès direct au nom de la classe
                write!(f, "<Instance of {}>", borrow.class.name)
            },
            Value::Interface(interface) => write!(f, "<Interface {}>", interface.name),
            Value::Native(name) => write!(f, "<Native Fn {}>", name),
            Value::Range(s, e, step) => write!(f, "{}..{} (step {})", s, e, step),
        }
    }
}

impl Value {
    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Integer(i) => Ok(*i),
            Value::Float(f) => Ok(*f as i64),
            Value::String(s) => s.trim().parse::<i64>().map_err(|_| "Cannot parse string to int".into()),
            _ => Err(format!("Cannot convert {:?} to int", self))
        }
    }

    pub fn as_float(&self) -> Result<f64, String> {
        match self {
            Value::Float(f) => Ok(*f),
            Value::Integer(i) => Ok(*i as f64), 
            _ => Err(format!("Expected Float, got {:?}", self))
        }
    }

    pub fn as_str(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(format!("Expected string, got {:?}", self))
        }
    }

    pub fn as_bool(&self) -> Result<bool, String> {
        match self {
            Value::Boolean(b) => Ok(*b),
            _ => Err(format!("Expected Boolean, got {:?}", self))
        }
    }
}