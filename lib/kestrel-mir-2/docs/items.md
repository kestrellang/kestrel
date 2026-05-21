# Item Definitions

Top-level declarations in the MIR module: functions, structs, enums,
protocols, statics. These are the "nouns" of the module — the instruction
set (ir.md) describes the "verbs" inside function bodies.

## FunctionDef

```rust
struct FunctionDef {
    entity: Entity,
    name: String,                       // fully qualified: "std.Array.append"
    kind: FunctionKind,
    type_params: Vec<TypeParamDef>,
    params: Vec<ParamDef>,
    ret: TyId,
    where_clause: Option<WhereClause>,
    body: Option<MirBody>,              // None for extern functions
    extern_info: Option<ExternInfo>,
}

enum FunctionKind {
    Free,
    Method { parent: Entity, receiver: ReceiverConvention },
    StaticMethod { parent: Entity },
    Initializer { parent: Entity },
    Deinit { parent: Entity },
    ClosureCall { env_struct: Entity },
    Closure { parent_func: Entity },
    Thunk { original: Entity },
    ModuleInit,
}

enum ReceiverConvention { Borrow, MutBorrow, Consuming }
```

### ParamDef

```rust
struct ParamDef {
    name: String,
    local: LocalId,
    ty: TyId,                           // semantic type (String, not Pointer(String))
    convention: ParamConvention,
    external_label: Option<String>,      // for mangling: "at" in `insert(at index:)`
}

enum ParamConvention { Borrow, MutBorrow, Consuming }
```

`ParamDef.ty` is always the unwrapped semantic type. The `convention` field
determines ABI and the local's physical type in the body (see
calling-conventions.md for the ParamDef.ty vs LocalDef.ty reconciliation).

`external_label` is used by the mangler to disambiguate overloads with
different labels.

### ExternInfo

```rust
struct ExternInfo {
    calling_convention: CallingConvention,
    symbol_name: String,                // linker symbol (may differ from Kestrel name)
}

enum CallingConvention { C }
```

### TypeParamDef

```rust
struct TypeParamDef {
    entity: Entity,
    name: String,                       // e.g. "T", "Element"
}
```

### WhereClause

```rust
struct WhereClause {
    constraints: Vec<WhereConstraint>,
}

enum WhereConstraint {
    Implements { type_param: Entity, protocol: Entity },
    NotImplements { type_param: Entity, protocol: Entity },
}
```

`NotImplements` is the opt-out mechanism: `where T: not Copyable` makes
a type parameter affine even though Kestrel is copy-by-default.

## StructDef

```rust
struct StructDef {
    entity: Entity,
    name: String,
    type_params: Vec<TypeParamDef>,
    fields: Vec<FieldDef>,
    type_info: TypeInfo,                // copy + drop + layout (see types.md)
}

struct FieldDef {
    name: String,                       // display name (lookup uses FieldIdx)
    ty: TyId,
}
```

Fields are indexed by `FieldIdx(u16)`. The `name` field is retained for
diagnostics and display, not for lookup. Lowering resolves field names
to FieldIdx at emission time.

## EnumDef

```rust
struct EnumDef {
    entity: Entity,
    name: String,
    type_params: Vec<TypeParamDef>,
    cases: Vec<EnumCaseDef>,
    type_info: TypeInfo,                // copy + drop + layout
}

struct EnumCaseDef {
    name: String,                       // display name (lookup uses VariantIdx)
    discriminant: u32,
    payload_fields: Vec<FieldDef>,      // payload as inline fields
}
```

Variants are indexed by `VariantIdx(u16)`. Discriminant values are
assigned sequentially starting from 0.

The old design backed each enum case's payload with a synthesized
`StructId`. The new design stores payload fields directly on EnumCaseDef
— simpler, no indirection through the struct table. Layout computation
treats payload fields as a sequential struct layout per variant.

## ProtocolDef

```rust
struct ProtocolDef {
    entity: Entity,
    name: String,
    type_params: Vec<TypeParamDef>,
    parent_protocols: Vec<Entity>,
    associated_types: Vec<AssociatedTypeDef>,
    methods: Vec<ProtocolMethodDef>,
}

struct AssociatedTypeDef {
    entity: Entity,                     // for AssociatedProjection lookup
    name: String,
    default: Option<TyId>,
}

struct ProtocolMethodDef {
    name: String,
    type_params: Vec<TypeParamDef>,
    params: Vec<(String, TyId)>,
    ret: TyId,
    has_default: bool,
}
```

