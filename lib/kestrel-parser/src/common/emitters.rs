//! Common event emitters shared across multiple parsers
//!
//! This module provides reusable event emission functions that build syntax
//! trees by emitting events through an EventSink. These functions are used
//! by multiple parser modules to avoid code duplication.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use super::data::{
    AssociatedTypeBoundsData, AssociatedTypeTargetData, AttributeArgData, AttributeArgValue,
    AttributeArgsData, AttributeData, ComputedBodyData, DeinitDeclarationData,
    EnumCaseDeclarationData, EnumDeclarationData, ExtensionBodyItem, ExtensionDeclarationData,
    FieldDeclarationData, FunctionDeclarationData, InitializerDeclarationData,
    ParameterAccessMode, ParameterData, ProtocolBodyItem, ProtocolDeclarationData,
    ReceiverModifier, StructDeclarationData, TypeAliasDeclarationData, TypeDeclarationBodyItem,
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
            sink.add_token(SyntaxKind::Dot, Span::from(span.start - 1..span.start));
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
        sink.add_token(
            SyntaxKind::Dot,
            Span::from(last_segment_end..last_segment_end + 1),
        );
        sink.add_token(
            SyntaxKind::LParen,
            Span::from(last_segment_end + 1..last_segment_end + 2),
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
                sink.add_token(SyntaxKind::Comma, Span::from(prev_end..prev_end + 1));
            }

            sink.start_node(SyntaxKind::ImportItem);
            sink.add_token(SyntaxKind::Identifier, name_span.clone());

            if let Some(alias_s) = alias_span {
                let as_start = name_span.end + 1;
                sink.add_token(SyntaxKind::As, Span::from(as_start..as_start + 2));
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
        sink.add_token(
            SyntaxKind::RParen,
            Span::from(last_item_end..last_item_end + 1),
        );
    } else if let Some(alias_span) = alias {
        let as_start = path_segments.last().unwrap().end + 1;
        sink.add_token(SyntaxKind::As, Span::from(as_start..as_start + 2));
        sink.add_token(SyntaxKind::Identifier, alias_span);
    }

    sink.finish_node();
}

// =============================================================================
// Attribute Emitters
// =============================================================================

/// Emit events for a single attribute argument value
fn emit_attribute_arg_value(sink: &mut EventSink, value: &AttributeArgValue) {
    match value {
        AttributeArgValue::String(span) => {
            sink.add_token(SyntaxKind::String, span.clone());
        }
        AttributeArgValue::Integer(span) => {
            sink.add_token(SyntaxKind::Integer, span.clone());
        }
        AttributeArgValue::Float(span) => {
            sink.add_token(SyntaxKind::Float, span.clone());
        }
        AttributeArgValue::Bool(span) => {
            sink.add_token(SyntaxKind::Boolean, span.clone());
        }
        AttributeArgValue::ImplicitMember {
            dot_span,
            name_span,
        } => {
            sink.add_token(SyntaxKind::Dot, dot_span.clone());
            sink.add_token(SyntaxKind::Identifier, name_span.clone());
        }
        AttributeArgValue::Path(segments) => {
            for (i, span) in segments.iter().enumerate() {
                if i > 0 {
                    // Emit dot between segments (approximate span)
                    let prev_end = segments[i - 1].end;
                    sink.add_token(SyntaxKind::Dot, Span::from(prev_end..prev_end + 1));
                }
                sink.add_token(SyntaxKind::Identifier, span.clone());
            }
        }
    }
}

/// Emit events for a single attribute argument
fn emit_attribute_arg(sink: &mut EventSink, arg: &AttributeArgData) {
    sink.start_node(SyntaxKind::AttributeArg);

    if let Some(label_span) = &arg.label {
        sink.add_token(SyntaxKind::Identifier, label_span.clone());
        if let Some(colon_span) = &arg.colon {
            sink.add_token(SyntaxKind::Colon, colon_span.clone());
        }
    }

    emit_attribute_arg_value(sink, &arg.value);

    sink.finish_node();
}

/// Emit events for attribute arguments (the parenthesized list)
fn emit_attribute_args(sink: &mut EventSink, args: &AttributeArgsData) {
    sink.start_node(SyntaxKind::AttributeArgs);
    sink.add_token(SyntaxKind::LParen, args.lparen_span.clone());

    for arg in &args.args {
        emit_attribute_arg(sink, arg);
    }

    sink.add_token(SyntaxKind::RParen, args.rparen_span.clone());
    sink.finish_node();
}

/// Emit events for a single attribute
fn emit_attribute(sink: &mut EventSink, attr: &AttributeData) {
    sink.start_node(SyntaxKind::Attribute);
    sink.add_token(SyntaxKind::At, attr.at_span.clone());
    sink.add_token(SyntaxKind::Identifier, attr.name_span.clone());

    if let Some(args) = &attr.args {
        emit_attribute_args(sink, args);
    }

    sink.finish_node();
}

