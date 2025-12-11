//! Common event emitters shared across multiple parsers
//!
//! This module provides reusable event emission functions that build syntax
//! trees by emitting events through an EventSink. These functions are used
//! by multiple parser modules to avoid code duplication.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use super::data::{
    AssociatedTypeBoundsData, AssociatedTypeTargetData,
    ExtensionBodyItem, ExtensionDeclarationData,
    FieldDeclarationData, FunctionDeclarationData, InitializerDeclarationData, ParameterData,
    ProtocolBodyItem, ProtocolDeclarationData, ReceiverModifier, StructBodyItem, StructDeclarationData,
    TypeAliasDeclarationData,
};
use crate::block::emit_code_block;
use crate::event::EventSink;
use crate::ty::emit_ty_variant;
use crate::type_param::{emit_conformance_list, emit_type_parameter_list, emit_where_clause};

// =============================================================================
// Module and Import Emitters
// =============================================================================

/// Emit events for a module path
///
/// Emits a ModulePath node containing identifier tokens separated by dot tokens.
pub fn emit_module_path(sink: &mut EventSink, segments: &[Span]) {
    sink.start_node(SyntaxKind::ModulePath);
    for (i, span) in segments.iter().enumerate() {
        if i > 0 {
            // Emit dot token between segments
            sink.add_token(SyntaxKind::Dot, span.start - 1..span.start);
        }
        sink.add_token(SyntaxKind::Identifier, span.clone());
    }
    sink.finish_node();
}

/// Emit events for a module declaration
pub fn emit_module_declaration(sink: &mut EventSink, module_span: Span, path_segments: &[Span]) {
    sink.start_node(SyntaxKind::ModuleDeclaration);
    sink.add_token(SyntaxKind::Module, module_span);
    emit_module_path(sink, path_segments);
    sink.finish_node();
}

/// Emit events for an import declaration
pub fn emit_import_declaration(
    sink: &mut EventSink,
    import_span: Span,
    path_segments: &[Span],
    alias: Option<Span>,
    items: Option<Vec<(Span, Option<Span>)>>,
) {
    sink.start_node(SyntaxKind::ImportDeclaration);
    sink.add_token(SyntaxKind::Import, import_span);

    emit_module_path(sink, path_segments);

    if let Some(items_list) = &items {
        let last_segment_end = path_segments.last().unwrap().end;
        sink.add_token(SyntaxKind::Dot, last_segment_end..last_segment_end + 1);
        sink.add_token(
            SyntaxKind::LParen,
            last_segment_end + 1..last_segment_end + 2,
        );

        for (i, (name_span, alias_span)) in items_list.iter().enumerate() {
            if i > 0 {
                let prev_end = if let Some(alias_s) =
                    items_list.get(i - 1).and_then(|(_, alias)| alias.as_ref())
                {
                    alias_s.end
                } else {
                    items_list.get(i - 1).unwrap().0.end
                };
                sink.add_token(SyntaxKind::Comma, prev_end..prev_end + 1);
            }

            sink.start_node(SyntaxKind::ImportItem);
            sink.add_token(SyntaxKind::Identifier, name_span.clone());

            if let Some(alias_s) = alias_span {
                let as_start = name_span.end + 1;
                sink.add_token(SyntaxKind::As, as_start..as_start + 2);
                sink.add_token(SyntaxKind::Identifier, alias_s.clone());
            }
            sink.finish_node();
        }

        let last_item = items_list.last().unwrap();
        let last_item_end = if let Some(alias_s) = &last_item.1 {
            alias_s.end
        } else {
            last_item.0.end
        };
        sink.add_token(SyntaxKind::RParen, last_item_end..last_item_end + 1);
    } else if let Some(alias_span) = alias {
        let as_start = path_segments.last().unwrap().end + 1;
        sink.add_token(SyntaxKind::As, as_start..as_start + 2);
        sink.add_token(SyntaxKind::Identifier, alias_span);
    }

    sink.finish_node();
}

// =============================================================================
// Visibility and Modifier Emitters
// =============================================================================

