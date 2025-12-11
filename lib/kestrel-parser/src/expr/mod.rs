//! Expression parsing
//!
//! This module provides parsing for Kestrel expressions.
//! Currently supports:
//! - Unit expression: ()

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::block::{BlockItem, CodeBlockData, emit_code_block};
use crate::common::skip_trivia;
use crate::event::{EventSink, TreeBuilder};
use crate::stmt::{StmtVariant, VariableDeclarationData};
use crate::ty::{TyVariant, emit_ty_variant, ty_parser};

/// Represents an expression
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl Expression {
    /// Create a new Expression from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the kind of this expression
    pub fn kind(&self) -> SyntaxKind {
        self.syntax
            .children()
            .next()
            .map(|child| child.kind())
            .unwrap_or(SyntaxKind::Error)
    }

    /// Check if this is a unit expression
    pub fn is_unit(&self) -> bool {
        self.kind() == SyntaxKind::ExprUnit
    }

    /// Check if this is an integer literal
    pub fn is_integer(&self) -> bool {
        self.kind() == SyntaxKind::ExprInteger
    }

    /// Check if this is a float literal
    pub fn is_float(&self) -> bool {
        self.kind() == SyntaxKind::ExprFloat
    }

    /// Check if this is a string literal
    pub fn is_string(&self) -> bool {
        self.kind() == SyntaxKind::ExprString
    }

    /// Check if this is a boolean literal
    pub fn is_bool(&self) -> bool {
        self.kind() == SyntaxKind::ExprBool
    }

    /// Check if this is an array literal
    pub fn is_array(&self) -> bool {
        self.kind() == SyntaxKind::ExprArray
    }

    /// Check if this is a tuple literal
    pub fn is_tuple(&self) -> bool {
        self.kind() == SyntaxKind::ExprTuple
    }

    /// Check if this is a grouping expression
    pub fn is_grouping(&self) -> bool {
        self.kind() == SyntaxKind::ExprGrouping
    }

    /// Check if this is a path expression
    pub fn is_path(&self) -> bool {
        self.kind() == SyntaxKind::ExprPath
    }

    /// Check if this is a unary expression
    pub fn is_unary(&self) -> bool {
        self.kind() == SyntaxKind::ExprUnary
    }

    /// Check if this is a null literal
    pub fn is_null(&self) -> bool {
        self.kind() == SyntaxKind::ExprNull
    }

    /// Check if this is a call expression
    pub fn is_call(&self) -> bool {
        self.kind() == SyntaxKind::ExprCall
    }

    /// Check if this is an assignment expression
    pub fn is_assignment(&self) -> bool {
        self.kind() == SyntaxKind::ExprAssignment
    }

    /// Check if this is an if expression
    pub fn is_if(&self) -> bool {
        self.kind() == SyntaxKind::ExprIf
    }

    /// Check if this is a while expression
    pub fn is_while(&self) -> bool {
        self.kind() == SyntaxKind::ExprWhile
    }

    /// Check if this is a loop expression
    pub fn is_loop(&self) -> bool {
        self.kind() == SyntaxKind::ExprLoop
    }

    /// Check if this is a break expression
    pub fn is_break(&self) -> bool {
        self.kind() == SyntaxKind::ExprBreak
    }

    /// Check if this is a continue expression
    pub fn is_continue(&self) -> bool {
        self.kind() == SyntaxKind::ExprContinue
    }

    /// Check if this is a return expression
    pub fn is_return(&self) -> bool {
        self.kind() == SyntaxKind::ExprReturn
    }
}

/// Parser for type arguments with full type support: [T, (A, B), [Int], (X) -> Y]
/// Returns (lbracket, types, rbracket)
fn full_type_args_parser() -> impl Parser<Token, TypeArgsData, Error = Simple<Token>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBracket).map_with_span(|_, span| Span::from(span)))
        .then(
            ty_parser()
                .separated_by(skip_trivia().ignore_then(just(Token::Comma)))
                .allow_trailing(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RBracket).map_with_span(|_, span| Span::from(span)))
        .map(|((lbracket, args), rbracket)| TypeArgsData {
            lbracket,
            args,
            rbracket,
        })
}

/// A path segment with optional type arguments
/// Syntax: Ident or Ident[T, U]
#[derive(Debug, Clone)]
pub struct PathSegmentData {
    /// The identifier name
    pub name: Span,
    /// Optional type arguments: [T, U]
    pub type_args: Option<TypeArgsData>,
}

/// Type arguments data for expressions
/// Syntax: [T, U, V] where each can be any type (path, tuple, function, array)
#[derive(Debug, Clone)]
pub struct TypeArgsData {
    /// Left bracket span
    pub lbracket: Span,
    /// The type arguments (full types, not just paths)
    pub args: Vec<TyVariant>,
    /// Right bracket span
    pub rbracket: Span,
}

/// A call argument with optional label
#[derive(Debug, Clone)]
pub struct CallArg {
    /// Optional label (identifier before colon)
    pub label: Option<Span>,
    /// The colon after the label (if labeled)
    pub colon: Option<Span>,
    /// The argument expression
    pub value: ExprVariant,
}

/// Internal enum to distinguish between expression variants during parsing
#[derive(Debug, Clone)]
pub enum ExprVariant {
    /// Unit expression: ()
    Unit(Span, Span), // (lparen, rparen)
    /// Integer literal: 42, 0xFF, 0b1010, 0o17
    Integer(Span),
    /// Float literal: 3.14, 1.0e10
    Float(Span),
    /// String literal: "hello"
    String(Span),
    /// Boolean literal: true, false
    Bool(Span),
    /// Null literal: null
    Null(Span),
    /// Array literal: [1, 2, 3]
    Array(Span, Vec<ExprVariant>, Vec<Span>, Span), // (lbracket, elements, commas, rbracket)
    /// Tuple literal: (1, 2, 3)
    Tuple(Span, Vec<ExprVariant>, Vec<Span>, Span), // (lparen, elements, commas, rparen)
    /// Grouping expression: (expr)
    Grouping(Span, Box<ExprVariant>, Span), // (lparen, inner, rparen)
    /// Path expression: a.b.c or a[T].b[U].c (used for initial path parsing)
    Path {
        segments: Vec<PathSegmentData>,
        dots: Vec<Span>,
    },
    /// Member access expression: base.member or base.member[T]
    MemberAccess {
        base: Box<ExprVariant>,
        dot: Span,
        member: Span,
        type_args: Option<TypeArgsData>,
    },
    /// Tuple index expression: tuple.0, tuple.1
    TupleIndex {
        base: Box<ExprVariant>,
        dot: Span,
        index: Span,
    },
    /// Unary prefix expression: -expr, !expr, not expr, +expr
    Unary(Token, Span, Box<ExprVariant>), // (operator_token, operator_span, operand)
    /// Postfix expression: expr!
    Postfix {
        operand: Box<ExprVariant>,
        operator: Token,
        operator_span: Span,
    },
    /// Binary expression: a + b (flat, no precedence applied yet)
    Binary {
        lhs: Box<ExprVariant>,
        operator: Token,
        operator_span: Span,
        rhs: Box<ExprVariant>,
    },
    /// Call expression: callee(args)
    Call {
        callee: Box<ExprVariant>,
        lparen: Span,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        rparen: Span,
    },
    /// Assignment expression: lhs = rhs
    Assignment {
        lhs: Box<ExprVariant>,
        equals: Span,
        rhs: Box<ExprVariant>,
    },
    /// If expression: if condition { then } else { else }
    If {
        if_span: Span,
        condition: Box<ExprVariant>,
        then_block: CodeBlockData,
        else_clause: Option<ElseClause>,
    },
    /// While expression: label: while condition { body }
    While {
        label: Option<LabelData>,
        while_span: Span,
        condition: Box<ExprVariant>,
        body: CodeBlockData,
    },
    /// Loop expression: label: loop { body }
    Loop {
        label: Option<LabelData>,
        loop_span: Span,
        body: CodeBlockData,
    },
    /// Break expression: break or break label
    Break {
        break_span: Span,
        label: Option<Span>,
    },
    /// Continue expression: continue or continue label
    Continue {
        continue_span: Span,
        label: Option<Span>,
    },
    /// Return expression: return or return expr
    Return {
        return_span: Span,
        value: Option<Box<ExprVariant>>,
    },
}

