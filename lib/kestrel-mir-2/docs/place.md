# Place Model

A place is a path to a memory location: a local or global variable,
optionally followed by a chain of projections (field access, tuple index,
enum downcast, dereference).

## Representation

```rust
struct Place {
    base: PlaceBase,
    projections: SmallVec<[PlaceElem; 2]>,
}

enum PlaceBase {
    Local(LocalId),
    Global(Entity),
}

enum PlaceElem {
    Field(FieldIdx),         // struct field by index
    TupleIndex(u32),         // tuple element by position
    Downcast(VariantIdx),    // enum variant refinement (valid after switch)
    Deref,                   // pointer/reference dereference
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct FieldIdx(u16);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct VariantIdx(u16);
```

## Why flat, not a recursive tree

The old design used a recursive boxed enum:

```rust
// Old design — heap-allocated tree
enum Place {
    Local(LocalId),
    Field { parent: Box<Place>, name: String },
    Downcast { parent: Box<Place>, variant: String },
    ...
}
```

Problems:
- Every `.field()` chain allocates a Box on the heap
- `clone()` walks the tree recursively
- `root_local()` is a recursive traversal
- Overlap checking (does place A conflict with place B?) requires
  recursive tree comparison
- String-keyed fields and variants enable display-name leak bugs and
  require string comparison at codegen

The flat representation fixes all of these:
- `Place` is a base + inline SmallVec — no heap allocation for ≤2
  projections (the common case: `local.field` or `local.downcast`)
- `clone()` is a memcpy
- `root_local()` is `self.base`
- Overlap checking is a slice prefix check
- FieldIdx/VariantIdx are resolved at lowering time — no strings in the IR

## Construction

Builder methods for ergonomic construction in the lowering:

```rust
impl Place {
    fn local(id: LocalId) -> Self;
    fn global(entity: Entity) -> Self;

    fn field(mut self, idx: FieldIdx) -> Self {
        self.projections.push(PlaceElem::Field(idx));
        self
    }

    fn tuple_index(mut self, i: u32) -> Self {
        self.projections.push(PlaceElem::TupleIndex(i));
        self
    }

    fn downcast(mut self, variant: VariantIdx) -> Self {
        self.projections.push(PlaceElem::Downcast(variant));
        self
    }

    fn deref(mut self) -> Self {
        self.projections.push(PlaceElem::Deref);
        self
    }
}
```

Chaining works the same as the old API: `Place::local(x).field(f).deref()`.
The difference is internal — projections append to a SmallVec instead of
wrapping in Box.

## Overlap and prefix checking

Two places conflict if one is a prefix of the other (including equality).
This is the core operation for move checking and borrow checking.

```rust
impl Place {
    fn conflicts_with(&self, other: &Place) -> bool {
        self.base == other.base && (
            self.projections.starts_with(&other.projections) ||
            other.projections.starts_with(&self.projections)
        )
    }

    fn is_prefix_of(&self, other: &Place) -> bool {
        self.base == other.base &&
            other.projections.starts_with(&self.projections)
    }
}
```

Examples:
- `s` conflicts with `s.f` (prefix)
- `s.f` conflicts with `s.f.g` (prefix)
- `s.f` does NOT conflict with `s.g` (divergent projections)
- `s.f` conflicts with `s` (s is a prefix of s.f)

## Future: move path tree

MIR-2 v1 does not support partial moves. Ownership and drop dataflow track
root locals, and the verifier rejects moving a projected field out of an
owned droppable aggregate. Places are still flat and indexed now, so the
future move-path representation can be added without changing the surface
shape of the IR.

For projection-aware ownership analysis, places will be interned into a tree
of move paths. Each node represents a trackable place (a local, a field, a
variant payload). The tree enables projection-aware tracking:

```
MovePathId(0): s           — root local
├── MovePathId(1): s.f     — field projection
│   └── MovePathId(3): s.f.g  — nested field
└── MovePathId(2): s.h     — sibling field
```

Moving `s.f` kills MovePathId(1) and propagates "partial" up to
MovePathId(0) — `s` is partially moved. `s.h` (MovePathId(2)) remains live.

The future dataflow lattice will use bitsets over MovePathId, with parent/child
propagation rules:
- Moving a child marks the parent as "partially moved"
- Moving a parent kills all children
- Initializing a child may restore the parent if all children become live

See `passes.md` for the current root-local restriction and how move paths will
extend drop elaboration and move checking later.

## Index resolution

FieldIdx and VariantIdx are resolved during HIR-to-MIR lowering. The
lowering looks up the field/variant position in the StructDef/EnumDef and
emits the index. The MIR never contains field or variant names — names are
a display concern, handled by StructDef/EnumDef when printing.

The mapping is:
- `StructDef.fields[idx]` → field name and type
- `EnumDef.cases[idx]` → variant name, discriminant, and payload

This makes the MIR immune to field/variant renaming and eliminates the
display-name-leak bug class (parens in variant names, qualified names
leaking into switch cases).
