# lib1 Execution Graph Review

Code quality review of `lib/kestrel-execution-graph` that motivated the lib redesign.

## High Priority Issues

### BinOp/UnOp hardcoded to "i64"/"f64"

All `BinOp` display strings are hardcoded: `BinOp::AddSigned => "i64.add.signed"`.
But the IR supports i8/i16/i32 types and f16/f32. A `BinOp` on two `i32` values prints
`i64.add.signed` — incorrect and misleading. Same issue with `UnOp::as_str()` and
`CastKind::as_str()`.

**Fix in lib**: `Op` carries `IntBits`/`FloatBits` directly.

### I1* vs Bool* redundancy

`Rvalue` has both `BinaryOp { op: BinOp::BoolAnd, .. }` and `I1And { lhs, rhs }`.
Same operations, two representations. Forces consumers to handle both.

**Fix in lib**: Single `Op::BoolAnd`/`BoolOr`/`BoolNot`/`BoolEq`.

### Rvalue::PtrNull duplicates ImmediateKind::NullPtr

Two ways to express null pointers. Consumers must check both.

**Fix in lib**: `Immediate::NullPtr(ty)` only.

## Medium Priority Issues

### FloatBits missing as_str()

6 identical `match bits { F16 => "f16", F32 => "f32", F64 => "f64" }` blocks in
`statement.rs`. Every other "bits" type has an `as_str()` method.

### Type params display duplication

Displaying `[T, U]` from `type_params` is copy-pasted identically in
`FunctionDefDisplay`, `StructDefDisplay`, `EnumDefDisplay`, `ProtocolDefDisplay`.

### Comma-separated list display duplication

The exact same comma-separated formatting loop appears ~20 times:
```rust
for (i, item) in items.iter().enumerate() {
    if i > 0 { write!(f, ", ")?; }
    write!(f, "{}", ...)?;
}
```

### Rvalue is a 40-variant god enum

Mixes core IR concepts (Move, Copy, BinaryOp, Call) with domain-specific intrinsics
(pointer ops, string ops, float ops, atomic ops). Every match must handle all variants.

**Fix in lib**: Core ops + arity-based `Op1`/`Op2`/`Op3` with a single `Op` enum.

### StatementKind::Call duplicates Rvalue::Call

Both carry `callee: Callee, args: Vec<CallArg>`. Display code is identical.

**Fix in lib**: Single `StatementKind::Call { dest: Option<Place>, ... }`.

### run_to_fixpoint fragile modified handling

```rust
result.merge(PassResult {
    diagnostics: iteration_result.diagnostics,
    modified: false, // misleading comment
});
if !iteration_result.modified { break; }
result.modified = true;
```

Manually destructs `iteration_result` to avoid move-before-check. Should be:
```rust
let changed = iteration_result.modified;
result.merge(iteration_result);
if !changed { break; }
```

## Low Priority Issues

### root_local() panics on global places

`Place::root_local()` panics with `"root_local() called on global place"`. Should
return `Option<LocalId>`.

### block_index silent unwrap_or(0)

Terminator display has:
```rust
let block_index = |id| self.blocks.iter().position(|&b| b == id).unwrap_or(0);
```
O(n) per block reference. Silently returns 0 on failure, hiding bugs.

### Inconsistent accessor coverage on MirContext

`function()`/`function_mut()` exist for functions but not for fields, enum_cases,
params, witnesses, statics, etc. Half-and-half approach.

### Place/Immediate carry unused Metadata

Every `Place` projection (`.field()`, `.deref()`) creates a new `Place` with a fresh
`Metadata::new()`. Metadata on intermediate projections is never used. Same for
`Immediate` — a literal `42` carries a `Metadata` and `Option<String>` inline_name.

### FunctionBuilder::param_with_label clones name 4 times

The `name` string is cloned for `LocalDef`, `ParamDef`, `params_by_name`, and
`locals_by_name`. Correct but noisy.

### #[allow(dead_code)] on BlockBuilder::func_id

Stored but unused. Either use it or remove it.

### QualifiedNameData uses Vec<String>

Heavy for interning — every lookup requires allocating a `Vec` and multiple `String`s.

### ProtocolMethodDef uses raw (String, Id<Ty>) for params

Unlike `FunctionDef` which uses proper `ParamDef` / `Id<Param>`, protocol methods
store params as raw tuples. Protocol method params don't get metadata, labels, or
local bindings.

## Monomorphization / Codegen Pain Points

### Self-type inference fallback chain

Codegen has a 4-step chain to infer self-type:
1. Try `subst.get_self_type()`
2. Infer from first argument type
3. Infer from method's containing type name (string matching)
4. Fail

**Fix in lib**: `Callee` carries explicit `self_type`.

### Extension method detection via string matching

Monomorphization detects extension methods by checking if the implementation function
name contains the protocol name.

**Fix in lib**: `MethodBinding.source: MethodSource` enum.

### Witness lookup is linear scan

```rust
for witness in mir.witnesses:
    if witness.protocol == target_protocol:
        if match_pattern(witness.implementing_type, for_type):
            return witness
```

O(n) for every witness resolution.

**Fix in lib**: `witness_index: HashMap<(Entity, MirTy), WitnessId>` on MirModule.

### Function lookup by name is linear scan

Codegen finds functions by iterating all functions and matching names.

**Fix in lib**: Functions indexed by entity.

### Bool representation ambiguity in codegen

Branch terminator compilation checks at runtime whether the condition is a bool
primitive (i8) or a pointer to a wrapper struct, then loads accordingly.

### Mutable MirContext during monomorphization

Collection phase mutates `MirContext` to intern new types, then codegen phase reads
it immutably. The boundary between mutable and immutable phases is implicit.

## Lowering Pain Points

### Deinit tracking interleaved with lowering

Scope stacks, deinit flags, 15+ case branches for merge logic, 8+ save/restore
operations per closure. Single biggest source of complexity.

**Fix in lib**: Deinit is a separate pass on the CFG.

### Closure context save/restore

Lowering a closure requires saving and restoring 8+ fields of the lowering context.
Manual and error-prone.

**Fix in lib**: Closures lowered independently. No context save/restore.

### Type parameter scope is flat

One global type parameter map, cleared between functions. No support for nested
generic contexts.

### Static init finds main by string matching

Suffix-matches the last segment of qualified names. Fragile with multiple "main"
functions.

**Fix in lib**: Explicit `entry_point` and `module_init` on MirModule.

### LoweringContext mixes state and builder

24 fields covering current function, block, scope, locals, temps, counters, and the
MIR being built. High cohesion makes unit testing difficult.
