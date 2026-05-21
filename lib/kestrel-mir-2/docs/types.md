# Type System

MIR types, interning, TypeInfo, and the substitution model.

## MirTy

```rust
enum MirTy {
    // === Primitives ===
    I8, I16, I32, I64,
    F16, F32, F64,
    Bool,
    Never,
    Str,

    // === Pointers ===
    Pointer(TyId),          // p[T] — raw pointer

    // === Compound ===
    Tuple(Vec<TyId>),
    Named { entity: Entity, type_args: Vec<TyId> },

    // === Generics (resolved by monomorphization) ===
    TypeParam(Entity),
    SelfType,
    AssociatedProjection { base: TyId, protocol: Entity, assoc_type: Entity },

    // === Function types ===
    FuncThin { params: Vec<(TyId, ParamConvention)>, ret: TyId },
    FuncThick { params: Vec<(TyId, ParamConvention)>, ret: TyId },

    // === Poison ===
    Error,
}
```

### What's not here

**No Ref(T) or RefMut(T).** References are not user-facing types in Kestrel.
The calling convention (borrow/mutating/consuming) lives on ParamDef as a
ParamConvention, not in the type. Borrow-mode parameters have type Pointer(T)
in the body — that's what they physically are. The convention is metadata on
the param, not a type wrapper.

This means:
- "Is this param consuming?" is `param.convention == Consuming`, not
  `!matches!(ty, Ref(_) | RefMut(_))`
- If user-facing references with borrow checking are added later, `Ref(TyId, Region)`
  comes back as a *semantic* type carrying lifetime info — a clean slot, not an
  overloaded one

**Unit is Tuple([]).** No dedicated Unit variant. Aligns with HIR.

## Interning

All MirTy values live in a TyArena. References are TyId (u32 index).

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct TyId(u32);

struct TyArena {
    types: Vec<MirTy>,
    intern_map: HashMap<MirTy, TyId>,
}
```

### Properties

- **Append-only.** TyIds are stable — new types can be interned without
  invalidating existing IDs. This means `&TyArena` for reads and
  interior-mutability for inserts can coexist safely.
- **Structural equality via ID comparison.** `ty_a == ty_b` is a u32
  comparison. No recursive tree walking.
- **One canonical copy per structural type.** `Named { entity: E, type_args: [I64] }`
  exists exactly once in the arena regardless of how many locals have that type.

### Interning API

```rust
impl TyArena {
    fn intern(&self, ty: MirTy) -> TyId;
    fn get(&self, id: TyId) -> &MirTy;

    // Convenience constructors
    fn i64(&self) -> TyId;
    fn bool(&self) -> TyId;
    fn unit(&self) -> TyId;
    fn pointer(&self, pointee: TyId) -> TyId;
    fn named(&self, entity: Entity, type_args: Vec<TyId>) -> TyId;
}
```

The arena uses an index-stable backing store (e.g. `typed_arena::Arena`,
a page-based allocator, or `Vec` behind `UnsafeCell` with the invariant
that no reallocation occurs while references are live). A `RefCell<Vec>`
is NOT safe — appending can invalidate outstanding `&MirTy` references.
The implementation must guarantee that `get()` returns references that
remain valid even after subsequent `intern()` calls.

## Type substitution

Substitution replaces TypeParam/SelfType/AssociatedProjection with concrete
types. It produces new interned TyIds.

```rust
fn substitute(
    arena: &TyArena,
    ty: TyId,
    subst: &SubstMap,
) -> TyId

struct SubstMap {
    type_params: HashMap<Entity, TyId>,    // TypeParam(E) → concrete
    self_type: Option<TyId>,               // SelfType → concrete
    assoc_types: HashMap<(TyId, Entity, Entity), TyId>,  // (base, protocol, assoc_type) → concrete
}
```

One function, in one place. The old design had three independent implementations
of `substitute_type_params` across ty.rs, drop_elaboration.rs, and verify.rs.
With interning, substitution is a walk over the TyId tree that interns the
result — the arena handles deduplication.

`SubstMap` has three components because substitution must handle all three
generic type forms:
- `TypeParam(Entity)` — looked up in `type_params`
- `SelfType` — replaced with `self_type`
- `AssociatedProjection { base, protocol, assoc_type }` — looked up in
  `assoc_types` by `(base, protocol, assoc_type)` triple. The full triple
  is needed because different base types conforming to the same protocol
  can have different associated type resolutions (e.g. `T.Element` vs
  `U.Element` where T: Container and U: Container).

The `assoc_types` map is populated during monomorphization by resolving
associated types through witness tables before body substitution begins.

### Substitution during monomorphization

The monomorphizer builds a `SubstMap` for each function instantiation,
then walks the body substituting all types. The result is a body with no
TypeParam, SelfType, or AssociatedProjection — all concrete TyIds.

## TypeInfo

Unified type metadata. Replaces the scattered CopyBehavior on StructDef,
DeinitBehavior as a separate concept, and layout computed in a separate pass.

```rust
struct TypeInfo {
    copy: CopyBehavior,
    drop: DropBehavior,
    layout: Option<Layout>,
}

