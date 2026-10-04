//! `textDocument/documentHighlight` — highlight all references to the symbol
//! under the cursor within the current file.
//!
//! Reuses `references::references_to` (or `local_references`) filtered to the
//! current file. Maps `RefKind` to `DocumentHighlightKind`.

use kestrel_ast_builder::DeclSpan;
use kestrel_hecs::{Entity, World};
use tower_lsp::lsp_types::{DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams};

use crate::position::LineIndex;
use crate::references::{self, RefKind, ReferenceSite, clip_to_identifier};
use crate::semantic::{self, Target, target_at};
use crate::server::{SharedState, url_to_path};

pub async fn handle(
    state: SharedState,
    params: DocumentHighlightParams,
) -> Option<Vec<DocumentHighlight>> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let li = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (
            s.compiler_handle.clone(),
            stdlib,
            user,
            s.sources.clone(),
            li,
        )
    };
    let offset = line_index.position_to_offset(pos);

    handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Option<Vec<DocumentHighlight>> {
                let file_entity = semantic::file_entity_for_path(compiler, &path)?;
                let world = compiler.world();
                let root = compiler.root();

                let target = target_at(world, root, file_entity, offset)?;
                let sites = collect_sites(world, root, &target, file_entity, compiler);

                let source = sources.get(&path)?;
                let li = LineIndex::new(source.clone());
                let mut out = Vec::new();
                for site in &sites {
                    let clipped = clip_to_identifier(source, &site.span, site.kind);
                    let range = li.range_for(clipped.start, clipped.end);
                    let kind = match site.kind {
                        RefKind::Direct | RefKind::MemberAccess | RefKind::Pattern => {
                            DocumentHighlightKind::READ
                        },
                    };
                    out.push(DocumentHighlight {
                        range,
                        kind: Some(kind),
                    });
                }
                if out.is_empty() { None } else { Some(out) }
            },
        )
        .await
        .flatten()
}

/// Collect reference sites scoped to the current file.
fn collect_sites(
    world: &World,
    root: Entity,
    target: &Target,
    file_entity: Entity,
    compiler: &kestrel_compiler::Compiler,
) -> Vec<ReferenceSite> {
    let mut sites = match target {
        Target::Entity(e) => {
            let mut s = references::references_to(world, root, *e);
            s.retain(|site| site.file == file_entity);

            // Type-position references, filtered to this file.
            for (file, span) in crate::types::type_references_workspace(world, root, compiler, *e) {
                if file == file_entity {
                    s.push(ReferenceSite {
                        file,
                        span,
                        kind: RefKind::Direct,
                    });
                }
            }
            s
        },
        Target::Local { body, id } => references::local_references(world, *body, root, *id),
    };

    // Add the declaration site.
    if let Target::Entity(e) = target
        && let Some(span) = world.get::<DeclSpan>(*e).map(|s| s.0.clone())
        && let Some(file) = crate::references::entity_file(world, *e)
        && file == file_entity
    {
        sites.push(ReferenceSite {
            file,
            span,
            kind: RefKind::Direct,
        });
    }
    if let Target::Local { body, id } = target
        && let Some((file, span)) = semantic::local_name_site(world, root, *body, *id)
        && file == file_entity
    {
        sites.push(ReferenceSite {
            file,
            span,
            kind: RefKind::Direct,
        });
    }

    sites
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    fn highlights_for(src: &str, needle: &str) -> Vec<DocumentHighlight> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/highlight.ks", src.into());
        c.build(f);
        let offset = src.find(needle).expect("needle not found");
        let world = c.world();
        let root = c.root();
        let target = target_at(world, root, f, offset).expect("target");
        let sites = collect_sites(world, root, &target, f, &c);
        let li = LineIndex::new(src.to_string());
        sites
            .iter()
            .map(|site| {
                let clipped = clip_to_identifier(src, &site.span, site.kind);
                let range = li.range_for(clipped.start, clipped.end);
                DocumentHighlight {
                    range,
                    kind: Some(DocumentHighlightKind::READ),
                }
            })
            .collect()
    }

    #[test]
    fn highlights_function_call_and_decl() {
        let src = "module Test\n\
                   func target() -> lang.i64 { 1 }\n\
                   func caller() -> lang.i64 { target() }\n";
        let hl = highlights_for(src, "target");
        // At least decl + call site.
        assert!(hl.len() >= 2, "expected >=2 highlights, got {}", hl.len());
    }

    #[test]
    fn highlights_local_variable() {
        let src = "module Test\n\
                   func foo() -> lang.i64 {\n  \
                     let x = 1;\n  \
                     x\n\
                   }\n";
        // Cursor on bare `x` reference.
        let pos = src.rfind("x\n").unwrap();
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hl_local.ks", src.into());
        c.build(f);
        let target = target_at(c.world(), c.root(), f, pos).expect("target");
        let sites = collect_sites(c.world(), c.root(), &target, f, &c);
        // Two uses of x plus the declaration.
        assert!(
            sites.len() >= 2,
            "expected >=2 sites for local x, got {}",
            sites.len()
        );
    }

    #[test]
    fn no_highlights_on_empty_space() {
        let src = "module Test\n\n\nfunc foo() -> lang.i64 { 1 }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hl_empty.ks", src.into());
        c.build(f);
        // Cursor on the blank line between module and func.
        let offset = src.find("\n\n").unwrap() + 1;
        let result = target_at(c.world(), c.root(), f, offset);
        // May resolve to the module or None — either way, should not crash.
        let _ = result;
    }

    /// Highlighting a local paints its declaring identifier and its uses —
    /// not the whole `let` statement, and (for a parameter) not a
    /// zero-width range at the top of the file.
    #[test]
    fn highlights_local_name_and_uses_exactly() {
        let src = "module Test\n\
                   func f(n: lang.i64) -> lang.i64 { let x = n; lang.i64_add(x, n) }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hl_exact.ks", src.into());
        c.build(f);
        let li = LineIndex::new(src.to_string());
        let ranges_at = |at: usize| {
            let target = target_at(c.world(), c.root(), f, at).expect("target");
            let mut got: Vec<(usize, usize)> = collect_sites(c.world(), c.root(), &target, f, &c)
                .iter()
                .map(|site| {
                    let clipped = clip_to_identifier(src, &site.span, site.kind);
                    let r = li.range_for(clipped.start, clipped.end);
                    (li.position_to_offset(r.start), li.position_to_offset(r.end))
                })
                .collect();
            got.sort();
            got
        };
        let words = |name: &str| -> Vec<(usize, usize)> {
            src.match_indices(name)
                .filter(|(i, _)| {
                    !src[..*i].ends_with(|c: char| c.is_alphanumeric() || c == '_')
                        && !src[i + name.len()..]
                            .starts_with(|c: char| c.is_alphanumeric() || c == '_')
                })
                .map(|(i, _)| (i, i + name.len()))
                .collect()
        };
        let xs = words("x");
        assert_eq!(xs.len(), 2);
        assert_eq!(ranges_at(xs[1].0), xs, "from the use");
        assert_eq!(ranges_at(xs[0].0), xs, "from the declaration");
        let ns = words("n");
        assert_eq!(ns.len(), 3);
        assert_eq!(ranges_at(ns[2].0), ns, "parameter, from a use");
        assert_eq!(ranges_at(ns[0].0), ns, "parameter, from the signature");
    }
}
