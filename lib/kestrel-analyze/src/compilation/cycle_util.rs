//! Shared cycle-detection DFS state.
//!
//! Thin back-edge detector reused by the four cycle analyzers (struct
//! containment, type alias, protocol inheritance, generic constraint).
//! Each analyzer supplies its own edge walker; this helper only tracks the
//! DFS stack and produces a [`Cycle`] the moment an entity already on the
//! stack is re-entered.

use kestrel_hecs::Entity;
use std::collections::HashSet;

/// Participants in a detected cycle, in DFS-stack order from the cycle
/// origin through the closing entity (which matches `participants[0]`).
/// The closing duplicate is NOT repeated in the vector — callers that want
/// to render `A -> B -> C -> A` should append `participants[0]` at display
/// time.
pub struct Cycle {
    pub participants: Vec<Entity>,
}

/// Active-set DFS state. Construct one per top-level traversal root and
/// thread it through the analyzer's recursive walker.
pub struct CycleDetector {
    active: HashSet<Entity>,
    path: Vec<Entity>,
}

impl CycleDetector {
    pub fn new() -> Self {
        Self {
            active: HashSet::new(),
            path: Vec::new(),
        }
    }

    /// Push `e` onto the DFS stack. If `e` is already on the stack a
    /// [`Cycle`] is returned holding the participants from the previous
    /// occurrence of `e` to the current tip.
    pub fn enter(&mut self, e: Entity) -> Result<(), Cycle> {
        if self.active.contains(&e) {
            let start = self.path.iter().position(|&x| x == e).unwrap();
            let participants = self.path[start..].to_vec();
            return Err(Cycle { participants });
        }
        self.active.insert(e);
        self.path.push(e);
        Ok(())
    }

    /// Pop `e` from the DFS stack. Must be called after a successful
    /// [`enter`](Self::enter), once recursion for `e` has returned.
    pub fn exit(&mut self, e: Entity) {
        debug_assert_eq!(self.path.last().copied(), Some(e));
        self.path.pop();
        self.active.remove(&e);
    }
}

impl Default for CycleDetector {
    fn default() -> Self {
        Self::new()
    }
}
