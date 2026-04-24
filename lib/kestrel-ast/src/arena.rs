//! Simple arena allocator for AST nodes.
//!
//! Vec-backed with phantom-typed `Idx<T>` indices. Used to store expressions,
//! patterns, and statements in flat arenas within `AstBody`.

use std::fmt;
use std::marker::PhantomData;
use std::ops::Index;

/// A typed index into an `Arena<T>`.
///
/// Copy + lightweight (u32). The phantom type prevents mixing indices
/// from different arena types.
pub struct Idx<T> {
    raw: u32,
    _marker: PhantomData<fn() -> T>,
}

// Manual impls to avoid requiring T: Clone/Copy/etc.
impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Idx<T> {}

impl<T> PartialEq for Idx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}
impl<T> Eq for Idx<T> {}

impl<T> std::hash::Hash for Idx<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.raw.hash(state);
    }
}

impl<T> fmt::Debug for Idx<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Idx({})", self.raw)
    }
}

impl<T> Idx<T> {
    /// Get the raw u32 index value.
    pub fn raw(self) -> u32 {
        self.raw
    }
}

/// A flat arena that owns values of type `T`, addressable by `Idx<T>`.
#[derive(Clone, Debug)]
pub struct Arena<T> {
    data: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Allocate a value and return its index.
    pub fn alloc(&mut self, value: T) -> Idx<T> {
        let idx = Idx {
            raw: self.data.len() as u32,
            _marker: PhantomData,
        };
        self.data.push(value);
        idx
    }

    /// Number of elements in the arena.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Iterate over all (index, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> {
        self.data.iter().enumerate().map(|(i, v)| {
            (
                Idx {
                    raw: i as u32,
                    _marker: PhantomData,
                },
                v,
            )
        })
    }
}

impl<T: std::hash::Hash> std::hash::Hash for Arena<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Index<Idx<T>> for Arena<T> {
    type Output = T;

    fn index(&self, idx: Idx<T>) -> &T {
        &self.data[idx.raw as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_and_index() {
        let mut arena: Arena<String> = Arena::new();
        let a = arena.alloc("hello".into());
        let b = arena.alloc("world".into());

        assert_eq!(arena[a], "hello");
        assert_eq!(arena[b], "world");
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn idx_is_copy() {
        let mut arena: Arena<i32> = Arena::new();
        let idx = arena.alloc(42);
        let idx2 = idx; // Copy
        assert_eq!(arena[idx], arena[idx2]);
    }

    #[test]
    fn iter_pairs() {
        let mut arena: Arena<&str> = Arena::new();
        arena.alloc("a");
        arena.alloc("b");
        arena.alloc("c");

        let pairs: Vec<_> = arena.iter().map(|(i, v)| (i.raw(), *v)).collect();
        assert_eq!(pairs, vec![(0, "a"), (1, "b"), (2, "c")]);
    }

    #[test]
    fn empty_arena() {
        let arena: Arena<i32> = Arena::new();
        assert!(arena.is_empty());
        assert_eq!(arena.len(), 0);
    }
}
