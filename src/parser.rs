use chumsky::{input::ValueInput, prelude::*};
use logos::Logos;

use crate::{ast, token::Token};

pub fn parser<'a, I>() -> impl Parser<'a, I, ast::Program<'a>, extra::Err<Rich<'a, Token<'a>>>>
where
    I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
{
    let identifier = select! {
        Token::Identifier(value) => value
    };

    let r#type = select! {
        Token::Identifier(value) if value == "i32" => ast::Type::I32,
        Token::Identifier(value) if value == "i64" => ast::Type::I64,
        Token::Identifier(value) if value == "f32" => ast::Type::F32,
        Token::Identifier(value) if value == "f64" => ast::Type::F64,
        Token::Identifier(value) if value == "void" => ast::Type::Void,
        Token::Identifier(value) if value == "string" => ast::Type::String,
    };

    let expr = recursive(|expr| {
        let literal = select! {
            Token::Integer(value) => ast::Expr::IntLit(value.parse().unwrap())
        };

        let primary = choice((
            // literal
            literal,
            // "(" expr ")"
            expr.clone()
                .delimited_by(just(Token::LParen), just(Token::RParen)),
        ));

        let unary = choice((
            // "-" primary
            just(Token::Sub)
                .ignore_then(primary.clone())
                .map(|expr| ast::Expr::UnaryOp {
                    op: ast::UnaryOp::Neg,
                    expr: Box::new(expr),
                }),
            // primary
            primary,
        ));

        let multiplication = unary.clone().foldl(
            choice((
                just(Token::Mul).to(ast::BinOp::Mul),
                just(Token::Div).to(ast::BinOp::Div),
            ))
            .then(unary)
            .repeated(),
            |lhs, (op, rhs)| ast::Expr::BinOp {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            },
        );

        let addition = multiplication.clone().foldl(
            choice((
                just(Token::Add).to(ast::BinOp::Add),
                just(Token::Sub).to(ast::BinOp::Sub),
            ))
            .then(multiplication)
            .repeated(),
            |lhs, (op, rhs)| ast::Expr::BinOp {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            },
        );

        addition
    });

    let statements = recursive(|statements| {
        // expr ";"
        let expr_statement = expr
            .clone()
            .then_ignore(just(Token::Semicolon))
            .map(|expr| ast::Stmt::ExprStmt {
                expr: Box::new(expr),
            });

        // identifier ":" type
        let function_parameter = identifier
            .then_ignore(just(Token::Colon))
            .then(r#type)
            .map(|(name, ty)| ast::FunctionParameter { name, r#type: ty });

        // "(" { function_parameter "," } function_parameter ")"
        let function_parameters = just(Token::LParen)
            .ignore_then(
                function_parameter
                    .separated_by(just(Token::Comma))
                    .collect::<Vec<_>>(),
            )
            .then_ignore(just(Token::RParen));

        // "{" statements [ expr ] "}"
        let function_body = just(Token::LBrace)
            .ignore_then(statements.clone())
            .then_ignore(just(Token::RBrace));

        // "fn" identifier function_parameters "->" type function_body
        let function_declaration = just(Token::FunctionDeclaration)
            .ignore_then(identifier)
            .then(function_parameters)
            .then_ignore(just(Token::RightArrow))
            .then(r#type)
            .then(function_body)
            .map(|(((name, params), return_type), body)| ast::Stmt::FnDecl {
                name,
                params,
                r#type: return_type,
                body,
            });

        let statement = choice((function_declaration, expr_statement));

        statement
            .repeated()
            .collect::<Vec<_>>()
            .then(expr.clone().or_not().map(|expr| {
                expr.map(|expr| ast::Stmt::Expr {
                    expr: Box::new(expr),
                })
            }))
            .map(|(statements, expr)| {
                let mut body: Vec<ast::Stmt> = statements;
                if let Some(expr) = expr {
                    body.push(expr);
                }
                body
            })
    });

    let program = statements
        .then_ignore(end())
        .map(|statements| ast::Program { statements });

    program
}

pub fn parse(src: &str) -> ParseResult<ast::Program, chumsky::error::Rich<'_, Token<'_>>> {
    // Create a logos lexer over the source code
    let token_iter = Token::lexer(src)
        .spanned()
        // Convert logos errors into tokens. We want parsing to be recoverable and not fail at the lexing stage, so
        // we have a dedicated `Token::Error` variant that represents a token error that was previously encountered
        .map(|(tok, span)| match tok {
            // Turn the `Range<usize>` spans logos gives us into chumsky's `SimpleSpan` via `Into`, because it's easier
            // to work with
            Ok(tok) => (tok, span.into()),
            Err(()) => (Token::Error, span.into()),
        });

    // Turn the token iterator into a stream that chumsky can use for things like backtracking
    let token_stream = chumsky::input::Stream::from_iter(token_iter)
        // Tell chumsky to split the (Token, SimpleSpan) stream into its parts so that it can handle the spans for us
        // This involves giving chumsky an 'end of input' span: we just use a zero-width span at the end of the string
        .map((0..src.len()).into(), |(t, s): (_, _)| (t, s));

    // Parse the token stream with our chumsky parser
    parser().parse(token_stream)
}
