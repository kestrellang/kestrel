//! Walks a compiled Kestrel `World` and extracts public declarations into
//! a JSON-serializable shape suitable for a rustdoc-style site.
//!
//! Mirrors the LSP hover renderer in shape: each item carries a signature
//! sliced from source (decl-span start → body-block start) plus its
//! `Documentation` component as raw markdown.

#[cfg(test)]
mod repro_test;

use kestrel_ast_builder::{
    CstNode, DeclSpan, Documentation, FileId, FilePath, Name, NodeKind, Vis,
};
use kestrel_hecs::{Entity, World};
use kestrel_syntax_tree::{SyntaxKind, SyntaxNode};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ModuleIndex {
    /// Top-level module entries (e.g. `std.core`, `std.collections`). Each
    /// has its own JSON file written alongside the index.
    pub modules: Vec<ModuleSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModuleSummary {
    /// Dotted module path (`std.core`).
    pub path: String,
    /// Last segment of the path (`core`).
    pub name: String,
    /// Total public items in this module subtree (including submodules).
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModulePage {
    pub path: String,
    pub name: String,
    /// Direct submodules (dotted paths).
    pub submodules: Vec<String>,
    /// Public items declared in this module.
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub kind: String,
    pub name: String,
    /// Anchor slug (`fn-bump`, `struct-Array`).
    pub anchor: String,
    /// Source-sliced signature (no body).
    pub signature: String,
    /// Doc-comment markdown (`Documentation` component), possibly empty.
    pub doc: String,
    /// Source file path the item is defined in.
    pub source_path: Option<String>,
    /// Nested members — fields/cases/methods on structs, enums, protocols,
    /// and extensions. Empty for plain functions / type aliases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Item>,
}

/// Extract a doc-site index + per-module pages from a populated world.
///
/// Walks the entire module tree (from `root`), skipping the synthetic
/// `lang` module. Emits one `ModulePage` per module — a top-level module
/// like `std` and each of its descendants (`std.core`, `std.collections`,
/// `std.core.ordering`, …) all become independent pages.
pub fn extract(world: &World, root: Entity) -> (ModuleIndex, Vec<ModulePage>) {
    let mut pages = Vec::new();
    let mut summaries = Vec::new();

    let mut roots: Vec<Entity> = world
        .children_of(root)
        .iter()
        .copied()
        .filter(|&e| matches!(world.get::<NodeKind>(e), Some(NodeKind::Module)))
        .filter(|&e| {
            world
                .get::<Name>(e)
                .map(|n| n.0 != "lang")
                .unwrap_or(false)
        })
        .collect();
    roots.sort_by_key(|&e| world.get::<Name>(e).map(|n| n.0.clone()).unwrap_or_default());

    let mut stack = roots;
    while let Some(module) = stack.pop() {
        let path = module_path(world, module);
        let name = world
            .get::<Name>(module)
            .map(|n| n.0.clone())
            .unwrap_or_default();
        let page = build_page(world, module, &path);

        // Push child modules so they get their own pages too.
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

/// Build a page for a module — collects its direct items and lists the
/// dotted paths of its submodules.
fn build_page(world: &World, module: Entity, path: &str) -> ModulePage {
    let name = world
        .get::<Name>(module)
        .map(|n| n.0.clone())
        .unwrap_or_default();

    let mut submodules: Vec<String> = Vec::new();
    let mut items: Vec<Item> = Vec::new();

    let mut children: Vec<Entity> = world.children_of(module).iter().copied().collect();
    children.sort_by_key(|&e| world.get::<Name>(e).map(|n| n.0.clone()).unwrap_or_default());

    for child in children {
        let Some(kind) = world.get::<NodeKind>(child) else {
            continue;
        };
        if matches!(kind, NodeKind::Module) {
            submodules.push(module_path(world, child));
            continue;
        }
        if !is_public_top_level(world, child) {
            continue;
        }
        if let Some(item) = build_item(world, child) {
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

fn build_item(world: &World, entity: Entity) -> Option<Item> {
    let kind = world.get::<NodeKind>(entity)?;
    if !is_documented(kind) {
        return None;
    }
    let name = world
        .get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| anonymous_name(kind));

    let signature = signature_text(world, entity).unwrap_or_default();
    let doc = world
        .get::<Documentation>(entity)
        .map(|d| d.0.clone())
        .unwrap_or_default();
    let source_path = entity_file_path(world, entity);

    let kind_str = kind_label(kind, world, entity);
    let anchor = make_anchor(&kind_str, &name);

    // Collect public members for container kinds. Extensions get all
    // public members regardless of kind, since extensions only exist to
    // add them.
    let members = if has_members(kind) {
        let mut child_items = Vec::new();
        for &child in world.children_of(entity) {
            let Some(child_kind) = world.get::<NodeKind>(child) else {
                continue;
            };
            if !is_member_kind(child_kind) {
                continue;
            }
            if !is_public_member(world, child) {
                continue;
            }
            if let Some(item) = build_item(world, child) {
                child_items.push(item);
            }
        }
        child_items.sort_by(|a, b| a.name.cmp(&b.name));
        child_items
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
        members,
    })
}

/// Slice the source from the declaration's start to its body-block start
/// (or to end-of-decl when there is no body). This is the same shape the
/// LSP hover uses, just minus its CST/source-cache plumbing.
fn signature_text(world: &World, entity: Entity) -> Option<String> {
    let cst = world.get::<CstNode>(entity)?;
    let decl_span = world.get::<DeclSpan>(entity)?.0.clone();
    let path = entity_file_path(world, entity)?;
    let file_entity = source_file_entity(world, entity)?;
    let source = world
        .get::<kestrel_compiler::SourceText>(file_entity)
        .map(|s| s.0.clone())
        .or_else(|| std::fs::read_to_string(&path).ok())?;

    let body_start = first_body_block_offset(&cst.0).unwrap_or(decl_span.end);
    let end = body_start.min(decl_span.end);
    let raw = source.get(decl_span.start..end).unwrap_or("");
    Some(raw.trim_end_matches([';', ' ', '\t', '\n', '\r']).to_string())
}

/// Offset of where the entity's body / accessor block actually begins. We
/// can't trust the body node's `text_range().start()` directly: the parser
/// sometimes folds preceding tokens (e.g. the trailing `]` of a generic
/// return type) into the body node's leading trivia, which would chop off
/// the last character of the rendered signature. Instead, walk the body
/// node's tokens and take the first `LBrace` — that's the source-level `{`
/// that opens the block.
fn first_body_block_offset(cst: &SyntaxNode) -> Option<usize> {
    for child in cst.children() {
        if !matches!(
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
            continue;
        }
        let opener = child
            .descendants_with_tokens()
            .filter_map(|el| el.into_token())
            .find(|t| t.kind() == SyntaxKind::LBrace);
        if let Some(tok) = opener {
            return Some(tok.text_range().start().into());
        }
        return Some(child.text_range().start().into());
    }
    None
}

fn entity_file_path(world: &World, entity: Entity) -> Option<String> {
    if let Some(p) = world.get::<FilePath>(entity) {
        return Some(p.0.clone());
    }
    let fid = world.get::<FileId>(entity)?;
    world.get::<FilePath>(fid.0).map(|p| p.0.clone())
}

fn source_file_entity(world: &World, entity: Entity) -> Option<Entity> {
    world.get::<FileId>(entity).map(|f| f.0)
}

/// Top-level items (direct module children) only ship if they're explicitly
/// `public`. Stdlib follows this convention strictly.
fn is_public_top_level(world: &World, entity: Entity) -> bool {
    matches!(world.get::<Vis>(entity).map(|v| (*v).clone()), Some(Vis::Public))
}

/// Members of containers (protocol methods, extension methods, etc.) often
/// lack an explicit visibility — protocol APIs are public by convention.
/// Filter only the explicitly-private ones.
fn is_public_member(world: &World, entity: Entity) -> bool {
    match world.get::<Vis>(entity).map(|v| (*v).clone()) {
        Some(Vis::Private) | Some(Vis::Fileprivate) => false,
        _ => true,
    }
}

fn is_documented(kind: &NodeKind) -> bool {
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
            | NodeKind::Extension
            | NodeKind::TypeAlias
    )
}

fn has_members(kind: &NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension
    )
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

fn kind_label(kind: &NodeKind, _world: &World, _entity: Entity) -> String {
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

fn count_items(page: &ModulePage) -> usize {
    page.items
        .iter()
        .map(|it| 1 + it.members.len())
        .sum::<usize>()
}
