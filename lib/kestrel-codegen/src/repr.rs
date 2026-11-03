//! MIR type → machine representation ("scalar or memory").
//!
//! A value is either a machine scalar (held in an SSA value) or an aggregate
//! (held by pointer to memory). Layouts are NOT recomputed here — they come
//! from the MIR layout pass (`type_info.layout`), the single source of truth.
//! A single-field newtype delegates to its field's representation (so
//! `Float64` is an `f64`, not an `i64`, and `Pointer[T]` is the pointer scalar).

use kestrel_debug::ktrace;
use kestrel_hecs::Entity;
use kestrel_mir::mono::MonoModule;
use kestrel_mir::{FloatBits, IntBits, Layout, MirTy, StructLayout, TyArena, TyId};

/// A backend's machine scalars (Cranelift `ir::Type`, LLVM `ScalarTy`),
/// implemented by a marker type in each backend crate. A marker (rather than
/// a trait on the scalar type itself) lets Cranelift plug in `ir::Type`, which
/// it does not own.
///
/// The pointer scalar is not built through this trait: it is target-dependent
/// and backend-specific, so [`TypeCache::new`] takes it as a value.
pub trait ScalarModel: Copy + std::fmt::Debug {
    type Scalar: Copy + std::fmt::Debug;
    fn from_int_bits(bits: IntBits) -> Self::Scalar;
    fn from_float_bits(bits: FloatBits) -> Self::Scalar;
    /// Size (and alignment) in bytes.
    fn size_bytes(scalar: Self::Scalar) -> u64;
}

#[derive(Debug, Clone, Copy)]
pub enum TypeRepr<M: ScalarModel> {
    Scalar(M::Scalar),
    Aggregate { size: u64, align: u64 },
    Zst,
}

impl<M: ScalarModel> TypeRepr<M> {
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
            Self::Scalar(t) => M::size_bytes(*t),
            Self::Aggregate { size, .. } => *size,
            Self::Zst => 0,
        }
    }

    pub fn align(&self) -> u64 {
        match self {
            Self::Scalar(t) => M::size_bytes(*t),
            Self::Aggregate { align, .. } => *align,
            Self::Zst => 1,
        }
    }
}

/// Memoized [`TypeRepr`] per `TyId` of one mono module.
pub struct TypeCache<M: ScalarModel> {
    reprs: Vec<Option<TypeRepr<M>>>,
    /// The scalar pointer-width values travel as (addresses, aggregate
    /// references, `Pointer`/`FuncThin`, function pointers).
    pub ptr_scalar: M::Scalar,
    pub ptr_size: u64,
}

impl<M: ScalarModel> TypeCache<M> {
    pub fn new(module: &MonoModule, ptr_scalar: M::Scalar, ptr_size: u64) -> Self {
        Self {
            reprs: vec![None; module.ty_arena.len()],
            ptr_scalar,
            ptr_size,
        }
    }

    pub fn cached_repr(&self, ty: TyId) -> Option<TypeRepr<M>> {
        self.reprs.get(ty.index()).copied().flatten()
    }

