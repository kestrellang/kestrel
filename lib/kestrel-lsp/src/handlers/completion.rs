//! `textDocument/completion`.
//!
//! Two modes, picked from the byte before the cursor:
//!
//! * **Member completion** (`receiver.|`): resolve the receiver to a type,
//!   list members of that type plus its extensions and protocol conformances.
//!   The receiver detection is text-based for the M3 first cut — only a
//!   bare identifier (`foo.`) is supported. Method-chains and parenthesised
//!   receivers will use a CST walk in a follow-up.
//!
//! * **Scope completion** (bare prefix): walk the `ScopeFor` chain from the
//!   enclosing declaration up to the module, plus locals from the enclosing
//!   `HirBody`. Filter by the identifier prefix at the cursor.

use std::collections::{HashMap, HashSet};

use kestrel_ast_builder::{Body, Callable, Name, NodeKind, TypeParams};
use kestrel_hecs::{Entity, QueryContext, World};
use kestrel_hir::body::HirBody;
use kestrel_hir_lower::LowerBody;
use kestrel_name_res::{NameResolution, ResolveName, Scope, ScopeFor};
use kestrel_type_infer::result::ResolvedTy;
use kestrel_type_infer::InferBody;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
    InsertTextFormat,
};

use crate::semantic;
use crate::server::{rebuild_compiler, url_to_path, SharedState};
use crate::syntax;

pub async fn handle(
    state: SharedState,
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let uri = params.text_document_position.text_document.uri;
    let pos = params.text_document_position.position;
    let path = url_to_path(&uri);

    let (sources, line_index) = {
        let s = state.lock().await;
        let line_index = s.docs.get(&uri).map(|d| d.line_index.clone())?;
        (s.sources.clone(), line_index)
    };
    let offset = line_index.position_to_offset(pos);
    let text = line_index.text().to_string();

    let items = tokio::task::spawn_blocking(move || -> Vec<CompletionItem> {
        let (compiler, _) = rebuild_compiler(&sources);
        let Some(file_entity) = semantic::file_entity_for_path(&compiler, &path) else {
            return vec![];
        };
        let world = compiler.world();
        let root = compiler.root();
        let ctx = world.query_context();
        let enclosing = semantic::enclosing_decl_at(world, file_entity, offset);

        if syntax::is_after_dot(&text, offset) {
            member_completion(&ctx, world, root, &text, offset, file_entity, enclosing)
        } else {
            let prefix = syntax::identifier_prefix(&text, offset);
            let mut items = scope_completion(
                &ctx, world, root, prefix, enclosing, offset, file_entity,
            );
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
        }
    })
    .await
    .ok()?;

    Some(CompletionResponse::Array(items))
}

// ===== Member completion =====

fn member_completion(
    ctx: &QueryContext<'_>,
    world: &World,
    root: Entity,
    text: &str,
    offset: usize,
    file_entity: Entity,
    enclosing: Option<Entity>,
) -> Vec<CompletionItem> {
    let Some(receiver_name) = syntax::dot_receiver_identifier(text, offset) else {
        return vec![];
    };
    // Prefer the body entity at offset (works through partial parses); fall
    // back to the enclosing decl, then root for free-name lookups.
    // Prefer the body entity at offset (works through partial parses); fall
    // back to the enclosing decl, then root for free-name lookups.
    let context = semantic::body_entity_at(world, file_entity, offset)
        .or(enclosing)
        .unwrap_or(root);

    let receiver_ty = receiver_type(ctx, world, root, file_entity, receiver_name, context);

    let Some(ty) = receiver_ty else {
        return vec![];
    };
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    push_members_for_type(ctx, world, root, &ty, &mut out, &mut seen);
    out
}

/// Resolve a bare identifier in the current scope to a type.
///
/// Tries: (1) a local in the enclosing body, (2) a value entity via
/// `ResolveName`, (3) a type entity via `ResolveName` (for `TypeName.` —
/// static member access).
fn receiver_type(
    ctx: &QueryContext<'_>,
    world: &World,
    root: Entity,
    file_entity: Entity,
    name: &str,
    context: Entity,
) -> Option<ResolvedTy> {
    // Local in body?
    if let Some(body_entity) = body_entity_containing(world, file_entity, context)
        && let (Some(typed), Some(hir)) = (
            ctx.query(InferBody { entity: body_entity, root }),
            ctx.query(LowerBody { entity: body_entity, root }),
        )
    {
        let hir: HirBody = hir;
        for (id, local) in hir.locals.iter() {
            if local.name == name
                && let Some(ty) = typed.local_types.get(&id)
            {
                return Some(ty.clone());
            }
        }
    }

    // Free name → entity. Prefer the value resolution if present, otherwise
    // fall back to the type itself (so `String.` static completion works).
    let res = ctx.query(ResolveName {
        name: name.to_string(),
        context,
        root,
    });
    let entity = match res {
        NameResolution::Found(es) if !es.is_empty() => es[0],
        _ => return None,
    };

    // If the entity is itself a type, the receiver is the type — return a
    // `Named` of the type with no args. Members will include statics.
    let kind = world.get::<NodeKind>(entity)?;
    match kind {
        NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::TypeAlias => {
            Some(ResolvedTy::Named { entity, args: vec![] })
        },
        // Otherwise it's a value — try to read its type-annotation entity to
        // produce a `Named` ty. This is heuristic; fully accurate typing
        // requires a body context.
        _ => entity_value_type(ctx, world, entity),
    }
}

fn body_entity_containing(world: &World, file_entity: Entity, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<Body>(entity).is_some()
            && world
                .get::<kestrel_ast_builder::FileId>(entity)
                .map(|f| f.0 == file_entity)
                .unwrap_or(false)
        {
            return Some(entity);
        }
        entity = world.parent_of(entity)?;
    }
}


/// Best-effort: read a value entity's type. For globals / fields / functions
/// we'd need to lower / infer the type annotation. M3 leaves this as a stub
/// (returns None) — most member completion the user reaches in practice
/// will be on locals, which the body-level path above handles.
fn entity_value_type(_ctx: &QueryContext<'_>, _world: &World, _entity: Entity) -> Option<ResolvedTy> {
    None
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
    let exts = ctx.query(kestrel_name_res::ExtensionsFor { target: entity, root });
    for ext in exts {
        for &child in world.children_of(ext) {
            push_member_entity(world, child, out, seen);
        }
    }

    // Protocol conformances aren't expanded here; M3 keeps it simple.
    // Methods provided by extensions are already covered above.
    if matches!(world.get::<NodeKind>(entity), Some(&NodeKind::Protocol)) {
        let members = ctx.query(kestrel_name_res::ProtocolMembers { protocol: entity, root });
        for member in members {
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
    let Some(name) = world.get::<Name>(entity) else { return };
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
        && let Some(hir) = ctx.query(LowerBody { entity: body_entity, root })
    {
        for (_id, local) in hir.locals.iter() {
            if local.span.start > offset {
                continue; // not yet in scope
            }
            if !local.name.starts_with(prefix) {
                continue;
            }
            items.entry(local.name.clone()).or_insert_with(|| CompletionItem {
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
        let scope: std::sync::Arc<Scope> =
            ctx.query(ScopeFor { entity: scope_entity, root });
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
    let Some(name) = world.get::<Name>(entity) else { return };
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