/// Loop label data: label:
#[derive(Debug, Clone)]
pub struct LabelData {
    pub name: Span,
    pub colon: Span,
}

/// Else clause: either a block or another if expression
#[derive(Debug, Clone)]
pub enum ElseClause {
    /// else { block }
    Block {
        else_span: Span,
        block: CodeBlockData,
    },
    /// else if ...
    ElseIf {
        else_span: Span,
        if_expr: Box<ExprVariant>,
    },
}

/// Parser for expressions
///
/// Supports:
/// - Unit expression: ()
/// - Integer literals: 42, 0xFF, 0b1010, 0o17
/// - Float literals: 3.14, 1.0e10
/// - String literals: "hello"
/// - Boolean literals: true, false
/// - Array literals: [1, 2, 3]
/// - Tuple literals: (1, 2, 3)
/// - Grouping: (expr)
pub fn expr_parser() -> impl Parser<Token, ExprVariant, Error = Simple<Token>> + Clone {
    recursive(|expr| {
        // Integer literal
        let integer = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::Integer => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .map(ExprVariant::Integer);

        // Float literal
        let float = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::Float => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .map(ExprVariant::Float);

        // String literal
        let string = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::String => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .map(ExprVariant::String);

        // Boolean literal
        let boolean = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::Boolean => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .map(ExprVariant::Bool);

        // Null literal
        let null = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::Null => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .map(ExprVariant::Null);

        // Path segment: identifier optionally followed by type args [T, U]
        let path_segment = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::Identifier => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .then(full_type_args_parser().or_not())
            .map(|(name, type_args)| PathSegmentData { name, type_args });

        // Path expression: a.b.c or a[T].b[U].c
        let path = path_segment
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Dot).map_with_span(|_, span| Span::from(span)))
                    .then(path_segment.clone())
                    .repeated(),
            )
            .map(|(first, rest)| {
                let mut segments = vec![first];
                let mut dots = Vec::new();
                for (dot, segment) in rest {
                    dots.push(dot);
                    segments.push(segment);
                }
                ExprVariant::Path { segments, dots }
            });

        // Array literal: [elem, elem, ...]
        let array = skip_trivia()
            .ignore_then(just(Token::LBracket).map_with_span(|_, span| Span::from(span)))
            .then(
                expr.clone()
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Comma).map_with_span(|_, span| Span::from(span)))
                            .then(skip_trivia().ignore_then(expr.clone()))
                            .repeated(),
                    )
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Comma).map_with_span(|_, span| Span::from(span)))
                            .or_not(),
                    )
                    .map(|((first, rest), trailing)| {
                        let mut elements = vec![first];
                        let mut commas = Vec::new();
                        for (comma, elem) in rest {
                            commas.push(comma);
                            elements.push(elem);
                        }
                        if let Some(tc) = trailing {
                            commas.push(tc);
                        }
                        (elements, commas)
                    })
                    .or_not(),
            )
            .then(skip_trivia().ignore_then(just(Token::RBracket).map_with_span(|_, span| Span::from(span))))
            .map(|((lbracket, contents), rbracket)| {
                let (elements, commas) = contents.unwrap_or_else(|| (vec![], vec![]));
                ExprVariant::Array(lbracket, elements, commas, rbracket)
            });

        // Parenthesized expressions: (), (expr), (expr,), (expr, expr, ...)
        let paren_expr = skip_trivia()
            .ignore_then(just(Token::LParen).map_with_span(|_, span| Span::from(span)))
            .then(
                // Empty parens: ()
                skip_trivia()
                    .ignore_then(just(Token::RParen).map_with_span(|_, span| Span::from(span)))
                    .map(|rparen| ParenContent::Unit(rparen))
                    .or(
                        // Non-empty: expr followed by optional comma and more
                        expr.clone()
                            .then(
                                skip_trivia()
                                    .ignore_then(just(Token::Comma).map_with_span(|_, span| Span::from(span)))
                                    .then(
                                        skip_trivia()
                                            .ignore_then(expr.clone())
                                            .then(
                                                skip_trivia()
                                                    .ignore_then(
                                                        just(Token::Comma)
                                                            .map_with_span(|_, span| Span::from(span)),
                                                    )
                                                    .then(skip_trivia().ignore_then(expr.clone()))
                                                    .repeated(),
                                            )
                                            .then(
                                                skip_trivia()
                                                    .ignore_then(
                                                        just(Token::Comma)
                                                            .map_with_span(|_, span| Span::from(span)),
                                                    )
                                                    .or_not(),
                                            )
                                            .map(|((second, rest), trailing)| {
                                                let mut elements = vec![second];
                                                let mut commas = Vec::new();
                                                for (comma, elem) in rest {
                                                    commas.push(comma);
                                                    elements.push(elem);
                                                }
                                                if let Some(tc) = trailing {
                                                    commas.push(tc);
                                                }
                                                (elements, commas)
                                            })
                                            .or_not(),
                                    )
                                    .or_not(),
                            )
                            .then(
                                skip_trivia()
                                    .ignore_then(just(Token::RParen).map_with_span(|_, span| Span::from(span))),
                            )
                            .map(|((first, comma_rest), rparen)| {
                                match comma_rest {
                                    None => {
                                        // (expr) - grouping
                                        ParenContent::Grouping(first, rparen)
                                    }
                                    Some((first_comma, None)) => {
                                        // (expr,) - single-element tuple
                                        ParenContent::Tuple(vec![first], vec![first_comma], rparen)
                                    }
                                    Some((first_comma, Some((more_elems, more_commas)))) => {
                                        // (expr, expr, ...) - multi-element tuple
                                        let mut elements = vec![first];
                                        elements.extend(more_elems);
                                        let mut commas = vec![first_comma];
                                        commas.extend(more_commas);
                                        ParenContent::Tuple(elements, commas, rparen)
                                    }
                                }
                            }),
                    ),
            )
            .map(|(lparen, content)| match content {
                ParenContent::Unit(rparen) => ExprVariant::Unit(lparen, rparen),
                ParenContent::Grouping(inner, rparen) => {
                    ExprVariant::Grouping(lparen, Box::new(inner), rparen)
                }
                ParenContent::Tuple(elements, commas, rparen) => {
                    ExprVariant::Tuple(lparen, elements, commas, rparen)
                }
            });

        // Primary expressions without if (no nested expr references, for conditions)
        // These are used for if conditions to avoid infinite recursion
        let condition_primary = float
            .clone()
            .or(integer.clone())
            .or(string.clone())
            .or(boolean.clone())
            .or(null.clone())
            .or(path.clone());

        // Argument parser for call expressions: unlabeled or labeled
        // labeled: identifier: expr
        // unlabeled: expr
        let argument = skip_trivia().ignore_then(
            // Try labeled argument: identifier: expr
            filter_map(|span, token| match token {
                Token::Identifier => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            })
            .then(skip_trivia().ignore_then(just(Token::Colon).map_with_span(|_, span| Span::from(span))))
            .then(skip_trivia().ignore_then(expr.clone()))
            .map(|((label, colon), value)| CallArg {
                label: Some(label),
                colon: Some(colon),
                value,
            })
            // Or unlabeled: just expr
            .or(expr.clone().map(|value| CallArg {
                label: None,
                colon: None,
                value,
            })),
        );

        // Argument list: (arg, arg, ...)
        let arg_list = skip_trivia()
            .ignore_then(just(Token::LParen).map_with_span(|_, span| Span::from(span)))
            .then(
                // Empty arg list
                skip_trivia()
                    .ignore_then(just(Token::RParen).map_with_span(|_, span| Span::from(span)))
                    .map(|rparen| (vec![], vec![], rparen))
                    .or(
                        // Non-empty: arg followed by optional commas and more
                        argument
                            .clone()
                            .then(
                                skip_trivia()
                                    .ignore_then(just(Token::Comma).map_with_span(|_, span| Span::from(span)))
                                    .then(skip_trivia().ignore_then(argument.clone()))
                                    .repeated(),
                            )
                            .then(
                                skip_trivia()
                                    .ignore_then(just(Token::Comma).map_with_span(|_, span| Span::from(span)))
                                    .or_not(),
                            )
                            .then(
                                skip_trivia()
                                    .ignore_then(just(Token::RParen).map_with_span(|_, span| Span::from(span))),
                            )
                            .map(|(((first, rest), trailing), rparen)| {
                                let mut arguments = vec![first];
                                let mut commas = Vec::new();
                                for (comma, arg) in rest {
                                    commas.push(comma);
                                    arguments.push(arg);
                                }
                                if let Some(tc) = trailing {
                                    commas.push(tc);
                                }
                                (arguments, commas, rparen)
                            }),
                    ),
            )
            .map(|(lparen, (arguments, commas, rparen))| PostfixOp::Call {
                lparen,
                arguments,
                commas,
                rparen,
            });

        // Member access: .identifier or .identifier[T] or tuple index: .0, .1
        let member_access = skip_trivia()
            .ignore_then(just(Token::Dot).map_with_span(|_, span| Span::from(span)))
            .then(
                skip_trivia().ignore_then(filter_map(|span, token| match token {
                    Token::Identifier => Ok((token, Span::from(span))),
                    Token::Integer => Ok((token, Span::from(span))),
                    _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
                })),
            )
            .then(full_type_args_parser().or_not())
            .map(|((dot, (token, span)), type_args)| match token {
                Token::Integer => PostfixOp::TupleIndex { dot, index: span },
                _ => PostfixOp::MemberAccess {
                    dot,
                    member: span,
                    type_args,
                },
            });

        // Postfix unwrap operator: expr!
        let postfix_bang = skip_trivia()
            .ignore_then(just(Token::Bang).map_with_span(|tok, span| (tok, Span::from(span))))
            .map(|(tok, span)| PostfixOp::PostfixOperator {
                operator: tok,
                operator_span: span,
            });

        // Postfix operations: can be call (args), member access .identifier, or postfix !
        // These can be chained: a.b().c.d()!
        let postfix_op = arg_list.clone().or(member_access).or(postfix_bang);

        // Postfix expression for conditions: using condition_primary
        // Supports member access (.foo), .foo[T], tuple index (.0), and function calls (args use full expr)
        let condition_member_access = skip_trivia()
            .ignore_then(just(Token::Dot).map_with_span(|_, span| Span::from(span)))
            .then(
                skip_trivia().ignore_then(filter_map(|span, token| match token {
                    Token::Identifier => Ok((token, Span::from(span))),
                    Token::Integer => Ok((token, Span::from(span))),
                    _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
                })),
            )
            .then(full_type_args_parser().or_not())
            .map(|((dot, (token, span)), type_args)| match token {
                Token::Integer => ConditionPostfixOp::TupleIndex { dot, index: span },
                _ => ConditionPostfixOp::MemberAccess {
                    dot,
                    member: span,
                    type_args,
                },
            });

        // Function call arguments use the full expr parser (filled in via recursive reference)
        let condition_call = arg_list.clone().map(|op| match op {
            PostfixOp::Call {
                lparen,
                arguments,
                commas,
                rparen,
            } => ConditionPostfixOp::Call {
                lparen,
                arguments,
                commas,
                rparen,
            },
            _ => unreachable!(),
        });

        let condition_postfix_op = condition_member_access.or(condition_call);

        let condition_postfix = condition_primary
            .clone()
            .then(condition_postfix_op.repeated())
            .map(|(base, ops)| {
                ops.into_iter().fold(base, |acc, op| match op {
                    ConditionPostfixOp::MemberAccess {
                        dot,
                        member,
                        type_args,
                    } => ExprVariant::MemberAccess {
                        base: Box::new(acc),
                        dot,
                        member,
                        type_args,
                    },
                    ConditionPostfixOp::TupleIndex { dot, index } => ExprVariant::TupleIndex {
                        base: Box::new(acc),
                        dot,
                        index,
                    },
                    ConditionPostfixOp::Call {
                        lparen,
                        arguments,
                        commas,
                        rparen,
                    } => ExprVariant::Call {
                        callee: Box::new(acc),
                        lparen,
                        arguments,
                        commas,
                        rparen,
                    },
                })
            });

        // Prefix unary operators: -, +, !, not
        let unary_op = skip_trivia().ignore_then(
            just(Token::Minus)
                .map_with_span(|tok, span| (tok, Span::from(span)))
                .or(just(Token::Plus).map_with_span(|tok, span| (tok, Span::from(span))))
                .or(just(Token::Bang).map_with_span(|tok, span| (tok, Span::from(span))))
                .or(just(Token::Not).map_with_span(|tok, span| (tok, Span::from(span)))),
        );

        // Binary operator parser - matches any binary operator token
        let binary_op = skip_trivia().ignore_then(filter_map(|span, token: Token| {
            if is_binary_operator(&token) {
                Ok((token, Span::from(span)))
            } else {
                Err(Simple::expected_input_found(span, vec![], Some(token)))
            }
        }));

        // Unary expression for conditions (no nested expr)
        let condition_unary = unary_op
            .clone()
            .then(condition_postfix.clone())
            .map(|((tok, span), operand)| ExprVariant::Unary(tok, span, Box::new(operand)));

        // Non-assignment expression for conditions
        let condition_non_assignment = condition_unary.or(condition_postfix.clone());

        // Binary expression for conditions (no nested expr, no if)
        let condition_binary = condition_non_assignment
            .clone()
            .then(
                binary_op
                    .clone()
                    .then(condition_non_assignment.clone())
                    .repeated(),
            )
            .map(|(first, rest)| {
                rest.into_iter()
                    .fold(first, |lhs, ((op_token, op_span), rhs)| {
                        ExprVariant::Binary {
                            lhs: Box::new(lhs),
                            operator: op_token,
                            operator_span: op_span,
                            rhs: Box::new(rhs),
                        }
                    })
            });

        // Inline variable declaration parser (uses expr for initializer)
        let inline_var_decl = {
            let expr_for_init = expr.clone();
            skip_trivia()
                .ignore_then(
                    just(Token::Let)
                        .map_with_span(|_, span| (Span::from(span), false))
                        .or(just(Token::Var).map_with_span(|_, span| (Span::from(span), true))),
                )
                .then(
                    skip_trivia().ignore_then(filter_map(|span, token| match token {
                        Token::Identifier => Ok(Span::from(span)),
                        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
                    })),
                )
                .then(
                    // Optional type annotation: : Type
                    skip_trivia()
                        .ignore_then(just(Token::Colon).map_with_span(|_, span| Span::from(span)))
                        .then(ty_parser())
                        .or_not(),
                )
                .then(
                    // Optional initializer: = expr
                    skip_trivia()
                        .ignore_then(just(Token::Equals).map_with_span(|_, span| Span::from(span)))
                        .then(expr_for_init)
                        .or_not(),
                )
                .then(
                    skip_trivia().ignore_then(just(Token::Semicolon).map_with_span(|_, span| Span::from(span))),
                )
                .map(
                    |(
                        (
                            (((mutability_span, is_mutable), name_span), type_annotation),
                            initializer,
                        ),
                        semicolon,
                    )| {
                        StmtVariant::VariableDeclaration(VariableDeclarationData {
                            mutability_span,
                            is_mutable,
                            name_span,
                            type_annotation,
                            initializer,
                            semicolon,
                        })
                    },
                )
        };

        // Inline expression statement parser (uses expr)
        let inline_expr_stmt = {
            let expr_for_stmt = expr.clone();
            expr_for_stmt
                .then(
                    skip_trivia().ignore_then(just(Token::Semicolon).map_with_span(|_, span| Span::from(span))),
                )
                .map(|(e, semi)| StmtVariant::Expression(e, semi))
        };

        // Inline statement parser (for statements with semicolons)
        let inline_stmt = inline_var_decl.clone().or(inline_expr_stmt.clone());

        // Code block parser that uses the local expr reference (to avoid mutual recursion)
        // Syntax: { statement* expression? }
        //
        // Handles:
        // 1. Regular statements (with semicolons)
        // 2. Statement-like expressions (if, while, loop) without semicolons
        // 3. A final trailing expression
        let inline_code_block = {
            let expr_for_block = expr.clone();
            let expr_for_stmt_like = expr.clone();

            // A block item is either:
            // 1. A regular statement (with semicolon)
            // 2. An expression followed by optional semicolon
            //    - If it has a semicolon: it's a statement
            //    - If it's statement-like (if/while/loop) without semicolon: continue parsing
            //    - Otherwise: it's the trailing expression (let the outer parser handle it)
            let inline_block_item =
                inline_var_decl
                    .clone()
                    .map(BlockItem::Statement)
                    .or(expr_for_stmt_like
                        .then(
                            skip_trivia()
                                .ignore_then(just(Token::Semicolon).map_with_span(|_, span| Span::from(span)))
                                .map(Some)
                                .or(empty().map(|_| None)),
                        )
                        .try_map(|(e, maybe_semi), span| {
                            if let Some(semi) = maybe_semi {
                                // Has semicolon - it's a regular expression statement
                                Ok(BlockItem::Statement(StmtVariant::Expression(e, semi)))
                            } else if is_inline_statement_like(&e) {
                                // No semicolon but it's statement-like (if/while/loop) - OK
                                Ok(BlockItem::StatementExpr(e))
                            } else {
                                // No semicolon and not statement-like - fail, let it be parsed as trailing
                                Err(Simple::custom(span, "expected semicolon"))
                            }
                        }));

            skip_trivia()
                .ignore_then(just(Token::LBrace).map_with_span(|_, span| Span::from(span)))
                .then(
                    inline_block_item
                        .repeated()
                        .then(expr_for_block.map(BlockItem::TrailingExpression).or_not())
                        .map(|(mut statements, trailing)| {
                            if let Some(expr) = trailing {
                                statements.push(expr);
                            }
                            statements
                        }),
                )
                .then(skip_trivia().ignore_then(just(Token::RBrace).map_with_span(|_, span| Span::from(span))))
                .map(|((lbrace, items), rbrace)| CodeBlockData {
                    lbrace,
                    items,
                    rbrace,
                })
        };

        // If expression: if condition { then } else { else }
        // Uses condition_binary for condition to avoid infinite recursion
        // Uses inline_code_block for blocks (shares expr reference)
        let if_expr = skip_trivia()
            .ignore_then(just(Token::If).map_with_span(|_, span| Span::from(span)))
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .then(
                // Optional else clause
                skip_trivia()
                    .ignore_then(just(Token::Else).map_with_span(|_, span| Span::from(span)))
                    .then(
                        // Either another expression (for else if) or a block
                        // Using expr.clone() allows `else if` to work via the recursive parser
                        expr.clone()
                            .map(ElseClauseVariant::ElseIf)
                            .or(inline_code_block.clone().map(ElseClauseVariant::Block)),
                    )
                    .or_not(),
            )
            .map(|(((if_span, condition), then_block), else_opt)| {
                let else_clause = else_opt.map(|(else_span, else_variant)| match else_variant {
                    ElseClauseVariant::Block(block) => ElseClause::Block { else_span, block },
                    ElseClauseVariant::ElseIf(if_expr) => ElseClause::ElseIf {
                        else_span,
                        if_expr: Box::new(if_expr),
                    },
                });
                ExprVariant::If {
                    if_span,
                    condition: Box::new(condition),
                    then_block,
                    else_clause,
                }
            });

        // Optional label parser: identifier followed by colon
        let label_parser = skip_trivia()
            .ignore_then(filter_map(|span, token| match token {
                Token::Identifier => Ok(Span::from(span)),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }))
            .then(skip_trivia().ignore_then(just(Token::Colon).map_with_span(|_, span| Span::from(span))))
            .map(|(name, colon)| LabelData { name, colon });

        // While expression: label: while condition { body }
        let while_expr = label_parser
            .clone()
            .or_not()
            .then(skip_trivia().ignore_then(just(Token::While).map_with_span(|_, span| Span::from(span))))
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(
                |(((label, while_span), condition), body)| ExprVariant::While {
                    label,
                    while_span,
                    condition: Box::new(condition),
                    body,
                },
            );

        // Loop expression: label: loop { body }
        let loop_expr = label_parser
            .or_not()
            .then(skip_trivia().ignore_then(just(Token::Loop).map_with_span(|_, span| Span::from(span))))
            .then(inline_code_block.clone())
            .map(|((label, loop_span), body)| ExprVariant::Loop {
                label,
                loop_span,
                body,
            });

        // Break expression: break or break label
        let break_expr = skip_trivia()
            .ignore_then(just(Token::Break).map_with_span(|_, span| Span::from(span)))
            .then(
                skip_trivia()
                    .ignore_then(filter_map(|span, token| match token {
                        Token::Identifier => Ok(Span::from(span)),
                        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
                    }))
                    .or_not(),
            )
            .map(|(break_span, label)| ExprVariant::Break { break_span, label });

        // Continue expression: continue or continue label
        let continue_expr = skip_trivia()
            .ignore_then(just(Token::Continue).map_with_span(|_, span| Span::from(span)))
            .then(
                skip_trivia()
                    .ignore_then(filter_map(|span, token| match token {
                        Token::Identifier => Ok(Span::from(span)),
                        _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
                    }))
                    .or_not(),
            )
            .map(|(continue_span, label)| ExprVariant::Continue {
                continue_span,
                label,
            });

        // Return expression: return or return expr
        // Note: return can have an optional value expression
        let return_expr = skip_trivia()
            .ignore_then(just(Token::Return).map_with_span(|_, span| Span::from(span)))
            .then(
                // Try to parse an expression after return
                // Use the full expr parser recursively
                expr.clone().map(Box::new).or_not(),
            )
            .map(|(return_span, value)| ExprVariant::Return { return_span, value });

        // Full primary expressions (includes arrays, tuples, paren, if, while, loop, break, continue, return)
        let primary = float
            .or(integer)
            .or(string)
            .or(boolean)
            .or(null)
            .or(array)
            .or(paren_expr)
            .or(if_expr)
            .or(while_expr)
            .or(loop_expr)
            .or(break_expr)
            .or(continue_expr)
            .or(return_expr)
            .or(path);

        // Postfix expression: primary followed by zero or more postfix operations
        let postfix = primary
            .clone()
            .then(postfix_op.repeated())
            .map(|(base, ops)| {
                ops.into_iter().fold(base, |acc, op| match op {
                    PostfixOp::Call {
                        lparen,
                        arguments,
                        commas,
                        rparen,
                    } => ExprVariant::Call {
                        callee: Box::new(acc),
                        lparen,
                        arguments,
                        commas,
                        rparen,
                    },
                    PostfixOp::MemberAccess {
                        dot,
                        member,
                        type_args,
                    } => ExprVariant::MemberAccess {
                        base: Box::new(acc),
                        dot,
                        member,
                        type_args,
                    },
                    PostfixOp::TupleIndex { dot, index } => ExprVariant::TupleIndex {
                        base: Box::new(acc),
                        dot,
                        index,
                    },
                    PostfixOp::PostfixOperator {
                        operator,
                        operator_span,
                    } => ExprVariant::Postfix {
                        operand: Box::new(acc),
                        operator,
                        operator_span,
                    },
                })
            });

        let unary = unary_op
            .then(expr.clone())
            .map(|((tok, span), operand)| ExprVariant::Unary(tok, span, Box::new(operand)));

        // Non-assignment expression (unary or postfix)
        // Order matters: try unary first to handle -42, then postfix expressions
        let non_assignment = unary.or(postfix);

        // Binary expression: lhs op rhs op rhs ...
        // We parse as a flat left-to-right chain, precedence is handled in semantic phase
        let binary = non_assignment
            .clone()
            .then(binary_op.then(non_assignment.clone()).repeated())
            .map(|(first, rest)| {
                rest.into_iter()
                    .fold(first, |lhs, ((op_token, op_span), rhs)| {
                        ExprVariant::Binary {
                            lhs: Box::new(lhs),
                            operator: op_token,
                            operator_span: op_span,
                            rhs: Box::new(rhs),
                        }
                    })
            });

        // Assignment expression: lhs = rhs
        // Assignment is right-associative, so rhs recursively parses as expr (which includes assignment)
        // This gives us: a = b = c parses as a = (b = c)
        // Assignment has lowest precedence
        binary
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Equals).map_with_span(|_, span| Span::from(span)))
                    .then(expr.clone())
                    .or_not(),
            )
            .map(|(lhs, rhs_opt)| match rhs_opt {
                Some((equals, rhs)) => ExprVariant::Assignment {
                    lhs: Box::new(lhs),
                    equals,
                    rhs: Box::new(rhs),
                },
                None => lhs,
            })
    })
}

