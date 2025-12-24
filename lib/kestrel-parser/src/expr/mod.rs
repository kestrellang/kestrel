//! Expression parsing
//!
//! This module provides parsing for Kestrel expressions.
//! Currently supports:
//! - Unit expression: ()

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::block::{emit_code_block, BlockItem, CodeBlockData, ElseBlockItem, GuardLetData};
use crate::common::skip_trivia;
use crate::event::{EventSink, TreeBuilder};
use crate::input::{create_input, prepare_tokens, to_kestrel_span, ParserExtra, ParserInput};
use crate::stmt::{StmtVariant, VariableDeclarationData};
use crate::ty::{emit_ty_variant, ty_parser, TyVariant};

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

    /// Check if this is a closure expression
    pub fn is_closure(&self) -> bool {
        self.kind() == SyntaxKind::ExprClosure
    }

    /// Check if this is an implicit member access expression
    pub fn is_implicit_member_access(&self) -> bool {
        self.kind() == SyntaxKind::ExprImplicitMemberAccess
    }
}

/// Parser for type arguments with full type support: [T, (A, B), [Int], (X) -> Y]
/// Returns (lbracket, types, rbracket)
fn full_type_args_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeArgsData, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            ty_parser()
                .separated_by(skip_trivia().ignore_then(just(Token::Comma)))
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span())))
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
    /// Call expression: callee(args) or callee { closure }
    Call {
        callee: Box<ExprVariant>,
        /// Left paren (None for trailing-closure-only calls)
        lparen: Option<Span>,
        arguments: Vec<CallArg>,
        commas: Vec<Span>,
        /// Right paren (None for trailing-closure-only calls)
        rparen: Option<Span>,
    },
    /// Assignment expression: lhs = rhs
    Assignment {
        lhs: Box<ExprVariant>,
        equals: Span,
        rhs: Box<ExprVariant>,
    },
    /// If expression: if condition { then } else { else }
    /// Also supports if-let: if let pattern = expr { then } else { else }
    /// And if-let chains: if let .Some(x) = a, let .Some(y) = b { ... }
    If {
        if_span: Span,
        /// List of conditions (at least one). Each is either:
        /// - A boolean expression
        /// - A let-binding (pattern + expression)
        conditions: Vec<IfCondition>,
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
    /// While-let expression: label: while let pattern = expr { body }
    WhileLet {
        label: Option<LabelData>,
        while_span: Span,
        let_span: Span,
        pattern: crate::pattern::PatternVariant,
        equals_span: Span,
        value: Box<ExprVariant>,
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
    /// Closure expression: { params in body } or { body }
    Closure {
        lbrace: Span,
        params: Option<ClosureParamsData>,
        in_span: Option<Span>,
        body: Vec<BlockItem>,
        rbrace: Span,
    },
    /// Implicit member access expression: .Case or .Case(args)
    /// Used for enum shorthand: let x: Direction = .north
    ImplicitMemberAccess {
        dot: Span,
        member: Span,
        /// Optional argument list for enum cases with associated values
        arguments: Option<ArgumentListData>,
    },
    /// Match expression: match scrutinee { pattern => expr, ... }
    Match {
        match_span: Span,
        scrutinee: Box<ExprVariant>,
        lbrace: Span,
        arms: Vec<MatchArmData>,
        rbrace: Span,
    },
}

/// Argument list data for implicit member access
#[derive(Debug, Clone)]
pub struct ArgumentListData {
    pub lparen: Span,
    pub arguments: Vec<CallArg>,
    pub commas: Vec<Span>,
    pub rparen: Span,
}

/// Match arm data: pattern [if guard] => expression
#[derive(Debug, Clone)]
pub struct MatchArmData {
    pub pattern: crate::pattern::PatternVariant,
    pub guard: Option<MatchGuardData>,
    pub fat_arrow: Span,
    pub body: Box<ExprVariant>,
}

/// Match guard data: if condition
#[derive(Debug, Clone)]
pub struct MatchGuardData {
    pub if_span: Span,
    pub condition: Box<ExprVariant>,
}

/// Loop label data: label:
#[derive(Debug, Clone)]
pub struct LabelData {
    pub name: Span,
    pub colon: Span,
}

/// Closure parameter list data: (x, y: Int)
#[derive(Debug, Clone)]
pub struct ClosureParamsData {
    pub lparen: Span,
    pub params: Vec<ClosureParamData>,
    pub commas: Vec<Span>,
    pub rparen: Span,
}

/// Single closure parameter: name or name: Type
#[derive(Debug, Clone)]
pub struct ClosureParamData {
    pub name: Span,
    pub colon: Option<Span>,
    pub ty: Option<TyVariant>,
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

/// A single condition in an if or if-let expression
#[derive(Debug, Clone)]
pub enum IfCondition {
    /// Boolean expression condition
    Expr(ExprVariant),
    /// Let binding condition: let pattern = expr
    Let {
        let_span: Span,
        pattern: crate::pattern::PatternVariant,
        equals_span: Span,
        value: ExprVariant,
    },
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
    /// Function call: (args) - always has parens since this is parsed from `(args)` syntax
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
fn is_inline_statement_like(expr: &ExprVariant) -> bool {
    matches!(
        expr,
        ExprVariant::If { .. }
            | ExprVariant::While { .. }
            | ExprVariant::WhileLet { .. }
            | ExprVariant::Loop { .. }
            | ExprVariant::Match { .. }
            | ExprVariant::Return { .. }
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
        }

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
pub fn expr_parser<'tokens>(
) -> impl Parser<'tokens, ParserInput<'tokens>, ExprVariant, ParserExtra<'tokens>> + Clone {
    recursive(|expr| {
        // Literals - these are simple and don't need boxing
        let integer = skip_trivia()
            .ignore_then(select! { Token::Integer = e => to_kestrel_span(e.span()) })
            .map(ExprVariant::Integer);

        let float = skip_trivia()
            .ignore_then(select! { Token::Float = e => to_kestrel_span(e.span()) })
            .map(ExprVariant::Float);

        let string = skip_trivia()
            .ignore_then(select! { Token::String = e => to_kestrel_span(e.span()) })
            .map(ExprVariant::String);

        let boolean = skip_trivia()
            .ignore_then(select! { Token::Boolean = e => to_kestrel_span(e.span()) })
            .map(ExprVariant::Bool);

        let null = skip_trivia()
            .ignore_then(select! { Token::Null = e => to_kestrel_span(e.span()) })
            .map(ExprVariant::Null);

        // Path segment: identifier optionally followed by type args [T, U]
        let path_segment = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(full_type_args_parser().or_not())
            .map(|(name, type_args)| PathSegmentData { name, type_args });

        // Path expression: a.b.c or a[T].b[U].c
        let path = path_segment
            .clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(path_segment.clone())
                    .repeated()
                    .collect::<Vec<_>>(),
            )
            .map(|(first, rest)| {
                let mut segments = vec![first];
                let mut dots = Vec::new();
                for (dot, segment) in rest {
                    dots.push(dot);
                    segments.push(segment);
                }
                ExprVariant::Path { segments, dots }
            })
            .boxed();

        // Array literal: [elem, elem, ...]
        let array = skip_trivia()
            .ignore_then(just(Token::LBracket).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                expr.clone()
                    .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))))
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .map(|elements| {
                        let commas: Vec<Span> = vec![];
                        (elements, commas)
                    })
                    .or_not(),
            )
            .then(skip_trivia().ignore_then(just(Token::RBracket).map_with(|_, e| to_kestrel_span(e.span()))))
            .map(|((lbracket, contents), rbracket)| {
                let (elements, commas) = contents.unwrap_or_else(|| (vec![], vec![]));
                ExprVariant::Array(lbracket, elements, commas, rbracket)
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
                                    .ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span())))
                                    .then(
                                        // After comma: either more elements or just rparen
                                        expr.clone()
                                            .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))))
                                            .allow_trailing()
                                            .collect::<Vec<_>>()
                                            .or_not()
                                    )
                                    .map(|(first_comma, more)| (true, first_comma, more.unwrap_or_default()))
                                    .or(empty().to((false, Span::new(0, 0..0), vec![])))
                            )
                            .then(skip_trivia().ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span()))))
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
                }
                ParenContent::Tuple(elements, commas, rparen) => {
                    ExprVariant::Tuple(lparen, elements, commas, rparen)
                }
            })
            .boxed();

        // Argument parser for call expressions
        // We need to carefully handle labeled vs unlabeled arguments
        // to avoid partial consumption issues with identifier followed by non-colon.
        //
        // The key insight is that a labeled argument is: identifier COLON expr
        // If we see identifier NOT followed by colon, it's the start of an expression.
        //
        // We use lookahead logic: first check if we have identifier:, only then commit.
        let labeled_argument = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(skip_trivia().ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span()))))
            .then(skip_trivia().ignore_then(expr.clone()))
            .map(|((label, colon), value)| CallArg {
                label: Some(label),
                colon: Some(colon),
                value,
            });

        let unlabeled_argument = skip_trivia()
            .ignore_then(expr.clone())
            .map(|value| CallArg {
                label: None,
                colon: None,
                value,
            });

        // Try labeled first (more specific), then unlabeled
        let argument = labeled_argument.or(unlabeled_argument);

        // Argument list: (arg, arg, ...)
        let arg_list = skip_trivia()
            .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
            .then(
                argument
                    .clone()
                    .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))))
                    .allow_trailing()
                    .collect::<Vec<_>>(),
            )
            .then(skip_trivia().ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span()))))
            .map(|((lparen, arguments), rparen)| PostfixOp::Call {
                lparen: Some(lparen),
                arguments,
                commas: vec![],
                rparen: Some(rparen),
            })
            .boxed();

        // Member access: .identifier or .identifier[T] or tuple index: .0, .1
        let member_access = skip_trivia()
            .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
            .then(skip_trivia().ignore_then(select! {
                Token::Identifier = e => (Token::Identifier, to_kestrel_span(e.span())),
                Token::Integer = e => (Token::Integer, to_kestrel_span(e.span())),
            }))
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
            .ignore_then(just(Token::Bang).map_with(|tok, e| (tok, to_kestrel_span(e.span()))))
            .map(|(tok, span)| PostfixOp::PostfixOperator {
                operator: tok,
                operator_span: span,
            });

        let postfix_op = arg_list.clone().or(member_access).or(postfix_bang);

        // Prefix unary operators: -, +, !, not
        let unary_op = skip_trivia().ignore_then(
            just(Token::Minus).map_with(|tok, e| (tok, to_kestrel_span(e.span())))
                .or(just(Token::Plus).map_with(|tok, e| (tok, to_kestrel_span(e.span()))))
                .or(just(Token::Bang).map_with(|tok, e| (tok, to_kestrel_span(e.span()))))
                .or(just(Token::Not).map_with(|tok, e| (tok, to_kestrel_span(e.span())))),
        );

        // Binary operator parser
        let binary_op = skip_trivia().ignore_then(select! {
            Token::Plus = e => (Token::Plus, to_kestrel_span(e.span())),
            Token::Minus = e => (Token::Minus, to_kestrel_span(e.span())),
            Token::Star = e => (Token::Star, to_kestrel_span(e.span())),
            Token::Slash = e => (Token::Slash, to_kestrel_span(e.span())),
            Token::Percent = e => (Token::Percent, to_kestrel_span(e.span())),
            Token::Ampersand = e => (Token::Ampersand, to_kestrel_span(e.span())),
            Token::Pipe = e => (Token::Pipe, to_kestrel_span(e.span())),
            Token::Caret = e => (Token::Caret, to_kestrel_span(e.span())),
            Token::LessLess = e => (Token::LessLess, to_kestrel_span(e.span())),
            Token::GreaterGreater = e => (Token::GreaterGreater, to_kestrel_span(e.span())),
            Token::Less = e => (Token::Less, to_kestrel_span(e.span())),
            Token::Greater = e => (Token::Greater, to_kestrel_span(e.span())),
            Token::LessEquals = e => (Token::LessEquals, to_kestrel_span(e.span())),
            Token::GreaterEquals = e => (Token::GreaterEquals, to_kestrel_span(e.span())),
            Token::EqualsEquals = e => (Token::EqualsEquals, to_kestrel_span(e.span())),
            Token::BangEquals = e => (Token::BangEquals, to_kestrel_span(e.span())),
            Token::And = e => (Token::And, to_kestrel_span(e.span())),
            Token::Or = e => (Token::Or, to_kestrel_span(e.span())),
            Token::QuestionQuestion = e => (Token::QuestionQuestion, to_kestrel_span(e.span())),
            Token::DotDotEquals = e => (Token::DotDotEquals, to_kestrel_span(e.span())),
            Token::DotDotLess = e => (Token::DotDotLess, to_kestrel_span(e.span())),
        });

        // Inline variable declaration parser (uses expr for initializer)
        let inline_var_decl = skip_trivia()
            .ignore_then(
                just(Token::Let).map_with(|_, e| (to_kestrel_span(e.span()), false))
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
            .then(skip_trivia().ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span()))))
            .map(|((((( mutability_span, is_mutable), pattern), type_annotation), initializer), semicolon)| {
                StmtVariant::VariableDeclaration(VariableDeclarationData {
                    mutability_span,
                    is_mutable,
                    pattern,
                    type_annotation,
                    initializer,
                    semicolon,
                })
            })
            .boxed();

        // Inline code block parser
        let inline_code_block = {
            let expr_for_block = expr.clone();
            let expr_for_stmt_like = expr.clone();
            let expr_for_guard = expr.clone();
            let expr_for_else = expr.clone();

            // Inline else block items parser (for guard-let else blocks)
            let inline_else_item = inline_var_decl
                .clone()
                .map(ElseBlockItem::Statement)
                .or(expr_for_else
                    .clone()
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                            .map(Some)
                            .or(empty().to(None)),
                    )
                    .try_map(|(e, maybe_semi), _extra| {
                        if let Some(semi) = maybe_semi {
                            Ok(ElseBlockItem::Statement(StmtVariant::Expression(e, semi)))
                        } else if is_inline_statement_like(&e) {
                            Ok(ElseBlockItem::StatementExpr(e))
                        } else {
                            Err(Rich::custom(chumsky::span::Span::new((), 0..0), "expected semicolon"))
                        }
                    }));

            let inline_else_items = inline_else_item
                .repeated()
                .collect::<Vec<_>>()
                .then(expr_for_else.map(ElseBlockItem::TrailingExpression).or_not())
                .map(|(mut items, trailing)| {
                    if let Some(e) = trailing {
                        items.push(e);
                    }
                    items
                });

            // Inline guard-let parser
            let inline_guard_let = skip_trivia()
                .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
                .then(skip_trivia().ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(crate::pattern::pattern_parser())
                .then(skip_trivia().ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(expr_for_guard)
                .then(skip_trivia().ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(skip_trivia().ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(inline_else_items)
                .then(skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .map(|((((((((guard_span, let_span), pattern), equals_span), value), else_span), else_lbrace), else_items), else_rbrace)| {
                    BlockItem::GuardLet(GuardLetData {
                        guard_span,
                        let_span,
                        pattern,
                        equals_span,
                        value,
                        else_span,
                        else_lbrace,
                        else_items,
                        else_rbrace,
                    })
                });

            let inline_block_item = inline_guard_let
                .or(inline_var_decl
                    .clone()
                    .map(BlockItem::Statement))
                .or(expr_for_stmt_like
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                            .map(Some)
                            .or(empty().to(None)),
                    )
                    .try_map(|(e, maybe_semi), _extra| {
                        if let Some(semi) = maybe_semi {
                            Ok(BlockItem::Statement(StmtVariant::Expression(e, semi)))
                        } else if is_inline_statement_like(&e) {
                            Ok(BlockItem::StatementExpr(e))
                        } else {
                            Err(Rich::custom(chumsky::span::Span::new((), 0..0), "expected semicolon"))
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
                .then(skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .map(|((lbrace, items), rbrace)| CodeBlockData { lbrace, items, rbrace })
                .boxed()
        };

        // Condition expression (simplified, no nested if/while/loop to avoid recursion issues)
        let condition_primary = float.clone()
            .or(integer.clone())
            .or(string.clone())
            .or(boolean.clone())
            .or(null.clone())
            .or(path.clone());

        let condition_postfix_op = arg_list.clone().map(|op| match op {
            PostfixOp::Call { lparen, arguments, commas, rparen } => ConditionPostfixOp::Call { lparen, arguments, commas, rparen },
            _ => unreachable!(),
        }).or(
            skip_trivia()
                .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
                .then(skip_trivia().ignore_then(select! {
                    Token::Identifier = e => (Token::Identifier, to_kestrel_span(e.span())),
                    Token::Integer = e => (Token::Integer, to_kestrel_span(e.span())),
                }))
                .then(full_type_args_parser().or_not())
                .map(|((dot, (token, span)), type_args)| match token {
                    Token::Integer => ConditionPostfixOp::TupleIndex { dot, index: span },
                    _ => ConditionPostfixOp::MemberAccess { dot, member: span, type_args },
                })
        );

        let condition_postfix = condition_primary
            .clone()
            .then(condition_postfix_op.repeated().collect::<Vec<_>>())
            .map(|(base, ops)| {
                ops.into_iter().fold(base, |acc, op| match op {
                    ConditionPostfixOp::MemberAccess { dot, member, type_args } => ExprVariant::MemberAccess {
                        base: Box::new(acc), dot, member, type_args,
                    },
                    ConditionPostfixOp::TupleIndex { dot, index } => ExprVariant::TupleIndex {
                        base: Box::new(acc), dot, index,
                    },
                    ConditionPostfixOp::Call { lparen, arguments, commas, rparen } => ExprVariant::Call {
                        callee: Box::new(acc), lparen, arguments, commas, rparen,
                    },
                })
            });

        let condition_unary = unary_op.clone()
            .then(condition_postfix.clone())
            .map(|((tok, span), operand)| ExprVariant::Unary(tok, span, Box::new(operand)));

        let condition_non_assignment = condition_unary.or(condition_postfix.clone());

        let condition_binary = condition_non_assignment.clone()
            .then(binary_op.clone().then(condition_non_assignment.clone()).repeated().collect::<Vec<_>>())
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, ((op_token, op_span), rhs)| {
                    ExprVariant::Binary { lhs: Box::new(lhs), operator: op_token, operator_span: op_span, rhs: Box::new(rhs) }
                })
            })
            .boxed();

        // If-let condition: let pattern = expr
        let if_let_condition = skip_trivia()
            .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
            .then(crate::pattern::pattern_parser())
            .then(skip_trivia().ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))))
            .then(condition_binary.clone())
            .map(|(((let_span, pattern), equals_span), value)| {
                IfCondition::Let { let_span, pattern, equals_span, value }
            });

        // Single condition: either if-let or boolean expression
        let single_condition = if_let_condition.clone()
            .or(condition_binary.clone().map(IfCondition::Expr));

        // Condition list: comma-separated conditions (for if-let chains)
        let condition_list = single_condition
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
                    ElseClauseVariant::ElseIf(if_expr) => ElseClause::ElseIf { else_span, if_expr: Box::new(if_expr) },
                });
                ExprVariant::If { if_span, conditions, then_block, else_clause }
            })
            .boxed();

        // Label parser: identifier: (for loop labels like outer: while ...)
        // Only parses when we see "identifier :" followed by while/loop
        let label_parser = skip_trivia()
            .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
            .then(skip_trivia().ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span()))))
            .map(|(name, colon)| LabelData { name, colon });

        // While-let condition parser: let pattern = expr
        let while_let_condition = skip_trivia()
            .ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span())))
            .then(crate::pattern::pattern_parser())
            .then(skip_trivia().ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))))
            .then(condition_binary.clone());

        // While expression with optional label
        // Use separate parsers for labeled and unlabeled to avoid partial-match issues
        // Also handle while-let: while let pattern = expr { body }
        let labeled_while_let = label_parser.clone()
            .then(skip_trivia().ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span()))))
            .then(while_let_condition.clone())
            .then(inline_code_block.clone())
            .map(|(((label, while_span), (((let_span, pattern), equals_span), value)), body)| ExprVariant::WhileLet {
                label: Some(label), while_span, let_span, pattern, equals_span, value: Box::new(value), body,
            });

        let unlabeled_while_let = skip_trivia()
            .ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span())))
            .then(while_let_condition.clone())
            .then(inline_code_block.clone())
            .map(|((while_span, (((let_span, pattern), equals_span), value)), body)| ExprVariant::WhileLet {
                label: None, while_span, let_span, pattern, equals_span, value: Box::new(value), body,
            });

        let labeled_while = label_parser.clone()
            .then(skip_trivia().ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span()))))
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(|(((label, while_span), condition), body)| ExprVariant::While {
                label: Some(label), while_span, condition: Box::new(condition), body,
            });

        let unlabeled_while = skip_trivia()
            .ignore_then(just(Token::While).map_with(|_, e| to_kestrel_span(e.span())))
            .then(condition_binary.clone())
            .then(inline_code_block.clone())
            .map(|((while_span, condition), body)| ExprVariant::While {
                label: None, while_span, condition: Box::new(condition), body,
            });

        // Try while-let first (more specific), then regular while
        let while_expr = labeled_while_let.or(unlabeled_while_let).or(labeled_while).or(unlabeled_while).boxed();

        // Loop expression with optional label
        let labeled_loop = label_parser
            .then(skip_trivia().ignore_then(just(Token::Loop).map_with(|_, e| to_kestrel_span(e.span()))))
            .then(inline_code_block.clone())
            .map(|((label, loop_span), body)| ExprVariant::Loop { label: Some(label), loop_span, body });

        let unlabeled_loop = skip_trivia()
            .ignore_then(just(Token::Loop).map_with(|_, e| to_kestrel_span(e.span())))
            .then(inline_code_block.clone())
            .map(|(loop_span, body)| ExprVariant::Loop { label: None, loop_span, body });

        let loop_expr = labeled_loop.or(unlabeled_loop).boxed();

        // Break expression
        let break_expr = skip_trivia()
            .ignore_then(just(Token::Break).map_with(|_, e| to_kestrel_span(e.span())))
            .then(skip_trivia().ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) }).or_not())
            .map(|(break_span, label)| ExprVariant::Break { break_span, label });

        // Continue expression
        let continue_expr = skip_trivia()
            .ignore_then(just(Token::Continue).map_with(|_, e| to_kestrel_span(e.span())))
            .then(skip_trivia().ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) }).or_not())
            .map(|(continue_span, label)| ExprVariant::Continue { continue_span, label });

        // Return expression
        let return_expr = skip_trivia()
            .ignore_then(just(Token::Return).map_with(|_, e| to_kestrel_span(e.span())))
            .then(expr.clone().map(Box::new).or_not())
            .map(|(return_span, value)| ExprVariant::Return { return_span, value });

        // Match expression: match scrutinee { pattern => expr, ... }
        let match_expr = {
            use crate::pattern::pattern_parser;

            // Match arm: pattern [if guard] => expression
            let match_arm = pattern_parser()
                .then(
                    skip_trivia()
                        .ignore_then(just(Token::If).map_with(|_, e| to_kestrel_span(e.span())))
                        .then(condition_binary.clone())
                        .map(|(if_span, condition)| MatchGuardData {
                            if_span,
                            condition: Box::new(condition),
                        })
                        .or_not()
                )
                .then(skip_trivia().ignore_then(just(Token::FatArrow).map_with(|_, e| to_kestrel_span(e.span()))))
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
                .then(skip_trivia().ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(
                    match_arm
                        .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, _| ())))
                        .allow_trailing()
                        .collect::<Vec<_>>()
                )
                .then(skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .map(|((((match_span, scrutinee), lbrace), arms), rbrace)| ExprVariant::Match {
                    match_span,
                    scrutinee: Box::new(scrutinee),
                    lbrace,
                    arms,
                    rbrace,
                })
                .boxed()
        };

        // Closure expression
        let closure_expr = {
            let closure_param = skip_trivia()
                .ignore_then(just(Token::Identifier).map_with(|_, e| to_kestrel_span(e.span())))
                .then(
                    skip_trivia()
                        .ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span())))
                        .then(ty_parser())
                        .or_not(),
                )
                .map(|(name, ty_opt)| {
                    let (colon, ty) = match ty_opt {
                        Some((c, t)) => (Some(c), Some(t)),
                        None => (None, None),
                    };
                    ClosureParamData { name, colon, ty }
                });

            let closure_params = skip_trivia()
                .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
                .then(
                    closure_param
                        .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))))
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(skip_trivia())
                .then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span())))
                .then_ignore(skip_trivia())
                .then(just(Token::In).map_with(|_, e| to_kestrel_span(e.span())))
                .map(|(((lparen, params), rparen), in_span)| {
                    (Some(ClosureParamsData { lparen, params, commas: vec![], rparen }), Some(in_span))
                });

            let expr_for_closure = expr.clone();
            let expr_for_closure_guard = expr.clone();
            let expr_for_closure_else = expr.clone();

            // Inline else block items parser for guard-let in closures
            let closure_else_item = inline_var_decl
                .clone()
                .map(ElseBlockItem::Statement)
                .or(expr_for_closure_else
                    .clone()
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                            .map(Some)
                            .or(empty().to(None)),
                    )
                    .try_map(|(e, maybe_semi), _extra| {
                        if let Some(semi) = maybe_semi {
                            Ok(ElseBlockItem::Statement(StmtVariant::Expression(e, semi)))
                        } else if is_inline_statement_like(&e) {
                            Ok(ElseBlockItem::StatementExpr(e))
                        } else {
                            Err(Rich::custom(chumsky::span::Span::new((), 0..0), "expected semicolon"))
                        }
                    }));

            let closure_else_items = closure_else_item
                .repeated()
                .collect::<Vec<_>>()
                .then(expr_for_closure_else.map(ElseBlockItem::TrailingExpression).or_not())
                .map(|(mut items, trailing)| {
                    if let Some(e) = trailing {
                        items.push(e);
                    }
                    items
                });

            // Inline guard-let parser for closures
            let closure_guard_let = skip_trivia()
                .ignore_then(just(Token::Guard).map_with(|_, e| to_kestrel_span(e.span())))
                .then(skip_trivia().ignore_then(just(Token::Let).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(crate::pattern::pattern_parser())
                .then(skip_trivia().ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(expr_for_closure_guard)
                .then(skip_trivia().ignore_then(just(Token::Else).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(skip_trivia().ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(closure_else_items)
                .then(skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .map(|((((((((guard_span, let_span), pattern), equals_span), value), else_span), else_lbrace), else_items), else_rbrace)| {
                    BlockItem::GuardLet(GuardLetData {
                        guard_span,
                        let_span,
                        pattern,
                        equals_span,
                        value,
                        else_span,
                        else_lbrace,
                        else_items,
                        else_rbrace,
                    })
                });

            let closure_block_item = closure_guard_let
                .or(inline_var_decl
                    .clone()
                    .map(BlockItem::Statement))
                .or(expr_for_closure
                    .clone()
                    .then(
                        skip_trivia()
                            .ignore_then(just(Token::Semicolon).map_with(|_, e| to_kestrel_span(e.span())))
                            .map(Some)
                            .or(empty().to(None)),
                    )
                    .try_map(|(e, maybe_semi), _extra| {
                        if let Some(semi) = maybe_semi {
                            Ok(BlockItem::Statement(StmtVariant::Expression(e, semi)))
                        } else if is_inline_statement_like(&e) {
                            Ok(BlockItem::StatementExpr(e))
                        } else {
                            Err(Rich::custom(chumsky::span::Span::new((), 0..0), "expected semicolon"))
                        }
                    }));

            skip_trivia()
                .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
                .then(closure_params.or_not().map(|opt| opt.unwrap_or((None, None))))
                .then(
                    closure_block_item
                        .repeated()
                        .collect::<Vec<_>>()
                        .then(expr_for_closure.map(BlockItem::TrailingExpression).or_not())
                        .map(|(mut statements, trailing)| {
                            if let Some(expr) = trailing {
                                statements.push(expr);
                            }
                            statements
                        }),
                )
                .then(skip_trivia().ignore_then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span()))))
                .map(|(((lbrace, (params, in_span)), body), rbrace)| ExprVariant::Closure {
                    lbrace, params, in_span, body, rbrace,
                })
                .boxed()
        };

        // Implicit member access: .Case or .Case(args)
        let implicit_member_access = {
            let implicit_arg_list = skip_trivia()
                .ignore_then(just(Token::LParen).map_with(|_, e| to_kestrel_span(e.span())))
                .then(
                    argument
                        .clone()
                        .separated_by(skip_trivia().ignore_then(just(Token::Comma).map_with(|_, e| to_kestrel_span(e.span()))))
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then(skip_trivia().ignore_then(just(Token::RParen).map_with(|_, e| to_kestrel_span(e.span()))))
                .map(|((lparen, arguments), rparen)| ArgumentListData {
                    lparen, arguments, commas: vec![], rparen,
                });

            skip_trivia()
                .ignore_then(just(Token::Dot).map_with(|_, e| to_kestrel_span(e.span())))
                .then(skip_trivia().ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) }))
                .then(implicit_arg_list.or_not())
                .map(|((dot, member), arguments)| ExprVariant::ImplicitMemberAccess { dot, member, arguments })
        };

        // Trailing closure argument
        // Can be either:
        // - Just a closure: { ... }
        // - Labeled closure: label: { ... }
        let trailing_closure_arg = {
            let closure_for_trailing = closure_expr.clone();

            // Labeled trailing closure: identifier: { closure }
            let labeled = skip_trivia()
                .ignore_then(select! { Token::Identifier = e => to_kestrel_span(e.span()) })
                .then(skip_trivia().ignore_then(just(Token::Colon).map_with(|_, e| to_kestrel_span(e.span()))))
                .then(closure_for_trailing.clone())
                .map(|((label, colon), closure)| CallArg {
                    label: Some(label),
                    colon: Some(colon),
                    value: closure,
                });

            // Unlabeled trailing closure: { closure }
            let unlabeled = closure_for_trailing
                .map(|closure| CallArg {
                    label: None,
                    colon: None,
                    value: closure,
                });

            labeled.or(unlabeled)
        };

        // Primary expressions
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
            .or(match_expr)
            .or(closure_expr)
            .or(implicit_member_access)
            .or(path)
            .boxed();

        // Postfix expression with trailing closures
        let postfix = primary
            .then(postfix_op.repeated().collect::<Vec<_>>())
            .then(trailing_closure_arg.repeated().collect::<Vec<_>>())
            .map(|((base, ops), trailing_closures)| {
                let result = ops.into_iter().fold(base, |acc, op| match op {
                    PostfixOp::Call { lparen, arguments, commas, rparen } => ExprVariant::Call {
                        callee: Box::new(acc), lparen, arguments, commas, rparen,
                    },
                    PostfixOp::MemberAccess { dot, member, type_args } => ExprVariant::MemberAccess {
                        base: Box::new(acc), dot, member, type_args,
                    },
                    PostfixOp::TupleIndex { dot, index } => ExprVariant::TupleIndex {
                        base: Box::new(acc), dot, index,
                    },
                    PostfixOp::PostfixOperator { operator, operator_span } => ExprVariant::Postfix {
                        operand: Box::new(acc), operator, operator_span,
                    },
                });
                if trailing_closures.is_empty() { result } else { attach_trailing_closures(result, trailing_closures) }
            })
            .boxed();

        // Unary expression
        let unary = unary_op
            .then(expr.clone())
            .map(|((tok, span), operand)| ExprVariant::Unary(tok, span, Box::new(operand)));

        let non_assignment = unary.or(postfix);

        // Binary expression
        let binary = non_assignment.clone()
            .then(binary_op.then(non_assignment.clone()).repeated().collect::<Vec<_>>())
            .map(|(first, rest)| {
                rest.into_iter().fold(first, |lhs, ((op_token, op_span), rhs)| {
                    ExprVariant::Binary { lhs: Box::new(lhs), operator: op_token, operator_span: op_span, rhs: Box::new(rhs) }
                })
            })
            .boxed();

        // Assignment expression
        binary.clone()
            .then(
                skip_trivia()
                    .ignore_then(just(Token::Equals).map_with(|_, e| to_kestrel_span(e.span())))
                    .then(expr.clone())
                    .or_not(),
            )
            .map(|(lhs, rhs_opt)| match rhs_opt {
                Some((equals, rhs)) => ExprVariant::Assignment { lhs: Box::new(lhs), equals, rhs: Box::new(rhs) },
                None => lhs,
            })
            // Consume any trailing trivia at the end
            .then_ignore(skip_trivia())
    })
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
        ExprVariant::MemberAccess { base, dot, member, type_args } => {
            emit_member_access_expr(sink, base, dot.clone(), member.clone(), type_args.as_ref());
        }
        ExprVariant::TupleIndex { base, dot, index } => {
            emit_tuple_index_expr(sink, base, dot.clone(), index.clone());
        }
        ExprVariant::Unary(tok, span, operand) => {
            emit_unary_expr(sink, tok.clone(), span.clone(), operand);
        }
        ExprVariant::Call { callee, lparen, arguments, commas, rparen } => {
            emit_call_expr(sink, callee, lparen.as_ref(), arguments, commas, rparen.as_ref());
        }
        ExprVariant::Assignment { lhs, equals, rhs } => {
            emit_assignment_expr(sink, lhs, equals.clone(), rhs);
        }
        ExprVariant::Postfix { operand, operator, operator_span } => {
            emit_postfix_expr(sink, operand, operator.clone(), operator_span.clone());
        }
        ExprVariant::Binary { lhs, operator, operator_span, rhs } => {
            emit_binary_expr(sink, lhs, operator.clone(), operator_span.clone(), rhs);
        }
        ExprVariant::If { if_span, conditions, then_block, else_clause } => {
            emit_if_expr(sink, if_span.clone(), conditions, then_block, else_clause.as_ref());
        }
        ExprVariant::While { label, while_span, condition, body } => {
            emit_while_expr(sink, label.as_ref(), while_span.clone(), condition, body);
        }
        ExprVariant::WhileLet { label, while_span, let_span, pattern, equals_span, value, body } => {
            emit_while_let_expr(sink, label.as_ref(), while_span.clone(), let_span.clone(), pattern, equals_span.clone(), value, body);
        }
        ExprVariant::Loop { label, loop_span, body } => {
            emit_loop_expr(sink, label.as_ref(), loop_span.clone(), body);
        }
        ExprVariant::Break { break_span, label } => {
            emit_break_expr(sink, break_span.clone(), label.as_ref());
        }
        ExprVariant::Continue { continue_span, label } => {
            emit_continue_expr(sink, continue_span.clone(), label.as_ref());
        }
        ExprVariant::Return { return_span, value } => {
            emit_return_expr(sink, return_span.clone(), value.as_deref());
        }
        ExprVariant::Closure { lbrace, params, in_span, body, rbrace } => {
            emit_closure_expr(sink, lbrace.clone(), params, in_span, body, rbrace.clone());
        }
        ExprVariant::ImplicitMemberAccess { dot, member, arguments } => {
            emit_implicit_member_access_expr(sink, dot.clone(), member.clone(), arguments.as_ref());
        }
        ExprVariant::Match { match_span, scrutinee, lbrace, arms, rbrace } => {
            emit_match_expr(sink, match_span.clone(), scrutinee, lbrace.clone(), arms, rbrace.clone());
        }
    }
}

