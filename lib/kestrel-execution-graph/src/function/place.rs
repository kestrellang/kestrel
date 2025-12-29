//! Place expressions (memory locations).

use crate::id::{Id, Local};
use crate::metadata::Metadata;
use crate::MirContext;
use std::fmt;

/// A place is a memory location that can be read, written, or referenced.
#[derive(Debug, Clone)]
pub struct Place {
    pub meta: Metadata,
    /// Optional inline name for this place expression.
    pub inline_name: Option<String>,
    /// The kind of place.
    pub kind: PlaceKind,
}

/// The different kinds of place expressions.
#[derive(Debug, Clone)]
pub enum PlaceKind {
    /// A local variable: `%x`
    Local(Id<Local>),

    /// Field access: `<place>.field`
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
    pub fn local(id: Id<Local>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Local(id),
        }
    }

    /// Create a field projection of this place.
    pub fn field(self, name: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Field {
                parent: Box::new(self),
                name: name.into(),
            },
        }
    }

    /// Create a tuple index projection of this place.
    pub fn index(self, index: usize) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Index {
                parent: Box::new(self),
                index,
            },
        }
    }

    /// Create an enum downcast projection of this place.
    pub fn downcast(self, variant: impl Into<String>) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Downcast {
                parent: Box::new(self),
                variant: variant.into(),
            },
        }
    }

    /// Create a dereference of this place.
    pub fn deref(self) -> Self {
        Self {
            meta: Metadata::new(),
            inline_name: None,
            kind: PlaceKind::Deref(Box::new(self)),
        }
    }

    /// Set an inline name for this place.
    pub fn with_inline_name(mut self, name: impl Into<String>) -> Self {
        self.inline_name = Some(name.into());
        self
    }

    /// Get the root local of this place.
    pub fn root_local(&self) -> Id<Local> {
        match &self.kind {
            PlaceKind::Local(id) => *id,
            PlaceKind::Field { parent, .. } => parent.root_local(),
            PlaceKind::Index { parent, .. } => parent.root_local(),
            PlaceKind::Downcast { parent, .. } => parent.root_local(),
            PlaceKind::Deref(parent) => parent.root_local(),
        }
    }

    /// Check if this place is just a simple local variable.
    pub fn is_local(&self) -> bool {
        matches!(self.kind, PlaceKind::Local(_))
    }

    /// Get the local ID if this is a simple local place.
    pub fn as_local(&self) -> Option<Id<Local>> {
        match &self.kind {
            PlaceKind::Local(id) => Some(*id),
            _ => None,
        }
    }

    /// Create a display wrapper for printing this place.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        PlaceDisplay { place: self, ctx }
    }
}

struct PlaceDisplay<'a> {
    place: &'a Place,
    ctx: &'a MirContext,
}

impl fmt::Display for PlaceDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.place.kind {
            PlaceKind::Local(id) => {
                write!(f, "%{}", self.ctx.locals[*id].name)
            }
            PlaceKind::Field { parent, name } => {
                write!(f, "{}.{}", parent.display(self.ctx), name)
            }
            PlaceKind::Index { parent, index } => {
                write!(f, "{}.{}", parent.display(self.ctx), index)
            }
            PlaceKind::Downcast { parent, variant } => {
                write!(f, "{}.{}", parent.display(self.ctx), variant)
            }
            PlaceKind::Deref(inner) => {
                write!(f, "(deref {})", inner.display(self.ctx))
            }
        }
    }
}
