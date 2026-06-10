//! Shared member-discovery traversal and the name-indexed member map.
//!
//! `TypeMembers` and `ProtocolMembers` (and `ProtocolAssociatedTypes`)
//! walk the same shape — the queried entity's direct children, then its
//! extensions' children, then the protocols it transitively conforms to —
//! differing only in whether parent protocols contribute their *direct*
//! children (protocol inheritance does, type conformance does not) and in
//! the member filter. `collect_members_transitive` is the single source of
//! truth for that walk; the per-query modules supply filter + constructor.
//!
//! `MemberMap` is the Arc-wrapped output shape for the full-member queries:
//! a flat member list in emission order plus a build-time name index so
//! `*MembersByName` is a bucket lookup instead of a full re-scan.

use std::collections::BTreeMap;
use std::ops::Index;

use kestrel_hecs::{Entity, QueryContext};

use crate::conformances::ConformingProtocols;
use crate::extensions::ExtensionsFor;
use crate::helpers::member_lookup_name;

/// Generic member traversal. Emission order is load-bearing: consumers derive
/// member precedence from position (see the insert-overwrite notes in
/// type_members.rs / protocol_members.rs), so reordering this walk changes
/// which member wins. The order:
/// 1. Direct children of `entity`
/// 2. Children of every extension targeting `entity`
/// 3. For each protocol `entity` transitively conforms to (in
///    `ConformingProtocols` order): its direct children when
///    `include_parent_direct_children` (protocol inheritance), then the
///    children of every extension targeting it
///
/// `make` builds the per-query member record from
/// `(member, via_protocol, extension)` — `via_protocol` is `None` for
/// members found on `entity` itself, `extension` is `None` for direct
/// children.
pub(crate) fn collect_members_transitive<M>(
    ctx: &QueryContext<'_>,
    entity: Entity,
    root: Entity,
    include_parent_direct_children: bool,
    filter: fn(&QueryContext<'_>, Entity) -> bool,
    mut make: impl FnMut(Entity, Option<Entity>, Option<Entity>) -> M,
) -> Vec<M> {
    let mut out = Vec::new();

    // The queried entity itself: direct children, then extension children.
    collect_one(ctx, entity, root, None, true, filter, &mut make, &mut out);

    // Transitive conformed/inherited protocols, in ConformingProtocols
    // order (already deduplicated and expanded transitively).
    let conformed = ctx.query(ConformingProtocols { entity, root });
    for &proto in &conformed {
        collect_one(
            ctx,
            proto,
            root,
            Some(proto),
            include_parent_direct_children,
            filter,
            &mut make,
            &mut out,
        );
    }

    out
}

/// One traversal step: `target`'s direct children (when `include_direct`),
/// then the children of every extension targeting `target`.
#[allow(clippy::too_many_arguments)]
fn collect_one<M>(
    ctx: &QueryContext<'_>,
    target: Entity,
    root: Entity,
    via_protocol: Option<Entity>,
    include_direct: bool,
    filter: fn(&QueryContext<'_>, Entity) -> bool,
    make: &mut impl FnMut(Entity, Option<Entity>, Option<Entity>) -> M,
    out: &mut Vec<M>,
) {
    if include_direct {
        for &child in ctx.children_of(target) {
            if filter(ctx, child) {
                out.push(make(child, via_protocol, None));
            }
        }
    }

    let extensions = ctx.query(ExtensionsFor { target, root });
    for &ext in &extensions {
        for &child in ctx.children_of(ext) {
            if filter(ctx, child) {
                out.push(make(child, via_protocol, Some(ext)));
            }
        }
    }
}

/// Name-indexed member collection, the Arc-wrapped output of the full
/// `TypeMembers` / `ProtocolMembers` queries.
///
/// Keeps the flat member list in emission order — consumers iterating the
/// whole collection see exactly the order `collect_members_transitive`
/// produced — plus a name index for O(log n) bucket lookup. Within a
/// bucket, emission order (and therefore source precedence: direct →
/// extension → protocol extension) is preserved. Nameless init/subscript
/// members bucket under the reserved `"init"` / `"subscript"` keyword
/// sentinels (see `member_lookup_name`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MemberMap<M> {
    /// All members in emission order.
    members: Vec<M>,
    /// Lookup name → indices into `members`, ascending (= emission order).
    /// BTreeMap (not HashMap) so the derived `Hash` impl QueryFn needs is
    /// deterministic.
    by_name: BTreeMap<String, Vec<u32>>,
}

impl<M> MemberMap<M> {
    /// Build the index from members in emission order. `entity_of`
    /// projects the member entity out of the provenance-carrying record.
    pub(crate) fn build(
        ctx: &QueryContext<'_>,
        members: Vec<M>,
        entity_of: impl Fn(&M) -> Entity,
    ) -> Self {
        let mut by_name: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        for (i, m) in members.iter().enumerate() {
            // Members with no lookup name (no Name, not init/subscript)
            // stay in the flat list but can't be found by name.
            if let Some(name) = member_lookup_name(ctx, entity_of(m)) {
                by_name.entry(name.to_string()).or_default().push(i as u32);
            }
        }
        Self { members, by_name }
    }

    /// All members in emission order.
    pub fn iter(&self) -> std::slice::Iter<'_, M> {
        self.members.iter()
    }

    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Members answering to `name` (including the init/subscript
    /// sentinels), in emission order. Not visibility-filtered.
    pub fn named<'a>(&'a self, name: &str) -> impl Iterator<Item = &'a M> {
        self.by_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|&i| &self.members[i as usize])
    }
}

impl<M> Index<usize> for MemberMap<M> {
    type Output = M;

    fn index(&self, index: usize) -> &M {
        &self.members[index]
    }
}

impl<'a, M> IntoIterator for &'a MemberMap<M> {
    type Item = &'a M;
    type IntoIter = std::slice::Iter<'a, M>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