/// Emit events for an attribute list (zero or more attributes)
pub fn emit_attribute_list(sink: &mut EventSink, attributes: &[AttributeData]) {
    if !attributes.is_empty() {
        sink.start_node(SyntaxKind::AttributeList);
        for attr in attributes {
            emit_attribute(sink, attr);
        }
        sink.finish_node();
    }
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

    // Emit access mode if present (mutating/consuming)
    if let Some((mode, span)) = param.access_mode {
        let kind = match mode {
            ParameterAccessMode::Borrow => unreachable!("Borrow is default, not emitted"),
            ParameterAccessMode::Mutating => SyntaxKind::Mutating,
            ParameterAccessMode::Consuming => SyntaxKind::Consuming,
        };
        sink.add_token(kind, span);
    }

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

    emit_attribute_list(sink, &data.attributes);
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

    // Emit optional trailing semicolon
    if let Some(semicolon_span) = data.semicolon {
        sink.add_token(SyntaxKind::Semicolon, semicolon_span);
    }

    sink.finish_node();
}

/// Emit events for property accessors (computed property body)
fn emit_property_accessors(sink: &mut EventSink, computed_body: &ComputedBodyData) {
    sink.start_node(SyntaxKind::PropertyAccessors);

    match computed_body {
        ComputedBodyData::Shorthand(body) => {
            // Shorthand: just emit the code block directly
            emit_code_block(sink, body);
        }
        ComputedBodyData::Accessors { getter, setter } => {
            // Emit getter
            if let Some(getter_body) = getter {
                // Full getter with body: emit GetterClause containing Get token and code block
                sink.start_node(SyntaxKind::GetterClause);
                // Emit Get token - use the start of the code block as approximate span
                let get_span =
                    Span::from(getter_body.lbrace.start.saturating_sub(4)..getter_body.lbrace.start);
                sink.add_token(SyntaxKind::Get, get_span);
                emit_code_block(sink, getter_body);
                sink.finish_node();
            } else {
                // Protocol requirement: just emit Get token without body (no GetterClause wrapper)
                sink.add_token(SyntaxKind::Get, Span::from(0..3));
            }

            // Emit setter
            if let Some(setter_body) = setter {
                // Check if this is a real setter body or a placeholder for protocol requirement
                if setter_body.lbrace.start == 0 && setter_body.lbrace.end == 0 {
                    // Protocol requirement: just emit Set token without body (no SetterClause wrapper)
                    sink.add_token(SyntaxKind::Set, Span::from(0..3));
                } else {
                    // Full setter with body: emit SetterClause containing Set token and code block
                    sink.start_node(SyntaxKind::SetterClause);
                    let set_span = Span::from(
                        setter_body.lbrace.start.saturating_sub(4)..setter_body.lbrace.start,
                    );
                    sink.add_token(SyntaxKind::Set, set_span);
                    emit_code_block(sink, setter_body);
                    sink.finish_node();
                }
            }
        }
    }

    sink.finish_node();
}

/// Emit events for an initializer declaration
///
/// This is the single source of truth for initializer declaration emission.
pub fn emit_initializer_declaration(sink: &mut EventSink, data: InitializerDeclarationData) {
    sink.start_node(SyntaxKind::InitializerDeclaration);

    emit_attribute_list(sink, &data.attributes);
    emit_visibility(sink, data.visibility);
    sink.add_token(SyntaxKind::Init, data.init_span);

    if let Some((lbracket, params, rbracket)) = data.type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    emit_parameter_list(sink, data.lparen, data.parameters, data.rparen);

    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    if let Some(ref block) = data.body {
        emit_function_body(sink, block);
    }

    sink.finish_node();
}

/// Emit events for a deinitializer declaration
///
/// This is the single source of truth for deinit declaration emission.
pub fn emit_deinit_declaration(sink: &mut EventSink, data: DeinitDeclarationData) {
    sink.start_node(SyntaxKind::DeinitDeclaration);
    sink.add_token(SyntaxKind::Deinit, data.deinit_span);
    emit_function_body(sink, &data.body);
    sink.finish_node();
}

/// Emit events for a struct declaration
///
/// This is the single source of truth for struct declaration emission.
pub fn emit_struct_declaration(sink: &mut EventSink, data: StructDeclarationData) {
    sink.start_node(SyntaxKind::StructDeclaration);

    emit_attribute_list(sink, &data.attributes);
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
        emit_type_declaration_body_item(sink, item);
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace_span);
    sink.finish_node(); // StructBody

    sink.finish_node(); // StructDeclaration
}

