//! Kestrel Parser
//!
//! This crate provides event-driven parsing functionality for the Kestrel language.
//! All parsers emit events that are then converted into syntax trees using the `rowan` library
//! via `kestrel-syntax-tree`.
//!
//! # Event-Driven Architecture
//!
//! Parsers in this crate follow an event-driven architecture:
//! 1. Parsers take an `EventSink` and emit parsing events (StartNode, AddToken, FinishNode, Error)
//! 2. Events are collected in the sink
//! 3. A `TreeBuilder` converts events into a concrete syntax tree
//!
//! # Example
//!
//! ```no_run
//! use kestrel_parser::{parse_module_declaration_from_source, event::EventSink};
//! use kestrel_lexer::lex;
//!
//! let source = "module A.B.C";
//! let tokens: Vec<_> = lex(source, 0)
//!     .filter_map(|t| t.ok())
//!     .map(|spanned| (spanned.value, spanned.span))
//!     .collect();
//!
//! // High-level convenience function
//! let decl = parse_module_declaration_from_source(source, tokens.into_iter());
//!
//! // Access the syntax tree
//! println!("Syntax tree: {:?}", decl.syntax);
//!
//! // Access the parsed data via the syntax tree
//! let path = decl.path();
//! println!("Module path: {:?}", path.segment_names());
//! ```
//!
//! # Low-Level Event-Driven API
//!
//! For more control, you can use the event-driven API directly:
//!
//! ```no_run
//! use kestrel_parser::{module::parse_module_declaration, event::{EventSink, TreeBuilder}};
//! use kestrel_parser::module::ModuleDeclaration;
//! use kestrel_lexer::lex;
//! use kestrel_span::Span;
//!
//! let source = "module A.B.C";
//! let file_id = 0;
//! let tokens: Vec<_> = lex(source, file_id)
//!     .filter_map(|t| t.ok())
//!     .map(|spanned| (spanned.value, spanned.span))
//!     .collect();
//!
//! let mut sink = EventSink::new(file_id);
//! parse_module_declaration(source, tokens.into_iter(), &mut sink);
//!
//! // Build tree from events
//! let tree = TreeBuilder::new(source, sink.into_events()).build();
//! let decl = ModuleDeclaration { syntax: tree, span: Span::new(file_id, 0..source.len()) };
//! ```

pub mod attribute;
pub mod block;
pub mod common;
pub mod declaration_item;
pub mod enum_decl;
pub mod event;
pub mod expr;
pub mod extension;
pub mod field;
pub mod function;
pub mod import;
pub mod input;
pub mod module;
pub mod parser;
pub mod pattern;
pub mod protocol;
pub mod stmt;
pub mod r#struct;
pub mod subscript;
pub mod ty;
pub mod type_alias;
pub mod type_decl;
pub mod type_param;

use event::{EventSink, TreeBuilder};
use kestrel_lexer::Token;
use kestrel_span::Span;

// Re-export commonly used types
pub use block::CodeBlock;
pub use declaration_item::DeclarationItem;
pub use enum_decl::EnumDeclaration;
pub use expr::Expression;
pub use extension::ExtensionDeclaration;
pub use field::FieldDeclaration;
pub use function::FunctionDeclaration;
pub use import::ImportDeclaration;
pub use module::{ModuleDeclaration, ModulePath};
pub use protocol::ProtocolDeclaration;
pub use stmt::Statement;
pub use r#struct::StructDeclaration;
pub use subscript::SubscriptDeclaration;
pub use ty::TyExpression;
pub use type_alias::TypeAliasDeclaration;

// Re-export event-driven parse functions
pub use block::parse_code_block;
pub use declaration_item::{parse_declaration_item, parse_source_file};
pub use enum_decl::parse_enum_declaration;
pub use expr::parse_expr;
pub use extension::parse_extension_declaration;
pub use field::parse_field_declaration;
pub use function::parse_function_declaration;
pub use import::parse_import_declaration;
pub use module::{parse_module_declaration, parse_module_path};
pub use protocol::parse_protocol_declaration;
pub use stmt::parse_stmt;
pub use r#struct::parse_struct_declaration;
pub use subscript::parse_subscript_declaration;
pub use ty::parse_ty;
pub use type_alias::parse_type_alias_declaration;

