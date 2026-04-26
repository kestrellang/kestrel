//! `textDocument/hover` — show docs and a signature for the symbol under
//! the cursor. Falls back to the inferred expression type when the cursor
//! isn't on a symbol (literals, locals, sub-expressions, etc).
//!
//! For function-like entities (functions, methods, initializers, setters,
//! subscripts) the signature is the source text from the decl span up to
//! (but not including) the body block. For types (structs, enums,
//! protocols, extensions, type aliases) it's the source text up to the
//! body block too — so the user sees the type's header + generics +
//! conformances without the noise of the body. Stored fields use the full
//! decl span (no body to trim). Doc comments are read from the
//! `Documentation` component attached during AST building.

use kestrel_ast_builder::{CstNode, DeclSpan, Documentation, FileId, FilePath, NodeKind};
use kestrel_syntax_tree::utils::get_name_span;
use kestrel_type_infer::result::ResolvedTy;
use kestrel_hecs::{Entity, World};
use kestrel_hir::body::{HirBody, HirExpr, HirExprId};
use kestrel_hir_lower::LowerBody;
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use kestrel_type_infer::InferBody;
use std::collections::HashMap;
use tower_lsp::lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind, Range};

use crate::semantic;
use crate::server::{url_to_path, SharedState};
use crate::ty_format::format_ty;

pub async fn handle(state: SharedState, params: HoverParams) -> Option<Hover> {
    let uri = params.text_document_position_params.text_document.uri;
    let pos = params.text_document_position_params.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, sources, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, s.sources.clone(), line_index)
    };
    let offset = line_index.position_to_offset(pos);

    let result = handle.with_compiler(stdlib, user, move |compiler, _by_path| -> Option<(String, Range)> {
        let file_entity = semantic::file_entity_for_path(compiler, &path)?;
        let world = compiler.world();
        let root = compiler.root();

        // Try entity-shaped hover first (functions, types, methods, etc).
        // Range is the cursor's expression / identifier span — NOT the
        // entity's whole DeclSpan, otherwise hovering anywhere on a name
        // would highlight the entire declaration that defines it.
        if let Some(md) = entity_hover_at(world, &sources, file_entity, offset, root) {
            let range = entity_hover_range(world, file_entity, offset, root, &line_index);
            return Some((md, range));
        }

        // Fall back to "inferred type of the expression at the cursor".
        let body_entity = semantic::body_entity_at(world, file_entity, offset)?;
        let ctx = world.query_context();
        let hir: HirBody = ctx.query(LowerBody {
            entity: body_entity,
            root,
        })?;
        let typed = ctx.query(InferBody {
            entity: body_entity,
            root,
        })?;
        let expr_id = semantic::hir_expr_at(&hir, offset)?;
        let ty = typed.expr_types.get(&expr_id)?;
        let rendered = format_ty(world, ty);
        let md = match &hir.exprs[expr_id] {
            HirExpr::Local(local_id, _) => {
                let local = &hir.locals[*local_id];
                let kw = if local.is_mut { "var" } else { "let" };
                let mut s = format!("```kestrel\n{} {}: {}\n```", kw, local.name, rendered);
                if let Some(link) = type_decl_link(world, &sources, ty) {
                    s.push_str(&format!("\n\n[Go to type definition]({link})"));
                }
                s
            },
            _ => format!("```kestrel\n{}\n```", rendered),
        };
        let span = semantic::hir_expr_span(&hir.exprs[expr_id]);
        let range = line_index.range_for(span.start, span.end);
        Some((md, range))
    })
    .await??;

    let (md, range) = result;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: md,
        }),
        range: Some(range),
    })
}

/// Locate the entity at the cursor and render its signature + docs as
/// markdown. Returns `None` when the cursor isn't on a symbol that resolves
/// to a declaration entity.
fn entity_hover_at(
    world: &World,
    sources: &HashMap<String, String>,
    file_entity: Entity,
    offset: usize,
    root: Entity,
) -> Option<String> {
    let entity = entity_at_cursor(world, file_entity, offset, root)?;
    render_entity(world, sources, entity)
}

