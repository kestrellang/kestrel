# Type Inference & Type Substitutions

This document covers Kestrel's type inference system and the four type substitution mechanisms:
Self type, type aliases, associated types, and generic parameter substitutions.

## Overview

Kestrel uses **Hindley-Milner style constraint-based type inference**. The system is decoupled from the
semantic model via the `TypeOracle` trait, keeping the solver reusable and testable in isolation.

### Where it runs in the pipeline

```
Phase 1 analyzers (pre-inference):
  Cycle detection, conformance resolution, field validation, etc.
      ↓
Phase 2 analyzers (type resolution):
  TypeInferenceAnalyzer   ← RUNS HERE (per function/init/getter/setter)
  Pattern checking, type checking, exhaustiveness
      ↓
Phase 3 analyzers (post-checking):
  Protocol methods, duplicate symbols, visibility, generics
```

The `TypeInferenceAnalyzer` in `kestrel-semantic-analyzers` runs inference for each symbol that
has an `ExecutableBehavior` (code body). It queries `InferenceResultFor { symbol_id }` which
triggers constraint generation, solving, and solution application.

## Key Files

| File | Purpose |
|------|---------|
| `lib/kestrel-semantic-type-inference/src/context.rs` | `InferenceContext` — solver state |
| `lib/kestrel-semantic-type-inference/src/constraint.rs` | `Constraint` enum — type relationships |
| `lib/kestrel-semantic-type-inference/src/constraint_generator.rs` | Generates constraints from code |
| `lib/kestrel-semantic-type-inference/src/solver.rs` | Unification + fixpoint iteration |
| `lib/kestrel-semantic-type-inference/src/solution.rs` | `Solution` — inference result |
| `lib/kestrel-semantic-type-inference/src/oracle.rs` | `TypeOracle` trait — type queries |
| `lib/kestrel-semantic-type-inference/src/apply.rs` | Applies solution back to code |
| `lib/kestrel-semantic-type-inference/src/error.rs` | Inference errors |
| `lib/kestrel-semantic-tree/src/ty/mod.rs` | `Ty`, `TyId` — type representation |
| `lib/kestrel-semantic-tree/src/ty/kind.rs` | `TyKind` enum — all type variants |
| `lib/kestrel-semantic-tree/src/ty/substitutions.rs` | `Substitutions` — type param mapping |
| `lib/kestrel-semantic-tree/src/ty/where_clause.rs` | `WhereClause`, `Constraint` |
| `lib/kestrel-semantic-analyzers/src/analyzers/type_inference/mod.rs` | `TypeInferenceAnalyzer` |

## Type Representation

Every type is a `Ty` with a unique `TyId` (atomic counter), a `TyKind`, and a `Span`.

### TyKind variants

```rust
TyKind::Unit                    // ()
TyKind::Never                   // ! (bottom type)
TyKind::Int(IntBits)            // i8, i16, i32, i64
TyKind::Float(FloatBits)        // f16, f32, f64
TyKind::Bool                    // lang.i1
TyKind::String                  // lang.str
TyKind::Tuple(Vec<Ty>)          // (T1, T2, ...)
TyKind::Pointer(Box<Ty>)        // lang.ptr[T]
TyKind::Function { params, return_type }  // (P1, P2, ...) -> R

// Nominal types — carry Substitutions for their type arguments
TyKind::Struct { symbol, substitutions }       // Array[Int], Point, etc.
TyKind::Enum { symbol, substitutions }         // Optional[T], Result[T,E], etc.
TyKind::Protocol { symbol, substitutions }     // Iterator, Equatable, etc.
TyKind::TypeAlias { symbol, substitutions }    // Expanded during resolution

// Generic system
TyKind::TypeParameter(Arc<TypeParameterSymbol>)  // T, U, etc.
TyKind::AssociatedType { symbol, container }      // Item, Element, etc.
TyKind::SelfType                                  // Self keyword

// Inference
TyKind::Infer                                 // Placeholder — to be determined
TyKind::UnresolvedFunction { param_info, return_type }  // Closure with unknown params
TyKind::UnresolvedPath { segments }            // Path not yet resolved

TyKind::Error                   // Poison value — suppresses cascading errors
```

### TyId

