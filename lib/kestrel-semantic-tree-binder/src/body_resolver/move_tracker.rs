//! Move tracking for non-copyable types.
//!
//! Tracks which local variables have been moved and reports errors
//! when moved values are used.
//!
//! # Move States
//!
//! - `Valid` - Variable can be used
//! - `Moved` - Variable was definitely moved (e.g., passed to a consuming function)
//! - `MaybeMoved` - Variable was moved in one code path but not another (e.g., if branch)
//!
//! # Branching Semantics
//!
//! When control flow diverges (if/else, match), we need to track what happens in
//! each branch and merge the results:
//!
//! - If moved in **all** branches → `Moved`
//! - If moved in **some** branches → `MaybeMoved`
//! - If moved in **no** branches → `Valid`

use std::collections::{HashMap, HashSet};

use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_span::Span;

/// State of a local variable regarding moves.
#[derive(Clone, Debug)]
pub enum MoveState {
    /// Variable is valid and can be used
    Valid,
    /// Variable has been moved and cannot be used
    Moved {
        /// Where the move occurred
        moved_at: Span,
    },
    /// Variable may have been moved (moved in some branches but not all)
    MaybeMoved {
        /// Where a potential move occurred
        moved_at: Span,
    },
}

/// A snapshot of move state that can be restored later
pub type MoveSnapshot = HashMap<LocalId, MoveState>;

/// Tracks moved values during body resolution.
#[derive(Clone, Debug, Default)]
pub struct MoveTracker {
    /// Move state per local variable (by LocalId)
    states: HashMap<LocalId, MoveState>,
}

impl MoveTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a variable as moved.
    pub fn mark_moved(&mut self, local_id: LocalId, span: Span) {
        self.states
            .insert(local_id, MoveState::Moved { moved_at: span });
    }

    /// Check if a variable has been definitely moved. Returns the move span if so.
    pub fn get_move_span(&self, local_id: LocalId) -> Option<Span> {
        match self.states.get(&local_id) {
            Some(MoveState::Moved { moved_at }) => Some(moved_at.clone()),
            _ => None,
        }
    }

    /// Check if a variable may have been moved. Returns the move span if so.
    pub fn get_maybe_move_span(&self, local_id: LocalId) -> Option<Span> {
        match self.states.get(&local_id) {
            Some(MoveState::MaybeMoved { moved_at }) => Some(moved_at.clone()),
            _ => None,
        }
    }

    /// Check if a variable is definitely valid (not moved or maybe-moved).
    pub fn is_valid(&self, local_id: LocalId) -> bool {
        !matches!(
            self.states.get(&local_id),
            Some(MoveState::Moved { .. } | MoveState::MaybeMoved { .. })
        )
    }

    /// Create a snapshot of the current move states for later restoration.
    pub fn snapshot(&self) -> MoveSnapshot {
        self.states.clone()
    }

    /// Restore a previous snapshot of move states.
    pub fn restore(&mut self, snapshot: MoveSnapshot) {
        self.states = snapshot;
    }

    /// Merge two branch states (e.g., from if/else branches).
    ///
    /// This implements the following semantics:
    /// - If a variable is `Moved` in both branches → `Moved`
    /// - If a variable is `Moved` in one branch but not the other → `MaybeMoved`
    /// - If a variable is `MaybeMoved` in any branch → `MaybeMoved`
    /// - Otherwise → `Valid` (or unchanged)
    ///
    /// After calling this, `self` contains the merged state.
    pub fn merge(&mut self, other: &MoveSnapshot) {
        // Collect all variables that might have changed
        let all_ids: HashSet<LocalId> = self.states.keys().chain(other.keys()).copied().collect();

        for id in all_ids {
            let state_self = self.states.get(&id);
            let state_other = other.get(&id);

            let merged = match (state_self, state_other) {
                // Both valid (or not tracked) → stays valid/untracked
                (None, None) => continue,
                (Some(MoveState::Valid), Some(MoveState::Valid)) => continue,
                (Some(MoveState::Valid), None) | (None, Some(MoveState::Valid)) => continue,

                // Both definitely moved → stays moved (use first span)
                (Some(MoveState::Moved { moved_at }), Some(MoveState::Moved { .. })) => {
                    MoveState::Moved {
                        moved_at: moved_at.clone(),
                    }
                }

                // One moved, one not (or valid) → maybe moved
                (Some(MoveState::Moved { moved_at }), Some(MoveState::Valid))
                | (Some(MoveState::Moved { moved_at }), None) => MoveState::MaybeMoved {
                    moved_at: moved_at.clone(),
                },
                (Some(MoveState::Valid), Some(MoveState::Moved { moved_at }))
                | (None, Some(MoveState::Moved { moved_at })) => MoveState::MaybeMoved {
                    moved_at: moved_at.clone(),
                },

                // Any maybe-moved → stays maybe-moved
                (Some(MoveState::MaybeMoved { moved_at }), _) => MoveState::MaybeMoved {
                    moved_at: moved_at.clone(),
                },
                (_, Some(MoveState::MaybeMoved { moved_at })) => MoveState::MaybeMoved {
                    moved_at: moved_at.clone(),
                },
            };

            self.states.insert(id, merged);
        }
    }

    /// Merge multiple branch states (e.g., from match arms).
    ///
    /// This is a generalization of `merge` for multiple branches:
    /// - If a variable is `Moved` in **all** branches → `Moved`
    /// - If a variable is `Moved` in **some** branches → `MaybeMoved`
    /// - Otherwise → `Valid`
    pub fn merge_all(&mut self, branches: &[MoveSnapshot]) {
        if branches.is_empty() {
            return;
        }

        // Start with first branch
        self.states = branches[0].clone();

        // Merge each subsequent branch
        for branch in &branches[1..] {
            self.merge(branch);
        }
    }

    /// Promote all `MaybeMoved` states to `Moved`.
    ///
    /// This is used for loops - after a loop body, any variable that was
    /// maybe-moved inside the loop is definitely invalid for subsequent
    /// iterations (and after the loop for `loop`).
    pub fn promote_maybe_to_moved(&mut self) {
        for state in self.states.values_mut() {
            if let MoveState::MaybeMoved { moved_at } = state {
                *state = MoveState::Moved {
                    moved_at: moved_at.clone(),
                };
            }
        }
    }
}
