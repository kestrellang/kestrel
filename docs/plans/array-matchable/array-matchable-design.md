# ArrayMatchable Protocol Design

## Overview

Add an `ArrayMatchable` protocol that enables array pattern matching (`[a, b, ..rest, y, z]`) on any conforming type. This generalizes array patterns beyond the built-in `Array[T]` to work with any sequential collection that can provide length, indexed access, and slicing.

### Motivation

Currently, array patterns work on `Array[T]` via direct field access, but:
1. Suffix patterns (`[a, .., z]`) are not yet supported
2. Other collection types cannot participate in array pattern matching
3. There's no protocol-based abstraction like `RangeMatchable` for ranges

The `ArrayMatchable` protocol provides a principled way to:
1. Enable full array patterns including prefix, suffix, and rest capture
2. Allow `Slice[T]` to participate in array matching (recursive destructuring)
3. Provide a consistent protocol-based approach matching `RangeMatchable`

## Syntax

### Protocol Definition

```kestrel
@builtin(.ArrayMatchable)
public protocol ArrayMatchable {
    type Element

    /// Returns the number of elements
    @builtin(.ArrayMatchableMatchLength)
    func matchLength(self) -> Int64

    /// Returns the element at the given index
    /// Caller guarantees index is valid (0 <= index < matchLength())
    @builtin(.ArrayMatchableMatchGet)
    func matchGet(index: Int64) -> Element

    /// Returns a slice from `from` (inclusive) to `to` (exclusive)
    /// Caller guarantees valid bounds (0 <= from <= to <= matchLength())
    @builtin(.ArrayMatchableMatchSlice)
    func matchSlice(from: Int64, to: Int64) -> Slice[Element]
}
```

### Conforming Types

```kestrel
// Array conformance
extend Array[T]: ArrayMatchable {
    type Element = T

    public func matchLength() -> Int64 {
        self.count()
    }

    public func matchGet(index: Int64) -> T {
        self.getUnchecked(index)
    }

    public func matchSlice(from: Int64, to: Int64) -> Slice[T] {
        Slice(pointer: self.pointer().offset(by: from), count: to - from)
    }
}

// Slice conformance (enables recursive destructuring)
extend Slice[T]: ArrayMatchable {
    type Element = T

    public func matchLength() -> Int64 {
        self.count()
    }

    public func matchGet(index: Int64) -> T {
        self.pointer.offset(by: index).read()
    }

    public func matchSlice(from: Int64, to: Int64) -> Slice[T] {
        Slice(pointer: self.pointer.offset(by: from), count: to - from)
    }
}
```

### Pattern Syntax

| Pattern | Meaning | Min Length | Protocol Calls |
|---------|---------|------------|----------------|
| `[]` | Empty array | 0 (exact) | `matchLength() == 0` |
| `[a, b, c]` | Exactly 3 elements | 3 (exact) | `matchLength() == 3`, `matchGet(0,1,2)` |
| `[a, ..]` | At least 1 element | 1 | `matchLength() >= 1`, `matchGet(0)` |
| `[a, ..rest]` | At least 1, bind rest | 1 | `matchLength() >= 1`, `matchGet(0)`, `matchSlice(1, len)` |
| `[.., z]` | At least 1, get last | 1 | `matchLength() >= 1`, `matchGet(len-1)` |
| `[a, .., z]` | At least 2 elements | 2 | `matchLength() >= 2`, `matchGet(0)`, `matchGet(len-1)` |
| `[a, ..rest, z]` | At least 2, bind middle | 2 | `matchLength() >= 2`, `matchGet(0)`, `matchGet(len-1)`, `matchSlice(1, len-1)` |
| `[..rest]` | Bind whole as slice | 0 | `matchSlice(0, len)` |
| `[..]` | Matches any array | 0 | (none, just type check) |

### Usage Examples

```kestrel
// Basic prefix/suffix destructuring
func endpoints(arr: [Int64]) -> Optional[(Int64, Int64)] {
    match arr {
        [first, .., last] => .Some((first, last)),
        [only] => .Some((only, only)),
        [] => .None
    }
}

// Rest capture
func tail(arr: [Int64]) -> Slice[Int64] {
    match arr {
        [_, ..rest] => rest,
        [] => Slice(pointer: Pointer(raw: lang.ptr_null[Int64]()), count: 0)
    }
}

// Recursive destructuring on captured slice
func sumFirstTwo(arr: [Int64]) -> Int64 {
    match arr {
        [a, b, ..rest] => {
            match rest {
                [c, d, ..] => a + b + c + d,
                _ => a + b
            }
        },
        [a] => a,
        [] => 0
    }
}

// Pattern matching with nested patterns
func findPair(arr: [Optional[Int64]]) -> Optional[(Int64, Int64)] {
    match arr {
        [.Some(a), .Some(b), ..] => .Some((a, b)),
        [_, ..rest] => findPair(rest.toArray()),  // would need Slice.toArray()
        [] => .None
    }
}
```

## Semantic Behavior

### Type Resolution

When the compiler encounters an array pattern `[a, ..rest, z]`:

