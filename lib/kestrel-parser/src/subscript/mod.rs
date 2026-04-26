//! Subscript declaration parsing
//!
//! This module is the single source of truth for subscript declaration parsing.
//! Subscripts support generics with type parameters and where clauses.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::block::{CodeBlockData, code_block_parser, emit_code_block};
use crate::common::{
    AttributeData, ParameterData, emit_attribute_list, emit_parameter_list, emit_return_type,
    emit_static_modifier, emit_visibility, parameter_list_parser, skip_trivia, static_parser,
    token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::parse_and_emit;
use crate::ty::{TyVariant, ty_parser};
use crate::type_param::{
    TypeParameterData, WhereClauseData, emit_type_parameter_list, emit_where_clause,
    type_parameter_list_parser, where_clause_parser,
};

/// Represents a subscript declaration: (visibility)? (static)? subscript[T]?(params) -> Type (where ...)? { }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubscriptDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl SubscriptDeclaration {
    /// Create a new SubscriptDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the visibility modifier if present
    pub fn visibility(&self) -> Option<SyntaxKind> {
        let visibility_node = self
            .syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Visibility)?;

        visibility_node
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| {
                matches!(
                    tok.kind(),
                    SyntaxKind::Public
                        | SyntaxKind::Private
                        | SyntaxKind::Internal
                        | SyntaxKind::Fileprivate
                )
            })
            .map(|tok| tok.kind())
    }

    /// Check if this subscript has the static modifier
    pub fn is_static(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier)
    }

    /// Get the parameter list node
    pub fn parameter_list(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ParameterList)
    }

    /// Get the return type node
    pub fn return_type(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::ReturnType)
    }

    /// Get the subscript body node
    pub fn body(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::SubscriptBody)
    }

    /// Get the property accessors node if present (for explicit get/set)
    pub fn property_accessors(&self) -> Option<SyntaxNode> {
        self.body()?
            .children()
            .find(|child| child.kind() == SyntaxKind::PropertyAccessors)
    }

    /// Get the getter clause if present (for explicit getter syntax)
    pub fn getter_clause(&self) -> Option<SyntaxNode> {
        // First try in PropertyAccessors (explicit form)
        if let Some(accessors) = self.property_accessors()
            && let Some(getter) = accessors
                .children()
                .find(|child| child.kind() == SyntaxKind::GetterClause)
        {
            return Some(getter);
        }
        None
    }

    /// Get the setter clause if present
    pub fn setter_clause(&self) -> Option<SyntaxNode> {
        // Try in PropertyAccessors
        if let Some(accessors) = self.property_accessors()
            && let Some(setter) = accessors
                .children()
                .find(|child| child.kind() == SyntaxKind::SetterClause)
        {
            return Some(setter);
        }
        None
    }

    /// Get the getter body (for shorthand or explicit form)
    pub fn getter_body(&self) -> Option<SyntaxNode> {
        let body = self.body()?;

        // For shorthand form, the body is directly a CodeBlock
        if let Some(code_block) = body
            .children()
            .find(|child| child.kind() == SyntaxKind::CodeBlock)
        {
            // Check if it's inside a PropertyAccessors (explicit form)
            if self.property_accessors().is_none() {
                return Some(code_block);
            }
        }

        // For explicit form, find the code block in GetterClause
        self.getter_clause()?
            .children()
            .find(|child| child.kind() == SyntaxKind::CodeBlock)
    }

    /// Get the setter body if present
    pub fn setter_body(&self) -> Option<SyntaxNode> {
        self.setter_clause()?
            .children()
            .find(|child| child.kind() == SyntaxKind::CodeBlock)
    }

    /// Check if this subscript is getter-only (no setter)
    pub fn is_getter_only(&self) -> bool {
        self.setter_clause().is_none() && !self.has_protocol_setter_requirement()
    }

    /// Check if this is a protocol requirement (has `{ get }` or `{ get set }` without bodies)
    pub fn is_protocol_requirement(&self) -> bool {
        if let Some(accessors) = self.property_accessors() {
            // Protocol requirements have Get/Set tokens but no GetterClause/SetterClause children
            let has_getter_clause = accessors
                .children()
                .any(|c| c.kind() == SyntaxKind::GetterClause);
            let has_get_token = accessors
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|t| t.kind() == SyntaxKind::Get);

            // If there's a Get token but no GetterClause, it's a protocol requirement
            has_get_token && !has_getter_clause
        } else {
            false
        }
    }

    /// Check if this has a protocol setter requirement (has `{ get set }` without bodies)
    fn has_protocol_setter_requirement(&self) -> bool {
        if let Some(accessors) = self.property_accessors() {
            let has_setter_clause = accessors
                .children()
                .any(|c| c.kind() == SyntaxKind::SetterClause);
            let has_set_token = accessors
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .any(|t| t.kind() == SyntaxKind::Set);

            // If there's a Set token but no SetterClause, it's a protocol setter requirement
            has_set_token && !has_setter_clause
        } else {
            false
        }
    }

    /// Check if this subscript has type parameters
    pub fn has_type_parameters(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::TypeParameterList)
    }

    /// Check if this subscript has a where clause
    pub fn has_where_clause(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::WhereClause)
    }
}

