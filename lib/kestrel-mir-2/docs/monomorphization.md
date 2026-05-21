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

### MonoCallee

```rust
enum MonoCallee {
    Direct(MonoFuncId),    // resolved to a concrete function in this module
    Thin(Place),           // indirect call through function pointer
    Thick(Place),          // indirect call through closure
}
```

Three variants. No Witness (resolved). No type_args (substituted).
No self_type (baked in). Codegen matches three arms.

### MonoBody

```rust
struct MonoBody {
    locals: Vec<LocalDef>,
    blocks: Vec<MonoBasicBlock>,
    param_count: usize,
    entry: BlockId,
    local_scopes: HashMap<LocalId, ScopeId>,
    failure_return_blocks: HashSet<BlockId>,
}

struct MonoBasicBlock {
    stmts: Vec<MonoStatement>,
    terminator: Terminator,     // Terminator is shared (no Entity in callees)
}

struct MonoStatement {
    kind: MonoStatementKind,
    span: Option<Span>,
}
```

MonoBody mirrors MirBody but uses MonoStatement/MonoStatementKind. The
terminator, LocalDef, ScopeId, and block structure are shared types.

### MonoStatementKind

```rust
enum MonoStatementKind {
    Assign { dest: Place, rvalue: MonoRvalue },
    Call { dest: Option<Place>, callee: MonoCallee, args: Vec<(Operand, ArgMode)> },
    Drop { place: Place },
    DropIf { place: Place, flag: LocalId },
    SetDropFlag { flag: LocalId, value: bool },
    ScopeLive(LocalId),
}
```

### MonoRvalue

```rust
enum MonoRvalue {
    // Shared with Rvalue — structurally identical variants
    Use(Operand, UseMode),
    Ref(Place),
    RefMut(Place),
    Op1 { op: Op, arg: Operand },
    Op2 { op: Op, lhs: Operand, rhs: Operand },
    Op3 { op: Op, a: Operand, b: Operand, c: Operand },
    Construct { ty: TyId, fields: Vec<(FieldIdx, Operand, UseMode)> },
    Tuple(Vec<(Operand, UseMode)>),
    EnumVariant { enum_ty: TyId, variant: VariantIdx, payload: Vec<(Operand, UseMode)> },
    ArrayLiteral { element_ty: TyId, values: Vec<(Operand, UseMode)> },

    // Mono-specific: Entity → MonoFuncId
    ApplyPartial { func: MonoFuncId, captures: Vec<(Operand, UseMode)> },
}
```

MonoRvalue replaces `ApplyPartial { func: Entity }` with `func: MonoFuncId`.
All other variants are structurally identical to Rvalue.

`ImmediateKind` gains one mono-specific variant for function references:

```rust
ImmediateKind::MonoFunctionRef(MonoFuncId)
```

This replaces `FunctionRef { func: Entity, type_args, self_type }` after
monomorphization. The mono verifier checks that no generic `FunctionRef`
survives.

### Entity references that survive in MonoModule

After the mono-specific types above, the remaining `Entity` references are:

- `PlaceBase::Global(Entity)` — statics are module-global; codegen resolves
  them by entity from `MonoModule.statics`.
- `ImmediateKind::FunctionRef { func: Entity }` — rewritten to use
  MonoFuncId during Phase 3.
- `MirTy::Named { entity, type_args }` — the struct/enum entity + type_args
  survive for type identity. Codegen looks up MonoStruct/MonoEnum by
  `(entity, type_args)` pair. Multiple instantiations share the same entity
  but differ by type_args.

The pragmatic approach: `Entity` survives in positions where it serves as
a type identity key. Direct call targets and function references are
rewritten to `MonoFuncId`. The mono verifier checks that no unresolved
generic entities remain.

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

1. Build a `SubstMap` (type_params + self_type + assoc_types) for this
   instantiation. Associated types are resolved via witness lookup before
   body substitution begins.
2. Clone the generic MirBody
3. Walk every type reference in the body, applying `substitute(arena, ty, &subst)`
4. Walk every `Callee::Witness`:
   a. Substitute self_type and method_type_args using the SubstMap
   b. Find the matching WitnessDef via pattern matching
   c. Bind pattern variables and construct the concrete function's type args
   d. Record the resolved `(func_entity, type_args, self_type)` triple
      alongside the callee for Phase 3 rewriting

Witness resolution happens here alongside body substitution. Phase 1
already resolved witnesses for *discovery* (reachability). Phase 2
re-resolves for *rewriting*. The resolution logic is shared and cached —
Phase 1's results seed Phase 2 so most resolutions are cache hits.

### Phase 3: ID assignment + callee rewriting

After ALL bodies are monomorphized (so every target is known), assign
MonoFuncIds and rewrite callees:

1. Build the mapping `(Entity, Vec<TyId>, Option<TyId>) → MonoFuncId`
   from the complete set of discovered instantiations
2. Walk all monomorphized bodies, replacing resolved callee triples
   with `MonoCallee::Direct(MonoFuncId)` via the mapping
3. Rewrite `ImmediateKind::FunctionRef` — replace `Entity` + `type_args`
   with the looked-up MonoFuncId. This requires a mono-specific Immediate
   variant: `ImmediateKind::MonoFunctionRef(MonoFuncId)`.
4. `PlaceBase::Global(Entity)` for statics is NOT rewritten — codegen
   resolves statics by entity from `MonoModule.statics`. Entity serves
   as a stable identity for module-global names.

MonoFuncIds do NOT exist during Phase 2. Phase 2 records resolved triples;
Phase 3 assigns IDs and rewrites in a single pass.

### Phase 4: Type and layout resolution

1. Collect all concrete `Named` types referenced in monomorphized bodies
2. For each: substitute into the generic StructDef/EnumDef to get
   concrete field types and TypeInfo
3. Compute layouts using the shared `StructLayout` arithmetic helpers
   (same code used by the non-generic layout pass — see types.md).
   Requires `TargetConfig` for pointer width. All types are concrete,
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