/// Compute the LSP range to highlight for an entity hover. We want the
/// identifier under the cursor, not the entity's whole declaration. Order
/// of preference: HIR expression span at the cursor → enclosing decl's
/// name span (for cursor on the decl's own identifier).
fn entity_hover_range(
    world: &World,
    file_entity: Entity,
    offset: usize,
    root: Entity,
    line_index: &crate::position::LineIndex,
) -> Range {
    if let Some(body_entity) = semantic::body_entity_at(world, file_entity, offset) {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        }) {
            if let Some(expr_id) = semantic::hir_expr_at(&hir, offset) {
                let span = semantic::hir_expr_span(&hir.exprs[expr_id]);
                return line_index.range_for(span.start, span.end);
            }
        }
    }
    if let Some(decl) = semantic::enclosing_decl_at(world, file_entity, offset) {
        if let Some(cst) = world.get::<CstNode>(decl) {
            if let Some(decl_span) = world.get::<DeclSpan>(decl) {
                if let Some(name_span) = get_name_span(&cst.0, decl_span.0.file_id) {
                    return line_index.range_for(name_span.start, name_span.end);
                }
                return line_index.range_for(decl_span.0.start, decl_span.0.end);
            }
        }
    }
    // Last resort: zero-length range at the cursor.
    let pos = line_index.offset_to_position(offset);
    Range { start: pos, end: pos }
}

/// What entity does the cursor refer to? Tries (in order):
///   1. The HIR expression at the cursor (Def, OverloadSet[0], Field /
///      MethodCall / Call / ImplicitMember / ProtocolCall via inference's
///      `resolutions`). Cursor on `foo` in `foo()` → the `foo` entity.
///   2. **Only when no HIR expression covers the cursor** — the smallest
///      enclosing declaration whose `DeclSpan` covers the cursor. This
///      catches "cursor on `func foo` itself" (signature position has no
///      body expression). Crucially, when there IS an expression but it's
///      a local / literal / other non-entity value, we return `None` so
///      the caller falls back to the type-of-expression renderer instead
///      of incorrectly showing the enclosing function's signature.
fn entity_at_cursor(
    world: &World,
    file_entity: Entity,
    offset: usize,
    root: Entity,
) -> Option<Entity> {
    if let Some(body_entity) = semantic::body_entity_at(world, file_entity, offset) {
        let ctx = world.query_context();
        if let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        }) {
            if let Some(expr_id) = semantic::hir_expr_at(&hir, offset) {
                return entity_from_expr(&hir, body_entity, expr_id, &ctx, root);
            }
        }
    }
    semantic::enclosing_decl_at(world, file_entity, offset)
}

fn entity_from_expr(
    hir: &HirBody,
    body: Entity,
    expr_id: HirExprId,
    ctx: &kestrel_hecs::QueryContext<'_>,
    root: Entity,
) -> Option<Entity> {
    match &hir.exprs[expr_id] {
        HirExpr::Def(entity, _, _) => Some(*entity),
        HirExpr::OverloadSet { candidates, .. } => candidates.first().copied(),
        HirExpr::MethodCall { .. }
        | HirExpr::Field { .. }
        | HirExpr::Call { .. }
        | HirExpr::ImplicitMember { .. }
        | HirExpr::ProtocolCall { .. } => {
            let typed = ctx.query(InferBody { entity: body, root })?;
            typed.resolutions.get(&expr_id).copied()
        },
        _ => None,
    }
}

/// Build the markdown body for an entity hover. The fenced kestrel block is
/// the entity's signature (decl-span source, trimmed to the body block when
/// there is one); the prose under it is its leading doc comments.
fn render_entity(
    world: &World,
    sources: &HashMap<String, String>,
    entity: Entity,
) -> Option<String> {
    let kind = world.get::<NodeKind>(entity)?;
    if !is_renderable(kind) {
        return None;
    }
    let cst = world.get::<CstNode>(entity)?;
    let decl_span = world.get::<DeclSpan>(entity)?.0.clone();
    let file_path = entity_file_path(world, entity)?;
    let source = sources.get(&file_path)?;

    let signature = signature_text(source, &cst.0, &decl_span);
    let docs = world
        .get::<Documentation>(entity)
        .map(|d| d.0.clone())
        .unwrap_or_default();

    let mut md = String::new();
    md.push_str("```kestrel\n");
    md.push_str(signature.trim());
    md.push_str("\n```");
    if !docs.is_empty() {
        md.push_str("\n\n");
        md.push_str(&docs);
    }

    Some(md)
}

