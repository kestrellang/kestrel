# Generics and Copy Semantics

Kestrel generics follow a "copy-by-default" rule to maintain ergonomics for application developers.

## The Default: Copyable

A generic type parameter `T` is assumed to be `Copyable` by default:

```kestrel
func duplicate[T](item: T) -> (T, T) {
    return (item, item)  // OK! T is assumed Copyable
}

let p = Point(x: 1, y: 1)
duplicate(p)  // Works - Point is Copyable

let f = FileHandle(...)
duplicate(f)  // ERROR: FileHandle does not conform to Copyable
```

This means most generic code "just works" for application developers without needing to understand ownership.

## Relaxing the Constraint: `not Copyable`

Library authors and systems programmers can write code that works with **any** type (including non-copyable resources) by relaxing the constraint:

```kestrel
// "T: not Copyable" means "T has no Copyable requirement"
struct List[T: not Copyable] {
    var items: Array[T]
}

// Works with both Copyable and non-copyable types
let points: List[Point] = ...        // OK
let files: List[FileHandle] = ...    // OK
```

### Operations in `not Copyable` Contexts

When `T: not Copyable`, you **cannot** copy values of type `T`:

```kestrel
func push[T: not Copyable](mutating list: List[T], item: consuming T) {
    // We can MOVE 'item' into the list
    // We CANNOT copy 'item' because we don't know if it's Copyable
    list.items.append(item)
}

func bad[T: not Copyable](item: T) -> (T, T) {
    return (item, item)  // ERROR: cannot copy T
}
```

## Summary of Generic Bounds

| Syntax | Meaning | Can Copy? |
|--------|---------|-----------|
| `[T]` | T must be Copyable | Yes |
| `[T: Copyable]` | Explicit: T must be Copyable | Yes |
| `[T: not Copyable]` | T has no Copyable requirement | No |

## Combining with Other Bounds

Generic bounds can be combined:

```kestrel
// T must be Copyable (default) AND Equatable
func findAll[T: Equatable](items: Array[T], target: T) -> Array[T] { ... }

// T can be anything, but must be Printable
func printAll[T: not Copyable + Printable](items: Array[T]) { ... }
```

---

## Potential Issues

### 1. Inverted Mental Model

Most languages require you to add constraints. Kestrel requires you to *remove* them:

```kestrel
// Other languages: "I need T to be copyable, let me add that"
func duplicate<T: Clone>(item: T) -> (T, T)  // Rust

// Kestrel: "I need T to accept non-copyable, let me remove Copyable"
func wrap[T: not Copyable](item: consuming T) -> Box[T]
```

**Concern**: This inverted model may confuse developers coming from Rust, Swift, or other languages.

**Counterargument**: It matches the "application first" philosophy. Application code doesn't need to think about it; only library authors need to opt out.

### 2. Forgetting `not Copyable` in Library Code

Library authors might forget to add `not Copyable`, making their types unusable with resources:

```kestrel
// Oops! This List only works with Copyable types
struct List[T] {
    var items: Array[T]
}

let files: List[FileHandle] = ...  // ERROR: FileHandle is not Copyable
```

**Mitigation**: 
- Linter rule for collection types suggesting `not Copyable`?
- Good documentation and examples?

### 3. Conditional Copyability

What if a generic type should be Copyable only when its parameter is?

```kestrel
struct Box[T: not Copyable] {
    var value: T
}

// Should Box[Int] be Copyable? Box[FileHandle] not Copyable?
```

**Options**:
1. Box is never Copyable (simplest)
2. Conditional conformance: `Box[T]: Copyable where T: Copyable`
3. Compiler infers Copyable-ness based on fields (current approach for non-generic types)

**Recommendation**: Apply the same inference rule. `Box[T]` is Copyable iff `T` is Copyable. This requires the compiler to track conditional conformance.

### 4. Interaction with Variance

How does `not Copyable` interact with variance?

```kestrel
struct Container[T: not Copyable] { ... }

// Is Container[Cat] a subtype of Container[Animal]?
// Does the not Copyable bound affect this?
```

**Concern**: Variance rules become more complex with ownership.

### 5. Conditional Method Availability

Methods can have additional bounds using `where` clauses:

```kestrel
struct List[T: not Copyable] {
    // Always available
    func push(mutating self, item: consuming T) { ... }
    
    // Only available when T: Copyable
    func clone(self) -> List[T] where T: Copyable { ... }
}
```

This is fully supported and used extensively in the standard library.

### 6. Defaulting Creates Two "Languages"

The copy-by-default rule creates two modes of thinking:

1. **Application mode**: Everything copies, don't worry about ownership
2. **Systems mode**: Ownership matters, use `not Copyable` everywhere

**Concern**: This bifurcation might create confusion. Code written in "application mode" may not compose well with "systems mode" libraries.

**Counterargument**: This is intentional. The goal is to let application developers ignore ownership while giving systems developers full control.

### 7. Error Messages for Bound Violations

When a non-copyable type is passed to a generic function expecting Copyable:

```kestrel
func duplicate[T](item: T) -> (T, T) { ... }

duplicate(myFileHandle)  // ERROR
```

**Question**: How clear is the error message?

Good: "FileHandle does not conform to Copyable, required by duplicate[T]"

Bad: "Type mismatch in generic instantiation"

### 8. Standard Library Design

The standard library must decide which types use `not Copyable`:

```kestrel
// These should probably use not Copyable:
struct Array[T: not Copyable] { ... }
struct Optional[T: not Copyable] { ... }
struct Result[T: not Copyable, E: not Copyable] { ... }

// These might not need it:
struct Range[T: Comparable] { ... }  // T is usually Int, which is Copyable
```

**Recommendation**: Err on the side of `not Copyable` for fundamental types to maximize flexibility.
