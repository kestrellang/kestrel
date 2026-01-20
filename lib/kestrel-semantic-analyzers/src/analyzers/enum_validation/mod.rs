//! Analyzers for validating enum declarations.
//!
//! This module contains:
//! - `DuplicateCaseAnalyzer`: Detects duplicate case names within an enum
//! - `DuplicateLabelAnalyzer`: Detects duplicate parameter labels within a case
//! - `RecursiveEnumAnalyzer`: Detects recursive enums without `indirect` keyword

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_model::queries::StructFieldTypes;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use kestrel_span::Span;
use semantic_tree::symbol::{Symbol, SymbolId};

mod diagnostics;
pub use diagnostics::*;

// ============================================================================
// DuplicateCaseAnalyzer
// ============================================================================

/// Analyzer that detects duplicate case names within an enum.
///
/// Reports an error when the same case name is used multiple times:
/// ```ignore
/// enum Color {
///     case Red
///     case Red  // Error: duplicate case 'Red'
/// }
/// ```
pub struct DuplicateCaseAnalyzer {
    enums: Mutex<Vec<Arc<EnumSymbol>>>,
}

impl DuplicateCaseAnalyzer {
    pub fn new() -> Self {
        Self {
            enums: Mutex::new(Vec::new()),
        }
    }
}

impl Default for DuplicateCaseAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DuplicateCaseAnalyzer {
    fn name(&self) -> &'static str {
        "duplicate_enum_case"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() == KestrelSymbolKind::Enum {
            if let Ok(enum_sym) = symbol.clone().downcast_arc::<EnumSymbol>() {
                self.enums.lock().unwrap().push(enum_sym);
            }
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        for enum_sym in self.enums.lock().unwrap().iter() {
            check_duplicate_cases(enum_sym.as_ref(), ctx);
        }
    }
}

fn check_duplicate_cases(enum_sym: &EnumSymbol, ctx: &mut AnalysisContext) {
    let cases = enum_sym.cases();
    let mut seen: HashMap<String, Span> = HashMap::new();

    for case in &cases {
        let name = case.metadata().name().value.clone();
        let span = case.metadata().span().clone();

        if let Some(first_span) = seen.get(&name) {
            ctx.report(DuplicateCaseError {
                case_name: name.clone(),
                first_span: first_span.clone(),
                duplicate_span: span,
            });
        } else {
            seen.insert(name, span);
        }
    }
}

// ============================================================================
// DuplicateLabelAnalyzer
// ============================================================================

/// Analyzer that detects duplicate parameter labels within an enum case.
///
/// Reports an error when the same label is used multiple times in a case's parameters:
/// ```ignore
/// enum Bad {
///     case Foo(x: Int, x: String)  // Error: duplicate label 'x'
/// }
/// ```
pub struct DuplicateLabelAnalyzer {
    enums: Mutex<Vec<Arc<EnumSymbol>>>,
}

impl DuplicateLabelAnalyzer {
    pub fn new() -> Self {
        Self {
            enums: Mutex::new(Vec::new()),
        }
    }
}

impl Default for DuplicateLabelAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DuplicateLabelAnalyzer {
    fn name(&self) -> &'static str {
        "duplicate_enum_label"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() == KestrelSymbolKind::Enum {
            if let Ok(enum_sym) = symbol.clone().downcast_arc::<EnumSymbol>() {
                self.enums.lock().unwrap().push(enum_sym);
            }
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        for enum_sym in self.enums.lock().unwrap().iter() {
            check_duplicate_labels(enum_sym.as_ref(), ctx);
        }
    }
}

