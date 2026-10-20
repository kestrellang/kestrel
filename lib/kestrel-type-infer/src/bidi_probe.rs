//! BIDI PROTOTYPE — local measurement/prototype hooks for the bidirectional
//! type-checking design (docs/design/bidirectional-typechecking.md). NOT for
//! commit. Every hook is inert unless its `KESTREL_DEBUG` category is set:
//!
//! - `bidi-stmt`: statement-scoped literal defaulting. Each statement outside a
//!   closure body is an inference region: at its end the solver runs to a
//!   fixpoint and every literal created inside the statement that is still
//!   unresolved takes its default. Literal types never flow across statements.
//! - `bidi-op`: symmetric operator literal rule. A literal receiver of an
//!   operator adopts the other operand's type when that type is concrete and
//!   conforms to the literal's `ExpressibleBy*` protocol (fixes N3).
//! - `bidi-recv`: measurement. Before every member lookup is emitted, solve
//!   everything generated so far and classify the receiver: is its type known
//!   at the point a left-to-right synth-first checker would need it?
//! - `bidi-n4`: measurement. At the end of `solve`, report every unannotated
//!   `let x = <numeric literal>` whose final type is not the literal default
//!   (sites whose meaning depends on cross-statement back-flow).

use kestrel_ast_builder::Name;
use kestrel_hir::Builtin;
use kestrel_hir::body::{HirBody, HirExpr, HirExprId, HirLiteral, HirStmt};
use kestrel_span::Span;

use crate::constraint::Constraint;
use crate::ctx::InferCtx;
use crate::solver;
use crate::ty::{LiteralKind, TyKind, TySlot, TyVar};
use crate::unify;

/// `bidi-stmt` hook, called after each statement at closure depth 0.
pub(crate) fn statement_boundary(ctx: &mut InferCtx<'_>, start: usize) {
    // Same graduated order as `solve`'s relax loop, but defaulting only the
    // literals this statement created: a literal whose receiver/callee is
    // still pending waits (level 0/1) so dispatch can pin it first — the
    // order a synth-receiver-first checker gets for free.
    let mut relax = 0u8;
    for _ in 0..64 {
        solver::fixpoint(ctx);
        if solver::apply_operator_shape_projections(ctx) {
            relax = 0;
            continue;
        }
        if kestrel_debug::is_enabled("bidi-op") && op_literal_adopts_operand(ctx) {
            relax = 0;
            continue;
        }
        if solver::apply_context_literals(ctx) {
            relax = 0;
            continue;
        }
        if solver::apply_literal_defaults_from(ctx, relax, start) {
            relax = 0;
            continue;
        }
        if solver::apply_ref_decay_defaults(ctx) {
            relax = 0;
            continue;
        }
        relax += 1;
        if relax > 2 {
            break;
        }
    }
}

/// `bidi-op` rule: `lit op x` with `x: T` concrete and `T: ExpressibleBy<lit>`
/// pins the literal to `T` (the mirror of what `x op lit` already does via
/// argument coercion). Returns true on progress.
pub(crate) fn op_literal_adopts_operand(ctx: &mut InferCtx<'_>) -> bool {
    let mut pairs: Vec<(TyVar, TyVar)> = Vec::new();
    for c in &ctx.constraints {
        let Constraint::Member {
            receiver,
            args,
            expr,
            ..
        } = c
        else {
            continue;
        };
        if !ctx.operator_members.contains(expr) || args.len() != 1 {
            continue;
        }
        let recv = ctx.resolve(*receiver);
        let TySlot::Unresolved { literal: Some(lit) } = ctx.slot(recv) else {
            continue;
        };
        let arg = ctx.resolve(args[0].ty);
        let TySlot::Resolved(kind) = ctx.slot(arg) else {
            continue;
        };
        if !kind.is_nominal_concrete() || !unify::conforms_to_literal_protocol(ctx, kind, *lit) {
            continue;
        }
        pairs.push((recv, arg));
    }
    let mut progress = false;
    for (recv, arg) in pairs {
        if matches!(ctx.slot(ctx.resolve(recv)), TySlot::Unresolved { literal: Some(_) })
            && unify::unify(ctx, recv, arg).is_ok()
        {
            kestrel_debug::ktrace!("bidi-op", "literal receiver adopts operand type");
            progress = true;
        }
    }
    progress
}

