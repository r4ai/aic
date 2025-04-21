use anyhow::Result;
use chumsky::prelude::*;

use crate::ast::{BinOp, Expr, Program, UnaryOp};

/// Parse the input string into a `Program`
pub fn parse(input: &str) -> Result<Program> {
    // Create a parser for expressions
    let expr = recursive(|expr: Recursive<'_, char, Expr, Simple<char>>| {
        // Parse integer literals
        let int = text::int(10)
            .map(|s: String| Expr::IntLit(s.parse().unwrap()))
            .padded();

        // Parse expressions in parentheses
        let atom = int
            .or(expr.clone().delimited_by(just('('), just(')')))
            .or(just('-')
                .padded()
                .ignore_then(expr.clone())
                .map(|expr| Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                }));

        // Parse products and quotients with higher precedence
        let mul_div = just('*')
            .to(BinOp::Mul)
            .or(just('/').to(BinOp::Div))
            .padded();

        let product = atom
            .clone()
            .then(mul_div.then(atom.clone()).repeated())
            .map(|(first, rest)| {
                rest.iter().fold(first, |lhs, (op, rhs)| Expr::BinOp {
                    lhs: Box::new(lhs),
                    op: *op,
                    rhs: Box::new(rhs.clone()),
                })
            });

        // Parse sums and differences with lower precedence
        let add_sub = just('+')
            .to(BinOp::Add)
            .or(just('-').to(BinOp::Sub))
            .padded();

        product
            .clone()
            .then(add_sub.then(product.clone()).repeated())
            .map(|(first, rest)| {
                rest.iter().fold(first, |lhs, (op, rhs)| Expr::BinOp {
                    lhs: Box::new(lhs),
                    op: *op,
                    rhs: Box::new(rhs.clone()),
                })
            })
    });

    // Parse the entire input as an expression and convert to a program
    let parser = expr.then_ignore(end());

    match parser.parse(input.trim()) {
        Ok(expr) => Ok(Program { expr }),
        Err(errors) => {
            let error_msg = errors
                .into_iter()
                .map(|e| format!("{:?}", e))
                .collect::<Vec<_>>()
                .join(", ");
            Err(anyhow::anyhow!("Parse error: {}", error_msg))
        }
    }
}
