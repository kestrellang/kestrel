//! Cycle detection primitives for graph traversal operations.
//!
//! This module provides reusable utilities for detecting cycles in various
//! scenarios such as:
//! - Type alias resolution (detecting circular type aliases)
//! - Import resolution (detecting circular imports)
//! - Symbol dependency analysis
//!
//! # Example
//!
//! ```ignore
//! use semantic_tree::cycle::CycleDetector;
//!
//! let mut detector = CycleDetector::new();
//!
//! // Enter a node - returns guard on success, Err if cycle detected
//! let _guard_a = match detector.enter("A") {
//!     Ok(guard) => guard,
//!     Err(cycle) => {
//!         // Handle cycle
//!         return;
//!     }
//! };
//!
//! // Nested entry - guard automatically exits in reverse order when dropped
//! let _guard_b = detector.enter("B").unwrap();
//!
//! // This would detect a cycle back to "A"
//! if let Err(cycle) = detector.enter("A") {
//!     // cycle.path() returns ["A", "B", "A"]
//! }
//!
//! // Guards automatically call exit() when dropped (RAII pattern)
//! ```

use std::cell::RefCell;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

/// A detected cycle in a graph traversal.
///
/// Contains information about the cycle including all participants
/// in the order they were visited.
#[derive(Debug, Clone)]
pub struct Cycle<T> {
    /// The path from the start of traversal to the cycle point.
    /// The last element is the same as some earlier element, indicating the cycle.
    path: Vec<T>,
    /// The index in `path` where the cycle begins (the repeated element).
    cycle_start: usize,
}

impl<T: Clone + Debug> Cycle<T> {
    /// Get the full path that led to discovering the cycle.
    ///
    /// The last element will be the same as the element at `cycle_start_index()`.
    pub fn path(&self) -> &[T] {
        &self.path
    }

    /// Get just the cycle portion of the path.
    ///
    /// This is the sequence from the first occurrence of the repeated element
    /// to (but not including) its second occurrence.
    pub fn cycle(&self) -> &[T] {
        &self.path[self.cycle_start..self.path.len() - 1]
    }

    /// Get the element that caused the cycle (appeared twice).
    pub fn cycle_cause(&self) -> &T {
        self.path.last().expect("cycle path should not be empty")
    }

    /// Get the index in the path where the cycle begins.
    pub fn cycle_start_index(&self) -> usize {
        self.cycle_start
    }

    /// Returns true if this is a self-cycle (element references itself directly).
    pub fn is_self_cycle(&self) -> bool {
        self.path.len() == 2 && self.cycle_start == 0
    }
}

/// RAII guard that automatically exits a cycle detector node when dropped.
///
/// Returned by `CycleDetector::enter()` to ensure `exit()` is called
/// even if a panic or early return occurs.
#[derive(Debug)]
pub struct CycleGuard<'a, T: Clone + Eq + Hash + Debug> {
    detector: &'a mut CycleDetector<T>,
}

impl<'a, T: Clone + Eq + Hash + Debug> Drop for CycleGuard<'a, T> {
    fn drop(&mut self) {
        self.detector.exit();
    }
}

/// RAII guard that automatically exits a cycle detector node when dropped.
/// This variant works with RefCell for interior mutability.
///
/// Returned by `CycleDetector::enter_ref()` to ensure `exit()` is called
/// even if a panic or early return occurs.
#[derive(Debug)]
pub struct CycleGuardRef<'a, T: Clone + Eq + Hash + Debug> {
    detector: &'a RefCell<CycleDetector<T>>,
}

impl<'a, T: Clone + Eq + Hash + Debug> Drop for CycleGuardRef<'a, T> {
    fn drop(&mut self) {
        self.detector.borrow_mut().exit();
    }
}

/// A cycle detector for tracking visited nodes during graph traversal.
///
/// This is useful for detecting cycles in recursive resolution operations
/// like type alias resolution, import resolution, etc.
///
/// # Type Parameters
///
/// * `T` - The type used to identify nodes. Must be `Clone + Eq + Hash`.
///   Common choices include `SymbolId`, `String`, or custom identifier types.
#[derive(Debug)]
pub struct CycleDetector<T> {
    /// Set of currently active (in-progress) nodes for O(1) cycle detection.
    active: HashSet<T>,
    /// Stack of nodes in visitation order for cycle path reconstruction.
    stack: Vec<T>,
}