fn default_builtin(lit: LiteralKind) -> Builtin {
    match lit {
        LiteralKind::Integer => Builtin::DefaultIntegerLiteralType,
        LiteralKind::Float => Builtin::DefaultFloatLiteralType,
        LiteralKind::String => Builtin::DefaultStringLiteralType,
        LiteralKind::Bool => Builtin::DefaultBooleanLiteralType,
        LiteralKind::Char => Builtin::DefaultCharLiteralType,
        LiteralKind::Null => Builtin::DefaultNullLiteralType,
        LiteralKind::Array => Builtin::DefaultArrayLiteralType,
        LiteralKind::Dictionary => Builtin::DefaultDictionaryLiteralType,
        LiteralKind::StringInterpolation => Builtin::DefaultStringInterpolation,
    }
}

/// `bidi-recv` measurement, called just before a member lookup is emitted.
pub(crate) fn receiver(
    ctx: &mut InferCtx<'_>,
    hir: &HirBody,
    recv_expr: HirExprId,
    recv_tv: TyVar,
    site: &str,
    span: &Span,
) {
    if !kestrel_debug::is_enabled("bidi-recv") {
        return;
    }
    // Simulate what a synth-first walker already knows here: everything to the
    // left is solved, Rule L applied eagerly, ref values decayed into
    // unannotated bindings.
    for _ in 0..16 {
        solver::fixpoint(ctx);
        let l = op_literal_adopts_operand(ctx);
        let d = solver::apply_ref_decay_defaults(ctx);
        if !l && !d {
            break;
        }
    }
    let root = ctx.resolve(recv_tv);
    let class = match ctx.slot(root) {
        TySlot::Resolved(TyKind::Error) => "error".to_string(),
        TySlot::Resolved(_) => "known".to_string(),
        TySlot::Unresolved { literal: Some(k) } => format!("literal-{k:?}"),
        TySlot::Unresolved { literal: None } => {
            format!("unknown<-{}", root_cause(ctx, hir, recv_expr, 0))
        },
        TySlot::Redirect(_) => unreachable!(),
    };
    kestrel_debug::ktrace!(
        "bidi-recv",
        "site={site} class={class} owner={} file={} at={}",
        owner_name(ctx),
        span.file_id,
        span.start
    );
}

/// An argument whose type is still an open LITERAL var (or rooted in one).
fn is_lit_arg(ctx: &InferCtx<'_>, hir: &HirBody, e: HirExprId) -> bool {
    let Some(tv) = ctx.expr_types.get(&e) else {
        return false;
    };
    match ctx.slot(ctx.resolve(*tv)) {
        TySlot::Unresolved { literal: Some(_) } => true,
        TySlot::Unresolved { literal: None } => root_cause(ctx, hir, e, 30).contains("literal"),
        _ => false,
    }
}

/// For a member/call whose receiver and literal args are fine: the first
/// still-unresolved argument's root cause, or "args-known".
fn unresolved_arg_cause(
    ctx: &InferCtx<'_>,
    hir: &HirBody,
    args: &[kestrel_hir::body::HirCallArg],
    depth: u32,
) -> String {
    args.iter()
        .find(|a| is_unresolved(ctx, a.value))
        .map(|a| format!("arg<-{}", root_cause(ctx, hir, a.value, depth + 1)))
        .unwrap_or_else(|| "args-known".into())
}

fn is_unresolved(ctx: &InferCtx<'_>, e: HirExprId) -> bool {
    ctx.expr_types
        .get(&e)
        .is_some_and(|tv| matches!(ctx.slot(ctx.resolve(*tv)), TySlot::Unresolved { .. }))
}

