//! Walks a compiled Kestrel `World` and extracts public declarations into
//! a JSON-serializable shape suitable for a rustdoc-style site.
//!
//! Signatures are built directly from the AST components (`Callable`,
//! `TypeAnnotation`, `TypeParams`, `Conformances`, accessor children),
//! never by slicing source — that side-steps parser quirks where the
//! decl/body span boundary lands a character or two off.

#[cfg(test)]
mod repro_test;

pub mod markdown;
pub mod signature;

use std::collections::HashMap;

use kestrel_ast_builder::{
    AstType, Conformances, DeclSpan, Documentation, ExtensionTarget, FileId, FilePath, Name,
    NodeKind, Vis,
};
use kestrel_hecs::{Entity, World};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ModuleIndex {
    pub modules: Vec<ModuleSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleSummary {
    pub path: String,
    pub name: String,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModulePage {
    pub path: String,
    pub name: String,
    pub submodules: Vec<String>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub kind: String,
    pub name: String,
    /// Anchor slug (`function-bump`, `struct-Array`).
    pub anchor: String,
    pub signature: String,
    pub doc: String,
    pub source_path: Option<String>,

    /// For container types (struct, enum, protocol, extension), members
    /// are split into the type's own declarations plus one group per
    /// protocol it conforms to. Empty for leaf items.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_groups: Vec<MemberGroup>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberGroup {
    /// `"direct"` for items declared on the type itself, `"protocol"`
    /// for items pulled from a protocol the type conforms to.
    pub kind: String,
    /// Display label (e.g. the protocol's short name). `None` for direct.
    pub label: Option<String>,
    /// Dotted module path the protocol lives at, when known. Used by
    /// the frontend to link "Cloneable" → the protocol's page.
    pub source_path: Option<String>,
    pub members: Vec<Item>,
}

/// Extract a doc-site index + per-module pages from a populated world.
pub fn extract(world: &World, root: Entity) -> (ModuleIndex, Vec<ModulePage>) {
    let protocol_index = build_protocol_index(world);
    let extensions_by_target = build_extension_index(world);

    let mut pages = Vec::new();
    let mut summaries = Vec::new();

    let mut roots: Vec<Entity> = world
        .children_of(root)
        .iter()
        .copied()
        .filter(|&e| matches!(world.get::<NodeKind>(e), Some(NodeKind::Module)))
        .filter(|&e| world.get::<Name>(e).map(|n| n.0 != "lang").unwrap_or(false))
        .collect();
    roots.sort_by_key(|&e| {
        world
            .get::<Name>(e)
            .map(|n| n.0.clone())
            .unwrap_or_default()
    });

    let mut stack = roots;
    while let Some(module) = stack.pop() {
        let path = module_path(world, module);
        let name = world
            .get::<Name>(module)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let page = build_page(world, module, &path, &protocol_index, &extensions_by_target);

        for &child in world.children_of(module) {
            if matches!(world.get::<NodeKind>(child), Some(NodeKind::Module)) {
                stack.push(child);
            }
        }

        summaries.push(ModuleSummary {
            path: path.clone(),
            name,
            item_count: count_items(&page),
        });
        pages.push(page);
    }

    summaries.sort_by(|a, b| a.path.cmp(&b.path));
    pages.sort_by(|a, b| a.path.cmp(&b.path));
    (ModuleIndex { modules: summaries }, pages)
}

fn build_page(
    world: &World,
    module: Entity,
    path: &str,
    protocol_index: &HashMap<String, Entity>,
    extensions_by_target: &HashMap<Entity, Vec<Entity>>,
) -> ModulePage {
    let name = world
        .get::<Name>(module)
        .map(|n| n.0.clone())
        .unwrap_or_default();

    let mut submodules: Vec<String> = Vec::new();
    let mut items: Vec<Item> = Vec::new();

    let mut children: Vec<Entity> = world.children_of(module).to_vec();
    children.sort_by_key(|&e| {
        world
            .get::<Name>(e)
            .map(|n| n.0.clone())
            .unwrap_or_default()
    });

    for child in children {
        let Some(kind) = world.get::<NodeKind>(child) else {
            continue;
        };
        if matches!(kind, NodeKind::Module) {
            submodules.push(module_path(world, child));
            continue;
        }
        // Extensions are merged into the type they target — never rendered
        // as their own item. Their members + conformances flow into the
        // target's member groups via `build_member_groups`.
        if matches!(kind, NodeKind::Extension) {
            continue;
        }
        if !is_public_top_level(world, child) {
            continue;
        }
        if signature::is_private(world, child) {
            continue;
        }
        if let Some(item) = build_item(world, child, protocol_index, extensions_by_target) {
            items.push(item);
        }
    }

    ModulePage {
        path: path.to_string(),
        name,
        submodules,
        items,
    }
}

fn build_item(
    world: &World,
    entity: Entity,
    protocol_index: &HashMap<String, Entity>,
    extensions_by_target: &HashMap<Entity, Vec<Entity>>,
) -> Option<Item> {
    let kind = world.get::<NodeKind>(entity)?;
    if !is_documented(kind) {
        return None;
    }
    let raw_name = world
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| anonymous_name(kind));

    let opts = signature::Options::default();
    let signature = signature::build(world, entity, opts);
    let raw_doc = world
        .get::<Documentation>(entity)
        .map(|d| d.0.clone())
        .filter(|s| !s.is_empty())
        .or_else(|| docs_from_source(world, entity))
        .unwrap_or_default();
    let (name_directive, doc) = extract_name_directive(&raw_doc);
    let source_path = entity_file_path(world, entity);

    // For inits/subscripts the entity's `Name` is just "init"/"subscript",
    // which collapses every overload to the same row. If the doc carries
    // a `@name <Display Name>` directive, use that instead so each
    // overload gets a distinct, human-readable label.
    let display_name = match (kind, &name_directive) {
        (NodeKind::Initializer, Some(n)) => n.clone(),
        (NodeKind::Subscript, Some(n)) => n.clone(),
        _ => raw_name.clone(),
    };

    let kind_str = kind_label(kind);
    let anchor = make_anchor(&kind_str, &display_name);
    let name = display_name;

    let member_groups = if has_members(kind) {
        let empty: Vec<Entity> = Vec::new();
        let extensions = extensions_by_target.get(&entity).unwrap_or(&empty);
        build_member_groups(
            world,
            entity,
            protocol_index,
            extensions,
            extensions_by_target,
        )
    } else {
        Vec::new()
    };

    Some(Item {
        kind: kind_str,
        name,
        anchor,
        signature,
        doc,
        source_path,
        member_groups,
    })
}

/// Build the docs.rs-style member layout. Walks the type's own children
/// and routes each one to the conformed-protocol group whose declaration
/// it satisfies (by raw-name match), so an `Iterable` impl's
/// `type Item = T` lands under the **Implements Iterable** heading
/// instead of in `Direct`. Anything that doesn't match a protocol stays
/// in `Direct`. Each protocol group is then padded out with the
/// protocol's own members for slots the type didn't override (showing
/// default-method/abstract signatures).
fn build_member_groups(
    world: &World,
    entity: Entity,
    protocol_index: &HashMap<String, Entity>,
    extensions: &[Entity],
    extensions_by_target: &HashMap<Entity, Vec<Entity>>,
) -> Vec<MemberGroup> {
    // Step 1: collect member entities from the type itself + every
    // extension targeting it. Extensions are flattened into the type's
    // docs surface so users see one consolidated view.
    let mut direct_entities: Vec<(Entity, String)> = Vec::new();
    let mut sources: Vec<Entity> = vec![entity];
    sources.extend(extensions.iter().copied());
    for &source in &sources {
        for &child in world.children_of(source) {
            let Some(kind) = world.get::<NodeKind>(child) else {
                continue;
            };
            if !is_member_kind(kind) {
                continue;
            }
            if signature::is_private(world, child) {
                continue;
            }
            let raw_name = world
                .get::<Name>(child)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            direct_entities.push((child, raw_name));
        }
    }

    // Step 2: collect protocols this type conforms to — both directly and
    // via every extension — in source order, deduped.
    let mut conformed: Vec<Entity> = Vec::new();
    for &source in &sources {
        if let Some(conformances) = world.get::<Conformances>(source) {
            for item in &conformances.0 {
                let kestrel_ast_builder::ConformanceItem::Positive(conformance_ty, _) = item else {
                    continue;
                };
                let Some(protocol) =
                    signature::resolve_protocol(world, protocol_index, conformance_ty)
                else {
                    continue;
                };
                if protocol == entity || conformed.contains(&protocol) {
                    continue;
                }
                conformed.push(protocol);
            }
        }
    }

    // Step 3: route each direct entity to a protocol group whose
    // declaration it satisfies. First match wins so we don't double-list.
    let mut by_protocol: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let mut assigned: std::collections::HashSet<Entity> = Default::default();
    for &protocol in &conformed {
        let protocol_member_names: std::collections::HashSet<String> = world
            .children_of(protocol)
            .iter()
            .filter_map(|&c| world.get::<Name>(c).map(|n| n.0.clone()))
            .collect();
        for (e, raw_name) in &direct_entities {
            if assigned.contains(e) {
                continue;
            }
            if protocol_member_names.contains(raw_name) {
                by_protocol.entry(protocol).or_default().push(*e);
                assigned.insert(*e);
            }
        }
    }

    // Step 4: build the Direct group from anything left over.
    let mut groups = Vec::new();
    let mut direct_items: Vec<Item> = direct_entities
        .iter()
        .filter(|(e, _)| !assigned.contains(e))
        .filter_map(|(e, _)| build_item(world, *e, protocol_index, extensions_by_target))
        .collect();
    direct_items.sort_by(|a, b| a.name.cmp(&b.name));
    if !direct_items.is_empty() {
        groups.push(MemberGroup {
            kind: "direct".into(),
            label: None,
            source_path: None,
            members: direct_items,
        });
    }

    // Step 5: build a group per conformed protocol — the type's
    // implementations first, then any protocol-declared members the type
    // didn't override (so abstract / default-only items still show up).
    for protocol in conformed {
        let label = protocol_short_name(world, protocol);
        let source_path = Some(module_path_for(world, protocol));

        let mut members: Vec<Item> = Vec::new();
        let mut covered_names: std::collections::HashSet<String> = Default::default();

        for &e in by_protocol.get(&protocol).unwrap_or(&Vec::new()) {
            let raw_name = world
                .get::<Name>(e)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            if let Some(item) = build_item(world, e, protocol_index, extensions_by_target) {
                covered_names.insert(raw_name);
                members.push(item);
            }
        }
        // Collect members from the protocol itself and any extensions of
        // the protocol (default implementations like `extend Str`).
        let empty_ext: Vec<Entity> = Vec::new();
        let protocol_extensions = extensions_by_target.get(&protocol).unwrap_or(&empty_ext);
        let mut protocol_sources: Vec<Entity> = vec![protocol];
        protocol_sources.extend(protocol_extensions.iter().copied());
        for &proto_source in &protocol_sources {
            for &child in world.children_of(proto_source) {
                let Some(kind) = world.get::<NodeKind>(child) else {
                    continue;
                };
                if !is_member_kind(kind) {
                    continue;
                }
                if signature::is_private(world, child) {
                    continue;
                }
                let raw_name = world
                    .get::<Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                if covered_names.contains(&raw_name) {
                    continue;
                }
                covered_names.insert(raw_name);
                if let Some(item) = build_item(world, child, protocol_index, extensions_by_target) {
                    members.push(item);
                }
            }
        }

        members.sort_by(|a, b| a.name.cmp(&b.name));
        if !members.is_empty() {
            groups.push(MemberGroup {
                kind: "protocol".into(),
                label: Some(label),
                source_path,
                members,
            });
        }
    }

    groups
}

/// Group every Extension in the world by the entity it targets, so a
/// container's docs page can pull in members and conformances from each
/// extension that extends it. Resolution prefers a same-module match
/// when multiple types share a short name.
fn build_extension_index(world: &World) -> HashMap<Entity, Vec<Entity>> {
    let type_index = build_type_index(world);
    let mut by_target: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for (ext, kind) in world.iter_component::<NodeKind>() {
        if !matches!(kind, NodeKind::Extension) {
            continue;
        }
        let Some(target) = world.get::<ExtensionTarget>(ext) else {
            continue;
        };
        let Some(name) = target_head_name(&target.0) else {
            continue;
        };
        let Some(target_entity) = pick_type_for_extension(world, &type_index, &name, ext) else {
            continue;
        };
        by_target.entry(target_entity).or_default().push(ext);
    }
    by_target
}

/// Index every container-kind entity by its short name. Used to resolve
/// an `extension Foo` target back to the `Foo` declaration.
fn build_type_index(world: &World) -> HashMap<String, Vec<Entity>> {
    let mut map: HashMap<String, Vec<Entity>> = HashMap::new();
    for (e, kind) in world.iter_component::<NodeKind>() {
        if !matches!(kind, NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) {
            continue;
        }
        if let Some(name) = world.get::<Name>(e) {
            map.entry(name.0.clone()).or_default().push(e);
        }
    }
    map
}

fn target_head_name(t: &AstType) -> Option<String> {
    match t {
        AstType::Named { segments, .. } => segments.last().map(|s| s.name.clone()),
        _ => None,
    }
}

/// When multiple types share a short name, prefer one defined in the
/// same module as the extension. Otherwise return the first match.
fn pick_type_for_extension(
    world: &World,
    type_index: &HashMap<String, Vec<Entity>>,
    name: &str,
    extension: Entity,
) -> Option<Entity> {
    let candidates = type_index.get(name)?;
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return Some(candidates[0]);
    }
    let ext_module = world.parent_of(extension)?;
    candidates
        .iter()
        .copied()
        .find(|&c| world.parent_of(c) == Some(ext_module))
        .or_else(|| candidates.first().copied())
}

