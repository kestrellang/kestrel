# Architecture

kestrel-mir-2 is the mid-level intermediate representation for the Kestrel
compiler. It sits between the typed HIR (ECS-based semantic tree) and Cranelift
codegen. The design is place-based and non-SSA, modeled on Rust MIR with
ownership conventions drawn from Swift SIL.

## Module structure

```
MirModule (generic, polymorphic)
├── functions: Vec<FunctionDef>      — signatures + optional bodies
├── structs: Vec<StructDef>          — with TypeInfo (copy + drop + layout)
├── enums: Vec<EnumDef>              — with TypeInfo
├── protocols: Vec<ProtocolDef>      — method requirements
├── witnesses: Vec<WitnessDef>       — conformance tables
├── statics: Vec<StaticDef>          — module-level variables
├── ty_arena: TyArena                — interned types (append-only)
└── entity_names: IndexMap<Entity, String>  — display without ECS
```

```
MonoModule (concrete, monomorphized)
├── functions: Vec<MonoFunction>     — concrete bodies (or extern stubs)
├── structs: Vec<MonoStruct>         — concrete layouts computed
├── enums: Vec<MonoEnum>             — concrete layouts computed
├── statics: Vec<MonoStatic>         — module-level variables
├── ty_arena: TyArena                — shared with generic MIR (append-only)
├── entity_names: IndexMap<Entity, String>  — for diagnostics/display
└── No protocols, witnesses, or type params — resolved away
```

See `items.md` for the full definition of each item type (FunctionDef,
StructDef, EnumDef, ProtocolDef, StaticDef, and their mono equivalents).

## Layer diagram

```
Operand              — Place(Place) | Const(Immediate)
   │                   mode-free; just "what" is being read
   ▼
Rvalue               — Use(Operand, UseMode) | Ref(Place) | RefMut(Place)
   │                   Op1/Op2/Op3 | Construct | Tuple | EnumVariant
   │                   ArrayLiteral | ApplyPartial
   ▼
Statement            — Assign | Call | Drop | DropIf | SetDropFlag | ScopeLive
   │                   Assign/Call are primary; Drop variants emitted by drop elab
   ▼
BasicBlock           — Vec<Statement> + Terminator
   │
   ▼
MirBody              — Vec<BasicBlock> + Vec<LocalDef> + scope info
   │
   ▼
FunctionDef          — signature + body + where clauses
   │
   ▼
MirModule            — all items, the type arena, entity names
```

## Crate structure

```
kestrel-mir-2         — IR types, passes, monomorphization, layout, mangling
kestrel-codegen-cranelift — Cranelift emission only (reads MonoModule)
```

The old `kestrel-codegen` crate (layout cache, mangling, target config)
dissolves into kestrel-mir-2. Layout computation, name mangling, and
type substitution all run during monomorphization — before codegen sees
the module. Codegen is a pure emitter: it reads concrete layouts and
mangled names, never computes them.

### Module layout

