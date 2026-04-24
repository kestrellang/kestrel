//! Common event emitters shared across multiple parsers
//!
//! This module provides reusable event emission functions that build syntax
//! trees by emitting events through an EventSink. These functions are used
//! by multiple parser modules to avoid code duplication.

use kestrel_lexer::Token;
use kestrel_span::Span;
use kestrel_syntax_tree::SyntaxKind;

use super::data::{
    AttributeArgData, AttributeArgValue, AttributeArgsData, AttributeData, DeinitDeclarationData,
    EnumCaseDeclarationData, EnumDeclarationData, ExtensionBodyItem, ExtensionDeclarationData,
    FunctionBodyData, InitializerDeclarationData, ParameterAccessMode, ParameterData,
    ProtocolBodyItem, ProtocolDeclarationData, StructDeclarationData, TypeDeclarationBodyItem,
};
use crate::block::emit_code_block;
use crate::event::EventSink;
use crate::expr::emit_expr_variant;
use crate::field::emit_field_declaration;
use crate::function::emit_function_declaration;
use crate::import::emit_import_declaration;
use crate::module::emit_module_declaration;
use crate::pattern::emit_pattern_variant;
use crate::subscript::emit_subscript_declaration;
use crate::ty::emit_ty_variant;
use crate::type_alias::emit_type_alias_declaration;
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
            sink.add_token(
                SyntaxKind::Dot,
                Span::new(span.file_id, span.start - 1..span.start),
            );
        }
        sink.add_token(SyntaxKind::Identifier, span.clone());
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
        },
        AttributeArgValue::Integer(span) => {
            sink.add_token(SyntaxKind::Integer, span.clone());
        },
        AttributeArgValue::Float(span) => {
            sink.add_token(SyntaxKind::Float, span.clone());
        },
        AttributeArgValue::Bool(span) => {
            sink.add_token(SyntaxKind::Boolean, span.clone());
        },
        AttributeArgValue::ImplicitMember {
            dot_span,
            name_span,
        } => {
            sink.add_token(SyntaxKind::Dot, dot_span.clone());
            sink.add_token(SyntaxKind::Identifier, name_span.clone());
        },
        AttributeArgValue::Path(segments) => {
            for (i, span) in segments.iter().enumerate() {
                if i > 0 {
                    // Emit dot between segments (approximate span)
                    let prev_end = segments[i - 1].end;
                    sink.add_token(
                        SyntaxKind::Dot,
                        Span::new(segments[i - 1].file_id, prev_end..prev_end + 1),
                    );
                }
                sink.add_token(SyntaxKind::Identifier, span.clone());
            }
        },
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

    // Emit label if present (external parameter name)
    if let Some(label_span) = param.label {
        emit_name(sink, label_span);
    }

    // Emit the parameter pattern (identifier, tuple, struct, or wildcard)
    emit_pattern_variant(sink, &param.pattern);

    sink.add_token(SyntaxKind::Colon, param.colon);
    emit_ty_variant(sink, &param.ty);

    // Emit default value if present
    if let Some((equals_span, ref default_expr)) = param.default {
        sink.start_node(SyntaxKind::DefaultValue);
        sink.add_token(SyntaxKind::Equals, equals_span);
        emit_expr_variant(sink, default_expr);
        sink.finish_node();
    }

    sink.finish_node();
}

/// Emit events for a return type
pub fn emit_return_type(sink: &mut EventSink, arrow_span: Span, return_ty: crate::ty::TyVariant) {
    sink.start_node(SyntaxKind::ReturnType);
    sink.add_token(SyntaxKind::Arrow, arrow_span);
    emit_ty_variant(sink, &return_ty);
    sink.finish_node();
}

/// Emit events for a function body (block or expression)
pub fn emit_function_body(sink: &mut EventSink, body: &FunctionBodyData) {
    sink.start_node(SyntaxKind::FunctionBody);
    match body {
        FunctionBodyData::Block(block) => {
            emit_code_block(sink, block);
        },
        FunctionBodyData::Expression(eq_span, expr) => {
            sink.add_token(SyntaxKind::Equals, eq_span.clone());
            sink.start_node(SyntaxKind::Expression);
            emit_expr_variant(sink, expr);
            sink.finish_node();
        },
    }
    sink.finish_node();
}

// =============================================================================
// Declaration Emitters - Single Source of Truth
// =============================================================================

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
        emit_function_body(sink, &FunctionBodyData::Block(block.clone()));
    }

    sink.finish_node();
}

/// Emit events for a deinitializer declaration
///
/// This is the single source of truth for deinit declaration emission.
pub fn emit_deinit_declaration(sink: &mut EventSink, data: DeinitDeclarationData) {
    sink.start_node(SyntaxKind::DeinitDeclaration);
    sink.add_token(SyntaxKind::Deinit, data.deinit_span);
    emit_function_body(sink, &FunctionBodyData::Block(data.body));
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
        TypeDeclarationBodyItem::Subscript(data) => emit_subscript_declaration(sink, data),
        TypeDeclarationBodyItem::Initializer(data) => emit_initializer_declaration(sink, data),
        TypeDeclarationBodyItem::Deinit(data) => emit_deinit_declaration(sink, data),
        TypeDeclarationBodyItem::Struct(data) => emit_struct_declaration(sink, *data),
        TypeDeclarationBodyItem::Enum(data) => emit_enum_declaration(sink, *data),
        TypeDeclarationBodyItem::EnumCase(data) => emit_enum_case(sink, data),
        TypeDeclarationBodyItem::TypeAlias(data) => emit_type_alias_declaration(sink, data),
        TypeDeclarationBodyItem::Module(module_span, path_segments) => {
            emit_module_declaration(sink, module_span, &path_segments);
        },
        TypeDeclarationBodyItem::Import(import_span, path_segments, alias, items) => {
            emit_import_declaration(sink, import_span, &path_segments, alias, items);
        },
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
            ProtocolBodyItem::Subscript(subscript_data) => {
                emit_subscript_declaration(sink, subscript_data)
            },
            ProtocolBodyItem::AssociatedType(type_data) => {
                emit_type_alias_declaration(sink, type_data)
            },
            ProtocolBodyItem::Initializer(init_data) => {
                emit_initializer_declaration(sink, init_data)
            },
            ProtocolBodyItem::Field(field_data) => emit_field_declaration(sink, field_data),
        }
    }

    sink.add_token(SyntaxKind::RBrace, data.rbrace_span);
    sink.finish_node(); // ProtocolBody

    sink.finish_node(); // ProtocolDeclaration
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
        ExtensionBodyItem::Subscript(data) => emit_subscript_declaration(sink, data),
        ExtensionBodyItem::Initializer(data) => emit_initializer_declaration(sink, data),
        ExtensionBodyItem::TypeAlias(data) => emit_type_alias_declaration(sink, data),
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