`AssociatedTypeDef.entity` is the key used in `MirTy::AssociatedProjection`
and `WitnessDef.type_bindings`. It's an ECS entity, not a string name.

## StaticDef

```rust
struct StaticDef {
    entity: Entity,
    name: String,
    ty: TyId,
    is_mutable: bool,
    initializer: Option<Immediate>,     // compile-time known value
    init_order: u32,                    // topological initialization order
    file_constant_data: Option<FileConstantData>,
}

struct FileConstantData {
    relative_path: String,
    element_ty: TyId,
    base_path: Option<PathBuf>,
}
```

Statics with runtime initializers are handled by a synthesized
`__kestrel_init_statics` function (FunctionKind::ModuleInit).

## WitnessMethodKey

```rust
struct WitnessMethodKey {
    name: String,
    labels: Vec<Option<String>>,        // parameter labels, None for unlabeled
}
```

Disambiguates protocol method overloads by name + label structure.
`foo()` and `foo(bar:)` produce different keys. This captures both
arity and label identity.

## Mono item types

After monomorphization, items become concrete. See monomorphization.md
for the full definitions of:

- `MonoFunction` — mangled name, concrete params/ret, MonoBody or extern
- `MonoStruct` — source entity + type_args, concrete fields, computed layout
- `MonoEnum` — source entity + type_args, concrete cases, computed layout
- `MonoStatic` — concrete type, resolved initializer
- `MonoParam` — concrete type + convention
- `MonoField` — concrete type

```rust
struct MonoParam {
    name: String,
    local: LocalId,
    ty: TyId,                           // concrete
    convention: ParamConvention,
}

struct MonoField {
    name: String,
    ty: TyId,                           // concrete
}

struct MonoStatic {
    entity: Entity,
    name: String,
    ty: TyId,
    is_mutable: bool,
    initializer: Option<Immediate>,
    file_constant_data: Option<FileConstantData>,
}
```

## TargetConfig

```rust
struct TargetConfig {
    pointer_width: u64,                 // bytes (8 for 64-bit targets)
}
```

`TargetConfig` is NOT part of MirModule or MonoModule — the MIR is
target-agnostic. It is passed as a parameter to the two consumers that
need it:

- The non-generic layout pass: `run_layout(module: &mut MirModule, target: &TargetConfig)`
- Monomorphization: `monomorphize(module: MirModule, target: &TargetConfig) -> MonoModule`

Codegen also needs it for ABI decisions (aggregate passing thresholds,
stack alignment). It receives it separately from the compiler driver,
not from the MonoModule.

`TargetConfig` lives in a shared location (kestrel-mir-2 or a tiny
`kestrel-target` crate) so all three consumers can reference it without
depending on each other.

## TyArena implementation

```rust
struct TyArena {
    types: Vec<MirTy>,                  // append-only, index-stable
    intern_map: HashMap<MirTy, TyId>,
}
```

The backing `Vec<MirTy>` must be index-stable — appending new types must
not invalidate references to existing types. Implementation options:

- **`typed_arena::Arena<MirTy>`** — stable references, no invalidation.
  `get()` returns `&MirTy` safely.
- **Page-based allocator** — grows by allocating new pages, never moving
  existing data.
- **`Vec<MirTy>` with `UnsafeCell`** — acceptable if the invariant "no
  outstanding `&MirTy` during `intern()`" is enforced by API design (e.g.
  `intern()` takes `&mut self`, `get()` takes `&self`).

The simplest correct approach: `intern(&mut self)` and `get(&self)`. The
`&mut` prevents concurrent reads during insertion. This means type
interning cannot happen while a pass holds a `&MirTy` reference from
`get()` — passes should copy the `TyId` and work with IDs, not hold
references across interning calls.

If concurrent read+intern is needed (e.g. during body substitution),
use `typed_arena` or a page-based allocator where appending is safe
alongside outstanding references.
