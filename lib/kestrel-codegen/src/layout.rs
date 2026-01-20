//! Type layout calculation.
//!
//! Computes size and alignment for MIR types based on the target configuration.

use crate::TargetConfig;
use kestrel_execution_graph::{Id, MirContext, MirTy, Struct, Ty};
use std::collections::HashMap;

/// Memory layout information for a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    /// Size in bytes.
    pub size: usize,
    /// Alignment in bytes.
    pub align: usize,
}

impl Layout {
    /// Create a new layout.
    pub fn new(size: usize, align: usize) -> Self {
        Self { size, align }
    }

    /// Layout with zero size but specified alignment.
    pub fn zero(align: usize) -> Self {
        Self { size: 0, align }
    }

    /// Round size up to alignment.
    pub fn pad_to_align(self) -> Self {
        let padded = (self.size + self.align - 1) & !(self.align - 1);
        Self {
            size: padded,
            align: self.align,
        }
    }

    /// Compute offset for appending a field with the given layout.
    /// Returns (offset, new_total_layout).
    pub fn append(self, field: Layout) -> (usize, Layout) {
        let offset = (self.size + field.align - 1) & !(field.align - 1);
        let new_size = offset + field.size;
        let new_align = self.align.max(field.align);
        (offset, Layout::new(new_size, new_align))
    }
}

/// Cache for computed type layouts.
pub struct LayoutCache<'a> {
    ctx: &'a MirContext,
    target: &'a TargetConfig,
    cache: HashMap<Id<Ty>, Layout>,
    struct_layouts: HashMap<Id<Struct>, StructLayout>,
}

/// Layout information for a struct including field offsets.
#[derive(Debug, Clone)]
pub struct StructLayout {
    /// Overall struct layout.
    pub layout: Layout,
    /// Offset of each field by name.
    pub field_offsets: HashMap<String, usize>,
}

impl<'a> LayoutCache<'a> {
    /// Create a new layout cache.
    pub fn new(ctx: &'a MirContext, target: &'a TargetConfig) -> Self {
        Self {
            ctx,
            target,
            cache: HashMap::new(),
            struct_layouts: HashMap::new(),
        }
    }

    /// Get the layout of a type.
    pub fn layout_of(&mut self, ty: Id<Ty>) -> Layout {
        if let Some(&layout) = self.cache.get(&ty) {
            return layout;
        }

        let layout = self.compute_layout(ty);
        self.cache.insert(ty, layout);
        layout
    }

    /// Get the layout of a struct with field offsets.
    pub fn struct_layout(&mut self, struct_id: Id<Struct>) -> &StructLayout {
        if !self.struct_layouts.contains_key(&struct_id) {
            let layout = self.compute_struct_layout(struct_id);
            self.struct_layouts.insert(struct_id, layout);
        }
        &self.struct_layouts[&struct_id]
    }