/// Emit events for a visibility modifier
pub fn emit_visibility(sink: &mut EventSink, visibility: Option<(Token, Span)>) {
    sink.start_node(SyntaxKind::Visibility);
    if let Some((vis_token, vis_span)) = visibility {
        let vis_kind = match vis_token {
            Token::Public => SyntaxKind::Public,
            Token::Private => SyntaxKind::Private,
            Token::Internal => SyntaxKind::Internal,
            Token::Fileprivate => SyntaxKind::Fileprivate,
            _ => unreachable!("visibility should only contain visibility tokens"),
        };
        sink.add_token(vis_kind, vis_span);
    }
    sink.finish_node();
}

/// Emit events for a static modifier
pub fn emit_static_modifier(sink: &mut EventSink, static_span: Option<Span>) {
    if let Some(span) = static_span {
        sink.start_node(SyntaxKind::StaticModifier);
        sink.add_token(SyntaxKind::Static, span);
        sink.finish_node();
    }
}

/// Emit events for a name node
pub fn emit_name(sink: &mut EventSink, name_span: Span) {
    sink.start_node(SyntaxKind::Name);
    sink.add_token(SyntaxKind::Identifier, name_span);
    sink.finish_node();
}

// =============================================================================
// Parameter Emitters
// =============================================================================

/// Emit events for a parameter list
pub fn emit_parameter_list(
    sink: &mut EventSink,
    lparen: Span,
    parameters: Vec<ParameterData>,
    rparen: Span,
) {
    sink.start_node(SyntaxKind::ParameterList);
    sink.add_token(SyntaxKind::LParen, lparen);

    for param in parameters {
        emit_parameter(sink, param);
    }

    sink.add_token(SyntaxKind::RParen, rparen);
    sink.finish_node();
}

/// Emit events for a single parameter
pub fn emit_parameter(sink: &mut EventSink, param: ParameterData) {
    sink.start_node(SyntaxKind::Parameter);

    if let Some(label_span) = param.label {
        emit_name(sink, label_span);
    }

    emit_name(sink, param.bind_name);
    sink.add_token(SyntaxKind::Colon, param.colon);
    emit_ty_variant(sink, &param.ty);

    sink.finish_node();
}

/// Emit events for a return type
pub fn emit_return_type(sink: &mut EventSink, arrow_span: Span, return_ty: crate::ty::TyVariant) {
    sink.start_node(SyntaxKind::ReturnType);
    sink.add_token(SyntaxKind::Arrow, arrow_span);
    emit_ty_variant(sink, &return_ty);
    sink.finish_node();
}

/// Emit events for a function body (wraps a code block)
pub fn emit_function_body(sink: &mut EventSink, block: &crate::block::CodeBlockData) {
    sink.start_node(SyntaxKind::FunctionBody);
    emit_code_block(sink, block);
    sink.finish_node();
}

// =============================================================================
// Declaration Emitters - Single Source of Truth
// =============================================================================

/// Emit events for a function declaration
///
/// This is the single source of truth for function declaration emission.
pub fn emit_function_declaration(sink: &mut EventSink, data: FunctionDeclarationData) {
    sink.start_node(SyntaxKind::FunctionDeclaration);

    emit_visibility(sink, data.visibility);
    emit_static_modifier(sink, data.is_static);

    // Emit receiver modifier (mutating/consuming) if present
    if let Some((modifier, span)) = data.receiver_modifier {
        let kind = match modifier {
            ReceiverModifier::Mutating => SyntaxKind::Mutating,
            ReceiverModifier::Consuming => SyntaxKind::Consuming,
        };
        sink.add_token(kind, span);
    }

    sink.add_token(SyntaxKind::Func, data.fn_span);
    emit_name(sink, data.name_span);

    if let Some((lbracket, params, rbracket)) = data.type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    emit_parameter_list(sink, data.lparen, data.parameters, data.rparen);

    if let Some((arrow_span, return_ty)) = data.return_type {
        emit_return_type(sink, arrow_span, return_ty);
    }

    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    if let Some(ref block) = data.body {
        emit_function_body(sink, block);
    }

    sink.finish_node();
}