/// Emit events for a unit expression
pub fn emit_unit_expr(sink: &mut EventSink, lparen: Span, rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprUnit);
    sink.add_token(SyntaxKind::LParen, lparen);
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
    sink.finish_node();
}

fn emit_integer_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprInteger);
    sink.add_token(SyntaxKind::Integer, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_float_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprFloat);
    sink.add_token(SyntaxKind::Float, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_string_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprString);
    sink.add_token(SyntaxKind::String, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_bool_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBool);
    sink.add_token(SyntaxKind::Boolean, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_null_expr(sink: &mut EventSink, span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprNull);
    sink.add_token(SyntaxKind::Null, span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_array_expr(sink: &mut EventSink, lbracket: Span, elements: &[ExprVariant], commas: &[Span], rbracket: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprArray);
    sink.add_token(SyntaxKind::LBracket, lbracket);
    for (i, element) in elements.iter().enumerate() {
        emit_expr_variant(sink, element);
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token(SyntaxKind::RBracket, rbracket);
    sink.finish_node();
    sink.finish_node();
}

fn emit_tuple_expr(sink: &mut EventSink, lparen: Span, elements: &[ExprVariant], commas: &[Span], rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTuple);
    sink.add_token(SyntaxKind::LParen, lparen);
    for (i, element) in elements.iter().enumerate() {
        emit_expr_variant(sink, element);
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
    sink.finish_node();
}

fn emit_grouping_expr(sink: &mut EventSink, lparen: Span, inner: &ExprVariant, rparen: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprGrouping);
    sink.add_token(SyntaxKind::LParen, lparen);
    emit_expr_variant(sink, inner);
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
    sink.finish_node();
}

fn emit_type_args(sink: &mut EventSink, type_args: &TypeArgsData) {
    sink.start_node(SyntaxKind::TypeArgumentList);
    sink.add_token(SyntaxKind::LBracket, type_args.lbracket.clone());
    for arg in type_args.args.iter() {
        emit_ty_variant(sink, arg);
    }
    sink.add_token(SyntaxKind::RBracket, type_args.rbracket.clone());
    sink.finish_node();
}

fn emit_path_expr(sink: &mut EventSink, segments: &[PathSegmentData], dots: &[Span]) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPath);
    for (i, segment) in segments.iter().enumerate() {
        sink.add_token(SyntaxKind::Identifier, segment.name.clone());
        if let Some(ref type_args) = segment.type_args {
            emit_type_args(sink, type_args);
        }
        if i < dots.len() {
            sink.add_token(SyntaxKind::Dot, dots[i].clone());
        }
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_expr_variant_inner(sink: &mut EventSink, variant: &ExprVariant) {
    match variant {
        ExprVariant::Path { segments, dots } => {
            for (i, segment) in segments.iter().enumerate() {
                sink.add_token(SyntaxKind::Identifier, segment.name.clone());
                if let Some(ref type_args) = segment.type_args {
                    emit_type_args(sink, type_args);
                }
                if i < dots.len() {
                    sink.add_token(SyntaxKind::Dot, dots[i].clone());
                }
            }
        }
        ExprVariant::MemberAccess { base, dot, member, type_args } => {
            emit_expr_variant_inner(sink, base);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::Identifier, member.clone());
            if let Some(type_args) = type_args {
                emit_type_args(sink, type_args);
            }
        }
        ExprVariant::TupleIndex { base, dot, index } => {
            emit_expr_variant_inner(sink, base);
            sink.add_token(SyntaxKind::Dot, dot.clone());
            sink.add_token(SyntaxKind::Integer, index.clone());
        }
        _ => emit_expr_variant(sink, variant),
    }
}