```
kestrel-mir-2/src/
├── lib.rs              — MirModule, pub use, pipeline entry point
├── ty.rs               — MirTy, TyId, TyArena
├── ty_query.rs         — copy_behavior(), needs_drop() — shared type queries
├── place.rs            — Place, PlaceBase, PlaceElem, FieldIdx, VariantIdx
├── operand.rs          — Operand, UseMode, ArgMode
├── statement.rs        — StatementKind, Rvalue, Callee, Statement
├── terminator.rs       — TerminatorKind, SwitchCase, Terminator
├── body.rs             — MirBody, BasicBlock, LocalDef, ScopeId
├── immediate.rs        — ImmediateKind, Immediate
├── op.rs               — Op, IntBits, FloatBits, Signedness
├── item/
│   ├── mod.rs          — TypeInfo, CopyBehavior, DropBehavior, TypeParamDef
│   ├── function.rs     — FunctionDef, ParamDef, ExternInfo, FunctionKind
│   ├── struct_def.rs   — StructDef, FieldDef
│   ├── enum_def.rs     — EnumDef, EnumCaseDef
│   ├── protocol.rs     — ProtocolDef, AssociatedTypeDef, ProtocolMethodDef
│   ├── witness.rs      — WitnessDef, WitnessMethodBinding, WitnessMethodKey
│   └── static_def.rs   — StaticDef, FileConstantData
├── layout.rs           — StructLayout, EnumLayout, layout arithmetic helpers
├── substitute.rs       — SubstMap, substitute()
├── builder.rs          — ModuleBuilder, FunctionBuilder, BlockBuilder (pub)
├── display.rs          — MIR pretty-printing
├── passes/
│   ├── mod.rs          — pass orchestration
│   ├── dataflow.rs     — CfgInfo, forward_fixpoint, backward_fixpoint
│   ├── liveness.rs     — backward liveness (used by clone elab, verifier)
│   ├── init_state.rs   — forward init tracking (used by drop elab, verifier)
│   ├── clone_elab.rs   — clone elaboration (transformation)
│   ├── drop_elab.rs    — drop insertion (transformation)
│   ├── drop_shim.rs    — __drop$T synthesis from type graph
│   ├── layout.rs       — non-generic layout pass
│   └── verify.rs       — generic MIR verification
└── mono/
    ├── mod.rs          — monomorphize() entry point, MonoModule
    ├── collect.rs      — instantiation discovery (BFS)
    ├── witness.rs      — witness resolution
    ├── mangle.rs       — name mangling (v0 scheme)
    ├── types.rs        — MonoFunction, MonoBody, MonoCallee, MonoRvalue, etc.
    └── verify.rs       — mono verification
```

### Public API

The compiler driver calls one entry point:

```rust
pub fn lower_and_monomorphize(
    module: MirModule,
    target: &TargetConfig,
) -> Result<MonoModule, VerifyResult>
```

This runs the full pipeline: clone elab → drop elab → layout → verify →
monomorphize → mono verify. Returns `Err(VerifyResult)` if any
verification stage fails (ICE). The lowering crate (`kestrel-mir-lower`)
produces the `MirModule`; this function takes it from there.

Individual passes are also `pub` for testing, but the pipeline entry
point is the expected interface for production use.

### Pretty-printing

`MirModule` and `MonoModule` implement a text display format for
debugging, accessed via `.display() -> impl fmt::Display`. The format
is human-readable pseudocode, not a serialization format. See the
display format spec below. Entity names are resolved via
`MirModule.entity_names`. TyIds are resolved via the arena. The display
implementation lives in `display.rs` and is available from day one.

### Display format

**Types:**
```
i8  i16  i32  i64  f16  f32  f64  bool  !  str  <error>
p[i32]                              — Pointer
(i32, bool)                         — Tuple
()                                  — Unit (empty tuple)
std.Array[i64]                      — Named with type args
T                                   — TypeParam
Self                                — SelfType
(Iterator.Item for T)               — AssociatedProjection
func(i32, i32) -> bool              — FuncThin
func escaping(i64) -> ()            — FuncThick
```

**Places** (flat, with index-based projections):
```
%x                                  — Local
@std.globalVar                      — Global
%s.0                                — Field (FieldIdx 0)
%s.1.0                              — Nested field
%e:2                                — Downcast (VariantIdx 2)
(deref %p)                          — Deref
(deref %p).0                        — Deref + field
```

Field projections use `.N` (numeric index). Downcasts use `:N` to
distinguish from fields. Display can optionally resolve indices to
names via StructDef/EnumDef for readability: `%s.name` instead of `%s.0`,
`%e:Some` instead of `%e:0`.

**Operands:**
```
%x                                  — Operand::Place (bare)
42_i64                              — Operand::Const (int literal)
true                                — Operand::Const (bool)
"hello"                             — Operand::Const (string literal)
()                                  — Operand::Const (unit)
std.foo[i64]                        — Operand::Const (function ref)
```

**Rvalues:**
```
use copy %x                         — Use(Place, Copy)
use move %x                         — Use(Place, Move)
ref %x                              — Ref(Place)
ref_mut %x                          — RefMut(Place)
add.i64 %a, %b                     — Op2
neg.i32 %a                          — Op1
fma.f64 %a, %b, %c                 — Op3
construct std.Point { .0: copy %x, .1: copy %y }
tuple (copy %a, move %b)
enum Optional[i64]:0 (copy %val)    — EnumVariant (VariantIdx 0)
array[i64] [copy %a, copy %b]
apply_partial std.foo (move %cap)
```

