//! Type-position cursor & reference helpers.
//!
//! Type names in declaration positions (`func bar(x: Foo)`, return types,
//! struct fields, …) aren't recorded as resolved entities anywhere —
//! `ResolveTypePath` is the query that resolves them, but it's invoked
//! transiently during AST build / HIR lowering and the result isn't kept
//! on any node. So both:
//!
//! * **Cursor → entity** (hover, goto-def at a type-position cursor), and
//! * **Entity → cursor sites** (find-references for a target type)
//!
//! re-walk the file's CST and call `ResolveTypePath` to recover the
//! resolution. The work is bounded: type paths are typically short (one
//! or two segments) and scoped to per-file CSTs.

use kestrel_ast_builder::FilePath;
use kestrel_hecs::{Entity, World};
use kestrel_name_res::{ResolveTypePath, TypeResolution};
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use rowan::TextSize;

use crate::semantic::enclosing_decl_at;

/// Resolve a type-position cursor to an entity + the cursor identifier's
/// span. Returns `None` when the cursor isn't on an identifier inside a
/// `TyPath`, or when the path doesn't resolve.
///
/// The resolution context is the smallest enclosing decl, which is what
/// `ResolveTypePath` expects (it uses scope chains rooted at the decl).
pub fn type_at_cursor(
    world: &World,
    root: Entity,
    file_cst: &SyntaxNode,
    file_entity: Entity,
    offset: usize,
) -> Option<(Entity, Span)> {
    let pos = TextSize::from(offset as u32);
    let token = file_cst.token_at_offset(pos).right_biased()?;
    if token.kind() != SyntaxKind::Identifier {
        return None;
    }

    let path_element = token
        .parent_ancestors()
        .find(|n| n.kind() == SyntaxKind::PathElement)?;
    let path = path_element
        .parent()
        .filter(|n| n.kind() == SyntaxKind::Path)?;
    // Distinguish type paths from expr paths: the Path must live inside a
    // TyPath wrapper. Without this guard we'd resolve `foo` in `foo.bar()`
    // as a type, hijacking expression-side hover/goto-def.
    path.parent().filter(|n| n.kind() == SyntaxKind::TyPath)?;

    let mut segments: Vec<String> = Vec::new();
    for child in path.children() {
        if child.kind() != SyntaxKind::PathElement {
            continue;
        }
        let name = identifier_text(&child)?;
        segments.push(name);
        if child == path_element {
            break;
        }
    }
    if segments.is_empty() {
        return None;
    }

    let context = enclosing_decl_at(world, file_entity, offset)?;
    let ctx = world.query_context();
    let entity = match ctx.query(ResolveTypePath {
        segments,
        context,
        root,
    }) {
        TypeResolution::Found(e) => e,
        _ => return None,
    };

    let r = token.text_range();
    Some((
        entity,
        Span {
            file_id: file_entity.index(),
            start: r.start().into(),
            end: r.end().into(),
        },
    ))
}

/// Walk every `TyPath` in `file_cst` and collect identifier-spans whose
/// resolution lands on `target`. Each `PathElement` is resolved
/// independently (using the prefix of segments up to and including it),
/// so multi-segment paths like `std.collections.Array` produce a hit on
/// every segment that names `target` directly — typically just the last
/// segment for a leaf type.
pub fn type_references_in_file(
    world: &World,
    root: Entity,
    file_cst: &SyntaxNode,
    file_entity: Entity,
    target: Entity,
) -> Vec<Span> {
    let mut sites: Vec<Span> = Vec::new();
    for ty_path in file_cst
        .descendants()
        .filter(|n| n.kind() == SyntaxKind::TyPath)
    {
        let Some(path) = ty_path.children().find(|n| n.kind() == SyntaxKind::Path) else {
            continue;
        };
        let context = enclosing_decl_at(world, file_entity, path.text_range().start().into());
        let Some(context) = context else { continue };

        let mut segments: Vec<String> = Vec::new();
        for elem in path
            .children()
            .filter(|n| n.kind() == SyntaxKind::PathElement)
        {
            let Some(name) = identifier_text(&elem) else {
                continue;
            };
            segments.push(name);
            // Resolve the running prefix; record this segment if it lands on target.
            let ctx = world.query_context();
            if let TypeResolution::Found(e) = ctx.query(ResolveTypePath {
                segments: segments.clone(),
                context,
                root,
            }) {
                if e == target {
                    let ident = elem
                        .children_with_tokens()
                        .filter_map(|c| c.into_token())
                        .find(|t| t.kind() == SyntaxKind::Identifier);
                    if let Some(tok) = ident {
                        let r = tok.text_range();
                        sites.push(Span {
                            file_id: file_entity.index(),
                            start: r.start().into(),
                            end: r.end().into(),
                        });
                    }
                }
            }
        }
    }
    sites
}