1. Determine the scrutinee type `T`
2. Check if `T: ArrayMatchable`
3. If not, report error: "type `T` does not conform to `ArrayMatchable`"
4. Resolve element patterns against `T.Element`
5. If rest is bound, its type is `Slice[T.Element]`

### Pattern Compilation

Array patterns compile to `Constructor::Array` in the decision tree with:
- `prefix_len`: Number of fixed elements at start
- `suffix_len`: Number of fixed elements at end
- `has_rest`: Whether there's a `..` or `..name` pattern

### MIR Lowering

For types conforming to `ArrayMatchable`, emit witness method calls:

```
// For pattern: [a, ..rest, z] matching value with length >= 2
%len = call ArrayMatchable.matchLength(%scrutinee)
%min_len = const 2
%len_ok = binop ge %len, %min_len
branch %len_ok, check_elements, no_match

check_elements:
// Get prefix element
%a = call ArrayMatchable.matchGet(%scrutinee, 0)

// Get suffix element (calculate index from end)
%last_idx = binop sub %len, 1
%z = call ArrayMatchable.matchGet(%scrutinee, %last_idx)

// Get rest slice
%rest_start = const 1
%rest_end = binop sub %len, 1
%rest = call ArrayMatchable.matchSlice(%scrutinee, %rest_start, %rest_end)

// Continue to match arm...
```

### Slice Lifetime

The `Slice` returned by `matchSlice` borrows from the original collection. This slice is only valid within the match arm scope. The compiler does not currently enforce this at the type level, but it is a semantic requirement:

```kestrel
// UNSAFE: Slice escapes match arm
func bad(arr: [Int64]) -> Slice[Int64] {
    match arr {
        [_, ..rest] => rest  // rest borrows from arr, but arr may be dropped
    }
}

// SAFE: Use slice within match arm
func good(arr: [Int64]) -> Int64 {
    match arr {
        [_, ..rest] => rest.count(),  // OK: use slice, return value
        [] => 0
    }
}
```

Future work may add lifetime tracking to enforce this at compile time.

### Builtin Annotation

The `@builtin(.ArrayMatchable)` annotation registers the protocol with the compiler's builtin registry, enabling:
- Detection of ArrayMatchable conformance during pattern resolution
- Method lookup for witness calls during MIR lowering

## Error Cases

| Condition | Error Message |
|-----------|---------------|
| Array pattern on non-ArrayMatchable type | "type `T` does not conform to `ArrayMatchable`" |
| Multiple rest patterns | "array pattern cannot have multiple rest patterns" |
| Rest pattern not in correct position | "rest pattern `..` must appear at most once in array pattern" |

## Edge Cases

### Empty Arrays
- `[]` matches only empty arrays (length == 0)
- `[..]` matches any array including empty
- `[..rest]` on empty array binds `rest` to empty slice

### Single Element
- `[x]` matches exactly one element
- `[x, ..]` matches one or more (rest may be empty)
- `[.., x]` matches one or more (prefix may be empty)

### Rest Without Binding
- `[a, .., z]` checks length >= 2 but doesn't allocate/bind the middle
- Compiler optimizes by not calling `matchSlice`

### Overlapping Patterns
```kestrel
match arr {
    [] => "empty",
    [x] => "one",        // Shadows [x, ..] for length 1
    [x, ..rest] => "many"  // rest could be empty for length 1, but [x] wins
}
```
Pattern order matters. More specific patterns should come first.

### Nested Patterns
Element patterns can themselves be complex:
```kestrel
match arr {
    [.Some(x), .None, ..] => x,
    _ => 0
}
```

## Open Questions (Resolved)

**Q: Should the rest type be configurable via associated type?**
A: No. Always use `Slice[Element]` for simplicity and consistency. If users need an owned copy, they can call a conversion method.

**Q: Should String conform to ArrayMatchable?**
A: No. String has complex UTF-8 encoding where "characters" don't map directly to byte indices. Users can explicitly convert to `[Char]` if needed.

**Q: Should we have a `matchGetFromEnd(index)` method?**
A: No. Calculate the index as `matchLength() - 1 - index`. Simpler protocol, same capability.

**Q: What about exhaustiveness checking?**
A: The existing `Constructor::Array` representation with `prefix_len`, `suffix_len`, and `has_rest` already supports exhaustiveness checking. Variable-length arrays (with rest) are non-exhaustive without a catch-all.

## Implementation Notes

### Stdlib Changes
- Add `ArrayMatchable` protocol to `std.core.protocols`
- Add `Array[T]: ArrayMatchable` extension to `std.collections.array`
- Add `Slice[T]: ArrayMatchable` extension to `std.memory.pointer` (or separate file)

### Compiler Changes
- Register `@builtin(.ArrayMatchable)` and method builtins in builtin registry
- Update pattern resolution to check `ArrayMatchable` conformance
- Remove `ArraySuffixPatternError` - suffix patterns now supported
- Update MIR lowering to emit witness calls for ArrayMatchable methods
- Bind rest patterns as `Slice[Element]` type

### Semantic Model Changes
- `PatternKind::Array` already has `prefix`, `rest`, `suffix` fields
- Rest binding type changes from `Array[T]` to `Slice[T]`
