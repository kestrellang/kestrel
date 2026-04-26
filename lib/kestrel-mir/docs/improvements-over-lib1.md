# Improvements Over lib1 Execution Graph

This document catalogs every deliberate change from the lib1 `kestrel-execution-graph`
crate and the rationale behind each.

## Structural Changes

### No shared arenas or MirContext god-object

**lib1**: All MIR data lives in a single `MirContext` with 16+ arenas. Statements,
blocks, locals, types, and names all stored in separate arenas with `Id<T>` handles.

**lib**: `MirModule` contains items as plain `Vec`s. Function bodies (`MirBody`) store
locals and blocks inline. Statements live inside their blocks. No shared arenas.

**Why**: Function bodies are self-contained — a statement in function A never references
a local in function B. Shared arenas add indirection with no benefit. Plain vecs with
`u32` indices are simpler, faster, and cache-friendly.

### Types by value, no interning

**lib1**: Types are interned in a `HashMap<MirTy, Id<Ty>>`. Every type reference is an
`Id<Ty>` that requires a lookup in the context.

**lib**: `MirTy` is used by value wherever it appears. No interning, no `Id<Ty>`.

**Why**: Type interning is only valuable when types are frequently compared for equality
or used as map keys. In the MIR, types are mostly just read during codegen. By-value
types eliminate the interning machinery and the `MirContext` dependency for type access.

### Entity references instead of Id<QualifiedName>

**lib1**: Items reference each other via `Id<QualifiedName>` — an interned qualified
name string. Display works standalone but identity is string-based.

**lib**: Items reference each other via `Entity` from the ECS. An `entity_names` map
on `MirModule` stores resolved names for display.

**Why**: Entity identity is already established by the ECS. Using it avoids building a
parallel naming/identity system. Display still works standalone because names are stored
alongside the entity references.

### Non-optional terminators

**lib1**: `BasicBlock.terminator` is `Option<Terminator>`. Blocks can exist without a
terminator, which is a construction convenience but allows invalid MIR.

**lib**: `BasicBlock.terminator` is `Terminator` (non-optional). The builder enforces
this — you can't finalize a block without a terminator.

**Why**: Every basic block must have a terminator. Making this a type-level invariant
eliminates a class of "forgot to set terminator" bugs.

### Inline statements in blocks

**lib1**: Statements are stored in a separate `Arena<Statement, StatementData>`. Blocks
hold `Vec<Id<Statement>>`.

**lib**: Blocks hold `Vec<Statement>` directly. No statement arena, no statement IDs.

**Why**: Statements are only ever accessed through their block. There's no cross-block
statement reference. Inline storage avoids an indirection per statement.

## Instruction Set Changes

### Unified Op enum replaces BinOp + UnOp + scattered intrinsics

**lib1**: Three separate categories — `BinOp` (38 variants), `UnOp` (8 variants), and
~25 intrinsic-specific `Rvalue` variants for pointer ops, string ops, float ops, etc.

**lib**: Single `Op` enum with all operations. Arity enforced at the `Rvalue` level
via `Op1`/`Op2`/`Op3` variants.

**Why**: The categorical split was arbitrary. `i64.add(a, b)` is the same shape as
`ptr.offset(ptr, offset)`. A unified enum means codegen has one dispatch point, and
adding a new operation means adding one `Op` variant — not touching `Rvalue`.

### BinOp/UnOp carry width

**lib1**: `BinOp::AddSigned` displays as `"i64.add.signed"` regardless of actual
operand width. Width information is lost.

**lib**: `Op::Add(IntBits::I32, Signedness::Signed)` carries the actual width.

**Why**: The MIR should be self-describing. An add on two `i32` values should not print
as `i64.add.signed`. Codegen shouldn't need to infer width from operand types.

### Call is a statement, not an rvalue

**lib1**: Two ways to represent calls — `StatementKind::Call` (void return) and
`Rvalue::Call` (with return value). Both carry `callee` and `args`. Display code
is duplicated.

