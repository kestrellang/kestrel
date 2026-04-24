//! Place expressions — paths to memory locations.
//!
//! Bare enum, no metadata. Spans belong on the statement that uses the place.

use crate::id::LocalId;
use kestrel_hecs::Entity;

/// A place is a path to a memory location that can be read, written, or referenced.
#[derive(Debug, Clone)]
pub enum Place {
    /// A local variable: `%x`
    Local(LocalId),

    /// A global/static variable: `@Module.Path.var`
    Global(Entity),

    /// Field access: `<place>.field_name`
    Field { parent: Box<Place>, name: String },

    /// Tuple index: `<place>.0`
    Index { parent: Box<Place>, index: usize },

    /// Enum downcast: `<place>.SomeCase` (valid after switch)
    Downcast { parent: Box<Place>, variant: String },

    /// Dereference: `deref <place>`
    Deref(Box<Place>),
}

impl Place {
    /// Create a place referencing a local variable.
    pub fn local(id: LocalId) -> Self {
        Place::Local(id)
    }

    /// Create a place referencing a global/static variable.
    pub fn global(entity: Entity) -> Self {
        Place::Global(entity)
    }

    /// Project into a field. Chainable: `Place::local(x).field("y").deref()`
    pub fn field(self, name: impl Into<String>) -> Self {
        Place::Field {
            parent: Box::new(self),
            name: name.into(),
        }
    }

    /// Project into a tuple element by index.
    pub fn index(self, index: usize) -> Self {
        Place::Index {
            parent: Box::new(self),
            index,
        }
    }

    /// Downcast an enum to a specific variant.
    pub fn downcast(self, variant: impl Into<String>) -> Self {
        Place::Downcast {
            parent: Box::new(self),
            variant: variant.into(),
        }
    }

    /// Dereference a pointer or reference.
    pub fn deref(self) -> Self {
        Place::Deref(Box::new(self))
    }

    /// Get the root local of this place, if it has one.
    /// Returns None for global places.
    pub fn root_local(&self) -> Option<LocalId> {
        match self {
            Place::Local(id) => Some(*id),
            Place::Global(_) => None,
            Place::Field { parent, .. }
            | Place::Index { parent, .. }
            | Place::Downcast { parent, .. }
            | Place::Deref(parent) => parent.root_local(),
        }
    }

    /// Check if this is a simple local variable (no projections).
    pub fn is_local(&self) -> bool {
        matches!(self, Place::Local(_))
    }

    /// Get the local ID if this is a simple local place.
    pub fn as_local(&self) -> Option<LocalId> {
        match self {
            Place::Local(id) => Some(*id),
            _ => None,
        }
    }
}
