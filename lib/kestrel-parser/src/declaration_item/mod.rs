//! Declaration item parsing
//!
//! This module acts as a ROUTER that dispatches to the appropriate module's parser.
//! It does NOT contain its own parsing or emitting implementations.
//!
//! Following the principles in principles.md:
//! - Each declaration type has its parser and emitter in its own module
//! - This module aggregates all declaration types and routes to them

use chumsky::prelude::*;
use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};

use crate::enum_decl::{EnumDeclaration, EnumDeclarationData, emit_enum_declaration};
use crate::event::EventSink;
use crate::extension::{
    ExtensionDeclaration, ExtensionDeclarationData, emit_extension_declaration,
    extension_declaration_parser_internal,
};
use crate::field::{
    FieldDeclaration, FieldDeclarationData, emit_field_declaration,
    field_declaration_parser_internal,
};
use crate::function::{
    FunctionDeclaration, FunctionDeclarationData, emit_function_declaration,
    function_declaration_parser_internal,
};
use crate::import::{ImportDeclaration, emit_import_declaration, import_declaration_parser_internal};
use crate::input::{ParserExtra, ParserInput};
use crate::parse_and_emit;
use crate::module::{
    ModuleDeclaration, emit_module_declaration, module_declaration_parser_internal,
};
use crate::protocol::{
    ProtocolDeclaration, ProtocolDeclarationData, emit_protocol_declaration,
    protocol_declaration_parser_internal,
};
use crate::r#struct::{StructDeclaration, StructDeclarationData, emit_struct_declaration};
use crate::subscript::{
    SubscriptDeclaration, SubscriptDeclarationData, emit_subscript_declaration,
    subscript_declaration_parser_internal,
};
use crate::type_alias::{
    TypeAliasDeclaration, TypeAliasDeclarationData, emit_type_alias_declaration,
    type_alias_declaration_parser_internal,
};
use crate::type_decl::{TypeDeclarationData, type_declaration_parser_internal};

/// Represents a declaration item - a top-level unit of code in a Kestrel file
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclarationItem {
    Module(ModuleDeclaration),
    Import(ImportDeclaration),
    Protocol(ProtocolDeclaration),
    Struct(StructDeclaration),
    Enum(EnumDeclaration),
    Extension(ExtensionDeclaration),
    Field(FieldDeclaration),
    Function(FunctionDeclaration),
    Subscript(SubscriptDeclaration),
    TypeAlias(TypeAliasDeclaration),
}

impl DeclarationItem {
    /// Get the span of this declaration item
    pub fn span(&self) -> &Span {
        match self {
            DeclarationItem::Module(decl) => &decl.span,
            DeclarationItem::Import(decl) => &decl.span,
            DeclarationItem::Protocol(decl) => &decl.span,
            DeclarationItem::Struct(decl) => &decl.span,
            DeclarationItem::Enum(decl) => &decl.span,
            DeclarationItem::Extension(decl) => &decl.span,
            DeclarationItem::Field(decl) => &decl.span,
            DeclarationItem::Function(decl) => &decl.span,
            DeclarationItem::Subscript(decl) => &decl.span,
            DeclarationItem::TypeAlias(decl) => &decl.span,
        }
    }

    /// Get the syntax tree for this declaration item
    pub fn syntax(&self) -> &SyntaxNode {
        match self {
            DeclarationItem::Module(decl) => &decl.syntax,
            DeclarationItem::Import(decl) => &decl.syntax,
            DeclarationItem::Protocol(decl) => &decl.syntax,
            DeclarationItem::Struct(decl) => &decl.syntax,
            DeclarationItem::Enum(decl) => &decl.syntax,
            DeclarationItem::Extension(decl) => &decl.syntax,
            DeclarationItem::Field(decl) => &decl.syntax,
            DeclarationItem::Function(decl) => &decl.syntax,
            DeclarationItem::Subscript(decl) => &decl.syntax,
            DeclarationItem::TypeAlias(decl) => &decl.syntax,
        }
    }
}

/// Parsed data for a declaration item - routes to the appropriate module's data type
#[derive(Debug, Clone)]
enum DeclarationItemData {
    Module(Span, Vec<Span>),
    Import(
        Span,
        Vec<Span>,
        Option<Span>,
        Option<Vec<(Span, Option<Span>)>>,
    ),
    Protocol(ProtocolDeclarationData),
    Struct(StructDeclarationData),
    Enum(EnumDeclarationData),
    Extension(ExtensionDeclarationData),
    Field(FieldDeclarationData),
    Function(FunctionDeclarationData),
    Subscript(SubscriptDeclarationData),
    TypeAlias(TypeAliasDeclarationData),
}

