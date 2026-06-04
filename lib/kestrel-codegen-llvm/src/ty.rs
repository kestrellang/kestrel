//! MIR type -> LLVM scalar/aggregate classification.
//!
//! Faithful port of the Cranelift backend's `ty.rs`. The model is "scalar or
//! memory": a value is either a machine scalar (held in an LLVM SSA value) or an
//! aggregate (held by pointer to memory). Layouts are NOT recomputed here — they
//! come from the MIR layout pass (`s.type_info.layout`), the single source of
//! truth. Like the Cranelift backend, a single-field newtype delegates to its
//! field's representation (so `Float64` is an `f64`, not an `i64`).
//!
//! Pointer-width scalars are represented as the integer `ScalarTy::I64`/`I32`
//! (NOT an LLVM `ptr`), exactly mirroring Cranelift's `ptr_ty = I64`. The `ptr`
//! type only materialises at memory-access/call boundaries (see `mem`), keeping
//! all arithmetic, ABI, and offset logic identical to the Cranelift backend.

use inkwell::context::Context;
use inkwell::types::BasicTypeEnum;
use kestrel_hecs::Entity;
use kestrel_mir::mono::MonoModule;
use kestrel_mir::{FloatBits, IntBits, Layout, MirTy, StructLayout, TyArena, TyId};

/// A machine scalar type, independent of any LLVM context lifetime (Copy, like
/// Cranelift's `ir::Type`). Pointer-width scalars use `I64`/`I32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalarTy {
    I8,
    I16,
    I32,
    I64,
    F16,
    F32,
    F64,
}

impl ScalarTy {
    pub fn bytes(self) -> u64 {
        match self {
            Self::I8 => 1,
            Self::I16 | Self::F16 => 2,
            Self::I32 | Self::F32 => 4,
            Self::I64 | Self::F64 => 8,
        }
    }