/// Helper enum for parsing parenthesized expressions
#[derive(Debug, Clone)]
enum ParenContent {
    Unit(Span),
    Grouping(ExprVariant, Span),
    Tuple(Vec<ExprVariant>, Vec<Span>, Span),
}

/// Helper enum for parsing else clauses
#[derive(Debug, Clone)]
enum ElseClauseVariant {
    Block(CodeBlockData),
    ElseIf(ExprVariant),
}

/// Helper enum for postfix operations (calls, member access, and postfix operators)
#[derive(Debug, Clone)]
enum PostfixOp {
    /// Function call: (args)
    Call {
        lparen: Span,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        rparen: Span,
    },
    /// Member access: .identifier or .identifier[T]
    MemberAccess {
        dot: Span,
        member: Span,
        type_args: Option<TypeArgsData>,
    },
    /// Tuple index: .0, .1
    TupleIndex { dot: Span, index: Span },
    /// Postfix operator: expr!
    PostfixOperator {
        operator: Token,
        operator_span: Span,
    },
}

/// Helper enum for postfix operations in condition expressions
/// (calls and member access only - no postfix operators like !)
#[derive(Debug, Clone)]
enum ConditionPostfixOp {
    /// Function call: (args)
    Call {
        lparen: Span,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        rparen: Span,
    },
    /// Member access: .identifier or .identifier[T]
    MemberAccess {
        dot: Span,
        member: Span,
        type_args: Option<TypeArgsData>,
    },
    /// Tuple index: .0, .1
    TupleIndex { dot: Span, index: Span },
}

