use super::value::Value; // Import Value from sibling module
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDefinition {
    pub name: String,
    pub parent: Option<String>,
    pub params: Vec<String>,
    pub methods: HashMap<String, (Vec<String>, Vec<Instruction>)>
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(String),

    // Arithmetic
    Add(Box<Expression>, Box<Expression>),
    Sub(Box<Expression>, Box<Expression>),
    Mul(Box<Expression>, Box<Expression>),
    Div(Box<Expression>, Box<Expression>),
    Modulo(Box<Expression>, Box<Expression>),

    // Comparison
    Equal(Box<Expression>, Box<Expression>),
    NotEqual(Box<Expression>, Box<Expression>),
    LessThan(Box<Expression>, Box<Expression>),
    GreaterThan(Box<Expression>, Box<Expression>),
    LessEqual(Box<Expression>, Box<Expression>),
    GreaterEqual(Box<Expression>, Box<Expression>),

    // Logic
    And(Box<Expression>, Box<Expression>),
    Or(Box<Expression>, Box<Expression>),
    Not(Box<Expression>),

    // Structures & Calls
    FunctionCall(String, Vec<Expression>),
    New(String, Vec<Expression>),
    GetAttr(Box<Expression>, String),
    CallMethod(Box<Expression>, String, Vec<Expression>),
    List(Vec<Expression>),
    Dict(Vec<(String, Expression)>),
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
    Input(String, Expression),
    Class(ClassDefinition),
    SetAttr(Box<Expression>, String, Expression),
    Import(String),
}