fn is_renderable(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Function
            | NodeKind::Initializer
            | NodeKind::Setter
            | NodeKind::Subscript
            | NodeKind::Field
            | NodeKind::Struct
            | NodeKind::Enum
            | NodeKind::EnumCase
            | NodeKind::Protocol
            | NodeKind::Extension
            | NodeKind::TypeAlias
    )
}

/// Slice the source from the declaration's start to the start of its body
/// block (or to the end of the decl span when there is no body, e.g.
/// stored fields, type aliases). Trims trailing semicolons / whitespace
/// for a clean header.
fn signature_text(source: &str, cst: &SyntaxNode, decl_span: &kestrel_span::Span) -> String {
    let body_start = first_body_block_offset(cst).unwrap_or(decl_span.end);
    let end = body_start.min(decl_span.end);
    let raw = source.get(decl_span.start..end).unwrap_or("");
    raw.trim_end_matches([';', ' ', '\t', '\n', '\r']).to_string()
}

/// Find the byte offset where the declaration's body block (or computed
/// property accessors) begins, if any.
fn first_body_block_offset(cst: &SyntaxNode) -> Option<usize> {
    for child in cst.children() {
        if matches!(
            child.kind(),
            SyntaxKind::FunctionBody
                | SyntaxKind::StructBody
                | SyntaxKind::EnumBody
                | SyntaxKind::ProtocolBody
                | SyntaxKind::ExtensionBody
                | SyntaxKind::SubscriptBody
                | SyntaxKind::PropertyAccessors
                | SyntaxKind::CodeBlock
        ) {
            return Some(child.text_range().start().into());
        }
    }
    None
}

/// Find the on-disk path of the file that contains `entity`. Decl entities
/// carry a `FileId(file_entity)` pointing at the file entity, which itself
/// owns the `FilePath` — `FilePath` is NOT propagated to children, so we
/// resolve via `FileId` rather than walking parents.
fn entity_file_path(world: &World, entity: Entity) -> Option<String> {
    if let Some(p) = world.get::<FilePath>(entity) {
        return Some(p.0.clone());
    }
    let fid = world.get::<FileId>(entity)?;
    world.get::<FilePath>(fid.0).map(|p| p.0.clone())
}