/// Parser that skips trivia tokens
fn skip_trivia<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, (), ParserExtra<'tokens>> + Clone {
    any()
        .filter(|token: &Token| {
            matches!(
                token,
                Token::Whitespace | Token::Newline | Token::LineComment | Token::BlockComment
            )
        })
        .repeated()
        .ignored()
}

/// Internal Chumsky parser for a single declaration item
///
/// This parser ROUTES to the module-specific parsers - it does not implement
/// parsing logic itself.
fn declaration_item_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, DeclarationItemData, ParserExtra<'tokens>> + Clone {
    // Route to module-specific parsers
    let module_parser = module_declaration_parser_internal()
        .map(|(span, path)| DeclarationItemData::Module(span, path));

    let import_parser =
        import_declaration_parser_internal().map(|(import_span, path, alias, items)| {
            DeclarationItemData::Import(import_span, path, alias, items)
        });

    let protocol_parser = protocol_declaration_parser_internal().map(DeclarationItemData::Protocol);

    // Use the unified type declaration parser for both struct and enum
    // This uses a single recursive context to avoid stack overflow on deeply nested types
    let type_declaration_parser = type_declaration_parser_internal().map(|data| match data {
        TypeDeclarationData::Struct(s) => DeclarationItemData::Struct(s),
        TypeDeclarationData::Enum(e) => DeclarationItemData::Enum(e),
    });

    let extension_parser =
        extension_declaration_parser_internal().map(DeclarationItemData::Extension);

    let function_parser = function_declaration_parser_internal().map(DeclarationItemData::Function);

    let subscript_parser =
        subscript_declaration_parser_internal().map(DeclarationItemData::Subscript);

    let field_parser = field_declaration_parser_internal().map(DeclarationItemData::Field);

    let type_alias_parser =
        type_alias_declaration_parser_internal().map(DeclarationItemData::TypeAlias);

    // Try parsers in order - more specific first
    module_parser
        .or(import_parser)
        .or(protocol_parser)
        .or(type_declaration_parser) // Handles both struct and enum
        .or(extension_parser)
        .or(function_parser)
        .or(subscript_parser)
        .or(field_parser)
        .or(type_alias_parser)
        .boxed()
}

/// Internal Chumsky parser for multiple declaration items
fn declaration_items_parser_internal<'tokens>()
-> impl Parser<'tokens, ParserInput<'tokens>, Vec<DeclarationItemData>, ParserExtra<'tokens>> + Clone
{
    declaration_item_parser_internal()
        .repeated()
        .at_least(0)
        .collect()
        .then_ignore(skip_trivia())
        .boxed()
}

/// Emit a declaration item to the event sink
///
/// Routes to the appropriate module's emitter.
fn emit_declaration_item(sink: &mut EventSink, data: DeclarationItemData) {
    match data {
        DeclarationItemData::Module(module_span, path_segments) => {
            emit_module_declaration(sink, module_span, &path_segments);
        },
        DeclarationItemData::Import(import_span, path_segments, alias, items) => {
            emit_import_declaration(sink, import_span, &path_segments, alias, items);
        },
        DeclarationItemData::Protocol(data) => {
            emit_protocol_declaration(sink, data);
        },
        DeclarationItemData::Struct(data) => {
            emit_struct_declaration(sink, data);
        },
        DeclarationItemData::Enum(data) => {
            emit_enum_declaration(sink, data);
        },
        DeclarationItemData::Extension(data) => {
            emit_extension_declaration(sink, data);
        },
        DeclarationItemData::Function(data) => {
            emit_function_declaration(sink, data);
        },
        DeclarationItemData::Subscript(data) => {
            emit_subscript_declaration(sink, data);
        },
        DeclarationItemData::Field(data) => {
            emit_field_declaration(sink, data);
        },
        DeclarationItemData::TypeAlias(data) => {
            emit_type_alias_declaration(sink, data);
        },
    }
}

/// Parse a declaration item and emit events
///
/// This is the primary event-driven parser function.
/// Uses the same Chumsky declaration router as source-file parsing.
pub fn parse_declaration_item<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    parse_and_emit!(
        source,
        tokens,
        sink,
        declaration_item_parser_internal().then_ignore(skip_trivia()),
        emit_declaration_item
    );
}

