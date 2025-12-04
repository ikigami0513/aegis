use std::fmt;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<Value>),
    Dict(HashMap<String, Value>),
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
                for (i, v) in l.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            },
            Value::Dict(d) => {
                write!(f, "{{")?;
                for (i, (k, v)) in d.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Literal(Value),
    Variable(String),
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    FunctionCall(String, Vec<Expression>),
    Equal(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
}

#[derive(Debug, Clone)]
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
}
