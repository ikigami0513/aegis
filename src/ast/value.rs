use std::fmt;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceData {
    pub class_name: String,
    // Note: Value is used here, so it's a recursive type definition
    pub fields: HashMap<String, super::Value>, 
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Rc<RefCell<Vec<Value>>>),
    Dict(Rc<RefCell<HashMap<String, Value>>>),
    Instance(Rc<RefCell<InstanceData>>),
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
            Value::Instance(inst) => {
                let data = inst.borrow();
                write!(f, "<Instance of {}>", data.class_name)
            }
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

    pub fn as_str(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(format!("Expected string, got {:?}", self))
        }
    }
}