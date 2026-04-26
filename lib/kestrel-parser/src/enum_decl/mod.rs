//! Enum declaration parsing
//!
//! This module is the single source of truth for enum declaration parsing.
//! Enum bodies can contain: cases, functions, initializers, nested structs/enums,
//! type aliases, modules, and imports.
//!
//! Note: The actual parsing is delegated to the unified type_decl module to handle
//! mutual recursion between structs and enums efficiently.

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::attribute::attribute_list_parser;
use crate::common::{
    AttributeData, ConformanceListData, TypeDeclarationBodyItem, emit_attribute_list, emit_name,
    emit_type_declaration_body_item, emit_visibility, identifier,
    initializer_declaration_parser_internal, skip_trivia, token, visibility_parser_internal,
};
use crate::event::{EventSink, TreeBuilder};
use crate::field::field_declaration_parser_internal;
use crate::function::function_declaration_parser_internal;
use crate::import::import_declaration_parser_internal;
use crate::input::{ParserExtra, ParserInput, to_kestrel_span};
use crate::module::module_declaration_parser_internal;
use crate::parse_and_emit;
use crate::subscript::subscript_declaration_parser_internal;
use crate::ty::{TyVariant, emit_ty_variant, ty_parser};
use crate::type_alias::type_alias_declaration_parser_internal;
use crate::type_decl::{TypeDeclarationData, enum_declaration_parser_unified};
use crate::type_param::{
    TypeParameterData, WhereClauseData, conformance_list_parser, emit_conformance_list,
    emit_type_parameter_list, emit_where_clause, type_parameter_list_parser, where_clause_parser,
};

/// Represents an enum declaration: (visibility)? (indirect)? enum Name[T]? (: Conformances)? (where ...)? { ... }
///
/// The declaration is stored as a lossless syntax tree. All data is derived
/// from the tree rather than stored separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumDeclaration {
    pub syntax: SyntaxNode,
    pub span: Span,
}

impl EnumDeclaration {
    /// Create a new EnumDeclaration from events and source text
    pub fn from_events(source: &str, events: Vec<crate::event::Event>, span: Span) -> Self {
        let builder = TreeBuilder::new(source, events);
        let syntax = builder.build();
        Self { syntax, span }
    }

    /// Get the enum name from this declaration
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

    /// Check if this enum has the `indirect` modifier
    pub fn is_indirect(&self) -> bool {
        self.syntax
            .children()
            .any(|child| child.kind() == SyntaxKind::IndirectModifier)
    }

