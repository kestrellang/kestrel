# Monomorphization

Monomorphization is a MIR-to-MIR transformation that consumes the generic
`MirModule` and produces a concrete `MonoModule`. It resolves all generic
types, witness dispatch, and associated type projections, producing a
self-contained module where every function has a concrete body and every
type has a computed layout.

## Why it's a MIR pass, not a codegen concern

In kestrel-mir-1, monomorphization lived in codegen. The monomorphizer
discovered instantiations but didn't produce concrete MIR bodies — codegen
carried `subst: HashMap<Entity, MirTy>` per function and applied it at
every value, type, and call site during emission. This had consequences:

- **841 lines** of per-function type substitution in codegen
- **Witness resolution happened twice** (discovery + call emission)
- **Enum layout was ad-hoc** — computed during codegen, not precomputed
- **Every codegen function threaded substitution maps** through every helper

Moving monomorphization to a MIR pass eliminates all of this. Codegen
receives fully concrete IR and emits it directly.

## MonoModule

```rust
struct MonoModule {
    functions: Vec<MonoFunction>,
    structs: Vec<MonoStruct>,
    enums: Vec<MonoEnum>,
    statics: Vec<MonoStatic>,
    ty_arena: TyArena,                       // shared, append-only
    entity_names: IndexMap<Entity, String>,   // for diagnostics
    // No protocols. No witnesses. No type_params.
}
```

### MonoFuncId

```rust
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct MonoFuncId(u32);  // indexes into MonoModule.functions
```

Every callable in the MonoModule is referenced by index, not by Entity.
The monomorphizer builds the mapping during instantiation. Codegen never
does entity lookup — it indexes directly.

### MonoFunction

```rust
struct MonoFunction {
    name: String,               // mangled name, e.g. "Array$Int64$.append"
    source: Entity,             // back-reference to generic origin
    type_args: Vec<TyId>,       // what this was instantiated with
    self_type: Option<TyId>,    // for methods — part of the dedup key
    params: Vec<MonoParam>,
    ret: TyId,
    body: Option<MonoBody>,     // None for extern functions
    extern_info: Option<ExternInfo>,
}
```

### No separate mono IR types

MonoModule bodies reuse the same `MirBody`, `StatementKind`, `Rvalue`,
`Place`, `Operand`, and `Terminator` types as generic MIR. There is no
`MonoBody`, `MonoStatementKind`, or `MonoRvalue`.

Instead, the shared enums have variants for both stages:

```rust
enum Callee {
    Direct { func: Entity, type_args: Vec<TyId>, self_type: Option<TyId> },
    Resolved(MonoFuncId),       // post-mono: direct call by index
    Thin(Place),
    Thick(Place),
    Witness { ... },            // pre-mono only
}

enum ImmediateKind {
    FunctionRef { func: Entity, type_args: Vec<TyId>, self_type: Option<TyId> },
    MonoFunctionRef(MonoFuncId),  // post-mono: function ref by index
    ...
}
```

The verifier enforces stage-appropriate variants:
- **Generic MIR**: `Direct`, `Thin`, `Thick`, `Witness` allowed. `Resolved` rejected.
- **Mono MIR**: `Resolved`, `Thin`, `Thick` allowed. `Direct`, `Witness` rejected.

Same pattern for `ImmediateKind::FunctionRef` (generic) vs `MonoFunctionRef` (mono).

`Rvalue::ApplyPartial { func: Entity }` keeps its `Entity` field. During
Phase 3, monomorphization rewrites it by looking up the entity in the
`(Entity, type_args, self_type) → MonoFuncId` mapping and replacing the
Call that the thunk wraps. The ApplyPartial itself references the thunk's
entity, which the monomorphizer resolves.

**Why no duplication:** Every new statement variant or rvalue variant
exists once. Display, operand traversal, and passes operate on one set
of types. The mono/generic distinction is enforced by the verifier, not
the type system. This matches how `ImmediateKind` already works.

### Entity references in MonoModule

After monomorphization, `Entity` survives in positions where it serves
as a type identity key:

- `PlaceBase::Global(Entity)` — statics are module-global; codegen resolves
  by entity from `MonoModule.statics`
- `MirTy::Named { entity, type_args }` — struct/enum identity; codegen
  looks up `MonoStruct`/`MonoEnum` by `(entity, type_args)` pair