// Re-export Parser API
pub use parser::{
    ParseError, ParseErrorKind, ParseResult, Parser, format_token_for_display, suggest_fix,
};

/// Extract file_id from the first token, defaulting to 0 if no tokens
fn extract_file_id<I>(tokens: &I) -> usize
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    tokens
        .clone()
        .next()
        .map(|(_, span)| span.file_id)
        .unwrap_or(0)
}

/// Convenience function to parse a module declaration from source and tokens
/// Returns a fully built ModuleDeclaration with its syntax tree
pub fn parse_module_declaration_from_source<I>(source: &str, tokens: I) -> ModuleDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    module::parse_module_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    ModuleDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a module path from source and tokens
/// Returns a fully built ModulePath with its syntax tree
pub fn parse_module_path_from_source<I>(source: &str, tokens: I) -> ModulePath
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    module::parse_module_path(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    ModulePath { syntax: tree }
}

/// Convenience function to parse an import declaration from source and tokens
/// Returns a fully built ImportDeclaration with its syntax tree
pub fn parse_import_declaration_from_source<I>(source: &str, tokens: I) -> ImportDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    import::parse_import_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    ImportDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a struct declaration from source and tokens
/// Returns a fully built StructDeclaration with its syntax tree
pub fn parse_struct_declaration_from_source<I>(source: &str, tokens: I) -> StructDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    r#struct::parse_struct_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    StructDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a type alias declaration from source and tokens
/// Returns a fully built TypeAliasDeclaration with its syntax tree
pub fn parse_type_alias_declaration_from_source<I>(source: &str, tokens: I) -> TypeAliasDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    type_alias::parse_type_alias_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    TypeAliasDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a function declaration from source and tokens
/// Returns a fully built FunctionDeclaration with its syntax tree
pub fn parse_function_declaration_from_source<I>(source: &str, tokens: I) -> FunctionDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    function::parse_function_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    FunctionDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a protocol declaration from source and tokens
/// Returns a fully built ProtocolDeclaration with its syntax tree
pub fn parse_protocol_declaration_from_source<I>(source: &str, tokens: I) -> ProtocolDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    protocol::parse_protocol_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    ProtocolDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse an enum declaration from source and tokens
/// Returns a fully built EnumDeclaration with its syntax tree
pub fn parse_enum_declaration_from_source<I>(source: &str, tokens: I) -> EnumDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    enum_decl::parse_enum_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    EnumDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a subscript declaration from source and tokens
/// Returns a fully built SubscriptDeclaration with its syntax tree
pub fn parse_subscript_declaration_from_source<I>(source: &str, tokens: I) -> SubscriptDeclaration
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    subscript::parse_subscript_declaration(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    SubscriptDeclaration {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse a declaration item from source and tokens
/// Returns events - caller must decide how to convert to DeclarationItem
pub fn parse_declaration_item_events<I>(source: &str, tokens: I) -> Vec<event::Event>
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    declaration_item::parse_declaration_item(source, tokens, &mut sink);
    sink.into_events()
}

/// Convenience function to parse a source file (multiple declarations) from source and tokens
/// Returns a ParseResult with the syntax tree and any errors
pub fn parse_source_file_from_source<I>(source: &str, tokens: I) -> ParseResult
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    Parser::parse(source, tokens, parse_source_file, file_id)
}

/// Convenience function to parse a type expression from source and tokens
/// Returns a fully built TyExpression with its syntax tree
pub fn parse_ty_from_source<I>(source: &str, tokens: I) -> TyExpression
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    ty::parse_ty(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    TyExpression {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}

/// Convenience function to parse an expression from source and tokens
/// Returns a fully built Expression with its syntax tree
pub fn parse_expr_from_source<I>(source: &str, tokens: I) -> Expression
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    let file_id = extract_file_id(&tokens);
    let mut sink = EventSink::new(file_id);
    expr::parse_expr(source, tokens, &mut sink);
    let tree = TreeBuilder::new(source, sink.into_events()).build();
    Expression {
        syntax: tree,
        span: Span::new(file_id, 0..source.len()),
    }
}
