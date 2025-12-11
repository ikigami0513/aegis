use super::value::Value; // Import Value from sibling module
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassDefinition {
    pub name: String,
    pub parent: Option<String>,
    pub methods: HashMap<String, (Vec<(String, Option<String>)>, Vec<Statement>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Value),
    Variable(String),
    Function {
        params: Vec<(String, Option<String>)>,
        ret_type: Option<String>,
        body: Vec<Statement>
    },

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
    BitAnd(Box<Expression>, Box<Expression>),
    BitOr(Box<Expression>, Box<Expression>),
    BitXor(Box<Expression>, Box<Expression>),
    ShiftLeft(Box<Expression>, Box<Expression>),
    ShiftRight(Box<Expression>, Box<Expression>),

    // Structures & Calls
    Call(Box<Expression>, Vec<Expression>),
    New(Box<Expression>, Vec<Expression>),
    GetAttr(Box<Expression>, String),
    CallMethod(Box<Expression>, String, Vec<Expression>),
    List(Vec<Expression>),
    Dict(Vec<(String, Expression)>),
    SuperCall(String, Vec<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Set(String, Option<String>, Expression),
    Print(Expression),
    If {
        condition: Expression,
        body: Vec<Statement>,
        else_body: Vec<Statement>
    },
    While {
        condition: Expression,
        body: Vec<Statement>
    },
    ForRange {
        var_name: String,
        start: Expression,
        end: Expression,
        step: Expression,
        body: Vec<Statement>,
    },
    Return(Expression),
    ExpressionStatement(Expression),
    Function {
        name: String,
        params: Vec<(String, Option<String>)>,
        ret_type: Option<String>,
        body: Vec<Statement>
    },
    Input(String, Expression),
    Class(ClassDefinition),
    SetAttr(Box<Expression>, String, Expression),
    Import(String),
    TryCatch {
        try_body: Vec<Statement>,
        error_var: String,
        catch_body: Vec<Statement>,
    },
    Switch {
        value: Expression,
        cases: Vec<(Expression, Vec<Statement>)>, 
        default: Vec<Statement>,
    },
    Namespace {
        name: String,
        body: Vec<Statement>
    },
    Throw(Expression),
    Continue
}

#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub kind: Instruction,
    pub line: usize
}
