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

### Static Methods on Type Parameters

**Status**: ✅ DONE

Support calling static methods and initializers on type parameters.

**What was done**:

- [x] Protocol initializer declarations (`init()` in protocol bodies)
  - Added `Initializer` variant to `ProtocolBodyItem`
  - Made initializer body optional (`Option<CodeBlockData>`)
  - Updated `InitializerResolver` to allow initializers in protocols
  - Fixed parameter label handling for single-name parameters
- [x] Calling initializers on type parameters (`T()`)
  - Updated `resolve_type_param_init_call()` to find initializers
  - Full call resolution with signature matching
- [x] Inherited protocol method/init lookup
  - Updated `collect_protocol_static_methods()` to use flattened behavior
  - Updated `collect_protocol_initializers()` to recursively search inherited protocols
  - Both now properly traverse protocol hierarchies
- [x] Type parameter validation
  - Added `validate_not_standalone_type_param()` to prevent using `T` as a value
  - Applied in variable initializers, return statements, and function arguments
- [x] Generic protocol bound validation
  - Detects `T: Container[E]` syntax (generic protocol bounds)
  - Emits `UnsupportedGenericProtocolBoundError` during where clause resolution
  - Prevents invalid generic bounds before semantic analysis

**Test Results**: 840 tests passing, all static type parameter tests complete

### Protocol Method Linking

**Status**: ✅ DONE

Link struct methods to the protocol methods they implement.

**What was done**:

- [x] Track which protocol a method satisfies when struct conforms
- [x] Resolve protocol method calls to concrete implementations
- [x] Error if conforming type is missing required methods
- [x] `ProtocolImplementationBehavior` for storing method bindings

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

### Tighten Type Parameter Assignability

**Status**: ✅ DONE

Type parameters are now only assignable to themselves (same SymbolId).

**What was done**:

- [x] Updated `is_assignable_to` in `lib/kestrel-semantic-tree/src/ty/mod.rs`
- [x] Type parameters compared by SymbolId (same symbol = assignable)
- [x] Type parameter vs any other type = not assignable
- [x] Fixed substitutions for generic struct field access
- [x] Fixed substitutions for Call expressions (stored in Expression)
- [x] Fixed Self substitution for protocol method calls on type parameters
- [x] Fixed explicit type arguments on method calls

**Rules**:

- `T` assignable to `T` ✓ (same SymbolId)
- `T` NOT assignable to `U` ✗ (different type parameters)
- `T` NOT assignable to `Int` ✗ (type param vs concrete)
- `Int` NOT assignable to `T` ✗ (concrete vs type param)

---

### Where Clause Equality Constraints

**Status**: TODO

The parser supports `where T.Item == Int` syntax, but the semantic layer ignores it. This prevents expressing type equality constraints.

**Current State**:

- Parser: ✅ `TypeEqualityData` parsed correctly
- Syntax tree: ✅ `TypeEquality` nodes emitted
- `WhereClause::Constraint`: ❌ No `TypeEquality` variant
- `extract_where_clause()`: ❌ Ignores `TypeEquality` nodes
- Type checking: ❌ Cannot enforce equality constraints

**Tasks**:

- [ ] Add `TypeEquality { left: TypePath, right: Ty }` variant to `Constraint` enum
- [ ] Update `extract_where_clause()` to parse `TypeEquality` syntax nodes
- [ ] Make `is_assignable_to` consult where clause for type equality
- [ ] Handle `where T == U` (type parameter equality)
- [ ] Handle `where T.Item == Int` (associated type equality)
- [ ] Handle `where T.Item == U.Item` (associated type to associated type)

**Example** (should work after implementation):

```kestrel
func intOnly[T](iter: T) where T: Iterator, T.Item == Int {
    // T.Item is known to be Int
}

func transfer[T, U](x: T) -> U where T == U {
    return x  // Valid - T and U are constrained equal
}
```

---

## Notes

- Type aliases are expanded for comparison
- No implicit coercions (`Int` ≠ `Float`)
- `Self` type is compatible with the containing struct/protocol type
- Type parameters only assignable to themselves (same SymbolId) - strict checking now enforced
