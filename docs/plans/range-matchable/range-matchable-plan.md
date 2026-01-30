# RangeMatchable Implementation Plan

## Test Strategy

- Basic range patterns on Char type (currently failing)
- Open-ended range patterns (`..=end`, `..<end`, `start..`)
- Heterogeneous range matching (Int64 against Char bounds)
- Custom types conforming to RangeMatchable
- Error cases (non-conforming types, invalid bounds)
- Exhaustiveness checking with open-ended ranges

## Implementation Phases

### Phase 0: Tests (First!)

Files: `lib/kestrel-test-suite/tests/patterns/range_matchable.rs`

- [ ] Basic char range pattern with stdlib Char
- [ ] Open-ended pattern: `..=end` (up to inclusive)
- [ ] Open-ended pattern: `..<end` (up to exclusive)
- [ ] Open-ended pattern: `start..` (from start)
- [ ] Custom type with RangeMatchable conformance
- [ ] Heterogeneous matching (Int64 against Char bounds)
- [ ] Error: range pattern on non-RangeMatchable type
- [ ] Verify existing integer range tests still pass

### Phase 1: Stdlib - Add RangeMatchable Protocol

Files: `lang/std/core/protocols.ks`

- [ ] Add `RangeMatchable[Bound = Self]` protocol with `@builtin(.RangeMatchable)` annotation
- [ ] Add three methods: `isAtLeast`, `isAtMost`, `isBelow`
- [ ] Add `extend Comparable: RangeMatchable[Self]` default implementation

```kestrel
@builtin(.RangeMatchable)
public protocol RangeMatchable[Bound = Self] {
    func isAtLeast(bound: Bound) -> Bool
    func isAtMost(bound: Bound) -> Bool
    func isBelow(bound: Bound) -> Bool
}

extend Comparable: RangeMatchable[Self] {
    func isAtLeast(bound: Self) -> Bool { self >= bound }
    func isAtMost(bound: Self) -> Bool { self <= bound }
    func isBelow(bound: Self) -> Bool { self < bound }
}
```

### Phase 2: Builtin Registry

Files: `lib/kestrel-semantic-tree/src/builtins.rs`

- [ ] Add `RangeMatchable` to `BuiltinFeature` enum
- [ ] Add `RangeMatchableIsAtLeast`, `RangeMatchableIsAtMost`, `RangeMatchableIsBelow` for method lookup
- [ ] Update `BuiltinRegistry` to track the protocol

### Phase 3: Parser - Open-Ended Range Patterns

Files: `lib/kestrel-parser/src/pattern/mod.rs`

- [ ] Add `RangeToInclusive` pattern: `..=end`
- [ ] Add `RangeToExclusive` pattern: `..<end`
- [ ] Add `RangeFrom` pattern: `start..`
- [ ] Update `range_pattern` parser to handle optional start/end

Current parser (line 236-256) requires both start and end:
```rust
let range_pattern = range_start
    .then(operator)
    .then(range_end)  // Required!
```

Need to also parse:
- `..= <end>` - starts with operator
- `..< <end>` - starts with operator
- `<start> ..` - ends with just `..`

### Phase 4: Syntax Tree - Optional Bounds

Files: `lib/kestrel-syntax-tree/src/lib.rs`

- [ ] Add `RangeToPattern` syntax kind (for `..=end`, `..<end`)
- [ ] Add `RangeFromPattern` syntax kind (for `start..`)
- [ ] Or update existing `RangePattern` to handle all cases

### Phase 5: Semantic Model - Optional Range Bounds

Files: `lib/kestrel-semantic-tree/src/pattern.rs`

- [ ] Change `PatternKind::Range` to have optional bounds:

```rust
Range {
    /// The start bound (None for ..=end, ..<end patterns)
    start: Option<RangeBound>,
    /// The end bound (None for start.. patterns)
    end: Option<RangeBound>,
    /// Whether the end is inclusive (..=) or exclusive (..<)
    inclusive: bool,
}
```

- [ ] Update `Pattern` constructors and accessors

### Phase 6: Pattern Resolution - RangeMatchable Check

Files: `lib/kestrel-semantic-tree-binder/src/body_resolver/patterns.rs`

