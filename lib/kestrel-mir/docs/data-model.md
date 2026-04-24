# MIR Data Model

## MirModule

The top-level container. A complete, self-describing snapshot of the compiled program.

```rust
pub struct MirModule {
    pub name: String,

    // Items
    pub structs: Vec<StructDef>,
    pub enums: Vec<EnumDef>,
    pub protocols: Vec<ProtocolDef>,
    pub witnesses: Vec<WitnessDef>,
    pub functions: Vec<FunctionDef>,
    pub statics: Vec<StaticDef>,
    pub closures: Vec<ClosureInfo>,

    // Module-level metadata
    pub entry_point: Option<FunctionId>,
    pub module_init: Option<FunctionId>,

    // Name resolution for display — every entity referenced in the MIR has
    // its qualified name stored here so display() works without the ECS
    pub entity_names: IndexMap<Entity, String>,

    // Witness index for O(1) lookup during monomorphization
    pub witness_index: HashMap<(Entity, MirTy), WitnessId>,
}
```

No shared arenas. No interning. Items reference each other via `Entity` (for ECS-backed
items like types and declarations) or local indices (for MIR-internal items like blocks
and locals).

## Types

Types are by-value. No interning, no `Id<Ty>` indirection. Stored inline wherever
they're used.

```rust
pub enum MirTy {
    // Primitives
    I8, I16, I32, I64,
    F16, F32, F64,
    Bool, Unit, Never, Str,

    // Pointers and references
    Pointer(Box<MirTy>),
    Ref(Box<MirTy>),
    RefMut(Box<MirTy>),

    // Compound
    Tuple(Vec<MirTy>),
    Named { entity: Entity, type_args: Vec<MirTy> },

    // Generics
    TypeParam(Entity),
    SelfType,
    AssociatedProjection {
        base: Box<MirTy>,
        protocol: Entity,
        name: String,
    },

    // Functions
    FuncThin { params: Vec<MirTy>, ret: Box<MirTy> },
    FuncThick { params: Vec<MirTy>, ret: Box<MirTy> },

    // Error/poison
    Error,
}
```

`Entity` references point into the ECS for struct/enum/protocol/type-param identity.
The `entity_names` map on `MirModule` resolves these to strings for display.

## Function Bodies

Function bodies are self-contained. A statement in function A never references a local
in function B. No shared arenas needed.

```rust
pub struct MirBody {
    pub locals: Vec<LocalDef>,    // index = LocalId (u32)
    pub blocks: Vec<BasicBlock>,  // index = BlockId (u32)
    pub entry: BlockId,
    pub param_count: usize,       // first N locals are params
}
```

### Basic Blocks

Statements are inline in their block. Terminators are non-optional — every block must
end with a terminator. Blocks under construction use a separate builder type.

```rust
pub struct BasicBlock {
    pub stmts: Vec<Statement>,
    pub terminator: Terminator,   // NOT Option — enforced at construction
}
```

### Locals

Plain structs, no metadata overhead:

```rust
pub struct LocalDef {
    pub name: String,
    pub ty: MirTy,
}
```

## IDs

MIR-internal IDs are plain `u32` newtypes — indices into the parent container:

```rust
pub struct LocalId(u32);   // index into MirBody.locals
pub struct BlockId(u32);   // index into MirBody.blocks
pub struct FunctionId(u32); // index into MirModule.functions
pub struct StructId(u32);  // index into MirModule.structs
// etc.
```

Cross-crate references (to types, declarations, protocols) use `Entity` from the ECS.

## Item Definitions

### StructDef

```rust
pub struct StructDef {
    pub entity: Entity,
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub fields: Vec<FieldDef>,

    // Precomputed by layout pass
    pub layout: Option<StructLayout>,
    pub drop_fields: Vec<FieldId>,  // fields that need dropping, in order
    pub needs_drop: bool,
}

pub struct StructLayout {
    pub size: u64,
    pub align: u64,
    pub field_offsets: Vec<u64>,
}

pub struct FieldDef {
    pub name: String,
    pub ty: MirTy,
}
```

### EnumDef

```rust
pub struct EnumDef {
    pub entity: Entity,
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub cases: Vec<EnumCaseDef>,
}

pub struct EnumCaseDef {
    pub name: String,
    pub discriminant: u32,
    pub payload_struct: StructId,  // points to a struct in MirModule.structs
}
```

### ProtocolDef

```rust
pub struct ProtocolDef {
    pub entity: Entity,
    pub name: String,
    pub type_params: Vec<TypeParamDef>,
    pub parent_protocols: Vec<Entity>,
    pub associated_types: Vec<AssociatedTypeDef>,
    pub methods: Vec<ProtocolMethodDef>,
}
```

### FunctionDef

```rust
pub struct FunctionDef {
    pub entity: Entity,
    pub name: String,
    pub kind: FunctionKind,
    pub type_params: Vec<TypeParamDef>,
    pub params: Vec<ParamDef>,
    pub ret: MirTy,
    pub where_clause: Option<WhereClause>,
    pub body: Option<MirBody>,    // None for extern functions
    pub extern_info: Option<ExternInfo>,
}

pub enum FunctionKind {
    Free,
    Method { parent: Entity, receiver: ReceiverConvention },
    StaticMethod { parent: Entity },
    Initializer { parent: Entity },
    Deinit { parent: Entity },
    ClosureCall { env_struct: StructId },
    Thunk { original: Entity },
    ModuleInit,
}

pub enum ReceiverConvention {
    Ref,       // &Self
    RefMut,    // &var Self
    Consuming, // Self by value
}

pub struct ParamDef {
    pub name: String,
    pub local: LocalId,
    pub ty: MirTy,
    pub external_label: Option<String>,
}
```

### WitnessDef

```rust
pub struct WitnessDef {
    pub implementing_type: MirTy,
    pub protocol: Entity,
    pub protocol_type_args: IndexMap<String, MirTy>,
    pub type_params: Vec<TypeParamDef>,
    pub type_bindings: IndexMap<String, MirTy>,
    pub method_bindings: IndexMap<String, MethodBinding>,
}

pub struct MethodBinding {
    pub implementation: Entity,
    pub type_args: Vec<MirTy>,
    pub source: MethodSource,
}

pub enum MethodSource {
    /// Method defined directly on the implementing type
    Direct,
    /// Default implementation from protocol extension
    Extension { protocol: Entity },
}
```

### StaticDef

```rust
pub struct StaticDef {
    pub entity: Entity,
    pub name: String,
    pub ty: MirTy,
    pub is_mutable: bool,
    pub initializer: Option<Immediate>,
    pub init_order: u32, // topologically sorted
    pub file_constant_data: Option<FileConstantData>,
}
```

### ClosureInfo

```rust
pub struct ClosureInfo {
    pub env_struct: StructId,
    pub call_function: FunctionId,
    pub captures: Vec<CaptureInfo>,
}

pub struct CaptureInfo {
    pub name: String,
    pub ty: MirTy,
    pub mode: CaptureMode,
}

pub enum CaptureMode {
    ByRef,
    ByMutRef,
    ByMove,
    ByCopy,
}
```

## TypeParamDef

```rust
pub struct TypeParamDef {
    pub entity: Entity,
    pub name: String,
}
```

Type parameter ownership (which item owns this type param) is implicit from where the
def appears — in the owning item's `type_params` vec.
