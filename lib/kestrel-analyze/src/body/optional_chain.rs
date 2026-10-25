//! # Optional Chain Analyzer
//!
//! Checks `?.` chains against the mode inference settled for each bind
//! (`TypedBody::chain_modes`). A bind on a place lends its payload by
//! reference (`chainBorrow` / `chainMutate`), so the rest of the chain can't
//! consume it. Only a temporary base (`chainTake`) hands its payload over.
//!
//! ## Diagnostics
//!
//! ### E158 — `consume_through_optional_chain` (Error, Correctness)
//!
//! **Message:** "can't consume through `?.` on a place"
//!
//! **Labels:**
//! - Primary: the consuming call
//!   - Span source: `util::expr_span` on the `MethodCall` / `ProtocolCall` /
//!     `Call` `HirExprId` whose consuming receiver or argument is rooted at
//!     the bind's payload
//!   - Message: "this consumes the borrowed payload"
//!
//! **Notes:**
//! - "use `a.take()?.…` to move the value out"

use crate::body::move_tracking::ty_copy_semantics;
use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind, ReceiverKind};
use kestrel_hecs::Entity;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_semantics::CopySemantics;
use kestrel_type_infer::result::{ChainMode, ResolvedTy};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E158",
    name: "consume_through_optional_chain",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct OptionalChainAnalyzer;

impl Describe for OptionalChainAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::OptionalChain
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for OptionalChainAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Payload binders that are references into a place.
        let borrowed: Vec<LocalId> = cx
            .hir
            .optional_chain_binds()
            .into_iter()
            .filter(|b| cx.typed.chain_modes.get(&b.scrutinee) != Some(&ChainMode::Take))
            .map(|b| b.binder)
            .collect();
        if borrowed.is_empty() {
            return Vec::new();
        }
        // Consuming a Copyable or Cloneable value only duplicates it.
        let is_borrowed = |e: HirExprId| {
            cx.hir.place_root(e).is_some_and(|l| borrowed.contains(&l)) && moves(cx, e)
        };

        let mut diags = Vec::new();
        for (expr_id, expr) in cx.hir.exprs.iter() {
            let (callee, receiver, args) = match expr {
                HirExpr::MethodCall { receiver, args, .. } => (
                    cx.typed.resolutions.get(&expr_id).copied(),
                    Some(*receiver),
                    args,
                ),
                HirExpr::ProtocolCall {
                    receiver,
                    protocol,
                    method,
                    args,
                    ..
                } => {
                    let method = cx
                        .typed
                        .protocol_method_name(expr_id, method.as_str_or_empty());
                    (
                        protocol_method(cx, *protocol, method),
                        Some(*receiver),
                        args,
                    )
                },
                HirExpr::Call { callee, args, .. } => {
                    let entity = match &cx.hir.exprs[*callee] {
                        HirExpr::Def(entity, _, _) => Some(*entity),
                        _ => cx.typed.resolutions.get(callee).copied(),
                    };
                    (entity, None, args)
                },
                _ => continue,
            };
            let Some(callable) = callee.and_then(|c| cx.query.get::<Callable>(c)) else {
                continue;
            };
            let consumes_receiver = receiver.is_some_and(|r| {
                matches!(callable.receiver, Some(ReceiverKind::Consuming)) && is_borrowed(r)
            });
            let consumes_arg = args
                .iter()
                .zip(callable.params.iter())
                .any(|(arg, param)| param.is_consuming && is_borrowed(arg.value));
            if consumes_receiver || consumes_arg {
                diags.push(consume_diag(util::expr_span(cx.hir, expr_id)));
            }
        }
        diags
    }
}

fn consume_diag(span: kestrel_span::Span) -> AnalyzeDiagnostic {
    let d = &DESCRIPTORS[0];
    AnalyzeDiagnostic {
        descriptor_id: d.id,
        severity: d.default_severity,
        message: "can't consume through `?.` on a place".into(),
        labels: vec![DiagLabel {
            span,
            message: "this consumes the borrowed payload".into(),
            is_primary: true,
        }],
        notes: vec!["use `a.take()?.…` to move the value out".into()],
    }
}

/// Would consuming `expr` move it (its type, seen through a reference, is
/// neither Copyable nor Cloneable)?
fn moves(cx: &BodyContext<'_>, expr: HirExprId) -> bool {
    let Some(mut ty) = cx.typed.expr_types.get(&expr) else {
        return false;
    };
    while let ResolvedTy::Ref { pointee, .. } = ty {
        ty = pointee;
    }
    ty_copy_semantics(cx, ty) == CopySemantics::NotCopyable
}

/// The protocol's requirement named `name`.
fn protocol_method(cx: &BodyContext<'_>, protocol: Entity, name: &str) -> Option<Entity> {
    util::children_named_of_kind(cx.query, protocol, name, NodeKind::Function)
        .first()
        .copied()
}