fn emit_member_access_expr(sink: &mut EventSink, base: &ExprVariant, dot: Span, member: Span, type_args: Option<&TypeArgsData>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPath);
    emit_expr_variant_inner(sink, base);
    sink.add_token(SyntaxKind::Dot, dot);
    sink.add_token(SyntaxKind::Identifier, member);
    if let Some(type_args) = type_args {
        emit_type_args(sink, type_args);
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_tuple_index_expr(sink: &mut EventSink, base: &ExprVariant, dot: Span, index: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprTupleIndex);
    emit_expr_variant(sink, base);
    sink.add_token(SyntaxKind::Dot, dot);
    sink.add_token(SyntaxKind::Integer, index);
    sink.finish_node();
    sink.finish_node();
}

fn emit_unary_expr(sink: &mut EventSink, tok: Token, span: Span, operand: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprUnary);
    sink.add_token(SyntaxKind::from(tok), span);
    emit_expr_variant(sink, operand);
    sink.finish_node();
    sink.finish_node();
}

fn emit_call_expr(sink: &mut EventSink, callee: &ExprVariant, lparen: Option<&Span>, arguments: &[CallArg], commas: &[Span], rparen: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprCall);
    emit_expr_variant(sink, callee);
    sink.start_node(SyntaxKind::ArgumentList);
    if let Some(lp) = lparen {
        sink.add_token(SyntaxKind::LParen, lp.clone());
    }
    for (i, arg) in arguments.iter().enumerate() {
        sink.start_node(SyntaxKind::Argument);
        if let (Some(label), Some(colon)) = (&arg.label, &arg.colon) {
            sink.add_token(SyntaxKind::Identifier, label.clone());
            sink.add_token(SyntaxKind::Colon, colon.clone());
        }
        emit_expr_variant(sink, &arg.value);
        sink.finish_node();
        if i < commas.len() {
            sink.add_token(SyntaxKind::Comma, commas[i].clone());
        }
    }
    if let Some(rp) = rparen {
        sink.add_token(SyntaxKind::RParen, rp.clone());
    }
    sink.finish_node();
    sink.finish_node();
    sink.finish_node();
}

