//! Static variable definitions.

use crate::MirContext;
use crate::function::Immediate;
use crate::id::{Id, QualifiedName, Ty};
use crate::metadata::{Metadata, Prior};
use std::fmt;

/// A static variable (global constant or mutable static).
///
/// ```text
/// static Module.Path.CONSTANT: Type = value
/// static var Module.Path.mutable_global: Type = value
/// ```
#[derive(Debug, Clone)]
pub struct StaticDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<StaticDef>>,
    /// Fully qualified name of this static.
    pub name: Id<QualifiedName>,
    /// Type of this static.
    pub ty: Id<Ty>,
    /// Whether this static is mutable (`var`).
    pub is_mutable: bool,
    /// Initial value (if known at compile time).
    pub initializer: Option<Immediate>,
}

impl StaticDef {
    pub fn new(name: Id<QualifiedName>, ty: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            ty,
            is_mutable: false,
            initializer: None,
        }
    }

    pub fn mutable(mut self) -> Self {
        self.is_mutable = true;
        self
    }

    pub fn with_initializer(mut self, init: Immediate) -> Self {
        self.initializer = Some(init);
        self
    }

    /// Create a display wrapper for printing this static.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        StaticDefDisplay { def: self, ctx }
    }
}

struct StaticDefDisplay<'a> {
    def: &'a StaticDef,
    ctx: &'a MirContext,
}

impl fmt::Display for StaticDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "static ")?;
        if self.def.is_mutable {
            write!(f, "var ")?;
        }
        write!(
            f,
            "{}: {}",
            self.ctx.name(self.def.name),
            self.ctx.ty(self.def.ty).display(self.ctx)
        )?;

        if let Some(init) = &self.def.initializer {
            write!(f, " = {}", init.display(self.ctx))?;
        }

        Ok(())
    }
}
