//! Attribute targets: the kinds of declaration an attribute may sit on.
//!
//! The target is computed from the entity, never stored: its `NodeKind`, its
//! parent's kind and whether it has a body ([`attr_target`]). Adding a new
//! place attributes can appear is a grammar change and one variant here.

use kestrel_hecs::{Entity, QueryContext};

use crate::components::{FieldBacking, FieldClass, FieldOwner, NodeKind, Valued};

/// Where an attribute can appear.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum AttrTarget {
    Struct,
    Enum,
    Protocol,
    EnumCase,
    TypeAlias,
    AssociatedType,
    FreeFunction,
    Method,
    /// A protocol requirement without a body.
    Requirement,
    Initializer,
    Deinit,
    Subscript,
    StoredField,
    ComputedProperty,
    /// A module-level `let`/`var`.
    ModuleValue,
    Extension,
}

impl AttrTarget {
    /// Every target, in display order.
    pub const ALL: [AttrTarget; 16] = [
        AttrTarget::Struct,
        AttrTarget::Enum,
        AttrTarget::Protocol,
        AttrTarget::EnumCase,
        AttrTarget::TypeAlias,
        AttrTarget::AssociatedType,
        AttrTarget::FreeFunction,
        AttrTarget::Method,
        AttrTarget::Requirement,
        AttrTarget::Initializer,
        AttrTarget::Deinit,
        AttrTarget::Subscript,
        AttrTarget::StoredField,
        AttrTarget::ComputedProperty,
        AttrTarget::ModuleValue,
        AttrTarget::Extension,
    ];

    /// "a struct", "a protocol requirement", ... for diagnostics.
    pub fn describe(self) -> &'static str {
        match self {
            AttrTarget::Struct => "a struct",
            AttrTarget::Enum => "an enum",
            AttrTarget::Protocol => "a protocol",
            AttrTarget::EnumCase => "an enum case",
            AttrTarget::TypeAlias => "a type alias",
            AttrTarget::AssociatedType => "an associated type",
            AttrTarget::FreeFunction => "a function",
            AttrTarget::Method => "a method",
            AttrTarget::Requirement => "a protocol requirement",
            AttrTarget::Initializer => "an initializer",
            AttrTarget::Deinit => "a deinit",
            AttrTarget::Subscript => "a subscript",
            AttrTarget::StoredField => "a stored field",
            AttrTarget::ComputedProperty => "a computed property",
            AttrTarget::ModuleValue => "a module-level value",
            AttrTarget::Extension => "an extension",
        }
    }

    /// Plural noun for "it applies to ..." lists.
    fn plural(self) -> &'static str {
        match self {
            AttrTarget::Struct => "structs",
            AttrTarget::Enum => "enums",
            AttrTarget::Protocol => "protocols",
            AttrTarget::EnumCase => "enum cases",
            AttrTarget::TypeAlias => "type aliases",
            AttrTarget::AssociatedType => "associated types",
            AttrTarget::FreeFunction => "functions",
            AttrTarget::Method => "methods",
            AttrTarget::Requirement => "protocol requirements",
            AttrTarget::Initializer => "initializers",
            AttrTarget::Deinit => "deinits",
            AttrTarget::Subscript => "subscripts",
            AttrTarget::StoredField => "stored fields",
            AttrTarget::ComputedProperty => "computed properties",
            AttrTarget::ModuleValue => "module-level values",
            AttrTarget::Extension => "extensions",
        }
    }
}

/// A set of [`AttrTarget`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TargetSet(u32);

impl TargetSet {
    pub const ALL: TargetSet = TargetSet::of(&AttrTarget::ALL);

    pub const fn of(targets: &[AttrTarget]) -> TargetSet {
        let mut bits = 0u32;
        let mut i = 0;
        while i < targets.len() {
            bits |= 1 << targets[i] as u8;
            i += 1;
        }
        TargetSet(bits)
    }

    /// This set minus `target`.
    pub const fn without(self, target: AttrTarget) -> TargetSet {
        TargetSet(self.0 & !(1 << target as u8))
    }

    pub fn contains(self, target: AttrTarget) -> bool {
        self.0 & (1 << target as u8) != 0
    }

    /// "functions, initializers and subscripts".
    pub fn describe(self) -> String {
        let names: Vec<_> = AttrTarget::ALL
            .iter()
            .filter(|&&t| self.contains(t))
            .map(|t| t.plural())
            .collect();
        match names.as_slice() {
            [] => String::new(),
            [one] => (*one).to_string(),
            [init @ .., last] => format!("{} and {last}", init.join(", ")),
        }
    }
}

/// The target kind of a declaration entity, or `None` for entities that
/// cannot carry attributes (modules, imports, accessors, ...).
pub fn attr_target(ctx: &QueryContext<'_>, entity: Entity) -> Option<AttrTarget> {
    let kind = ctx.get::<NodeKind>(entity)?;
    let parent_kind = ctx
        .parent_of(entity)
        .and_then(|p| ctx.get::<NodeKind>(p).cloned());
    Some(match kind {
        NodeKind::Struct => AttrTarget::Struct,
        NodeKind::Enum => AttrTarget::Enum,
        NodeKind::Protocol => AttrTarget::Protocol,
        NodeKind::EnumCase => AttrTarget::EnumCase,
        NodeKind::Extension => AttrTarget::Extension,
        NodeKind::Initializer => AttrTarget::Initializer,
        NodeKind::Deinit => AttrTarget::Deinit,
        NodeKind::Subscript => AttrTarget::Subscript,
        NodeKind::TypeAlias if parent_kind == Some(NodeKind::Protocol) => {
            AttrTarget::AssociatedType
        },
        NodeKind::TypeAlias => AttrTarget::TypeAlias,
        NodeKind::Function => match parent_kind {
            Some(NodeKind::Module) | None => AttrTarget::FreeFunction,
            Some(NodeKind::Protocol) if !ctx.has::<Valued>(entity) => AttrTarget::Requirement,
            _ => AttrTarget::Method,
        },
        NodeKind::Field => match ctx.get::<FieldClass>(entity) {
            Some(class) if class.owner == FieldOwner::Module => AttrTarget::ModuleValue,
            Some(class)
                if class.backing == FieldBacking::Computed
                    || class.owner == FieldOwner::Protocol =>
            {
                AttrTarget::ComputedProperty
            },
            _ => AttrTarget::StoredField,
        },
        NodeKind::Module
        | NodeKind::Setter
        | NodeKind::RefAccessor
        | NodeKind::Import
        | NodeKind::TypeParameter
        | NodeKind::ParamDefault => return None,
    })
}
