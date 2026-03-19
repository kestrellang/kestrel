//! Type layout computation.
//!
//! Computes size and alignment for MIR types based on the target configuration.
//! Leverages precomputed layouts from the MIR layout pass for non-generic structs,
//! and computes layouts on-demand for generic instantiations during monomorphization.
//!
//! # Improvements over lib1
//!
//! - O(1) entity→struct/enum lookup (built at construction, replaces linear scan)
//! - Uses precomputed `StructLayout` from MIR layout pass when available
//! - `u64` throughout for cross-compilation safety
//! - By-value type substitution (no interning)
//! - `ktrace!` debug output instead of `eprintln!` + backtrace
//! - Exhaustive match in type substitution (no silent catch-all)

use crate::TargetConfig;
use kestrel_debug::ktrace;
use kestrel_hecs::Entity;
use kestrel_mir::{EnumId, MirModule, MirTy, StructId};
use std::collections::HashMap;

/// Memory layout of a type: size and alignment in bytes.
///
/// Uses `u64` (not `usize`) so layout computation is correct when
/// cross-compiling for a 64-bit target on a 32-bit host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub size: u64,
    pub align: u64,
}

impl Layout {
    pub fn new(size: u64, align: u64) -> Self {
        debug_assert!(align.is_power_of_two(), "alignment must be a power of 2");
        Self { size, align }
    }

    /// Zero-size layout with the given alignment.
    pub fn zero(align: u64) -> Self {
        Self::new(0, align)
    }

    /// Round size up to the alignment boundary.
    pub fn pad_to_align(self) -> Self {
        let padded = (self.size + self.align - 1) & !(self.align - 1);
        Self {
            size: padded,
            align: self.align,
        }
    }

    /// Compute offset for appending a field with the given layout.
    /// Returns `(field_offset, updated_aggregate_layout)`.
    pub fn append(self, field: Layout) -> (u64, Layout) {
        // Align the current size up to the field's alignment
        let offset = (self.size + field.align - 1) & !(field.align - 1);
        let new_size = offset + field.size;
        let new_align = self.align.max(field.align);
        (offset, Layout::new(new_size, new_align))
    }
}

/// Detailed layout for a struct, including per-field byte offsets.
#[derive(Debug, Clone)]
pub struct DetailedStructLayout {
    /// Overall layout (total size + alignment).
    pub layout: Layout,
    /// Byte offset of each field, indexed by field declaration order.
    pub field_offsets: Vec<u64>,
}

/// Cache for computed type layouts.
///
/// Holds a reference to the `MirModule` and pre-built lookup maps for O(1)
/// entity-to-struct/enum resolution (replaces lib1's linear scan).
pub struct LayoutCache<'a> {
    module: &'a MirModule,
    target: &'a TargetConfig,
    /// O(1) entity → StructId lookup.
    entity_to_struct: HashMap<Entity, StructId>,
    /// O(1) entity → EnumId lookup.
    entity_to_enum: HashMap<Entity, EnumId>,
    /// Cached layouts for concrete `MirTy` values.
    type_cache: HashMap<MirTy, Layout>,
    /// Cached layouts for generic struct instantiations.
    struct_cache: HashMap<(StructId, Vec<MirTy>), DetailedStructLayout>,
    /// Cached layouts for generic enum instantiations.
    enum_cache: HashMap<(EnumId, Vec<MirTy>), Layout>,
}

impl<'a> LayoutCache<'a> {
    /// Create a new layout cache, building entity lookup maps from the module.
    pub fn new(module: &'a MirModule, target: &'a TargetConfig) -> Self {
        let entity_to_struct = module
            .structs
            .iter()
            .enumerate()
            .map(|(i, s)| (s.entity, StructId::new(i)))
            .collect();

        let entity_to_enum = module
            .enums
            .iter()
            .enumerate()
            .map(|(i, e)| (e.entity, EnumId::new(i)))
            .collect();

        Self {
            module,
            target,
            entity_to_struct,
            entity_to_enum,
            type_cache: HashMap::new(),
            struct_cache: HashMap::new(),
            enum_cache: HashMap::new(),
        }
    }

    /// Get the layout of a MIR type.
    pub fn layout_of(&mut self, ty: &MirTy) -> Layout {
        if let Some(&cached) = self.type_cache.get(ty) {
            return cached;
        }
        let layout = self.compute_layout(ty);
        self.type_cache.insert(ty.clone(), layout);
        layout
    }

