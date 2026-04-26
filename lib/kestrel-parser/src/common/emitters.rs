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
    FunctionBodyData, InitializerDeclarationData, ParameterAccessMode, ParameterData,
    TypeDeclarationBodyItem,
};
use crate::block::emit_code_block;
use crate::enum_decl::{emit_enum_case, emit_enum_declaration};
use crate::event::EventSink;
use crate::expr::emit_expr_variant;
use crate::field::emit_field_declaration;
use crate::function::emit_function_declaration;
use crate::import::emit_import_declaration;
use crate::module::emit_module_declaration;
use crate::pattern::emit_pattern_variant;
use crate::r#struct::emit_struct_declaration;
use crate::subscript::emit_subscript_declaration;
use crate::ty::emit_ty_variant;
use crate::type_alias::emit_type_alias_declaration;
use crate::type_param::{emit_type_parameter_list, emit_where_clause};

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

/// Emit events for an initializer declaration.
///
/// Destructures `InitializerDeclarationData` without a `..` rest pattern:
/// adding a field forces this function to stop compiling until the new
/// field is handled in emission.
pub fn emit_initializer_declaration(sink: &mut EventSink, data: InitializerDeclarationData) {
    let InitializerDeclarationData {
        attributes,
        visibility,
        init_span,
        type_params,
        lparen,
        parameters,
        rparen,
        where_clause,
        body,
    } = data;

    sink.start_node(SyntaxKind::InitializerDeclaration);

    emit_attribute_list(sink, &attributes);
    emit_visibility(sink, visibility);
    sink.add_token(SyntaxKind::Init, init_span);

    if let Some((lbracket, params, rbracket)) = type_params {
        emit_type_parameter_list(sink, lbracket, params, rbracket);
    }

    emit_parameter_list(sink, lparen, parameters, rparen);

    if let Some(wc) = where_clause {
        emit_where_clause(sink, wc);
    }

    if let Some(block) = body {
        emit_function_body(sink, &FunctionBodyData::Block(block));
    }

    sink.finish_node();
}

impl crate::event::EmitSyntax for InitializerDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_initializer_declaration(sink, self);
    }
}

/// Emit events for a deinitializer declaration.
///
/// Destructures `DeinitDeclarationData` without a `..` rest pattern: adding
/// a field forces this function to stop compiling until the new field is
/// handled in emission.
pub fn emit_deinit_declaration(sink: &mut EventSink, data: DeinitDeclarationData) {
    let DeinitDeclarationData { deinit_span, body } = data;
    sink.start_node(SyntaxKind::DeinitDeclaration);
    sink.add_token(SyntaxKind::Deinit, deinit_span);
    emit_function_body(sink, &FunctionBodyData::Block(body));
    sink.finish_node();
}

impl crate::event::EmitSyntax for DeinitDeclarationData {
    fn emit(self, sink: &mut EventSink) {
        emit_deinit_declaration(sink, self);
    }
}

/// Emit events for a type declaration body item (struct or enum body item)
pub(crate) fn emit_type_declaration_body_item(
    sink: &mut EventSink,
    item: TypeDeclarationBodyItem,
) {
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