fn emit_assignment_expr(sink: &mut EventSink, lhs: &ExprVariant, equals: Span, rhs: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprAssignment);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::Equals, equals);
    emit_expr_variant(sink, rhs);
    sink.finish_node();
    sink.finish_node();
}

fn emit_postfix_expr(sink: &mut EventSink, operand: &ExprVariant, operator: Token, operator_span: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprPostfix);
    emit_expr_variant(sink, operand);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    sink.finish_node();
    sink.finish_node();
}

fn emit_binary_expr(sink: &mut EventSink, lhs: &ExprVariant, operator: Token, operator_span: Span, rhs: &ExprVariant) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBinary);
    emit_expr_variant(sink, lhs);
    sink.add_token(SyntaxKind::from(operator), operator_span);
    emit_expr_variant(sink, rhs);
    sink.finish_node();
    sink.finish_node();
}

fn emit_if_expr(sink: &mut EventSink, if_span: Span, conditions: &[IfCondition], then_block: &CodeBlockData, else_clause: Option<&ElseClause>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprIf);
    sink.add_token(SyntaxKind::If, if_span);
    // Emit each condition
    for (i, condition) in conditions.iter().enumerate() {
        match condition {
            IfCondition::Expr(expr) => {
                emit_expr_variant(sink, expr);
            }
            IfCondition::Let { let_span, pattern, equals_span, value } => {
                sink.start_node(SyntaxKind::IfLetCondition);
                sink.add_token(SyntaxKind::Let, let_span.clone());
                crate::pattern::emit_pattern_variant(sink, pattern);
                sink.add_token(SyntaxKind::Equals, equals_span.clone());
                emit_expr_variant(sink, value);
                sink.finish_node();
            }
        }
        // Add comma between conditions (but not after last)
        if i < conditions.len() - 1 {
            // Note: We don't track comma spans in the parsed data, 
            // so we skip emitting commas. The tree structure is still correct.
        }
    }
    emit_code_block(sink, then_block);
    if let Some(else_clause) = else_clause {
        sink.start_node(SyntaxKind::ElseClause);
        match else_clause {
            ElseClause::Block { else_span, block } => {
                sink.add_token(SyntaxKind::Else, else_span.clone());
                emit_code_block(sink, block);
            }
            ElseClause::ElseIf { else_span, if_expr } => {
                sink.add_token(SyntaxKind::Else, else_span.clone());
                emit_expr_variant(sink, if_expr);
            }
        }
        sink.finish_node();
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_match_expr(
    sink: &mut EventSink,
    match_span: Span,
    scrutinee: &ExprVariant,
    lbrace: Span,
    arms: &[MatchArmData],
    rbrace: Span,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprMatch);
    sink.add_token(SyntaxKind::Match, match_span);
    emit_expr_variant(sink, scrutinee);
    sink.add_token(SyntaxKind::LBrace, lbrace);
    for arm in arms {
        sink.start_node(SyntaxKind::MatchArm);
        crate::pattern::emit_pattern_variant(sink, &arm.pattern);
        if let Some(guard) = &arm.guard {
            sink.start_node(SyntaxKind::MatchArmGuard);
            sink.add_token(SyntaxKind::If, guard.if_span.clone());
            emit_expr_variant(sink, &guard.condition);
            sink.finish_node();
        }
        sink.add_token(SyntaxKind::FatArrow, arm.fat_arrow.clone());
        emit_expr_variant(sink, &arm.body);
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::RBrace, rbrace);
    sink.finish_node();
    sink.finish_node();
}