/// Check if an expression variant is "statement-like" (doesn't require semicolon).
/// This is used in inline code blocks within the expression parser to allow
/// if/while/loop expressions to be followed by more statements without semicolons.
fn is_inline_statement_like(expr: &ExprVariant) -> bool {
    matches!(
        expr,
        ExprVariant::If { .. }
            | ExprVariant::While { .. }
            | ExprVariant::Loop { .. }
            | ExprVariant::Return { .. }
    )
}

/// Check if a token is a binary operator
fn is_binary_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::Ampersand
            | Token::Pipe
            | Token::Caret
            | Token::LessLess
            | Token::GreaterGreater
            | Token::Less
            | Token::Greater
            | Token::LessEquals
            | Token::GreaterEquals
            | Token::EqualsEquals
            | Token::BangEquals
            | Token::And
            | Token::Or
            | Token::QuestionQuestion
            | Token::DotDotEquals
            | Token::DotDotLess
    )
}

/// Emit events for any expression variant
pub fn emit_expr_variant(sink: &mut EventSink, variant: &ExprVariant) {
    match variant {
        ExprVariant::Unit(lparen, rparen) => {
            emit_unit_expr(sink, lparen.clone(), rparen.clone());
        }
        ExprVariant::Integer(span) => {
            emit_integer_expr(sink, span.clone());
        }
        ExprVariant::Float(span) => {
            emit_float_expr(sink, span.clone());
        }
        ExprVariant::String(span) => {
            emit_string_expr(sink, span.clone());
        }
        ExprVariant::Bool(span) => {
            emit_bool_expr(sink, span.clone());
        }
        ExprVariant::Null(span) => {
            emit_null_expr(sink, span.clone());
        }
        ExprVariant::Array(lbracket, elements, commas, rbracket) => {
            emit_array_expr(sink, lbracket.clone(), elements, commas, rbracket.clone());
        }
        ExprVariant::Tuple(lparen, elements, commas, rparen) => {
            emit_tuple_expr(sink, lparen.clone(), elements, commas, rparen.clone());
        }
        ExprVariant::Grouping(lparen, inner, rparen) => {
            emit_grouping_expr(sink, lparen.clone(), inner, rparen.clone());
        }
        ExprVariant::Path { segments, dots } => {
            emit_path_expr(sink, segments, dots);
        }
        ExprVariant::MemberAccess {
            base,
            dot,
            member,
            type_args,
        } => {
            emit_member_access_expr(sink, base, dot.clone(), member.clone(), type_args.as_ref());
        }
        ExprVariant::TupleIndex { base, dot, index } => {
            emit_tuple_index_expr(sink, base, dot.clone(), index.clone());
        }
        ExprVariant::Unary(tok, span, operand) => {
            emit_unary_expr(sink, tok.clone(), span.clone(), operand);
        }
        ExprVariant::Call {
            callee,
            lparen,
            arguments,
            commas,
            rparen,
        } => {
            emit_call_expr(
                sink,
                callee,
                lparen.clone(),
                arguments,
                commas,
                rparen.clone(),
            );
        }
        ExprVariant::Assignment { lhs, equals, rhs } => {
            emit_assignment_expr(sink, lhs, equals.clone(), rhs);
        }
        ExprVariant::Postfix {
            operand,
            operator,
            operator_span,
        } => {
            emit_postfix_expr(sink, operand, operator.clone(), operator_span.clone());
        }
        ExprVariant::Binary {
            lhs,
            operator,
            operator_span,
            rhs,
        } => {
            emit_binary_expr(sink, lhs, operator.clone(), operator_span.clone(), rhs);
        }
        ExprVariant::If {
            if_span,
            condition,
            then_block,
            else_clause,
        } => {
            emit_if_expr(
                sink,
                if_span.clone(),
                condition,
                then_block,
                else_clause.as_ref(),
            );
        }
        ExprVariant::While {
            label,
            while_span,
            condition,
            body,
        } => {
            emit_while_expr(sink, label.as_ref(), while_span.clone(), condition, body);
        }
        ExprVariant::Loop {
            label,
            loop_span,
            body,
        } => {
            emit_loop_expr(sink, label.as_ref(), loop_span.clone(), body);
        }
        ExprVariant::Break { break_span, label } => {
            emit_break_expr(sink, break_span.clone(), label.as_ref());
        }
        ExprVariant::Continue {
            continue_span,
            label,
        } => {
            emit_continue_expr(sink, continue_span.clone(), label.as_ref());
        }
        ExprVariant::Return { return_span, value } => {
            emit_return_expr(sink, return_span.clone(), value.as_deref());
        }
    }
}

