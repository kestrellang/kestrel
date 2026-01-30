# ArrayMatchable Implementation Plan

## Overview

This plan implements the `ArrayMatchable` protocol to enable full array pattern matching including suffix patterns (`[a, .., z]`) and rest capture (`[a, ..rest, z]`).

## Test Strategy

### Test Categories
1. **Basic patterns**: `[]`, `[a]`, `[a, b, c]`
2. **Rest patterns**: `[a, ..]`, `[a, ..rest]`, `[..rest]`, `[..]`
3. **Suffix patterns**: `[.., z]`, `[a, .., z]`, `[..rest, y, z]`
4. **Combined patterns**: `[a, ..rest, z]`, `[a, b, ..rest, y, z]`
5. **Nested patterns**: `[.Some(x), .None, ..]`
6. **Slice conformance**: Matching on `Slice[T]` values
7. **Error cases**: Non-ArrayMatchable types

### Key Behaviors to Verify
- Length checking works correctly for all pattern variants
- Rest binding has type `Slice[Element]`
- Empty rest produces empty slice
- Prefix and suffix elements extracted correctly
- Nested destructuring of captured rest works

## Implementation Phases

### Phase 0: Tests (First!)

**Files**: `lib/kestrel-test-suite/tests/patterns/array_matchable.rs`

- [ ] Create test file with module structure
- [ ] Basic array patterns (prefix only)
- [ ] Rest patterns without binding
- [ ] Rest patterns with binding (as Slice)
- [ ] Suffix patterns (requires ArrayMatchable)
- [ ] Combined prefix + rest + suffix
- [ ] Nested patterns in array elements
- [ ] Slice conformance tests
- [ ] Error case: pattern on non-ArrayMatchable type

### Phase 1: Builtin Registry

**Files**: `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `LanguageFeature::ArrayMatchable` enum variant
- [ ] Add `LanguageFeature::ArrayMatchableMatchLength` enum variant
- [ ] Add `LanguageFeature::ArrayMatchableMatchGet` enum variant
- [ ] Add `LanguageFeature::ArrayMatchableMatchSlice` enum variant
- [ ] Add `from_name()` mappings for all four features
- [ ] Add `name()` mappings for all four features
- [ ] Add `definition()` for `ArrayMatchable` (Protocol kind)
- [ ] Add `definition()` for method features (ProtocolMethod kind)
- [ ] Add convenience method `array_matchable_protocol()` to `BuiltinRegistry`

### Phase 2: Standard Library Protocol

**Files**: `lang/std/core/protocols.ks`

- [ ] Add import for `Slice` type
- [ ] Define `ArrayMatchable` protocol with `@builtin(.ArrayMatchable)`
- [ ] Add `type Element` associated type
- [ ] Add `matchLength(self) -> Int64` with `@builtin(.ArrayMatchableMatchLength)`
- [ ] Add `matchGet(index: Int64) -> Element` with `@builtin(.ArrayMatchableMatchGet)`
- [ ] Add `matchSlice(from: Int64, to: Int64) -> Slice[Element]` with `@builtin(.ArrayMatchableMatchSlice)`

### Phase 3: Array Conformance

**Files**: `lang/std/collections/array.ks`

- [ ] Add `extend Array[T]: ArrayMatchable` block
- [ ] Define `type Element = T`
- [ ] Implement `matchLength()` -> `self.count()`
- [ ] Implement `matchGet(index)` -> `self.getUnchecked(index)`
- [ ] Implement `matchSlice(from, to)` -> create Slice from pointer arithmetic

### Phase 4: Slice Conformance

**Files**: `lang/std/memory/pointer.ks`

- [ ] Add `extend Slice[T]: ArrayMatchable` block
- [ ] Define `type Element = T`
- [ ] Implement `matchLength()` -> `self.count`
- [ ] Implement `matchGet(index)` -> `self.ptr.offset(by: index).read()`
- [ ] Implement `matchSlice(from, to)` -> create new Slice with offset pointer

### Phase 5: Pattern Resolution Updates

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/patterns.rs`

- [ ] Remove `ArraySuffixPatternError` check (lines ~1138-1143)
- [ ] Update rest binding type from `Array[T]` to `Slice[T]` (line ~1120)
- [ ] Add helper to resolve `Slice[T]` type from element type
- [ ] Verify ArrayMatchable conformance for non-builtin array types (optional - can rely on MIR lowering errors)