/// Index every Protocol entity by its short name. Stdlib happens to have
/// unique protocol names, so we don't need full path-based resolution
/// for `Conformances` (which often carry just the short name).
fn build_protocol_index(world: &World) -> HashMap<String, Entity> {
    let mut map = HashMap::new();
    for (e, kind) in world.iter_component::<NodeKind>() {
        if !matches!(kind, NodeKind::Protocol) {
            continue;
        }
        if let Some(name) = world.get::<Name>(e) {
            map.entry(name.0.clone()).or_insert(e);
        }
    }
    map
}

fn protocol_short_name(world: &World, protocol: Entity) -> String {
    world
        .get::<Name>(protocol)
        .map(|n| n.0.clone())
        .unwrap_or_default()
}

fn module_path_for(world: &World, entity: Entity) -> String {
    if let Some(parent) = world.parent_of(entity)
        && matches!(world.get::<NodeKind>(parent), Some(NodeKind::Module)) {
            let mp = module_path(world, parent);
            if let Some(name) = world.get::<Name>(entity) {
                return format!("{}.{}", mp, name.0);
            }
            return mp;
        }
    String::new()
}

fn entity_file_path(world: &World, entity: Entity) -> Option<String> {
    if let Some(p) = world.get::<FilePath>(entity) {
        return Some(p.0.clone());
    }
    let fid = world.get::<FileId>(entity)?;
    world.get::<FilePath>(fid.0).map(|p| p.0.clone())
}