/// Emit events for a unit expression
pub fn emit_unit_expr(sink: &mut EventSink, lparen: Span, rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprUnit);
    sink.add_token(SyntaxKind::LParen, lparen);
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node(); // Finish ExprUnit
    sink.finish_node(); // Finish Expression
}

/// Emit events for an integer literal expression
fn emit_integer_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprInteger);
    sink.add_token(SyntaxKind::Integer, span);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a float literal expression
fn emit_float_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprFloat);
    sink.add_token(SyntaxKind::Float, span);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a string literal expression
fn emit_string_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprString);
    sink.add_token(SyntaxKind::String, span);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a boolean literal expression
fn emit_bool_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBool);
    sink.add_token(SyntaxKind::Boolean, span);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a null literal expression
fn emit_null_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprNull);
    sink.add_token(SyntaxKind::Null, span);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for an array literal expression
fn emit_array_expr(
    sink: &mut EventSink,
    lbracket: Span,
    elements: &[ExprVariant],
    commas: &[Span],
    rbracket: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprArray);
    sink.add_token(SyntaxKind::LBracket, lbracket);
    for (i, element) in elements.iter().enumerate() {
        emit_expr_variant(sink, element);
        // Add comma after element if there is one
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token(SyntaxKind::RBracket, rbracket);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a tuple literal expression
fn emit_tuple_expr(
    sink: &mut EventSink,
    lparen: Span,
    elements: &[ExprVariant],
    commas: &[Span],
    rparen: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTuple);
    sink.add_token(SyntaxKind::LParen, lparen);
    for (i, element) in elements.iter().enumerate() {
        emit_expr_variant(sink, element);
        // Add comma after element if there is one
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a grouping expression
fn emit_grouping_expr(sink: &mut EventSink, lparen: Span, inner: &ExprVariant, rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprGrouping);
    sink.add_token(SyntaxKind::LParen, lparen);
    emit_expr_variant(sink, inner);
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for type arguments: [T, U]
/// Supports full types including tuples, functions, and arrays
fn emit_type_args(sink: &mut EventSink, type_args: &TypeArgsData) {
    sink.start_node(SyntaxKind::TypeArgumentList);
    sink.add_token(SyntaxKind::LBracket, type_args.lbracket.clone());
    for arg in type_args.args.iter() {
        emit_ty_variant(sink, arg);
    }
    sink.add_token(SyntaxKind::RBracket, type_args.rbracket.clone());
    sink.finish_node();
}

/// Emit events for a path expression
fn emit_path_expr(sink: &mut EventSink, segments: &[PathSegmentData], dots: &[Span]) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPath);
    for (i, segment) in segments.iter().enumerate() {
        sink.add_token(SyntaxKind::Identifier, segment.name.clone());
        // Emit type args if present
        if let Some(ref type_args) = segment.type_args {
            emit_type_args(sink, type_args);
        }
        // Add dot after segment if there is one
        if i < dots.len() {
            sink.add_token(SyntaxKind::Dot, dots[i].clone());
        }
    }
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a member access expression
/// Member access is represented using ExprPath for consistency with existing AST structure
fn emit_member_access_expr(
    sink: &mut EventSink,
    base: &ExprVariant,
    dot: Span,
    member: Span,
    type_args: Option<&TypeArgsData>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPath);
    // Emit the base expression first (unwrapped from Expression wrapper)
    emit_expr_variant_inner(sink, base);
    // Then emit the dot and member
    sink.add_token(SyntaxKind::Dot, dot);
    sink.add_token(SyntaxKind::Identifier, member);
    // Emit type args if present
    if let Some(type_args) = type_args {
        emit_type_args(sink, type_args);
    }
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a tuple index expression
fn emit_tuple_index_expr(sink: &mut EventSink, base: &ExprVariant, dot: Span, index: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTupleIndex);
    // Emit the base expression
    emit_expr_variant(sink, base);
    // Then emit the dot and index
    sink.add_token(SyntaxKind::Dot, dot);
    sink.add_token(SyntaxKind::Integer, index);
    sink.finish_node();
    sink.finish_node();
}

