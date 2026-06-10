//! `textDocument/completion`.
//!
//! Two modes, dispatched by what the HIR says is at the cursor:
//!
//! * **Member completion** (`receiver.|`): the parser-recovery work
//!   (Phases 1–3 of the missing-token effort) makes `foo.` parse to a
//!   well-formed `HirExpr::Field { base, name: HirName::Missing }`, so we
//!   can look up the smallest `Field` covering the offset and ask
//!   inference for the *base* expression's type. Works for arbitrary
//!   receivers (locals, method-chains, parenthesised) — anything the
//!   parser can produce a base expression for.
//!
//! * **Scope completion** (bare prefix): walk the `ScopeFor` chain from
//!   the enclosing declaration up to the module, plus locals from the
//!   enclosing `HirBody`. Filter by the identifier prefix at the cursor.

use std::collections::{HashMap, HashSet};

use kestrel_ast_builder::{Body, Callable, FileId, Name, NodeKind, TypeParams};
use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_hir::body::{HirBody, HirExpr};
use kestrel_hir_lower::LowerBody;
use kestrel_name_res::{Scope, ScopeFor};
use kestrel_type_infer::InferBody;
use kestrel_type_infer::result::{ResolvedTy, TypedBody};
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, InsertTextFormat,
};

use crate::semantic;
use crate::server::{SharedState, url_to_path};
use crate::syntax;

pub async fn handle(state: SharedState, params: CompletionParams) -> Option<CompletionResponse> {
    let uri = params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let path = url_to_path(&uri);

    let (handle, stdlib, user, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        let (stdlib, user) = s.partition_sources();
        (s.compiler_handle.clone(), stdlib, user, line_index)
    };
    let offset = line_index.position_to_offset(pos);
    let text = line_index.text().to_string();

    let items = handle
        .with_compiler(
            stdlib,
            user,
            move |compiler, _by_path| -> Vec<CompletionItem> {
                let Some(file_entity) = semantic::file_entity_for_path(compiler, &path) else {
                    return vec![];
                };
                let world = compiler.world();
                let root = compiler.root();
                let ctx = world.query_context();
                let enclosing = semantic::enclosing_decl_at(world, file_entity, offset);
                let prefix = syntax::identifier_prefix(&text, offset);

                // Member completion fires when the smallest HIR expression covering
                // the cursor is a Field — that's exactly the `foo.|` / `foo.bar|`
                // shape, surfaced through the parser-recovery work. Falls back to
                // scope completion when there's no Field at the cursor (no body, no
                // dot, etc.).
                if let Some(items) =
                    member_completion(&ctx, world, root, file_entity, offset, prefix)
                {
                    return items;
                }

                let mut items =
                    scope_completion(&ctx, world, root, prefix, enclosing, offset, file_entity);
                // At module / file top level, also offer keyword snippets. We
                // detect "top level" as: enclosing is the file's module entity.
                let is_top_level = enclosing
                    .map(|e| world.get::<NodeKind>(e) == Some(&NodeKind::Module))
                    .unwrap_or(true);
                if is_top_level {
                    for snip in top_level_snippets() {
                        if snip.label.starts_with(prefix) {
                            items.push(snip);
                        }
                    }
                }
                items
            },
        )
        .await?;

    Some(CompletionResponse::Array(items))
}

// ===== Member completion =====

/// Returns `Some(items)` when the cursor is at a member-access position
/// (the smallest HIR expression covering it is a `Field`). `None` means
/// the caller should fall through to scope completion.
///
/// `prefix` is the partial identifier already typed; we filter the
/// member list against it for client-side parity with scope completion.
fn member_completion(
    ctx: &QueryContext<'_>,
    world: &World,
    root: Entity,
    file_entity: Entity,
    offset: usize,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let body_entity = semantic::body_entity_at(world, file_entity, offset)?;
    let hir = ctx.query(LowerBody {
        entity: body_entity,
        root,
    })?;
    let typed = ctx.query(InferBody {
        entity: body_entity,
        root,
    })?;

    // Two paths: the HIR-Field path handles `foo.|` cleanly when the
    // parser produced a Field with HirName::Missing. The CST-Dot path
    // handles `foo.|\n    bar.x` where the parser greedily fuses the
    // newline-separated identifier into the path (so the HIR sees one
    // long Field chain whose spans don't cover the cursor).
    // Try the HIR-Field path first; treat ResolvedTy::Error like a miss so
    // we fall back to the CST-Dot locator. Error commonly arises when the
    // parser greedily fused the trailing dot into a longer expression
    // (e.g. `g().` followed by another statement → fused into a Call), and
    // the CST locator can still recover the real receiver.
    let receiver_ty = receiver_type_at_dot(&hir, &typed, world, root, ctx, offset)
        .filter(|ty| !matches!(ty, ResolvedTy::Error))
        .or_else(|| receiver_type_via_cst_dot(world, &hir, &typed, body_entity, offset))?;

    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    push_members_for_type(ctx, world, root, &receiver_ty, &mut out, &mut seen);
    if !prefix.is_empty() {
        out.retain(|it| it.label.starts_with(prefix));
    }
    Some(out)
}

