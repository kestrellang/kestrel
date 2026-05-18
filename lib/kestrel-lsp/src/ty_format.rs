//! Render `ResolvedTy` to a human-readable string.
//!
//! `kestrel_type_infer::result::ResolvedTy` doesn't ship a `Display` impl
//! because it carries entities — formatting needs a `World` to look up
//! names. We do that here.

use kestrel_ast_builder::Name;
use kestrel_hecs::{Entity, World};
use kestrel_type_infer::result::ResolvedTy;

pub fn format_ty(world: &World, ty: &ResolvedTy) -> String {
    let mut out = String::new();
    write_ty(world, ty, &mut out);
    out
}

fn write_ty(world: &World, ty: &ResolvedTy, out: &mut String) {
    match ty {
        ResolvedTy::Named { entity, args } => {
            out.push_str(&entity_path(world, *entity));
            if !args.is_empty() {
                out.push('[');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write_ty(world, a, out);
                }
                out.push(']');
            }
        },
        ResolvedTy::Param { entity } => {
            out.push_str(&name_of(world, *entity).unwrap_or_else(|| "?".into()));
        },
        ResolvedTy::SelfType { .. } => out.push_str("Self"),
        ResolvedTy::Tuple(elems) => {
            out.push('(');
            for (i, e) in elems.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_ty(world, e, out);
            }
            if elems.len() == 1 {
                out.push(',');
            }
            out.push(')');
        },
        ResolvedTy::Function { params, ret } => {
            out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_ty(world, p, out);
            }
            out.push_str(") -> ");
            write_ty(world, ret, out);
        },
        ResolvedTy::AssocProjection { base, assoc } => {
            write_ty(world, base, out);
            out.push('.');
            out.push_str(&name_of(world, *assoc).unwrap_or_else(|| "?".into()));
        },
        ResolvedTy::Opaque { bounds, .. } => {
            out.push_str("some ");
            for (i, (proto, _)) in bounds.iter().enumerate() {
                if i > 0 {
                    out.push_str(" and ");
                }
                out.push_str(&entity_path(world, *proto));
            }
            if bounds.is_empty() {
                out.push('?');
            }
        },
        ResolvedTy::Never => out.push('!'),
        ResolvedTy::Error => out.push_str("<error>"),
    }
}

fn name_of(world: &World, entity: Entity) -> Option<String> {
    world.get::<Name>(entity).map(|n| n.0.clone())
}

/// Build a dotted path from the root: `std.core.String`. Falls back to the
/// raw name if the chain is broken.
fn entity_path(world: &World, entity: Entity) -> String {
    let mut parts = Vec::new();
    let mut cur = Some(entity);
    while let Some(e) = cur {
        if let Some(n) = name_of(world, e) {
            if n == "<root>" {
                break;
            }
            parts.push(n);
        } else {
            break;
        }
        cur = world.parent_of(e);
    }
    parts.reverse();
    if parts.is_empty() {
        format!("?{:?}", entity)
    } else {
        parts.join(".")
    }
}
