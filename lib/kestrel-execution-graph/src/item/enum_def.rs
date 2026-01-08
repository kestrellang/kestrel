//! Enum definitions in MIR.

use crate::MirContext;
use crate::id::{EnumCase, Id, QualifiedName, TypeParam};
use crate::metadata::{Metadata, Prior};
use std::collections::HashMap;
use std::fmt;

/// An enum definition.
///
/// ```text
/// enum Module.Path.EnumName[T] {
///     CaseName1: Module.Path.EnumName."cases".CaseName1[T]
///     CaseName2: Module.Path.EnumName."cases".CaseName2[T]
/// }
/// ```
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<EnumDef>>,
    /// Fully qualified name of this enum.
    pub name: Id<QualifiedName>,
    /// Generic type parameters.
    pub type_params: Vec<Id<TypeParam>>,
    /// Cases in declaration order.
    pub cases: Vec<Id<EnumCase>>,
    /// Case lookup by name.
    pub cases_by_name: HashMap<String, Id<EnumCase>>,
}

impl EnumDef {
    pub fn new(name: Id<QualifiedName>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            type_params: Vec::new(),
            cases: Vec::new(),
            cases_by_name: HashMap::new(),
        }
    }

    /// Add a case to this enum.
    pub fn add_case(&mut self, name: String, id: Id<EnumCase>) {
        self.cases_by_name.insert(name, id);
        self.cases.push(id);
    }

    /// Look up a case by name.
    pub fn case_by_name(&self, name: &str) -> Option<Id<EnumCase>> {
        self.cases_by_name.get(name).copied()
    }

    /// Create a display wrapper for printing this enum.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        EnumDefDisplay { def: self, ctx }
    }
}

struct EnumDefDisplay<'a> {
    def: &'a EnumDef,
    ctx: &'a MirContext,
}

impl fmt::Display for EnumDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "enum {}", self.ctx.name(self.def.name))?;

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

        for case_id in &self.def.cases {
            let case = &self.ctx.enum_cases[*case_id];
            write!(f, "    {}: {}", case.name, self.ctx.name(case.struct_name))?;

            // Include type parameters if the enum has any
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
            writeln!(f)?;
        }

        write!(f, "}}")
    }
}