- [ ] Update `resolve_range_pattern` to handle optional bounds
- [ ] Add conformance check: scrutinee type must conform to `RangeMatchable[BoundType]`
- [ ] Resolve bound type from pattern literals
- [ ] Report error if type doesn't conform to RangeMatchable

Current location: `resolve_range_pattern` around line 779-901

### Phase 7: Pattern Matching Compilation

Files: `lib/kestrel-semantic-pattern-matching/src/constructor.rs`

- [ ] Update `Constructor::IntRange` and `Constructor::CharRange` to have optional bounds:

```rust
IntRange {
    start: Option<i64>,
    end: Option<i64>,
    inclusive: bool,
}
CharRange {
    start: Option<char>,
    end: Option<char>,
    inclusive: bool,
}
```

- [ ] Or add new constructors for open-ended ranges
- [ ] Update exhaustiveness checking for open-ended ranges

### Phase 8: MIR Lowering - Witness Calls

Files: `lib/kestrel-execution-graph-lowering/src/match_lowering.rs`

- [ ] Update `emit_int_switch` to handle optional bounds (lines 624-677, 708-761)
- [ ] Add handling for struct types conforming to RangeMatchable:
  - Detect RangeMatchable conformance
  - Emit witness calls to `isAtLeast`, `isAtMost`, `isBelow`
  - Combine results with `BoolAnd`

```rust
// For start..=end on RangeMatchable type:
// %ge_start = call RangeMatchable.isAtLeast(%scrutinee, %start)
// %le_end = call RangeMatchable.isAtMost(%scrutinee, %end)
// %in_range = and %ge_start, %le_end
```

- [ ] Keep primitive optimization: `lang.i64` etc. still use direct BinOp

### Phase 9: Update All Range Pattern Consumers

Files that process `PatternKind::Range`:

- [ ] `lib/kestrel-execution-graph-lowering/src/pattern.rs` (lines 100-103, 344)
- [ ] `lib/kestrel-execution-graph-lowering/src/stmt.rs` (line 698)
- [ ] `lib/kestrel-execution-graph-lowering/src/expr.rs` (lines 5339, 6151)
- [ ] `lib/kestrel-execution-graph-lowering/src/lowerer/function.rs` (line 1491)

Update all match arms on `PatternKind::Range` to handle optional start/end.

## Verification

- [ ] All existing tests pass: `cargo test`
- [ ] New range_matchable tests pass
- [ ] Enable previously ignored char range tests
- [ ] Linting clean: `cargo clippy`
- [ ] Formatted: `cargo fmt`

## File Change Summary

| File | Changes |
|------|---------|
| `lang/std/core/protocols.ks` | Add RangeMatchable protocol + Comparable extension |
| `lib/kestrel-semantic-tree/src/builtins.rs` | Register RangeMatchable builtin |
| `lib/kestrel-parser/src/pattern/mod.rs` | Parse open-ended range patterns |
| `lib/kestrel-syntax-tree/src/lib.rs` | Add syntax kinds if needed |
| `lib/kestrel-semantic-tree/src/pattern.rs` | Make Range bounds optional |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/patterns.rs` | RangeMatchable conformance check |
| `lib/kestrel-semantic-pattern-matching/src/constructor.rs` | Update constructors |
| `lib/kestrel-execution-graph-lowering/src/match_lowering.rs` | Emit witness calls |
| `lib/kestrel-execution-graph-lowering/src/pattern.rs` | Handle optional bounds |
| `lib/kestrel-execution-graph-lowering/src/stmt.rs` | Handle optional bounds |
| `lib/kestrel-execution-graph-lowering/src/expr.rs` | Handle optional bounds |
| `lib/kestrel-execution-graph-lowering/src/lowerer/function.rs` | Handle optional bounds |
| `lib/kestrel-test-suite/tests/patterns/range_matchable.rs` | New tests |

## Dependencies

- Phase 1 (Stdlib) can be done independently
- Phase 2 (Builtin) depends on Phase 1
- Phases 3-5 (Parser/Syntax/Semantic) can be done together
- Phases 6-9 depend on Phases 3-5
- Tests should be written first but will fail until implementation complete