/// Helper to emit expression variant without the Expression wrapper
/// Used for member access where we need to chain path segments
fn emit_expr_variant_inner(sink: &mut EventSink, variant: &ExprVariant) {
    match variant {
        ExprVariant::Path { segments, dots } => {
            // Emit path segments directly without Expression wrapper
            for (i, segment) in segments.iter().enumerate() {
                sink.add_token(SyntaxKind::Identifier, segment.name.clone());
                // Emit type args if present
                if let Some(ref type_args) = segment.type_args {
                    emit_type_args(sink, type_args);
                }
                if i < dots.len() {
                    sink.add_token(SyntaxKind::Dot, dots[i].clone());
                }
            }
        }
        ExprVariant::MemberAccess {
            base,
            dot,
            member,
            type_args,
        } => {
            // Recursively emit base, then dot and member
            emit_expr_variant_inner(sink, base);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::Identifier, member.clone());
            // Emit type args if present
            if let Some(type_args) = type_args {
                emit_type_args(sink, type_args);
            }
        }
        ExprVariant::TupleIndex { base, dot, index } => {
            // Recursively emit base, then dot and index
            emit_expr_variant_inner(sink, base);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::Integer, index.clone());
        }
        ExprVariant::Call { .. } => {
            // For calls, we need the full expression wrapper for the callee
            emit_expr_variant(sink, variant);
        }
        _ => {
            // For other expressions, emit with wrapper
            emit_expr_variant(sink, variant);
        }
    }
}

/// Emit events for a unary expression
fn emit_unary_expr(sink: &mut EventSink, tok: Token, span: Span, operand: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprUnary);
    sink.add_token(SyntaxKind::from(tok), span);
    emit_expr_variant(sink, operand);
    sink.finish_node();
    sink.finish_node();
}