/// Parse a source file (multiple declaration items) and emit events
///
/// Creates a SourceFile root node containing all declarations.
pub fn parse_source_file<I>(source: &str, tokens: I, sink: &mut EventSink)
where
    I: Iterator<Item = (Token, Span)> + Clone,
{
    fn emit_items(sink: &mut EventSink, items: Vec<DeclarationItemData>) {
        for item_data in items {
            emit_declaration_item(sink, item_data);
        }
    }
    sink.start_node(SyntaxKind::SourceFile);
    parse_and_emit!(
        source,
        tokens,
        sink,
        declaration_items_parser_internal(),
        emit_items
    );
    sink.finish_node();
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_lexer::lex;
    use kestrel_syntax_tree::SyntaxKind;

    #[test]
    fn test_declaration_item_module() {
        let source = "module A.B.C";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_declaration_item(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let has_module = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::ModuleDeclaration)
        });
        assert!(has_module, "Should have parsed as module declaration");
    }

    #[test]
    fn test_declaration_item_import() {
        let source = "import A.B.C";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_declaration_item(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let has_import = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::ImportDeclaration)
        });
        assert!(has_import, "Should have parsed as import declaration");
    }

    #[test]
    fn test_generic_struct() {
        let source = "module Test\nstruct Box[T] {}";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let has_type_param_list = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::TypeParameterList)
        });
        assert!(
            has_type_param_list,
            "Should have TypeParameterList in syntax tree"
        );
    }

    #[test]
    fn test_generic_protocol() {
        let source = "module Test\nprotocol Collection[T] {}";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let has_type_param_list = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::TypeParameterList)
        });
        let has_protocol = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::ProtocolDeclaration)
        });
        assert!(
            has_protocol,
            "Should have ProtocolDeclaration in syntax tree"
        );
        assert!(
            has_type_param_list,
            "Should have TypeParameterList in syntax tree"
        );
    }

    #[test]
    fn test_generic_function() {
        let source = "module Test\nfunc identity[T](value: T) -> T {}";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let has_type_param_list = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::TypeParameterList)
        });
        let has_function = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::FunctionDeclaration)
        });
        assert!(
            has_function,
            "Should have FunctionDeclaration in syntax tree"
        );
        assert!(
            has_type_param_list,
            "Should have TypeParameterList in syntax tree"
        );
    }

    #[test]
    fn test_struct_with_fields_and_functions() {
        let source = "module Test\nstruct Person { let name: String func greet() {} }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let has_field = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::FieldDeclaration)
        });
        let has_function = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::FunctionDeclaration)
        });
        assert!(has_field, "Should have FieldDeclaration in syntax tree");
        assert!(
            has_function,
            "Should have FunctionDeclaration in syntax tree"
        );
    }

    #[test]
    fn test_struct_with_two_fields_and_initializer() {
        let source = "module Test\nstruct Point { var x: Int var y: Int init() {} }";
        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();
        let field_count = events.iter().filter(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::FieldDeclaration)
        }).count();
        let has_init = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::InitializerDeclaration)
        });
        let has_struct = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::StructDeclaration)
        });
        assert!(has_struct, "Should have StructDeclaration in syntax tree");
        assert_eq!(
            field_count, 2,
            "Should have 2 FieldDeclarations in syntax tree"
        );
        assert!(
            has_init,
            "Should have InitializerDeclaration in syntax tree"
        );
    }

    #[test]
    fn test_extension_followed_by_function() {
        let source = r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point { func sum() -> Int { return self.x + self.y; } }
            func test() -> Int { return 1; }"#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();

        // Count function declarations - should be 2 (one in extension, one at module level)
        let func_count = events.iter().filter(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::FunctionDeclaration)
        }).count();

        // Count function bodies - should also be 2
        let func_body_count = events.iter().filter(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::FunctionBody)
        }).count();

        let has_extension = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::ExtensionDeclaration)
        });

        assert!(has_extension, "Should have ExtensionDeclaration");
        assert_eq!(
            func_count, 2,
            "Should have 2 FunctionDeclarations (one in extension, one after)"
        );
        assert_eq!(
            func_body_count, 2,
            "Each function should have a FunctionBody"
        );
    }

    #[test]
    fn test_extension_followed_by_function_tree_structure() {
        use crate::event::TreeBuilder;

        let source = r#"module Test
            struct Point { var x: Int; var y: Int }
            extend Point { func sum() -> Int { return self.x + self.y; } }
            func test() -> Int { let p = 1; return p; }"#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();

        // Find all FunctionDeclaration nodes
        fn find_nodes(
            node: &kestrel_syntax_tree::SyntaxNode,
            kind: SyntaxKind,
        ) -> Vec<kestrel_syntax_tree::SyntaxNode> {
            let mut result = Vec::new();
            if node.kind() == kind {
                result.push(node.clone());
            }
            for child in node.children() {
                result.extend(find_nodes(&child, kind));
            }
            result
        }

        let func_decls = find_nodes(&tree, SyntaxKind::FunctionDeclaration);
        assert_eq!(func_decls.len(), 2, "Should have 2 FunctionDeclarations");

        // Each FunctionDeclaration should have a FunctionBody child
        for (i, func_decl) in func_decls.iter().enumerate() {
            let has_body = func_decl
                .children()
                .any(|c| c.kind() == SyntaxKind::FunctionBody);
            assert!(
                has_body,
                "FunctionDeclaration #{} should have FunctionBody as direct child",
                i
            );
        }
    }

    #[test]
    fn test_enum_declaration_in_source_file() {
        let source = r#"module Test
            enum Color { case Red case Green case Blue }
            enum Option[T] { case Some(value: T) case None }
            "#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();

        // Count enum declarations - should be 2
        let enum_count = events.iter().filter(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::EnumDeclaration)
        }).count();

        // Count enum case declarations - should be 5 (3 for Color + 2 for Option)
        let case_count = events.iter().filter(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::EnumCaseDeclaration)
        }).count();

        assert_eq!(enum_count, 2, "Should have 2 EnumDeclarations");
        assert_eq!(case_count, 5, "Should have 5 EnumCaseDeclarations");
    }

    #[test]
    fn test_enum_with_struct_in_source_file() {
        let source = r#"module Test
            struct Point { var x: Int; var y: Int }
            enum Shape {
                case Circle(center: Point, radius: Int)
                case Rectangle(origin: Point, width: Int, height: Int)
            }
            "#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();

        let has_struct = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::StructDeclaration)
        });
        let has_enum = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::EnumDeclaration)
        });

        assert!(has_struct, "Should have StructDeclaration in syntax tree");
        assert!(has_enum, "Should have EnumDeclaration in syntax tree");
    }

    #[test]
    fn test_shorthand_in_assignment_expression() {
        use crate::event::TreeBuilder;

        // Test that .CaseName works in assignment expressions
        let source = r#"module Test
            enum Status {
                case Pending
                case Active
                case Complete
            }

            func test() {
                var status: Status = .Pending;
                status = .Active;
                status = .Complete;
            }
        "#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let tree = TreeBuilder::new(source, sink.into_events()).build();

        fn find_nodes(
            node: &kestrel_syntax_tree::SyntaxNode,
            kind: SyntaxKind,
        ) -> Vec<kestrel_syntax_tree::SyntaxNode> {
            let mut result = Vec::new();
            if node.kind() == kind {
                result.push(node.clone());
            }
            for child in node.children() {
                result.extend(find_nodes(&child, kind));
            }
            result
        }

        let func_decls = find_nodes(&tree, SyntaxKind::FunctionDeclaration);
        assert_eq!(func_decls.len(), 1, "Should have 1 FunctionDeclaration");

        for func in &func_decls {
            let has_body = func
                .children()
                .any(|c| c.kind() == SyntaxKind::FunctionBody);
            assert!(has_body, "Function should have a body");
        }

        let implicit_accesses = find_nodes(&tree, SyntaxKind::ExprImplicitMemberAccess);
        assert_eq!(
            implicit_accesses.len(),
            3,
            "Should have 3 ExprImplicitMemberAccess (.Pending, .Active, .Complete)"
        );
    }

    #[test]
    fn test_function_with_deinit_statement() {
        // Test parsing a full source file with a function containing a deinit statement
        let source = r#"module Test
            func example() {
                var x: Int = 42;
                deinit x;
            }
        "#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();

        // Check for parse errors
        let errors: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                crate::event::Event::Error { message, .. } => Some(message.clone()),
                _ => None,
            })
            .collect();

        assert!(errors.is_empty(), "Got parse errors: {:?}", errors);

        // Check that we have a DeinitStatement in the tree
        let has_deinit = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::DeinitStatement)
        });
        assert!(has_deinit, "Should have DeinitStatement in syntax tree");
    }

    #[test]
    fn test_struct_with_nested_enum_with_nested_enum() {
        // This test verifies that deeply nested type declarations parse correctly
        // It was added to catch stack overflow issues with mutually recursive parsers
        let source = r#"module Test
            struct Level1 {
                enum Level2 {
                    case Value
                    enum Level3 {
                        case DeepValue
                    }
                }
            }
        "#;

        let tokens: Vec<_> = lex(source, 0)
            .filter_map(|t| t.ok())
            .map(|spanned| (spanned.value, spanned.span))
            .collect::<Vec<_>>();

        let mut sink = EventSink::new(0);
        parse_source_file(source, tokens.into_iter(), &mut sink);

        let events = sink.events();

        let has_struct = events.iter().any(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::StructDeclaration)
        });
        let enum_count = events.iter().filter(|e| {
            matches!(e, crate::event::Event::StartNode(kind) if *kind == SyntaxKind::EnumDeclaration)
        }).count();

        assert!(has_struct, "Should have StructDeclaration in syntax tree");
        assert_eq!(
            enum_count, 2,
            "Should have 2 EnumDeclarations in syntax tree"
        );
    }
}