enum CopyBehavior {
    Bitwise,              // memcpy is sufficient
    Clone(Entity),        // requires calling clone witness method
    None,                 // affine — must be moved, never copied
}

enum DropBehavior {
    None,                           // no cleanup needed

    // --- Structs ---
    StructDrop {
        deinit: Option<Entity>,     // optional user-defined deinit method
        fields: Vec<FieldIdx>,      // fields that need dropping (may be empty)
    },

    // --- Enums ---
    EnumDrop {
        deinit: Option<Entity>,     // optional user deinit on the enum itself
        variants: Vec<(VariantIdx, Vec<FieldIdx>)>,  // per-variant field drops
    },
}

enum Layout {
    Struct(StructLayout),
    Enum(EnumLayout),
}

struct StructLayout {
    size: u64,
    align: u64,
    field_offsets: Vec<u64>,
}

struct EnumLayout {
    size: u64,                    // total: discriminant + max payload + padding
    align: u64,
    discriminant_width: IntBits,  // I8, I16, or I32 depending on variant count
    payload_offset: u64,          // byte offset of payload after discriminant
    variant_layouts: Vec<StructLayout>,  // per-variant payload layout
}
```

TypeInfo lives on StructDef and EnumDef. Every pass that needs to know
"how does this type behave in memory" reads TypeInfo — one lookup, not
three separate queries.

### How TypeInfo is populated

1. **CopyBehavior** — computed during struct/enum lowering by querying
   the semantic tree (same as today). For generic types, this is a
   conservative upper bound: a struct containing a `T` field gets
   `CopyBehavior::None` unless constrained (`T: Copyable`). The
   per-function where-clause is consulted at use sites to refine.
2. **DropBehavior** — computed during lowering from the deinit method
   presence and field types. FieldCascade lists only fields whose types
   themselves need dropping. For generic fields (`T`), DropBehavior
   conservatively includes them — the drop shim's body conditionally
   calls `__drop$T`, which is a no-op for bitwise types after
   monomorphization.
3. **Layout** — computed in two stages:
   - **Non-generic layout pass** (pre-mono): computes layouts for types
     with no type params using the shared layout arithmetic helpers.
     Generic types get `layout: None`.
   - **Monomorphization Phase 4**: computes layouts for all concrete
     instantiations of generic types. All MonoStruct/MonoEnum have
     `layout: Some(...)` — fully resolved.

### Layout arithmetic

Shared helpers used by both the non-generic layout pass and
monomorphization. These live in kestrel-mir-2 (absorbed from the old
kestrel-codegen crate).

```rust
impl StructLayout {
    /// Sequentially append a field, returning its byte offset.
    fn append_field(&mut self, field_layout: StructLayout) -> u64;

    /// Round size up to alignment boundary.
    fn pad_to_align(&mut self);
}
```

Uses `u64` (not `usize`) throughout for cross-compilation safety — the
host and target may have different pointer widths.

A `TargetConfig { pointer_width: u64 }` is passed as a parameter to
layout computation. The MIR itself is target-agnostic — TargetConfig
is not stored on the module (see items.md).

### Copy behavior queries

```rust
fn copy_behavior(arena: &TyArena, module: &MirModule, ty: TyId) -> CopyBehavior
```

Replaces the recursive `MirTy::copy_behavior(&module)` method. With interned
types, the result can be cached per TyId. For TypeParam, the query checks
where-clause constraints (same logic, but one call site instead of three).

## Generic types in MIR

MIR remains generic until monomorphization. TypeParam(Entity),
SelfType, and AssociatedProjection survive in function bodies and are
resolved during monomorphization.

After monomorphization, MonoModule contains only concrete types. The
MonoStruct and MonoEnum types carry their concrete TypeInfo with fully
computed layouts:

```rust
struct MonoStruct {
    source: Entity,
    type_args: Vec<TyId>,
    fields: Vec<MonoField>,
    type_info: TypeInfo,   // layout is always Some
}
```

Codegen matches `type_info.layout` (always `Some` in MonoModule), unwraps
the `Layout::Struct` or `Layout::Enum` variant, and reads offsets directly.
No computation, no substitution, no struct-def lookup chains.