fn emit_while_expr(sink: &mut EventSink, label: Option<&LabelData>, while_span: Span, condition: &ExprVariant, body: &CodeBlockData) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprWhile);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::While, while_span);
    emit_expr_variant(sink, condition);
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_while_let_expr(
    sink: &mut EventSink,
    label: Option<&LabelData>,
    while_span: Span,
    let_span: Span,
    pattern: &crate::pattern::PatternVariant,
    equals_span: Span,
    value: &ExprVariant,
    body: &CodeBlockData,
) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprWhile);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::While, while_span);
    // Emit the while-let condition
    sink.start_node(SyntaxKind::WhileLetCondition);
    sink.add_token(SyntaxKind::Let, let_span);
    crate::pattern::emit_pattern_variant(sink, pattern);
    sink.add_token(SyntaxKind::Equals, equals_span);
    emit_expr_variant(sink, value);
    sink.finish_node();
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_loop_expr(sink: &mut EventSink, label: Option<&LabelData>, loop_span: Span, body: &CodeBlockData) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprLoop);
    if let Some(label_data) = label {
        sink.start_node(SyntaxKind::LoopLabel);
        sink.add_token(SyntaxKind::Identifier, label_data.name.clone());
        sink.add_token(SyntaxKind::Colon, label_data.colon.clone());
        sink.finish_node();
    }
    sink.add_token(SyntaxKind::Loop, loop_span);
    emit_code_block(sink, body);
    sink.finish_node();
    sink.finish_node();
}

