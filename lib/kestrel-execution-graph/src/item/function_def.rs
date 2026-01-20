//! Function definitions in MIR.

use crate::MirContext;
use crate::id::{Block, Id, Local, Param, QualifiedName, Ty, TypeParam};
use crate::metadata::{Metadata, Prior};
use std::collections::HashMap;
use std::fmt;

/// Calling conventions for extern functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    /// C calling convention (cdecl on most platforms)
    C,
}

/// Information about an extern function.
///
/// When present, indicates that this function has no body and
/// will be linked from external code at compile time.
#[derive(Debug, Clone)]
pub struct ExternInfo {
    /// The calling convention for this extern function.
    pub calling_convention: CallingConvention,
    /// The symbol name to use for linking.
    /// This may differ from the function's Kestrel name if `mangleName` was specified.
    pub symbol_name: String,
}

/// A function definition.
///
/// ```text
/// func Module.Path.function_name[T](param1: Type1, param2: Type2) -> ReturnType
/// where T: Protocol, T.Item = Int
/// {
///     locals:
///         %name: Type
///         ...
///
///     bb0:
///         // statements
///         // terminator
/// }
/// ```
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub meta: Metadata,
    pub priors: Vec<Prior<FunctionDef>>,
    /// Fully qualified name of this function.
    pub name: Id<QualifiedName>,
    /// Generic type parameters.
    pub type_params: Vec<Id<TypeParam>>,
    /// Parameters in declaration order.
    pub params: Vec<Id<Param>>,
    /// Parameter lookup by name.
    pub params_by_name: HashMap<String, Id<Param>>,
    /// Return type.
    pub ret: Id<Ty>,
    /// Where clause constraints.
    pub where_clause: Option<WhereClause>,
    /// Local variables (includes parameters).
    pub locals: Vec<Id<Local>>,
    /// Local lookup by name.
    pub locals_by_name: HashMap<String, Id<Local>>,
    /// Basic blocks in this function.
    pub blocks: Vec<Id<Block>>,
    /// Entry block (first block).
    pub entry_block: Option<Id<Block>>,
    /// Extern function info (if this is an @extern function).
    /// When Some, the function has no body and will be linked externally.
    pub extern_info: Option<ExternInfo>,
}

impl FunctionDef {
    pub fn new(name: Id<QualifiedName>, ret: Id<Ty>) -> Self {
        Self {
            meta: Metadata::new(),
            priors: Vec::new(),
            name,
            type_params: Vec::new(),
            params: Vec::new(),
            params_by_name: HashMap::new(),
            ret,
            where_clause: None,
            locals: Vec::new(),
            locals_by_name: HashMap::new(),
            blocks: Vec::new(),
            entry_block: None,
            extern_info: None,
        }
    }

    /// Check if this is an extern function (has no body, linked externally).
    pub fn is_extern(&self) -> bool {
        self.extern_info.is_some()
    }

    /// Add a local variable to this function.
    pub fn add_local(&mut self, name: String, id: Id<Local>) {
        self.locals_by_name.insert(name, id);
        self.locals.push(id);
    }

    /// Look up a local by name.
    pub fn local_by_name(&self, name: &str) -> Option<Id<Local>> {
        self.locals_by_name.get(name).copied()
    }

    /// Create a display wrapper for printing this function.
    pub fn display<'a>(&'a self, ctx: &'a MirContext) -> impl fmt::Display + 'a {
        FunctionDefDisplay { def: self, ctx }
    }
}

/// Where clause for generic constraints.
#[derive(Debug, Clone)]
pub struct WhereClause {
    pub constraints: Vec<WhereConstraint>,
}

impl WhereClause {
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    pub fn add_constraint(&mut self, constraint: WhereConstraint) {
        self.constraints.push(constraint);
    }
}

impl Default for WhereClause {
    fn default() -> Self {
        Self::new()
    }
}

/// A single constraint in a where clause.
#[derive(Debug, Clone)]
pub enum WhereConstraint {
    /// `T: Protocol`
    Implements {
        type_param: Id<TypeParam>,
        protocol: Id<QualifiedName>,
    },
    /// `T.Item = i64`
    TypeEquals {
        base: Id<TypeParam>,
        associated: String,
        equals: Id<Ty>,
    },
}

struct FunctionDefDisplay<'a> {
    def: &'a FunctionDef,
    ctx: &'a MirContext,
}

impl fmt::Display for FunctionDefDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // func name[T, U](params...) -> Ret
        write!(f, "func {}", self.ctx.name(self.def.name))?;

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

        write!(f, "(")?;
        for (i, param_id) in self.def.params.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            let param = &self.ctx.params[*param_id];
            write!(
                f,
                "{}: {}",
                param.name,
                self.ctx.ty(param.ty).display(self.ctx)
            )?;
        }
        write!(f, ") -> {}", self.ctx.ty(self.def.ret).display(self.ctx))?;

        // where clause
        if let Some(wc) = &self.def.where_clause {
            write!(f, "\nwhere ")?;
            for (i, constraint) in wc.constraints.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match constraint {
                    WhereConstraint::Implements {
                        type_param,
                        protocol,
                    } => {
                        write!(
                            f,
                            "{}: {}",
                            self.ctx.type_param(*type_param).name,
                            self.ctx.name(*protocol)
                        )?;
                    }
                    WhereConstraint::TypeEquals {
                        base,
                        associated,
                        equals,
                    } => {
                        write!(
                            f,
                            "{}.{} = {}",
                            self.ctx.type_param(*base).name,
                            associated,
                            self.ctx.ty(*equals).display(self.ctx)
                        )?;
                    }
                }
            }
        }

        writeln!(f, "\n{{")?;

        // locals (excluding parameters)
        let non_param_locals: Vec<_> = self
            .def
            .locals
            .iter()
            .filter(|l| {
                !self
                    .def
                    .params
                    .iter()
                    .any(|p| self.ctx.params[*p].local == **l)
            })
            .collect();

        if !non_param_locals.is_empty() {
            writeln!(f, "    locals:")?;
            for local_id in non_param_locals {
                let local = &self.ctx.locals[*local_id];
                writeln!(
                    f,
                    "        %{}: {}",
                    local.name,
                    self.ctx.ty(local.ty).display(self.ctx)
                )?;
            }
            writeln!(f)?;
        }

        // blocks
        for (i, block_id) in self.def.blocks.iter().enumerate() {
            let block = &self.ctx.blocks[*block_id];
            writeln!(f, "    bb{}:", i)?;
            write!(
                f,
                "{}",
                block.display(self.ctx, "        ", &self.def.blocks)
            )?;
            if i < self.def.blocks.len() - 1 {
                writeln!(f)?;
            }
        }

        write!(f, "}}")
    }
}