- `Rvalue::ApplyPartial { func: Entity }` — thunk entity; codegen resolves
  via `MonoModule.functions`

Direct call targets (`Callee::Resolved(MonoFuncId)`) and function
references (`ImmediateKind::MonoFunctionRef(MonoFuncId)`) use indices
into `MonoModule.functions` — no entity lookup needed for the hot path.

### MonoStruct / MonoEnum

```rust
struct MonoStruct {
    source: Entity,
    type_args: Vec<TyId>,
    fields: Vec<MonoField>,
    type_info: TypeInfo,        // layout is always Some — fully computed
}

struct MonoEnum {
    source: Entity,
    type_args: Vec<TyId>,
    cases: Vec<MonoEnumCase>,
    type_info: TypeInfo,        // layout is always Some
    discriminant_width: IntBits,
    payload_offset: u32,
}
```

Codegen reads `type_info.layout` directly. No computation, no substitution,
no struct-def lookup chains.

## Monomorphization algorithm

### Phase 1: Instantiation discovery

BFS from entry points (non-generic, non-closure functions):

1. Seed the worklist with all non-generic function defs that have bodies
2. For each function in the worklist:
   a. Walk the body for callees (Direct, Witness, ApplyPartial targets)
   b. Substitute type args to get concrete callee signatures
   c. For Witness callees: resolve to concrete function via witness lookup
   d. Record each `(func_entity, type_args, self_type)` as an instantiation
   e. Add new instantiations to the worklist
3. Continue until no new instantiations are discovered

The discovery uses a dedup set: `HashSet<(Entity, Vec<TyId>, Option<TyId>)>`
to avoid processing the same instantiation twice.

### Phase 2: Body monomorphization + witness resolution

For each discovered instantiation:

**Step 1: Build the SubstMap.**

```rust
struct SubstMap {
    type_params: HashMap<Entity, TyId>,
    self_type: Option<TyId>,
    assoc_types: HashMap<(TyId, Entity, Entity), TyId>,
}
```

Populate `type_params` from the instantiation key. Set `self_type` for
methods. Then resolve ALL reachable associated types up front:

For each `(protocol, assoc_type)` pair reachable from the function's
type params and where-clause constraints:
1. Substitute the protocol's self_type with the concrete type from
   the SubstMap
2. Look up the `WitnessCache` (populated by Phase 1)
3. Read the `type_bindings` entry for the associated type entity
4. Substitute the binding's type params using the witness match bindings
5. Add to `SubstMap.assoc_types`

This pre-resolution means the body walk is a **single pass** — no
fallback strategies, no re-resolution during the walk. The existing
code's three-fallback complexity (`subst values → parent_self → all
protocols`) collapses because everything is pre-resolved.

**Step 2: Clone and substitute the body.**

Clone the generic `MirBody`. Walk every type reference and apply
`substitute(arena, ty, &subst)`. This is one pass over all locals,
statements, rvalues, callees, and terminators.

**Step 3: Resolve witness callees.**

Walk every `Callee::Witness` in the substituted body:
1. The self_type and method_type_args are already concrete (substituted
   in step 2)
2. Look up the `WitnessCache` — cache hit from Phase 1 in most cases
3. Record the resolved `(func_entity, type_args, self_type)` triple
   alongside the callee for Phase 3 rewriting

### Witness cache

```rust
struct WitnessCache {
    resolved: HashMap<(Entity, TyId), WitnessCacheEntry>,
}

