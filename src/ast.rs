use serde::Serialize;

/// Expression
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Expr {
    /// An integer literal
    IntLit(i64),
    /// A binary operation
    BinOp {
        /// The left-hand side expression
        lhs: Box<Expr>,
        /// The operator
        op: BinOp,
        /// The right-hand side expression
        rhs: Box<Expr>,
    },
    /// A unary operation
    UnaryOp {
        /// The operator
        op: UnaryOp,
        /// The expression
        expr: Box<Expr>,
    },
}

/// Binary operator
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum BinOp {
    /// Addition (+)
    Add,
    /// Subtraction (-)
    Sub,
    /// Multiplication (*)
    Mul,
    /// Division (/)
    Div,
}

/// Unary operator
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum UnaryOp {
    /// Negation (-)
    Neg,
}

/// Type
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Type {
    I32,
    I64,
    F32,
    F64,
    Void,
    String,
}

/// Function parameter
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FunctionParameter<'a> {
    /// The name of the parameter
    pub name: &'a str,

    /// The type of the parameter
    pub r#type: Type,
}

/// Statements
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Stmt<'a> {
    /// A function declaration
    FnDecl {
        /// The name of the function
        name: &'a str,
        /// The parameters of the function
        params: Vec<FunctionParameter<'a>>,
        /// The return type of the function
        r#type: Type,
        /// The body of the function
        body: Vec<Stmt<'a>>,
    },

    /// An expression statement
    ExprStmt {
        /// The expression
        expr: Box<Expr>,
    },

    /// An expression
    Expr {
        /// The expression
        expr: Box<Expr>,
    },
}

/// The top-level program structure
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Program<'a> {
    /// The expression that makes up the program
    pub statements: Vec<Stmt<'a>>,
}
