pub mod enum_def;
pub mod function;
pub mod protocol;
pub mod static_def;
pub mod struct_def;
pub mod witness;

use kestrel_hecs::Entity;

use crate::layout::{EnumLayout, StructLayout};
use crate::{FieldIdx, VariantIdx};

pub use enum_def::{EnumCaseDef, EnumDef};
pub use function::{
    CallingConvention, ExternInfo, FunctionDef, FunctionKind, ParamDef, WhereClause,
    WhereConstraint,
};
pub use protocol::{AssociatedTypeDef, ProtocolDef, ProtocolMethodDef};
pub use static_def::{FileConstantData, StaticDef};
pub use struct_def::{FieldDef, StructDef};
pub use witness::{WitnessDef, WitnessMethodBinding, WitnessMethodKey};

#[derive(Debug, Clone, PartialEq)]
pub struct TypeParamDef {
    pub entity: Entity,
    pub name: String,
}

impl TypeParamDef {
    pub fn new(entity: Entity, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CopyBehavior {
    Bitwise,
    Clone(Entity),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DropBehavior {
    None,
    StructDrop {
        deinit: Option<Entity>,
        fields: Vec<FieldIdx>,
    },
    EnumDrop {
        deinit: Option<Entity>,
        variants: Vec<(VariantIdx, Vec<FieldIdx>)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub copy: CopyBehavior,
    pub drop: DropBehavior,
    pub layout: Option<Layout>,
    /// The function that clones this nominal: the synthesized `__clone$T` if one
    /// was generated, else the user's `clone()`. Written once by
    /// `passes::clone_shim::synthesize_clone_shims` — the pass that *decides*
    /// which it is — and read-only thereafter (mono collection, the expand
    /// pass's `CopyValue` lowering). `None` before that pass runs, and for a
    /// type that needs no clone at all.
    ///
    /// This is a **per-nominal** fact and deliberately NOT folded into
    /// `CopyBehavior::Clone`, which is per-*instantiation* and gets rewritten by
    /// `refine_mono_copy_behavior` (`Optional[String]` → Clone, `Optional[Int64]`
    /// → Bitwise, `Optional[File]` → None). All three instances share this one
    /// clone impl, so nesting it in the enum would drop it on the refined-away
    /// instances. The invariant is one-directional:
    /// `copy == Clone(_)` ⇒ `clone_impl.is_some()`, never the converse — a
    /// conditionally-Copyable container and a primitive-only struct both get a
    /// shim while keeping a `None`/`Bitwise` base.
    pub clone_impl: Option<Entity>,
    /// The synthesized `__drop$T` for this nominal, on the same terms as
    /// [`Self::clone_impl`]. Written once by
    /// `passes::drop_shim::synthesize_drop_shims`. Distinct from
    /// [`Self::drop`], which is the field-by-field *recipe*; this is the entry
    /// point that runs it.
    pub drop_impl: Option<Entity>,
}

impl TypeInfo {
    pub fn none() -> Self {
        Self {
            copy: CopyBehavior::Bitwise,
            drop: DropBehavior::None,
            layout: None,
            clone_impl: None,
            drop_impl: None,
        }
    }

    pub fn bitwise() -> Self {
        Self {
            copy: CopyBehavior::Bitwise,
            drop: DropBehavior::None,
            layout: None,
            clone_impl: None,
            drop_impl: None,
        }
    }
}

impl Default for TypeInfo {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    Struct(StructLayout),
    Enum(EnumLayout),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TargetConfig {
    pub pointer_width: u64,
}

impl TargetConfig {
    pub fn host_64() -> Self {
        Self { pointer_width: 8 }
    }
}
