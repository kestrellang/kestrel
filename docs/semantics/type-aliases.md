# Type Aliases

Type aliases create alternative names for existing types, improving code readability and enabling abstraction.

## Syntax

```
TypeAliasDeclaration → Visibility? TYPE Identifier EQUALS Identifier SEMICOLON
```

### Tokens
- `TYPE` - The `type` keyword
- `EQUALS` - The `=` character
- `SEMICOLON` - The `;` character
- `Visibility` - Optional visibility modifier

**Note:** Currently, the aliased type is limited to a single identifier. Full type expressions (tuples, function types, paths) are planned.

## Examples

```kestrel
// Simple alias
type ID = Int;

// With visibility
public type UserID = Int;
private type InternalID = Int;

// Aliasing another type
type Name = String;
type Coordinate = Float;
```

## Semantic Rules

### Rule 1: No Self-Reference

A type alias cannot directly reference itself.

```
ERROR: CircularTypeAliasError
WHEN: Type alias body contains its own name
WHY: Would create an infinite type with no base
```

**Example (invalid):**
```kestrel
type A = A;    // ERROR: circular type alias: A -> A
```

### Rule 2: No Indirect Circular References

Type aliases cannot form circular chains through other aliases.

```
ERROR: CircularTypeAliasError
WHEN: Following the alias chain leads back to the starting alias
WHY: Would create an infinite type with no resolution
```

**Example (invalid):**
```kestrel
type A = B;
type B = C;
type C = A;    // ERROR: circular type alias: A -> B -> C -> A
```

**Error reporting:**
The error message includes the complete cycle chain to help identify the problem:
- Origin alias (where cycle was detected)
- Full path through the cycle
- Returns to origin

### Rule 3: Aliased Type Must Exist

The target of a type alias must resolve to a valid type.

```
ERROR: (type resolution error)
WHEN: The aliased type name doesn't exist
WHY: Cannot create an alias to a non-existent type
```

**Example (invalid):**
```kestrel
type MyType = NonExistent;    // ERROR: type 'NonExistent' not found
```

### Rule 4: Visibility Consistency

A public type alias cannot expose a less-visible type.

```
ERROR: VisibilityConsistencyError
WHEN: Public type alias refers to private/internal/fileprivate type
WHY: Would allow external code to use types they can't directly access
```

**Example (invalid):**
```kestrel
private struct Secret { }
public type Exposed = Secret;    // ERROR: public type alias 'Exposed' exposes private type 'Secret'
```

See [Visibility](visibility.md) for complete visibility rules.

## Type Alias Resolution

### Build Phase

During the build phase, type aliases are created with their syntactic type:

```
TypeAliasSymbol {
    name: "MyAlias",
    typed_behavior: TypedBehavior(Path(["TargetType"])),  // Unresolved
    visibility: Public,
}
```

### Bind Phase

During binding, the aliased type is resolved:

1. **Enter cycle detection** - Track that we're resolving this alias
2. **Resolve the type** - Convert path to concrete type
3. **Check for cycles** - Detect if resolution leads back to self
4. **Store resolved type** - Add `TypeAliasTypedBehavior` with resolved type
5. **Exit cycle detection** - Allow other aliases to reference this one

### Cycle Detection Algorithm

```
cycle_detector = active_set()

bind_type_alias(alias):
    cycle_detector.enter(alias.id)

    resolved = resolve_type(alias.syntactic_type)

    if contains_cycle(resolved, alias):
        error(CircularTypeAliasError)

    alias.add_behavior(TypeAliasTypedBehavior(resolved))

    cycle_detector.exit(alias.id)

contains_cycle(type, current_alias):
    match type:
        TypeAlias(other):
            if other.id == current_alias.id:
                return true  // Direct cycle
            if cycle_detector.is_active(other.id):
                return true  // Indirect cycle (being resolved)
            return false
        Tuple(elements):
            return any(contains_cycle(e, current_alias) for e in elements)
        Function(params, ret):
            return any(contains_cycle(p, current_alias) for p in params)
                   or contains_cycle(ret, current_alias)
        _:
            return false
```

### Post-Binding Cycle Check

Some cycles aren't detectable during sequential binding (e.g., A → B → A where both are already bound). A post-binding pass catches these:

```
check_all_type_alias_cycles():
    for alias in all_type_aliases:
        chain = follow_alias_chain(alias)
        if alias in chain:
            error(CircularTypeAliasError(chain))

follow_alias_chain(alias):
    chain = []
    current = alias.resolved_type
    while current is TypeAlias:
        if current in chain:
            return chain  // Cycle found
        chain.append(current)
        current = current.resolved_type
    return chain
```

## Type Alias Behaviors

Type alias symbols have two typed behaviors:

1. **TypedBehavior** - The syntactic aliased type (for cycle checking)
2. **TypeAliasTypedBehavior** - The fully resolved type (added after binding)

This distinction allows:
- Cycle detection using the syntactic form
- Type resolution using the resolved form
- Preserving the alias identity vs. its underlying type

## Type Alias vs. Underlying Type

Type aliases create a new name but don't create a new type:

```kestrel
type UserID = Int;
type OrderID = Int;

// UserID and OrderID are both Int
// They are interchangeable (structural, not nominal)
```

**Current behavior:** Aliases are transparent - `UserID` and `Int` are the same type.

**Future consideration:** Opaque type aliases could create distinct types for stronger type safety.

## Examples

### Valid Type Aliases

```kestrel
module MyApp

// Primitive aliases
type Count = Int;
type Amount = Float;
type Flag = Bool;

// Semantic naming
type UserID = Int;
type Timestamp = Int;

// With visibility
public type PublicID = Int;
internal type ModuleID = Int;
private type LocalID = Int;
```

### Invalid Type Aliases

```kestrel
// Self-reference
type Bad = Bad;                    // ERROR: circular

// Mutual recursion
type A = B;
type B = A;                        // ERROR: circular A -> B -> A

// Three-way cycle
type X = Y;
type Y = Z;
type Z = X;                        // ERROR: circular X -> Y -> Z -> X

// Non-existent target
type Missing = NoSuchType;         // ERROR: not found

// Visibility violation
private struct Secret { }
public type Exposed = Secret;      // ERROR: exposes private type
```

## Formal Semantics

### Type Alias Declaration

For a type alias declaration `type A = T`:

```
Preconditions:
    - T must be a valid type expression
    - T must not contain A (directly or transitively)
    - visibility(A) ≤ visibility(T) if T is nominal

Effect:
    - Creates TypeAliasSymbol with name A
    - Binds A to resolved type of T
    - A can be used wherever T is expected
```

### Type Equivalence

```
For type alias A = T:
    A ≡ T (type equivalence)
    uses of A are interchangeable with T
```

### Cycle Detection Invariant

```
For all type aliases A:
    following the chain A → T₁ → T₂ → ... → Tₙ
    must terminate at a non-alias type
    (no Tᵢ = A)
```

## Source Location

- **Build/lowering:** `lib/kestrel-semantic-tree-builder/src/builders/type_alias.rs`
- **Bind:** `lib/kestrel-semantic-tree-binder/src/binders/type_alias.rs`
- **Symbol:** `lib/kestrel-semantic-tree/src/symbol/type_alias.rs`
- **Cycle detector:** `lib/semantic-tree/src/cycle.rs`
- **Errors:** `lib/kestrel-semantic-tree/src/error.rs`
- **Resolve aliased type:** `lib/kestrel-semantic-model/src/queries/resolved_aliased_type.rs`
- **Validate (cycles):** `lib/kestrel-semantic-analyzers/src/analyzers/type_alias_cycles/mod.rs`
