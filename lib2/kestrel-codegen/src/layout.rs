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

    /// Expose the underlying MIR module so callers can reuse witness-table
    /// lookups (notably `normalize_projection`) without threading the module
    /// separately.
    pub fn module(&self) -> &'a MirModule {
        self.module
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

    /// Get the byte offset of the enum payload area.
    ///
    /// This must stay in lockstep with `compute_enum_layout` so enum construction,
    /// downcasts, and stack slot sizing all agree on where the payload begins.
    pub fn enum_payload_offset(&mut self, enum_id: EnumId, type_args: &[MirTy]) -> u64 {
        let payload = self.max_enum_payload_layout(enum_id, type_args);
        let discriminant = Layout::new(4, 4);
        let (offset, _) = discriminant.append(payload);
        offset
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
            MirTy::Never => Layout::zero(1),

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
            },

            // Named struct or enum: O(1) entity lookup
            MirTy::Named { entity, type_args } => {
                let entity = *entity;
                let type_args = type_args.clone();
                match self.resolve_named(entity) {
                    NamedKind::Struct(id) => self.struct_layout(id, &type_args).layout,
                    NamedKind::Enum(id) => self.enum_layout(id, &type_args),
                    NamedKind::Unknown => {
                        let name = self.module.resolve_name(entity);
                        if !name.contains("Equatable")
                            && !name.contains("Comparable")
                            && !name.contains("Iterator")
                        {
                            eprintln!(
                                "[DIAG] layout_of: unknown Named entity {:?} ({}), using pointer-size fallback",
                                entity, name
                            );
                        }
                        Layout::new(ptr, ptr)
                    },
                }
            },

            // These must be substituted before layout computation
            MirTy::TypeParam(entity) => {
                eprintln!(
                    "[DIAG] layout_of: TypeParam({:?}) reached layout computation without substitution",
                    entity
                );
                Layout::new(ptr, ptr)
            },
            MirTy::SelfType => {
                eprintln!(
                    "[DIAG] layout_of: SelfType reached layout computation without substitution"
                );
                Layout::new(ptr, ptr)
            },
            MirTy::AssociatedProjection {
                base,
                protocol,
                name,
            } => {
                // Try resolving through the witness table before falling back
                // to the pointer-size placeholder. Without this, sub-i64
                // payload types (e.g. Optional<I.Item> where Item = Int8)
                // get sized as 8 bytes and disagree with downstream aggregate
                // classification.
                let normalized = normalize_projection(ty, self.module);
                if !matches!(normalized, MirTy::AssociatedProjection { .. }) {
                    return self.layout_of(&normalized);
                }
                ktrace!(
                    "codegen",
                    "AssociatedProjection reached layout: base={:?}, protocol={:?}, name={}",
                    base,
                    protocol,
                    name
                );
                Layout::new(ptr, ptr)
            },

            MirTy::Error => {
                eprintln!(
                    "[DIAG] layout_of: MirTy::Error reached layout computation — unresolved type leaked into codegen"
                );
                Layout::zero(1)
            },
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
        let payload = self.max_enum_payload_layout(enum_id, type_args);

        // Enum = discriminant + max(payload), padded to alignment
        let discriminant = Layout::new(4, 4);
        let (_, layout) = discriminant.append(payload);
        layout.pad_to_align()
    }

    fn max_enum_payload_layout(&mut self, enum_id: EnumId, type_args: &[MirTy]) -> Layout {
        let enum_def = &self.module.enums[enum_id.index()];

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

        max_payload
    }
}

/// Result of resolving a Named entity.
#[derive(Debug, Clone, Copy)]
pub enum NamedKind {
    Struct(StructId),
    Enum(EnumId),
    Unknown,
}

/// Maximum recursion depth when normalizing projections. Chosen large enough
/// for deeply chained `A.B.C.D` projections but small enough to abort cycles.
const NORMALIZE_PROJECTION_MAX_DEPTH: u32 = 32;