    /// Get child declaration items (cases, nested structs, imports, modules, functions, initializers)
    pub fn children(&self) -> Vec<SyntaxNode> {
        self.syntax
            .children()
            .find(|child| child.kind() == SyntaxKind::EnumBody)
            .map(|body| {
                body.children()
                    .filter(|child| {
                        matches!(
                            child.kind(),
                            SyntaxKind::EnumCaseDeclaration
                                | SyntaxKind::StructDeclaration
                                | SyntaxKind::EnumDeclaration
                                | SyntaxKind::ImportDeclaration
                                | SyntaxKind::ModuleDeclaration
                                | SyntaxKind::FieldDeclaration
                                | SyntaxKind::FunctionDeclaration
                                | SyntaxKind::InitializerDeclaration
                                | SyntaxKind::TypeAliasDeclaration
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all enum cases in this declaration
    pub fn cases(&self) -> Vec<SyntaxNode> {
        self.children()
            .into_iter()
            .filter(|child| child.kind() == SyntaxKind::EnumCaseDeclaration)
            .collect()
    }
}

/// Raw parsed data for enum case parameter
///
/// Supports both named (`label: Type`) and unnamed (`Type`) forms:
/// - Named: `case Some(value: T)` - label and colon present
/// - Unnamed: `case Some(T)` - label and colon are None
#[derive(Debug, Clone)]
pub struct EnumCaseParameterData {
    /// Optional label name (None for unnamed parameters)
    pub label: Option<Span>,
    /// Optional colon (present only when label is present)
    pub colon: Option<Span>,
    /// The type of the parameter
    pub ty: TyVariant,
}

/// Raw parsed data for enum case declaration
#[derive(Debug, Clone)]
pub struct EnumCaseDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub case_span: Span,
    pub name_span: Span,
    pub parameters: Option<(Span, Vec<EnumCaseParameterData>, Span)>, // (lparen, params, rparen)
}

/// Raw parsed data for enum declaration
#[derive(Debug, Clone)]
pub struct EnumDeclarationData {
    pub attributes: Vec<AttributeData>,
    pub visibility: Option<(Token, Span)>,
    pub indirect: Option<Span>,
    pub enum_span: Span,
    pub name_span: Span,
    pub type_params: Option<(Span, Vec<TypeParameterData>, Span)>,
    pub conformances: Option<ConformanceListData>,
    pub where_clause: Option<WhereClauseData>,
    pub lbrace_span: Span,
    pub body: Vec<TypeDeclarationBodyItem>,
    pub rbrace_span: Span,
}

/// Internal Chumsky parser for enum declaration
///
/// This delegates to the unified type_decl parser which handles both struct and enum
/// in a single recursive context to avoid stack overflow on deeply nested types.
pub fn enum_declaration_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>> + Clone {
    enum_declaration_parser_unified()
}

/// Parser for enum case parameter: `label: Type` or just `Type`.
///
/// Named form (`label: Type`) is tried first so it wins over the unnamed
/// `Type`-only fallback when both would match.
pub(crate) fn enum_case_parameter_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumCaseParameterData, ParserExtra<'tokens>> + Clone {
    let named = identifier()
        .then(token(Token::Colon))
        .then(ty_parser())
        .map(|((label, colon), ty)| EnumCaseParameterData {
            label: Some(label),
            colon: Some(colon),
            ty,
        });

    let unnamed = ty_parser().map(|ty| EnumCaseParameterData {
        label: None,
        colon: None,
        ty,
    });

    named.or(unnamed).boxed()
}

/// Parser for enum case declaration:
/// `(@attr)* case Name` or `(@attr)* case Name(label: Type, ...)`.
pub(crate) fn enum_case_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, EnumCaseDeclarationData, ParserExtra<'tokens>> + Clone
{
    attribute_list_parser()
        .then(token(Token::Case))
        .then(identifier())
        .then(
            token(Token::LParen)
                .then(
                    enum_case_parameter_parser()
                        .separated_by(just(Token::Comma))
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then(token(Token::RParen))
                .map(|((lparen, params), rparen)| Some((lparen, params, rparen)))
                .or(empty().map(|_| None)),
        )
        .map(
            |(((attributes, case_span), name_span), parameters)| EnumCaseDeclarationData {
                attributes,
                case_span,
                name_span,
                parameters,
            },
        )
        .boxed()
}

/// Parser for the optional `indirect` modifier that precedes `enum`.
pub(crate) fn indirect_modifier_parser<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Option<Span>, ParserExtra<'tokens>> + Clone {
    skip_trivia()
        .ignore_then(just(Token::Indirect).map_with(|_, e| Some(to_kestrel_span(e.span()))))
        .or(empty().to(None))
}

/// Parser for a single enum-body item. Takes the shared `type_parser` handle
/// so nested type declarations (struct or enum) can be parsed inside the body
/// without creating a separate recursive context.
pub(crate) fn enum_body_item_parser<'tokens, P>(
    type_parser: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, TypeDeclarationBodyItem, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, TypeDeclarationData, ParserExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let module_parser = module_declaration_parser_internal()
        .map(|(module_span, path)| TypeDeclarationBodyItem::Module(module_span, path));

    let import_parser =
        import_declaration_parser_internal().map(|(import_span, path, alias, items)| {
            TypeDeclarationBodyItem::Import(import_span, path, alias, items)
        });

    let nested_type_parser = type_parser.map(|data| match data {
        TypeDeclarationData::Struct(s) => TypeDeclarationBodyItem::Struct(Box::new(s)),
        TypeDeclarationData::Enum(e) => TypeDeclarationBodyItem::Enum(Box::new(e)),
    });

    let initializer_parser =
        initializer_declaration_parser_internal().map(TypeDeclarationBodyItem::Initializer);
    let function_parser =
        function_declaration_parser_internal().map(TypeDeclarationBodyItem::Function);
    let subscript_parser =
        subscript_declaration_parser_internal().map(TypeDeclarationBodyItem::Subscript);
    let type_alias_parser =
        type_alias_declaration_parser_internal().map(TypeDeclarationBodyItem::TypeAlias);
    let field_parser = field_declaration_parser_internal().map(TypeDeclarationBodyItem::Field);
    let case_parser = enum_case_parser().map(TypeDeclarationBodyItem::EnumCase);

    module_parser
        .or(import_parser)
        .or(case_parser)
        .or(nested_type_parser)
        .or(initializer_parser)
        .or(type_alias_parser)
        .or(function_parser)
        .or(subscript_parser)
        .or(field_parser)
        .boxed()
}

/// Parser for a full enum declaration, taking the shared `type_parser` handle
/// so nested type declarations are parsed through the same recursive context.
///
/// Returns `EnumDeclarationData`. The unified `type_declaration_parser_internal`
/// wraps this in `TypeDeclarationData::Enum`.
pub(crate) fn enum_parser_with_recursion<'tokens, P>(
    type_parser: P,
) -> impl Parser<'tokens, ParserInput<'tokens>, EnumDeclarationData, ParserExtra<'tokens>> + Clone
where
    P: Parser<'tokens, ParserInput<'tokens>, TypeDeclarationData, ParserExtra<'tokens>>
        + Clone
        + 'tokens,
{
    let enum_body_parser = enum_body_item_parser(type_parser)
        .repeated()
        .collect::<Vec<_>>()
        .boxed();

    attribute_list_parser()
        .then(visibility_parser_internal())
        .then(indirect_modifier_parser())
        .then(token(Token::Enum))
        .then(identifier())
        .then(type_parameter_list_parser().or_not())
        .then(conformance_list_parser().or_not())
        .then(where_clause_parser().or_not())
        .then(token(Token::LBrace))
        .then(enum_body_parser)
        .then(token(Token::RBrace))
        .map(
            |(
                (
                    (
                        (
                            (
                                (
                                    (
                                        (((attributes, visibility), indirect), enum_span),
                                        name_span,
                                    ),
                                    type_params,
                                ),
                                conformances,
                            ),
                            where_clause,
                        ),
                        lbrace_span,
                    ),
                    body,
                ),
                rbrace_span,
            )| EnumDeclarationData {
                attributes,
                visibility,
                indirect,
                enum_span,
                name_span,
                type_params,
                conformances: conformances.map(|(colon_span, items)| ConformanceListData {
                    colon_span,
                    conformances: items,
                }),
                where_clause,
                lbrace_span,
                body,
                rbrace_span,
            },
        )
        .boxed()
}

/// Emit events for an indirect modifier
pub(crate) fn emit_indirect_modifier(sink: &mut EventSink, indirect_span: Span) {
    sink.start_node(SyntaxKind::IndirectModifier);
    sink.add_token(SyntaxKind::Indirect, indirect_span);
    sink.finish_node();
}

/// Emit events for an enum case parameter
///
/// Supports both named (`label: Type`) and unnamed (`Type`) forms.
pub(crate) fn emit_enum_case_parameter(sink: &mut EventSink, data: &EnumCaseParameterData) {
    sink.start_node(SyntaxKind::EnumCaseParameter);
    if let (Some(label), Some(colon)) = (&data.label, &data.colon) {
        emit_name(sink, label.clone());
        sink.add_token(SyntaxKind::Colon, colon.clone());
    }
    emit_ty_variant(sink, &data.ty);
    sink.finish_node();
}

/// Emit events for an enum case parameter list
pub(crate) fn emit_enum_case_parameter_list(
    sink: &mut EventSink,
    lparen: Span,
    parameters: &[EnumCaseParameterData],
    rparen: Span,
) {
    sink.start_node(SyntaxKind::EnumCaseParameterList);
    sink.add_token(SyntaxKind::LParen, lparen);
    for param in parameters {
        emit_enum_case_parameter(sink, param);
    }
    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
}

/// Emit events for an enum case declaration.
///
/// Destructures `EnumCaseDeclarationData` without a `..` rest pattern: adding
/// a field forces this function to stop compiling until the new field is
/// handled in emission.
pub fn emit_enum_case(sink: &mut EventSink, data: EnumCaseDeclarationData) {
    let EnumCaseDeclarationData {
        attributes,
        case_span,
        name_span,
        parameters,
    } = data;

    sink.start_node(SyntaxKind::EnumCaseDeclaration);
    emit_attribute_list(sink, &attributes);
    sink.add_token(SyntaxKind::Case, case_span);
    emit_name(sink, name_span);

    if let Some((lparen, ref params, rparen)) = parameters {
        emit_enum_case_parameter_list(sink, lparen, params, rparen);
    }

    sink.finish_node();
}

impl crate::event::EmitSyntax for EnumCaseDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_enum_case(sink, self);
    }
}