    pub fn is_int(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    pub fn is_float(self) -> bool {
        !self.is_int()
    }

    /// Materialise the inkwell type for this scalar.
    pub fn llvm<'ctx>(self, cx: &'ctx Context) -> BasicTypeEnum<'ctx> {
        match self {
            Self::I8 => cx.i8_type().into(),
            Self::I16 => cx.i16_type().into(),
            Self::I32 => cx.i32_type().into(),
            Self::I64 => cx.i64_type().into(),
            Self::F16 => cx.f16_type().into(),
            Self::F32 => cx.f32_type().into(),
            Self::F64 => cx.f64_type().into(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TypeRepr {
    Scalar(ScalarTy),
    Aggregate { size: u64, align: u64 },
    Zst,
}

impl TypeRepr {
    pub fn is_scalar(&self) -> bool {
        matches!(self, Self::Scalar(_))
    }

    pub fn is_aggregate(&self) -> bool {
        matches!(self, Self::Aggregate { .. })
    }

    pub fn is_zst(&self) -> bool {
        matches!(self, Self::Zst)
    }

    pub fn size(&self) -> u64 {
        match self {
            Self::Scalar(t) => t.bytes(),
            Self::Aggregate { size, .. } => *size,
            Self::Zst => 0,
        }
    }

    pub fn align(&self) -> u64 {
        match self {
            Self::Scalar(t) => t.bytes(),
            Self::Aggregate { align, .. } => *align,
            Self::Zst => 1,
        }
    }
}

pub struct TypeCache {
    reprs: Vec<Option<TypeRepr>>,
    pub ptr_scalar: ScalarTy,
    pub ptr_size: u64,
}

impl TypeCache {
    pub fn new(module: &MonoModule, ptr_size: u64) -> Self {
        let ptr_scalar = if ptr_size == 8 {
            ScalarTy::I64
        } else {
            ScalarTy::I32
        };
        Self {
            reprs: vec![None; module.ty_arena.len()],
            ptr_scalar,
            ptr_size,
        }
    }

    pub fn cached_repr(&self, ty: TyId) -> Option<TypeRepr> {
        self.reprs.get(ty.index()).copied().flatten()
    }

    pub fn repr(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> TypeRepr {
        if let Some(cached) = self.reprs[ty.index()] {
            return cached;
        }
        let repr = self.classify(ty, arena, module);
        self.reprs[ty.index()] = Some(repr);
        repr
    }

    /// The scalar type a *value* of this MIR type is carried as. Aggregates and
    /// ZSTs are carried by pointer, i.e. as the pointer-width integer scalar.
    pub fn value_scalar(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> ScalarTy {
        match self.repr(ty, arena, module) {
            TypeRepr::Scalar(t) => t,
            TypeRepr::Aggregate { .. } | TypeRepr::Zst => self.ptr_scalar,
        }
    }

    fn classify(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> TypeRepr {
        let ptr_scalar = self.ptr_scalar;
        let ptr_size = self.ptr_size;

        match arena.get(ty) {
            MirTy::I8 | MirTy::Bool => TypeRepr::Scalar(ScalarTy::I8),
            MirTy::I16 => TypeRepr::Scalar(ScalarTy::I16),
            MirTy::I32 => TypeRepr::Scalar(ScalarTy::I32),
            MirTy::I64 => TypeRepr::Scalar(ScalarTy::I64),
            MirTy::F16 => TypeRepr::Scalar(ScalarTy::F16),
            MirTy::F32 => TypeRepr::Scalar(ScalarTy::F32),
            MirTy::F64 => TypeRepr::Scalar(ScalarTy::F64),

            MirTy::Pointer(_) | MirTy::FuncThin { .. } => TypeRepr::Scalar(ptr_scalar),

            MirTy::Never => TypeRepr::Zst,

            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                if elems.is_empty() {
                    return TypeRepr::Zst;
                }
                let mut layout = StructLayout::new();
                for &elem in &elems {
                    let elem_repr = self.repr(elem, arena, module);
                    layout.append_field(StructLayout::scalar(elem_repr.size(), elem_repr.align()));
                }
                layout.pad_to_align();
                if layout.size == 0 {
                    TypeRepr::Zst
                } else {
                    TypeRepr::Aggregate {
                        size: layout.size,
                        align: layout.align,
                    }
                }
            },

            MirTy::Str => TypeRepr::Aggregate {
                size: ptr_size * 2,
                align: ptr_size,
            },

            MirTy::FuncThick { .. } => TypeRepr::Aggregate {
                size: ptr_size * 2,
                align: ptr_size,
            },

            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                let type_args = type_args.clone();
                self.classify_named(entity, &type_args, arena, module)
            },

            MirTy::Error => TypeRepr::Scalar(ScalarTy::I8),

            MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => {
                debug_assert!(
                    false,
                    "unresolved generic type in codegen: {:?}",
                    arena.get(ty)
                );
                TypeRepr::Scalar(ptr_scalar)
            },
        }
    }

    fn classify_named(
        &mut self,
        entity: Entity,
        type_args: &[TyId],
        arena: &TyArena,
        module: &MonoModule,
    ) -> TypeRepr {
        let key = (entity, type_args.to_vec());

        // `is_single_field`: this Named type is carried as a single scalar — a
        // one-field struct (newtype) or a pure-discriminant enum.
        // `single_field_ty`: the field type of a one-field struct. A newtype's
        // value *is* its field's value, so its representation delegates to the
        // field's repr (the single source of truth — see the collapse below).
        let (layout, is_single_field, single_field_ty) = if let Some(s) = module.structs.get(&key) {
            let single_field_ty = (s.fields.len() == 1).then(|| s.fields[0].ty);
            (
                s.type_info.layout.as_ref(),
                s.fields.len() <= 1,
                single_field_ty,
            )
        } else if let Some(e) = module.enums.get(&key) {
            let pure_disc = e.cases.iter().all(|c| c.payload_fields.is_empty());
            (e.type_info.layout.as_ref(), pure_disc, None)
        } else {
            return TypeRepr::Scalar(self.ptr_scalar);
        };

        let Some(layout) = layout else {
            return TypeRepr::Scalar(self.ptr_scalar);
        };

        let (size, align) = match layout {
            Layout::Struct(sl) => (sl.size, sl.align),
            Layout::Enum(el) => (el.size, el.align),
        };

        if size == 0 {
            return TypeRepr::Zst;
        }

        if is_single_field && size <= 8 {
            // A single-field newtype's value is exactly its field's value, so its
            // representation must be the field's representation. Collapsing by
            // byte size alone would mis-type e.g. `Float64` (an f64 newtype) as
            // I64 while the body carries an f64 — making the auto clone-shim's
            // signature disagree with its body. Delegating keeps layout single-
            // sourced. Pure-discriminant enums and one-field structs over a
            // non-scalar field fall through to the integer-by-size mapping below.
            if let Some(field_ty) = single_field_ty {
                if let TypeRepr::Scalar(t) = self.repr(field_ty, arena, module) {
                    return TypeRepr::Scalar(t);
                }
            }
            let scalar = match size {
                1 => ScalarTy::I8,
                2 => ScalarTy::I16,
                3..=4 => ScalarTy::I32,
                _ => ScalarTy::I64,
            };
            return TypeRepr::Scalar(scalar);
        }

        TypeRepr::Aggregate { size, align }
    }
}

pub fn int_bits_to_scalar(bits: IntBits) -> ScalarTy {
    match bits {
        IntBits::I8 => ScalarTy::I8,
        IntBits::I16 => ScalarTy::I16,
        IntBits::I32 => ScalarTy::I32,
        IntBits::I64 => ScalarTy::I64,
    }
}

pub fn float_bits_to_scalar(bits: FloatBits) -> ScalarTy {
    match bits {
        FloatBits::F16 => ScalarTy::F16,
        FloatBits::F32 => ScalarTy::F32,
        FloatBits::F64 => ScalarTy::F64,
    }
}