/// Walk a `MirTy` and resolve every `AssociatedProjection` whose base is a
/// fully-concrete type by consulting the witness table in `module`.
///
/// Substitution alone cannot turn an `AssociatedProjection { base: ..., name }`
/// into its bound concrete type — that requires looking up the witness for
/// `base: Protocol`. Classification (`layout_of`, `is_aggregate`, field-type
/// derivation) needs the concrete type so read and consume sites agree on
/// by-value vs by-pointer handling. Callers should normalize before classifying.
///
/// Leaves the projection unchanged when the base still contains abstract
/// `TypeParam` / `SelfType` / unresolved inner projections — there's nothing
/// to resolve against, and pre-monomorphization callers rely on the no-op.
pub fn normalize_projection(ty: &MirTy, module: &MirModule) -> MirTy {
    normalize_projection_inner(ty, module, 0)
}

fn normalize_projection_inner(ty: &MirTy, module: &MirModule, depth: u32) -> MirTy {
    if depth >= NORMALIZE_PROJECTION_MAX_DEPTH {
        return ty.clone();
    }
    let next = depth + 1;
    let rec = |t: &MirTy| normalize_projection_inner(t, module, next);

    match ty {
        MirTy::AssociatedProjection {
            base,
            protocol,
            name,
        } => {
            let normalized_base = rec(base);
            if is_concrete(&normalized_base)
                && let Some(resolved) =
                    resolve_assoc_type_via_witness(module, *protocol, &normalized_base, name)
            {
                return normalize_projection_inner(&resolved, module, next);
            }
            MirTy::AssociatedProjection {
                base: Box::new(normalized_base),
                protocol: *protocol,
                name: name.clone(),
            }
        },

        MirTy::Named { entity, type_args } => MirTy::Named {
            entity: *entity,
            type_args: type_args.iter().map(&rec).collect(),
        },
        MirTy::Tuple(elems) => MirTy::Tuple(elems.iter().map(&rec).collect()),
        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(rec(inner))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(rec(inner))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(rec(inner))),
        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(&rec).collect(),
            ret: Box::new(rec(ret)),
        },
        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(&rec).collect(),
            ret: Box::new(rec(ret)),
        },

        // Leaves — nothing to normalize.
        MirTy::I8
        | MirTy::I16
        | MirTy::I32
        | MirTy::I64
        | MirTy::F16
        | MirTy::F32
        | MirTy::F64
        | MirTy::Bool
        | MirTy::Never
        | MirTy::Str
        | MirTy::TypeParam(_)
        | MirTy::SelfType
        | MirTy::Error => ty.clone(),
    }
}

/// A type is concrete (safe to use as a witness self-type for projection
/// resolution) if it contains no `TypeParam`, `SelfType`, `Error`, or still-
/// abstract `AssociatedProjection`. Witness `match_pattern` would happily
/// bind a `TypeParam` on the implementing-type side to a `TypeParam` on the
/// query side, producing garbage — so require concreteness up front.
fn is_concrete(ty: &MirTy) -> bool {
    match ty {
        MirTy::TypeParam(_) | MirTy::SelfType | MirTy::Error => false,
        MirTy::AssociatedProjection { base, .. } => is_concrete(base),
        MirTy::Pointer(t) | MirTy::Ref(t) | MirTy::RefMut(t) => is_concrete(t),
        MirTy::Tuple(elems) => elems.iter().all(is_concrete),
        MirTy::Named { type_args, .. } => type_args.iter().all(is_concrete),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().all(is_concrete) && is_concrete(ret)
        },
        _ => true,
    }
}

/// Find a witness of `protocol` that matches `self_type`, read its
/// associated-type binding for `name`, and substitute the witness's
/// implementation-type-param bindings into the result.
///
/// Returns `None` if no witness matches, no binding exists, or the witness
/// lacks the requested name — callers fall back to leaving the projection
/// unresolved rather than erroring (this runs during layout and must be
/// tolerant of partially-lowered types).
fn resolve_assoc_type_via_witness(
    module: &MirModule,
    protocol: Entity,
    self_type: &MirTy,
    name: &str,
) -> Option<MirTy> {
    for witness in &module.witnesses {
        if witness.protocol != protocol {
            continue;
        }
        let mut bindings = HashMap::new();
        if !witness_match(&witness.implementing_type, self_type, &mut bindings) {
            continue;
        }
        let bound = match witness.type_bindings.get(name) {
            Some(ty) => ty,
            None => continue,
        };
        return Some(substitute_type(bound, &bindings));
    }
    None
}