Every `Ty` gets a globally unique `TyId` at construction. The inference system uses TyIds
to track types without cloning. The solver maintains a `HashMap<TyId, Ty>` for substitutions.

---

## Constraint-Based Inference

### Constraints

The solver works with these constraint types:

| Constraint | Meaning | Example |
|-----------|---------|---------|
| `Equals { a, b }` | Types must be identical | `let x: Int = expr` → equate(expr.ty, Int) |
| `Conforms { ty, protocol }` | Type must implement protocol | `5` → conforms(ty, ExpressibleByIntLiteral) |
| `Normalizes { base, assoc_name, result }` | Associated type projection | `T.Item` → normalizes(T, "Item", result) |
| `MemberAccess { receiver, member, ... }` | Type-directed member lookup | `x.foo()` → member_access(x.ty, "foo", ...) |
| `ImplicitMember { expr_ty, member_name, ... }` | Enum shorthand `.Case` | `.Some(v)` → implicit_member(ty, "Some", ...) |
| `EnumPatternBinding { enum_ty, case_name, ... }` | Pattern match on enum case | `case .Some(v):` |
| `StructPatternBinding { struct_ty, ... }` | Pattern match on struct fields | `case Point { x, y }:` |
| `Promotable { from_ty, to_ty }` | Implicit wrapping (Optional, Result) | `let x: Int? = 5` |
| `TupleIndexAccess { tuple, index, result }` | Tuple element access | `t.0` |

### Constraint Generation

`generate_constraints()` walks the code block and creates constraints:

- **Literals**: `conforms(expr.ty, ExpressibleBy*Literal)`
- **Function calls**: `equate(arg.ty, param.ty)` for each argument
- **Deferred method calls**: `member_access(receiver.ty, name, args, result.ty)`
- **Closures**: Create expected function type, unify with closure type
- **Patterns**: `enum_pattern_binding(...)` or `struct_pattern_binding(...)`
- **Return/assignment**: `promotable(value.ty, target.ty)`
- **If branches**: `promotable(branch.ty, if_expr.ty)`
- **Match arms**: `equate(pattern.ty, scrutinee.ty)`

### Solving Algorithm

The solver in `solver.rs` uses fixpoint iteration:

```
1. PRE-SCAN: Identify literal TyIds that have default types (Int, Float, String, etc.)

2. MAIN LOOP (repeat until no progress):
   For each constraint:
     Equals       → unify(a, b)
     Conforms     → oracle.conforms_to(ty, protocol)
     Normalizes   → oracle.resolve_associated_type(base, name), unify with result
     MemberAccess → oracle.resolve_member(receiver, name), create arg constraints
     Promotable   → try unify, else check FromValue conformance
     ...

   Each attempt returns: Solved | Deferred | Error
   - Solved: constraint processed, possibly produced new substitutions
   - Deferred: type not resolved enough yet, retry next round
   - Error: accumulated (non-fatal), solving continues

3. DEFAULT APPLICATION LOOP (until no more defaults):
   For unresolved literals: equate(literal.ty, default_type)
   - Int literal → Int64, Float → Float64, String → String, etc.
   Re-solve new constraints

4. FINAL CHECK: Report any types still Infer as errors
```

### Unification

`unify(a, b)` is the core algorithm:

1. Both `Infer` → map one to the other
2. One `Infer` → occurs check, then substitute
3. `Error` or `Never` → accept (special handling)
4. `Struct`/`Enum`/`Protocol` → check symbol identity, unify substitutions
5. `Function`/`UnresolvedFunction` → unify params and return
6. `Tuple` → check arity, unify elements
7. `TypeAlias` → expand, retry
8. Primitives → exact equality
9. Anything else → type mismatch error

**Occurs check**: Prevents infinite types (`T = List[T]`).

### Solution

```rust
Solution {
    types: HashMap<TyId, Ty>,              // Resolved inference variables
    values: HashMap<ExprId, ValueResolution>,  // Resolved member access symbols
    promotions: HashMap<ExprId, PromotionInfo>, // Expressions needing FromValue wrapping
    errors: Vec<InferenceError>,
}
```

`apply_solution()` walks the code block, replacing `Infer` types with resolved types,
recording promotions, and updating local variable types.