/// Emit events for a field declaration
///
/// This is the single source of truth for field declaration emission.
pub fn emit_field_declaration(sink: &mut EventSink, data: FieldDeclarationData) {
    sink.start_node(SyntaxKind::FieldDeclaration);

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

    // Emit optional trailing semicolon
    if let Some(semicolon_span) = data.semicolon {
        sink.add_token(SyntaxKind::Semicolon, semicolon_span);
    }

    sink.finish_node();
}

/// Emit events for an initializer declaration
///
/// This is the single source of truth for initializer declaration emission.
pub fn emit_initializer_declaration(sink: &mut EventSink, data: InitializerDeclarationData) {
    sink.start_node(SyntaxKind::InitializerDeclaration);

    emit_visibility(sink, data.visibility);
    sink.add_token(SyntaxKind::Init, data.init_span);
    emit_parameter_list(sink, data.lparen, data.parameters, data.rparen);

    if let Some(ref block) = data.body {
        emit_function_body(sink, block);
    }

    sink.finish_node();
}

/// Emit events for a struct declaration
///
/// This is the single source of truth for struct declaration emission.
pub fn emit_struct_declaration(sink: &mut EventSink, data: StructDeclarationData) {
    sink.start_node(SyntaxKind::StructDeclaration);

    emit_visibility(sink, data.visibility);
    sink.add_token(SyntaxKind::Struct, data.struct_span);
    emit_name(sink, data.name_span);

    if let Some((lbracket, params, rbracket)) = data.type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    if let Some(conf) = data.conformances {
        emit_conformance_list(sink, conf.colon_span, &conf.conformances);
    }

    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    sink.start_node(SyntaxKind::StructBody);
    sink.add_token(SyntaxKind::LBrace, data.lbrace_span);

    for item in data.body {
        emit_struct_body_item(sink, item);
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace_span);
    sink.finish_node(); // StructBody

    sink.finish_node(); // StructDeclaration
}

/// Emit events for a struct body item
fn emit_struct_body_item(sink: &mut EventSink, item: StructBodyItem) {
    match item {
        StructBodyItem::Field(data) => emit_field_declaration(sink, data),
        StructBodyItem::Function(data) => emit_function_declaration(sink, data),
        StructBodyItem::Initializer(data) => emit_initializer_declaration(sink, data),
        StructBodyItem::Struct(data) => emit_struct_declaration(sink, data),
        StructBodyItem::TypeAlias(data) => emit_type_alias_declaration(sink, data),
        StructBodyItem::Module(module_span, path_segments) => {
            emit_module_declaration(sink, module_span, &path_segments);
        }
        StructBodyItem::Import(import_span, path_segments, alias, items) => {
            emit_import_declaration(sink, import_span, &path_segments, alias, items);
        }
    }
}

/// Emit events for a protocol declaration
///
/// This is the single source of truth for protocol declaration emission.
pub fn emit_protocol_declaration(sink: &mut EventSink, data: ProtocolDeclarationData) {
    sink.start_node(SyntaxKind::ProtocolDeclaration);

    emit_visibility(sink, data.visibility);
    sink.add_token(SyntaxKind::Protocol, data.protocol_span);
    emit_name(sink, data.name_span);

    if let Some((lbracket, params, rbracket)) = data.type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    if let Some(inherited) = data.inherited {
        emit_conformance_list(sink, inherited.colon_span, &inherited.conformances);
    }

    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    sink.start_node(SyntaxKind::ProtocolBody);
    sink.add_token(SyntaxKind::LBrace, data.lbrace_span);

    for item in data.body {
        match item {
            ProtocolBodyItem::Function(func_data) => emit_function_declaration(sink, func_data),
            ProtocolBodyItem::AssociatedType(type_data) => emit_type_alias_declaration(sink, type_data),
            ProtocolBodyItem::Initializer(init_data) => emit_initializer_declaration(sink, init_data),
        }
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace_span);
    sink.finish_node(); // ProtocolBody

    sink.finish_node(); // ProtocolDeclaration
}

