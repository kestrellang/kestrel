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
    /// Index into `MirBody.blocks`.
    BlockId
);
define_id!(
    /// Index into `MirBody.locals`.
    LocalId
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_id_round_trip() {
        let id = BlockId::new(5);
        assert_eq!(id.index(), 5);
        assert_eq!(id.raw(), 5);
    }

    #[test]
    fn local_id_round_trip() {
        let id = LocalId::new(42);
        assert_eq!(id.index(), 42);
    }

    #[test]
    fn ty_id_round_trip() {
        let id = TyId::new(0);
        assert_eq!(id.index(), 0);
        assert_eq!(id.raw(), 0);
    }

    #[test]
    fn mono_func_id_round_trip() {
        let id = MonoFuncId::new(100);
        assert_eq!(id.index(), 100);
    }

    #[test]
    fn field_idx_round_trip() {
        let idx = FieldIdx::new(3);
        assert_eq!(idx.index(), 3);
        assert_eq!(idx.raw(), 3);
    }

    #[test]
    fn variant_idx_round_trip() {
        let idx = VariantIdx::new(7);
        assert_eq!(idx.index(), 7);
    }

    #[test]
    fn id_equality() {
        assert_eq!(BlockId::new(0), BlockId::new(0));
        assert_ne!(BlockId::new(0), BlockId::new(1));
        assert_eq!(FieldIdx::new(0), FieldIdx::new(0));
        assert_ne!(FieldIdx::new(0), FieldIdx::new(1));
    }

    #[test]
    fn id_copy_semantics() {
        let a = LocalId::new(10);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn id_debug_format() {
        assert_eq!(format!("{:?}", BlockId::new(5)), "BlockId(5)");
        assert_eq!(format!("{:?}", LocalId::new(0)), "LocalId(0)");
        assert_eq!(format!("{:?}", TyId::new(3)), "TyId(3)");
        assert_eq!(format!("{:?}", MonoFuncId::new(1)), "MonoFuncId(1)");
        assert_eq!(format!("{:?}", FieldIdx::new(2)), "FieldIdx(2)");
        assert_eq!(format!("{:?}", VariantIdx::new(4)), "VariantIdx(4)");
    }

    #[test]
    fn id_hash_works() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(BlockId::new(0));
        set.insert(BlockId::new(1));
        set.insert(BlockId::new(0));
        assert_eq!(set.len(), 2);
    }
}
