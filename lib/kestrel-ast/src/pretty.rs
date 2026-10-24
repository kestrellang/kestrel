//! Rendering type syntax back to source.

use crate::ast_type::{AnyBound, AstType};

/// Render an `AstType` as the Kestrel syntax that produced it.
///
/// **The only type renderer.** `kestrel-doc` used to carry a fork of this
/// function; both copies printed the never type as `Never`, which is not
/// Kestrel syntax — the language writes `!` — so the generated stdlib
/// reference published `func fatalError(String) -> Never`, a signature that
/// does not parse. Callers outside this crate use this; do not re-implement.
pub fn format_type(ty: &AstType) -> String {
    match ty {
        AstType::Named { segments, .. } => segments
            .iter()
            .map(|s| {
                if s.type_args.is_empty() {
                    s.name.clone()
                } else {
                    let args: Vec<_> = s.type_args.iter().map(format_type).collect();
                    format!("{}[{}]", s.name, args.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("."),
        AstType::Tuple(elems, _) => {
            let inner: Vec<_> = elems.iter().map(format_type).collect();
            format!("({})", inner.join(", "))
        },
        AstType::Function {
            kind,
            params,
            return_type,
            ..
        } => {
            let p: Vec<_> = params.iter().map(format_type).collect();
            format!(
                "{}({}) -> {}",
                kind.prefix(),
                p.join(", "),
                format_type(return_type)
            )
        },
        AstType::Array(inner, _) => format!("[{}]", format_type(inner)),
        AstType::Dictionary(k, v, _) => format!("[{}: {}]", format_type(k), format_type(v)),
        AstType::Optional(inner, _) => format!("{}?", format_type(inner)),
        AstType::Result { ok, err, .. } => {
            format!("{} throws {}", format_type(ok), format_type(err))
        },
        AstType::Unit(_) => "()".into(),
        AstType::Never(_) => "!".into(),
        AstType::Inferred(_) => "_".into(),
        AstType::Some {
            bounds, negative, ..
        } => {
            let b: Vec<_> = bounds.iter().map(format_type).collect();
            match negative {
                Some(neg) => format!("some {} and not {}", b.join(" and "), format_type(neg)),
                None => format!("some {}", b.join(" and ")),
            }
        },
        AstType::Any { box_ty, bounds, .. } => {
            let b: Vec<_> = bounds.iter().map(format_any_bound).collect();
            match box_ty {
                Some(bx) => format!("any[{}] {}", format_type(bx), b.join(" and ")),
                None => format!("any {}", b.join(" and ")),
            }
        },
        AstType::Ref {
            inner, mutating, ..
        } => {
            let kw = if *mutating { "&mutating " } else { "&" };
            format!("{kw}{}", format_type(inner))
        },
    }
}

/// `P` or `P[A = X, B = Y]`. The protocol's own type arguments, if any,
/// come first: `P[Int64, A = X]`.
fn format_any_bound(bound: &AnyBound) -> String {
    let base = format_type(&bound.protocol);
    if bound.bindings.is_empty() {
        return base;
    }
    let pins: Vec<_> = bound
        .bindings
        .iter()
        .map(|b| format!("{} = {}", b.name, format_type(&b.ty)))
        .collect();
    match base.strip_suffix(']') {
        Some(open) => format!("{open}, {}]", pins.join(", ")),
        None => format!("{base}[{}]", pins.join(", ")),
    }
}
