//! Protocol definitions in MIR.

use crate::id::{AssociatedType, Id, ProtocolMethod, QualifiedName, TypeParam};
use crate::metadata::{Metadata, Prior};
use crate::MirContext;
use std::collections::HashMap;
use std::fmt;

/// A protocol definition.
///
/// ```text
/// protocol Module.Path.ProtocolName[T] {
///     type AssociatedType
///     func method(self: &Self, args...) -> ReturnType
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ProtocolDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<ProtocolDef>>,
    /// Fully qualified name of this protocol.
    pub name: Id<QualifiedName>,
    /// Generic type parameters.
    pub type_params: Vec<Id<TypeParam>>,
    /// Parent protocols (for inheritance like `protocol Shape: Drawable`).
    /// References the qualified names of inherited protocols.
    pub parent_protocols: Vec<Id<QualifiedName>>,
    /// Associated types in declaration order.
    pub associated_types: Vec<Id<AssociatedType>>,
    /// Associated type lookup by name.
    pub associated_types_by_name: HashMap<String, Id<AssociatedType>>,
    /// Methods in declaration order.
    pub methods: Vec<Id<ProtocolMethod>>,
    /// Method lookup by name.
    pub methods_by_name: HashMap<String, Id<ProtocolMethod>>,
}

impl ProtocolDef {
    pub fn new(name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            type_params: Vec::new(),
            parent_protocols: Vec::new(),
            associated_types: Vec::new(),
            associated_types_by_name: HashMap::new(),
            methods: Vec::new(),
            methods_by_name: HashMap::new(),
        }
    }

    /// Add a parent protocol (for inheritance).
    pub fn add_parent(&mut self, parent: Id<QualifiedName>) {
        self.parent_protocols.push(parent);
    }

    /// Add an associated type to this protocol.
    pub fn add_associated_type(&mut self, name: String, id: Id<AssociatedType>) {
        self.associated_types_by_name.insert(name, id);
        self.associated_types.push(id);
    }

    /// Add a method to this protocol.
    pub fn add_method(&mut self, name: String, id: Id<ProtocolMethod>) {
        self.methods_by_name.insert(name, id);
        self.methods.push(id);
    }

    /// Look up an associated type by name.
    pub fn associated_type_by_name(&self, name: &str) -> Option<Id<AssociatedType>> {
        self.associated_types_by_name.get(name).copied()
    }

    /// Look up a method by name.
    pub fn method_by_name(&self, name: &str) -> Option<Id<ProtocolMethod>> {
        self.methods_by_name.get(name).copied()
    }

    /// Create a display wrapper for printing this protocol.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        ProtocolDefDisplay { def: self, ctx }
    }
}

struct ProtocolDefDisplay<'a> {
    def: &'a ProtocolDef,
    ctx: &'a MirContext,
}

impl fmt::Display for ProtocolDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "protocol {}", self.ctx.name(self.def.name))?;

        if !self.def.type_params.is_empty() {
            write!(f, "[")?;
            for (i, tp) in self.def.type_params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.ctx.type_param(*tp).name)?;
            }
            write!(f, "]")?;
        }

        // Show parent protocols if any
        if !self.def.parent_protocols.is_empty() {
            write!(f, ": ")?;
            for (i, parent) in self.def.parent_protocols.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", self.ctx.name(*parent))?;
            }
        }

        writeln!(f, " {{")?;

        for assoc_id in &self.def.associated_types {
            let assoc = &self.ctx.associated_types[*assoc_id];
            writeln!(f, "    type {}", assoc.name)?;
        }

        for method_id in &self.def.methods {
            let method = &self.ctx.protocol_methods[*method_id];
            write!(f, "    func {}(", method.name)?;
            for (i, (param_name, param_ty)) in method.params.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}: {}", param_name, self.ctx.ty(*param_ty).display(self.ctx))?;
            }
            writeln!(f, ") -> {}", self.ctx.ty(method.ret).display(self.ctx))?;
        }

        write!(f, "}}")
    }
}
