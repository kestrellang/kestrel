//! Function definitions.

use crate::body::MirBody;
use crate::id::{LocalId, StructId};
use crate::item::TypeParamDef;
use crate::ty::MirTy;
use kestrel_hecs::Entity;
use indexmap::IndexMap;

/// A function definition.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// The ECS entity for this function.
    pub entity: Entity,
    /// Fully qualified name.
    pub name: String,
    /// What kind of function this is.
    pub kind: FunctionKind,
    /// Generic type parameters.
    pub type_params: Vec<TypeParamDef>,
    /// Parameters in declaration order.
    pub params: Vec<ParamDef>,
    /// Parameter lookup by name.
    pub params_by_name: IndexMap<String, usize>,
    /// Return type.
    pub ret: MirTy,
    /// Where clause constraints.
    pub where_clause: Option<WhereClause>,
    /// Local lookup by name (for the body).
    pub locals_by_name: IndexMap<String, LocalId>,
    /// Function body (None for extern functions).
    pub body: Option<MirBody>,
    /// Extern function info (if this is an @extern function).
    pub extern_info: Option<ExternInfo>,
}

impl FunctionDef {
    pub fn new(entity: Entity, name: impl Into<String>, ret: MirTy) -> Self {
        Self {
            entity,
            name: name.into(),
            kind: FunctionKind::Free,
            type_params: Vec::new(),
            params: Vec::new(),
            params_by_name: IndexMap::new(),
            ret,
            where_clause: None,
            locals_by_name: IndexMap::new(),
            body: None,
            extern_info: None,
        }
    }

    /// Check if this is an extern function.
    pub fn is_extern(&self) -> bool {
        self.extern_info.is_some()
    }

    /// Look up a local by name.
    pub fn local_by_name(&self, name: &str) -> Option<LocalId> {
        self.locals_by_name.get(name).copied()
    }
}

/// What kind of function this is — explicitly stated, no inference needed.
#[derive(Debug, Clone)]
pub enum FunctionKind {
    /// Free function (not attached to a type).
    Free,
    /// Instance method on a type.
    Method {
        parent: Entity,
        receiver: ReceiverConvention,
    },
    /// Static method on a type (no self parameter).
    StaticMethod { parent: Entity },
    /// Initializer (constructor) for a type.
    Initializer { parent: Entity },
    /// Deinitializer (destructor) for a type.
    Deinit { parent: Entity },
    /// Closure's call method (with captures, has env struct).
    ClosureCall { env_struct: StructId },
    /// Non-capturing closure (no env struct, but still only discovered via ApplyPartial).
    Closure,
    /// Thunk wrapping a function for thick-callable ABI compatibility.
    Thunk { original: Entity },
    /// Module initialization function (runs static initializers).
    ModuleInit,
}

/// Receiver convention for methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReceiverConvention {
    /// `&Self` — borrowing receiver
    Ref,
    /// `&var Self` — mutating receiver
    RefMut,
    /// `Self` by value — consuming receiver
    Consuming,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct ParamDef {
    /// Parameter name.
    pub name: String,
    /// The local variable this parameter is bound to.
    pub local: LocalId,
    /// Parameter type.
    pub ty: MirTy,
    /// External label for this parameter (used in mangling).
    pub external_label: Option<String>,
}

impl ParamDef {
    pub fn new(name: impl Into<String>, local: LocalId, ty: MirTy) -> Self {
        Self {
            name: name.into(),
            local,
            ty,
            external_label: None,
        }
    }

    pub fn with_label(
        name: impl Into<String>,
        local: LocalId,
        ty: MirTy,
        label: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            local,
            ty,
            external_label: label,
        }
    }
}

/// Where clause for generic constraints.
#[derive(Debug, Clone, Default)]
pub struct WhereClause {
    pub constraints: Vec<WhereConstraint>,
}

impl WhereClause {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_constraint(&mut self, constraint: WhereConstraint) {
        self.constraints.push(constraint);
    }
}

/// A single constraint in a where clause.
#[derive(Debug, Clone)]
pub enum WhereConstraint {
    /// `T: Protocol`
    Implements {
        type_param: Entity,
        protocol: Entity,
    },
    /// `T.Item = ConcreteType`
    TypeEquals {
        base: Entity,
        associated: String,
        equals: MirTy,
    },
}

/// Calling conventions for extern functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    /// C calling convention.
    C,
}

/// Information about an extern function.
#[derive(Debug, Clone)]
pub struct ExternInfo {
    /// The calling convention.
    pub calling_convention: CallingConvention,
    /// The symbol name to use for linking (may differ from Kestrel name).
    pub symbol_name: String,
}
