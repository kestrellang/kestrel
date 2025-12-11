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

**Status**: 🔄 IN PROGRESS (85% complete - fixing stack overflow with complex generics)

Add protocol conformances to existing types via extensions.

**Completed**:

- [x] Lexer: `Extend` token in `lib/kestrel-lexer/src/lib.rs`
- [x] Syntax tree: `ExtensionDeclaration`, `ExtensionBody`, `Extend` SyntaxKinds
- [x] Parser: `extension_declaration_parser_internal()` using `ty_parser()` for target
- [x] Emitters: `emit_extension_declaration`, `emit_extension_body_item`
- [x] Semantic symbol: `ExtensionSymbol` (~110 lines)
- [x] Behavior: `ExtensionTargetBehavior` with `target_type`, `type_arguments`, `referenced_type_parameters`, `where_clause`
- [x] Registry: `ExtensionRegistry` - `HashMap<SymbolId, Vec<SymbolId>>` for lookup by target
- [x] Resolver: `ExtensionResolver` with BUILD (creates symbol) and BIND (resolves target, registers) phases
- [x] Extension method resolution - methods in extensions are found during member lookup
- [x] Conformance satisfaction - extension methods count toward protocol requirements
- [x] Type parameter substitution - `self.field` in specialized extensions resolves correctly
- [x] Basic generic extensions - `extend Box[T]` works
- [x] Specialized extensions - `extend Box[Int]` works
- [x] Test suite: `lib/kestrel-test-suite/tests/declarations/extensions.rs` (~660 lines, 38 tests)

**Current Issue**: Stack overflow with complex type parameter patterns

- **Symptom**: Tests with swapped type parameters (e.g., `Pair[U, T]` return type from `Pair[T, U]` target) cause infinite recursion
- **Examples**: `extension_two_type_params_generic`, some generic extension tests
- **Workaround**: Applicability filtering temporarily disabled to avoid stack overflow
- **Impact**: All extensions are searched instead of only applicable ones (correctness OK, performance issue)

**Remaining Tasks**:

- [ ] Fix stack overflow in type comparison/resolution for complex generic patterns
- [ ] Re-enable extension applicability filtering with recursion guards
- [ ] Specialized extension priority (more specific extensions should win)
- [ ] Generic type inference for extension method calls
- [ ] Conflict detection at same specificity level

**Key Files**:

- `lib/kestrel-parser/src/extension/mod.rs` (242 lines) - parser
- `lib/kestrel-semantic-tree/src/symbol/extension.rs` (~110 lines) - symbol
- `lib/kestrel-semantic-tree/src/behavior/extension_target.rs` (~100 lines) - behavior
- `lib/kestrel-semantic-tree-builder/src/resolvers/extension.rs` (~486 lines) - resolver
- `lib/kestrel-semantic-tree-builder/src/database/extension_registry.rs` (~100 lines) - registry

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

**Status**: ✅ DONE

Type equality constraints in where clauses are now fully supported.

**What was done**:

- [x] Changed syntax from `==` to `=` for equality constraints
- [x] Added `TypeEquality { left: Ty, right: Ty, span: Span }` variant to `Constraint` enum
- [x] Updated `resolve_where_clause()` to handle `SyntaxKind::TypeEquality` nodes
- [x] Added `resolve_type_equality()` to resolve both sides of equality constraints
- [x] Added `resolve_path_in_where_clause()` for resolving T.Item paths using collected bounds
- [x] Implemented constraint-aware type assignability (`is_assignable_with_constraints`)
- [x] Walk parent chain to collect all where clause constraints
- [x] Normalize types using equality constraints before assignability check
- [x] Handle `where T = U` (type parameter equality)
- [x] Handle `where T.Item = Int` (associated type equality)
- [x] Handle `where T.Item = U.Item` (associated type to associated type)

**Test Results**: 840 tests passing

**Example** (now works):

```kestrel
func intOnly[T](iter: T) where T: Iterator, T.Item = Int {
    // T.Item is known to be Int
}

func collect[T, U](iter: T) -> U where T: Iterator, T.Item = U {
    iter.next()  // ✅ Works - T.Item equals U
}

func zip[A, B](a: A, b: B) where A: Iterator, B: Iterator, A.Item = B.Item {
    // ✅ Works - A.Item and B.Item are constrained equal
}
```

---

## Notes

- Type aliases are expanded for comparison
- No implicit coercions (`Int` ≠ `Float`)
- `Self` type is compatible with the containing struct/protocol type
- Type parameters only assignable to themselves (same SymbolId) - strict checking now enforced
- Where clause equality constraints use `=` syntax (not `==`) to avoid confusion with comparison operators