### TypeOracle

The `TypeOracle` trait decouples the solver from the semantic model:

```rust
trait TypeOracle {
    fn resolve_member(&self, receiver_ty: &Ty, member: &str, is_static: bool) -> ...;
    fn conforms_to(&self, ty: &Ty, protocol_id: SymbolId) -> bool;
    fn resolve_associated_type(&self, container: &Ty, assoc_name: &str) -> Option<Ty>;
    fn expand_type_alias(&self, ty: &Ty) -> Ty;
    fn default_integer_type(&self, span: Span) -> Ty;  // Int64
    fn default_float_type(&self, span: Span) -> Ty;    // Float64
    fn check_from_value_conformance(&self, target: &Ty, source: &Ty) -> Option<...>;
    // ... more queries
}
```

The concrete implementation (`ContextualOracle` in `kestrel-semantic-model`) delegates
to the `SemanticModel` query system.

---

## Type Substitution Mechanisms

Kestrel has four distinct type substitution systems. They operate at different phases
and serve different purposes.

### 1. Generic Parameter Substitutions

**What**: Replaces type parameters (`T`, `U`, etc.) with concrete types at instantiation sites.

**Representation**: `Substitutions` — a `HashMap<SymbolId, Ty>` mapping type parameter IDs to types.

**Where it lives**: Carried by every nominal type variant (`Struct`, `Enum`, `Protocol`, `TypeAlias`).

```rust
// Array[Int] is represented as:
TyKind::Struct {
    symbol: <Array symbol>,
    substitutions: { T.id() → Ty::int(I64) }
}

// Dictionary[String, Int] is:
TyKind::Struct {
    symbol: <Dictionary symbol>,
    substitutions: { K.id() → Ty::string(), V.id() → Ty::int(I64) }
}
```

**How substitution works** (`Substitutions::apply`):

```rust
// Given substitutions: { T → Int }
// Apply to: Array[T]
// Result:   Array[Int]

// The apply method:
// 1. TypeParameter(T) → look up T.id() in map → Int
// 2. Struct { Array, {T → param} } → recursively apply to inner substitutions
// 3. Function { params, return } → recursively apply to each
// 4. Tuple, Pointer, etc. → recursively apply to components
// 5. Primitives, Error, SelfType, Infer → return as-is
```

**Cycle detection**: Uses a `visited: HashSet<SymbolId>` to break cycles where
`T → List[T]` would cause infinite recursion.

**When it happens**:
- During type resolution (binding phase) when generic types are instantiated
- During inference when method calls resolve type arguments
- During codegen when monomorphization substitutes all type parameters

### 2. Self Type Substitution

**What**: Replaces the `Self` keyword with the concrete type of the containing struct/enum/protocol.

**Representation**: `TyKind::SelfType` in the type system.

**Key method**: `Ty::substitute_self(replacement: &Ty) -> Ty`

```rust
// Inside struct Array[T]:
//   func clone() -> Self
// Self gets substituted with Array[T] (where T is still a TypeParameter)

// The concrete Self type for a generic struct S[T] is built as:
// Ty::generic_struct(S, { T → TypeParameter(T) })
```

**What it does recursively**:
- `SelfType` → replaced with the concrete type
- `AssociatedType { symbol, container: None }` → `AssociatedType { symbol, container: Some(replacement) }`
  (naked associated types like `Item` implicitly mean `Self.Item`)
- Composite types (Tuple, Function, Struct, Enum, etc.) → recurse into components
- Primitives → unchanged

**When it happens**:
- When entering a method body during binding: the `self` parameter and method signatures
  have `Self` replaced with the concrete struct/enum type
- Protocol default implementations: `Self` remains abstract until conformance resolution

### 3. Type Alias Expansion

**What**: Follows type alias chains to their underlying types.

**Representation**: `TyKind::TypeAlias { symbol, substitutions }`

**Key method**: `Ty::expand_aliases() -> Ty`

```kestrel
// In Kestrel:
type StringArray = Array[String]

// StringArray is:
TyKind::TypeAlias { symbol: <StringArray>, substitutions: {} }

// expand_aliases() follows the chain:
// 1. Get resolved type from TypeAliasTypedBehavior → Array[String]
// 2. Apply substitutions (if any)
// 3. Recursively expand (in case the target is also an alias)
```