**lib**: `StatementKind::Call { dest: Option<Place>, callee, args }`. One
representation, one display path.

**Why**: Calls have side effects and don't compose like pure rvalues. Having them as
rvalues means `%x = call foo(); %y = add %x, call bar()` is representable but
nonsensical. Making call a statement with optional dest eliminates the duplication
and clarifies semantics.

### No redundant null pointer representation

**lib1**: Two ways to express null pointers — `Rvalue::PtrNull { ty }` and
`Rvalue::Use(Immediate::null_ptr(ty))`. Consumers must check both.

**lib**: `Immediate::NullPtr(ty)` only. One representation.

### No redundant boolean operations

**lib1**: `BinOp::BoolAnd`/`BoolOr` and `UnOp::BoolNot` coexist with `Rvalue::I1And`/
`I1Or`/`I1Not`/`I1Eq`. Same operations, two representations.

**lib**: `Op::BoolAnd`/`BoolOr`/`BoolNot`/`BoolEq`. One representation.

### No Value::Unreachable

**lib1**: `Value::Unreachable` represents a diverging expression. Can appear anywhere
a value is expected, forcing every consumer to handle it.

**lib**: Divergence is represented only at the terminator level (`TerminatorKind::Unreachable`).
Values are always either places or immediates.

**Why**: Divergence is a control flow concept, not a value concept. A diverging expression
means the block terminates — it doesn't produce a "value" that can be used in an
assignment.

## Metadata Changes

### Place is a bare enum

**lib1**: `Place` carries `Metadata` and `inline_name` on every node. Every projection
(`.field()`, `.deref()`) creates a new `Place` with a fresh `Metadata`.

**lib**: `Place` is just the `PlaceKind` enum. No metadata, no names.

**Why**: Place metadata on intermediate projections is never useful. Spans belong on
the statement that uses the place, not on the place itself.

### Immediate is minimal

**lib1**: `Immediate` carries `Metadata` and `inline_name`.

**lib**: `Immediate` is just `ImmediateKind`. A literal `42` doesn't need a span or
debug comments.

### Statements carry optional spans, nothing else

**lib1**: `Statement` carries `Metadata` (span + origin + comments) and
`Vec<Prior<Statement>>` (transformation history).

**lib**: `Statement` carries `kind` and `Option<Span>`.

**Why**: The `Prior` system tracked transformation history across passes. In the new
architecture, passes produce new data structures rather than mutating in place, so
transformation history is implicit in the pipeline.

## Item Definition Changes

### FunctionKind replaces ad-hoc fields

**lib1**: `FunctionDef` has `receiver_convention: Option<ReceiverConvention>` and
`Origin` metadata to distinguish methods, closures, thunks, etc.

**lib**: `FunctionDef` has `kind: FunctionKind` — an enum that explicitly states what
kind of function this is and carries the relevant metadata (parent entity, env struct,
etc.).

**Why**: No inference needed anywhere. Codegen reads the kind. Monomorphization reads
the kind. The MIR is self-describing.

### MethodBinding carries source information

**lib1**: Monomorphization detects extension methods via string matching on function
names (checking if the name contains the protocol name).

**lib**: `MethodBinding` has a `source: MethodSource` field that explicitly records
whether this is a direct implementation or a protocol extension default.

**Why**: String matching on function names is fragile and wrong in edge cases.

### Explicit self-type on Callee

**lib1**: Codegen infers self-type through a 4-step fallback chain:
1. Try `subst.get_self_type()`
2. Infer from first argument type
3. Infer from method's containing type name
4. Fail

**lib**: `Callee::Direct` and `Callee::Witness` carry explicit `self_type` fields.

**Why**: Lowering always knows the self-type. Codegen shouldn't have to guess.

### ClosureInfo is a first-class item