/// Locate the smallest `HirExpr::Field` covering `offset` and return the
/// resolved type of its base expression. Returns `None` when the cursor
/// isn't at a member-access position (no Field in scope) or the base
/// type wasn't inferable (poisoned to `Error`, no entry in `expr_types`).
fn receiver_type_at_dot(
    hir: &HirBody,
    typed: &TypedBody,
    world: &World,
    root: Entity,
    ctx: &QueryContext<'_>,
    offset: usize,
) -> Option<ResolvedTy> {
    let field_id = smallest_field_at(hir, offset)?;
    let HirExpr::Field { base, .. } = &hir.exprs[field_id] else {
        return None;
    };
    let base_ty = typed.expr_types.get(base)?;

    // For static-member completion (`Foo.|`), the base expression is a
    // `Def(entity)` referring to a type — `expr_types` records that as a
    // first-class type value (e.g. the type itself, not an instance). We
    // unwrap that into a `Named { entity, args: [] }` so push_members
    // sees the type and lists its statics.
    if let HirExpr::Def(entity, _, _) = &hir.exprs[*base]
        && matches!(
            world.get::<NodeKind>(*entity),
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::TypeAlias)
        )
    {
        let _ = (root, ctx);
        return Some(ResolvedTy::Named {
            entity: *entity,
            args: vec![],
        });
    }

    Some(base_ty.clone())
}

/// CST-based fallback for `receiver.|` when the parser greedily fused
/// the newline after the dot into a longer path expression. We find the
/// `.` token at or just before `offset`, take the preceding token range
/// (the path element to its left — `arr` in `arr.|`), and pull the
/// matching HIR expression out by exact-end-of-span.
fn receiver_type_via_cst_dot(
    world: &World,
    hir: &HirBody,
    typed: &TypedBody,
    body_entity: Entity,
    offset: usize,
) -> Option<ResolvedTy> {
    use kestrel_ast_builder::Valued;
    use kestrel_syntax_tree::SyntaxKind;
    use rowan::TextSize;

    let cst = &world.get::<Valued>(body_entity)?.0;
    let pos = TextSize::from(offset as u32);
    // Find the smallest Dot token whose end == cursor (or whose range
    // contains the cursor).
    let mut dot_end: Option<usize> = None;
    for tok in cst.descendants_with_tokens().filter_map(|e| e.into_token()) {
        if tok.kind() != SyntaxKind::Dot {
            continue;
        }
        let r = tok.text_range();
        if r.end() == pos || (r.start() <= pos && pos <= r.end()) {
            dot_end = Some(r.end().into());
            break;
        }
    }
    let dot_end = dot_end?;
    // Receiver = HIR expression whose span ends just before the dot.
    // We walk all expressions and find the one whose end == dot_start.
    let dot_start = dot_end - 1;
    let mut best: Option<(kestrel_hir::body::HirExprId, usize)> = None;
    for (id, expr) in hir.exprs.iter() {
        let s = semantic::hir_expr_span(expr);
        if s.end == dot_start {
            let len = s.end - s.start;
            if best.map(|(_, l)| len > l).unwrap_or(true) {
                best = Some((id, len));
            }
        }
    }
    let (recv_id, _) = best?;
    typed.expr_types.get(&recv_id).cloned()
}

/// Find the smallest `HirExpr::Field` whose span covers `offset`. Walks
/// every Field in the body — bodies are small enough that this is cheap;
/// avoids needing a parent-pointer in the HIR.
fn smallest_field_at(hir: &HirBody, offset: usize) -> Option<kestrel_hir::body::HirExprId> {
    let mut best: Option<(kestrel_hir::body::HirExprId, usize)> = None;
    for (id, expr) in hir.exprs.iter() {
        let HirExpr::Field { span, .. } = expr else {
            continue;
        };
        if span.start <= offset && offset <= span.end {
            let len = span.end - span.start;
            if best.map(|(_, l)| len < l).unwrap_or(true) {
                best = Some((id, len));
            }
        }
    }
    best.map(|(id, _)| id)
}

