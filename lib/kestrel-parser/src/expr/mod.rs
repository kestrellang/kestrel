//! Expression parsing
//!
//! This module provides parsing for Kestrel expressions.
//! Currently supports:
//! - Unit expression: ()

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::block::{BlockItem, CodeBlockData, ElseBlockItem, GuardLetData};
use crate::common::{skip_inline_trivia, skip_trivia};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, create_input, prepare_tokens, to_kestrel_span};
use crate::stmt::{StmtVariant, VariableDeclarationData};
use crate::ty::ty_parser;

mod atom;
mod closure;
mod control;
mod data;
mod emit;
mod operators;
mod postfix;

use postfix::PostfixOp;

pub use data::{
    ArgumentListData, CallArg, ClosureParamData, ClosureParamsData, ElseClause, ExprVariant,
    IfCondition, LabelData, MatchArmData, MatchGuardData, PathSegmentData, TypeArgsData,
};
pub use emit::{emit_expr_variant, emit_if_condition, emit_unit_expr};

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

    /// Check if this is an interpolated string literal
    pub fn is_interpolated_string(&self) -> bool {
        self.kind() == SyntaxKind::ExprInterpolatedString
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

    /// Check if this is a closure expression
    pub fn is_closure(&self) -> bool {
        self.kind() == SyntaxKind::ExprClosure
    }

    /// Check if this is an implicit member access expression
    pub fn is_implicit_member_access(&self) -> bool {
        self.kind() == SyntaxKind::ExprImplicitMemberAccess
    }
}

// `full_type_args_parser` lives in `atom.rs`; use `atom::full_type_args_parser()`.
use atom::full_type_args_parser;

// Expression data types (`PathSegmentData`, `TypeArgsData`, `CallArg`,
// `ExprVariant`, `ArgumentListData`, `MatchArmData`, `MatchGuardData`,
// `LabelData`, `ClosureParamsData`, `ClosureParamData`, `ElseClause`,
// `IfCondition`) live in `data.rs` and are re-exported from this module.

/// Helper enum for parsing parenthesized expressions
#[derive(Debug, Clone)]
enum ParenContent {
    Unit(Span),
    Grouping(ExprVariant, Span),
    Tuple(Vec<ExprVariant>, Vec<Span>, Span),
}

/// Helper enum for parsing bracket expressions (arrays and dictionaries)
#[derive(Debug, Clone)]
enum BracketContent {
    EmptyArray(Span),      // []
    EmptyDictionary(Span), // [:]
    NonEmpty {
        first: ExprVariant,
        after: BracketContentAfterFirst,
    },
}

/// Helper enum for what comes after the first expression in a bracket
#[derive(Debug, Clone)]
enum BracketContentAfterFirst {
    ArraySingle {
        rbracket: Span,
    }, // [expr]
    ArrayMore {
        #[allow(dead_code)]
        first_comma: Span,
        more: Vec<ExprVariant>,
        rbracket: Span,
    }, // [expr, expr, ...]
    Dictionary {
        colon: Span,
        value: ExprVariant,
        more_entries: Vec<(Span, ExprVariant, Span, ExprVariant)>, // (comma, key, colon, value)
        rbracket: Span,
    },
}

/// Helper enum for parsing else clauses
#[derive(Debug, Clone)]
enum ElseClauseVariant {
    Block(CodeBlockData),
    ElseIf(ExprVariant),
}

// `PostfixOp` lives in `postfix.rs` and is imported above.

