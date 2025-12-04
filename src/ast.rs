use std::fmt;
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDefinition {
    pub name: String,
    pub parent: Option<String>,
    pub params: Vec<String>,
    pub methods: HashMap<String, (Vec<String>, Vec<Instruction>)>
}

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceData {
    pub class_name: String,
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
    /// Helper pour convertir proprement en i64 (depuis int, float ou string)
    pub fn as_int(&self) -> Result<i64, String> {
        match self {
            Value::Integer(i) => Ok(*i),
            Value::Float(f) => Ok(*f as i64),
            Value::String(s) => s.trim().parse::<i64>().map_err(|_| "Impossible de convertir la chaîne en entier".into()),
            _ => Err(format!("Impossible de convertir {:?} en entier", self))
        }
    }

    /// Helper pour récupérer la String interne ou erreur
    pub fn as_str(&self) -> Result<String, String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            _ => Err(format!("Attendu une chaîne de caractères, obtenu {:?}", self))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(String),

    // Arithmétique
    Add(Box<Expression>, Box<Expression>), // +
    Sub(Box<Expression>, Box<Expression>), // -
    Mul(Box<Expression>, Box<Expression>), // *
    Div(Box<Expression>, Box<Expression>), // /

    // Comparaisons Complètes
    Equal(Box<Expression>, Box<Expression>),        // ==
    NotEqual(Box<Expression>, Box<Expression>),     // !=
    LessThan(Box<Expression>, Box<Expression>),     // <
    GreaterThan(Box<Expression>, Box<Expression>),  // >
    LessEqual(Box<Expression>, Box<Expression>),    // <=
    GreaterEqual(Box<Expression>, Box<Expression>), // >=

    // Logique
    And(Box<Expression>, Box<Expression>), // &&
    Or(Box<Expression>, Box<Expression>),  // ||
    Not(Box<Expression>),                  // !

    FunctionCall(String, Vec<Expression>),
    New(String, Vec<Expression>), // new Class(args)
    GetAttr(Box<Expression>, String), // obj.attr
    CallMethod(Box<Expression>, String, Vec<Expression>), // obj.method(args)
    List(Vec<Expression>), // [expr, expr]
    Dict(Vec<(String, Expression)>), // { key: expr, key: expr }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Set(String, Expression),
    Print(Expression),
    If {
        condition: Expression,
        body: Vec<Instruction>,
        else_body: Vec<Instruction>
    },
    While {
        condition: Expression,
        body: Vec<Instruction>
    },
    ForRange {
        var_name: String,
        start: Expression,
        end: Expression,
        step: Expression,
        body: Vec<Instruction>,
    },
    Return(Expression),
    ExpressionStatement(Expression),
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Instruction>
    },
    // ["input", "var_name", "Prompt text"]
    Input(String, Expression),
    Class(ClassDefinition),
    SetAttr(Box<Expression>, String, Expression), // obj.attr = val
}