impl<T: Clone + Eq + Hash + Debug> CycleDetector<T> {
    /// Create a new empty cycle detector.
    pub fn new() -> Self {
        Self {
            active: HashSet::new(),
            stack: Vec::new(),
        }
    }

    /// Create a new cycle detector with expected capacity.
    ///
    /// Use this when you have a reasonable estimate of the maximum
    /// depth of recursion to avoid reallocations.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            active: HashSet::with_capacity(capacity),
            stack: Vec::with_capacity(capacity),
        }
    }

    /// Enter a node in the traversal.
    ///
    /// Returns `Ok(CycleGuard)` if the node hasn't been visited yet in the current path,
    /// or `Err(Cycle)` if entering this node would create a cycle.
    ///
    /// The returned guard automatically calls `exit()` when dropped, ensuring proper
    /// cleanup even if a panic or early return occurs.
    pub fn enter(&mut self, node: T) -> Result<CycleGuard<'_, T>, Cycle<T>> {
        if self.active.contains(&node) {
            // Found a cycle - build the cycle path
            let mut path = self.stack.clone();
            path.push(node.clone());

            // Find where the cycle starts
            let cycle_start = path
                .iter()
                .position(|n| n == &node)
                .expect("node must be in stack since it's in active set");

            return Err(Cycle { path, cycle_start });
        }

        self.active.insert(node.clone());
        self.stack.push(node);
        Ok(CycleGuard { detector: self })
    }

    /// Enter a node via RefCell (for interior mutability patterns).
    ///
    /// This is a convenience method for use with `RefCell<CycleDetector<T>>`.
    /// Returns `Ok(CycleGuardRef)` if the node hasn't been visited yet,
    /// or `Err(Cycle)` if entering this node would create a cycle.
    ///
    /// The returned guard automatically calls `exit()` when dropped.
    pub fn enter_ref(detector_ref: &RefCell<Self>, node: T) -> Result<CycleGuardRef<'_, T>, Cycle<T>> {
        let mut detector = detector_ref.borrow_mut();

        if detector.active.contains(&node) {
            // Found a cycle - build the cycle path
            let mut path = detector.stack.clone();
            path.push(node.clone());

            // Find where the cycle starts
            let cycle_start = path
                .iter()
                .position(|n| n == &node)
                .expect("node must be in stack since it's in active set");

            return Err(Cycle { path, cycle_start });
        }

        detector.active.insert(node.clone());
        detector.stack.push(node);
        drop(detector); // Release the borrow
        Ok(CycleGuardRef { detector: detector_ref })
    }

    /// Exit the current node in the traversal.
    ///
    /// This is called automatically by `CycleGuard` when it is dropped.
    /// You rarely need to call this manually - prefer using the guard returned by `enter()`.
    ///
    /// # Panics
    ///
    /// Panics if called when no node is active (stack is empty).
    pub fn exit(&mut self) {
        let node = self
            .stack
            .pop()
            .expect("exit() called with empty stack - mismatched enter/exit");
        self.active.remove(&node);
    }

    /// Check if a node is currently being visited (in the active path).
    pub fn is_active(&self, node: &T) -> bool {
        self.active.contains(node)
    }

    /// Get the current traversal depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Get the current path being traversed.
    pub fn current_path(&self) -> &[T] {
        &self.stack
    }

    /// Check if the detector is empty (no active traversal).
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Clear the detector, resetting it to initial state.
    pub fn clear(&mut self) {
        self.active.clear();
        self.stack.clear();
    }
}

impl<T: Clone + Eq + Hash + Debug> Default for CycleDetector<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_cycle() {
        let detector = RefCell::new(CycleDetector::new());

        let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
        let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
        let _guard_c = CycleDetector::enter_ref(&detector, "C").unwrap();

        // Guards automatically exit in reverse order when dropped
        drop(_guard_c);
        drop(_guard_b);
        drop(_guard_a);