Compound operands show their mode: `copy %x` or `move %x`.

**Statements:**
```
%dest = <rvalue>                    — Assign
%ret = call std.Array.append(%self, %val) [ref, move]
                                    — Call with ArgMode annotations
call std.print(%msg) [ref]          — Call without return value
drop %x                             — Drop
drop %x if %flag                    — DropIf
%flag = true                        — SetDropFlag
scope_live %i                       — ScopeLive
```

Call args show their ArgMode in brackets after the arg list:
`[ref, move, copy]`. This makes the calling convention visible at a glance.

**Terminators:**
```
return %x                           — Return
jump bb3                            — Jump
branch %cond -> bb1, bb2            — Branch
switch %disc -> [0: bb1, 1: bb2, _: bb3]
                                    — Switch (VariantIdx or literal)
panic "index out of bounds"         — Panic
unreachable                         — Unreachable
```

**Function signatures:**
```
fn std.Array.append[T](&var self: p[Array[T]], value: T) -> () {
  locals:
    %self: p[Array[T]]              — param 0 (borrow)
    %value: T                       — param 1 (consuming)
    %_t0: i64                       — temp
  bb0:
    ...
  bb1:
    ...
}

fn __drop$MyStruct(consuming self: MyStruct) -> () { ... }

extern fn libc.malloc(size: i64) -> p[()] [C, symbol="malloc"]
```

Parameter convention is shown as `&` (borrow), `&var` (mutborrow), or
bare (consuming) before `self`/param name. Extern functions show calling
convention and symbol name in brackets.

**Module-level:**
```
module "main"

struct std.Point { x: i64, y: i64 }
  copy: Bitwise  drop: None  layout: { size: 16, align: 8, offsets: [0, 8] }

enum std.Optional[T] { None(0), Some(1) }
  copy: <from T>  drop: EnumDrop { variants: [(1, [0])] }

protocol std.Equatable { fn equals(&self, &other) -> bool }

witness std.Array[T]: std.Equatable where T: Equatable
  equals -> std.Array.equals[T]

static @std.VERSION: str = "1.0"

fn std.main() -> () { ... }
fn std.Array.append[T](...) -> () { ... }
```

## Pass pipeline

```
HIR (ECS semantic tree)
 │
 ▼  kestrel-mir-lower
MirModule (generic)
 │
 ├─ clone elaboration    — Copy of Clone types → explicit witness clone calls
 ├─ drop elaboration     — dataflow → drop shim calls at scope exits
 ├─ layout (non-generic) — struct + enum sizes for non-generic types
 ├─ verify (generic)     — structural + ownership invariants
 │
 ▼  monomorphization     — produces MonoModule
MonoModule (concrete)   — includes layout for all types, mangled names
 │
 ├─ verify (mono)        — no TypeParam, no Witness, all layouts present
 │
 ▼  kestrel-codegen-cranelift
native code
```

Passes run on `&mut MirModule` in the order above. Clone elaboration must
precede drop elaboration (drop elab assumes Clone copies have been rewritten
to Move-of-clone-temp). Layout runs after drop elaboration because drop
shims may introduce new struct references. The non-generic layout pass
computes layouts for types with no type params; generic type layouts are
computed during monomorphization when all type args are concrete.

Verification runs twice: once on generic MIR (structural + ownership checks)
and once post-monomorphization (no unresolved types, no witness calls, all
layouts present).

Monomorphization consumes the generic `MirModule` and produces a `MonoModule`
where every type is concrete, every witness call is resolved to a direct call,
every type has a computed layout, and every function has a mangled symbol name.
Codegen receives `&MonoModule` — no substitution maps, no witness resolution,
no layout computation, no name mangling.

## Key invariants

- **No MirTy::TypeParam in MonoModule.** Monomorphization resolves all generic
  types. The mono verifier checks this.
