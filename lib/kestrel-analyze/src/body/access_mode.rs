//! # Access Mode Analyzer
//!
//! Validates that arguments passed to `mutating` parameters are mutable lvalues.
//! A `let` binding, temporary value, or immutable field cannot be passed to a
//! `mutating` parameter since the callee would modify it.
//!
//! Walks all Call/MethodCall/ProtocolCall expressions, looks up the callee's
//! `Callable` component, and classifies each argument's mutability.
//!
//! ## Diagnostics
//!
//! - E203: `let` binding passed to `mutating` parameter
//! - E204: immutable field passed to `mutating` parameter
//! - E205: temporary value passed to `mutating` parameter

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind, ReceiverKind, Settable};
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E203",
        name: "let_to_mutating",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E204",
        name: "immutable_field_to_mutating",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E205",
        name: "rvalue_to_mutating",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E206",
        name: "let_to_consuming",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    // Stage-1 references (E-REF-20): mutating use through a SHARED `&T` —
    // the const-cast guard. Sibling of E203-E205, hence this family.
    DiagnosticDescriptor {
        id: "E207",
        name: "mutating_through_shared_ref",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    // Stage-1.5 named ref bindings: `&mutating expr` of a place that
    // isn't mutable (let local/field, shared ref, get/set-only member).
    DiagnosticDescriptor {
        id: "E210",
        name: "mutable_borrow_of_immutable",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    // Stage-1.5 named ref bindings: `&expr` of an rvalue — there is no
    // place to borrow.
    DiagnosticDescriptor {
        id: "E499",
        name: "borrow_of_temporary",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct AccessModeAnalyzer;

impl Describe for AccessModeAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::AccessMode
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// Result of classifying an argument expression's mutability.
enum MutClass {
    Mutable,
    ImmutableLocal(String), // local name
    ImmutableField(String), // field name
    /// A shared `&T` place (ret_borrow call result). Distinct from Temporary
    /// — temporaries are owned and may be mutated; a shared ref must not be.
    SharedRef,
    Temporary,
}

impl BodyCheck for AccessModeAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for (expr_id, expr) in cx.hir.exprs.iter() {
            match expr {
                HirExpr::Call { callee, args, .. } => {
                    // Resolve callee entity from typed resolutions
                    let callee_entity = match &cx.hir.exprs[*callee] {
                        HirExpr::Def(entity, _, _) => Some(*entity),
                        _ => cx.typed.resolutions.get(callee).copied(),
                    };
                    if let Some(entity) = callee_entity {
                        check_call_args(cx, entity, args, None, &mut diags);
                    }
                },
                HirExpr::MethodCall { receiver, args, .. } => {
                    // Method resolution stored on the MethodCall expr itself
                    if let Some(&entity) = cx.typed.resolutions.get(&expr_id) {
                        check_call_args(cx, entity, args, Some(*receiver), &mut diags);
                    }
                },
                HirExpr::ProtocolCall {
                    receiver,
                    protocol,
                    method,
                    args,
                    ..
                } => {
                    // Find the protocol method entity
                    if let Some(method_entity) =
                        find_protocol_method(cx, *protocol, method.as_str_or_empty())
                    {
                        check_call_args(cx, method_entity, args, Some(*receiver), &mut diags);
                    }
                },
                HirExpr::Borrow {
                    inner,
                    mutating,
                    span,
                } => {
                    check_borrow_init(cx, *inner, *mutating, span, &mut diags);
                },
                _ => {},
            }
        }

        diags
    }
}

/// Borrow-initializer legality (`let r = &expr;`, stage 1.5 item 2): a
/// borrow needs a PLACE (E499 otherwise); a `&mutating` borrow needs a
/// MUTABLE place (E210 otherwise). Mutability is `classify_mutability` —
/// the same predicate every access-mode check uses.
fn check_borrow_init(
    cx: &BodyContext<'_>,
    inner: HirExprId,
    mutating: bool,
    span: &kestrel_span::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let push = |diags: &mut Vec<AnalyzeDiagnostic>, d: usize, message: String, note: String| {
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[d].id,
            severity: DESCRIPTORS[d].default_severity,
            message,
            labels: vec![DiagLabel {
                span: span.clone(),
                message: "borrow initializer".into(),
                is_primary: true,
            }],
            notes: vec![note],
        });
    };
    const E210: usize = 5;
    const E499: usize = 6;

    // A get/set-only member has no mutable place to lend — the writeback
    // temp the lowering would borrow strands every later store made
    // through the binding.
    if mutating
        && let Some(&member) = cx.typed.resolutions.get(&inner)
        && matches!(
            cx.query.get::<NodeKind>(member),
            Some(NodeKind::Subscript | NodeKind::Field)
        )
        && util::accessor_place_mut_base(cx, inner).is_some()
        && !cx
            .query
            .query(kestrel_hir_lower::PlaceAccessors { entity: member })
            .is_some_and(|info| info.mutating_ref_accessor.is_some())
    {
        push(
            diags,
            E210,
            "cannot take a `&mutating` borrow of a get/set member".to_string(),
            "writes through this member go get→set; a borrowable place needs a \
             `mutating ref` accessor"
                .to_string(),
        );
        return;
    }

    match classify_mutability(cx, inner) {
        MutClass::Temporary => push(
            diags,
            E499,
            "cannot borrow a temporary value".to_string(),
            "a borrow names an existing place; bind the value first (`let x = ...;`) \
             and borrow that"
                .to_string(),
        ),
        MutClass::SharedRef if mutating => push(
            diags,
            E210,
            "cannot take a `&mutating` borrow through a shared reference".to_string(),
            "the place is reached through `&T`, which permits reads only".to_string(),
        ),
        MutClass::ImmutableLocal(name) if mutating => push(
            diags,
            E210,
            format!("cannot take a `&mutating` borrow of immutable variable '{name}'"),
            "declare the variable with `var`, or take a shared `&` borrow".to_string(),
        ),
        MutClass::ImmutableField(name) if mutating => push(
            diags,
            E210,
            format!("cannot take a `&mutating` borrow of immutable field '{name}'"),
            "the field is declared with `let`".to_string(),
        ),
        _ => {},
    }
}