fn body_entity_containing(
    world: &World,
    file_entity: Entity,
    mut entity: Entity,
) -> Option<Entity> {
    loop {
        if world.get::<Body>(entity).is_some()
            && world
                .get::<FileId>(entity)
                .map(|f| f.0 == file_entity)
                .unwrap_or(false)
        {
            return Some(entity);
        }
        entity = world.parent_of(entity)?;
    }
}

fn push_members_for_type(
    ctx: &QueryContext<'_>,
    world: &World,
    root: Entity,
    ty: &ResolvedTy,
    out: &mut Vec<CompletionItem>,
    seen: &mut HashSet<String>,
) {
    let entity = match ty {
        ResolvedTy::Named { entity, .. } => *entity,
        _ => return,
    };

    // Direct children (fields, methods, init) of the nominal type.
    for &child in world.children_of(entity) {
        push_member_entity(world, child, out, seen);
    }

    // Extensions targeting this type, then their children.
    let exts = ctx.query(kestrel_name_res::ExtensionsFor {
        target: entity,
        root,
    });
    for ext in exts {
        for &child in world.children_of(ext) {
            push_member_entity(world, child, out, seen);
        }
    }

    // Protocol conformances aren't expanded here; M3 keeps it simple.
    // Methods provided by extensions are already covered above.
    if matches!(world.get::<NodeKind>(entity), Some(&NodeKind::Protocol)) {
        let members = ctx.query(kestrel_name_res::ProtocolMembers {
            protocol: entity,
            root,
        });
        for member in members.iter() {
            push_member_entity(world, member.entity, out, seen);
        }
    }
}

fn push_member_entity(
    world: &World,
    entity: Entity,
    out: &mut Vec<CompletionItem>,
    seen: &mut HashSet<String>,
) {
    let Some(name) = world.get::<Name>(entity) else {
        return;
    };
    let kind = world.get::<NodeKind>(entity).cloned();
    if !seen.insert(format!("{}::{:?}", name.0, kind)) {
        return;
    }
    let item_kind = match kind {
        Some(NodeKind::Function) => CompletionItemKind::METHOD,
        Some(NodeKind::Field) => CompletionItemKind::FIELD,
        Some(NodeKind::Initializer) => CompletionItemKind::CONSTRUCTOR,
        Some(NodeKind::TypeAlias) => CompletionItemKind::TYPE_PARAMETER,
        Some(NodeKind::EnumCase) => CompletionItemKind::ENUM_MEMBER,
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => CompletionItemKind::CLASS,
        _ => return,
    };
    let detail = signature_detail(world, entity);
    out.push(CompletionItem {
        label: name.0.clone(),
        kind: Some(item_kind),
        detail,
        ..Default::default()
    });
}