    /// Get the detailed layout (with field offsets) of a struct instantiation.
    ///
    /// For non-generic structs with a precomputed layout, returns the MIR layout
    /// directly. For generic instantiations, computes the layout with the given
    /// concrete type arguments.
    pub fn struct_layout(
        &mut self,
        struct_id: StructId,
        type_args: &[MirTy],
    ) -> DetailedStructLayout {
        let key = (struct_id, type_args.to_vec());
        if let Some(cached) = self.struct_cache.get(&key) {
            return cached.clone();
        }
        let layout = self.compute_struct_layout(struct_id, type_args);
        self.struct_cache.insert(key, layout.clone());
        layout
    }

    /// Get the layout of an enum instantiation (tagged union).
    pub fn enum_layout(&mut self, enum_id: EnumId, type_args: &[MirTy]) -> Layout {
        let key = (enum_id, type_args.to_vec());
        if let Some(&cached) = self.enum_cache.get(&key) {
            return cached;
        }
        let layout = self.compute_enum_layout(enum_id, type_args);
        self.enum_cache.insert(key, layout);
        layout
    }

    /// Resolve a Named entity to its struct or enum ID.
    pub fn resolve_named(&self, entity: Entity) -> NamedKind {
        if let Some(&id) = self.entity_to_struct.get(&entity) {
            return NamedKind::Struct(id);
        }
        if let Some(&id) = self.entity_to_enum.get(&entity) {
            return NamedKind::Enum(id);
        }
        NamedKind::Unknown
    }

    fn compute_layout(&mut self, ty: &MirTy) -> Layout {
        let ptr = self.target.pointer_size();

        match ty {
            // Primitives
            MirTy::I8 | MirTy::Bool => Layout::new(1, 1),
            MirTy::I16 | MirTy::F16 => Layout::new(2, 2),
            MirTy::I32 | MirTy::F32 => Layout::new(4, 4),
            MirTy::I64 | MirTy::F64 => Layout::new(8, 8),
            MirTy::Unit | MirTy::Never => Layout::zero(1),

            // Fat string pointer: (ptr, len)
            MirTy::Str => Layout::new(ptr * 2, ptr),

            // Single-word pointer types
            MirTy::Pointer(_) | MirTy::Ref(_) | MirTy::RefMut(_) => Layout::new(ptr, ptr),

            // Thin function pointer
            MirTy::FuncThin { .. } => Layout::new(ptr, ptr),

            // Thick function pointer: (func_ptr, env_ptr)
            MirTy::FuncThick { .. } => Layout::new(ptr * 2, ptr),

            // Tuple: sequential fields with alignment
            MirTy::Tuple(elems) => {
                let mut layout = Layout::zero(1);
                for elem in elems {
                    let field_layout = self.layout_of(elem);
                    (_, layout) = layout.append(field_layout);
                }
                layout.pad_to_align()
            }

            // Named struct or enum: O(1) entity lookup
            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                let type_args = type_args.clone();
                match self.resolve_named(entity) {
                    NamedKind::Struct(id) => self.struct_layout(id, &type_args).layout,
                    NamedKind::Enum(id) => self.enum_layout(id, &type_args),
                    NamedKind::Unknown => {
                        ktrace!(
                            "codegen",
                            "layout_of: unknown Named entity {:?}, using pointer-size fallback",
                            entity
                        );
                        Layout::new(ptr, ptr)
                    }
                }
            }

            // These must be substituted before layout computation
            MirTy::TypeParam(entity) => {
                ktrace!(
                    "codegen",
                    "TypeParam {:?} reached layout computation without substitution",
                    entity
                );
                panic!("TypeParam reached layout computation without substitution")
            }
            MirTy::SelfType => {
                panic!("SelfType reached layout computation without substitution")
            }
            MirTy::AssociatedProjection {
                base,
                protocol,
                name,
            } => {
                ktrace!(
                    "codegen",
                    "AssociatedProjection reached layout: base={:?}, protocol={:?}, name={}",
                    base,
                    protocol,
                    name
                );
                panic!("AssociatedProjection reached layout without resolution")
            }

            MirTy::Error => Layout::zero(1),
        }
    }

    fn compute_struct_layout(
        &mut self,
        struct_id: StructId,
        type_args: &[MirTy],
    ) -> DetailedStructLayout {
        let struct_def = &self.module.structs[struct_id.index()];

        // Leverage the precomputed layout for non-generic structs
        if type_args.is_empty()
            && struct_def.type_params.is_empty()
            && let Some(ref precomputed) = struct_def.layout
        {
            return DetailedStructLayout {
                layout: Layout::new(precomputed.size, precomputed.align),
                field_offsets: precomputed.field_offsets.clone(),
            };
        }

        // Build substitution map: type_param entity → concrete type
        let subst: HashMap<Entity, MirTy> = self.module.structs[struct_id.index()]
            .type_params
            .iter()
            .zip(type_args.iter())
            .map(|(tp, arg)| (tp.entity, arg.clone()))
            .collect();

        // Clone field types to avoid borrow conflict with &mut self
        let field_types: Vec<MirTy> = self.module.structs[struct_id.index()]
            .fields
            .iter()
            .map(|f| substitute_type(&f.ty, &subst))
            .collect();

        // Compute layout field by field
        let mut layout = Layout::zero(1);
        let mut field_offsets = Vec::with_capacity(field_types.len());

        for field_ty in &field_types {
            let field_layout = self.layout_of(field_ty);
            let offset;
            (offset, layout) = layout.append(field_layout);
            field_offsets.push(offset);
        }

        DetailedStructLayout {
            layout: layout.pad_to_align(),
            field_offsets,
        }
    }

    fn compute_enum_layout(&mut self, enum_id: EnumId, type_args: &[MirTy]) -> Layout {
        let enum_def = &self.module.enums[enum_id.index()];

        // Discriminant is i32 (4 bytes, 4-aligned)
        let discriminant = Layout::new(4, 4);

        // Find the largest payload across all cases
        let case_payload_structs: Vec<StructId> =
            enum_def.cases.iter().map(|c| c.payload_struct).collect();

        let mut max_payload = Layout::zero(1);
        for payload_struct_id in case_payload_structs {
            let payload = self.compute_struct_layout(payload_struct_id, type_args);
            if payload.layout.size > max_payload.size
                || (payload.layout.size == max_payload.size
                    && payload.layout.align > max_payload.align)
            {
                max_payload = payload.layout;
            }
        }

        // Enum = discriminant + max(payload), padded to alignment
        let (_, layout) = discriminant.append(max_payload);
        layout.pad_to_align()
    }
}