/// Check arguments against the callee's parameter access modes.
fn check_call_args(
    cx: &BodyContext<'_>,
    callee: kestrel_hecs::Entity,
    args: &[HirCallArg],
    receiver: Option<HirExprId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let Some(callable) = cx.query.get::<Callable>(callee) else {
        return;
    };

    // Check receiver for mutating methods. Unlike argument position, a
    // receiver that is a temporary (call result, literal, if-expr, etc.) is
    // an owned mutable place — the caller owns it for the duration of the
    // call chain, so mutating it is fine. Only reject when the receiver is a
    // *named* immutable place (let binding or let field).
    if let (Some(recv_id), Some(recv_kind)) = (receiver, &callable.receiver)
        && matches!(recv_kind, ReceiverKind::Mutating)
    {
        check_mutating_receiver(cx, recv_id, diags);
    }

    // Check each argument against its corresponding parameter.
    // Only check mutating params (is_mut && !is_consuming).
    // Consuming params accept any argument (they take ownership).
    for (i, arg) in args.iter().enumerate() {
        if let Some(param) = callable.params.get(i)
            && param.is_mut
            && !param.is_consuming
        {
            check_mutating_arg(cx, arg.value, diags);
        }
    }
}

/// Check that a receiver passed to a `mutating self` method is acceptable.
/// Temporaries (call results, literals, etc.) are owned mutable places and
/// pass; only named immutable bindings/fields are rejected.
fn check_mutating_receiver(
    cx: &BodyContext<'_>,
    recv_id: HirExprId,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let span = util::expr_span(cx.hir, recv_id);
    match classify_mutability(cx, recv_id) {
        MutClass::Mutable | MutClass::Temporary => {}, // ok — owned or mutable place
        MutClass::SharedRef => {
            diags.push(shared_ref_diag(
                span,
                "cannot call a 'mutating' method through a shared reference",
            ));
        },
        MutClass::ImmutableLocal(name) => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "cannot pass immutable binding '{}' to 'mutating' parameter",
                    name
                ),
                labels: vec![DiagLabel {
                    span,
                    message: "cannot pass to 'mutating' parameter".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        },
        MutClass::ImmutableField(name) => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!(
                    "cannot pass immutable field '{}' to 'mutating' parameter",
                    name
                ),
                labels: vec![DiagLabel {
                    span,
                    message: "cannot pass to 'mutating' parameter".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        },
    }
}

/// Check that an argument to a `mutating` parameter is a mutable lvalue.
fn check_mutating_arg(cx: &BodyContext<'_>, arg_id: HirExprId, diags: &mut Vec<AnalyzeDiagnostic>) {
    let span = util::expr_span(cx.hir, arg_id);
    match classify_mutability(cx, arg_id) {
        MutClass::Mutable => {}, // ok
        MutClass::SharedRef => {
            diags.push(shared_ref_diag(
                span,
                "cannot pass a shared reference to a 'mutating' parameter",
            ));
        },
        MutClass::ImmutableLocal(name) => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "cannot pass immutable binding '{}' to 'mutating' parameter",
                    name
                ),
                labels: vec![DiagLabel {
                    span,
                    message: "cannot pass to 'mutating' parameter".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        },
        MutClass::ImmutableField(name) => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!(
                    "cannot pass immutable field '{}' to 'mutating' parameter",
                    name
                ),
                labels: vec![DiagLabel {
                    span,
                    message: "cannot pass to 'mutating' parameter".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        },
        MutClass::Temporary => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[2].id,
                severity: DESCRIPTORS[2].default_severity,
                message: "cannot pass temporary value to 'mutating' parameter".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "temporary values cannot be mutated".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        },
    }
}

/// E207: mutating use through a shared `&T` — the const-cast guard.
fn shared_ref_diag(span: kestrel_span::Span, message: &str) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[4].id,
        severity: DESCRIPTORS[4].default_severity,
        message: message.into(),
        labels: vec![DiagLabel {
            span,
            message: "this is a shared `&` reference".into(),
            is_primary: true,
        }],
        notes: vec![
            "mutation needs `&mutating` — use the mutable accessor (e.g. `mutableAt`, \
             `mutatingValue`) on a mutable base"
                .into(),
        ],
    }
}