fn is_public_top_level(world: &World, entity: Entity) -> bool {
    // Top-level items must be explicitly `public`. Stdlib follows this
    // convention strictly; anything internal stays out of the docs.
    if let Some(vis) = world.get::<Vis>(entity) {
        return matches!(vis, Vis::Public);
    }
    matches!(signature::visibility(world, entity), Some("public"))
}

fn is_documented(kind: &NodeKind) -> bool {
    // Extensions are intentionally excluded — they're folded into the
    // type they target rather than shown as standalone items.
    matches!(
        kind,
        NodeKind::Function
            | NodeKind::Initializer
            | NodeKind::Subscript
            | NodeKind::Field
            | NodeKind::Struct
            | NodeKind::Enum
            | NodeKind::EnumCase
            | NodeKind::Protocol
            | NodeKind::TypeAlias
    )
}

fn has_members(kind: &NodeKind) -> bool {
    matches!(kind, NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol)
}

fn is_member_kind(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Function
            | NodeKind::Initializer
            | NodeKind::Subscript
            | NodeKind::Field
            | NodeKind::EnumCase
            | NodeKind::TypeAlias
    )
}

fn kind_label(kind: &NodeKind) -> String {
    match kind {
        NodeKind::Function => "function",
        NodeKind::Initializer => "initializer",
        NodeKind::Subscript => "subscript",
        NodeKind::Field => "field",
        NodeKind::Struct => "struct",
        NodeKind::Enum => "enum",
        NodeKind::EnumCase => "case",
        NodeKind::Protocol => "protocol",
        NodeKind::Extension => "extension",
        NodeKind::TypeAlias => "typealias",
        _ => "other",
    }
    .to_string()
}