/// Raw parsed data for subscript declaration internals
///
/// Subscript syntax: `(visibility)? (static)? subscript[T]?(params) -> Type (where ...)? { body }`
/// Body can be shorthand `{ expr }`, explicit `{ get { } set { } }`, or protocol `{ get }` / `{ get set }`
#[derive(Debug, Clone)]
pub struct SubscriptDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub is_static: Option<Span>,
    pub subscript_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub lparen: Span,
    pub parameters: Vec<ParameterData>,
    pub rparen: Span,
    pub return_type: (Span, TyVariant), // (arrow_span, return_ty) - required for subscripts
    pub where_clause: Option<WhereClauseData>,
    pub body: SubscriptBodyData,
}

/// Body data for subscript declarations
#[derive(Debug, Clone)]
pub enum SubscriptBodyData {
    /// Shorthand: `{ expr }` - just a code block with an expression
    Shorthand(CodeBlockData),
    /// Explicit: `{ get { } set { } }` - with explicit getter and optional setter
    Accessors {
        /// Span of the opening brace (for subscript body block)
        lbrace: Span,
        /// Span of the "get" keyword
        get_span: Span,
        getter: Option<CodeBlockData>, // None for protocol `{ get }`
        /// Span of the "set" keyword (if present)
        set_span: Option<Span>,
        setter: Option<CodeBlockData>, // None for protocol `{ get set }` without body
        /// Span of the closing brace (for subscript body block)
        rbrace: Span,
    },
}

/// Parser for subscript body
///
/// Handles three forms:
/// 1. Shorthand: `{ expr }` - just a code block with an expression
/// 2. Explicit accessors: `{ get { expr } }` or `{ get { expr } set { expr } }`
/// 3. Protocol requirements: `{ get }` or `{ get set }` (no bodies, just keywords)
fn subscript_body_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, SubscriptBodyData, ParserExtra<'tokens>> + Clone {
    // Protocol requirement: { get } or { get set }
    let protocol_requirement = skip_trivia()
        .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
        .then_ignore(skip_trivia())
        .then(just(Token::Get).map_with(|_, e| to_kestrel_span(e.span())))
        .then(
            skip_trivia()
                .ignore_then(just(Token::Set).map_with(|_, e| to_kestrel_span(e.span())))
                .map(Some)
                .or(empty().to(None)),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span())))
        .map(|(((lbrace_span, get_span), set_span_opt), rbrace_span)| {
            SubscriptBodyData::Accessors {
                lbrace: lbrace_span,
                get_span,
                getter: None,
                set_span: set_span_opt.clone(),
                setter: if set_span_opt.is_some() {
                    Some(CodeBlockData {
                        lbrace: Span::new(0, 0..0),
                        items: vec![],
                        rbrace: Span::new(0, 0..0),
                    })
                } else {
                    None
                },
                rbrace: rbrace_span,
            }
        });

    // Explicit accessors: { get { body } set { body }? }
    let explicit_accessors = skip_trivia()
        .ignore_then(just(Token::LBrace).map_with(|_, e| to_kestrel_span(e.span())))
        .then_ignore(skip_trivia())
        .then(just(Token::Get).map_with(|_, e| to_kestrel_span(e.span())))
        .then(code_block_parser())
        .then(
            skip_trivia()
                .ignore_then(just(Token::Set).map_with(|_, e| to_kestrel_span(e.span())))
                .then(code_block_parser())
                .or_not(),
        )
        .then_ignore(skip_trivia())
        .then(just(Token::RBrace).map_with(|_, e| to_kestrel_span(e.span())))
        .map(
            |((((lbrace_span, get_span), getter_body), setter_opt), rbrace_span)| {
                let (set_span, setter_body) = match setter_opt {
                    Some((set_span, setter_body)) => (Some(set_span), Some(setter_body)),
                    None => (None, None),
                };
                SubscriptBodyData::Accessors {
                    lbrace: lbrace_span,
                    get_span,
                    getter: Some(getter_body),
                    set_span,
                    setter: setter_body,
                    rbrace: rbrace_span,
                }
            },
        );

    let shorthand = code_block_parser().map(SubscriptBodyData::Shorthand);

    protocol_requirement
        .or(explicit_accessors)
        .or(shorthand)
        .boxed()
}