/// Emit events for a call expression
fn emit_call_expr(
    sink: &mut EventSink,
    callee: &ExprVariant,
    lparen: Span,
    arguments: &[CallArg],
    commas: &[Span],
    rparen: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprCall);

    // Emit the callee expression
    emit_expr_variant(sink, callee);

    // Emit the argument list
    sink.start_node(SyntaxKind::ArgumentList);
    sink.add_token(SyntaxKind::LParen, lparen);

    for (i, arg) in arguments.iter().enumerate() {
        sink.start_node(SyntaxKind::Argument);

        // If labeled, emit label and colon
        if let (Some(label), Some(colon)) = (&arg.label, &arg.colon) {
            sink.add_token(SyntaxKind::Identifier, label.clone());
            sink.add_token(SyntaxKind::Colon, colon.clone());
        }

        // Emit the argument value
        emit_expr_variant(sink, &arg.value);

        sink.finish_node(); // Argument

        // Add comma after argument if there is one
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }

    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node(); // ArgumentList

    sink.finish_node(); // ExprCall
    sink.finish_node(); // Expression
}

/// Emit events for an assignment expression
fn emit_assignment_expr(sink: &mut EventSink, lhs: &ExprVariant, equals: Span, rhs: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprAssignment);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::Equals, equals);
    emit_expr_variant(sink, rhs);
    sink.finish_node(); // ExprAssignment
    sink.finish_node(); // Expression
}

/// Emit events for a postfix expression (e.g., expr!)
fn emit_postfix_expr(
    sink: &mut EventSink,
    operand: &ExprVariant,
    operator: Token,
    operator_span: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPostfix);
    emit_expr_variant(sink, operand);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    sink.finish_node(); // ExprPostfix
    sink.finish_node(); // Expression
}

/// Emit events for a binary expression (e.g., a + b)
fn emit_binary_expr(
    sink: &mut EventSink,
    lhs: &ExprVariant,
    operator: Token,
    operator_span: Span,
    rhs: &ExprVariant,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBinary);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    emit_expr_variant(sink, rhs);
    sink.finish_node(); // ExprBinary
    sink.finish_node(); // Expression
}

/// Emit events for an if expression
fn emit_if_expr(
    sink: &mut EventSink,
    if_span: Span,
    condition: &ExprVariant,
    then_block: &CodeBlockData,
    else_clause: Option<&ElseClause>,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprIf);

    // if keyword
    sink.add_token(SyntaxKind::If, if_span);

    // condition expression
    emit_expr_variant(sink, condition);

    // then block
    emit_code_block(sink, then_block);

    // optional else clause
    if let Some(else_clause) = else_clause {
        sink.start_node(SyntaxKind::ElseClause);
        match else_clause {
            ElseClause::Block { else_span, block } => {
                sink.add_token(SyntaxKind::Else, else_span.clone());
                emit_code_block(sink, block);
            }
            ElseClause::ElseIf { else_span, if_expr } => {
                sink.add_token(SyntaxKind::Else, else_span.clone());
                // Recursively emit the if expression
                emit_expr_variant(sink, if_expr);
            }
        }
        sink.finish_node(); // ElseClause
    }

    sink.finish_node(); // ExprIf
    sink.finish_node(); // Expression
}

/// Emit events for a while expression
fn emit_while_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    while_span: Span,
    condition: &ExprVariant,
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprWhile);

    // Optional label
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node(); // LoopLabel
    }

    // while keyword
    sink.add_token(SyntaxKind::While, while_span);

    // condition expression
    emit_expr_variant(sink, condition);

    // body block
    emit_code_block(sink, body);

    sink.finish_node(); // ExprWhile
    sink.finish_node(); // Expression
}

/// Emit events for a loop expression
fn emit_loop_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    loop_span: Span,
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprLoop);

    // Optional label
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node(); // LoopLabel
    }

    // loop keyword
    sink.add_token(SyntaxKind::Loop, loop_span);

    // body block
    emit_code_block(sink, body);

    sink.finish_node(); // ExprLoop
    sink.finish_node(); // Expression
}

/// Emit events for a break expression
fn emit_break_expr(sink: &mut EventSink, break_span: Span, label: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBreak);

    // break keyword
    sink.add_token(SyntaxKind::Break, break_span);

    // Optional label
    if let Some(label_span) = label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
    }

    sink.finish_node(); // ExprBreak
    sink.finish_node(); // Expression
}

/// Emit events for a continue expression
fn emit_continue_expr(sink: &mut EventSink, continue_span: Span, label: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprContinue);

    // continue keyword
    sink.add_token(SyntaxKind::Continue, continue_span);

    // Optional label
    if let Some(label_span) = label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
    }

    sink.finish_node(); // ExprContinue
    sink.finish_node(); // Expression
}

/// Emit events for a return expression
fn emit_return_expr(sink: &mut EventSink, return_span: Span, value: Option<&ExprVariant>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprReturn);

    // return keyword
    sink.add_token(SyntaxKind::Return, return_span);

    // Optional value expression
    if let Some(val) = value {
        emit_expr_variant(sink, val);
    }

    sink.finish_node(); // ExprReturn
    sink.finish_node(); // Expression
}

