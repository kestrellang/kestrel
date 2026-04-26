//! Newtype IDs for MIR items.
//!
//! Each ID is a u32 index into the corresponding Vec in MirModule or MirBody.

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u32);

        impl $name {
            pub fn new(index: usize) -> Self {
                Self(index as u32)
            }

            pub fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(
    /// Index into `MirModule.functions`.
    FunctionId
);
define_id!(
    /// Index into `MirModule.structs`.
    StructId
);
define_id!(
    /// Index into `MirModule.enums`.
    EnumId
);
define_id!(
    /// Index into `MirModule.protocols`.
    ProtocolId
);
define_id!(
    /// Index into `MirModule.witnesses`.
    WitnessId
);
define_id!(
    /// Index into `MirModule.statics`.
    StaticId
);
define_id!(
    /// Index into `MirModule.closures`.
    ClosureId
);
define_id!(
    /// Index into `StructDef.fields`.
    FieldId
);
define_id!(
    /// Index into `MirBody.blocks`.
    BlockId
);
define_id!(
    /// Index into `MirBody.locals`.
    LocalId
);
