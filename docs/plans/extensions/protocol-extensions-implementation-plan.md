# Protocol Extensions Implementation Plan

This document outlines the step-by-step implementation plan for protocol extensions.

## Phase 1: Binder - Protocol Extension Recognition

### Step 1.1: Update Extension Binder to Accept Protocol Targets

**File:** `lib/kestrel-semantic-tree-binder/src/binders/extension.rs`

Currently, the extension binder only accepts struct/enum targets. Update to also accept protocols:

1. In the target type resolution, allow `TyKind::Protocol` as a valid target
2. For protocol targets:
   - No type arguments to extract (protocols don't have substitutions in the same way)
   - No referenced type parameters from target
   - Where clause is purely the extension's own constraints

**Key changes:**
- Add branch for `TyKind::Protocol` in target resolution
- Create `ExtensionTargetBehavior` with protocol type as target
- Register in `ExtensionRegistry` by protocol's SymbolId

### Step 1.2: Add Protocol Extension Detection Helper

**File:** `lib/kestrel-semantic-tree/src/behavior/extension_target.rs`

Add helper method to `ExtensionTargetBehavior`:

```rust
pub fn is_protocol_extension(&self) -> bool {
    matches!(self.target_type.kind(), TyKind::Protocol { .. })
}

pub fn target_protocol(&self) -> Option<&Arc<ProtocolSymbol>> {
    if let TyKind::Protocol { symbol, .. } = self.target_type.kind() {
        Some(symbol)
    } else {
        None
    }
}
```

### Step 1.3: Test Protocol Extension Binding

**File:** `lib/kestrel-test-suite/tests/declarations/extensions.rs`

Add tests:
- Basic protocol extension parses and binds
- Protocol extension registered in ExtensionRegistry
- Protocol extension with where clause binds correctly

---

## Phase 2: Where Clause - Self Handling

### Step 2.1: Add Self Constraint Variant

**File:** `lib/kestrel-semantic-tree/src/ty/where_clause.rs`

Add new constraint variant for Self-based constraints:

```rust
pub enum Constraint {
    // ... existing variants ...

    /// A constraint on Self or Self's associated type in a protocol extension
    /// Syntax: `Self: Protocol` or `Self.Item: Protocol`
    SelfBound {
        /// The path after Self (empty for `Self: Protocol`, ["Item"] for `Self.Item: Protocol`)
        associated_type_path: Vec<String>,
        /// Spans for error reporting
        path_spans: Vec<Span>,
        /// The bounds that must be satisfied
        bounds: Vec<Ty>,
    },
}
```

### Step 2.2: Update Where Clause Resolution for Self

**File:** `lib/kestrel-semantic-tree-binder/src/binders/where_clause.rs` (or wherever where clauses are resolved)

When resolving a where clause constraint in a protocol extension context:

1. Check if the first path element is "Self"
2. If so, create `Constraint::SelfBound` instead of `Constraint::TypeBound`
3. Parse remaining path elements as associated type path

### Step 2.3: Test Self Constraints

**File:** `lib/kestrel-test-suite/tests/declarations/extensions.rs`

Add tests:
- `extend Protocol where Self: OtherProtocol` binds correctly
- `extend Protocol where Self.Item: Bound` binds correctly
- Error on `Self` in non-protocol-extension context

---

## Phase 3: Method Resolution - Protocol Extension Lookup

### Step 3.1: Add Protocol Extensions Query

**File:** `lib/kestrel-semantic-model/src/queries/` (new file or extend existing)

Add query to get protocol extensions:

```rust
pub struct ProtocolExtensionsFor {
    pub protocol_id: SymbolId,
}
```

This returns all extensions where the target is the given protocol.

### Step 3.2: Update Member Resolution

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

In `resolve_member_access`, after checking type extensions:

1. Get protocols the concrete type conforms to
2. For each protocol:
   - Query extensions for that protocol
   - Filter to applicable extensions (where clause satisfaction)
   - Search for the member
3. Collect all matches across all protocols
4. Apply specificity ordering
5. Error if ambiguous (multiple matches with equal specificity)

### Step 3.3: Implement Self Constraint Satisfaction

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs` (or utils)

Add function to check if a protocol extension's where clause is satisfied:

```rust
fn is_protocol_extension_applicable(
    extension: &ExtensionSymbol,
    concrete_type: &Ty,
    ctx: &BodyResolutionContext,
) -> bool {
    // Get where clause from ExtensionTargetBehavior
    // For each SelfBound constraint:
    //   - Self: Protocol -> check concrete_type conforms to Protocol
    //   - Self.Item: Protocol -> resolve concrete_type's Item binding, check conformance
}
```

### Step 3.4: Implement Associated Type Resolution

When checking `Self.Item: Equatable` against concrete type `IntRange`:

1. Find the protocol that declares `Item` (e.g., `Iterator`)
2. Find `IntRange`'s conformance to `Iterator`
3. Find the type alias in `IntRange` with `ConformsToBehavior` pointing to `Iterator.Item`
4. Get the bound type (e.g., `Int`)
5. Check if `Int` conforms to `Equatable`

### Step 3.5: Test Method Resolution

**File:** `lib/kestrel-test-suite/tests/declarations/extensions.rs`

Add tests:
- Method from protocol extension is found
- Concrete type's method takes priority over protocol extension
- Type extension takes priority over protocol extension
- Most constrained protocol extension wins
- Ambiguous protocol extensions produce error

---

## Phase 4: Body Resolution in Protocol Extensions

### Step 4.1: Self Type in Protocol Extension Methods

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/`

When resolving the body of a method in a protocol extension:

1. `self` has type `Self` (TyKind::SelfType)
2. Method calls on `self` should resolve against the protocol's methods
3. Associated type references like `Self.Item` should work

### Step 4.2: Protocol Method Access on Self

When calling a method on `self` in a protocol extension:

1. Get the target protocol from the extension
2. Look up the method in the protocol's flattened methods
3. The return type may reference `Self` or associated types

### Step 4.3: Test Protocol Extension Bodies

**File:** `lib/kestrel-test-suite/tests/declarations/extensions.rs`

Add tests:
- Can call protocol methods on `self`
- Can use `Self` in return types
- Can use associated types in method bodies
- Can call other protocol extension methods

---

## Phase 5: Specificity and Conflict Resolution

### Step 5.1: Calculate Protocol Extension Specificity

**File:** `lib/kestrel-semantic-tree/src/behavior/extension_target.rs`

Add or update specificity calculation for protocol extensions:

```rust
pub fn protocol_extension_specificity(&self) -> usize {
    // Count constraints in where clause
    self.where_clause.constraints().len()
}
```

### Step 5.2: Implement Conflict Detection

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs`

When multiple protocol extensions provide the same method:

1. Group by specificity
2. If single highest-specificity match → use it
3. If multiple matches at highest specificity → error (ambiguous)

### Step 5.3: Test Specificity

**File:** `lib/kestrel-test-suite/tests/declarations/extensions.rs`

Add tests:
- More constrained extension wins
- Equal specificity from same protocol = error
- Equal specificity from different protocols = error

---

## Phase 6: Edge Cases and Polish

### Step 6.1: Validation

Add semantic analysis for:
- Protocol extension methods can't conflict with protocol requirements (same signature)
- Warning if extension method shadows a requirement without providing implementation

### Step 6.2: Error Messages

Ensure clear error messages for:
- Ambiguous method resolution
- Self used outside protocol extension
- Invalid associated type path

### Step 6.3: Documentation

Update:
- `docs/semantics/protocols.md`
- `docs/semantics/extensions.md`
- Add examples to test suite as documentation

---

## Implementation Order

Recommended order to minimize merge conflicts and allow incremental testing:

1. **Phase 1.1-1.2**: Binder accepts protocol targets (can merge independently)
2. **Phase 1.3**: Basic binding tests
3. **Phase 2**: Self constraint handling
4. **Phase 3.1-3.2**: Basic method resolution lookup
5. **Phase 4**: Body resolution in extensions
6. **Phase 3.3-3.5**: Constraint satisfaction and full resolution
7. **Phase 5**: Specificity and conflicts
8. **Phase 6**: Polish and edge cases

---

## Files to Modify

| File | Changes |
|------|---------|
| `lib/kestrel-semantic-tree-binder/src/binders/extension.rs` | Accept protocol targets |
| `lib/kestrel-semantic-tree/src/behavior/extension_target.rs` | Add helper methods |
| `lib/kestrel-semantic-tree/src/ty/where_clause.rs` | Add SelfBound constraint |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/members.rs` | Protocol extension lookup |
| `lib/kestrel-semantic-tree-binder/src/body_resolver/utils.rs` | Constraint satisfaction helpers |
| `lib/kestrel-semantic-model/src/queries/` | Protocol extensions query |
| `lib/kestrel-test-suite/tests/declarations/extensions.rs` | Tests |

---

## Estimated Complexity

- Phase 1: Low - straightforward extension of existing patterns
- Phase 2: Medium - new constraint type and resolution logic
- Phase 3: High - most complex phase, method resolution changes
- Phase 4: Medium - extends existing body resolution
- Phase 5: Low - builds on Phase 3 infrastructure
- Phase 6: Low - polish and edge cases
