//! # Unsized Parameter Analyzer (existentials)
//!
//! A type parameter relaxed with `where T: not Sized` may be instantiated at
//! a raw existential, whose size is only known at runtime. Such a `T` can
//! only be used behind a pointer (`Pointer[T]`, `&T`, or another `not Sized`
//! type argument) or as the **last stored field** of a struct (an unsized
//! tail). Everything that takes or returns `T` by value belongs in an
//! `extend … where T: Sized` extension, where `T` is Sized again
//! (docs/plans/existentials/existentials-design.md §2).
//!
//! Checks declaration signatures: stored fields, computed fields, function /
//! initializer / subscript parameters and return types, and enum payloads.
//! A reference-typed parameter (`x: &T`, `x: &mutating T`) is behind a
//! pointer, so a bare `T` there is exempt too.
//! A field or subscript whose accessors are all `ref` / `mutating ref` /
//! `modifying ref` only lends `&T`, so its declared type is behind a
//! reference and exempt. Uses inside bodies are covered by the implicit `T: Sized` bound on every
//! generic they reach and by member availability.
//!
//! ## Diagnostics
//!
//! ### E403 -- `unsized_param_by_value` (Error, Correctness)
//!
//! **Message:** "'{T}' may be unsized here ('where {T}: not Sized'), so it can
//! only be used behind a pointer or as the last stored field"
//!
//! **Labels:**
//! - Primary: the declaration — "uses '{T}' by value"
//!
//! **Notes:**
//! - "move by-value members into an extension 'where {T}: Sized'"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, DeclaredAccessors, FieldClass, Intrinsic, NodeKind};
use kestrel_hecs::Entity;
use kestrel_hir::{Builtin, HirTy};
use kestrel_hir_lower::{LowerCallableReturnType, LowerCallableTypes, LowerTypeAnnotation};
use kestrel_name_res::ResolveBuiltin;
use kestrel_semantics::{last_stored_field, relaxed_param_by_value};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E403",
    name: "unsized_param_by_value",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct UnsizedParamAnalyzer;

impl Describe for UnsizedParamAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::UnsizedParam
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for UnsizedParamAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Field,
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::Subscript,
            NodeKind::EnumCase,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Stdlib-less fixtures without the builtin can't spell `not Sized`.
        if cx
            .query
            .query(ResolveBuiltin {
                builtin: Builtin::Sized,
                root: cx.root,
            })
            .is_none()
        {
            return vec![];
        }
        // Intrinsics (`lang.ptr_read[T]`, …) take any `T` at the ABI level;
        // they are the primitives sized code is built from, not users of it.
        let is_intrinsic = |e: Entity| cx.query.get::<Intrinsic>(e).is_some();
        if is_intrinsic(cx.entity) || cx.query.parent_of(cx.entity).is_some_and(is_intrinsic) {
            return vec![];
        }
        let offender = signature_types(cx).into_iter().find_map(|(ty, tail_ok)| {
            if tail_ok && matches!(ty, HirTy::Param(..)) {
                return None;
            }
            relaxed_param_by_value(cx.query, &ty, cx.entity, cx.root)
        });
        let Some(param) = offender else {
            return vec![];
        };
        let name = util::entity_name(cx.query, param);
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "'{name}' may be unsized here ('where {name}: not Sized'), so it can only be \
                 used behind a pointer or as the last stored field"
            ),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: format!("uses '{name}' by value"),
                is_primary: true,
            }],
            notes: vec![format!(
                "move by-value members into an extension 'where {name}: Sized'"
            )],
        }]
    }
}

/// The declaration's signature types, each flagged with whether a bare
/// unsized `T` is allowed there: the last stored field of a struct (an
/// unsized tail) or a reference-typed param (`s: &S`).
pub(crate) fn signature_types(cx: &DeclContext<'_>) -> Vec<(HirTy, bool)> {
    let q = cx.query;
    let entity = cx.entity;
    match q.get::<NodeKind>(entity) {
        Some(NodeKind::Field) => {
            if lends_only(cx) {
                return vec![];
            }
            let Some(ty) = q.query(LowerTypeAnnotation {
                entity,
                root: cx.root,
            }) else {
                return vec![];
            };
            let stored = q
                .get::<FieldClass>(entity)
                .is_some_and(FieldClass::is_stored_instance);
            let is_tail = stored
                && q.parent_of(entity).is_some_and(|owner| {
                    q.get::<NodeKind>(owner) == Some(&NodeKind::Struct)
                        && last_stored_field(q, owner) == Some(entity)
                });
            vec![(ty, is_tail)]
        },
        Some(kind) => {
            // A reference-typed param (`s: &S`, `s: &mutating S`) is behind
            // a pointer: `func sum[S](s: &S) where S: not Sized` takes a
            // `Slice[T]` or a raw existential in place. A plain `s: S`
            // stays by value. Enum payloads are storage, never refs.
            let params = q
                .get::<Callable>(entity)
                .map(|c| c.params.clone())
                .unwrap_or_default();
            let borrowed =
                |i: usize| kind != &NodeKind::EnumCase && params.get(i).is_some_and(|p| p.by_ref);
            let mut out: Vec<(HirTy, bool)> = q
                .query(LowerCallableTypes {
                    entity,
                    root: cx.root,
                })
                .unwrap_or_default()
                .into_iter()
                .enumerate()
                .filter_map(|(i, t)| t.map(|t| (t, borrowed(i))))
                .collect();
            if q.get::<NodeKind>(entity) != Some(&NodeKind::EnumCase) && !lends_only(cx) {
                out.push((
                    q.query(LowerCallableReturnType {
                        entity,
                        root: cx.root,
                    }),
                    false,
                ));
            }
            out
        },
        None => vec![],
    }
}

/// The member only lends places (`ref` accessors, no `get`/`set`): its type
/// is reached through `&T`, never by value.
fn lends_only(cx: &DeclContext<'_>) -> bool {
    let Some(acc) = cx.query.get::<DeclaredAccessors>(cx.entity) else {
        return false;
    };
    let lends = acc.ref_ || acc.modifying_ref || acc.mutating_ref;
    lends && !acc.get && !acc.modifying_get && !acc.set
}