/// Classify an expression's mutability for access mode checking.
fn classify_mutability(cx: &BodyContext<'_>, expr_id: HirExprId) -> MutClass {
    // Stage-1.5 accessor places FIRST: a member with a `mutating ref`
    // accessor is a place PROJECTION — classify the base (`x(i) += 1`
    // requires `x` mutable). Must precede the ref_place consult below:
    // a ref-provider member's READ types `&T`, which would wrongly
    // classify the projection as SharedRef.
    if let Some(base) = util::accessor_place_mut_base(cx, expr_id) {
        return classify_mutability(cx, base);
    }
    // Stage-1 refs next, before the syntactic walk: a `&mutating T` call
    // result is a mutable place (it would fall to Temporary below and
    // wrongly trip E205), and a shared `&T` must not fall to Temporary —
    // the receiver check ACCEPTS temporaries, so without this consult a
    // mutating method on a shared ref would silently pass (const-cast hole).
    match util::ref_place(cx, expr_id) {
        Some(true) => return MutClass::Mutable,
        Some(false) => return MutClass::SharedRef,
        None => {},
    }
    match &cx.hir.exprs[expr_id] {
        // Local variable reference — check if binding is mutable
        HirExpr::Local(local_id, _) => {
            let local = &cx.hir.locals[*local_id];
            // A closure param with an inferred `MutBorrow` convention (#106) is
            // a mutable place even without a `mutating` annotation, so calling a
            // mutating method / passing it to a mutating param is allowed.
            if local.is_mut || util::is_mut_borrow_param(cx, *local_id) {
                MutClass::Mutable
            } else {
                MutClass::ImmutableLocal(local.name.clone())
            }
        },
        // Field access — walk the chain checking field settability and base mutability
        HirExpr::Field { base, name, .. } => {
            // Check if the field entity itself is immutable (let field)
            if let Some(&field_entity) = cx.typed.resolutions.get(&expr_id)
                && !cx.query.has::<Settable>(field_entity)
            {
                return MutClass::ImmutableField(name.as_str_or_empty().to_string());
            }
            // Field is settable — check the base
            classify_mutability(cx, *base)
        },
        // Tuple index — like field access, check the base
        HirExpr::TupleIndex { base, .. } => classify_mutability(cx, *base),
        // Everything else is a temporary (call results, literals, if-exprs, etc.)
        _ => MutClass::Temporary,
    }
}

/// Find a method entity on a protocol by name.
fn find_protocol_method(
    cx: &BodyContext<'_>,
    protocol: kestrel_hecs::Entity,
    method_name: &str,
) -> Option<kestrel_hecs::Entity> {
    util::children_named_of_kind(cx.query, protocol, method_name, NodeKind::Function)
        .first()
        .copied()
}