fn anonymous_name(kind: &NodeKind) -> String {
    match kind {
        NodeKind::Initializer => "init",
        NodeKind::Subscript => "subscript",
        NodeKind::Extension => "extension",
        _ => "_",
    }
    .to_string()
}

fn make_anchor(kind: &str, name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    format!("{}-{}", kind, slug)
}

fn module_path(world: &World, module: Entity) -> String {
    let mut segments: Vec<String> = Vec::new();
    let mut cur = Some(module);
    while let Some(e) = cur {
        if !matches!(world.get::<NodeKind>(e), Some(NodeKind::Module)) {
            break;
        }
        let Some(name) = world.get::<Name>(e) else {
            break;
        };
        if name.0 == "<root>" {
            break;
        }
        segments.push(name.0.clone());
        cur = world.parent_of(e);
    }
    segments.reverse();
    segments.join(".")
}

/// Fallback: when the AST builder didn't attach a `Documentation`
/// component (e.g. the parser folded the leading trivia into the wrong
/// node — happens reproducibly for all-but-the-first subscript on a
/// type), recover the doc body by scanning source-text lines preceding
/// the declaration. Returns `None` when there's no `///` block above
/// the decl.
fn docs_from_source(world: &World, entity: Entity) -> Option<String> {
    let span = world.get::<DeclSpan>(entity)?;
    let file = world.get::<FileId>(entity)?.0;
    let source = world.get::<kestrel_compiler::SourceText>(file)?.0.clone();
    let bytes = source.as_bytes();

    // Walk to the start of the line containing decl_span.start.
    let mut pos = span.0.start.min(source.len());
    while pos > 0 && bytes[pos - 1] != b'\n' {
        pos -= 1;
    }

    // Walk backward line-by-line. Collect contiguous `///` lines (with
    // optional blank lines between them). Stop at any other content.
    let mut chunks: Vec<String> = Vec::new();
    let mut blank_run = false;
    while pos > 0 {
        let line_end = pos - 1; // skip past the '\n' that ended the prior line
        let mut line_start = line_end;
        while line_start > 0 && bytes[line_start - 1] != b'\n' {
            line_start -= 1;
        }
        let line = &source[line_start..line_end];
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("///") {
            // `////` is a section divider, not a doc comment.
            if rest.starts_with('/') {
                break;
            }
            let content = rest.strip_prefix(' ').unwrap_or(rest).to_string();
            chunks.push(content);
            blank_run = false;
            pos = line_start;
        } else if trimmed.is_empty() {
            // Blank line — only allowed once between doc paragraphs.
            if blank_run {
                break;
            }
            blank_run = true;
            pos = line_start;
        } else {
            break;
        }
    }

    if chunks.is_empty() {
        return None;
    }
    chunks.reverse();
    Some(chunks.join("\n").trim().to_string())
}

/// Pull a `@name <Display Name>` directive out of a doc-comment body.
/// Returns `(directive, doc_with_directive_line_removed)`. Stdlib uses
/// these to disambiguate init/subscript overloads in the rendered docs.
fn extract_name_directive(doc: &str) -> (Option<String>, String) {
    let mut directive: Option<String> = None;
    let mut kept: Vec<&str> = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("@name ") {
            if directive.is_none() {
                directive = Some(rest.trim().to_string());
            }
            continue;
        }
        kept.push(line);
    }
    // Collapse any double-blank-lines left behind after stripping the
    // directive so doc bodies don't gain accidental gaps.
    let mut cleaned = String::new();
    let mut prev_blank = false;
    for line in kept {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        cleaned.push_str(line);
        cleaned.push('\n');
        prev_blank = is_blank;
    }
    (directive, cleaned.trim().to_string())
}

fn count_items(page: &ModulePage) -> usize {
    page.items
        .iter()
        .map(|it| {
            1 + it
                .member_groups
                .iter()
                .map(|g| g.members.len())
                .sum::<usize>()
        })
        .sum::<usize>()
}