**lib1**: Closure relationships (env struct, call function, captures) are encoded in
`Origin` metadata — debug info that's not part of the data model.

**lib**: `ClosureInfo` is a top-level item with explicit `env_struct`, `call_function`,
`captures` (with capture modes). Visible in the MIR dump.

**Why**: Capture modes matter for the deinit pass. Env struct relationships matter for
monomorphization. These are semantic, not debug info.

### Drop order precomputed in StructDef

**lib1**: Deinit emission walks semantic types at lowering time to discover field drop
order. MIR carries no drop information.

**lib**: `StructDef` has `drop_fields` (ordered list of fields that need dropping) and
`needs_drop`. Computed by the layout pass.

**Why**: Codegen and the deinit pass need this information. Computing it from semantic
types at lowering time couples the MIR to the semantic model and duplicates work.

### Layouts stored in MIR

**lib1**: Codegen maintains a separate `LayoutCache` that computes struct sizes and
field offsets.

**lib**: `StructDef` has `layout: Option<StructLayout>` with size, alignment, and
field offsets. Computed by a dedicated layout pass.

**Why**: Layout is structurally determined by the MIR. Computing it earlier lets MIR
passes reason about sizes. The MIR dump shows actual memory layout — zero abstraction.

### Module init is explicit

**lib1**: Static initialization finds `main` by suffix-matching the last segment of
qualified names. Injects a call into main's entry block.

**lib**: `MirModule` has `entry_point: Option<FunctionId>` and
`module_init: Option<FunctionId>`. Relationships are explicit in the data structure.

**Why**: String matching for main is fragile (multiple functions named main, different
qualified paths). Explicit fields are unambiguous.

### Static init order is explicit

**lib1**: Statics are initialized in symbol tree traversal order — implementation
detail, not specified.

**lib**: `StaticDef` has `init_order: u32` — topologically sorted based on
dependencies between static initializers.

**Why**: If static A's initializer references static B, B must be initialized first.
Implicit ordering from tree traversal is fragile and non-obvious.

## Determinism

### IndexMap/BTreeMap for all output-visible maps

**lib1**: `HashMap` throughout. Witness type bindings, method bindings, field lookups
all use `HashMap`. MIR dump output is non-deterministic.

**lib**: `IndexMap` for all maps that appear in output.

**Why**: Deterministic output enables snapshot testing, diffing MIR dumps between
compiler versions, and reproducible builds.

## Pass System Changes

### Passes are pipeline stages, not a PassManager

**lib1**: `PassManager` holds `Vec<Box<dyn MirPass>>`, runs them in sequence, supports
fixed-point iteration. Passes mutate a shared `MirContext`.

**lib**: Each pass is a function that takes a `MirModule` and returns a `MirModule`
(or modifies specific fields). The pipeline is explicit in code, not configured at
runtime.

**Why**: The PassManager pattern adds abstraction without benefit when the pass order
is fixed and known at compile time. Explicit pipeline stages are easier to understand,
debug, and type-check. Each intermediate is a distinct, dumpable state.

### Deinit is a pass, not interleaved with lowering

**lib1**: Deinit tracking is deeply interleaved with expression lowering — scope stacks,
flag proliferation, 8+ save/restore operations per closure, branch merge logic with
15+ case branches.

**lib**: Lowering emits naive MIR with no deinit logic. A separate deinit pass analyzes
liveness and inserts deinit statements.

**Why**: Lowering becomes a straightforward syntax-directed translation. The deinit pass
is a well-defined graph algorithm operating on the CFG. Both are simpler and
independently testable.

### Thunks generated in a dedicated pass

**lib1**: Thunks generated on-demand during lowering with a per-function cache. Witness
thunks not cached. Cross-function deduplication impossible.

**lib**: Thunk pass scans the complete MIR for `ApplyPartial` references, generates
all thunks, deduplicates globally.

**Why**: Global deduplication reduces code size. No cache management during lowering.
All thunks visible in the MIR dump.