/// Helper enum for postfix operations in condition expressions
/// (calls and member access only - no postfix operators like !)
#[derive(Debug, Clone)]
enum ConditionPostfixOp {
    /// Function call: (args)
    Call {
        lparen: Option<Span>,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        rparen: Option<Span>,
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
/// if/while/loop/match expressions to be followed by more statements without semicolons.
pub(super) fn is_inline_statement_like(expr: &ExprVariant) -> bool {
    matches!(
        expr,
        ExprVariant::If { .. }
            | ExprVariant::While { .. }
            | ExprVariant::WhileLet { .. }
            | ExprVariant::Loop { .. }
            | ExprVariant::For { .. }
            | ExprVariant::Match { .. }
            | ExprVariant::Return { .. }
            | ExprVariant::Throw { .. }
            | ExprVariant::Try { .. }
    )
}

/// Attach trailing closures to an expression
/// This converts `expr { closure }` into a Call expression
fn attach_trailing_closures(expr: ExprVariant, trailing: Vec<CallArg>) -> ExprVariant {
    match expr {
        // Existing call: append trailing closures to arguments
        ExprVariant::Call {
            callee,
            lparen,
            mut arguments,
            commas,
            rparen,
        } => {
            arguments.extend(trailing);
            ExprVariant::Call {
                callee,
                lparen,
                arguments,
                commas,
                rparen,
            }
        },

        // Path becomes a call with no parens
        path @ ExprVariant::Path { .. } => ExprVariant::Call {
            callee: Box::new(path),
            lparen: None,
            arguments: trailing,
            commas: vec![],
            rparen: None,
        },

        // MemberAccess becomes a call with no parens
        member @ ExprVariant::MemberAccess { .. } => ExprVariant::Call {
            callee: Box::new(member),
            lparen: None,
            arguments: trailing,
            commas: vec![],
            rparen: None,
        },

        // Other expressions can't have trailing closures attached
        other => other,
    }
}

/// Parser for expressions
///
/// Uses boxed() on key sub-parsers to reduce compile time.
pub fn expr_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone {
    recursive(|expr| {
        // Atomic parsers (literals, path) live in `atom.rs`. Bind local
        // aliases for the ones `expr_parser` still uses by name so the rest
        // of this closure reads the same as before the split.
        let literal = atom::literal_parser();
        let path = atom::path_parser();

        // Array or Dictionary literal: [elem, ...] or [key: value, ...] or [:] or []
        // We need to distinguish:
        // - [] = empty array
        // - [:] = empty dictionary
        // - [expr, ...] = array
        // - [expr: expr, ...] = dictionary
        let array_or_dict = skip_trivia()
            .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                // Empty dictionary: [:]
                skip_trivia()
                    .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(skip_trivia().ignore_then(
                        just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .map(|(_colon, rbracket)| BracketContent::EmptyDictionary(rbracket))
                    .or(
                        // Empty array: []
                        skip_trivia()
                            .ignore_then(
                                just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span())),
                            )
                            .map(BracketContent::EmptyArray),
                    )
                    .or(
                        // Non-empty: parse first expression, then determine array or dict
                        expr.clone()
                            .then(
                                // Check for colon (dictionary) or comma/rbracket (array)
                                skip_trivia()
                                    .ignore_then(
                                        just(Token::Colon)
                                            .map_with(|_, e| to_kestrel_span(e.span())),
                                    )
                                    .then(expr.clone())
                                    .then(
                                        // More dictionary entries
                                        skip_trivia()
                                            .ignore_then(
                                                just(Token::Comma)
                                                    .map_with(|_, e| to_kestrel_span(e.span())),
                                            )
                                            .then(expr.clone())
                                            .then(
                                                skip_trivia().ignore_then(
                                                    just(Token::Colon).map_with(|_, e| {
                                                        to_kestrel_span(e.span())
                                                    }),
                                                ),
                                            )
                                            .then(expr.clone())
                                            .map(|(((comma, key), colon), value)| {
                                                (comma, key, colon, value)
                                            })
                                            .repeated()
                                            .collect::<Vec<_>>(),
                                    )
                                    .then(
                                        // Optional trailing comma
                                        skip_trivia()
                                            .ignore_then(
                                                just(Token::Comma)
                                                    .map_with(|_, e| to_kestrel_span(e.span())),
                                            )
                                            .or_not(),
                                    )
                                    .then(
                                        skip_trivia().ignore_then(
                                            just(Token::RBracket)
                                                .map_with(|_, e| to_kestrel_span(e.span())),
                                        ),
                                    )
                                    .map(
                                        |(
                                            (((colon, value), more_entries), _trailing),
                                            rbracket,
                                        )| {
                                            BracketContentAfterFirst::Dictionary {
                                                colon,
                                                value,
                                                more_entries,
                                                rbracket,
                                            }
                                        },
                                    )
                                    .or(
                                        // Array: more elements after first
                                        skip_trivia()
                                            .ignore_then(
                                                just(Token::Comma)
                                                    .map_with(|_, e| to_kestrel_span(e.span())),
                                            )
                                            .then(
                                                expr.clone()
                                                    .separated_by(skip_trivia().ignore_then(
                                                        just(Token::Comma).map_with(|_, e| {
                                                            to_kestrel_span(e.span())
                                                        }),
                                                    ))
                                                    .allow_trailing()
                                                    .collect::<Vec<_>>(),
                                            )
                                            .then(
                                                skip_trivia().ignore_then(
                                                    just(Token::RBracket).map_with(|_, e| {
                                                        to_kestrel_span(e.span())
                                                    }),
                                                ),
                                            )
                                            .map(|((first_comma, more), rbracket)| {
                                                BracketContentAfterFirst::ArrayMore {
                                                    first_comma,
                                                    more,
                                                    rbracket,
                                                }
                                            })
                                            .or(
                                                // Single element array: [expr]
                                                skip_trivia()
                                                    .ignore_then(
                                                        just(Token::Comma)
                                                            .map_with(|_, e| {
                                                                to_kestrel_span(e.span())
                                                            })
                                                            .or_not(),
                                                    )
                                                    .then(skip_trivia().ignore_then(
                                                        just(Token::RBracket).map_with(|_, e| {
                                                            to_kestrel_span(e.span())
                                                        }),
                                                    ))
                                                    .map(|(_trailing, rbracket)| {
                                                        BracketContentAfterFirst::ArraySingle {
                                                            rbracket,
                                                        }
                                                    }),
                                            ),
                                    ),
                            )
                            .map(|(first, after)| BracketContent::NonEmpty { first, after }),
                    ),
            )
            .map(|(lbracket, content)| match content {
                BracketContent::EmptyArray(rbracket) => {
                    ExprVariant::Array(lbracket, vec![], vec![], rbracket)
                },
                BracketContent::EmptyDictionary(rbracket) => ExprVariant::Dictionary {
                    lbracket,
                    entries: vec![],
                    commas: vec![],
                    rbracket,
                },
                BracketContent::NonEmpty { first, after } => match after {
                    BracketContentAfterFirst::ArraySingle { rbracket } => {
                        ExprVariant::Array(lbracket, vec![first], vec![], rbracket)
                    },
                    BracketContentAfterFirst::ArrayMore {
                        first_comma: _,
                        more,
                        rbracket,
                    } => {
                        let mut elements = vec![first];
                        elements.extend(more);
                        ExprVariant::Array(lbracket, elements, vec![], rbracket)
                    },
                    BracketContentAfterFirst::Dictionary {
                        colon,
                        value,
                        more_entries,
                        rbracket,
                    } => {
                        let mut entries = vec![(first, colon, value)];
                        let mut commas = Vec::new();
                        for (comma, key, colon, value) in more_entries {
                            commas.push(comma);
                            entries.push((key, colon, value));
                        }
                        ExprVariant::Dictionary {
                            lbracket,
                            entries,
                            commas,
                            rbracket,
                        }
                    },
                },
            })
            .boxed();

        // Parenthesized expressions: (), (expr), (expr,), (expr, expr, ...)
        // We need to carefully distinguish between:
        // - () = unit
        // - (expr) = grouping
        // - (expr,) = single-element tuple
        // - (expr, expr, ...) = tuple
        let paren_expr = skip_trivia()
            .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                // Empty parens: ()
                skip_trivia()
                    .ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())))
                    .map(ParenContent::Unit)
                    .or(
                        // First element
                        expr.clone()
                            .then(
                                // Check for comma after first element
                                skip_trivia()
                                    .ignore_then(
                                        just(Token::Comma)
                                            .map_with(|_, e| to_kestrel_span(e.span())),
                                    )
                                    .then(
                                        // After comma: either more elements or just rparen
                                        expr.clone()
                                            .separated_by(
                                                skip_trivia().ignore_then(
                                                    just(Token::Comma).map_with(|_, e| {
                                                        to_kestrel_span(e.span())
                                                    }),
                                                ),
                                            )
                                            .allow_trailing()
                                            .collect::<Vec<_>>()
                                            .or_not(),
                                    )
                                    .map(|(first_comma, more)| {
                                        (true, first_comma, more.unwrap_or_default())
                                    })
                                    .or(empty().to((false, Span::new(0, 0..0), vec![]))),
                            )
                            .then(skip_trivia().ignore_then(
                                just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())),
                            ))
                            .map(|((first, (has_comma, _first_comma, more)), rparen)| {
                                if !has_comma {
                                    // (expr) - grouping
                                    ParenContent::Grouping(first, rparen)
                                } else if more.is_empty() {
                                    // (expr,) - single-element tuple
                                    ParenContent::Tuple(vec![first], vec![], rparen)
                                } else {
                                    // (expr, expr, ...) - multi-element tuple
                                    let mut elements = vec![first];
                                    elements.extend(more);
                                    ParenContent::Tuple(elements, vec![], rparen)
                                }
                            }),
                    ),
            )
            .map(|(lparen, content)| match content {
                ParenContent::Unit(rparen) => ExprVariant::Unit(lparen, rparen),
                ParenContent::Grouping(inner, rparen) => {
                    ExprVariant::Grouping(lparen, Box::new(inner), rparen)
                },
                ParenContent::Tuple(elements, commas, rparen) => {
                    ExprVariant::Tuple(lparen, elements, commas, rparen)
                },
            })
            .boxed();

        // Postfix pieces live in `postfix.rs`. `arg_list` is still named
        // locally because the condition-postfix logic below reuses it to
        // pick out `PostfixOp::Call` forms (conditions forbid postfix-bang).
        let arg_list = postfix::arg_list_parser(expr.clone());
        let postfix_op = postfix::postfix_op_parser(expr.clone());

        // Operator token parsers live in `operators.rs`.
        let unary_op = operators::unary_op_parser();
        let binary_op = operators::binary_op_parser();

        // Inline variable declaration parser (uses expr for initializer)
        let inline_var_decl = skip_trivia()
            .ignore_then(
                just(Token::Let)
                    .map_with(|_, e| (to_kestrel_span(e.span()), false))
                    .or(just(Token::Var).map_with(|_, e| (to_kestrel_span(e.span()), true))),
            )
            .then(crate::pattern::pattern_parser())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(ty_parser())
                    .or_not(),
            )
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(expr.clone())
                    .or_not(),
            )
            .then(
                skip_trivia().ignore_then(
                    just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())),
                ),
            )
            .map(
                |(
                    ((((mutability_span, is_mutable), pattern), type_annotation), initializer),
                    semicolon,
                )| {
                    StmtVariant::VariableDeclaration(VariableDeclarationData {
                        mutability_span,
                        is_mutable,
                        pattern,
                        type_annotation,
                        initializer,
                        semicolon,
                    })
                },
            )
            .boxed();

        // Inline code block parser
        let inline_code_block =
            {
                let expr_for_block = expr.clone();
                let expr_for_stmt_like = expr.clone();
                let expr_for_guard = expr.clone();
                let expr_for_else = expr.clone();

                // Inline else block items parser (for guard-let else blocks)
                let inline_else_item =
                    inline_var_decl
                        .clone()
                        .map(ElseBlockItem::Statement)
                        .or(expr_for_else
                            .clone()
                            .then(
                                skip_trivia()
                                    .ignore_then(
                                        just(Token::Semicolon)
                                            .map_with(|_, e| to_kestrel_span(e.span())),
                                    )
                                    .map(Some)
                                    .or(empty().to(None)),
                            )
                            .try_map(|(e, maybe_semi), _extra| {
                                if let Some(semi) = maybe_semi {
                                    Ok(ElseBlockItem::Statement(StmtVariant::Expression(e, semi)))
                                } else if is_inline_statement_like(&e) {
                                    Ok(ElseBlockItem::StatementExpr(e))
                                } else {
                                    Err(Rich::custom(
                                        chumsky::span::Span::new((), 0..0),
                                        "expected semicolon",
                                    ))
                                }
                            }));

                let inline_else_items = inline_else_item
                    .repeated()
                    .collect::<Vec<_>>()
                    .then(
                        expr_for_else
                            .map(ElseBlockItem::TrailingExpression)
                            .or_not(),
                    )
                    .map(|(mut items, trailing)| {
                        if let Some(e) = trailing {
                            items.push(e);
                        }
                        items
                    });

                // Inline guard-let parser with chain support
                // Single let condition: let pattern = expr
                let inline_guard_let_condition = skip_trivia()
                    .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(crate::pattern::pattern_parser())
                    .then(skip_trivia().ignore_then(
                        just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .then(expr_for_guard.clone())
                    .map(
                        |(((let_span, pattern), equals_span), value)| IfCondition::Let {
                            let_span,
                            pattern,
                            equals_span,
                            value,
                        },
                    );

                // Single condition: either let-binding or boolean expression
                let inline_guard_single_condition = inline_guard_let_condition
                    .clone()
                    .or(expr_for_guard.clone().map(IfCondition::Expr));

                // Condition list: first must be let, followed by comma-separated conditions
                let inline_guard_conditions = inline_guard_let_condition
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Comma))
                            .ignore_then(inline_guard_single_condition)
                            .repeated()
                            .collect::<Vec<_>>(),
                    )
                    .map(|(first, rest)| {
                        let mut conditions = vec![first];
                        conditions.extend(rest);
                        conditions
                    });

                let inline_guard_let =
                    skip_trivia()
                        .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
                        .then(inline_guard_conditions)
                        .then(skip_trivia().ignore_then(
                            just(Token::Else).map_with(|_, e| to_kestrel_span(e.span())),
                        ))
                        .then(skip_trivia().ignore_then(
                            just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())),
                        ))
                        .then(inline_else_items)
                        .then(skip_trivia().ignore_then(
                            just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span())),
                        ))
                        .map(
                            |(
                                ((((guard_span, conditions), else_span), else_lbrace), else_items),
                                else_rbrace,
                            )| {
                                BlockItem::GuardLet(GuardLetData {
                                    guard_span,
                                    conditions,
                                    else_span,
                                    else_lbrace,
                                    else_items,
                                    else_rbrace,
                                })
                            },
                        );

                let inline_block_item = inline_guard_let
                    .or(inline_var_decl.clone().map(BlockItem::Statement))
                    .or(expr_for_stmt_like
                        .then(
                            skip_trivia()
                                .ignore_then(
                                    just(Token::Semicolon)
                                        .map_with(|_, e| to_kestrel_span(e.span())),
                                )
                                .or_not(),
                        )
                        .try_map(|(e, maybe_semi), _extra| {
                            if let Some(semi) = maybe_semi {
                                Ok(BlockItem::Statement(StmtVariant::Expression(e, semi)))
                            } else if is_inline_statement_like(&e) {
                                Ok(BlockItem::StatementExpr(e))
                            } else {
                                Err(Rich::custom(
                                    chumsky::span::Span::new((), 0..0),
                                    "expected semicolon",
                                ))
                            }
                        }));

                skip_trivia()
                    .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(
                        inline_block_item
                            .repeated()
                            .collect::<Vec<_>>()
                            .then(expr_for_block.map(BlockItem::TrailingExpression).or_not())
                            .map(|(mut statements, trailing)| {
                                if let Some(expr) = trailing {
                                    statements.push(expr);
                                }
                                statements
                            }),
                    )
                    .then(skip_trivia().ignore_then(
                        just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .map(|((lbrace, items), rbrace)| CodeBlockData {
                        lbrace,
                        items,
                        rbrace,
                    })
                    .boxed()
            };

        // Implicit member access: .Case or .Case(args)
        // Defined early so it can be used in condition expressions
        let implicit_member_access =
            {
                let implicit_arg_list = skip_inline_trivia()
                    .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(
                        postfix::argument_parser(expr.clone())
                            .separated_by(skip_trivia().ignore_then(
                                just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())),
                            ))
                            .allow_trailing()
                            .collect::<Vec<_>>(),
                    )
                    .then(skip_trivia().ignore_then(
                        just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .map(|((lparen, arguments), rparen)| ArgumentListData {
                        lparen,
                        arguments,
                        commas: vec![],
                        rparen,
                    });

                skip_trivia()
                    .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(skip_trivia().ignore_then(
                        select! { Token::Identifier = e => to_kestrel_span(e.span()) },
                    ))
                    .then(implicit_arg_list.or_not())
                    .map(
                        |((dot, member), arguments)| ExprVariant::ImplicitMemberAccess {
                            dot,
                            member,
                            arguments,
                        },
                    )
            };

        // Condition expression (simplified, no block expressions like if/while/loop/match/closures)
        let condition_primary = literal
            .clone()
            .or(array_or_dict.clone())
            .or(paren_expr.clone())
            .or(implicit_member_access.clone())
            .or(path.clone());

        let condition_postfix_op = arg_list
            .clone()
            .map(|op| match op {
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
            })
            .or(skip_trivia()
                .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
                .then(skip_trivia().ignore_then(select! {
                    Token::Identifier = e => (Token::Identifier, to_kestrel_span(e.span())),
                    Token::Integer = e => (Token::Integer, to_kestrel_span(e.span())),
                }))
                .then(full_type_args_parser().or_not())
                .map(|((dot, (token, span)), type_args)| match token {
                    Token::Integer => ConditionPostfixOp::TupleIndex { dot, index: span },
                    _ => ConditionPostfixOp::MemberAccess {
                        dot,
                        member: span,
                        type_args,
                    },
                }));

        let condition_postfix = condition_primary
            .clone()
            .then(condition_postfix_op.repeated().collect::<Vec<_>>())
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

        let condition_unary = unary_op
            .clone()
            .then(condition_postfix.clone())
            .map(|((tok, span), operand)| ExprVariant::Unary(tok, span, Box::new(operand)));

        let condition_non_assignment = condition_unary.or(condition_postfix.clone());

        let condition_binary = condition_non_assignment
            .clone()
            .then(
                binary_op
                    .clone()
                    .then(condition_non_assignment.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
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
            })
            .boxed();

        // If-let condition: let pattern = expr
        let if_let_condition = skip_trivia()
            .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
            .then(crate::pattern::pattern_parser())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(condition_binary.clone())
            .map(
                |(((let_span, pattern), equals_span), value)| IfCondition::Let {
                    let_span,
                    pattern,
                    equals_span,
                    value,
                },
            );

        // Single condition: either if-let or boolean expression
        let single_condition = if_let_condition
            .clone()
            .or(condition_binary.clone().map(IfCondition::Expr));

        // Condition list: comma-separated conditions (for if-let chains)
        let condition_list = single_condition
            .clone()
            .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, _| ())))
            .at_least(1)
            .collect::<Vec<_>>();

        // If expression (including if-let)
        let if_expr = skip_trivia()
            .ignore_then(just(Token::If).map_with(|_, e| to_kestrel_span(e.span())))
            .then(condition_list)
            .then(inline_code_block.clone())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::If))
                            .rewind()
                            .ignore_then(expr.clone())
                            .map(ElseClauseVariant::ElseIf)
                            .or(inline_code_block.clone().map(ElseClauseVariant::Block)),
                    )
                    .or_not(),
            )
            .map(|(((if_span, conditions), then_block), else_opt)| {
                let else_clause = else_opt.map(|(else_span, else_variant)| match else_variant {
                    ElseClauseVariant::Block(block) => ElseClause::Block { else_span, block },
                    ElseClauseVariant::ElseIf(if_expr) => ElseClause::ElseIf {
                        else_span,
                        if_expr: Box::new(if_expr),
                    },
                });
                ExprVariant::If {
                    if_span,
                    conditions,
                    then_block,
                    else_clause,
                }
            })
            .boxed();

        // Label parser: name: (for loop labels like outer: while ...)
        let label_parser = control::label_parser();

        // While-let condition list: starts with let, followed by more let or bool conditions
        // Example: while let .Some(x) = a, let .Some(y) = b, x > 0 { }
        let while_let_first_condition = if_let_condition.clone();
        let while_let_rest_conditions = skip_trivia()
            .ignore_then(just(Token::Comma))
            .ignore_then(single_condition.clone())
            .repeated()
            .collect::<Vec<_>>();

        let while_let_conditions = while_let_first_condition
            .then(while_let_rest_conditions)
            .map(|(first, rest)| {
                let mut conditions = vec![first];
                conditions.extend(rest);
                conditions
            });

        // While expression with optional label
        // Use separate parsers for labeled and unlabeled to avoid partial-match issues
        // Also handle while-let: while let pattern = expr { body }
        let labeled_while_let = label_parser
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(while_let_conditions.clone())
            .then(inline_code_block.clone())
            .map(
                |(((label, while_span), conditions), body)| ExprVariant::WhileLet {
                    label: Some(label),
                    while_span,
                    conditions,
                    body,
                },
            );

        let unlabeled_while_let = skip_trivia()
            .ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span())))
            .then(while_let_conditions.clone())
            .then(inline_code_block.clone())
            .map(|((while_span, conditions), body)| ExprVariant::WhileLet {
                label: None,
                while_span,
                conditions,
                body,
            });

        let labeled_while = label_parser
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(
                |(((label, while_span), condition), body)| ExprVariant::While {
                    label: Some(label),
                    while_span,
                    condition: Box::new(condition),
                    body,
                },
            );

        let unlabeled_while = skip_trivia()
            .ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span())))
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(|((while_span, condition), body)| ExprVariant::While {
                label: None,
                while_span,
                condition: Box::new(condition),
                body,
            });

        // Try while-let first (more specific), then regular while
        let while_expr = labeled_while_let
            .or(unlabeled_while_let)
            .or(labeled_while)
            .or(unlabeled_while)
            .boxed();

        // Loop expression with optional label
        let labeled_loop = label_parser
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Loop).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(inline_code_block.clone())
            .map(|((label, loop_span), body)| ExprVariant::Loop {
                label: Some(label),
                loop_span,
                body,
            });

        let unlabeled_loop = skip_trivia()
            .ignore_then(just(Token::Loop).map_with(|_, e| to_kestrel_span(e.span())))
            .then(inline_code_block.clone())
            .map(|(loop_span, body)| ExprVariant::Loop {
                label: None,
                loop_span,
                body,
            });

        let loop_expr = labeled_loop.or(unlabeled_loop).boxed();

        // For expression with optional label: label: for pattern in iterable { body }
        let labeled_for = label_parser
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::For).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(crate::pattern::pattern_parser())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::In).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(
                |(((((label, for_span), pattern), in_span), iterable), body)| ExprVariant::For {
                    label: Some(label),
                    for_span,
                    pattern,
                    in_span,
                    iterable: Box::new(iterable),
                    body,
                },
            );

        let unlabeled_for = skip_trivia()
            .ignore_then(just(Token::For).map_with(|_, e| to_kestrel_span(e.span())))
            .then(crate::pattern::pattern_parser())
            .then(
                skip_trivia()
                    .ignore_then(just(Token::In).map_with(|_, e| to_kestrel_span(e.span()))),
            )
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(
                |((((for_span, pattern), in_span), iterable), body)| ExprVariant::For {
                    label: None,
                    for_span,
                    pattern,
                    in_span,
                    iterable: Box::new(iterable),
                    body,
                },
            );
        let for_expr = labeled_for.or(unlabeled_for).boxed();

        // Simple keyword-prefix control-flow parsers live in `control.rs`.
        let break_expr = control::break_parser();
        let continue_expr = control::continue_parser();
        let return_expr = control::return_parser(expr.clone());
        let throw_expr = control::throw_parser(expr.clone());
        let try_keyword = control::try_keyword_parser();

        // Match expression: match scrutinee { pattern => expr, ... }
        let match_expr =
            {
                use crate::pattern::pattern_parser;

                // Match arm: pattern [if guard] => expression
                let match_arm = pattern_parser()
                    .then(
                        skip_trivia()
                            .ignore_then(
                                just(Token::If).map_with(|_, e| to_kestrel_span(e.span())),
                            )
                            .then(condition_binary.clone())
                            .map(|(if_span, condition)| MatchGuardData {
                                if_span,
                                condition: Box::new(condition),
                            })
                            .or_not(),
                    )
                    .then(skip_trivia().ignore_then(
                        just(Token::FatArrow).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .then(expr.clone())
                    .map(|(((pattern, guard), fat_arrow), body)| MatchArmData {
                        pattern,
                        guard,
                        fat_arrow,
                        body: Box::new(body),
                    });

                skip_trivia()
                    .ignore_then(just(Token::Match).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(condition_binary.clone())
                    .then(skip_trivia().ignore_then(
                        just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .then(
                        match_arm
                            .separated_by(
                                skip_trivia().ignore_then(just(Token::Comma).map_with(|_, _| ())),
                            )
                            .allow_trailing()
                            .collect::<Vec<_>>(),
                    )
                    .then(skip_trivia().ignore_then(
                        just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span())),
                    ))
                    .map(
                        |((((match_span, scrutinee), lbrace), arms), rbrace)| ExprVariant::Match {
                            match_span,
                            scrutinee: Box::new(scrutinee),
                            lbrace,
                            arms,
                            rbrace,
                        },
                    )
                    .boxed()
            };

        // Closure parsing lives in `closure.rs`. We build two variants of the
        // closure parser — one that skips all trivia before the `{` and one
        // that only skips inline trivia (no newlines). The inline variant
        // feeds the trailing-closure slot so a `{ ... }` on the next line is
        // NOT silently absorbed as a trailing closure.
        let closure_expr = closure::closure_parser(
            skip_trivia()
                .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
                .boxed(),
            expr.clone(),
            inline_var_decl.clone(),
        );

        let closure_expr_inline = closure::closure_parser(
            skip_inline_trivia()
                .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
                .boxed(),
            expr.clone(),
            inline_var_decl.clone(),
        );

        let trailing_closure_arg = closure::trailing_closure_arg_parser(closure_expr_inline);

        // Primary expressions
        let primary = literal
            .or(array_or_dict)
            .or(paren_expr)
            .or(if_expr)
            .or(while_expr)
            .or(loop_expr)
            .or(for_expr)
            .or(break_expr)
            .or(continue_expr)
            .or(return_expr)
            .or(throw_expr)
            .or(match_expr)
            .or(closure_expr)
            .or(implicit_member_access)
            .or(path)
            .boxed();

        // Postfix expression with trailing closures. The fold into an
        // ExprVariant tree is in `postfix::fold_postfix_ops`; trailing
        // closures still thread through `attach_trailing_closures` here
        // since that helper lives at module scope.
        let postfix = primary
            .then(postfix_op.repeated().collect::<Vec<_>>())
            .then(trailing_closure_arg.repeated().collect::<Vec<_>>())
            .map(|((base, ops), trailing_closures)| {
                let result = postfix::fold_postfix_ops(base, ops);
                if trailing_closures.is_empty() {
                    result
                } else {
                    attach_trailing_closures(result, trailing_closures)
                }
            })
            .boxed();

        // Unary expression - binds tighter than binary operators
        // so `not false or x` parses as `(not false) or x`, not `not (false or x)`
        // Collect all consecutive unary operators, then fold right-to-left to support `--42`, `!!x`
        let unary = unary_op
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .then(postfix.clone())
            .map(|(ops, operand)| {
                ops.into_iter().rev().fold(operand, |acc, (tok, span)| {
                    ExprVariant::Unary(tok, span, Box::new(acc))
                })
            });

        // Try expression: try expr (high precedence - binds to postfix)
        let try_expr = try_keyword
            .then(postfix.clone())
            .map(|(try_span, operand)| ExprVariant::Try {
                try_span,
                operand: Box::new(operand),
            });

        let non_assignment = try_expr.or(unary).or(postfix);

        // Binary expression
        let binary = non_assignment
            .clone()
            .then(
                binary_op
                    .then(non_assignment.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
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
            })
            .boxed();

        // Compound assignment operators live in `operators.rs`.
        let compound_assign_op = operators::compound_assign_op_parser();

        // Assignment or compound assignment expression
        binary
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(
                        // Regular assignment: =
                        just(Token::Equals)
                            .map_with(|_, e| (None, to_kestrel_span(e.span())))
                            // Compound assignment: +=, -=, etc.
                            .or(compound_assign_op.map(|(tok, span)| (Some(tok), span))),
                    )
                    .then(expr.clone())
                    .or_not(),
            )
            .map(|(lhs, rhs_opt)| match rhs_opt {
                Some(((None, equals), rhs)) => ExprVariant::Assignment {
                    lhs: Box::new(lhs),
                    equals,
                    rhs: Box::new(rhs),
                },
                Some(((Some(op_token), op_span), rhs)) => ExprVariant::CompoundAssignment {
                    lhs: Box::new(lhs),
                    operator: op_token,
                    operator_span: op_span,
                    rhs: Box::new(rhs),
                },
                None => lhs,
            })
            // Consume any trailing trivia at the end
            .then_ignore(skip_trivia())
    })
}


/// Parse an expression and emit events
pub fn parse_expr<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let prepared = prepare_tokens(tokens);
    let input = create_input(&prepared, source.len());

    match expr_parser().parse(input).into_result() {
        Ok(variant) => {
            // Transform strings with interpolation to InterpolatedString variant
            let transformed = emit::maybe_convert_to_interpolated(source, variant);
            emit_expr_variant(sink, &transformed);
        },
        Err(errors) => {
            // Even on error, we need to emit a valid tree structure
            // Wrap errors in an Error node so the tree builder doesn't panic
            sink.start_node(SyntaxKind::Expression);
            sink.start_node(SyntaxKind::Error);
            for error in errors {
                sink.error_from_rich(&error);
            }
            sink.finish_node(); // Error
            sink.finish_node(); // Expression
        },
    }
}

#[cfg(test)]
mod tests;
