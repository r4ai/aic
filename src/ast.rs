/// The AST node representing an integer-only expression
#[derive(Debug, Clone, PartialEq)]
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

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
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

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    /// Negation (-)
    Neg,
}

/// The top-level program structure
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// The expression that makes up the program
    pub expr: Expr,
}