**Files**: `lib/kestrel-semantic-tree-binder/src/diagnostics.rs`

- [ ] Remove or repurpose `ArraySuffixPatternError` diagnostic

### Phase 6: MIR Lowering - Irrefutable Patterns

**Files**: `lib/kestrel-execution-graph-lowering/src/pattern.rs`

- [ ] Update `lower_array_pattern` to handle suffix patterns
- [ ] For suffix elements, emit witness calls to `matchLength` and `matchGet`
- [ ] For rest binding, emit witness call to `matchSlice`
- [ ] Add helper functions:
  - `emit_array_matchable_length(ctx, array_place, ty) -> Place`
  - `emit_array_matchable_get(ctx, array_place, index, ty) -> Place`
  - `emit_array_matchable_slice(ctx, array_place, from, to, ty) -> Place`

### Phase 7: MIR Lowering - Refutable Patterns (Match Expressions)

**Files**: `lib/kestrel-execution-graph-lowering/src/match_lowering.rs`

- [ ] Add handling for `Constructor::Array` in `emit_switch_decision`
- [ ] Emit length check: `matchLength() >= min_length`
- [ ] Branch based on length check result
- [ ] For matching arm, emit element extraction via witness calls
- [ ] Structure similar to `CharRange` handling with witness calls

### Phase 8: Update make_slice_type Helper

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs` (or appropriate location)

- [ ] Add `make_slice_type(element_ty) -> Ty` helper if not existing
- [ ] Use this for rest binding type resolution

## File Change Summary

| File | Changes |
|------|---------|
| `lib/kestrel-semantic-tree/src/builtins.rs` | Add 4 language features, definitions, convenience method |
| `lang/std/core/protocols.ks` | Add ArrayMatchable protocol |
| `lang/std/collections/array.ks` | Add Array conformance to ArrayMatchable |
| `lang/std/memory/pointer.ks` | Add Slice conformance to ArrayMatchable |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/patterns.rs` | Remove suffix error, update rest type |
| `lib/kestrel-semantic-tree-binder/src/diagnostics.rs` | Remove ArraySuffixPatternError |
| `lib/kestrel-execution-graph-lowering/src/pattern.rs` | Update irrefutable array pattern lowering |
| `lib/kestrel-execution-graph-lowering/src/match_lowering.rs` | Add Constructor::Array handling |
| `lib/kestrel-test-suite/tests/patterns/array_matchable.rs` | New test file |
| `lib/kestrel-test-suite/tests/patterns/mod.rs` | Add `mod array_matchable;` |

## Verification

After each phase:
```bash
cargo test
cargo clippy
cargo fmt
```

Final verification:
- [ ] All new tests pass
- [ ] Existing pattern tests still pass
- [ ] No clippy warnings
- [ ] Code formatted

## Dependencies

```
Phase 0 (Tests)
    ↓
Phase 1 (Builtins) → Phase 2 (Protocol) → Phase 3 (Array) → Phase 4 (Slice)
    ↓
Phase 5 (Resolution) - can start after Phase 1
    ↓
Phase 6 (Irrefutable) - needs Phase 2-4 complete
    ↓
Phase 7 (Refutable) - needs Phase 6 complete
```

## Notes

### Current State
- Array patterns with prefix only work via direct MIR indexing
- Suffix patterns blocked with `ArraySuffixPatternError`
- Rest bindings typed as `Array[T]` (will change to `Slice[T]`)
- `Constructor::Array` falls through to default in match lowering

### Key Insight
- Irrefutable patterns (let bindings) can use direct indexing for prefix
- Refutable patterns (match) need full length checking via protocol
- Both need protocol calls for suffix and rest

### Protocol Method Signatures
```kestrel
func matchLength(self) -> Int64
func matchGet(index: Int64) -> Element  // caller validates index
func matchSlice(from: Int64, to: Int64) -> Slice[Element]  // caller validates bounds
```

### Witness Call Pattern (from RangeMatchable)
```rust
let callee = Callee::witness(
    protocol_name,      // "std.core.ArrayMatchable"
    "matchLength",      // method name
    for_type,           // Array[T] or Slice[T]
    vec![],             // type args (empty for ArrayMatchable)
);
let call_args = vec![
    CallArg::borrow(Value::Place(array_place)),
];
ctx.emit_call_with_modes(result_place, callee, call_args);
```
