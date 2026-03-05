//! Analyzer for protocol field conformance validation
//!
//! Validates that when a struct/enum conforms to a protocol with `requires_fields_conform`,
//! all fields/case payloads also conform to that protocol.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::builtins::BuiltinKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use kestrel_semantic_type_inference::TypeOracle;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::{FieldsNotConformingToProtocolError, NonConformingField};

/// Analyzer that validates protocol field conformance for structs and enums.
pub struct ProtocolFieldConformanceAnalyzer;

impl ProtocolFieldConformanceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ProtocolFieldConformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ProtocolFieldConformanceAnalyzer {
    fn name(&self) -> &'static str {
        "protocol_field_conformance"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();
        if kind != KestrelSymbolKind::Struct && kind != KestrelSymbolKind::Enum {
            return;
        }

        let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>() else {
            return;
        };

        let type_kind = if kind == KestrelSymbolKind::Struct {
            "struct"
        } else {
            "enum"
        };

        for conformance_ty in conformances.conformances() {
            let TyKind::Protocol {
                symbol: protocol_sym,
                ..
            } = conformance_ty.kind()
            else {
                continue;
            };

            let protocol_id = protocol_sym.metadata().id();

            let Some(feature) = ctx.model.builtin_registry().protocol_feature(protocol_id) else {
                continue;
            };

            let definition = feature.definition();
            let BuiltinKind::Protocol {
                requires_fields_conform: true,
                ..
            } = definition.kind
            else {
                continue;
            };

            let mut non_conforming: Vec<NonConformingField> = Vec::new();

            if kind == KestrelSymbolKind::Struct {
                // Check struct fields
                for field in symbol
                    .metadata()
                    .children()
                    .iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
                {
                    if let Some(typed) = field.metadata().get_behavior::<TypedBehavior>() {
                        let field_ty = typed.ty();
                        if !ctx.model.conforms_to(field_ty, protocol_id) {
                            non_conforming.push(NonConformingField {
                                field_name: field.metadata().name().value.clone(),
                                field_ty: field_ty.to_string(),
                                span: field.metadata().span().clone(),
                            });
                        }
                    }
                }
            } else {
                // Check enum case payloads
                for case in symbol
                    .metadata()
                    .children()
                    .iter()
                    .filter(|c| c.metadata().kind() == KestrelSymbolKind::EnumCase)
                {
                    if let Some(callable) = case.metadata().get_behavior::<CallableBehavior>() {
                        for param in callable.parameters() {
                            if !ctx.model.conforms_to(&param.ty, protocol_id) {
                                non_conforming.push(NonConformingField {
                                    field_name: format!(
                                        "{}.{}",
                                        case.metadata().name().value,
                                        param.bind_name.value
                                    ),
                                    field_ty: param.ty.to_string(),
                                    span: case.metadata().span().clone(),
                                });
                            }
                        }
                    }
                }
            }

            if !non_conforming.is_empty() {
                ctx.report(FieldsNotConformingToProtocolError {
                    type_span: symbol.metadata().span().clone(),
                    type_name: symbol.metadata().name().value.clone(),
                    type_kind,
                    protocol_name: protocol_sym.metadata().name().value.clone(),
                    non_conforming_fields: non_conforming,
                });
            }
        }
    }
}