fn check_duplicate_labels(enum_sym: &EnumSymbol, ctx: &mut AnalysisContext) {
    for case in enum_sym.cases() {
        if let Some(callable) = case.callable_behavior() {
            let params = callable.parameters();
            let mut seen: HashMap<String, Span> = HashMap::new();

            for param in params {
                if let Some(label) = &param.label {
                    let label_name = label.value.clone();
                    if let Some(first_span) = seen.get(&label_name) {
                        ctx.report(DuplicateLabelError {
                            label_name: label_name.clone(),
                            case_name: case.metadata().name().value.clone(),
                            first_span: first_span.clone(),
                            duplicate_span: label.span.clone(),
                        });
                    } else {
                        seen.insert(label_name, label.span.clone());
                    }
                }
            }
        }
    }
}

// ============================================================================
// RecursiveEnumAnalyzer
// ============================================================================

/// Analyzer that detects recursive enums without the `indirect` keyword.
///
/// Recursive enums (enums that reference themselves in case parameters) require
/// the `indirect` keyword to compile, as they need boxing to have a known size.
///
/// Reports an error for:
/// ```ignore
/// enum Tree {
///     case Leaf(value: Int)
///     case Node(left: Tree, right: Tree)  // Error: recursive without indirect
/// }
/// ```
///
/// But allows:
/// ```ignore
/// indirect enum Tree {
///     case Leaf(value: Int)
///     case Node(left: Tree, right: Tree)  // OK: indirect makes it boxed
/// }
/// ```
pub struct RecursiveEnumAnalyzer {
    enums: Mutex<Vec<Arc<EnumSymbol>>>,
}

impl RecursiveEnumAnalyzer {
    pub fn new() -> Self {
        Self {
            enums: Mutex::new(Vec::new()),
        }
    }
}

impl Default for RecursiveEnumAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for RecursiveEnumAnalyzer {
    fn name(&self) -> &'static str {
        "recursive_enum"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        _ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() == KestrelSymbolKind::Enum {
            if let Ok(enum_sym) = symbol.clone().downcast_arc::<EnumSymbol>() {
                self.enums.lock().unwrap().push(enum_sym);
            }
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        for enum_sym in self.enums.lock().unwrap().iter() {
            // Skip if already marked indirect
            if enum_sym.is_indirect() {
                continue;
            }

            check_for_recursion(enum_sym.as_ref(), ctx.model, ctx);
        }
    }
}

fn check_for_recursion(enum_sym: &EnumSymbol, model: &SemanticModel, ctx: &mut AnalysisContext) {
    let enum_id = enum_sym.metadata().id();

    for case in enum_sym.cases() {
        if let Some(callable) = case.callable_behavior() {
            for param in callable.parameters() {
                let mut visited = HashSet::new();
                if type_contains_enum(&param.ty, enum_id, model, &mut visited) {
                    ctx.report(RecursiveEnumError {
                        enum_name: enum_sym.metadata().name().value.clone(),
                        enum_span: enum_sym.metadata().span().clone(),
                        case_name: case.metadata().name().value.clone(),
                        param_span: param.ty.span().clone(),
                    });
                    return; // One error per enum is enough
                }
            }
        }
    }
}

/// Check if a type contains a reference to the given enum.
///
/// This checks direct references in the type itself, tuples, and structs.
/// Arrays provide heap indirection so they don't count as containing the enum.
fn type_contains_enum(
    ty: &Ty,
    enum_id: SymbolId,
    model: &SemanticModel,
    visited: &mut HashSet<SymbolId>,
) -> bool {
    match ty.kind() {
        TyKind::Enum { symbol, .. } => symbol.metadata().id() == enum_id,
        TyKind::Tuple(elements) => elements
            .iter()
            .any(|e| type_contains_enum(e, enum_id, model, visited)),
        TyKind::Array(_) => false, // Arrays provide heap indirection
        TyKind::Struct { symbol, .. } => {
            let struct_id = symbol.metadata().id();
            // Avoid infinite loops on struct cycles
            if visited.contains(&struct_id) {
                return false;
            }
            visited.insert(struct_id);
            // Check each field of the struct
            for field in model.query(StructFieldTypes { struct_id }) {
                if type_contains_enum(&field.ty, enum_id, model, visited) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}
