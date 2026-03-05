use std::collections::{HashMap, HashSet};

use crate::entity::Entity;
use crate::fingerprint::Fingerprint;

/// Tracks which entities changed between revisions.
///
/// After the mutation phase computes a declaration's components, call
/// `record_fingerprint` with the entity's content hash. If it matches
/// the previous revision, the entity is "backdated" — removed from the
/// changed set. Downstream queries see it as unchanged and return their
/// cached results (early cutoff).
/// Clone support enables `World::snapshot()`.
#[derive(Clone)]
pub struct ChangeSet {
    changed: HashSet<Entity>,
    previous_fingerprints: HashMap<Entity, Fingerprint>,
    current_fingerprints: HashMap<Entity, Fingerprint>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            changed: HashSet::new(),
            previous_fingerprints: HashMap::new(),
            current_fingerprints: HashMap::new(),
        }
    }

    /// Mark an entity as changed in this revision.
    pub fn mark_changed(&mut self, entity: Entity) {
        self.changed.insert(entity);
    }

    /// Record the fingerprint of an entity's content.
    ///
    /// If the fingerprint matches the previous revision, the entity is
    /// backdated (removed from the changed set), enabling early cutoff.
    pub fn record_fingerprint(&mut self, entity: Entity, fp: Fingerprint) {
        self.current_fingerprints.insert(entity, fp);

        if self.previous_fingerprints.get(&entity) == Some(&fp) {
            // Content unchanged — backdate
            self.changed.remove(&entity);
        }
    }

    /// Check if an entity was changed in this revision.
    pub fn is_changed(&self, entity: Entity) -> bool {
        self.changed.contains(&entity)
    }

    /// Get all changed entities.
    pub fn changed_entities(&self) -> &HashSet<Entity> {
        &self.changed
    }

    /// Advance to the next revision: current fingerprints become previous,
    /// changed set is cleared.
    pub fn advance(&mut self) {
        self.previous_fingerprints = std::mem::take(&mut self.current_fingerprints);
        self.changed.clear();
    }
}

impl Default for ChangeSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(n: u32) -> Entity {
        Entity::from_raw(n)
    }

    #[test]
    fn mark_changed() {
        let mut cs = ChangeSet::new();
        assert!(!cs.is_changed(e(0)));
        cs.mark_changed(e(0));
        assert!(cs.is_changed(e(0)));
    }

    #[test]
    fn backdate_removes_from_changed() {
        let mut cs = ChangeSet::new();

        // Revision 1: record fingerprint
        cs.record_fingerprint(e(0), Fingerprint::of(&"hello"));
        cs.mark_changed(e(0));
        assert!(cs.is_changed(e(0)));

        // Advance to revision 2
        cs.advance();
        assert!(!cs.is_changed(e(0)));

        // Revision 2: same fingerprint — should backdate
        cs.mark_changed(e(0));
        assert!(cs.is_changed(e(0)));
        cs.record_fingerprint(e(0), Fingerprint::of(&"hello"));
        assert!(!cs.is_changed(e(0))); // backdated!
    }

    #[test]
    fn changed_fingerprint_stays_changed() {
        let mut cs = ChangeSet::new();

        // Revision 1
        cs.record_fingerprint(e(0), Fingerprint::of(&"hello"));
        cs.advance();

        // Revision 2: different fingerprint
        cs.mark_changed(e(0));
        cs.record_fingerprint(e(0), Fingerprint::of(&"world"));
        assert!(cs.is_changed(e(0))); // NOT backdated
    }

    #[test]
    fn advance_clears_changed() {
        let mut cs = ChangeSet::new();
        cs.mark_changed(e(0));
        cs.mark_changed(e(1));
        assert_eq!(cs.changed_entities().len(), 2);

        cs.advance();
        assert!(cs.changed_entities().is_empty());
    }

    #[test]
    fn new_entity_not_backdated() {
        let mut cs = ChangeSet::new();
        // No previous fingerprint exists, so even recording won't backdate
        cs.mark_changed(e(0));
        cs.record_fingerprint(e(0), Fingerprint::of(&"new"));
        assert!(cs.is_changed(e(0)));
    }
}