/// Build a `file://` URL pointing at the head entity's declaration site for
/// a Named type. Returns `None` for tuple/function/error/Self/Param types,
/// or for Named types whose head entity is intrinsic (no `FilePath`,
/// e.g. `lang.i64`).
fn type_decl_link(
    world: &World,
    sources: &HashMap<String, String>,
    ty: &ResolvedTy,
) -> Option<String> {
    let entity = match ty {
        ResolvedTy::Named { entity, .. } => *entity,
        _ => return None,
    };
    let path = entity_file_path(world, entity)?;
    let span = world.get::<DeclSpan>(entity)?.0.clone();
    // Resolve the line + column for the decl's start so the editor opens
    // at the right place.
    let source = sources.get(&path)?;
    let li = crate::position::LineIndex::new(source.clone());
    let pos = li.offset_to_position(span.start);
    // VS Code's hover renders `file://` URIs as clickable links that open
    // the file at the given line.
    Some(format!(
        "file://{path}#L{}",
        pos.line + 1
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    use kestrel_compiler::Compiler;
    use std::collections::HashMap;

    fn entity_hover_for(src: &str, needle: &str) -> Option<String> {
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hover_test.ks", src.into());
        c.build(f);
        let mut sources = HashMap::new();
        sources.insert("/tmp/hover_test.ks".to_string(), src.to_string());
        let offset = src.find(needle).expect("needle not in source");
        entity_hover_at(c.world(), &sources, f, offset, c.root())    }

    #[test]
    fn entity_hover_renders_function_signature_and_doc() {
        let src = "module Test\n\
                   /// Adds one to its argument.\n\
                   func bump(x: lang.i64) -> lang.i64 { x + 1 }\n\
                   func caller() -> lang.i64 { bump(1) }\n";
        let pos = src.rfind("bump").unwrap();
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hover_test.ks", src.into());
        c.build(f);
        let mut sources = HashMap::new();
        sources.insert("/tmp/hover_test.ks".to_string(), src.to_string());

        let md = entity_hover_at(c.world(), &sources, f, pos, c.root())
                        .expect("entity hover");
        assert!(md.contains("func bump(x: lang.i64) -> lang.i64"), "{md}");
        assert!(md.contains("Adds one to its argument."), "{md}");
    }

    #[test]
    fn entity_hover_renders_struct_signature() {
        let src = "module Test\n\
                   /// A 2D point.\n\
                   struct Point { var x: lang.i64; var y: lang.i64; }\n\
                   func at() -> Point { Point(x: 1, y: 2) }\n";
        // Cursor on `Point` inside `func at() -> Point {}`.
        let pos = src.find("-> Point").unwrap() + "-> ".len();
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hover_struct.ks", src.into());
        c.build(f);
        let mut sources = HashMap::new();
        sources.insert("/tmp/hover_struct.ks".to_string(), src.to_string());
        let _ = (pos, &sources, &c, f);
        // Type-position hovers aren't yet supported (see follow-up note).
        // For now, verify cursor on the struct's declaration name renders.
        let decl_pos = src.find("struct Point").unwrap() + "struct ".len();
        let md = entity_hover_at(c.world(), &sources, f, decl_pos, c.root())
                        .expect("entity hover");
        assert!(md.contains("struct Point"), "{md}");
        assert!(md.contains("A 2D point."), "{md}");
        // Struct body should be trimmed.
        assert!(!md.contains("var x:"), "body should be trimmed: {md}");
    }

    #[test]
    fn entity_hover_at_decl_identifier() {
        let src = "module Test\n\
                   /// Greets you.\n\
                   func greet() -> lang.i64 { 0 }\n";
        let md = entity_hover_for(src, "greet").expect("hover");
        assert!(md.contains("func greet()"), "{md}");
        assert!(md.contains("Greets you."), "{md}");
    }

    #[test]
    fn entity_hover_suppressed_for_local_variable() {
        // Cursor on a local should NOT render the enclosing function's
        // signature. It should return None so the caller falls back to
        // the inferred-type renderer.
        let src = "module Test\n\
                   func foo() -> lang.i64 {\n  \
                     let x = 42;\n  \
                     x\n\
                   }\n";
        // Cursor on the bare `x` reference (last occurrence).
        let pos = src.rfind("x").unwrap();
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/hover_local.ks", src.into());
        c.build(f);
        let mut sources = HashMap::new();
        sources.insert("/tmp/hover_local.ks".to_string(), src.to_string());
        let result = entity_hover_at(c.world(), &sources, f, pos, c.root());
        assert!(result.is_none(), "entity hover fired on local: {result:?}");
    }

    #[test]
    fn type_decl_link_returns_file_uri_for_named_type() {
        let src = "module Test\nstruct Point {}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/link.ks", src.into());
        c.build(f);
        let mut sources = HashMap::new();
        sources.insert("/tmp/link.ks".to_string(), src.to_string());

        use kestrel_ast_builder::{FileId as F, Name};
        let point = c
            .world()
            .iter_component::<Name>()
            .find(|(e, n)| n.0 == "Point" && c.world().get::<F>(*e).map(|f2| f2.0) == Some(f))
            .map(|(e, _)| e)
            .expect("Point entity");
        let ty = ResolvedTy::Named {
            entity: point,
            args: vec![],
        };
        let link = type_decl_link(c.world(), &sources, &ty).expect("link");
        assert!(link.starts_with("file:///tmp/link.ks#L"), "{link}");
    }

    #[test]
    fn entity_hover_renders_doc_with_visibility() {
        // Doc comments placed before `public` end up inside the Visibility
        // node, not as direct children of the decl.
        let src = "module Test\n\
                   /// Public-facing message.\n\
                   public func greet() -> lang.i64 { 0 }\n";
        let md = entity_hover_for(src, "greet").expect("hover");
        assert!(md.contains("public func greet()"), "{md}");
        assert!(md.contains("Public-facing message."), "{md}");
    }
}
