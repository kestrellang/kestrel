//! Field declaration parsing
//!
//! This module is the single source of truth for field declaration parsing.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::block::{CodeBlockData, code_block_parser, emit_code_block};
use crate::common::{
    AttributeData, emit_attribute_list, emit_name, emit_static_modifier, emit_visibility,
    identifier, let_var_parser, skip_trivia, static_parser, token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::expr::{ExprVariant, emit_expr_variant, expr_parser};
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::parse_and_emit;
use crate::ty::{TyVariant, emit_ty_variant, ty_parser};

/// Represents a field declaration: (visibility)? (static)? let/var name: Type
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl FieldDeclaration {
    /// Create a new FieldDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the field name from this declaration
    pub fn name(&self) -> Option<String> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Name)?
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .find(|tok| tok.kind() == SyntaxKind::Identifier)
            .map(|tok| tok.text().to_string())
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

    /// Check if this field has the static modifier
    pub fn is_static(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::StaticModifier)
    }

    /// Check if this field is mutable (var vs let)
    pub fn is_mutable(&self) -> bool {
        self.syntax
            .children_with_tokens()
            .filter_map(|elem| elem.into_token())
            .any(|tok| tok.kind() == SyntaxKind::Var)
    }

    /// Get the type expression
    pub fn ty(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::Ty)
    }

    /// Check if this is a computed property (has a getter body or accessor clause)
    pub fn is_computed(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::PropertyAccessors)
    }

    /// Get the property accessors node if this is a computed property
    pub fn property_accessors(&self) -> Option<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::PropertyAccessors)
    }

    /// Get the getter clause if present (for explicit getter syntax)
    pub fn getter_clause(&self) -> Option<SyntaxNode> {
        self.property_accessors()?
            .children()
            .find(|child| child.kind() == SyntaxKind::GetterClause)
    }

    /// Get the setter clause if present
    pub fn setter_clause(&self) -> Option<SyntaxNode> {
        self.property_accessors()?
            .children()
            .find(|child| child.kind() == SyntaxKind::SetterClause)
    }

    /// Check if this computed property is getter-only (no setter)
    pub fn is_getter_only(&self) -> bool {
        self.is_computed() && self.setter_clause().is_none()
    }
}

/// Body data for computed properties
#[derive(Debug, Clone)]
pub enum ComputedBodyData {
    /// Shorthand: `{ expr }`
    Shorthand(CodeBlockData),
    /// Explicit: `{ get { } set { } }`
    Accessors {
        /// Span of the opening brace (for property accessors block)
        lbrace: Span,
        /// Span of the "get" keyword
        get_span: Span,
        getter: Option<CodeBlockData>, // None for protocol `{ get }`
        /// Span of the "set" keyword (if present)
        set_span: Option<Span>,
        setter: Option<CodeBlockData>, // None for protocol `{ get set }`
        /// Span of the closing brace (for property accessors block)
        rbrace: Span,
    },
}

/// Raw parsed data for field declaration internals
#[derive(Debug, Clone)]
pub struct FieldDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub is_static: Option<Span>,
    pub mutability_span: Span,
    pub is_mutable: bool,
    pub name_span: Span,
    pub colon_span: Span,
    pub ty: TyVariant,
    /// For computed properties: shorthand body OR accessors
    pub computed_body: Option<ComputedBodyData>,
    /// For constant initialization: (equals_span, expression)
    pub initializer: Option<(Span, ExprVariant)>,
    /// Optional trailing semicolon (for inline field declarations)
    pub semicolon: Option<Span>,
}

/// Parser for computed property body
///
/// Handles three forms:
/// 1. Shorthand: `{ expr }` - just a code block with an expression
/// 2. Explicit accessors: `{ get { expr } }` or `{ get { expr } set { expr } }`
/// 3. Protocol requirements: `{ get }` or `{ get set }` (no bodies, just keywords)
///
/// Returns `None` if no `{` follows (stored property), or `Some(ComputedBodyData)`.
fn computed_body_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<ComputedBodyData>, ParserExtra<'tokens>> + Clone
{
    // Protocol requirement: { get } or { get set }
    // These have no code block bodies, just keywords
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
            ComputedBodyData::Accessors {
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
    // getter is required, setter is optional
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
                ComputedBodyData::Accessors {
                    lbrace: lbrace_span,
                    get_span,
                    getter: Some(getter_body),
                    set_span,
                    setter: setter_body,
                    rbrace: rbrace_span,
                }
            },
        );

    // Shorthand: { expr } - parsed as a code block
    let shorthand = code_block_parser().map(ComputedBodyData::Shorthand);

    // Try protocol requirement first (most specific - has get/set keywords but no code blocks)
    // Then explicit accessors (has get keyword followed by code block)
    // Then shorthand (just a code block)
    // Finally, nothing (stored property)
    protocol_requirement
        .or(explicit_accessors)
        .or(shorthand)
        .map(Some)
        .or(empty().to(None))
        .boxed()
}

