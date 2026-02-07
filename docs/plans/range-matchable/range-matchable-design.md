# RangeMatchable Protocol Design

## Overview

Add a `RangeMatchable` protocol that enables range pattern matching (`start..=end`, `start..<end`, `..=end`, `..<end`, `start..`) on any conforming type. This generalizes range patterns beyond primitive integers to work with any ordered type, and supports heterogeneous matching where the value and bounds can be different types.

### Motivation

Currently, range patterns work on primitive `lang.i64` via direct MIR comparisons, but fail on struct types like `Char`. The `RangeMatchable` protocol provides a principled way to:

1. Enable range patterns on stdlib types (`Char`, `Int64`, etc.)
2. Allow user-defined types to participate in range matching
3. Support heterogeneous matching (e.g., match `Int64` against `Char` bounds)
4. Provide a default implementation for all `Comparable` types

## Syntax

### Protocol Definition

```kestrel
@builtin(.RangeMatchable)
public protocol RangeMatchable[Bound = Self] {
    /// Returns true if self >= bound (for start of range patterns)
    func isAtLeast(bound: Bound) -> Bool

    /// Returns true if self <= bound (for inclusive end: ..=end)
    func isAtMost(bound: Bound) -> Bool

    /// Returns true if self < bound (for exclusive end: ..<end)
    func isBelow(bound: Bound) -> Bool
}
```

### Default Implementation

```kestrel
/// All Comparable types automatically support range patterns against Self
extend Comparable: RangeMatchable[Self] {
    func isAtLeast(bound: Self) -> Bool { self >= bound }
    func isAtMost(bound: Self) -> Bool { self <= bound }
    func isBelow(bound: Self) -> Bool { self < bound }
}
```

### Range Pattern Syntax

| Pattern | Meaning | Protocol Methods Used |
|---------|---------|----------------------|
| `start..=end` | Inclusive range | `isAtLeast(start) && isAtMost(end)` |
| `start..<end` | Exclusive end | `isAtLeast(start) && isBelow(end)` |
| `..=end` | Up to inclusive | `isAtMost(end)` |
| `..<end` | Up to exclusive | `isBelow(end)` |
| `start..` | From start | `isAtLeast(start)` |

### Usage Examples

```kestrel
// Basic char range (Char conforms to Comparable, gets RangeMatchable[Char])
func classify(c: Char) -> String {
    match c {
        'a'..='z' => "lowercase",
        'A'..='Z' => "uppercase",
        '0'..='9' => "digit",
        _ => "other"
    }
}

// Open-ended ranges
func ageCategory(age: Int64) -> String {
    match age {
        ..<0 => "invalid",
        0..=12 => "child",
        13..=19 => "teenager",
        20..=64 => "adult",
        65.. => "senior"
    }
}

// Custom type with Comparable conformance (gets RangeMatchable automatically)
struct Version: Comparable {
    var major: Int64
    var minor: Int64
    // ... compare implementation
}

match appVersion {
    Version(1,0)..=Version(1,9) => "legacy",
    Version(2,0)..<Version(3,0) => "current",
    Version(3,0).. => "preview"
}

// Heterogeneous matching: Int64 against Char bounds
extend Int64: RangeMatchable[Char] {
    func isAtLeast(bound: Char) -> Bool {
        self >= Int64(intLiteral: bound.value().raw)
    }
    func isAtMost(bound: Char) -> Bool {
        self <= Int64(intLiteral: bound.value().raw)
    }
    func isBelow(bound: Char) -> Bool {
        self < Int64(intLiteral: bound.value().raw)
    }
}

func isAsciiLetter(codePoint: Int64) -> Bool {
    match codePoint {
        'a'..='z' => true,
        'A'..='Z' => true,
        _ => false
    }
}
```

## Semantic Behavior

### Type Resolution

When the compiler encounters a range pattern `start..=end`:

1. Determine the scrutinee type `T` (the value being matched)
2. Determine the bound type `B` from the pattern literals
3. Check if `T: RangeMatchable[B]`
4. If not, report error: "type `T` does not support range matching against `B`"

### Pattern Compilation

Range patterns compile to `Constructor::Range` in the decision tree with:
- `bound_type`: The type of the bounds
- `start`: Optional start bound
- `end`: Optional end bound
- `inclusive`: Whether end is inclusive

### MIR Lowering

For struct types conforming to `RangeMatchable`, emit witness method calls:

```
// For pattern: start..=end
%in_range = call RangeMatchable[B].isAtLeast(%scrutinee, %start)
branch %in_range, check_end, no_match
check_end:
%at_most = call RangeMatchable[B].isAtMost(%scrutinee, %end)
branch %at_most, match_arm, no_match
```

For primitive types (`lang.i64`, `lang.i32`, etc.), continue using direct `BinOp` comparisons for efficiency.

### Builtin Annotation

The `@builtin(.RangeMatchable)` annotation registers the protocol with the compiler's builtin registry, enabling:
- Detection of RangeMatchable conformance during pattern resolution
- Method lookup for witness calls during MIR lowering

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Range pattern on non-RangeMatchable type | "type `T` does not conform to `RangeMatchable[B]`" |
| Invalid range bounds (start > end) | "range pattern has start bound greater than end bound" |
| Mismatched bound types in pattern | "range pattern bounds must have the same type" |
| Open start with open end (`....`) | "range pattern must have at least one bound" |

## Edge Cases

### Empty Ranges
- `10..=5` (start > end inclusive): Never matches, warning emitted
- `10..<10` (start == end exclusive): Never matches, warning emitted
- `10..=10` (start == end inclusive): Matches exactly 10

### Type Inference
- Bound literals infer type from scrutinee when possible
- `'a'..='z'` against `Int64` scrutinee: bounds are `Char`, needs `Int64: RangeMatchable[Char]`

### Primitive Optimization
- For `lang.i64`, `lang.i32`, etc., bypass witness calls and use direct BinOp
- This maintains performance parity with current implementation

### Exhaustiveness
- Open-ended ranges (`start..`, `..=end`) affect exhaustiveness checking
- `0.. ` on `Int64` is not exhaustive (negative numbers)
- Pattern matching engine already handles `IntRange`/`CharRange` constructors

## Open Questions (Resolved)

**Q: Should RangeMatchable require Comparable?**
A: No. The protocol is independent, but we provide a default implementation via `extend Comparable: RangeMatchable[Self]`. This allows types to implement RangeMatchable without Comparable if needed.

**Q: Should we support ranges with different start/end types?**
A: No. Both bounds must have the same type `B`. The heterogeneity is between scrutinee type `T` and bound type `B`, not between start and end.

**Q: What about floating-point ranges?**
A: Works naturally if `Float64: Comparable`. Edge cases with NaN are handled by the Comparable implementation.

**Q: How do open-ended patterns parse?**
A: New syntax nodes needed:
- `..=end` - RangeToInclusive
- `..<end` - RangeToExclusive
- `start..` - RangeFrom

## Implementation Notes

### Stdlib Changes
- Add `RangeMatchable` protocol to `std.core.protocols`
- Add `extend Comparable: RangeMatchable[Self]` extension
- All existing `Comparable` types automatically work

### Compiler Changes
- Register `@builtin(.RangeMatchable)` in builtin registry
- Update pattern resolution to check `RangeMatchable` conformance
- Update MIR lowering to emit witness calls for struct types
- Add parser support for open-ended range patterns (if not present)

### Semantic Model Changes
- `PatternKind::Range` needs `Option<RangeBound>` for start/end to support open-ended
- Add bound type to range pattern for heterogeneous matching
