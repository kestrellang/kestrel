# Kestrel TODO

This file tracks immediate next steps for Phase 6.

## Current Priority: Phase 6 - Generics & Protocols

---

## Phase 6: Generics & Protocols

### GenericsBehavior Refactor
**Status**: ✅ DONE

Refactored generics storage to use the behavior pattern consistently, eliminating `RwLock<WhereClause>` mutation.

**What was done**:
- [x] Created `GenericsBehavior` holding type parameters and where clause
- [x] Added `Generics` to `KestrelBehaviorKind`
- [x] Added `generics_behavior()` accessor to `BehaviorExt`
- [x] Removed `type_parameters` and `where_clause` fields from `FunctionSymbol`, `StructSymbol`, `ProtocolSymbol`, `TypeAliasSymbol`
- [x] Updated resolvers to add `GenericsBehavior` during BIND with fully resolved where clause
- [x] Added fallback to children for `type_parameters()` during BUILD phase (before BIND)

### Generic Constraint Enforcement
**Status**: ✅ DONE

Use `where` clause constraints to enable method calls on type parameters.

**What was done**:
- [x] Add `get_where_clause(symbol)` helper function
- [x] Modify `get_type_container()` to handle `TypeParameter` by looking up protocol bounds
- [x] Collect methods from ALL protocol bounds (not just first)
- [x] Substitute `Self` with receiver type when looking up protocol methods
- [x] Handle ambiguous methods (same signature in multiple protocols) with diagnostic
- [x] Search inherited protocol methods (protocol inheritance chain)
- [x] Add call-site constraint verification (self-contained, movable function)
- [x] Emit diagnostic (not hard error) for unsupported generic protocol bounds (defer to associated types)

**New Diagnostics** (implemented):
- [x] `UnconstrainedTypeParameterMemberError` - accessing member on type param with no bounds
- [x] `MethodNotInBoundsError` - method not found in any protocol bound
- [x] `AmbiguousConstrainedMethodError` - method found in multiple bounds with same signature
- [x] `ConstraintNotSatisfiedError` - call site fails to satisfy bound
- [x] `UnsupportedGenericProtocolBoundError` - generic protocol bounds deferred

**Example** (now works):
```kestrel
protocol Add {
    func add(other: Self) -> Self
}

func addThem[T](a: T, b: T) -> T where T: Add {
    return a.add(b)  // ✅ Works - looks up `add` from protocol bound
}
```

### Associated Types
**Status**: ✅ DONE

Protocol-level type placeholders that conforming types must specify.

**What was done**:
- [x] Parser support for `type Item` declarations in protocols
- [x] Associated type symbol representation (`AssociatedTypeSymbol`)
- [x] Associated type resolution in conforming types
- [x] Associated type constraints (`where T.Item: Equatable`)
- [x] Qualified bindings (`type Iterator.Item = Int`)
- [x] Protocol inheritance with associated type constraints
- [x] Nested associated type paths (`C.Iter.Item`)
- [x] Constraint satisfaction validation
- [x] Default associated types with override support

**Example**:
```kestrel
protocol Iterator {
    type Item
    func next() -> Item?
}

struct IntRange: Iterator {
    type Item = Int
    func next() -> Int? { ... }
}
```

### Protocol Method Linking
**Status**: TODO

Link struct methods to the protocol methods they implement.

**Tasks**:
- [ ] Track which protocol a method satisfies when struct conforms
- [ ] Resolve protocol method calls to concrete implementations
- [ ] Error if conforming type is missing required methods

**Example**:
```kestrel
protocol Drawable {
    func draw()
}

struct Circle: Drawable {
    func draw() { ... }  // Linked to Drawable.draw
}

func render[T](item: T) where T: Drawable {
    item.draw()  // Resolves to Circle.draw when T = Circle
}
```

### Extensions with Conformances
**Status**: TODO

Add protocol conformances to existing types via extensions.

**Tasks**:
- [ ] Parser support for `extend Type: Protocol { ... }`
- [ ] Extension symbol representation
- [ ] Methods in extension satisfy protocol requirements
- [ ] Retroactive conformance (add conformance to types you don't own)

**Example**:
```kestrel
protocol Printable {
    func toString() -> String
}

struct Point {
    var x: Int
    var y: Int
}

extend Point: Printable {
    func toString() -> String {
        return "Point"
    }
}
```

---

## Future Work (Deferred)

### Static Methods on Type Parameters
**Status**: DEFERRED (implement after constraint enforcement)

Support calling static methods on type parameters: `T.create()`.

**Tasks**:
- [ ] Recognize when a path refers to a type parameter used as a value
- [ ] Handle `TypeParameter.method()` syntax in member resolution
- [ ] Look up static methods from protocol bounds

**Example**:
```kestrel
protocol Factory {
    static func create() -> Self
}

func makeOne[T]() -> T where T: Factory {
    return T.create()  // Static call on type parameter
}
```

### Tighten Type Parameter Assignability
**Status**: DEFERRED (requires proper generic instantiation tracking)

Currently `is_assignable_to` allows any type parameter to be assigned to any other. This is intentionally permissive for Phase 5 but should be tightened.

**Tasks**:
- [ ] Only same type parameter should be assignable to itself
- [ ] Track type parameter identity through function calls
- [ ] Handle generic instantiation properly

---

## Notes

- Type aliases are expanded for comparison
- No implicit coercions (`Int` ≠ `Float`)
- `Self` type is compatible with the containing struct/protocol type
- Type parameter types currently treated as compatible with anything (to be fixed by constraint enforcement)