struct WitnessCacheEntry {
    witness_idx: usize,
    bindings: HashMap<Entity, TyId>,   // pattern match bindings
}
```

Keyed by `(protocol_entity, concrete_self_type)`. Phase 1 populates
during BFS. Phase 2 reads. Cache misses (rare — dead code paths) trigger
fresh witness resolution using the same logic.

### Phase 3: ID assignment + drop expansion + callee rewriting

After ALL bodies are monomorphized, three things happen in one pass:

**Drop expansion.** Every `Drop { place }` where `place` has type `T`
is rewritten to `Call { callee: Direct(__drop$T), args: [(place, Move)] }`.
Every `DropIf { place, flag }` is expanded to:
```
Branch { flag } → drop_block, skip_block
drop_block: Call { callee: Direct(__drop$T), args: [(place, Move)] }; Jump continue
skip_block: Jump continue
```

This ensures the BFS from Phase 1 (which already ran) doesn't need to
discover drop shims — instead, drop shim instantiations are collected
during Phase 3 by scanning the expanded Call targets. For each concrete
`__drop$T`, monomorphization clones and substitutes the generic shim body
(same Phase 2 logic, applied to shim bodies). This may discover further
nested shim calls (`__drop$FieldType`), so shim expansion iterates
until no new shims are needed.

After expansion, MonoModule contains NO `Drop`/`DropIf`/`SetDropFlag`
statements — all drops are direct calls and branches. Codegen never
needs to understand drop semantics.

**ID assignment.** Build the mapping
`(Entity, Vec<TyId>, Option<TyId>) → MonoFuncId` from the complete set of
instantiations (including drop shims discovered during expansion).

**Callee rewriting.** Walk all bodies:
1. Replace `Callee::Direct { func, type_args, self_type }` with
   `Callee::Resolved(MonoFuncId)` via the mapping
2. Replace `Callee::Witness` with `Callee::Resolved(MonoFuncId)` using
   the triples recorded in Phase 2
3. Replace `ImmediateKind::FunctionRef` with
   `ImmediateKind::MonoFunctionRef(MonoFuncId)`

MonoFuncIds do NOT exist during Phase 2 — they are assigned here.

### Phase 4: Type and layout resolution

1. Collect all concrete `Named` types referenced in monomorphized bodies
2. For each: substitute into the generic StructDef/EnumDef to get
   concrete field types and TypeInfo
3. Compute layouts using the shared `StructLayout` arithmetic helpers
   (same code used by the non-generic layout pass — see types.md).
   Takes `&TargetConfig` as a parameter for pointer width. All types are concrete,
   so layout computation always succeeds — no `layout: None`.
4. Build MonoStruct/MonoEnum with the computed layouts and TypeInfo
5. All MirTy::AssociatedProjection were already resolved during Phase 2
   body substitution — verify none remain

### Phase 5: Assembly

Construct the MonoModule:
- All MonoFunctions with their MonoFuncIds
- All MonoStructs/MonoEnums with computed layouts
- Verify: no TypeParam, no SelfType, no AssociatedProjection, no Callee::Witness

## Name mangling

MonoFunction names are mangled during Phase 3 to produce unique,
linker-safe symbol names. Codegen reads `mono_func.name` directly.

### v0 scheme grammar

```text
mangled     = "_K0" path receiver? signature? instantiation? self-disambig?
path        = ident | "N" ident+ "E"
ident       = length "_" utf8-bytes
receiver    = "r" | "m" | "c"              (borrow / mutborrow / consuming)
signature   = "Z" param* "E"
param       = ("L" ident)? type             (optional label + type)
instantiation = "I" type+ "E"
self-disambig = "S_" type

type = "i1" | "i2" | "i4" | "i8"          (I8..I64)
     | "f2" | "f4" | "f8"                  (F16..F64)
     | "b" | "v" | "n" | "s" | "X"         (Bool/Unit/Never/Str/Error)
     | "P" type                             (Pointer)
     | "T" type* "E"                        (Tuple)
     | path ("I" type+ "E")?                (Named + optional type args)
     | "F" count "_" type* type "E"         (FuncThin)
     | "C" count "_" type* type "E"         (FuncThick)
```

The mangler operates on concrete types only — no `SelfType` or
`AssociatedProjection` encoding. These are resolved before mangling
runs. If an abstract type reaches the mangler, it's a monomorphization bug.

### Examples

```
main                          → _K04_main
std.Array.count (borrow)      → _K0N3_std5_Array5_countErZE
Array[Int64].append           → _K0N5_Array6_appendErZEIi8E
Iterator.next (self=ArrayIter[Int64]) → ...S_13_ArrayIteratorIi8E
```

## Incremental considerations

Monomorphization is inherently non-incremental — a change to a generic
function affects all its instantiations. Caching strategies:

- Cache MonoFunction by `(source_entity, type_args)` key
- Invalidate when the source function's body changes
- Reuse layout computations when types haven't changed
- This is the same model as Rust's codegen units

The MonoModule is a per-compilation-unit artifact. Fine-grained caching
is a build system concern, not a MIR design concern.