/// Parser for required return type: `-> Type`
fn required_return_type_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (Span, TyVariant), ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Arrow).map_with(|_, e| to_kestrel_span(e.span())))
        .then(ty_parser())
        .boxed()
}

/// Parser for a subscript declaration
///
/// Syntax: `(@attr)* (visibility)? (static)? subscript[T, U]?(params) -> Type (where ...)? { body }`
///
/// This is the single source of truth for subscript declaration parsing.
pub fn subscript_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, SubscriptDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(static_parser())
        .then(token(Token::Subscript))
        .then(type_parameter_list_parser().or_not())
        .then(token(Token::LParen))
        .then(parameter_list_parser())
        .then(token(Token::RParen))
        .then(required_return_type_parser())
        .then(where_clause_parser().or_not())
        .then(subscript_body_parser())
        .map(
            |(
                (
                    (
                        (
                            (
                                (
                                    (
                                        (((attributes, visibility), is_static), subscript_span),
                                        type_params,
                                    ),
                                    lparen,
                                ),
                                parameters,
                            ),
                            rparen,
                        ),
                        return_type,
                    ),
                    where_clause,
                ),
                body,
            )| {
                SubscriptDeclarationData {
                    attributes,
                    visibility,
                    is_static,
                    subscript_span,
                    type_params,
                    lparen,
                    parameters,
                    rparen,
                    return_type,
                    where_clause,
                    body,
                }
            },
        )
        .boxed()
}

/// Emit events for subscript body
fn emit_subscript_body(sink: &mut EventSink, body: &SubscriptBodyData) {
    sink.start_node(SyntaxKind::SubscriptBody);

    match body {
        SubscriptBodyData::Shorthand(code_block) => {
            emit_code_block(sink, code_block);
        },
        SubscriptBodyData::Accessors {
            lbrace,
            get_span,
            getter,
            set_span,
            setter,
            rbrace,
        } => {
            sink.start_node(SyntaxKind::PropertyAccessors);
            // Emit the outer `{` so the event sink advances over it
            // instead of leaving it as orphan source text. Without this
            // the trivia between subscripts would land in the wrong
            // declaration's leading-trivia slot, swallowing doc comments.
            sink.add_token(SyntaxKind::LBrace, lbrace.clone());

            if let Some(getter_body) = getter {
                sink.start_node(SyntaxKind::GetterClause);
                sink.add_token(SyntaxKind::Get, get_span.clone());
                emit_code_block(sink, getter_body);
                sink.finish_node();
            } else {
                sink.add_token(SyntaxKind::Get, get_span.clone());
            }

            if let Some(setter_body) = setter {
                if setter_body.lbrace.start == 0 && setter_body.lbrace.end == 0 {
                    if let Some(set_span) = set_span {
                        sink.add_token(SyntaxKind::Set, set_span.clone());
                    }
                } else {
                    sink.start_node(SyntaxKind::SetterClause);
                    if let Some(set_span) = set_span {
                        sink.add_token(SyntaxKind::Set, set_span.clone());
                    }
                    emit_code_block(sink, setter_body);
                    sink.finish_node();
                }
            }

            sink.add_token(SyntaxKind::RBrace, rbrace.clone());
            sink.finish_node(); // PropertyAccessors
        },
    }

    sink.finish_node(); // SubscriptBody
}

