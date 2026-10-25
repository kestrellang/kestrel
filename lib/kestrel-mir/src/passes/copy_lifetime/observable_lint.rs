//! E508 — find `Cloneable` types whose drop looks observable but which are not
//! marked `@observabledrop`.
//!
//! Runs on the monomorphized module, where every call is resolved to a concrete
//! function (generic and protocol calls included), so the walk is exact:
//! a `deinit` that prints reaches `write` through `println → writeLine[W] →
//! W.write` with `W = Stdout`. Copies of an unmarked type may be dropped right
//! after their last read (`drop_at_last_use`), so if its deinit reaches an
//! extern other than the C allocator / `mem*` functions, that effect can
//! happen earlier than the source suggests. Crash paths (functions returning
//! `!`, like `fatalError`) are not followed.
//!
//! Returns findings; `kestrel-compiler` turns them into warnings with spans.

use rustc_hash::FxHashSet;

use super::TypeFacts;
use crate::TyId;
use crate::callee::Callee;
use crate::inst::InstKind;
use crate::item::{CopyBehavior, DropBehavior, TypeInfo};
use crate::mono::types::MonoModule;
use crate::ty::MirTy;
use kestrel_hecs::Entity;

/// Externs that only allocate, release or move memory; reaching them is not
/// observable.
const MEMORY_EXTERNS: &[&str] = &[
    "malloc", "calloc", "realloc", "free", "memcpy", "memmove", "memset", "memcmp",
    // The system allocator's aligned path (`SystemAllocator.allocate`).
    "posix_memalign", "aligned_alloc",
];

/// One unmarked `Cloneable` type whose deinit reaches an observable extern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservableDropFinding {
    /// The struct/enum declaration.
    pub type_entity: Entity,
    /// The call path from the deinit to the extern: the deinit's direct callee
    /// first, the extern last. Entries may be compiler-synthesized helpers
    /// without a source name.
    pub path: Vec<Entity>,
    /// The extern's C symbol (e.g. `close`, `write`).
    pub symbol: String,
}

/// One finding per type declaration (instantiations are deduplicated).
pub fn unmarked_observable_drops(mono: &MonoModule) -> Vec<ObservableDropFinding> {
    let mut facts = TypeFacts::default();
    let mut seen: FxHashSet<Entity> = FxHashSet::default();
    let mut findings = Vec::new();
    let types = mono
        .structs
        .values()
        .map(|s| {
            (
                s.source,
                &s.type_info,
                s.fields.iter().map(|f| f.ty).collect::<Vec<_>>(),
            )
        })
        .chain(mono.enums.values().map(|e| {
            let tys = e
                .cases
                .iter()
                .flat_map(|c| c.payload_fields.iter().map(|f| f.ty))
                .collect();
            (e.source, &e.type_info, tys)
        }));
    for (entity, info, fields) in types {
        if seen.contains(&entity) || !needs_check(mono, &mut facts, entity, info, &fields) {
            continue;
        }
        let Some(deinit) = deinit_of(info) else {
            continue;
        };
        if let Some(finding) = observable_call(mono, entity, deinit) {
            seen.insert(entity);
            findings.push(finding);
        }
    }
    findings
}

/// `Cloneable`, has a deinit, and its copies may be dropped early (neither it
/// nor anything it contains is `@observabledrop`).
fn needs_check(
    mono: &MonoModule,
    facts: &mut TypeFacts,
    entity: Entity,
    info: &TypeInfo,
    fields: &[TyId],
) -> bool {
    matches!(info.copy, CopyBehavior::Clone(_))
        && deinit_of(info).is_some()
        && !mono.observable_drop.contains(&entity)
        && !fields.iter().any(|t| facts.observable(mono, *t))
}

fn deinit_of(info: &TypeInfo) -> Option<Entity> {
    match &info.drop {
        DropBehavior::None => None,
        DropBehavior::StructDrop { deinit, .. } | DropBehavior::EnumDrop { deinit, .. } => *deinit,
    }
}

/// First call made directly by any instantiation of `deinit` whose call graph
/// reaches an observable extern.
fn observable_call(
    mono: &MonoModule,
    type_entity: Entity,
    deinit: Entity,
) -> Option<ObservableDropFinding> {
    let mut visited = FxHashSet::default();
    for root in (0..mono.functions.len()).filter(|&i| mono.functions[i].source == deinit) {
        visited.insert(root);
        for callee in direct_callees(mono, root) {
            let mut path = Vec::new();
            if let Some(symbol) = reaches_observable_extern(mono, callee, &mut visited, &mut path) {
                let path = path
                    .into_iter()
                    .rev()
                    .map(|fi| mono.functions[fi].source)
                    .collect();
                return Some(ObservableDropFinding {
                    type_entity,
                    path,
                    symbol,
                });
            }
        }
    }
    None
}

/// The observable extern reached by calling function `fi`, if any. On success
/// `path` holds the functions from the extern back to `fi` (reversed).
fn reaches_observable_extern(
    mono: &MonoModule,
    fi: usize,
    visited: &mut FxHashSet<usize>,
    path: &mut Vec<usize>,
) -> Option<String> {
    if let Some(ext) = &mono.functions[fi].extern_info {
        if MEMORY_EXTERNS.contains(&ext.symbol_name.as_str()) {
            return None;
        }
        path.push(fi);
        return Some(ext.symbol_name.clone());
    }
    // A function that never returns (`fatalError`, `-> !`) is a crash path:
    // whatever it prints is not the drop's effect, and following it would flag
    // every deinit that can fail an allocation.
    if matches!(mono.ty_arena.get(mono.functions[fi].ret), MirTy::Never) || !visited.insert(fi) {
        return None;
    }
    let symbol = direct_callees(mono, fi)
        .into_iter()
        .find_map(|callee| reaches_observable_extern(mono, callee, visited, path))?;
    path.push(fi);
    Some(symbol)
}

/// Statically resolved callees of function `fi` (calls through function
/// values are skipped).
fn direct_callees(mono: &MonoModule, fi: usize) -> Vec<usize> {
    let Some(body) = &mono.functions[fi].body else {
        return vec![];
    };
    body.blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter_map(|inst| match &inst.kind {
            InstKind::Call {
                callee: Callee::Resolved(id),
                ..
            } => Some(id.index()),
            _ => None,
        })
        .collect()
}
