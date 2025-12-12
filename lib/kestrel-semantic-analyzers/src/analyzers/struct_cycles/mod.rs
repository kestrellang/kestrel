//! Analyzer for detecting circular struct containment (infinite-size types)

use std::sync::{Arc, Mutex};

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::behavior::typed::TypedBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::cycle::{Cycle, CycleDetector};
use semantic_tree::symbol::{Symbol, SymbolId};

use diagnostics::{CircularStructContainmentError, CycleMember, SelfContainingStructError};

pub struct StructCycleAnalyzer {
    structs: Mutex<Vec<CollectedStruct>>,
}

struct CollectedStruct {
    symbol: Arc<dyn Symbol<KestrelLanguage>>,
    struct_sym: Arc<StructSymbol>,
}

impl StructCycleAnalyzer {
    pub fn new() -> Self { Self { structs: Mutex::new(Vec::new()) } }
}

impl Default for StructCycleAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for StructCycleAnalyzer {
    fn name(&self) -> &'static str { "struct_cycles" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {
        if symbol.metadata().kind() == KestrelSymbolKind::Struct {
            if let Some(struct_sym) = symbol.clone().into_any_arc().downcast::<StructSymbol>().ok() {
                self.structs.lock().unwrap().push(CollectedStruct { symbol: symbol.clone(), struct_sym });
            }
        }
    }

    fn finalize(&mut self, ctx: &mut AnalysisContext) {
        let model = ctx.model;
        for collected in self.structs.lock().unwrap().iter() {
            check_struct_for_cycles(&collected.struct_sym, &collected.symbol, model, ctx);
        }
    }
}

fn check_struct_for_cycles(
    struct_sym: &StructSymbol,
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    model: &kestrel_semantic_model::SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let struct_id = struct_sym.metadata().id();
    let struct_name = struct_sym.metadata().name().value.clone();

    // Iterate fields
    for field in symbol.metadata().children() {
        if field.metadata().kind() != KestrelSymbolKind::Field { continue; }

        // Get the field's resolved type via TypedBehavior
        let field_ty = field.metadata().behaviors().iter().find_map(|b| {
            if matches!(b.kind(), KestrelBehaviorKind::Typed) {
                b.as_ref().downcast_ref::<TypedBehavior>().map(|tb| tb.ty().clone())
            } else { None }
        });
        let Some(field_ty) = field_ty else { continue; };

        let field_name = field.metadata().name().value.clone();
        let field_span = field.metadata().span().clone();

        let mut detector: CycleDetector<SymbolId> = CycleDetector::new();
        if detector.enter(struct_id).is_err() { continue; }

        if let Some(cycle) = check_type_for_struct_cycle(&field_ty, &mut detector) {
            if cycle.is_self_cycle() {
                ctx.report(SelfContainingStructError {
                    struct_name: struct_name.clone(),
                    struct_span: struct_sym.metadata().declaration_span().clone(),
                    field_name,
                    field_span,
                });
            } else {
                let origin = CycleMember { name: struct_name.clone(), span: struct_sym.metadata().declaration_span().clone() };
                let cycle_members: Vec<CycleMember> = cycle
                    .cycle()
                    .iter()
                    .skip(1)
                    .filter_map(|&id| {
                        model.query(kestrel_semantic_model::SymbolFor { id }).map(|s| CycleMember {
                            name: s.metadata().name().value.clone(),
                            span: s.metadata().declaration_span().clone(),
                        })
                    })
                    .collect();
                ctx.report(CircularStructContainmentError { origin, cycle: cycle_members, field_name, field_span });
            }
        }
        detector.exit();
    }
}

fn check_type_for_struct_cycle(ty: &Ty, detector: &mut CycleDetector<SymbolId>) -> Option<Cycle<SymbolId>> {
    match ty.kind() {
        TyKind::Struct { symbol, .. } => {
            let struct_id = symbol.metadata().id();
            if let Err(cycle) = detector.enter(struct_id) { return Some(cycle); }
            let struct_dyn = symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>;
            for field in struct_dyn.metadata().children() {
                if field.metadata().kind() != KestrelSymbolKind::Field { continue; }
                let field_ty = field.metadata().behaviors().iter().find_map(|b| {
                    if matches!(b.kind(), KestrelBehaviorKind::Typed) {
                        b.as_ref().downcast_ref::<TypedBehavior>().map(|tb| tb.ty().clone())
                    } else { None }
                });
                if let Some(ft) = field_ty {
                    if let Some(c) = check_type_for_struct_cycle(&ft, detector) {
                        detector.exit();
                        return Some(c);
                    }
                }
            }
            detector.exit();
            None
        }
        TyKind::Tuple(elements) => {
            for e in elements { if let Some(c) = check_type_for_struct_cycle(e, detector) { return Some(c); } }
            None
        }
        TyKind::Array(_) => None,
        TyKind::Function { .. } => None,
        _ => None,
    }
}

pub mod diagnostics;

