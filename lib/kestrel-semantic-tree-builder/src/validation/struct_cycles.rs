//! Validator for detecting circular struct containment
//!
//! This validator detects cycles in struct field types that would create infinite-size types:
//! - Direct self-references: `struct Node { let next: Node }`
//! - Two-way cycles: `struct A { let b: B } struct B { let a: A }`
//! - Longer chains: `struct A { let b: B } struct B { let c: C } struct C { let a: A }`
//!
//! The algorithm collects all structs during the walk, then runs cycle detection
//! in the finalize phase using DFS.

use std::sync::{Arc, Mutex};

use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::cycle::CycleDetector;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::database::{Db, SemanticDatabase};
use crate::diagnostics::{CircularStructContainmentError, CycleMember, SelfContainingStructError};
use crate::syntax::get_file_id_for_symbol;
use crate::validation::{SymbolContext, Validator};

/// Validator that detects circular struct containment
pub struct StructCycleValidator {
    /// Collected structs during the walk
    structs: Mutex<Vec<CollectedStruct>>,
}

/// Data collected for each struct during the walk
struct CollectedStruct {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    struct_sym: Arc<StructSymbol>,
}

impl StructCycleValidator {
    const NAME: &'static str = "struct_cycles";

    pub fn new() -> Self {
        Self {
            structs: Mutex::new(Vec::new()),
        }
    }
}

impl Default for StructCycleValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for StructCycleValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Collect structs for later analysis
        if kind == KestrelSymbolKind::Struct {
            if let Some(struct_sym) = ctx.symbol.clone().into_any_arc().downcast::<StructSymbol>().ok() {
                self.structs.lock().unwrap().push(CollectedStruct {
                    symbol: ctx.symbol.clone(),
                    struct_sym,
                });
            }
        }
    }

    fn finalize(&self, db: &SemanticDatabase, diagnostics: &mut DiagnosticContext) {
        // Check each collected struct for cycles
        for collected in self.structs.lock().unwrap().iter() {
            check_struct_for_cycles(&collected.struct_sym, &collected.symbol, db, diagnostics);
        }
    }
}

/// Check if a specific struct participates in a containment cycle
fn check_struct_for_cycles(
    struct_sym: &StructSymbol,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    db: &SemanticDatabase,
    diagnostics: &mut DiagnosticContext,
) {
    let file_id = get_file_id_for_symbol(symbol, diagnostics);
    let struct_id = struct_sym.metadata().id();
    let struct_name = struct_sym.metadata().name().value.clone();

    // Get all fields and check their types
    for field in symbol.metadata().children() {
        if field.metadata().kind() != KestrelSymbolKind::Field {
            continue;
        }

        // Get the field's type from TypedBehavior
        let field_ty = field.metadata().behaviors().iter().find_map(|b| {
            if matches!(b.kind(), KestrelBehaviorKind::Typed) {
                b.as_ref().downcast_ref::<TypedBehavior>().map(|tb| tb.ty().clone())
            } else {
                None
            }
        });

        let Some(field_ty) = field_ty else {
            continue;
        };

        let field_name = field.metadata().name().value.clone();
        let field_span = field.metadata().span().clone();

        // Check if this field's type creates a cycle
        let mut detector: CycleDetector<SymbolId> = CycleDetector::new();

        // Enter the current struct
        if let Err(_) = detector.enter(struct_id) {
            // Shouldn't happen on first entry
            continue;
        }

        if let Some(cycle) = check_type_for_struct_cycle(&field_ty, &mut detector, db) {
            // Check if it's a self-cycle (direct self-reference)
            if cycle.is_self_cycle() {
                diagnostics.throw(
                    SelfContainingStructError {
                        struct_name: struct_name.clone(),
                        struct_span: struct_sym.metadata().declaration_span().clone(),
                        field_name,
                        field_span,
                    });
            } else {
                // Multi-struct cycle
                let origin = CycleMember {
                    name: struct_name.clone(),
                    span: struct_sym.metadata().declaration_span().clone(),
                };

                let cycle_members: Vec<CycleMember> = cycle
                    .cycle()
                    .iter()
                    .skip(1) // Skip the origin
                    .filter_map(|&id| {
                        db.symbol_by_id(id).map(|s| CycleMember {
                            name: s.metadata().name().value.clone(),
                            span: s.metadata().declaration_span().clone(),
                        })
                    })
                    .collect();

                diagnostics.throw(
                    CircularStructContainmentError {
                        origin,
                        cycle: cycle_members,
                        field_name,
                        field_span,
                    });
            }
        }

        detector.exit();
    }
}

/// Recursively check a type for struct cycles
///
/// Returns Some(cycle) if a cycle is detected, None otherwise.
fn check_type_for_struct_cycle(
    ty: &Ty,
    detector: &mut CycleDetector<SymbolId>,
    db: &SemanticDatabase,
) -> Option<semantic_tree::cycle::Cycle<SymbolId>> {
    match ty.kind() {
        TyKind::Struct { symbol, .. } => {
            let struct_id = symbol.metadata().id();

            // Try to enter - if it fails, we found a cycle
            if let Err(cycle) = detector.enter(struct_id) {
                return Some(cycle);
            }

            // Check all fields of this struct for cycles
            let struct_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for field in struct_dyn.metadata().children() {
                if field.metadata().kind() != KestrelSymbolKind::Field {
                    continue;
                }

                // Get the field's type
                let field_ty = field.metadata().behaviors().iter().find_map(|b| {
                    if matches!(b.kind(), KestrelBehaviorKind::Typed) {
                        b.as_ref().downcast_ref::<TypedBehavior>().map(|tb| tb.ty().clone())
                    } else {
                        None
                    }
                });

                if let Some(field_ty) = field_ty {
                    if let Some(cycle) = check_type_for_struct_cycle(&field_ty, detector, db) {
                        detector.exit();
                        return Some(cycle);
                    }
                }
            }

            detector.exit();
            None
        }
        TyKind::Tuple(elements) => {
            // Check each element of the tuple
            for elem in elements {
                if let Some(cycle) = check_type_for_struct_cycle(elem, detector, db) {
                    return Some(cycle);
                }
            }
            None
        }
        // Arrays, optionals, and other indirection types break the cycle
        // (they can hold a reference/pointer rather than embedding the value)
        TyKind::Array(_) => None,
        // Function types don't directly embed struct values
        TyKind::Function { .. } => None,
        // Primitives and other types don't create struct cycles
        _ => None,
    }
}