/// Walk an unknown receiver back to the first thing a bidirectional walker
/// would NOT already know. Benign roots (resolved by construction in the
/// design): a literal (Rule R/L/C), a closure parameter (checked against its
/// expectation), a desugaring temp (`$dsi` interpolation accumulator, `$iter`),
/// a pattern binder (patterns check against a synthesized scrutinee).
fn root_cause(ctx: &InferCtx<'_>, hir: &HirBody, mut e: HirExprId, depth: u32) -> String {
    if depth > 40 {
        return "deep".into();
    }
    while let HirExpr::Sugar { inner, .. } = &hir.exprs[e] {
        e = *inner;
    }
    match &hir.exprs[e] {
        HirExpr::Literal { .. } => "literal".into(),
        HirExpr::Local(l, _) => {
            let is_closure_param = hir.exprs.iter().any(|(_, x)| {
                matches!(x, HirExpr::Closure { params, .. } if params.iter().any(|p| p.local == *l))
            });
            if is_closure_param {
                return "closure-param".into();
            }
            if hir.locals[*l].name.starts_with('$') {
                return "desugar-temp".into();
            }
            if let Some(tv) = ctx.local_types.get(l)
                && ctx.pattern_binder_tvs.contains(tv)
            {
                return "pattern-binder".into();
            }
            let init = hir.stmts.iter().find_map(|(_, s)| match s {
                HirStmt::Let {
                    local, value: Some(v), ..
                } if local == l => Some(*v),
                _ => None,
            });
            match init {
                Some(v) => format!("local<-{}", root_cause(ctx, hir, v, depth + 1)),
                None => "pattern-local".into(),
            }
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => {
            if is_unresolved(ctx, *receiver) {
                root_cause(ctx, hir, *receiver, depth + 1)
            } else if args.iter().any(|a| is_lit_arg(ctx, hir, a.value)) {
                // Waiting on a literal argument: the design's overload step 3
                // (literal compatibility) or Rule C decides it on the spot.
                "member-with-literal-arg".into()
            } else {
                format!("member-result({})", unresolved_arg_cause(ctx, hir, args, depth))
            }
        },
        HirExpr::Field { base, .. } => {
            if is_unresolved(ctx, *base) {
                root_cause(ctx, hir, *base, depth + 1)
            } else {
                "field-result".into()
            }
        },
        HirExpr::Call { callee, args, .. } => {
            if matches!(hir.exprs[*callee], HirExpr::OverloadSet { .. }) {
                "overloaded-call-result".into()
            } else if is_unresolved(ctx, *callee) {
                root_cause(ctx, hir, *callee, depth + 1)
            } else if args.iter().any(|a| is_lit_arg(ctx, hir, a.value)) {
                "call-with-literal-arg".into()
            } else {
                format!("call-result({})", unresolved_arg_cause(ctx, hir, args, depth))
            }
        },
        HirExpr::ImplicitMember { .. } => "implicit-member".into(),
        HirExpr::Match { .. } | HirExpr::If { .. } | HirExpr::Block { .. } => {
            "control-flow".into()
        },
        HirExpr::TupleIndex { base, .. } => {
            if is_unresolved(ctx, *base) {
                root_cause(ctx, hir, *base, depth + 1)
            } else {
                "tuple-index".into()
            }
        },
        other => format!("other({})", variant_name(other)),
    }
}

#[allow(dead_code)]
fn classify_unknown(ctx: &InferCtx<'_>, hir: &HirBody, mut e: HirExprId) -> String {
    while let HirExpr::Sugar { inner, .. } = &hir.exprs[e] {
        e = *inner;
    }
    match &hir.exprs[e] {
        HirExpr::Local(l, _) => {
            let is_closure_param = hir.exprs.iter().any(|(_, x)| {
                matches!(x, HirExpr::Closure { params, .. } if params.iter().any(|p| p.local == *l))
            });
            if is_closure_param {
                return "unknown-closure-param".into();
            }
            if let Some(tv) = ctx.local_types.get(l)
                && ctx.pattern_binder_tvs.contains(tv)
            {
                return "unknown-pattern-binder".into();
            }
            format!("unknown-local({})", hir.locals[*l].name)
        },
        HirExpr::Call { .. } => "unknown-call-result".into(),
        HirExpr::MethodCall { .. } | HirExpr::ProtocolCall { .. } => {
            "unknown-method-result".into()
        },
        HirExpr::Field { .. } => "unknown-field".into(),
        HirExpr::ImplicitMember { .. } => "unknown-implicit-member".into(),
        HirExpr::Match { .. } | HirExpr::If { .. } | HirExpr::Block { .. } => {
            "unknown-control-flow".into()
        },
        other => format!("unknown-other({})", variant_name(other)),
    }
}

fn variant_name(e: &HirExpr) -> &'static str {
    match e {
        HirExpr::Literal { .. } => "Literal",
        HirExpr::Tuple { .. } => "Tuple",
        HirExpr::Array { .. } => "Array",
        HirExpr::Dict { .. } => "Dict",
        HirExpr::Closure { .. } => "Closure",
        HirExpr::Def(..) => "Def",
        HirExpr::TypeRef { .. } => "TypeRef",
        HirExpr::TupleIndex { .. } => "TupleIndex",
        HirExpr::Borrow { .. } => "Borrow",
        HirExpr::Error { .. } => "Error",
        _ => "?",
    }
}

