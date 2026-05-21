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
