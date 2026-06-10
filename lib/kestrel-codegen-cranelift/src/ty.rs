use cranelift_codegen::ir;
use kestrel_hecs::Entity;
use kestrel_mir::mono::MonoModule;
use kestrel_mir::{FloatBits, IntBits, Layout, MirTy, StructLayout, TyArena, TyId};

#[derive(Debug, Clone, Copy)]
pub enum TypeRepr {
    Scalar(ir::Type),
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
            Self::Scalar(t) => t.bytes() as u64,
            Self::Aggregate { size, .. } => *size,
            Self::Zst => 0,
        }
    }

    pub fn align(&self) -> u64 {
        match self {
            Self::Scalar(t) => t.bytes() as u64,
            Self::Aggregate { align, .. } => *align,
            Self::Zst => 1,
        }
    }
}

pub struct TypeCache {
    reprs: Vec<Option<TypeRepr>>,
    pub ptr_ty: ir::Type,
    pub ptr_size: u64,
}

impl TypeCache {
    pub fn new(module: &MonoModule, ptr_ty: ir::Type, ptr_size: u64) -> Self {
        Self {
            reprs: vec![None; module.ty_arena.len()],
            ptr_ty,
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

    pub fn cl_type(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> ir::Type {
        match self.repr(ty, arena, module) {
            TypeRepr::Scalar(t) => t,
            TypeRepr::Aggregate { .. } | TypeRepr::Zst => self.ptr_ty,
        }
    }

    fn classify(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> TypeRepr {
        let ptr_ty = self.ptr_ty;
        let ptr_size = self.ptr_size;

        match arena.get(ty) {
            MirTy::I8 | MirTy::Bool => TypeRepr::Scalar(ir::types::I8),
            MirTy::I16 => TypeRepr::Scalar(ir::types::I16),
            MirTy::I32 => TypeRepr::Scalar(ir::types::I32),
            MirTy::I64 => TypeRepr::Scalar(ir::types::I64),
            MirTy::F16 => TypeRepr::Scalar(ir::types::F16),
            MirTy::F32 => TypeRepr::Scalar(ir::types::F32),
            MirTy::F64 => TypeRepr::Scalar(ir::types::F64),

            // Ref appears on ret_borrow signatures only (never a value type);
            // its ABI is a raw pointer to the pointee.
            MirTy::Pointer(_) | MirTy::FuncThin { .. } | MirTy::Ref { .. } => {
                TypeRepr::Scalar(ptr_ty)
            },

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

            MirTy::Error => TypeRepr::Scalar(ir::types::I8),

            MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => {
                debug_assert!(
                    false,
                    "unresolved generic type in codegen: {:?}",
                    arena.get(ty)
                );
                TypeRepr::Scalar(ptr_ty)
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

        // `is_single_field`: this Named type is carried as a single scalar — a one-field
        // struct (newtype) or a pure-discriminant enum.
        // `single_field_ty`: the field type of a one-field struct, when applicable. A
        // newtype's value *is* its field's value, so its representation must delegate to
        // the field's repr (the single source of truth — see the collapse branch below).
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
            let name = module
                .entity_names
                .get(&entity)
                .map(|s| s.as_str())
                .unwrap_or("?");
            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                eprintln!(
                    "[classify_named] MISSING layout for {name} entity={entity:?} type_args={type_args:?} → Scalar fallback"
                );
            }
            return TypeRepr::Scalar(self.ptr_ty);
        };

        let Some(layout) = layout else {
            let name = module
                .entity_names
                .get(&entity)
                .map(|s| s.as_str())
                .unwrap_or("?");
            if std::env::var("KESTREL_DEBUG_CLONE").is_ok() {
                eprintln!(
                    "[classify_named] NO LAYOUT for {name} entity={entity:?} → Scalar fallback"
                );
            }
            return TypeRepr::Scalar(self.ptr_ty);
        };

        let (size, align) = match layout {
            Layout::Struct(sl) => (sl.size, sl.align),
            Layout::Enum(el) => (el.size, el.align),
        };

        if size == 0 {
            return TypeRepr::Zst;
        }

        if is_single_field && size <= 8 {
            // A single-field newtype's value is exactly its field's value (see
            // `compile_struct` / `compile_struct_extract` in inst.rs), so its
            // representation must be the field's representation. Collapsing by byte size
            // alone would mis-type e.g. `Float64` (an f64 newtype) as I64 while the body
            // carries an f64 — making the auto clone-shim's signature disagree with its
            // body and fail Cranelift verification. Delegating keeps layout single-
            // sourced. Pure-discriminant enums and one-field structs over a non-scalar
            // field fall through to the integer-by-size mapping below.
            if let Some(field_ty) = single_field_ty
                && let TypeRepr::Scalar(t) = self.repr(field_ty, arena, module)
            {
                return TypeRepr::Scalar(t);
            }
            let cl_ty = match size {
                1 => ir::types::I8,
                2 => ir::types::I16,
                3..=4 => ir::types::I32,
                _ => ir::types::I64,
            };
            return TypeRepr::Scalar(cl_ty);
        }

        TypeRepr::Aggregate { size, align }
    }
}

pub fn int_bits_to_cl(bits: IntBits) -> ir::Type {
    match bits {
        IntBits::I8 => ir::types::I8,
        IntBits::I16 => ir::types::I16,
        IntBits::I32 => ir::types::I32,
        IntBits::I64 => ir::types::I64,
    }
}

pub fn float_bits_to_cl(bits: FloatBits) -> ir::Type {
    match bits {
        FloatBits::F16 => ir::types::F16,
        FloatBits::F32 => ir::types::F32,
        FloatBits::F64 => ir::types::F64,
    }
}