/// Walk every `.ks` file in the workspace and collect type-position
/// references to `target`. Returns `(file_entity, ident_span)` pairs so
/// the LSP can convert them to per-file `Location`s.
pub fn type_references_workspace(
    world: &World,
    root: Entity,
    compiler: &kestrel_compiler::Compiler,
    target: Entity,
) -> Vec<(Entity, Span)> {
    let mut out: Vec<(Entity, Span)> = Vec::new();
    // Files are identified by carrying a `FilePath` directly; decls carry a
    // `FileId` pointing at the file entity instead.
    let file_entities: Vec<Entity> = world.iter_component::<FilePath>().map(|(e, _)| e).collect();
    for file_entity in file_entities {
        let cst = compiler.parse(file_entity).tree;
        for span in type_references_in_file(world, root, &cst, file_entity, target) {
            out.push((file_entity, span));
        }
    }
    out
}

fn identifier_text(node: &SyntaxNode) -> Option<String> {
    node.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| t.kind() == SyntaxKind::Identifier)
        .map(|t| t.text().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    #[test]
    fn type_at_cursor_resolves_param_type() {
        let src = "module T\nstruct Point { var x: lang.i64 }\nfunc f(p: Point) {}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/tac.ks", src.into());
        c.build(f);
        let cursor = src.find("p: Point").unwrap() + "p: Po".len(); // inside `Point`
        let cst = c.parse(f).tree;
        let (entity, span) = type_at_cursor(c.world(), c.root(), &cst, f, cursor)
            .expect("type at cursor must resolve");
        assert_eq!(&src[span.start..span.end], "Point");
        // The resolved entity must be the struct, not the function or anything else.
        use kestrel_ast_builder::Name;
        let name = c.world().get::<Name>(entity).map(|n| n.0.clone());
        assert_eq!(name, Some("Point".to_string()));
    }

    #[test]
    fn type_at_cursor_returns_none_in_expr_position() {
        // `Point` here is in an expression — not a TyPath. Resolution must
        // be left to the existing expr-side handlers.
        let src = "module T\nstruct Point { var x: lang.i64 }\nfunc f() { Point(x: 1); }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/tac_expr.ks", src.into());
        c.build(f);
        let cursor = src.find("Point(").unwrap() + 2;
        let cst = c.parse(f).tree;
        assert!(type_at_cursor(c.world(), c.root(), &cst, f, cursor).is_none());
    }

    #[test]
    fn type_references_collects_param_and_return_types() {
        let src = "module T\nstruct Point { var x: lang.i64 }\nfunc id(p: Point) -> Point { p }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/trefs.ks", src.into());
        c.build(f);
        // Find the Point struct entity.
        use kestrel_ast_builder::{FileId as F, Name};
        let point = c
            .world()
            .iter_component::<Name>()
            .find(|(e, n)| n.0 == "Point" && c.world().get::<F>(*e).map(|f2| f2.0) == Some(f))
            .map(|(e, _)| e)
            .expect("Point");
        let cst = c.parse(f).tree;
        let refs = type_references_in_file(c.world(), c.root(), &cst, f, point);
        let texts: Vec<&str> = refs.iter().map(|s| &src[s.start..s.end]).collect();
        // Two type-position references: `p: Point` and `-> Point`.
        let count = texts.iter().filter(|t| **t == "Point").count();
        assert_eq!(count, 2, "expected 2 Point refs; got {texts:?}");
    }
}