    pub fn repr(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> TypeRepr<M> {
        if let Some(cached) = self.reprs[ty.index()] {
            return cached;
        }
        let repr = self.classify(ty, arena, module);
        self.reprs[ty.index()] = Some(repr);
        repr
    }

    /// The scalar a *value* of this MIR type is carried as. Aggregates and
    /// ZSTs are carried by pointer, i.e. as the pointer scalar.
    pub fn value_scalar(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> M::Scalar {
        match self.repr(ty, arena, module) {
            TypeRepr::Scalar(t) => t,
            TypeRepr::Aggregate { .. } | TypeRepr::Zst => self.ptr_scalar,
        }
    }

    fn classify(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> TypeRepr<M> {
        let ptr_scalar = self.ptr_scalar;
        let ptr_size = self.ptr_size;
        let two_words = TypeRepr::Aggregate {
            size: ptr_size * 2,
            align: ptr_size,
        };

        // A pointer/ref to an unsized type, or a borrowed raw-existential
        // value: (data, table), two words.
        if module.is_fat(ty) {
            return two_words;
        }
        match arena.get(ty) {
            MirTy::I8 | MirTy::Bool => TypeRepr::Scalar(M::from_int_bits(IntBits::I8)),
            MirTy::I16 => TypeRepr::Scalar(M::from_int_bits(IntBits::I16)),
            MirTy::I32 => TypeRepr::Scalar(M::from_int_bits(IntBits::I32)),
            MirTy::I64 => TypeRepr::Scalar(M::from_int_bits(IntBits::I64)),
            MirTy::F16 => TypeRepr::Scalar(M::from_float_bits(FloatBits::F16)),
            MirTy::F32 => TypeRepr::Scalar(M::from_float_bits(FloatBits::F32)),
            MirTy::F64 => TypeRepr::Scalar(M::from_float_bits(FloatBits::F64)),

            // Ref appears on ret_borrow signatures only (never a value type);
            // its ABI is a pointer to the pointee.
            MirTy::Pointer(_) | MirTy::FuncThin { .. } | MirTy::Ref { .. } => {
                TypeRepr::Scalar(ptr_scalar)
            },

            MirTy::Never => TypeRepr::Zst,

            // Always fat (handled above); kept for exhaustiveness.
            MirTy::RawExistential { .. } => two_words,

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

            MirTy::Str => two_words,

            // `{fn, env}` at the view kinds; `{fn, env_handle, retain_fn,
            // release_fn}` at the boxed (escaping) kind — see
            // `kestrel_mir::ty::func_thick_words` (lockstep 9).
            MirTy::FuncThick { kind, .. } => TypeRepr::Aggregate {
                size: ptr_size * kestrel_mir::func_thick_words(*kind),
                align: ptr_size,
            },

            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                let type_args = type_args.clone();
                self.classify_named(entity, &type_args, arena, module)
            },

            MirTy::Error => TypeRepr::Scalar(M::from_int_bits(IntBits::I8)),

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
    ) -> TypeRepr<M> {
        let key = (entity, type_args.to_vec());

        // `is_single_field`: this Named type has at most one runtime component —
        // a one-field struct (newtype) or a pure-discriminant enum. It is
        // carried as a single scalar only when that component is itself scalar.
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
            ktrace!(
                "clone-shim",
                "[classify_named] MISSING layout for {} entity={entity:?} type_args={type_args:?} → Scalar fallback",
                entity_name(module, entity)
            );
            return TypeRepr::Scalar(self.ptr_scalar);
        };

        let Some(layout) = layout else {
            ktrace!(
                "clone-shim",
                "[classify_named] NO LAYOUT for {} entity={entity:?} → Scalar fallback",
                entity_name(module, entity)
            );
            return TypeRepr::Scalar(self.ptr_scalar);
        };

        let (size, align) = match layout {
            Layout::Struct(sl) => (sl.size, sl.align),
            Layout::Enum(el) => (el.size, el.align),
        };

        if size == 0 {
            return TypeRepr::Zst;
        }

        if !is_single_field || size > 8 {
            return TypeRepr::Aggregate { size, align };
        }

        // A single-field newtype's value is exactly its field's value, so its
        // representation must be the field's representation. Collapsing by
        // byte size alone would mis-type e.g. `Float64` (an f64 newtype) as I64
        // while the body carries an f64 — making the auto clone-shim's
        // signature disagree with its body.
        if let Some(field_ty) = single_field_ty {
            // Delegate to a SCALAR field (Float64 -> f64, Pointer[T] -> ptr).
            // A newtype over an AGGREGATE field is itself carried by address:
            // collapsing it to an integer would mismatch its by-memory clone/
            // construction (the body builds a slot and returns its `ptr`, which
            // is not an integer scalar) and every by-repr copy would move 8
            // bytes of ADDRESS instead of contents. See IoError (a newtype over
            // a payload-carrying enum).
            // Only when the layout is the field's own: `@align` or
            // `@layout(.Packed)` give the newtype a different size or
            // alignment, so it stays in memory.
            if let TypeRepr::Scalar(t) = self.repr(field_ty, arena, module)
                && M::size_bytes(t) == size
                && M::size_bytes(t) == align
            {
                return TypeRepr::Scalar(t);
            }
            return TypeRepr::Aggregate { size, align };
        }

        // Pure-discriminant enum: its discriminant, unless `@align` padded it.
        if let Some(e) = module.enums.get(&key) {
            let t = M::from_int_bits(e.discriminant_width);
            if M::size_bytes(t) == size && M::size_bytes(t) == align {
                return TypeRepr::Scalar(t);
            }
        }
        TypeRepr::Aggregate { size, align }
    }
}

fn entity_name(module: &MonoModule, entity: Entity) -> &str {
    module
        .entity_names
        .get(&entity)
        .map(|s| s.as_str())
        .unwrap_or("?")
}