/// Parser for a field declaration
///
/// Syntax: `(@attr)* (visibility)? (static)? let/var name: Type (ComputedBody | Initializer)? (;)?`
///
/// ComputedBody can be:
/// - Shorthand: `{ expr }` - just a code block with an expression
/// - Explicit accessors: `{ get { expr } }` or `{ get { expr } set { expr } }`
/// - Protocol requirements: `{ get }` or `{ get set }` (no bodies, just keywords)
///
/// Initializer is:
/// - `= expr` - for constant initialization (e.g., `let STDIN: i64 = 0`)
///
/// This is the single source of truth for field declaration parsing.
/// An optional trailing semicolon is allowed for inline field declarations.
pub fn field_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, FieldDeclarationData, ParserExtra<'tokens>> + Clone {
    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(static_parser())
        .then(let_var_parser())
        .then(identifier())
        .then(token(Token::Colon))
        .then(ty_parser())
        .then(computed_body_parser())
        .then(
            // Optional initializer: = expr
            skip_trivia()
                .ignore_then(token(Token::Equals))
                .then(expr_parser())
                .map(|(eq, expr)| (eq, expr))
                .or_not(),
        )
        .then(token(Token::Semicolon).or_not())
        .map(
            |(
                (
                    (
                        (
                            (
                                (
                                    (
                                        ((attributes, visibility), is_static),
                                        (mutability_span, is_mutable),
                                    ),
                                    name_span,
                                ),
                                colon_span,
                            ),
                            ty,
                        ),
                        computed_body,
                    ),
                    initializer,
                ),
                semicolon,
            )| {
                FieldDeclarationData {
                    attributes,
                    visibility,
                    is_static,
                    mutability_span,
                    is_mutable,
                    name_span,
                    colon_span,
                    ty,
                    computed_body,
                    initializer,
                    semicolon,
                }
            },
        )
        .boxed()
}

/// Emit events for property accessors (computed property body)
fn emit_property_accessors(sink: &mut EventSink, computed_body: &ComputedBodyData) {
    sink.start_node(SyntaxKind::PropertyAccessors);

    match computed_body {
        ComputedBodyData::Shorthand(body) => {
            // Shorthand: just emit the code block directly
            emit_code_block(sink, body);
        },
        ComputedBodyData::Accessors {
            lbrace: _,
            get_span,
            getter,
            set_span,
            setter,
            rbrace: _,
        } => {
            // Emit getter
            if let Some(getter_body) = getter {
                // Full getter with body: emit GetterClause containing Get token and code block
                sink.start_node(SyntaxKind::GetterClause);
                sink.add_token(SyntaxKind::Get, get_span.clone());
                emit_code_block(sink, getter_body);
                sink.finish_node();
            } else {
                // Protocol requirement: emit Get token without body (no GetterClause wrapper)
                sink.add_token(SyntaxKind::Get, get_span.clone());
            }

            // Emit setter
            if let Some(setter_body) = setter {
                // Check if this is a real setter body or a placeholder for protocol requirement
                if setter_body.lbrace.start == 0 && setter_body.lbrace.end == 0 {
                    // Protocol requirement: emit Set token without body
                    if let Some(set_span) = set_span {
                        sink.add_token(SyntaxKind::Set, set_span.clone());
                    }
                } else {
                    // Full setter with body: emit SetterClause containing Set token and code block
                    sink.start_node(SyntaxKind::SetterClause);
                    if let Some(set_span) = set_span {
                        sink.add_token(SyntaxKind::Set, set_span.clone());
                    }
                    emit_code_block(sink, setter_body);
                    sink.finish_node();
                }
            }
        },
    }

    sink.finish_node();
}

/// Emit events for a field declaration
///
/// This is the single source of truth for field declaration emission.
pub fn emit_field_declaration(sink: &mut EventSink, data: FieldDeclarationData) {
    sink.start_node(SyntaxKind::FieldDeclaration);

    emit_attribute_list(sink, &data.attributes);
    emit_visibility(sink, data.visibility);
    emit_static_modifier(sink, data.is_static);

    if data.is_mutable {
        sink.add_token(SyntaxKind::Var, data.mutability_span);
    } else {
        sink.add_token(SyntaxKind::Let, data.mutability_span);
    }

    emit_name(sink, data.name_span);
    sink.add_token(SyntaxKind::Colon, data.colon_span);
    emit_ty_variant(sink, &data.ty);

    // Emit computed property body if present
    if let Some(computed_body) = &data.computed_body {
        emit_property_accessors(sink, computed_body);
    }

    // Emit initializer if present
    if let Some((equals_span, initializer_expr)) = data.initializer {
        sink.add_token(SyntaxKind::Equals, equals_span);
        emit_expr_variant(sink, &initializer_expr);
    }

    // Emit optional trailing semicolon
    if let Some(semicolon_span) = data.semicolon {
        sink.add_token(SyntaxKind::Semicolon, semicolon_span);
    }

    sink.finish_node();
}

/// Parse a field declaration and emit events
///
/// This is the primary event-driven parser function for field declarations.
pub fn parse_field_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        field_declaration_parser_internal(),
        emit_field_declaration
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    #[test]
    fn test_field_declaration_basic() {
        let source = "let x: Int";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("x".to_string()));
        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_static());
        assert!(!decl.is_mutable());
    }

    #[test]
    fn test_field_declaration_var() {
        let source = "var count: Int";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("count".to_string()));
        assert!(decl.is_mutable());
    }

    #[test]
    fn test_field_declaration_static() {
        let source = "static let instance: Self";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("instance".to_string()));
        assert!(decl.is_static());
        assert!(!decl.is_mutable());
    }

    #[test]
    fn test_field_declaration_with_visibility() {
        let source = "public let name: String";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("name".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_field_declaration_full() {
        let source = "public static var counter: Int";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_field_declaration(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();
        let decl = FieldDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        };

        assert_eq!(decl.name(), Some("counter".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_static());
        assert!(decl.is_mutable());
    }
}