- **No Callee::Witness in MonoModule.** All witness dispatch is resolved to
  MonoCallee::Direct(MonoFuncId). Codegen never does witness lookup.
- **MonoFunctions have bodies or are extern.** Most MonoFunctions have concrete
  bodies. Extern functions (FFI, runtime) survive as bodyless stubs with
  `extern_info` — codegen emits an import, not a body. Intrinsics are lowered
  to Op variants during HIR-to-MIR lowering and don't appear as calls.
- **Types are interned.** All MirTy values live in the TyArena. References
  are TyId (u32 index). Equality is ID comparison.
- **Places are flat.** `Place { base, projections }` — no heap-allocated
  recursive tree. Projection elements use indices (FieldIdx, VariantIdx),
  not strings.
- **Ownership is on the use site.** Operands are mode-free. UseMode (Copy|Move)
  appears on Rvalue::Use and compound rvalue operands. ArgMode
  (Copy|Move|Ref|RefMut) appears on call arguments. Ref/RefMut are separate
  Rvalue variants for explicit reference creation.
- **Drop shims, not iterative expansion.** One `__drop$T` function per type
  that needs cleanup. Drop elaboration inserts `Drop { place }` and
  `DropIf { place, flag }` statements that call the shim. No intermediate
  Deinit markers, no fixed-point expansion loop.
- **Overwrite drops.** Assigning to an already-live droppable place first
  drops the old value. Drop elaboration inserts a `Drop` before the
  reassignment when the place is known live, or `DropIf` when maybe-live.
- **No partial moves in MIR-2 v1.** Ownership dataflow tracks root locals.
  Moving a projected field out of an owned droppable aggregate is rejected by
  the verifier until projection-aware move paths are implemented. Field
  assignment may still overwrite-drop the old field value when the parent is
  definitely initialized.

## Migration from kestrel-mir-1

kestrel-mir-2 is a new crate, not an in-place rewrite. The migration path:

1. **Build kestrel-mir-2 types alongside kestrel-mir.** Both crates exist
   simultaneously. kestrel-mir-2 defines `MirModule`, `MirBody`, `Operand`,
   `Rvalue`, `Place`, `MirTy`, `TyArena`, etc. as new types.

2. **Rewrite kestrel-mir-lower to emit kestrel-mir-2 types.** This is the
   main migration effort. The lowering is already organized by construct
   (calls, closures, match, literals) — each section migrates independently.
   Key changes during migration:
   - `Value::Copy/Move/Ref/RefMut` → `(Operand, UseMode)` or `(Operand, ArgMode)`
   - `Place::Field { parent: Box<Place>, name }` → `Place { base, projections }`
     with `PlaceElem::Field(FieldIdx)` — resolve field names to indices
   - `MirTy` by value → `TyId` via arena interning
   - `CallArg` eliminated — call args are `Vec<(Operand, ArgMode)>`
   - Clone insertion for consuming call args removed from lowering (clone
     elaboration handles it uniformly via liveness)

3. **Port passes.** Clone elaboration, drop elaboration, layout, and
   verification are rewritten against kestrel-mir-2 types. The shared
   dataflow infrastructure (`CfgInfo`, `forward_fixpoint`, `backward_fixpoint`)
   is built first — all passes depend on it.

4. **Build monomorphization.** New pass, absorbing logic from
   kestrel-codegen-cranelift's `monomorphize/` module and kestrel-codegen's
   `layout.rs` + `mangle.rs`.

5. **Rewrite kestrel-codegen-cranelift against MonoModule.** Codegen takes
   `&MonoModule` instead of `&MirModule`. The monomorphize/ directory, type
   substitution, witness resolution, and layout cache are deleted.

6. **Delete kestrel-mir, kestrel-ownership, kestrel-codegen.** Once
   kestrel-mir-2 is fully wired and tests pass, the old crates are removed.

Steps 1-3 can proceed without touching codegen. Step 4-5 can be done
together. The key constraint: both MIR representations cannot be active
in the same compilation pipeline simultaneously — the switchover from
kestrel-mir to kestrel-mir-2 happens in one commit that rewires
`kestrel-compiler`'s pipeline.
