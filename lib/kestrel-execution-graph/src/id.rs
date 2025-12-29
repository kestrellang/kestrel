//! Typed identifiers and arenas for MIR nodes.

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// A typed identifier referencing an item in an arena.
///
/// The phantom type `T` is a marker indicating what kind of item this ID references.
/// This provides compile-time safety: you can't accidentally use an `Id<Function>`
/// where an `Id<Block>` is expected.
pub struct Id<T> {
    raw: u32,
    _phantom: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn from_raw(raw: u32) -> Self {
        Self {
            raw,
            _phantom: PhantomData,
        }
    }

    pub fn raw(self) -> u32 {
        self.raw
    }
}

// Manual trait impls to avoid requiring bounds on T

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.raw == other.raw
    }
}

impl<T> Eq for Id<T> {}

impl<T> Hash for Id<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.hash(state)
    }
}

impl<T> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id({})", self.raw)
    }
}

/// A typed arena that stores items and hands out `Id<T>` handles.
///
/// The type parameter `T` is the marker type for IDs, and `V` is the actual
/// stored value type. For example, `Arena<Function, FunctionDef>` stores
/// `FunctionDef` values and returns `Id<Function>` handles.
#[derive(Debug, Clone)]
pub struct Arena<T, V> {
    items: Vec<V>,
    _phantom: PhantomData<fn() -> T>,
}

impl<T, V> Arena<T, V> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn alloc(&mut self, item: V) -> Id<T> {
        let id = Id::from_raw(self.items.len() as u32);
        self.items.push(item);
        id
    }

    pub fn get(&self, id: Id<T>) -> &V {
        &self.items[id.raw() as usize]
    }

    pub fn get_mut(&mut self, id: Id<T>) -> &mut V {
        &mut self.items[id.raw() as usize]
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id<T>, &V)> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, v)| (Id::from_raw(i as u32), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Id<T>, &mut V)> {
        self.items
            .iter_mut()
            .enumerate()
            .map(|(i, v)| (Id::from_raw(i as u32), v))
    }

    pub fn ids(&self) -> impl Iterator<Item = Id<T>> {
        (0..self.items.len() as u32).map(Id::from_raw)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T, V> Default for Arena<T, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, V> std::ops::Index<Id<T>> for Arena<T, V> {
    type Output = V;
    fn index(&self, id: Id<T>) -> &V {
        self.get(id)
    }
}

impl<T, V> std::ops::IndexMut<Id<T>> for Arena<T, V> {
    fn index_mut(&mut self, id: Id<T>) -> &mut V {
        self.get_mut(id)
    }
}

// === Marker types for Id<T> ===

/// Marker type for function IDs.
#[derive(Debug, Clone, Copy)]
pub struct Function;

/// Marker type for basic block IDs.
#[derive(Debug, Clone, Copy)]
pub struct Block;

/// Marker type for statement IDs.
#[derive(Debug, Clone, Copy)]
pub struct Statement;

/// Marker type for local variable IDs.
#[derive(Debug, Clone, Copy)]
pub struct Local;

/// Marker type for parameter IDs.
#[derive(Debug, Clone, Copy)]
pub struct Param;

/// Marker type for struct IDs.
#[derive(Debug, Clone, Copy)]
pub struct Struct;

/// Marker type for field IDs.
#[derive(Debug, Clone, Copy)]
pub struct Field;

/// Marker type for enum IDs.
#[derive(Debug, Clone, Copy)]
pub struct Enum;

/// Marker type for enum case IDs.
#[derive(Debug, Clone, Copy)]
pub struct EnumCase;

/// Marker type for protocol IDs.
#[derive(Debug, Clone, Copy)]
pub struct Protocol;

/// Marker type for associated type IDs.
#[derive(Debug, Clone, Copy)]
pub struct AssociatedType;

/// Marker type for protocol method IDs.
#[derive(Debug, Clone, Copy)]
pub struct ProtocolMethod;

/// Marker type for witness IDs.
#[derive(Debug, Clone, Copy)]
pub struct Witness;

/// Marker type for static variable IDs.
#[derive(Debug, Clone, Copy)]
pub struct Static;

/// Marker type for type parameter IDs.
#[derive(Debug, Clone, Copy)]
pub struct TypeParam;

/// Marker type for interned type IDs.
#[derive(Debug, Clone, Copy)]
pub struct Ty;

/// Marker type for interned qualified name IDs.
#[derive(Debug, Clone, Copy)]
pub struct QualifiedName;