/// Result of resolving a Named entity.
#[derive(Debug, Clone, Copy)]
pub enum NamedKind {
    Struct(StructId),
    Enum(EnumId),
    Unknown,
}

/// Apply type parameter substitutions to a MirTy, producing a new type by value.
///
/// Exhaustive match — every variant is handled explicitly (no catch-all).
pub fn substitute_type(ty: &MirTy, subst: &HashMap<Entity, MirTy>) -> MirTy {
    if subst.is_empty() {
        return ty.clone();
    }

    match ty {
        // Primitives — no substitution needed
        MirTy::I8
        | MirTy::I16
        | MirTy::I32
        | MirTy::I64
        | MirTy::F16
        | MirTy::F32
        | MirTy::F64
        | MirTy::Bool
        | MirTy::Unit
        | MirTy::Never
        | MirTy::Str
        | MirTy::Error => ty.clone(),

        MirTy::TypeParam(entity) => match subst.get(entity) {
            Some(concrete) => concrete.clone(),
            None => ty.clone(),
        },

        MirTy::SelfType => ty.clone(),

        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(substitute_type(inner, subst))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(substitute_type(inner, subst))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(substitute_type(inner, subst))),

        MirTy::Tuple(elems) => {
            MirTy::Tuple(elems.iter().map(|e| substitute_type(e, subst)).collect())
        }

        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args
                .iter()
                .map(|a| substitute_type(a, subst))
                .collect(),
        },

        MirTy::AssociatedProjection {
            base,
            protocol,
            name,
        } => MirTy::AssociatedProjection {
            base: Box::new(substitute_type(base, subst)),
            protocol: *protocol,
            name: name.clone(),
        },

        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|p| substitute_type(p, subst)).collect(),
            ret: Box::new(substitute_type(ret, subst)),
        },

        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|p| substitute_type(p, subst)).collect(),
            ret: Box::new(substitute_type(ret, subst)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_mir::{EnumCaseDef, EnumDef, FieldDef, StructDef, StructLayout, TypeParamDef};

    fn dummy_entity(id: u32) -> Entity {
        Entity::from_raw(id)
    }

    fn host_target() -> TargetConfig {
        TargetConfig::host()
    }

    // --- Layout arithmetic ---

    #[test]
    fn pad_to_align() {
        assert_eq!(Layout::new(5, 4).pad_to_align(), Layout::new(8, 4));
        assert_eq!(Layout::new(8, 4).pad_to_align(), Layout::new(8, 4));
        assert_eq!(Layout::new(0, 1).pad_to_align(), Layout::new(0, 1));
    }

    #[test]
    fn append_fields() {
        let base = Layout::zero(1);
        // First field: i32 (size=4, align=4)
        let (off1, layout) = base.append(Layout::new(4, 4));
        assert_eq!(off1, 0);
        assert_eq!(layout, Layout::new(4, 4));

        // Second field: i8 (size=1, align=1)
        let (off2, layout) = layout.append(Layout::new(1, 1));
        assert_eq!(off2, 4);
        assert_eq!(layout, Layout::new(5, 4));

        // Pad to align
        assert_eq!(layout.pad_to_align(), Layout::new(8, 4));
    }

    // --- Primitive layouts ---

    #[test]
    fn primitive_layouts() {
        let target = host_target();
        let module = MirModule::new("test");
        let mut cache = LayoutCache::new(&module, &target);

        assert_eq!(cache.layout_of(&MirTy::I8), Layout::new(1, 1));
        assert_eq!(cache.layout_of(&MirTy::I16), Layout::new(2, 2));
        assert_eq!(cache.layout_of(&MirTy::I32), Layout::new(4, 4));
        assert_eq!(cache.layout_of(&MirTy::I64), Layout::new(8, 8));
        assert_eq!(cache.layout_of(&MirTy::Bool), Layout::new(1, 1));
        assert_eq!(cache.layout_of(&MirTy::Unit), Layout::zero(1));
        assert_eq!(cache.layout_of(&MirTy::Never), Layout::zero(1));
    }

    #[test]
    fn pointer_layouts() {
        let target = host_target();
        let ptr = target.pointer_size();
        let module = MirModule::new("test");
        let mut cache = LayoutCache::new(&module, &target);

        assert_eq!(
            cache.layout_of(&MirTy::Pointer(Box::new(MirTy::I32))),
            Layout::new(ptr, ptr)
        );
        assert_eq!(
            cache.layout_of(&MirTy::Ref(Box::new(MirTy::I64))),
            Layout::new(ptr, ptr)
        );
        assert_eq!(
            cache.layout_of(&MirTy::RefMut(Box::new(MirTy::Bool))),
            Layout::new(ptr, ptr)
        );
    }

    #[test]
    fn string_layout() {
        let target = host_target();
        let ptr = target.pointer_size();
        let module = MirModule::new("test");
        let mut cache = LayoutCache::new(&module, &target);

        assert_eq!(cache.layout_of(&MirTy::Str), Layout::new(ptr * 2, ptr));
    }

    #[test]
    fn tuple_layout() {
        let target = host_target();
        let module = MirModule::new("test");
        let mut cache = LayoutCache::new(&module, &target);

        // (i32, i8) → size=8 (4 + 1 + 3 padding), align=4
        let ty = MirTy::Tuple(vec![MirTy::I32, MirTy::I8]);
        assert_eq!(cache.layout_of(&ty), Layout::new(8, 4));

        // (i8, i64) → offset 0: i8, offset 8: i64, size=16, align=8
        let ty = MirTy::Tuple(vec![MirTy::I8, MirTy::I64]);
        assert_eq!(cache.layout_of(&ty), Layout::new(16, 8));
    }

    // --- Struct layouts ---

    #[test]
    fn simple_struct_layout() {
        let target = host_target();
        let mut module = MirModule::new("test");

        let entity = dummy_entity(1);
        let mut def = StructDef::new(entity, "Point");
        def.add_field(FieldDef::new("x", MirTy::I64));
        def.add_field(FieldDef::new("y", MirTy::I64));
        module.add_struct(def);

        let mut cache = LayoutCache::new(&module, &target);
        let sl = cache.struct_layout(StructId::new(0), &[]);
        assert_eq!(sl.layout, Layout::new(16, 8));
        assert_eq!(sl.field_offsets, vec![0, 8]);
    }

    #[test]
    fn precomputed_layout_reused() {
        let target = host_target();
        let mut module = MirModule::new("test");

        let entity = dummy_entity(1);
        let mut def = StructDef::new(entity, "Precomputed");
        def.add_field(FieldDef::new("a", MirTy::I32));
        def.add_field(FieldDef::new("b", MirTy::I8));
        // Simulate the MIR layout pass having already computed this
        def.layout = Some(StructLayout {
            size: 8,
            align: 4,
            field_offsets: vec![0, 4],
        });
        module.add_struct(def);

        let mut cache = LayoutCache::new(&module, &target);
        let sl = cache.struct_layout(StructId::new(0), &[]);
        assert_eq!(sl.layout, Layout::new(8, 4));
        assert_eq!(sl.field_offsets, vec![0, 4]);
    }

    #[test]
    fn generic_struct_instantiation() {
        let target = host_target();
        let mut module = MirModule::new("test");

        let struct_entity = dummy_entity(1);
        let type_param_entity = dummy_entity(2);

        let mut def = StructDef::new(struct_entity, "Wrapper");
        def.type_params.push(TypeParamDef {
            entity: type_param_entity,
            name: "T".into(),
        });
        def.add_field(FieldDef::new("value", MirTy::TypeParam(type_param_entity)));
        def.add_field(FieldDef::new("flag", MirTy::Bool));
        module.add_struct(def);

        let mut cache = LayoutCache::new(&module, &target);

        // Wrapper[I64]: field 0 = i64 at 0, field 1 = bool at 8, size=16 align=8
        let sl = cache.struct_layout(StructId::new(0), &[MirTy::I64]);
        assert_eq!(sl.layout, Layout::new(16, 8));
        assert_eq!(sl.field_offsets, vec![0, 8]);

        // Wrapper[I8]: field 0 = i8 at 0, field 1 = bool at 1, size=2 align=1
        let sl = cache.struct_layout(StructId::new(0), &[MirTy::I8]);
        assert_eq!(sl.layout, Layout::new(2, 1));
        assert_eq!(sl.field_offsets, vec![0, 1]);
    }

    #[test]
    fn named_type_layout() {
        let target = host_target();
        let mut module = MirModule::new("test");

        let entity = dummy_entity(1);
        let mut def = StructDef::new(entity, "Pair");
        def.add_field(FieldDef::new("a", MirTy::I32));
        def.add_field(FieldDef::new("b", MirTy::I32));
        module.add_struct(def);

        let mut cache = LayoutCache::new(&module, &target);
        let ty = MirTy::Named {
            entity,
            type_args: vec![],
        };
        assert_eq!(cache.layout_of(&ty), Layout::new(8, 4));
    }

    // --- Enum layouts ---

    #[test]
    fn simple_enum_layout() {
        let target = host_target();
        let mut module = MirModule::new("test");

        // Payload structs for cases
        let none_entity = dummy_entity(10);
        let some_entity = dummy_entity(11);
        let none_struct = module.add_struct(StructDef::new(none_entity, "Optional.None"));
        let mut some_def = StructDef::new(some_entity, "Optional.Some");
        some_def.add_field(FieldDef::new("0", MirTy::I64));
        let some_struct = module.add_struct(some_def);

        // Enum definition
        let enum_entity = dummy_entity(1);
        let mut enum_def = EnumDef::new(enum_entity, "Optional");
        enum_def.add_case(EnumCaseDef::new("None", 0, none_struct));
        enum_def.add_case(EnumCaseDef::new("Some", 1, some_struct));
        module.add_enum(enum_def);

        let mut cache = LayoutCache::new(&module, &target);
        // discriminant (4 bytes) + padding (4 bytes) + i64 payload (8 bytes) = 16, align=8
        let layout = cache.enum_layout(EnumId::new(0), &[]);
        assert_eq!(layout, Layout::new(16, 8));
    }

    // --- Substitution ---

    #[test]
    fn substitute_type_basic() {
        let entity = dummy_entity(1);
        let mut subst = HashMap::new();
        subst.insert(entity, MirTy::I64);

        assert_eq!(
            substitute_type(&MirTy::TypeParam(entity), &subst),
            MirTy::I64
        );
        // Primitives unchanged
        assert_eq!(substitute_type(&MirTy::Bool, &subst), MirTy::Bool);
    }

    #[test]
    fn substitute_type_nested() {
        let entity = dummy_entity(1);
        let mut subst = HashMap::new();
        subst.insert(entity, MirTy::I32);

        let ty = MirTy::Ref(Box::new(MirTy::TypeParam(entity)));
        assert_eq!(
            substitute_type(&ty, &subst),
            MirTy::Ref(Box::new(MirTy::I32))
        );
    }

    #[test]
    fn substitute_type_named() {
        let param = dummy_entity(1);
        let struct_e = dummy_entity(2);
        let mut subst = HashMap::new();
        subst.insert(param, MirTy::I64);

        let ty = MirTy::Named {
            entity: struct_e,
            type_args: vec![MirTy::TypeParam(param)],
        };
        assert_eq!(
            substitute_type(&ty, &subst),
            MirTy::Named {
                entity: struct_e,
                type_args: vec![MirTy::I64],
            }
        );
    }

    #[test]
    fn substitute_empty_is_identity() {
        let ty = MirTy::Tuple(vec![MirTy::I32, MirTy::Bool]);
        let subst = HashMap::new();
        assert_eq!(substitute_type(&ty, &subst), ty);
    }
}