/// Emit events for a type declaration body item (struct or enum body item)
fn emit_type_declaration_body_item(sink: &mut EventSink, item: TypeDeclarationBodyItem) {
    match item {
        TypeDeclarationBodyItem::Field(data) => emit_field_declaration(sink, data),
        TypeDeclarationBodyItem::Function(data) => emit_function_declaration(sink, data),
        TypeDeclarationBodyItem::Initializer(data) => emit_initializer_declaration(sink, data),
        TypeDeclarationBodyItem::Deinit(data) => emit_deinit_declaration(sink, data),
        TypeDeclarationBodyItem::Struct(data) => emit_struct_declaration(sink, *data),
        TypeDeclarationBodyItem::Enum(data) => emit_enum_declaration(sink, *data),
        TypeDeclarationBodyItem::EnumCase(data) => emit_enum_case(sink, data),
        TypeDeclarationBodyItem::TypeAlias(data) => emit_type_alias_declaration(sink, data),
        TypeDeclarationBodyItem::Module(module_span, path_segments) => {
            emit_module_declaration(sink, module_span, &path_segments);
        }
        TypeDeclarationBodyItem::Import(import_span, path_segments, alias, items) => {
            emit_import_declaration(sink, import_span, &path_segments, alias, items);
        }
    }
}

/// Emit events for a protocol declaration
///
/// This is the single source of truth for protocol declaration emission.
pub fn emit_protocol_declaration(sink: &mut EventSink, data: ProtocolDeclarationData) {
    sink.start_node(SyntaxKind::ProtocolDeclaration);

    emit_attribute_list(sink, &data.attributes);
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
            ProtocolBodyItem::AssociatedType(type_data) => {
                emit_type_alias_declaration(sink, type_data)
            }
            ProtocolBodyItem::Initializer(init_data) => {
                emit_initializer_declaration(sink, init_data)
            }
            ProtocolBodyItem::Field(field_data) => emit_field_declaration(sink, field_data),
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
        AssociatedTypeTargetData::Qualified {
            protocol_path,
            dot_span,
            name_span,
        } => {
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
            sink.add_token(SyntaxKind::Comma, Span::from(prev_end..prev_end + 1));
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

    emit_attribute_list(sink, &data.attributes);
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

    // Emit where clause if present (for associated types with constraints)
    if let Some(wc) = data.where_clause {
        emit_where_clause(sink, wc);
    }

    // Emit aliased type if present (optional for abstract associated types)
    if let Some((equals_span, ref aliased_type)) = data.aliased {
        sink.add_token(SyntaxKind::Equals, equals_span);
        sink.start_node(SyntaxKind::AliasedType);
        emit_ty_variant(sink, aliased_type);
        sink.finish_node(); // AliasedType
    }

    if let Some(semicolon_span) = data.semicolon_span {
        sink.add_token(SyntaxKind::Semicolon, semicolon_span);
    }

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

// =============================================================================
// Enum Emitters
// =============================================================================

/// Emit events for an indirect modifier
pub fn emit_indirect_modifier(sink: &mut EventSink, indirect_span: Span) {
    sink.start_node(SyntaxKind::IndirectModifier);
    sink.add_token(SyntaxKind::Indirect, indirect_span);
    sink.finish_node();
}

/// Emit events for an enum case parameter
///
/// Supports both named (`label: Type`) and unnamed (`Type`) forms.
pub fn emit_enum_case_parameter(sink: &mut EventSink, data: &super::data::EnumCaseParameterData) {
    sink.start_node(SyntaxKind::EnumCaseParameter);
    // Emit label and colon if present (named parameter)
    if let (Some(label), Some(colon)) = (&data.label, &data.colon) {
        emit_name(sink, label.clone());
        sink.add_token(SyntaxKind::Colon, colon.clone());
    }
    // Always emit the type
    emit_ty_variant(sink, &data.ty);
    sink.finish_node();
}

/// Emit events for an enum case parameter list
pub fn emit_enum_case_parameter_list(
    sink: &mut EventSink,
    lparen: Span,
    parameters: &[super::data::EnumCaseParameterData],
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

/// Emit events for an enum case declaration
///
/// This is the single source of truth for enum case declaration emission.
pub fn emit_enum_case(sink: &mut EventSink, data: EnumCaseDeclarationData) {
    sink.start_node(SyntaxKind::EnumCaseDeclaration);
    emit_attribute_list(sink, &data.attributes);
    sink.add_token(SyntaxKind::Case, data.case_span);
    emit_name(sink, data.name_span);

    if let Some((lparen, ref params, rparen)) = data.parameters {
        emit_enum_case_parameter_list(sink, lparen, params, rparen);
    }

    sink.finish_node();
}

/// Emit events for an enum declaration
///
/// This is the single source of truth for enum declaration emission.
pub fn emit_enum_declaration(sink: &mut EventSink, data: EnumDeclarationData) {
    sink.start_node(SyntaxKind::EnumDeclaration);

    emit_attribute_list(sink, &data.attributes);
    emit_visibility(sink, data.visibility);

    if let Some(indirect_span) = data.indirect {
        emit_indirect_modifier(sink, indirect_span);
    }

    sink.add_token(SyntaxKind::Enum, data.enum_span);
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

    sink.start_node(SyntaxKind::EnumBody);
    sink.add_token(SyntaxKind::LBrace, data.lbrace_span);

    for item in data.body {
        emit_type_declaration_body_item(sink, item);
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace_span);
    sink.finish_node(); // EnumBody

    sink.finish_node(); // EnumDeclaration
}