fn emit_break_expr(sink: &mut EventSink, break_span: Span, label: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprBreak);
    sink.add_token(SyntaxKind::Break, break_span);
    if let Some(label_span) = label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_continue_expr(sink: &mut EventSink, continue_span: Span, label: Option<&Span>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprContinue);
    sink.add_token(SyntaxKind::Continue, continue_span);
    if let Some(label_span) = label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_return_expr(sink: &mut EventSink, return_span: Span, value: Option<&ExprVariant>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprReturn);
    sink.add_token(SyntaxKind::Return, return_span);
    if let Some(val) = value {
        emit_expr_variant(sink, val);
    }
    sink.finish_node();
    sink.finish_node();
}

fn emit_closure_expr(sink: &mut EventSink, lbrace: Span, params: &Option<ClosureParamsData>, in_span: &Option<Span>, body: &[BlockItem], rbrace: Span) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprClosure);
    sink.add_token(SyntaxKind::LBrace, lbrace);
    if let Some(params_data) = params {
        sink.start_node(SyntaxKind::ClosureParams);
        sink.add_token(SyntaxKind::LParen, params_data.lparen.clone());
        for (i, param) in params_data.params.iter().enumerate() {
            if i > 0 && i <= params_data.commas.len() {
                sink.add_token(SyntaxKind::Comma, params_data.commas[i - 1].clone());
            }
            sink.start_node(SyntaxKind::ClosureParam);
            sink.add_token(SyntaxKind::Identifier, param.name.clone());
            if let Some(ref colon) = param.colon {
                sink.add_token(SyntaxKind::Colon, colon.clone());
            }
            if let Some(ref ty) = param.ty {
                emit_ty_variant(sink, ty);
            }
            sink.finish_node();
        }
        sink.add_token(SyntaxKind::RParen, params_data.rparen.clone());
        sink.finish_node();
    }
    if let Some(in_sp) = in_span {
        sink.add_token(SyntaxKind::In, in_sp.clone());
    }
    for item in body {
        emit_block_item(sink, item);
    }
    sink.add_token(SyntaxKind::RBrace, rbrace);
    sink.finish_node();
    sink.finish_node();
}