**Generic type aliases**:
```kestrel
type Pair[T] = (T, T)
// Pair[Int] expands to (Int, Int)
```

The expansion applies the alias's substitutions to the resolved type before returning.

**When it happens**:
- During `is_assignable_to()` — both sides expanded before comparison
- During inference when the solver encounters a TypeAlias in unification
- The oracle's `expand_type_alias()` is called by the solver when needed

### 4. Associated Type Projection

**What**: Resolves protocol-defined associated types to their concrete implementations.

**Representation**: `TyKind::AssociatedType { symbol, container }`

- `container: None` — unqualified, within the protocol itself (e.g., `func next() -> Item`)
- `container: Some(ty)` — qualified (e.g., `T.Item` where `T: Iterator`)

**Resolution paths**:

**Via Normalizes constraint** (during inference):
```
// Array[Int].iter() returns ArrayIterator[Int]
// ArrayIterator[Int] conforms to Iterator where type Item = Int
// So: normalizes(ArrayIterator[Int], "Item", result) → result = Int

// The solver:
// 1. Resolves base type (ArrayIterator[Int])
// 2. Queries oracle: resolve_associated_type(ArrayIterator[Int], "Item")
// 3. Gets Int
// 4. Unifies result with Int
```

**Via direct oracle query** (during solution application):
```rust
// In apply_solution, unresolved AssociatedType nodes are resolved:
oracle.resolve_associated_type(&container, "Item")
```

**Via Self substitution** (during binding):
```rust
// Inside Iterator protocol:
//   func next() -> Item        // Item has container: None
// When substitute_self(ArrayIterator[Int]):
//   Item becomes ArrayIterator[Int].Item  // container: Some(ArrayIterator[Int])
// Then resolved to Int via oracle
```

**Where clause integration**:
```kestrel
func compactMap[T]() -> ... where Item = Optional[T]
```
Where clauses generate `Normalizes` constraints that drive associated type inference.
The solver extracts where clauses when resolving method calls and converts them to constraints.

---

## Literal Type Inference

Literals get special handling because they have both contextual types and defaults.

**Flow for `let x: Int? = 5`**:

```
1. Constraint generation:
   - conforms(5.ty, ExpressibleByIntLiteral)  // 5 is an int literal
   - promotable(5.ty, Int?)                   // must be assignable to Int?

2. Pre-scan: mark 5.ty as a literal TyId (has default)

3. Main loop round 1:
   - Conforms: Deferred (5.ty is Infer, can't check conformance)
   - Promotable: Deferred (5.ty is literal, defer until default)

4. Default application:
   - 5.ty still Infer → equate(5.ty, Int64)

5. Main loop round 2:
   - Equals: 5.ty = Int64 (substitution recorded)
   - Promotable(Int64, Int?): check FromValue conformance → yes
   - Record promotion: wrap with Optional.from()

6. Solution:
   types: { 5.ty → Int64 }
   promotions: { 5.expr → PromotionInfo { Optional.from, ... } }
```

**Deferral strategy**: Literals with defaults (Int, Float, String, Bool, Char) defer
Promotable constraints to give context a chance to propagate first. Null and array
literals do NOT defer — they need context to resolve at all.

---

## Debugging Type Inference

### Common issues

**"Cannot infer type"**: A TyId remained `Infer` after solving. Usually means:
- Missing constraint (expression not generating the right constraints)
- Constraint deferred forever (receiver type never resolves)

**"Type mismatch"**: Unification failed. Check:
- Are type aliases being expanded? (`expand_aliases()`)
- Are substitutions being applied correctly?
- Is Self being substituted before comparison?

### Adding constraints

If you need to add a new constraint kind:

1. Add variant to `Constraint` enum in `constraint.rs`
2. Add generation logic in `constraint_generator.rs`
3. Add solving logic in `solver.rs` (implement the `try_solve_*` function)
4. Handle the new constraint in the fixpoint loop

### Tracing

The substitutions code has debug logging for `Rhs` parameter substitutions
(in `substitutions.rs`). For general debugging, add `eprintln!` in `solver.rs`
to trace constraint solving progress.