/// Structural pattern match mirroring cranelift's `match_pattern`: a
/// `TypeParam` on the pattern side is a wildcard that binds to the concrete
/// counterpart; everything else must be structurally equal.
fn witness_match(
    pattern: &MirTy,
    concrete: &MirTy,
    bindings: &mut HashMap<Entity, MirTy>,
) -> bool {
    match (pattern, concrete) {
        (MirTy::TypeParam(entity), _) => match bindings.get(entity) {
            Some(existing) => existing == concrete,
            None => {
                bindings.insert(*entity, concrete.clone());
                true
            },
        },
        (
            MirTy::Named {
                entity: e1,
                type_args: a1,
            },
            MirTy::Named {
                entity: e2,
                type_args: a2,
            },
        ) => {
            e1 == e2
                && a1.len() == a2.len()
                && a1.iter().zip(a2).all(|(p, c)| witness_match(p, c, bindings))
        },
        (MirTy::Ref(a), MirTy::Ref(b))
        | (MirTy::RefMut(a), MirTy::RefMut(b))
        | (MirTy::Pointer(a), MirTy::Pointer(b)) => witness_match(a, b, bindings),
        (MirTy::Tuple(a), MirTy::Tuple(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(p, c)| witness_match(p, c, bindings))
        },
        (
            MirTy::FuncThin {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThin {
                params: p2,
                ret: r2,
            },
        )
        | (
            MirTy::FuncThick {
                params: p1,
                ret: r1,
            },
            MirTy::FuncThick {
                params: p2,
                ret: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1
                    .iter()
                    .zip(p2)
                    .all(|(p, c)| witness_match(p, c, bindings))
                && witness_match(r1, r2, bindings)
        },
        _ => pattern == concrete,
    }
}


/// Apply type parameter substitutions to a MirTy, producing a new type by value.
///
/// Exhaustive match — every variant is handled explicitly (no catch-all).
pub fn substitute_type(ty: &MirTy, subst: &HashMap<Entity, MirTy>) -> MirTy {
    substitute_type_with_self(ty, subst, None)
}

fn contains_self_type(ty: &MirTy) -> bool {
    match ty {
        MirTy::SelfType => true,
        MirTy::Pointer(i) | MirTy::Ref(i) | MirTy::RefMut(i) => contains_self_type(i),
        MirTy::Tuple(es) => es.iter().any(contains_self_type),
        MirTy::Named { type_args, .. } => type_args.iter().any(contains_self_type),
        MirTy::FuncThin { params, ret } | MirTy::FuncThick { params, ret } => {
            params.iter().any(contains_self_type) || contains_self_type(ret)
        },
        MirTy::AssociatedProjection { base, .. } => contains_self_type(base),
        _ => false,
    }
}

/// Substitute type parameters and SelfType in a type.
/// `self_type` provides the concrete type for `MirTy::SelfType` (used in protocol
/// extension methods where Self is abstract until monomorphization).
pub fn substitute_type_with_self(
    ty: &MirTy,
    subst: &HashMap<Entity, MirTy>,
    self_type: Option<&MirTy>,
) -> MirTy {
    let sub = |t: &MirTy| substitute_type_with_self(t, subst, self_type);

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
        | MirTy::Never
        | MirTy::Str
        | MirTy::Error => ty.clone(),

        MirTy::TypeParam(entity) => match subst.get(entity) {
            Some(concrete) => concrete.clone(),
            None => ty.clone(),
        },

        MirTy::SelfType => self_type.cloned().unwrap_or_else(|| ty.clone()),

        MirTy::Pointer(inner) => MirTy::Pointer(Box::new(sub(inner))),
        MirTy::Ref(inner) => MirTy::Ref(Box::new(sub(inner))),
        MirTy::RefMut(inner) => MirTy::RefMut(Box::new(sub(inner))),

        MirTy::Tuple(elems) => MirTy::Tuple(elems.iter().map(|e| sub(e)).collect()),

        MirTy::Named { entity, type_args } => {
            // Check if this Named entity is in the subst map (associated type resolution).
            // e.g., Iterable.Iter entity → ArrayIterator[Int64]
            if type_args.is_empty() {
                if let Some(concrete) = subst.get(entity) {
                    return concrete.clone();
                }
            }
            MirTy::Named {
                entity: *entity,
                type_args: type_args.iter().map(|a| sub(a)).collect(),
            }
        },

        MirTy::AssociatedProjection {
            base,
            protocol,
            name,
        } => {
            // Associated types inside a protocol extension (e.g. `Item` in
            // `extend Iterator { func collect() -> Array[Item] }`) lower with
            // `base = Named(protocol)` — HIR represents bare `Item` as
            // `AssocProjection { base: self_protocol, ... }`. At monomorphization
            // time we want that to behave like `SelfType`, so substitute it
            // with the caller-supplied `self_type` before recursing. Without
            // this, the projection never becomes concrete and layout defaults
            // to ptr (8 bytes), which disagrees with the actual Item layout
            // for sub-i64 types (UInt8, Char-as-UInt32, etc.).
            let sub_base = match base.as_ref() {
                MirTy::Named { entity, type_args } if type_args.is_empty() && entity == protocol => {
                    self_type.cloned().unwrap_or_else(|| sub(base))
                },
                _ => sub(base),
            };
            MirTy::AssociatedProjection {
                base: Box::new(sub_base),
                protocol: *protocol,
                name: name.clone(),
            }
        },

        MirTy::FuncThin { params, ret } => MirTy::FuncThin {
            params: params.iter().map(|p| sub(p)).collect(),
            ret: Box::new(sub(ret)),
        },

        MirTy::FuncThick { params, ret } => MirTy::FuncThick {
            params: params.iter().map(|p| sub(p)).collect(),
            ret: Box::new(sub(ret)),
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
        assert_eq!(cache.layout_of(&MirTy::unit()), Layout::zero(1));
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
        assert_eq!(cache.enum_payload_offset(EnumId::new(0), &[]), 8);
    }

    #[test]
    fn enum_payload_offset_matches_layout_with_misaligned_cases() {
        let target = host_target();
        let mut module = MirModule::new("test");

        // Large but low-alignment payload: size 12, align 4
        let large_entity = dummy_entity(10);
        let mut large_def = StructDef::new(large_entity, "Weird.Large");
        large_def.add_field(FieldDef::new("a", MirTy::I32));
        large_def.add_field(FieldDef::new("b", MirTy::I32));
        large_def.add_field(FieldDef::new("c", MirTy::I32));
        let large_struct = module.add_struct(large_def);

        // Smaller but higher-alignment payload: size 8, align 8
        let aligned_entity = dummy_entity(11);
        let mut aligned_def = StructDef::new(aligned_entity, "Weird.Aligned");
        aligned_def.add_field(FieldDef::new("a", MirTy::I64));
        let aligned_struct = module.add_struct(aligned_def);

        let enum_entity = dummy_entity(1);
        let mut enum_def = EnumDef::new(enum_entity, "Weird");
        enum_def.add_case(EnumCaseDef::new("Large", 0, large_struct));
        enum_def.add_case(EnumCaseDef::new("Aligned", 1, aligned_struct));
        module.add_enum(enum_def);

        let mut cache = LayoutCache::new(&module, &target);
        let layout = cache.enum_layout(EnumId::new(0), &[]);
        let payload_offset = cache.enum_payload_offset(EnumId::new(0), &[]);

        assert_eq!(layout, Layout::new(16, 4));
        assert_eq!(payload_offset, 4);
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