/// Emit events for an associated type target
///
/// Handles both simple names and qualified paths (e.g., `Iterator.Item`, `Add[Int].Output`).
fn emit_associated_type_target(sink: &mut EventSink, target: &AssociatedTypeTargetData) {
    match target {
        AssociatedTypeTargetData::Simple(name_span) => {
            emit_name(sink, name_span.clone());
        }
        AssociatedTypeTargetData::Qualified { protocol_path, dot_span, name_span } => {
            sink.start_node(SyntaxKind::AssociatedTypeTarget);
            emit_ty_variant(sink, protocol_path);
            sink.add_token(SyntaxKind::Dot, dot_span.clone());
            emit_name(sink, name_span.clone());
            sink.finish_node();
        }
    }
}

/// Emit events for associated type bounds (: Equatable, Hashable)
fn emit_associated_type_bounds(sink: &mut EventSink, bounds: &AssociatedTypeBoundsData) {
    sink.add_token(SyntaxKind::Colon, bounds.colon_span.clone());
    for (i, bound) in bounds.bounds.iter().enumerate() {
        if i > 0 {
            // Emit comma between bounds (approximated span)
            let prev_end = if i > 0 {
                // This is approximate - we don't track comma positions
                bounds.colon_span.end + i
            } else {
                bounds.colon_span.end
            };
            sink.add_token(SyntaxKind::Comma, prev_end..prev_end + 1);
        }
        emit_ty_variant(sink, bound);
    }
}

/// Emit events for a type alias declaration
///
/// This is the single source of truth for type alias declaration emission.
/// Handles:
/// - Regular type aliases: `type Alias = Type;`
/// - Associated types: `type Item;`, `type Item: Bound;`, `type Item = Default;`
/// - Qualified bindings: `type Iterator.Item = Int;`
pub fn emit_type_alias_declaration(sink: &mut EventSink, data: TypeAliasDeclarationData) {
    sink.start_node(SyntaxKind::TypeAliasDeclaration);

    emit_visibility(sink, data.visibility);
    sink.add_token(SyntaxKind::Type, data.type_span);

    emit_associated_type_target(sink, &data.target);

    if let Some((lbracket, params, rbracket)) = data.type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    // Emit bounds if present (for associated types)
    if let Some(ref bounds) = data.bounds {
        emit_associated_type_bounds(sink, bounds);
    }

    // Emit aliased type if present (optional for abstract associated types)
    if let Some((equals_span, ref aliased_type)) = data.aliased {
        sink.add_token(SyntaxKind::Equals, equals_span);
        sink.start_node(SyntaxKind::AliasedType);
        emit_ty_variant(sink, aliased_type);
        sink.finish_node(); // AliasedType
    }

    sink.add_token(SyntaxKind::Semicolon, data.semicolon_span);

    sink.finish_node(); // TypeAliasDeclaration
}

/// Emit events for an extension declaration
///
/// This is the single source of truth for extension declaration emission.
pub fn emit_extension_declaration(sink: &mut EventSink, data: ExtensionDeclarationData) {
    sink.start_node(SyntaxKind::ExtensionDeclaration);

    sink.add_token(SyntaxKind::Extend, data.extend_span);

    // Emit target type (e.g., Box[T, Int])
    emit_ty_variant(sink, &data.target_type);

    // Emit conformance list if present
    if let Some(conf) = data.conformances {
        emit_conformance_list(sink, conf.colon_span, &conf.conformances);
    }

    // Emit where clause if present
    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    sink.start_node(SyntaxKind::ExtensionBody);
    sink.add_token(SyntaxKind::LBrace, data.lbrace_span);

    for item in data.body {
        emit_extension_body_item(sink, item);
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace_span);
    sink.finish_node(); // ExtensionBody

    sink.finish_node(); // ExtensionDeclaration
}

/// Emit events for an extension body item
fn emit_extension_body_item(sink: &mut EventSink, item: ExtensionBodyItem) {
    match item {
        ExtensionBodyItem::Function(data) => emit_function_declaration(sink, data),
        ExtensionBodyItem::Initializer(data) => emit_initializer_declaration(sink, data),
    }
}