fn emit_block_item(sink: &mut EventSink, item: &BlockItem) {
    match item {
        BlockItem::Statement(stmt) => {
            use crate::stmt::emit_stmt_variant;
            emit_stmt_variant(sink, stmt);
        }
        BlockItem::StatementExpr(expr) => {
            emit_expr_variant(sink, expr);
        }
        BlockItem::TrailingExpression(expr) => {
            emit_expr_variant(sink, expr);
        }
        BlockItem::GuardLet(guard_data) => {
            // Guard-let in a closure/expression context
            use crate::block::ElseBlockItem;
            use crate::stmt::emit_stmt_variant;
            
            sink.start_node(SyntaxKind::Statement);
            sink.start_node(SyntaxKind::GuardLetStatement);
            sink.add_token(SyntaxKind::Guard, guard_data.guard_span.clone());
            sink.add_token(SyntaxKind::Let, guard_data.let_span.clone());
            crate::pattern::emit_pattern_variant(sink, &guard_data.pattern);
            sink.add_token(SyntaxKind::Equals, guard_data.equals_span.clone());
            emit_expr_variant(sink, &guard_data.value);
            sink.add_token(SyntaxKind::Else, guard_data.else_span.clone());
            
            sink.start_node(SyntaxKind::CodeBlock);
            sink.add_token(SyntaxKind::LBrace, guard_data.else_lbrace.clone());
            for else_item in &guard_data.else_items {
                match else_item {
                    ElseBlockItem::Statement(stmt) => {
                        emit_stmt_variant(sink, stmt);
                    }
                    ElseBlockItem::StatementExpr(expr) => {
                        sink.start_node(SyntaxKind::Statement);
                        sink.start_node(SyntaxKind::ExpressionStatement);
                        emit_expr_variant(sink, expr);
                        sink.finish_node();
                        sink.finish_node();
                    }
                    ElseBlockItem::TrailingExpression(expr) => {
                        emit_expr_variant(sink, expr);
                    }
                }
            }
            sink.add_token(SyntaxKind::RBrace, guard_data.else_rbrace.clone());
            sink.finish_node(); // CodeBlock
            
            sink.finish_node(); // GuardLetStatement
            sink.finish_node(); // Statement
        }
    }
}