pub(crate) fn owner_name(ctx: &InferCtx<'_>) -> String {
    let own = ctx
        .query_ctx
        .get::<Name>(ctx.owner)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "?".into());
    let parent = ctx
        .query_ctx
        .parent_of(ctx.owner)
        .and_then(|p| ctx.query_ctx.get::<Name>(p))
        .map(|n| n.0.clone())
        .unwrap_or_default();
    format!("{parent}.{own}")
}

/// Short display of a TyVar's resolved type (measurement output only).
pub(crate) fn ty_name(ctx: &InferCtx<'_>, tv: TyVar, depth: u32) -> String {
    if depth > 4 {
        return "..".into();
    }
    let root = ctx.resolve(tv);
    let name = |e| {
        ctx.query_ctx
            .get::<Name>(e)
            .map(|n| n.0.clone())
            .unwrap_or_else(|| "?".into())
    };
    match ctx.slot(root) {
        TySlot::Resolved(TyKind::Struct { entity, args })
        | TySlot::Resolved(TyKind::Enum { entity, args })
        | TySlot::Resolved(TyKind::Protocol { entity, args }) => {
            if args.is_empty() {
                name(*entity)
            } else {
                let a: Vec<String> = args.iter().map(|a| ty_name(ctx, *a, depth + 1)).collect();
                format!("{}[{}]", name(*entity), a.join(","))
            }
        },
        TySlot::Resolved(TyKind::Param { entity }) => name(*entity),
        TySlot::Resolved(TyKind::Ref { pointee, .. }) => {
            format!("&{}", ty_name(ctx, *pointee, depth + 1))
        },
        TySlot::Resolved(TyKind::Error) => "<error>".into(),
        TySlot::Resolved(k) => format!("{:?}", std::mem::discriminant(k)),
        TySlot::Unresolved { literal } => format!("?{literal:?}"),
        TySlot::Redirect(_) => unreachable!(),
    }
}

/// `bidi-n4` measurement, called at the end of `solve`.
pub(crate) fn report_literal_lets(ctx: &InferCtx<'_>, hir: &HirBody) {
    if !kestrel_debug::is_enabled("bidi-n4") {
        return;
    }
    for (_, stmt) in hir.stmts.iter() {
        let HirStmt::Let {
            local,
            ty: None,
            value: Some(v),
            ..
        } = stmt
        else {
            continue;
        };
        let Some(lit) = numeric_literal_kind(hir, *v) else {
            continue;
        };
        let Some(&tv) = ctx.local_types.get(local) else {
            continue;
        };
        let default = ctx
            .resolver
            .builtin(default_builtin(lit))
            .and_then(|e| ctx.query_ctx.get::<Name>(e).map(|n| n.0.clone()))
            .unwrap_or_default();
        let fin = ty_name(ctx, tv, 0);
        let verdict = if fin == default { "default" } else { "NONDEFAULT" };
        kestrel_debug::ktrace!(
            "bidi-n4",
            "{verdict} let {} = <{lit:?}> final={fin} owner={}",
            hir.locals[*local].name,
            owner_name(ctx)
        );
    }
}