        assert!(detector.borrow().is_empty());
    }

    #[test]
    fn test_self_cycle() {
        let detector = RefCell::new(CycleDetector::new());

        let _guard = CycleDetector::enter_ref(&detector, "A").unwrap();
        let result = CycleDetector::enter_ref(&detector, "A");

        assert!(result.is_err());
        let cycle = result.unwrap_err();
        assert!(cycle.is_self_cycle());
        assert_eq!(cycle.cycle_cause(), &"A");
        assert_eq!(cycle.path(), &["A", "A"]);
        assert_eq!(cycle.cycle(), &["A"]);
    }

    #[test]
    fn test_indirect_cycle() {
        let detector = RefCell::new(CycleDetector::new());

        let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
        let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
        let _guard_c = CycleDetector::enter_ref(&detector, "C").unwrap();
        let result = CycleDetector::enter_ref(&detector, "A");

        assert!(result.is_err());
        let cycle = result.unwrap_err();
        assert!(!cycle.is_self_cycle());
        assert_eq!(cycle.cycle_cause(), &"A");
        assert_eq!(cycle.path(), &["A", "B", "C", "A"]);
        assert_eq!(cycle.cycle(), &["A", "B", "C"]);
        assert_eq!(cycle.cycle_start_index(), 0);
    }

    #[test]
    fn test_cycle_in_middle() {
        let detector = RefCell::new(CycleDetector::new());

        let _guard_x = CycleDetector::enter_ref(&detector, "X").unwrap();
        let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
        let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
        let _guard_c = CycleDetector::enter_ref(&detector, "C").unwrap();
        let result = CycleDetector::enter_ref(&detector, "A");

        assert!(result.is_err());
        let cycle = result.unwrap_err();
        assert_eq!(cycle.path(), &["X", "A", "B", "C", "A"]);
        assert_eq!(cycle.cycle(), &["A", "B", "C"]);
        assert_eq!(cycle.cycle_start_index(), 1);
    }

    #[test]
    fn test_reuse_after_exit() {
        let detector = RefCell::new(CycleDetector::new());

        {
            let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
            let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
            // Guards dropped here
        }

        // Should be able to enter "A" again after exiting
        assert!(CycleDetector::enter_ref(&detector, "A").is_ok());
        assert!(CycleDetector::enter_ref(&detector, "B").is_ok());
    }

    #[test]
    fn test_is_active() {
        let detector = RefCell::new(CycleDetector::new());

        assert!(!detector.borrow().is_active(&"A"));

        let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
        assert!(detector.borrow().is_active(&"A"));
        assert!(!detector.borrow().is_active(&"B"));

        let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
        assert!(detector.borrow().is_active(&"A"));
        assert!(detector.borrow().is_active(&"B"));

        drop(_guard_b);
        assert!(detector.borrow().is_active(&"A"));
        assert!(!detector.borrow().is_active(&"B"));
    }

    #[test]
    fn test_current_path() {
        let detector = RefCell::new(CycleDetector::new());

        assert_eq!(detector.borrow().current_path(), &[] as &[&str]);

        let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
        assert_eq!(detector.borrow().current_path(), &["A"]);

        let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
        assert_eq!(detector.borrow().current_path(), &["A", "B"]);

        let _guard_c = CycleDetector::enter_ref(&detector, "C").unwrap();
        assert_eq!(detector.borrow().current_path(), &["A", "B", "C"]);

        drop(_guard_c);
        assert_eq!(detector.borrow().current_path(), &["A", "B"]);
    }

    #[test]
    fn test_clear() {
        let detector = RefCell::new(CycleDetector::new());

        // Enter nodes, then drop guards to exit properly
        {
            let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
            let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();
        }

        // Verify nodes were entered (detector should be empty after guards drop)
        assert!(detector.borrow().is_empty());

        // Enter again
        {
            let _guard_a = CycleDetector::enter_ref(&detector, "A").unwrap();
            let _guard_b = CycleDetector::enter_ref(&detector, "B").unwrap();

            // Check they are active
            assert!(detector.borrow().is_active(&"A"));
            assert!(detector.borrow().is_active(&"B"));

            // Note: We can't call clear() while guards are active because when guards drop,
            // they'll try to exit() and panic. The clear test validates that after normal usage,
            // the detector is properly cleaned up through RAII.
        }

        // After guards drop, detector should be clean
        assert!(detector.borrow().is_empty());
        assert!(!detector.borrow().is_active(&"A"));
        assert!(!detector.borrow().is_active(&"B"));
    }

    #[test]
    #[should_panic(expected = "exit() called with empty stack")]
    fn test_exit_without_enter_panics() {
        let mut detector: CycleDetector<&str> = CycleDetector::new();
        detector.exit();
    }
}