fn emit_implicit_member_access_expr(sink: &mut EventSink, dot: Span, member: Span, arguments: Option<&ArgumentListData>) {
    sink.start_node(SyntaxKind::Expression);
    sink.start_node(SyntaxKind::ExprImplicitMemberAccess);
    sink.add_token(SyntaxKind::Dot, dot);
    sink.start_node(SyntaxKind::Name);
    sink.add_token(SyntaxKind::Identifier, member);
    sink.finish_node();
    if let Some(args) = arguments {
        sink.start_node(SyntaxKind::ArgumentList);
        sink.add_token(SyntaxKind::LParen, args.lparen.clone());
        for (i, arg) in args.arguments.iter().enumerate() {
            sink.start_node(SyntaxKind::Argument);
            if let (Some(label), Some(colon)) = (&arg.label, &arg.colon) {
                sink.add_token(SyntaxKind::Identifier, label.clone());
                sink.add_token(SyntaxKind::Colon, colon.clone());
            }
            emit_expr_variant(sink, &arg.value);
            sink.finish_node();
            if i < args.commas.len() {
                sink.add_token(SyntaxKind::Comma, args.commas[i].clone());
            }
        }
        sink.add_token(SyntaxKind::RParen, args.rparen.clone());
        sink.finish_node();
    }
    sink.finish_node();
    sink.finish_node();
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
            emit_expr_variant(sink, &variant);
        }
        Err(errors) => {
            // Even on error, we need to emit a valid tree structure
            // Wrap errors in an Error node so the tree builder doesn't panic
            sink.start_node(SyntaxKind::Expression);
            sink.start_node(SyntaxKind::Error);
            for error in errors {
                let span = error.span();
                sink.error_at(format!("Parse error: {:?}", error), to_kestrel_span(*span));
            }
            sink.finish_node(); // Error
            sink.finish_node(); // Expression
        }
    }
}

#[cfg(test)]
mod tests;
