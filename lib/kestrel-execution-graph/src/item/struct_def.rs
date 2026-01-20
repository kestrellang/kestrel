//! Struct definitions in MIR.

use crate::MirContext;
use crate::id::{Field, Id, QualifiedName, TypeParam};
use crate::metadata::{Metadata, Prior};
use std::collections::HashMap;
use std::fmt;

/// A struct definition.
///
/// ```text
/// struct Module.Path.StructName[T, U] {
///     field1: Type1
///     field2: Type2
/// }
/// ```
#[derive(Debug, Clone)]
pub struct StructDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<StructDef>>,
    /// Fully qualified name of this struct.
    pub name: Id<QualifiedName>,
    /// Generic type parameters.
    pub type_params: Vec<Id<TypeParam>>,
    /// Fields in declaration order.
    pub fields: Vec<Id<Field>>,
    /// Field lookup by name.
    pub fields_by_name: HashMap<String, Id<Field>>,
}

impl StructDef {
    pub fn new(name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            type_params: Vec::new(),
            fields: Vec::new(),
            fields_by_name: HashMap::new(),
        }
    }

    /// Add a field to this struct.
    pub fn add_field(&mut self, name: String, id: Id<Field>) {
        self.fields_by_name.insert(name, id);
        self.fields.push(id);
    }

    /// Look up a field by name.
    pub fn field_by_name(&self, name: &str) -> Option<Id<Field>> {
        self.fields_by_name.get(name).copied()
    }

    /// Create a display wrapper for printing this struct.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        StructDefDisplay { def: self, ctx }
    }
}

struct StructDefDisplay<'a> {
    def: &'a StructDef,
    ctx: &'a MirContext,
}

impl fmt::Display for StructDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "struct {}", self.ctx.name(self.def.name))?;

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

        writeln!(f, " {{")?;

        for field_id in &self.def.fields {
            let field = &self.ctx.fields[*field_id];
            writeln!(
                f,
                "    {}: {}",
                field.name,
                self.ctx.ty(field.ty).display(self.ctx)
            )?;
        }

        write!(f, "}}")
    }
}