/// Emit events for a subscript declaration.
///
/// Destructures `SubscriptDeclarationData` without a `..` rest pattern:
/// adding a field forces this function to stop compiling until the new
/// field is handled in emission.
pub fn emit_subscript_declaration(sink: &mut EventSink, data: SubscriptDeclarationData) {
    let SubscriptDeclarationData {
        attributes,
        visibility,
        is_static,
        subscript_span,
        type_params,
        lparen,
        parameters,
        rparen,
        return_type,
        where_clause,
        body,
    } = data;

    sink.start_node(SyntaxKind::SubscriptDeclaration);

    emit_attribute_list(sink, &attributes);
    emit_visibility(sink, visibility);
    emit_static_modifier(sink, is_static);

    sink.add_token(SyntaxKind::Subscript, subscript_span);

    if let Some((lbracket, params, rbracket)) = type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    emit_parameter_list(sink, lparen, parameters, rparen);

    let (arrow_span, return_ty) = return_type;
    emit_return_type(sink, arrow_span, return_ty);

    if let Some(wc) = where_clause {
        emit_where_clause(sink, wc);
    }

    emit_subscript_body(sink, &body);

    sink.finish_node();
}

impl crate::event::EmitSyntax for SubscriptDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_subscript_declaration(sink, self);
    }
}

/// Parse a subscript declaration and emit events
///
/// This is the primary event-driven parser function for subscript declarations.
pub fn parse_subscript_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        subscript_declaration_parser_internal(),
        emit_subscript_declaration
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_subscript_declaration_shorthand() {
        let source = "subscript(index: Int) -> T { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_static());
        assert!(decl.parameter_list().is_some());
        assert!(decl.return_type().is_some());
        assert!(decl.body().is_some());
        assert!(decl.is_getter_only());
    }

    #[test]
    fn test_subscript_declaration_with_visibility() {
        let source = "public subscript(index: Int) -> T { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_subscript_declaration_static() {
        let source = "static subscript(key: String) -> T { Self.storage(key) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.is_static());
    }

    #[test]
    fn test_subscript_declaration_labeled_param() {
        let source = "subscript(safe index: Int) -> T { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.parameter_list().is_some());
    }

    #[test]
    fn test_subscript_declaration_with_generics() {
        let source = "subscript[K](key: K) -> V where K: Hashable { self.lookup(key) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.has_type_parameters());
        assert!(decl.has_where_clause());
    }

    #[test]
    fn test_subscript_declaration_explicit_get_set() {
        let source = "subscript(index: Int) -> T { get { self.data(index) } set { self.data(index) = newValue } }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(!decl.is_getter_only());
        assert!(decl.getter_clause().is_some());
        assert!(decl.setter_clause().is_some());
    }

    #[test]
    fn test_subscript_declaration_protocol_get() {
        let source = "subscript(index: Int) -> T { get }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.is_protocol_requirement());
        assert!(decl.is_getter_only());
    }

    #[test]
    fn test_subscript_declaration_protocol_get_set() {
        let source = "subscript(index: Int) -> T { get set }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert!(decl.is_protocol_requirement());
        assert!(!decl.is_getter_only());
    }

    #[test]
    fn test_subscript_declaration_full() {
        let source =
            "public static subscript[T](index: Int) -> T where T: Copyable { self.data(index) }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_subscript_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = SubscriptDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_static());
        assert!(decl.has_type_parameters());
        assert!(decl.has_where_clause());
        assert!(decl.parameter_list().is_some());
        assert!(decl.return_type().is_some());
    }
}
