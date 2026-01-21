//! Witness definitions (protocol implementations).

use crate::MirContext;
use crate::id::{Id, QualifiedName, Ty, TypeParam};
use crate::metadata::{Metadata, Prior};
use std::collections::HashMap;
use std::fmt;

/// A witness proves that a type implements a protocol.
///
/// ```text
/// witness Type[T]: Protocol {
///     type AssociatedType = ConcreteType
///     func method = path.to.implementation
/// }
/// ```
#[derive(Debug, Clone)]
pub struct WitnessDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<WitnessDef>>,
    /// The type that implements the protocol.
    pub implementing_type: Id<Ty>,
    /// The protocol being implemented.
    pub protocol: Id<QualifiedName>,
    /// Type parameters for this witness.
    pub type_params: Vec<Id<TypeParam>>,
    /// Associated type bindings: name -> concrete type.
    pub type_bindings: HashMap<String, Id<Ty>>,
    /// Method bindings: method name -> (implementation function path, type arguments).
    /// Type arguments are used when the implementation is a generic function that needs
    /// to be instantiated (e.g., protocol extension methods with Self type parameter).
    pub method_bindings: HashMap<String, (Id<QualifiedName>, Vec<Id<Ty>>)>,
}

impl WitnessDef {
    pub fn new(implementing_type: Id<Ty>, protocol: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            implementing_type,
            protocol,
            type_params: Vec::new(),
            type_bindings: HashMap::new(),
            method_bindings: HashMap::new(),
        }
    }

    /// Bind an associated type to a concrete type.
    pub fn bind_type(&mut self, name: impl Into<String>, ty: Id<Ty>) {
        self.type_bindings.insert(name.into(), ty);
    }

    /// Bind a method to its implementation.
    ///
    /// `type_args` are the type arguments to pass to the implementation function.
    /// For direct implementations (method on the implementing type), this is empty.
    /// For protocol extension methods, this typically includes `Self=implementing_type`.
    pub fn bind_method(
        &mut self,
        name: impl Into<String>,
        implementation: Id<QualifiedName>,
        type_args: Vec<Id<Ty>>,
    ) {
        self.method_bindings
            .insert(name.into(), (implementation, type_args));
    }

    /// Create a display wrapper for printing this witness.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        WitnessDefDisplay { def: self, ctx }
    }
}

struct WitnessDefDisplay<'a> {
    def: &'a WitnessDef,
    ctx: &'a MirContext,
}

impl fmt::Display for WitnessDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "witness {}: {}",
            self.ctx.ty(self.def.implementing_type).display(self.ctx),
            self.ctx.name(self.def.protocol)
        )?;

        writeln!(f, " {{")?;

        for (name, ty) in &self.def.type_bindings {
            writeln!(
                f,
                "    type {} = {}",
                name,
                self.ctx.ty(*ty).display(self.ctx)
            )?;
        }

        for (name, (impl_path, type_args)) in &self.def.method_bindings {
            write!(f, "    func {} = {}", name, self.ctx.name(*impl_path))?;
            if !type_args.is_empty() {
                write!(f, "[")?;
                for (i, ty) in type_args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", self.ctx.ty(*ty).display(self.ctx))?;
                }
                write!(f, "]")?;
            }
            writeln!(f)?;
        }

        write!(f, "}}")
    }
}