    /// Compute the layout of a type.
    fn compute_layout(&mut self, ty: Id<Ty>) -> Layout {
        let ptr_size = self.target.pointer_size();

        match self.ctx.ty(ty) {
            // Primitives
            MirTy::I8 => Layout::new(1, 1),
            MirTy::I16 => Layout::new(2, 2),
            MirTy::I32 => Layout::new(4, 4),
            MirTy::I64 => Layout::new(8, 8),
            MirTy::F16 => Layout::new(2, 2),
            MirTy::F32 => Layout::new(4, 4),
            MirTy::F64 => Layout::new(8, 8),
            MirTy::Bool => Layout::new(1, 1),
            MirTy::Unit => Layout::zero(1),
            MirTy::Never => Layout::zero(1),

            // String is a fat pointer: { ptr, len }
            MirTy::Str => Layout::new(ptr_size * 2, ptr_size),

            // Pointers and references are pointer-sized
            MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => Layout::new(ptr_size, ptr_size),

            // Array is thin pointer (for now)
            MirTy::Array(_) => Layout::new(ptr_size, ptr_size),

            // Tuple: lay out fields sequentially
            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                let mut layout = Layout::zero(1);
                for elem in elems {
                    let field_layout = self.layout_of(elem);
                    (_, layout) = layout.append(field_layout);
                }
                layout.pad_to_align()
            }

            // Named types need struct/enum lookup
            MirTy::Named { name, type_args: _ } => {
                // Look up struct by name
                let name_data = self.ctx.name(*name);
                for (id, def) in self.ctx.structs.iter() {
                    let def_name = self.ctx.name(def.name);
                    if def_name == name_data {
                        let struct_layout = self.compute_struct_layout(id);
                        return struct_layout.layout;
                    }
                }
                // Check enums
                for (id, def) in self.ctx.enums.iter() {
                    let def_name = self.ctx.name(def.name);
                    if def_name == name_data {
                        return self.compute_enum_layout(id);
                    }
                }
                // Unknown named type - use pointer size as fallback
                Layout::new(ptr_size, ptr_size)
            }

            // Type parameters are resolved at monomorphization
            MirTy::TypeParam(_) => Layout::new(ptr_size, ptr_size),

            // Function pointers
            MirTy::FuncThin { .. } => Layout::new(ptr_size, ptr_size),
            // Thick callable: function pointer + environment pointer
            MirTy::FuncThick { .. } => Layout::new(ptr_size * 2, ptr_size),

            // Self type - resolved at monomorphization
            MirTy::SelfType => Layout::new(ptr_size, ptr_size),

            // Associated type projection - resolved at monomorphization
            MirTy::AssociatedTypeProjection { .. } => Layout::new(ptr_size, ptr_size),

            // Error type
            MirTy::Error => Layout::zero(1),
        }
    }

    /// Compute layout for a struct.
    fn compute_struct_layout(&mut self, struct_id: Id<Struct>) -> StructLayout {
        let struct_def = self.ctx.struct_def(struct_id);
        // Clone the field IDs to avoid borrowing issues
        let field_ids: Vec<_> = struct_def.fields.clone();

        let mut layout = Layout::zero(1);
        let mut field_offsets = HashMap::new();

        for field_id in field_ids {
            let field_def = &self.ctx.fields[field_id];
            let field_name = field_def.name.clone();
            let field_layout = self.layout_of(field_def.ty);
            let offset;
            (offset, layout) = layout.append(field_layout);
            field_offsets.insert(field_name, offset);
        }

        StructLayout {
            layout: layout.pad_to_align(),
            field_offsets,
        }
    }

    /// Compute layout for an enum (tagged union).
    fn compute_enum_layout(&mut self, enum_id: Id<kestrel_execution_graph::Enum>) -> Layout {
        let enum_def = self.ctx.enum_def(enum_id);
        // Clone case IDs to avoid borrowing issues
        let case_ids: Vec<_> = enum_def.cases.clone();

        // Discriminant is i32
        let discriminant_layout = Layout::new(4, 4);

        // Find max payload size
        let mut max_payload = Layout::zero(1);
        for case_id in case_ids {
            let case_def = &self.ctx.enum_cases[case_id];
            // Each case has an associated payload struct
            // First try the direct struct_def reference if available
            if let Some(struct_id) = case_def.struct_def {
                let payload_layout = self.compute_struct_layout(struct_id).layout;
                if payload_layout.size > max_payload.size {
                    max_payload = payload_layout;
                }
            } else {
                // Otherwise look up by name
                let case_name = self.ctx.name(case_def.struct_name);
                for (id, def) in self.ctx.structs.iter() {
                    if self.ctx.name(def.name) == case_name {
                        let payload_layout = self.compute_struct_layout(id).layout;
                        if payload_layout.size > max_payload.size {
                            max_payload = payload_layout;
                        }
                        break;
                    }
                }
            }
        }

        // Enum = discriminant + max(payload sizes)
        let (_, layout) = discriminant_layout.append(max_payload);
        layout.pad_to_align()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_layouts() {
        let mut ctx = MirContext::new();
        let target = TargetConfig::host();

        let i8_ty = ctx.ty_i8();
        let i64_ty = ctx.ty_i64();
        let bool_ty = ctx.ty_bool();
        let unit_ty = ctx.ty_unit();

        let mut cache = LayoutCache::new(&ctx, &target);

        assert_eq!(cache.layout_of(i8_ty), Layout::new(1, 1));
        assert_eq!(cache.layout_of(i64_ty), Layout::new(8, 8));
        assert_eq!(cache.layout_of(bool_ty), Layout::new(1, 1));
        assert_eq!(cache.layout_of(unit_ty), Layout::zero(1));
    }

    #[test]
    fn test_pointer_layout() {
        let mut ctx = MirContext::new();
        let target = TargetConfig::host();

        let i64_ty = ctx.ty_i64();
        let ptr_ty = ctx.ty_ptr(i64_ty);
        let ref_ty = ctx.ty_ref(i64_ty);

        let mut cache = LayoutCache::new(&ctx, &target);

        assert_eq!(cache.layout_of(ptr_ty), Layout::new(8, 8));
        assert_eq!(cache.layout_of(ref_ty), Layout::new(8, 8));
    }

    #[test]
    fn test_tuple_layout() {
        let mut ctx = MirContext::new();
        let target = TargetConfig::host();

        // (i8, i64) - should have padding
        let i8_ty = ctx.ty_i8();
        let i64_ty = ctx.ty_i64();
        let tuple_ty = ctx.ty_tuple(vec![i8_ty, i64_ty]);

        let mut cache = LayoutCache::new(&ctx, &target);
        let layout = cache.layout_of(tuple_ty);

        // i8 at offset 0, padded to 8, i64 at offset 8
        // total size 16, align 8
        assert_eq!(layout.size, 16);
        assert_eq!(layout.align, 8);
    }

    #[test]
    fn test_str_layout() {
        let mut ctx = MirContext::new();
        let target = TargetConfig::host();

        let str_ty = ctx.ty_str();
        let mut cache = LayoutCache::new(&ctx, &target);
        let layout = cache.layout_of(str_ty);

        // str is { ptr, len } = 16 bytes on 64-bit
        assert_eq!(layout.size, 16);
        assert_eq!(layout.align, 8);
    }

    #[test]
    fn test_func_layouts() {
        let mut ctx = MirContext::new();
        let target = TargetConfig::host();

        let i64_ty = ctx.ty_i64();
        let thin_ty = ctx.intern_type(MirTy::FuncThin {
            params: vec![i64_ty],
            ret: i64_ty,
        });
        let thick_ty = ctx.intern_type(MirTy::FuncThick {
            params: vec![i64_ty],
            ret: i64_ty,
        });

        let mut cache = LayoutCache::new(&ctx, &target);

        // Thin function pointer: 8 bytes
        assert_eq!(cache.layout_of(thin_ty), Layout::new(8, 8));
        // Thick callable: 16 bytes (fn ptr + env ptr)
        assert_eq!(cache.layout_of(thick_ty), Layout::new(16, 8));
    }
}