/// `bidi-csid` measurement (with `bidi-stmt`): an unannotated `let` whose type
/// still contains unknowns after its statement was solved and its literals
/// defaulted — a binding that depends on LATER statements to be typed (legal
/// with body-scoped inference variables, rejected by per-statement inference).
pub(crate) fn report_open_let(
    ctx: &InferCtx<'_>,
    hir: &HirBody,
    id: kestrel_hir::body::HirStmtId,
) {
    if !kestrel_debug::is_enabled("bidi-csid") {
        return;
    }
    let HirStmt::Let {
        local,
        ty: None,
        value,
        ..
    } = &hir.stmts[id]
    else {
        return;
    };
    let Some(&tv) = ctx.local_types.get(local) else {
        return;
    };
    let shape = ty_name(ctx, tv, 0);
    if !shape.contains('?') {
        return;
    }
    let init = match value {
        Some(v) => {
            let mut e = *v;
            while let HirExpr::Sugar { inner, .. } = &hir.exprs[e] {
                e = *inner;
            }
            variant_name_full(&hir.exprs[e])
        },
        None => "none",
    };
    kestrel_debug::ktrace!(
        "bidi-csid",
        "open-let {} : {shape} init={init} owner={}",
        hir.locals[*local].name,
        owner_name(ctx)
    );
}

fn variant_name_full(e: &HirExpr) -> &'static str {
    match e {
        HirExpr::Call { .. } => "Call",
        HirExpr::MethodCall { .. } => "MethodCall",
        HirExpr::ProtocolCall { .. } => "ProtocolCall",
        HirExpr::Field { .. } => "Field",
        HirExpr::ImplicitMember { .. } => "ImplicitMember",
        HirExpr::Local(..) => "Local",
        HirExpr::If { .. } => "If",
        HirExpr::Match { .. } => "Match",
        HirExpr::Block { .. } => "Block",
        other => variant_name(other),
    }
}

/// `bidi-ret` measurement: a body with an omitted return type and a tail
/// value. Today the tail flows into a fresh var (callers see unit).
pub(crate) fn report_omitted_return(ctx: &InferCtx<'_>) {
    if !kestrel_debug::is_enabled("bidi-ret") || !ctx.probe_omitted_return {
        return;
    }
    let fin = ty_name(ctx, ctx.return_ty, 0);
    let root = ctx.resolve(ctx.return_ty);
    let unit = matches!(ctx.slot(root), TySlot::Resolved(TyKind::Tuple(e)) if e.is_empty())
        || matches!(ctx.slot(root), TySlot::Resolved(TyKind::Never));
    let verdict = if unit { "unit" } else { "VALUE" };
    kestrel_debug::ktrace!("bidi-ret", "{verdict} tail={fin} owner={}", owner_name(ctx));
}

fn numeric_literal_kind(hir: &HirBody, mut e: HirExprId) -> Option<LiteralKind> {
    loop {
        match &hir.exprs[e] {
            HirExpr::Sugar { inner, .. } => e = *inner,
            // Unary negation desugars to a receiver-only operator call.
            HirExpr::ProtocolCall {
                receiver,
                args,
                from_operator: true,
                ..
            } if args.is_empty() => e = *receiver,
            HirExpr::Literal {
                value: HirLiteral::Integer(_),
                ..
            } => return Some(LiteralKind::Integer),
            HirExpr::Literal {
                value: HirLiteral::Float(_),
                ..
            } => return Some(LiteralKind::Float),
            _ => return None,
        }
    }
}