/// Emit events for an enum declaration.
///
/// Destructures `EnumDeclarationData` without a `..` rest pattern: adding a
/// field forces this function to stop compiling until the new field is
/// handled in emission.
pub fn emit_enum_declaration(sink: &mut EventSink, data: EnumDeclarationData) {
    let EnumDeclarationData {
        attributes,
        visibility,
        indirect,
        enum_span,
        name_span,
        type_params,
        conformances,
        where_clause,
        lbrace_span,
        body,
        rbrace_span,
    } = data;

    sink.start_node(SyntaxKind::EnumDeclaration);

    emit_attribute_list(sink, &attributes);
    emit_visibility(sink, visibility);

    if let Some(indirect_span) = indirect {
        emit_indirect_modifier(sink, indirect_span);
    }

    sink.add_token(SyntaxKind::Enum, enum_span);
    emit_name(sink, name_span);

    if let Some((lbracket, params, rbracket)) = type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    if let Some(conf) = conformances {
        emit_conformance_list(sink, conf.colon_span, &conf.conformances);
    }

    if let Some(wc) = where_clause {
        emit_where_clause(sink, wc);
    }

    sink.start_node(SyntaxKind::EnumBody);
    sink.add_token(SyntaxKind::LBrace, lbrace_span);

    for item in body {
        emit_type_declaration_body_item(sink, item);
    }

    sink.add_token(SyntaxKind::RBrace, rbrace_span);
    sink.finish_node(); // EnumBody

    sink.finish_node(); // EnumDeclaration
}

