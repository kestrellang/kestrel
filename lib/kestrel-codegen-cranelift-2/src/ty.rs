use std::collections::HashMap;

use cranelift_codegen::ir;
use kestrel_hecs::Entity;
use kestrel_mir_2::{
    FloatBits, IntBits, Layout, MirTy, MonoModule, StructLayout, TyArena, TyId,
};

/// How a type is represented at the Cranelift level.
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

enum LayoutEntry {
    Struct(usize),
    Enum(usize),
}

/// Cached type classifications for an entire MonoModule.
pub struct TypeCache {
    reprs: Vec<Option<TypeRepr>>,
    layout_map: HashMap<(Entity, Vec<TyId>), LayoutEntry>,
    pub ptr_ty: ir::Type,
    pub ptr_size: u64,
}

impl TypeCache {
    pub fn new(module: &MonoModule, ptr_ty: ir::Type, ptr_size: u64) -> Self {
        let mut layout_map = HashMap::new();
        for (i, s) in module.structs.iter().enumerate() {
            layout_map.insert((s.source, s.type_args.clone()), LayoutEntry::Struct(i));
        }
        for (i, e) in module.enums.iter().enumerate() {
            layout_map.insert((e.source, e.type_args.clone()), LayoutEntry::Enum(i));
        }

        Self {
            reprs: vec![None; module.ty_arena.len()],
            layout_map,
            ptr_ty,
            ptr_size,
        }
    }

    /// Get the TypeRepr for a TyId, computing and caching it if needed.
    pub fn repr(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> TypeRepr {
        if let Some(cached) = self.reprs[ty.index()] {
            return cached;
        }
        let repr = self.classify(ty, arena, module);
        self.reprs[ty.index()] = Some(repr);
        repr
    }

    /// The Cranelift ir::Type to use for a Variable holding this type.
    /// Scalars use their native type; aggregates use ptr_ty (they hold a pointer).
    pub fn cl_type(&mut self, ty: TyId, arena: &TyArena, module: &MonoModule) -> ir::Type {
        match self.repr(ty, arena, module) {
            TypeRepr::Scalar(t) => t,
            TypeRepr::Aggregate { .. } | TypeRepr::Zst => self.ptr_ty,
        }
    }

    pub fn reprs_len(&self) -> usize {
        self.reprs.len()
    }

    pub fn peek(&self, ty: TyId) -> Option<TypeRepr> {
        self.reprs.get(ty.index()).and_then(|r| *r)
    }

    /// O(1) lookup: find the MonoStruct index for a Named type.
    pub fn find_struct_idx(&self, entity: Entity, type_args: &[TyId]) -> Option<usize> {
        let key = (entity, type_args.to_vec());
        match self.layout_map.get(&key) {
            Some(LayoutEntry::Struct(idx)) => Some(*idx),
            _ => None,
        }
    }

    /// O(1) lookup: find the MonoEnum index for a Named type.
    pub fn find_enum_idx(&self, entity: Entity, type_args: &[TyId]) -> Option<usize> {
        let key = (entity, type_args.to_vec());
        match self.layout_map.get(&key) {
            Some(LayoutEntry::Enum(idx)) => Some(*idx),
            _ => None,
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

            MirTy::Pointer(_) | MirTy::FuncThin { .. } => TypeRepr::Scalar(ptr_ty),

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
            }

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
                self.classify_named(entity, &type_args, module)
            }

            MirTy::Error => TypeRepr::Scalar(ir::types::I8),

            // Post-mono: these should never appear
            MirTy::TypeParam(_) | MirTy::AssociatedProjection { .. } => {
                debug_assert!(false, "unresolved generic type in codegen: {:?}", arena.get(ty));
                TypeRepr::Scalar(ptr_ty)
            }
        }
    }

    fn classify_named(
        &self,
        entity: Entity,
        type_args: &[TyId],
        module: &MonoModule,
    ) -> TypeRepr {
        let key = (entity, type_args.to_vec());
        let Some(entry) = self.layout_map.get(&key) else {
            return TypeRepr::Scalar(self.ptr_ty);
        };

        let (layout, is_single_field) = match entry {
            LayoutEntry::Struct(idx) => {
                let s = &module.structs[*idx];
                (s.type_info.layout.as_ref(), s.fields.len() <= 1)
            }
            LayoutEntry::Enum(idx) => {
                let e = &module.enums[*idx];
                // Pure-discriminant enum: no variant has payload fields
                let pure_disc = e.cases.iter().all(|c| c.payload_fields.is_empty());
                (e.type_info.layout.as_ref(), pure_disc)
            }
        };

        let Some(layout) = layout else {
            return TypeRepr::Scalar(self.ptr_ty);
        };

        let (size, align) = match layout {
            Layout::Struct(sl) => (sl.size, sl.align),
            Layout::Enum(el) => (el.size, el.align),
        };

        if size == 0 {
            return TypeRepr::Zst;
        }

        // Only promote to scalar for single-field newtypes and pure-discriminant enums.
        // Multi-field structs and payload enums stay as aggregates so field
        // projections can compute offsets from a pointer.
        if is_single_field && size <= 8 {
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
