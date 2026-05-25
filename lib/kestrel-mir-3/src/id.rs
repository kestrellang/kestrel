macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u32);

        impl $name {
            pub fn new(index: usize) -> Self {
                debug_assert!(index <= u32::MAX as usize, "{} index overflow: {index}", stringify!($name));
                Self(index as u32)
            }

            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn raw(self) -> u32 {
                self.0
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

macro_rules! define_idx {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(u16);

        impl $name {
            pub fn new(index: usize) -> Self {
                debug_assert!(index <= u16::MAX as usize, "{} index overflow: {index}", stringify!($name));
                Self(index as u16)
            }

            pub fn index(self) -> usize {
                self.0 as usize
            }

            pub fn raw(self) -> u16 {
                self.0
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
    /// Index into `TyArena.types`.
    TyId
);
define_id!(
    /// Index into `OssaBody.blocks`.
    BlockId
);
define_id!(
    /// SSA value identifier, unique per function body.
    ValueId
);
define_id!(
    /// Index into `MonoModule.functions`.
    MonoFuncId
);
define_id!(
    /// Index into `MirModule.functions`.
    FunctionIdx
);
define_id!(
    /// Index into `MirModule.structs`.
    StructIdx
);
define_id!(
    /// Index into `MirModule.enums`.
    EnumIdx
);
define_id!(
    /// Index into `MirModule.protocols`.
    ProtocolIdx
);
define_id!(
    /// Index into `MirModule.witnesses`.
    WitnessIdx
);
define_id!(
    /// Index into `MirModule.statics`.
    StaticIdx
);

define_idx!(
    /// Index into `StructDef.fields` or `EnumCaseDef.payload_fields`.
    FieldIdx
);
define_idx!(
    /// Index into `EnumDef.cases`.
    VariantIdx
);