/// Parse an expression and emit events
pub fn parse_expr<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let end_pos = source.len();
    let tokens_with_range = tokens.map(|(tok, span)| (tok, span.range()));
    let stream = chumsky::Stream::from_iter(end_pos..end_pos, tokens_with_range);

    match expr_parser().parse(stream) {
        Ok(variant) => {
            emit_expr_variant(sink, &variant);
        }
        Err(errors) => {
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), Span::from(span));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    fn parse_expr_from_source(source: &str) -> Expression {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();

        let mut sink = EventSink::new();
        parse_expr(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        Expression {
            syntax: tree,
            span: Span::from(0..source.len()),
        }
    }

    // ===== Unit Expression Tests =====

    #[test]
    fn test_unit_expression() {
        let source = "()";
        let expr = parse_expr_from_source(source);

        assert!(expr.is_unit());
    }

    #[test]
    fn test_unit_expression_with_whitespace() {
        let source = "  ()  ";
        let expr = parse_expr_from_source(source);

        assert!(expr.is_unit());
    }

    // ===== Integer Literal Tests =====

    #[test]
    fn test_integer_decimal() {
        let source = "42";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_integer());
    }

    #[test]
    fn test_integer_hex() {
        let source = "0xFF";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_integer());
    }

    #[test]
    fn test_integer_hex_uppercase() {
        let source = "0XAB";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_integer());
    }

    #[test]
    fn test_integer_binary() {
        let source = "0b1010";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_integer());
    }

    #[test]
    fn test_integer_octal() {
        let source = "0o755";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_integer());
    }

    // ===== Float Literal Tests =====

    #[test]
    fn test_float_simple() {
        let source = "3.14";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_float());
    }

    #[test]
    fn test_float_scientific() {
        let source = "1.0e10";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_float());
    }

    #[test]
    fn test_float_scientific_negative() {
        let source = "2.5E-3";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_float());
    }

    // ===== String Literal Tests =====

    #[test]
    fn test_string_simple() {
        let source = r#""hello""#;
        let expr = parse_expr_from_source(source);
        assert!(expr.is_string());
    }

    #[test]
    fn test_string_with_escapes() {
        let source = r#""hello\nworld""#;
        let expr = parse_expr_from_source(source);
        assert!(expr.is_string());
    }

    #[test]
    fn test_string_empty() {
        let source = r#""""#;
        let expr = parse_expr_from_source(source);
        assert!(expr.is_string());
    }

    // ===== Boolean Literal Tests =====

    #[test]
    fn test_bool_true() {
        let source = "true";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_bool());
    }

    #[test]
    fn test_bool_false() {
        let source = "false";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_bool());
    }

    // ===== Array Literal Tests =====

    #[test]
    fn test_array_empty() {
        let source = "[]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_array_single() {
        let source = "[1]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_array_multiple() {
        let source = "[1, 2, 3]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_array_trailing_comma() {
        let source = "[1, 2, 3,]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_array_nested() {
        let source = "[[1, 2], [3, 4]]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_array_mixed_types() {
        let source = r#"[1, "hello", true]"#;
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    // ===== Tuple Literal Tests =====

    #[test]
    fn test_tuple_single_element() {
        // Single element with trailing comma is a tuple
        let source = "(1,)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    #[test]
    fn test_tuple_two_elements() {
        let source = "(1, 2)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    #[test]
    fn test_tuple_multiple() {
        let source = "(1, 2, 3)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    #[test]
    fn test_tuple_trailing_comma() {
        let source = "(1, 2, 3,)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    #[test]
    fn test_tuple_nested() {
        let source = "((1, 2), (3, 4))";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    // ===== Grouping Expression Tests =====

    #[test]
    fn test_grouping_integer() {
        // Single element without trailing comma is grouping
        let source = "(42)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_grouping());
    }

    #[test]
    fn test_grouping_nested() {
        let source = "((42))";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_grouping());
    }

    #[test]
    fn test_grouping_string() {
        let source = r#"("hello")"#;
        let expr = parse_expr_from_source(source);
        assert!(expr.is_grouping());
    }

    // ===== Mixed/Complex Tests =====

    #[test]
    fn test_array_of_tuples() {
        let source = "[(1, 2), (3, 4)]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_tuple_of_arrays() {
        let source = "([1, 2], [3, 4])";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    #[test]
    fn test_deeply_nested() {
        let source = "[[(1,)]]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    // ===== Path Expression Tests =====

    #[test]
    fn test_path_single_segment() {
        let source = "foo";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_path_two_segments() {
        let source = "foo.bar";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_path_multiple_segments() {
        let source = "a.b.c.d";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_path_with_whitespace() {
        let source = "  foo . bar  ";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    // ===== Unary Expression Tests =====

    #[test]
    fn test_unary_minus_integer() {
        let source = "-42";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    #[test]
    fn test_unary_minus_float() {
        let source = "-3.14";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    #[test]
    fn test_unary_bang() {
        let source = "!true";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    #[test]
    fn test_unary_double_minus() {
        let source = "--42";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    #[test]
    fn test_unary_double_bang() {
        let source = "!!false";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    #[test]
    fn test_unary_minus_path() {
        let source = "-foo";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    #[test]
    fn test_unary_minus_grouped() {
        let source = "-(1)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_unary());
    }

    // ===== Null Literal Tests =====

    #[test]
    fn test_null() {
        let source = "null";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_null());
    }

    #[test]
    fn test_null_in_array() {
        let source = "[null, null]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_array());
    }

    #[test]
    fn test_null_in_tuple() {
        let source = "(null, 42)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_tuple());
    }

    // ===== Call Expression Tests =====

    #[test]
    fn test_call_no_args() {
        let source = "foo()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_single_arg() {
        let source = "foo(42)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_multiple_args() {
        let source = "foo(1, 2, 3)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_with_trailing_comma() {
        let source = "foo(1, 2,)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_labeled_arg() {
        let source = "foo(x: 42)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_mixed_labeled_unlabeled() {
        let source = "foo(1, name: \"test\", 3)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_chained() {
        let source = "foo()()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_method_call() {
        let source = "obj.method()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_method_call_with_args() {
        let source = "obj.method(1, 2)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_chained_method_calls() {
        let source = "a.b().c().d()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_with_expression_args() {
        let source = "foo((1, 2), [3, 4])";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    // ===== Assignment Expression Tests =====

    #[test]
    fn test_assignment_simple() {
        let source = "x = 5";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_assignment());
    }

    #[test]
    fn test_assignment_to_path() {
        let source = "point.x = 10";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_assignment());
    }

    #[test]
    fn test_assignment_with_expression_rhs() {
        let source = "x = foo()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_assignment());
    }

    #[test]
    fn test_assignment_chain() {
        // a = b = c should parse as a = (b = c)
        let source = "a = b = c";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_assignment());
    }

    #[test]
    fn test_assignment_with_complex_rhs() {
        let source = "result = obj.method(1, 2)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_assignment());
    }

    #[test]
    fn test_assignment_with_array_rhs() {
        let source = "arr = [1, 2, 3]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_assignment());
    }

    #[test]
    fn test_non_assignment_still_works() {
        // Verify that expressions without = still work
        let source = "foo.bar";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    // ===== If Expression Tests =====

    #[test]
    fn test_if_without_else() {
        let source = "if true { 1 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_if());
    }

    #[test]
    fn test_if_with_else() {
        let source = "if true { 1 } else { 2 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_if());
    }

    #[test]
    fn test_if_else_if() {
        let source = "if a { 1 } else if b { 2 } else { 3 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_if());
    }

    #[test]
    fn test_if_with_complex_condition() {
        let source = "if a and b { 1 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_if());
    }

    #[test]
    fn test_if_with_statements_in_block() {
        let source = "if true { let x: Int = 1; x }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_if());
    }

    #[test]
    fn test_nested_if() {
        let source = "if a { if b { 1 } else { 2 } } else { 3 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_if());
    }

    // ===== While Expression Tests =====

    #[test]
    fn test_while_basic() {
        let source = "while true { 1 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_while());
    }

    #[test]
    fn test_while_with_condition() {
        let source = "while x > 0 { x }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_while());
    }

    #[test]
    fn test_while_with_label() {
        let source = "outer: while true { 1 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_while());
    }

    // ===== Loop Expression Tests =====

    #[test]
    fn test_loop_basic() {
        let source = "loop { 1 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_loop());
    }

    #[test]
    fn test_loop_with_label() {
        let source = "outer: loop { 1 }";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_loop());
    }

    // ===== Break Expression Tests =====

    #[test]
    fn test_break_simple() {
        let source = "break";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_break());
    }

    #[test]
    fn test_break_with_label() {
        let source = "break outer";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_break());
    }

    // ===== Continue Expression Tests =====

    #[test]
    fn test_continue_simple() {
        let source = "continue";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_continue());
    }

    #[test]
    fn test_continue_with_label() {
        let source = "continue outer";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_continue());
    }

    // ===== Type Arguments in Expression Tests =====

    #[test]
    fn test_path_with_type_args() {
        let source = "List[Int]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_path_with_multiple_type_args() {
        let source = "Map[String, Int]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_path_with_nested_type_args() {
        let source = "List[Option[Int]]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_call_with_type_args() {
        let source = "foo[Int]()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_with_type_args_and_args() {
        let source = "helper[String](x)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_call_with_multiple_type_args() {
        let source = "convert[Int, String](42)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_method_call_with_type_args() {
        let source = "obj.method[Int]()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_chained_path_with_type_args() {
        let source = "Container[Int].Nested[String]";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_path());
    }

    #[test]
    fn test_static_method_with_type_args() {
        let source = "Container[Int].create()";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }

    #[test]
    fn test_path_type_args_then_method() {
        let source = "List[Int].new().push(1)";
        let expr = parse_expr_from_source(source);
        assert!(expr.is_call());
    }
}