impl crate::event::EmitSyntax for EnumDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_enum_declaration(sink, self);
    }
}

/// Parse an enum declaration and emit events
///
/// This is the primary event-driven parser function for enum declarations.
pub fn parse_enum_declaration<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        enum_declaration_parser_internal(),
        emit_enum_declaration
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;

    /// Helper to parse source code and return an EnumDeclaration
    fn parse(source: &str) -> EnumDeclaration {
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect();
        let mut sink = EventSink::new(0);
        parse_enum_declaration(source, tokens.into_iter(), &mut sink);
        let tree = TreeBuilder::new(source, sink.into_events()).build();
        EnumDeclaration {
            syntax: tree,
            span: Span::new(0, 0..source.len()),
        }
    }

    /// Helper to check if a syntax node exists as a child
    fn has_child(decl: &EnumDeclaration, kind: SyntaxKind) -> bool {
        decl.syntax.children().any(|child| child.kind() == kind)
    }

    #[test]
    fn test_enum_declaration_basic() {
        let decl = parse("enum Color { }");
        assert_eq!(decl.name(), Some("Color".to_string()));
        assert_eq!(decl.visibility(), None);
        assert!(!decl.is_indirect());
        assert_eq!(decl.syntax.kind(), SyntaxKind::EnumDeclaration);
    }

    #[test]
    fn test_enum_declaration_with_visibility() {
        let decl = parse("public enum Direction { }");
        assert_eq!(decl.name(), Some("Direction".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
    }

    #[test]
    fn test_enum_declaration_with_indirect() {
        let decl = parse("indirect enum Tree { }");
        assert_eq!(decl.name(), Some("Tree".to_string()));
        assert!(decl.is_indirect());
    }

    #[test]
    fn test_enum_declaration_with_visibility_and_indirect() {
        let decl = parse("public indirect enum LinkedList { }");
        assert_eq!(decl.name(), Some("LinkedList".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_indirect());
    }

    #[test]
    fn test_enum_with_simple_case() {
        let decl = parse("enum Color { case Red }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].kind(), SyntaxKind::EnumCaseDeclaration);
    }

    #[test]
    fn test_enum_with_multiple_cases() {
        let decl = parse("enum Color { case Red case Green case Blue }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 3);
    }

    #[test]
    fn test_enum_case_with_associated_values() {
        let decl = parse("enum Result { case Success(value: Int) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        // Check that the case has parameters
        let case_node = &cases[0];
        let has_param_list = case_node
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(has_param_list);
    }

    #[test]
    fn test_enum_case_with_unnamed_parameter() {
        // Unnamed parameter: just Type without label
        let decl = parse("enum Option[T] { case Some(T) case None }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 2);

        // Check that Some has parameters
        let some_case = &cases[0];
        let has_param_list = some_case
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(has_param_list);

        // Check that None has no parameters
        let none_case = &cases[1];
        let none_has_params = none_case
            .children()
            .any(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(!none_has_params);
    }

    #[test]
    fn test_enum_case_with_multiple_unnamed_parameters() {
        let decl = parse("enum Pair[A, B] { case Both(A, B) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        let case_node = &cases[0];
        let param_list = case_node
            .children()
            .find(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(param_list.is_some());

        let param_count = param_list
            .unwrap()
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumCaseParameter)
            .count();
        assert_eq!(param_count, 2);
    }

    #[test]
    fn test_enum_case_with_multiple_associated_values() {
        let decl = parse("enum Event { case Click(x: Int, y: Int) }");
        let cases = decl.cases();
        assert_eq!(cases.len(), 1);

        // Count parameters in the case
        let case_node = &cases[0];
        let param_list = case_node
            .children()
            .find(|c| c.kind() == SyntaxKind::EnumCaseParameterList);
        assert!(param_list.is_some());

        let param_count = param_list
            .unwrap()
            .children()
            .filter(|c| c.kind() == SyntaxKind::EnumCaseParameter)
            .count();
        assert_eq!(param_count, 2);
    }

    #[test]
    fn test_enum_with_type_params() {
        let decl = parse("enum Option[T] { }");
        assert_eq!(decl.name(), Some("Option".to_string()));
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
    }

    #[test]
    fn test_enum_with_conformance() {
        let decl = parse("enum Status: Equatable { }");
        assert_eq!(decl.name(), Some("Status".to_string()));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
    }

    #[test]
    fn test_enum_with_where_clause() {
        let decl = parse("enum Container[T] where T: Equatable { }");
        assert_eq!(decl.name(), Some("Container".to_string()));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
    }

    #[test]
    fn test_enum_full_syntax() {
        let decl = parse(
            "public indirect enum Result[T, E]: Equatable where E: Error { case Success(value: T) case Failure(error: E) }",
        );
        assert_eq!(decl.name(), Some("Result".to_string()));
        assert_eq!(decl.visibility(), Some(SyntaxKind::Public));
        assert!(decl.is_indirect());
        assert!(has_child(&decl, SyntaxKind::TypeParameterList));
        assert!(has_child(&decl, SyntaxKind::ConformanceList));
        assert!(has_child(&decl, SyntaxKind::WhereClause));
        assert_eq!(decl.cases().len(), 2);
    }

    #[test]
    fn test_enum_with_function() {
        let decl = parse("enum Color { case Red func describe() -> String { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::FunctionDeclaration);
    }

    #[test]
    fn test_enum_with_initializer() {
        let decl = parse("enum Direction { case North init() { } }");
        let children = decl.children();
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::InitializerDeclaration);
    }

    #[test]
    fn test_enum_with_computed_property() {
        // Test multiline computed property in enum
        let decl = parse(
            "enum Optional[T] {
                case Some(T)
                case None
                public var isSome: Bool {
                    get {
                        true
                    }
                }
            }",
        );
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::FieldDeclaration);
    }

    #[test]
    fn test_enum_with_shorthand_computed_property() {
        // Test shorthand computed property (just a block, no get/set)
        let decl = parse(
            "enum Optional[T] {
                case Some(T)
                case None
                public var isSome: Bool {
                    match self {
                        .Some(_) => true,
                        .None => false
                    }
                }
            }",
        );
        let children = decl.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[1].kind(), SyntaxKind::EnumCaseDeclaration);
        assert_eq!(children[2].kind(), SyntaxKind::FieldDeclaration);
    }
}