/// Build a one-line signature for `detail` — function arity, field type, etc.
/// Best-effort: skips heavy lifting and is empty when we can't tell cheaply.
fn signature_detail(world: &World, entity: Entity) -> Option<String> {
    if let Some(callable) = world.get::<Callable>(entity) {
        let params = callable
            .params
            .iter()
            .map(|p| {
                if let Some(label) = &p.label {
                    format!("{}: {}", label, p.name)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Some(format!("({})", params));
    }
    None
}

// ===== Scope completion =====

fn scope_completion(
    ctx: &QueryContext<'_>,
    world: &World,
    root: Entity,
    prefix: &str,
    enclosing: Option<Entity>,
    offset: usize,
    file_entity: Entity,
) -> Vec<CompletionItem> {
    let mut items: HashMap<String, CompletionItem> = HashMap::new();

    // 1. Locals in the enclosing body that are in scope at `offset`.
    if let Some(body_entity) = enclosing.and_then(|e| body_entity_containing(world, file_entity, e))
        && let Some(hir) = ctx.query(LowerBody {
            entity: body_entity,
            root,
        })
    {
        for (_id, local) in hir.locals.iter() {
            if local.span.start > offset {
                continue; // not yet in scope
            }
            if !local.name.starts_with(prefix) {
                continue;
            }
            items
                .entry(local.name.clone())
                .or_insert_with(|| CompletionItem {
                    label: local.name.clone(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    ..Default::default()
                });
        }
    }

    // 2. Walk the ScopeFor chain from enclosing → root, collecting visible
    //    names. ScopeFor handles imports + auto-imports of std for us.
    let mut cursor = enclosing;
    while let Some(scope_entity) = cursor {
        let scope: std::sync::Arc<Scope> = ctx.query(ScopeFor {
            entity: scope_entity,
            root,
        });
        push_scope_names(world, &scope, prefix, &mut items);
        // Wildcard imports: walk their immediate children and offer names.
        for &source in &scope.wildcard_imports {
            for &child in world.children_of(source) {
                push_decl_name(world, child, prefix, &mut items);
            }
        }
        cursor = scope.parent;
    }

    // 3. Type parameters of any enclosing decl that has them (struct / func /
    //    extension): walk parent chain.
    let mut cursor = enclosing;
    while let Some(e) = cursor {
        if let Some(tps) = world.get::<TypeParams>(e) {
            for &tp in &tps.0 {
                push_decl_name(world, tp, prefix, &mut items);
            }
        }
        cursor = world.parent_of(e);
    }

    items.into_values().collect()
}

fn push_scope_names(
    world: &World,
    scope: &Scope,
    prefix: &str,
    out: &mut HashMap<String, CompletionItem>,
) {
    for (name, entities) in &scope.declarations {
        if !name.starts_with(prefix) {
            continue;
        }
        let entity = entities[0];
        push_decl_name(world, entity, prefix, out);
        let _ = name;
    }
    for (name, entities) in &scope.selective_imports {
        if !name.starts_with(prefix) {
            continue;
        }
        let entity = entities[0];
        push_decl_name(world, entity, prefix, out);
        let _ = name;
    }
}

fn push_decl_name(
    world: &World,
    entity: Entity,
    prefix: &str,
    out: &mut HashMap<String, CompletionItem>,
) {
    let Some(name) = world.get::<Name>(entity) else {
        return;
    };
    if !name.0.starts_with(prefix) || name.0 == "<root>" {
        return;
    }
    let kind = world.get::<NodeKind>(entity).cloned();
    let item_kind = match kind {
        Some(NodeKind::Function) => CompletionItemKind::FUNCTION,
        Some(NodeKind::Struct) | Some(NodeKind::Enum) => CompletionItemKind::CLASS,
        Some(NodeKind::Protocol) => CompletionItemKind::INTERFACE,
        Some(NodeKind::TypeAlias) => CompletionItemKind::TYPE_PARAMETER,
        Some(NodeKind::Module) => CompletionItemKind::MODULE,
        Some(NodeKind::TypeParameter) => CompletionItemKind::TYPE_PARAMETER,
        Some(NodeKind::EnumCase) => CompletionItemKind::ENUM_MEMBER,
        _ => CompletionItemKind::VARIABLE,
    };
    let detail = signature_detail(world, entity);
    out.entry(name.0.clone()).or_insert_with(|| CompletionItem {
        label: name.0.clone(),
        kind: Some(item_kind),
        detail,
        ..Default::default()
    });
}

// ===== Top-level snippets =====

/// Snippets that make sense at file / module top level — rendered as
/// `CompletionItem`s with snippet text. Currently injected unconditionally
/// when the cursor is at the file root; real "is at top level" detection
/// (by inspecting the CST around the cursor) is left to a follow-up.
pub fn top_level_snippets() -> Vec<CompletionItem> {
    fn snip(label: &str, body: &str) -> CompletionItem {
        CompletionItem {
            label: label.into(),
            kind: Some(CompletionItemKind::SNIPPET),
            insert_text: Some(body.into()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        }
    }
    vec![
        snip("module", "module ${1:Name}\n"),
        snip("import", "import ${1:Module}\n"),
        snip("func", "func ${1:name}(${2}) {\n\t$0\n}"),
        snip("struct", "struct ${1:Name} {\n\t$0\n}"),
        snip("protocol", "protocol ${1:Name} {\n\t$0\n}"),
        snip("extend", "extend ${1:Type}: ${2:Protocol} {\n\t$0\n}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;

    /// Phase 5 verification: `foo.|` (cursor right after the dot, no member
    /// typed) must produce member completions. Phases 1–3 made the parser
    /// recover the missing identifier as `HirName::Missing`, lowered the
    /// chain to a `HirExpr::Field` whose base is the receiver, and
    /// short-circuited inference to leave the base type intact. This test
    /// confirms the LSP picks up that base type and lists fields.
    #[test]
    fn member_completion_after_trailing_dot_lists_struct_fields() {
        let mut c = Compiler::new();
        let src = "module Test\n\
                   struct P { var x: lang.i64; var y: lang.i64 }\n\
                   func foo(p: P) { p. }\n";
        let f = c.set_source("/tmp/p.ks", src.into());
        c.build(f);

        // Cursor right after the dot in `p.`.
        let dot = src.rfind("p.").unwrap() + 2;
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();

        let items = member_completion(&ctx, world, root, f, dot, "")
            .expect("member completion should fire on `p.`");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("x") && labels.contains("y"),
            "expected fields `x` and `y` in {:?}",
            labels
        );
    }

    /// Regression: typing `let z = p` (no semicolon) inside a function body
    /// must not bail out of the entire FunctionDeclaration. Parser recovery
    /// should keep the FunctionBody / CodeBlock structure intact so the
    /// LSP's `enclosing_decl_at` lands on the Function (not the Module),
    /// preventing top-level snippets (`protocol`, `extend`) from leaking
    /// in. Companion fix: a single missing-semicolon should not corrupt the
    /// surrounding body.
    #[test]
    fn malformed_statement_does_not_kill_function_decl() {
        use kestrel_syntax_tree::SyntaxKind;
        let src = "module Demo\nstruct P { var x: lang.i64 }\nfunc foo(p: P) {\n    let z = p\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/repro.ks", src.into());
        c.build(f);
        let cst = c.parse(f).tree;
        let kinds: Vec<_> = cst.descendants().map(|n| n.kind()).collect();
        assert!(
            kinds.contains(&SyntaxKind::FunctionBody),
            "FunctionBody must survive a missing-semicolon recovery; got {kinds:?}"
        );
    }

    /// Recovery diagnostics: parser-synthesised closing tokens (`;`, `}`,
    /// `)`) must surface as `expected …` diagnostics so the editor still
    /// shows the missing-token squiggle. Walks the compiler's reported
    /// diagnostics for each scenario.
    #[test]
    fn missing_token_recovery_emits_diagnostics() {
        let cases: &[(&str, &str, &str)] = &[
            (
                "missing `;`",
                "module D\nfunc f() {\n    let z = 42\n}\n",
                "expected `;`",
            ),
            (
                "missing `}` (function body)",
                "module D\nfunc f() {\n    let z = 42;\n",
                "expected `}`",
            ),
            (
                "missing `)`",
                "module D\nfunc f() {\n    let z = (42;\n}\n",
                "expected `)`",
            ),
            (
                "missing `}` (closure)",
                "module D\nfunc f() {\n    let g = { 42\n}\n",
                "expected `}`",
            ),
        ];
        for (label, src, want) in cases {
            // Exact LSP path: rebuild_compiler builds via set_source +
            // compiler.build, then driver runs infer_all/analyze_all, then
            // compiler.diagnostics() collects everything.
            let mut c = Compiler::new();
            let f = c.set_source("/tmp/diag.ks", (*src).into());
            c.build(f);
            let driver = kestrel_compiler_driver::CompilerDriver::new(&c);
            let _ = driver.infer_all();
            let _ = driver.analyze_all(false);
            let diags = c.diagnostics();
            let msgs: Vec<String> = diags.iter().map(|d| d.message.clone()).collect();
            // Diagnostic must carry the sink's file_id, not the chumsky
            // span's default `0` — the LSP's file_id → URL map drops
            // any diagnostic whose label points at a file_id it doesn't know.
            assert!(
                diags
                    .iter()
                    .all(|d| d.labels.iter().all(|l| l.file_id == f.index())),
                "[{label}] expected diagnostic file_id == {} for {msgs:?}",
                f.index()
            );
            assert!(
                msgs.iter().any(|m| m.contains(want)),
                "[{label}] expected compiler diagnostic containing {want:?}; got {msgs:?}"
            );
        }
    }

    /// Regression: `arr.|` followed by `arr.chunks();` on the next line.
    /// The parser greedily fuses both into one path expression, so HIR
    /// Field spans don't cover the cursor. The CST-Dot fallback in
    /// `receiver_type_via_cst_dot` finds the receiver via the `.` token's
    /// left sibling instead of relying on `smallest_field_at`.
    #[test]
    fn member_completion_with_following_statement() {
        let src = "module Demo\nstruct P { var x: lang.i64; var y: lang.i64 }\nfunc f(p: P) {\n    p.\n    p.x;\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/i1a.ks", src.into());
        c.build(f);
        let cur = src.find("p.\n").unwrap() + 2;
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire on `p.` even with following stmt");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("x") && labels.contains("y"),
            "expected fields x, y; got {:?}",
            labels
        );
    }

    /// Companion: with the missing-semicolon parser recovery in place, the
    /// trailing-dot in `let z = p.` should still resolve to a Field over
    /// the `p` receiver, so member completion lists struct fields rather
    /// than falling through to scope/snippet completion.
    #[test]
    fn member_completion_after_dot_in_let_rhs() {
        let src = "module Demo\nstruct P { var x: lang.i64; var y: lang.i64 }\nfunc foo(p: P) {\n    let z = p.\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/letrhs.ks", src.into());
        c.build(f);
        let cur = src.find("p.").unwrap() + 2; // right after the dot
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire for `let z = p.|`");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("x") && labels.contains("y"),
            "expected fields x, y in {:?}",
            labels
        );
    }

    /// Method-chain receiver: `a.b.c.|` must complete on `c`'s type.
    #[test]
    fn member_completion_on_chained_receiver() {
        let src = "module T\nstruct C { var z: lang.i64 }\nstruct B { var c: C }\nstruct A { var b: B }\nfunc f(a: A) { a.b.c. }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/chain.ks", src.into());
        c.build(f);
        let cur = src.find("a.b.c.").unwrap() + "a.b.c.".len();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire on `a.b.c.`");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("z"),
            "expected field z on C; got {:?}",
            labels
        );
    }

    /// Call-receiver: `g().|` must complete on the return type of `g`.
    #[test]
    fn member_completion_on_call_receiver() {
        let src = "module T\nstruct R { var x: lang.i64 }\nfunc g() -> R { R(x: 1) }\nfunc h() { g(). }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/call.ks", src.into());
        c.build(f);
        let cur = src.find("g().").unwrap() + "g().".len();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire on `g().`");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("x"),
            "expected field x on R; got {:?}",
            labels
        );
    }

    /// Parenthesised receiver: `(a.b).|` must complete on `b`'s type.
    #[test]
    fn member_completion_on_paren_receiver() {
        let src = "module T\nstruct C { var z: lang.i64 }\nstruct B { var c: C }\nstruct A { var b: B }\nfunc f(a: A) { (a.b). }\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/paren.ks", src.into());
        c.build(f);
        let cur = src.find("(a.b).").unwrap() + "(a.b).".len();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire on `(a.b).`");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("c"),
            "expected field c on B; got {:?}",
            labels
        );
    }

    /// Method-chain receiver followed by another statement: parser greedily
    /// fuses the chain into the next line. CST-Dot fallback must locate the
    /// trailing `.` and find the `a.b.c` receiver to its left.
    #[test]
    fn member_completion_on_chain_with_following_stmt() {
        let src = "module T\nstruct C { var z: lang.i64 }\nstruct B { var c: C }\nstruct A { var b: B }\nfunc f(a: A) {\n    a.b.c.\n    a.b.c.z;\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/chain_follow.ks", src.into());
        c.build(f);
        let cur = src.find("a.b.c.\n").unwrap() + "a.b.c.".len();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire on `a.b.c.|` with following stmt");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("z"),
            "expected field z on C; got {:?}",
            labels
        );
    }

    /// Call receiver followed by another statement.
    #[test]
    fn member_completion_on_call_with_following_stmt() {
        let src = "module T\nstruct R { var x: lang.i64 }\nfunc g() -> R { R(x: 1) }\nfunc h() {\n    g().\n    g().x;\n}\n";
        let mut c = Compiler::new();
        let f = c.set_source("/tmp/call_follow.ks", src.into());
        c.build(f);
        let cur = src.find("g().\n").unwrap() + "g().".len();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cur, "")
            .expect("member completion should fire on `g().|` with following stmt");
        let labels: HashSet<String> = items.iter().map(|i| i.label.clone()).collect();
        assert!(
            labels.contains("x"),
            "expected field x on R; got {:?}",
            labels
        );
    }

    /// Sanity: outside of a member-access position (cursor in a bare
    /// identifier), `member_completion` must return `None` so the caller
    /// falls through to scope completion.
    #[test]
    fn member_completion_returns_none_for_bare_identifier() {
        let mut c = Compiler::new();
        let src = "module Test\nfunc foo() { let x = 42; x }\n";
        let f = c.set_source("/tmp/q.ks", src.into());
        c.build(f);
        let cursor = src.rfind("x").unwrap();
        let world = c.world();
        let root = c.root();
        let ctx = world.query_context();
        let items = member_completion(&ctx, world, root, f, cursor, "");
        assert!(items.is_none(), "should fall through to scope: {:?}", items);
    }
}
