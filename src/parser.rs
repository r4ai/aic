use chumsky::{input::ValueInput, prelude::*};
use logos::Logos;

use crate::{ast, token::Token};

pub fn parser<'a, I>() -> impl Parser<'a, I, ast::Program, extra::Err<Rich<'a, Token<'a>>>>
where
    I: ValueInput<'a, Token = Token<'a>, Span = SimpleSpan>,
{
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

    expr.then_ignore(end()).map(|expr| ast::Program { expr })
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
