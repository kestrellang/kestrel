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
    /// Cache for struct layouts, keyed by (struct_id, type_args, self_type) to handle generic instantiations.
    struct_layouts: HashMap<(Id<Struct>, Vec<Id<Ty>>, Option<Id<Ty>>), StructLayout>,
    /// Cache for enum layouts, keyed by (enum_id, type_args) to handle generic instantiations.
    enum_layouts: HashMap<(Id<kestrel_execution_graph::Enum>, Vec<Id<Ty>>), Layout>,
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
            enum_layouts: HashMap::new(),
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
    ///
    /// `type_args` are the concrete type arguments for generic structs (e.g., `[Int]` for `Box[Int]`).
    /// For non-generic structs, pass an empty slice.
    /// Get the layout of a struct with type argument substitution and optional self_type.
    ///
    /// The `self_type` parameter is used to resolve `SelfType` in field types, which is needed
    /// for closure environment structs in protocol extension methods where fields may have
    /// associated type projections like `AssociatedTypeProjection { base: SelfType, ... }`.
    pub fn struct_layout(
        &mut self,
        struct_id: Id<Struct>,
        type_args: &[Id<Ty>],
        self_type: Option<Id<Ty>>,
    ) -> &StructLayout {
        let key = (struct_id, type_args.to_vec(), self_type);
        if !self.struct_layouts.contains_key(&key) {
            let layout =
                self.compute_struct_layout(struct_id, type_args, self_type, &HashMap::new());
            self.struct_layouts.insert(key.clone(), layout);
        }
        &self.struct_layouts[&key]
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

            // Tuple: lay out fields sequentially
            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                let mut layout = Layout::zero(1);
                for elem in elems {
                    let field_layout = self.layout_of(elem);
                    (_, layout) = layout.append(field_layout);
                }
                layout.pad_to_align()
            },

            // Named types need struct/enum lookup
            MirTy::Named { name, type_args } => {
                // Clone type_args to avoid borrowing issues
                let type_args = type_args.clone();
                // Look up struct by name
                let name_data = self.ctx.name(*name);
                for (id, def) in self.ctx.structs.iter() {
                    let def_name = self.ctx.name(def.name);
                    if def_name == name_data {
                        let struct_layout =
                            self.compute_struct_layout(id, &type_args, None, &HashMap::new());
                        return struct_layout.layout;
                    }
                }
                // Check enums
                for (id, def) in self.ctx.enums.iter() {
                    let def_name = self.ctx.name(def.name);
                    if def_name == name_data {
                        return self.compute_enum_layout(id, &type_args, &HashMap::new());
                    }
                }
                // Unknown named type - use pointer size as fallback
                Layout::new(ptr_size, ptr_size)
            },

            // Type parameters should be substituted before layout computation
            MirTy::TypeParam(tp) => {
                let tp_def = &self.ctx.type_params[*tp];
                panic!(
                    "TypeParam {:?} ('{}', owner: {:?}) reached layout computation without substitution - this is a bug",
                    tp, tp_def.name, tp_def.owner
                )
            },

            // Function pointers
            MirTy::FuncThin { .. } => Layout::new(ptr_size, ptr_size),
            // Thick callable: function pointer + environment pointer
            MirTy::FuncThick { .. } => Layout::new(ptr_size * 2, ptr_size),

            // Self type should be substituted before layout computation
            MirTy::SelfType => {
                panic!("SelfType reached layout computation without substitution - this is a bug")
            },

            // Associated type projection should be resolved before layout computation
            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                eprintln!("\n=== DEBUG: AssociatedTypeProjection in layout_of ===");
                eprintln!("Type ID: {:?}", ty);
                eprintln!("Base type ID: {:?}", base);
                eprintln!("Base type: {:?}", self.ctx.ty(*base));
                eprintln!("Protocol: {:?}", protocol);
                eprintln!("Associated: {}", associated);

                // Print backtrace to see where this is coming from
                eprintln!("\nBacktrace:");
                let bt = std::backtrace::Backtrace::force_capture();
                eprintln!("{}", bt);

                panic!(
                    "AssociatedTypeProjection (base={:?}, protocol={:?}, associated={}) reached layout computation without resolution - this is a bug",
                    base, protocol, associated
                )
            },

            // Error type
            MirTy::Error => Layout::zero(1),
        }
    }

    /// Compute layout for a struct with type argument substitution.
    ///
    /// For generic structs like `Box[T]`, the `type_args` (e.g., `[Int]`) are used to
    /// substitute type parameters in field types before computing their layouts.
    ///
    /// The `self_type` parameter is used to resolve `SelfType` in field types, which is needed
    /// for closure environment structs in protocol extension methods.
    fn compute_struct_layout(
        &mut self,
        struct_id: Id<Struct>,
        type_args: &[Id<Ty>],
        self_type: Option<Id<Ty>>,
        outer_subst: &HashMap<Id<kestrel_execution_graph::TypeParam>, Id<Ty>>,
    ) -> StructLayout {
        let struct_def = self.ctx.struct_def(struct_id);
        // Clone the field IDs and type_params to avoid borrowing issues
        let field_ids: Vec<_> = struct_def.fields.clone();
        let type_params: Vec<_> = struct_def.type_params.clone();

        // Build substitution map: parent substitutions + local type_param_id -> concrete_type
        let mut subst = outer_subst.clone();
        for (&tp, &ty) in type_params.iter().zip(type_args.iter()) {
            subst.insert(tp, ty);
        }

        let mut layout = Layout::zero(1);
        let mut field_offsets = HashMap::new();

        for field_id in field_ids {
            let field_def = &self.ctx.fields[field_id];
            let field_name = field_def.name.clone();
            // Compute field layout with substitution applied
            let field_layout = self.layout_of_with_subst(field_def.ty, &subst, self_type);
            let offset;
            (offset, layout) = layout.append(field_layout);
            field_offsets.insert(field_name, offset);
        }

        StructLayout {
            layout: layout.pad_to_align(),
            field_offsets,
        }
    }

    /// Compute layout for a type with substitution applied.
    ///
    /// This recursively computes layout while substituting type parameters with concrete types.
    /// Unlike looking up substituted types (which might not be interned), this directly computes
    /// the layout based on the structure of the type.
    ///
    /// The `self_type` parameter is used to resolve `SelfType` in types, which is needed
    /// for closure environment structs in protocol extension methods.
    fn layout_of_with_subst(
        &mut self,
        ty: Id<Ty>,
        subst: &HashMap<Id<kestrel_execution_graph::TypeParam>, Id<Ty>>,
        self_type: Option<Id<Ty>>,
    ) -> Layout {
        if subst.is_empty() && self_type.is_none() {
            return self.layout_of(ty);
        }

        let ptr_size = self.target.pointer_size();

        match self.ctx.ty(ty) {
            // Type parameter - look up in substitution and compute its layout
            MirTy::TypeParam(tp) => {
                if let Some(&concrete_ty) = subst.get(tp) {
                    // Keep substitution context active for nested projections in concrete_ty.
                    self.layout_of_with_subst(concrete_ty, subst, self_type)
                } else {
                    let tp_def = &self.ctx.type_params[*tp];
                    panic!(
                        "TypeParam {:?} ('{}', owner: {:?}) reached layout computation without substitution - this is a bug",
                        tp, tp_def.name, tp_def.owner
                    )
                }
            },

            // SelfType - substitute with self_type if available
            MirTy::SelfType => {
                if let Some(concrete_self) = self_type {
                    self.layout_of(concrete_self)
                } else {
                    panic!(
                        "SelfType reached layout computation without self_type substitution - this is a bug"
                    )
                }
            },

            // AssociatedTypeProjection - resolve base through substitution, then project Item.
            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                let resolved_base = match self.ctx.ty(*base) {
                    MirTy::SelfType => self_type.unwrap_or(*base),
                    MirTy::TypeParam(tp) => subst.get(tp).copied().unwrap_or(*base),
                    _ => *base,
                };

                // For iterator-like patterns where Item maps to the first type arg:
                // ArrayIterator[T].Item = T, PeekableIterator[I].Item = I.Item, etc.
                if *associated == "Item"
                    && let MirTy::Named { type_args, .. } = self.ctx.ty(resolved_base)
                    && !type_args.is_empty()
                {
                    return self.layout_of_with_subst(type_args[0], subst, self_type);
                }

                // Couldn't resolve - fall through to panic
                eprintln!("\n=== DEBUG: AssociatedTypeProjection in layout_of_with_subst ===");
                eprintln!("Type ID: {:?}", ty);
                eprintln!("Base type ID: {:?}", base);
                eprintln!("Base type: {:?}", self.ctx.ty(*base));
                eprintln!("Resolved base type: {:?}", self.ctx.ty(resolved_base));
                eprintln!("self_type: {:?}", self_type);
                if let Some(st) = self_type {
                    eprintln!("self_type resolved: {:?}", self.ctx.ty(st));
                }
                eprintln!("Protocol: {:?}", protocol);
                eprintln!("Associated: {}", associated);
                panic!(
                    "AssociatedTypeProjection (base={:?}, protocol={:?}, associated={}) could not be resolved in layout_of_with_subst",
                    base, protocol, associated
                )
            },

            // For Named types, recursively substitute type_args and compute layout
            MirTy::Named { name, type_args } => {
                // Substitute type_args
                let new_args: Vec<_> = type_args
                    .iter()
                    .map(|&arg| self.substitute_type_for_layout(arg, subst))
                    .collect();

                // Look up the struct/enum by name and compute layout with substituted type_args
                let name_data = self.ctx.name(*name);

                // Try to find struct
                for (id, def) in self.ctx.structs.iter() {
                    let def_name = self.ctx.name(def.name);
                    if def_name == name_data {
                        return self
                            .compute_struct_layout(id, &new_args, self_type, subst)
                            .layout;
                    }
                }

                // Try to find enum
                for (id, def) in self.ctx.enums.iter() {
                    let def_name = self.ctx.name(def.name);
                    if def_name == name_data {
                        return self.compute_enum_layout(id, &new_args, subst);
                    }
                }

                // Unknown named type - use pointer size as fallback
                Layout::new(ptr_size, ptr_size)
            },

            // Pointer/Ref types - always pointer-sized
            MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => Layout::new(ptr_size, ptr_size),

            // Tuple - substitute each element and compute layout
            MirTy::Tuple(elems) => {
                let elems = elems.clone();
                let mut layout = Layout::zero(1);
                for elem in elems {
                    let elem_layout = self.layout_of_with_subst(elem, subst, self_type);
                    (_, layout) = layout.append(elem_layout);
                }
                layout.pad_to_align()
            },

            // All other types - compute layout normally
            _ => self.layout_of(ty),
        }
    }

    /// Substitute type parameters in a type, returning the best available type ID.
    /// Used only for computing type_args to pass to struct/enum layout computation.
    fn substitute_type_for_layout(
        &self,
        ty: Id<Ty>,
        subst: &HashMap<Id<kestrel_execution_graph::TypeParam>, Id<Ty>>,
    ) -> Id<Ty> {
        if subst.is_empty() {
            return ty;
        }

        match self.ctx.ty(ty) {
            // Type parameter - look up in substitution
            MirTy::TypeParam(tp) => subst.get(tp).copied().unwrap_or(ty),

            MirTy::AssociatedTypeProjection {
                base,
                protocol,
                associated,
            } => {
                let new_base = self.substitute_type_for_layout(*base, subst);
                if new_base == *base {
                    ty
                } else {
                    self.ctx
                        .lookup_type(&MirTy::AssociatedTypeProjection {
                            base: new_base,
                            protocol: *protocol,
                            associated: associated.clone(),
                        })
                        .unwrap_or(ty)
                }
            },

            MirTy::Named { name, type_args } => {
                let new_args: Vec<_> = type_args
                    .iter()
                    .map(|&arg| self.substitute_type_for_layout(arg, subst))
                    .collect();

                if new_args == *type_args {
                    ty
                } else {
                    self.ctx
                        .lookup_type(&MirTy::Named {
                            name: *name,
                            type_args: new_args,
                        })
                        .unwrap_or(ty)
                }
            },

            MirTy::Tuple(elems) => {
                let new_elems: Vec<_> = elems
                    .iter()
                    .map(|&elem| self.substitute_type_for_layout(elem, subst))
                    .collect();

                if new_elems == *elems {
                    ty
                } else {
                    self.ctx.lookup_type(&MirTy::Tuple(new_elems)).unwrap_or(ty)
                }
            },

            MirTy::Pointer(inner) => {
                let new_inner = self.substitute_type_for_layout(*inner, subst);
                if new_inner == *inner {
                    ty
                } else {
                    self.ctx
                        .lookup_type(&MirTy::Pointer(new_inner))
                        .unwrap_or(ty)
                }
            },

            MirTy::Ref(inner) => {
                let new_inner = self.substitute_type_for_layout(*inner, subst);
                if new_inner == *inner {
                    ty
                } else {
                    self.ctx.lookup_type(&MirTy::Ref(new_inner)).unwrap_or(ty)
                }
            },

            MirTy::RefMut(inner) => {
                let new_inner = self.substitute_type_for_layout(*inner, subst);
                if new_inner == *inner {
                    ty
                } else {
                    self.ctx
                        .lookup_type(&MirTy::RefMut(new_inner))
                        .unwrap_or(ty)
                }
            },

            _ => ty,
        }
    }

    /// Get the layout of an enum (tagged union).
    ///
    /// `type_args` are the concrete type arguments for generic enums (e.g., `[Int]` for `Option[Int]`).
    /// For non-generic enums, pass an empty slice.
    pub fn enum_layout(
        &mut self,
        enum_id: Id<kestrel_execution_graph::Enum>,
        type_args: &[Id<Ty>],
    ) -> Layout {
        let key = (enum_id, type_args.to_vec());
        if let Some(&layout) = self.enum_layouts.get(&key) {
            return layout;
        }

        let layout = self.compute_enum_layout(enum_id, type_args, &HashMap::new());
        self.enum_layouts.insert(key, layout);
        layout
    }

    /// Compute layout for an enum (tagged union) with type argument substitution.
    fn compute_enum_layout(
        &mut self,
        enum_id: Id<kestrel_execution_graph::Enum>,
        type_args: &[Id<Ty>],
        outer_subst: &HashMap<Id<kestrel_execution_graph::TypeParam>, Id<Ty>>,
    ) -> Layout {
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
                // Pass the enum's type_args to the payload struct layout
                // The payload struct's type params correspond to the enum's type params
                let payload_layout = self
                    .compute_struct_layout(struct_id, type_args, None, outer_subst)
                    .layout;
                if payload_layout.size > max_payload.size {
                    max_payload = payload_layout;
                }
            } else {
                // Otherwise look up by name
                let case_name = self.ctx.name(case_def.struct_name);
                for (id, def) in self.ctx.structs.iter() {
                    if self.ctx.name(def.name) == case_name {
                        let payload_layout = self
                            .compute_struct_layout(id, type_args, None, outer_subst)
                            .layout;
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
