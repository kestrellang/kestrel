# Collections Principles

## Value Semantics with COW

Collections use copy-on-write (COW) semantics. Copies are O(1) until mutation:

```kestrel
let a = [1, 2, 3]
var b = a.clone()  // O(1), shares storage
b.append(element: 4)  // Now copies, O(n)
```

## Dual Iteration Model

Two ways to process collections:

| Approach | Style | Use When |
|----------|-------|----------|
| `.iter().map(...).collect()` | Lazy | Building complex pipelines, memory-sensitive |
| `.map(...)` | Eager | Simple transforms, want result immediately |

```kestrel
// Lazy - nothing happens until collect()
let lazy = numbers.iter().map(|x| x * 2).filter(|x| x > 10).collect()

// Eager - executes immediately, returns Array
let eager = numbers.map(|x| x * 2).filter(|x| x > 10)
```

## Subscript Modes

Five access patterns for different needs:

| Mode | Behavior | Use When |
|------|----------|----------|
| `arr(i)` | Panics if out of bounds | Index is known valid |
| `arr(checked: i)` | Returns `Optional` | Index might be invalid |
| `arr(unchecked: i)` | No bounds check | Performance-critical, already validated |
| `arr(wrapping: i)` | Wraps around (`-1` = last) | Circular access patterns |
| `arr(clamping: i)` | Clamps to valid range | Edge values acceptable |

## Mutating vs Non-Mutating

Pairs of methods for in-place vs copying operations:

| Mutating | Non-Mutating | Notes |
|----------|--------------|-------|
| `sort()` | `sorted()` | In-place vs new array |
| `reverse()` | `reversed()` | |
| `shuffle()` | `shuffled()` | |
| `dedup()` | `deduped()` | |

## Protocol Hierarchy

```
Iterable[T]           -- Can produce an iterator via .iter()
    │
    └── DirectIterable[T]  -- Also has eager methods (map, filter, etc.)

Iterator[T]           -- Lazy, single-pass traversal
    │
    ├── DoubleEndedIterator  -- Can traverse from both ends
    └── ExactSizeIterator    -- Knows remaining count
```

## Conditional Conformance

Methods appear based on element capabilities:

```kestrel
// Available on all arrays
arr.map(|x| ...)
arr.filter(|x| ...)

// Only when T: Equatable
arr.contains(element: x)
arr.dedup()

// Only when T: Comparable
arr.sort()
arr.min()

// Only when T: Hashable
arr.unique()

// Only when T: Addable
arr.sum()
```
