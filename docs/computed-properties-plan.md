# Computed Properties Implementation Plan

## Overview

Add computed properties to Kestrel with full parser and semantic support. Computed properties look like fields but execute getter/setter code when accessed.

**Design decisions:**
- Only `var` (not `let`) for computed properties
- Swift-style syntax: `{ get { } set { } }` or shorthand `{ expr }`
- Implicit `newValue` parameter in setters
- Same visibility for getter/setter (no split)
- Allowed in: structs, enums, protocols, extensions
- Static computed properties supported

## Architecture

### Semantic Tree Structure

```
FieldSymbol (isEmpty: Bool, is_computed=true)
  ├── GetterSymbol (CallableBehavior + ExecutableBehavior)
  └── SetterSymbol (optional, CallableBehavior + ExecutableBehavior)
```

- Field remains the parent symbol
- Getter/setter are child symbols with callable + executable behaviors
- `FieldAccess` expression unchanged - lowering handles computed vs stored

### Flow

```
Source: obj.isEmpty
   ↓
Semantic: FieldAccess { object, field: "isEmpty" }
   ↓
Lowering: field.is_computed()?
   ↓
MIR: Call(getter) or direct field load
```

---

## Phase 1: Parser

### 1.1 Add SyntaxKinds

**File:** `lib/kestrel-syntax-tree/src/lib.rs`

Add after `FieldDeclaration`:
```rust
GetterClause,      // get { ... }
SetterClause,      // set { ... }
PropertyAccessors, // { get { } set { } } or { get } { get set }
```

### 1.2 Extend FieldDeclarationData

**File:** `lib/kestrel-parser/src/common/data.rs`

```rust
pub struct FieldDeclarationData {
    // ... existing fields ...
    /// For computed properties: shorthand body OR accessors
    pub computed_body: Option<ComputedBodyData>,
}

pub enum ComputedBodyData {
    /// Shorthand: `{ expr }`
    Shorthand(CodeBlockData),
    /// Explicit: `{ get { } set { } }`
    Accessors {
        getter: Option<CodeBlockData>,  // None for protocol `{ get }`
        setter: Option<CodeBlockData>,  // None for protocol `{ get set }`
    },
}
```

### 1.3 Update Field Parser

**File:** `lib/kestrel-parser/src/common/parsers.rs`

After parsing type, check for `{`:
- If followed by `get` or `set` keyword → parse accessors
- Otherwise → parse as shorthand expression body

### 1.4 Add Accessor Methods to FieldDeclaration

**File:** `lib/kestrel-parser/src/field/mod.rs`

```rust
pub fn is_computed(&self) -> bool
pub fn getter_body(&self) -> Option<SyntaxNode>
pub fn setter_body(&self) -> Option<SyntaxNode>
pub fn is_getter_only(&self) -> bool
```

---

## Phase 2: Semantic Symbols

### 2.1 Add GetterSymbol and SetterSymbol

**File:** `lib/kestrel-semantic-tree/src/symbol/mod.rs`

Add new symbol kinds:
```rust
pub enum KestrelSymbolKind {
    // ... existing ...
    Getter,
    Setter,
}
```

**New file:** `lib/kestrel-semantic-tree/src/symbol/getter.rs`

```rust
pub struct GetterSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}

impl GetterSymbol {
    // Name will be synthetic: "get:fieldName"
    pub fn new(parent: SymbolId, field_name: &str, name_span: Span, full_span: Span) -> Self
}
```

**New file:** `lib/kestrel-semantic-tree/src/symbol/setter.rs`

```rust
pub struct SetterSymbol {
    metadata: SymbolMetadata<KestrelLanguage>,
}
```

### 2.2 Extend FieldSymbol

**File:** `lib/kestrel-semantic-tree/src/symbol/field.rs`

```rust
pub struct FieldSymbol {
    // ... existing ...
    is_computed: bool,
}

impl FieldSymbol {
    pub fn is_computed(&self) -> bool

    /// Get getter symbol (child)
    pub fn getter(&self) -> Option<SymbolRef<GetterSymbol>>

    /// Get setter symbol (child)
    pub fn setter(&self) -> Option<SymbolRef<SetterSymbol>>
}
```

---

## Phase 3: Builder

### 3.1 Update FieldBuilder

**File:** `lib/kestrel-semantic-tree-builder/src/builders/field.rs`

In `build_declaration`:
1. Check if field has computed body (via syntax accessor)
2. Set `is_computed = true` on FieldSymbol
3. Create GetterSymbol as child
4. If setter present, create SetterSymbol as child

---

## Phase 4: Binder

### 4.1 Create GetterBinder and SetterBinder

**New file:** `lib/kestrel-semantic-tree-binder/src/binders/getter.rs`

```rust
impl Binder for GetterBinder {
    fn bind_signature(&self, ctx: &mut BinderContext) {
        // Add CallableBehavior:
        // - No parameters
        // - Return type = parent field's type
        // - Receiver = Borrowing (instance) or None (static)
    }

    fn bind_body(&self, ctx: &mut BinderContext) {
        // Find getter body in syntax
        // Resolve body with self in scope
        // Add ExecutableBehavior
    }
}
```

**New file:** `lib/kestrel-semantic-tree-binder/src/binders/setter.rs`

```rust
impl Binder for SetterBinder {
    fn bind_signature(&self, ctx: &mut BinderContext) {
        // Add CallableBehavior:
        // - One parameter: newValue with field's type
        // - Return type = Unit
        // - Receiver = Mutating (instance) or None (static)
    }

    fn bind_body(&self, ctx: &mut BinderContext) {
        // Find setter body in syntax
        // Resolve body with self and newValue in scope
        // Add ExecutableBehavior
    }
}
```

### 4.2 Update FieldBinder

**File:** `lib/kestrel-semantic-tree-binder/src/binders/field.rs`

After binding field type:
1. If field is computed:
   - Add `ComputedMemberAccessBehavior` (not regular `MemberAccessBehavior`)
   - This behavior holds references to getter/setter symbols
2. Delegate to GetterBinder/SetterBinder for children

**New file:** `lib/kestrel-semantic-tree/src/behavior/computed_member_access.rs`

```rust
pub struct ComputedMemberAccessBehavior {
    member_name: String,
    member_type: Ty,
    getter: SymbolId,           // Always present
    setter: Option<SymbolId>,   // None for getter-only
}

impl ComputedMemberAccessBehavior {
    pub fn access(&self, parent: Expression, span: Span) -> Expression {
        // Returns FieldAccess - lowering will convert to getter call
        Expression::field_access(parent, self.member_name.clone(), ...)
    }
}
```

---

## Phase 5: Semantic Analysis

### 5.1 Assignment Validation

**File:** `lib/kestrel-semantic-tree-binder/src/body_resolver/` (or analyzers)

When resolving assignment to a field:
1. Check if field is computed
2. If computed and no setter → error: "cannot assign to read-only property 'X'"
3. If computed with setter → valid (lowering will generate setter call)

### 5.2 Let vs Var Validation

Computed properties must use `var`:
- Error: "computed properties must use 'var', not 'let'"

---

## Phase 6: Protocol Property Requirements

### 6.1 Syntax

```kestrel
protocol Container {
    var count: Int { get }        // read-only requirement
    var value: Int { get set }    // read-write requirement
}
```

### 6.2 Conformance Checking

When checking protocol conformance:
1. Find field with matching name and type
2. If protocol requires `{ get }` → field must exist (stored or computed getter)
3. If protocol requires `{ get set }` → field must be mutable OR have setter

---

## Phase 7: Lowering/Codegen

### 7.1 Field Access Lowering

When lowering `FieldAccess`:
1. Look up field symbol
2. If `field.is_computed()`:
   - Get getter symbol
   - Generate call to getter
3. Else:
   - Generate direct field load

### 7.2 Assignment Lowering

When lowering assignment to field:
1. Look up field symbol
2. If `field.is_computed()`:
   - Get setter symbol
   - Generate call to setter with RHS as argument
3. Else:
   - Generate direct field store

---

## Files to Modify/Create

### Parser
- `lib/kestrel-syntax-tree/src/lib.rs` - Add SyntaxKinds
- `lib/kestrel-parser/src/common/data.rs` - Add ComputedBodyData
- `lib/kestrel-parser/src/common/parsers.rs` - Parse computed bodies
- `lib/kestrel-parser/src/common/emitters.rs` - Emit computed bodies
- `lib/kestrel-parser/src/field/mod.rs` - Add accessor methods

### Semantic Tree
- `lib/kestrel-semantic-tree/src/symbol/mod.rs` - Add Getter/Setter kinds
- `lib/kestrel-semantic-tree/src/symbol/getter.rs` - NEW
- `lib/kestrel-semantic-tree/src/symbol/setter.rs` - NEW
- `lib/kestrel-semantic-tree/src/symbol/field.rs` - Add is_computed, getter(), setter()
- `lib/kestrel-semantic-tree/src/behavior/computed_member_access.rs` - NEW ComputedMemberAccessBehavior

### Builder
- `lib/kestrel-semantic-tree-builder/src/builders/field.rs` - Build getter/setter children
- `lib/kestrel-semantic-tree-builder/src/builders/getter.rs` - NEW
- `lib/kestrel-semantic-tree-builder/src/builders/setter.rs` - NEW

### Binder
- `lib/kestrel-semantic-tree-binder/src/binders/field.rs` - Delegate to getter/setter binders
- `lib/kestrel-semantic-tree-binder/src/binders/getter.rs` - NEW
- `lib/kestrel-semantic-tree-binder/src/binders/setter.rs` - NEW

### Analyzers
- Assignment validation for computed properties
- Let vs var validation

### Lowering
- Field access lowering
- Assignment lowering

---

## Verification

```bash
# Parser tests
cargo test -p kestrel-parser

# Semantic tests
cargo test -p kestrel-semantic-tree-binder

# Integration - check stdlib compiles
cargo run -- check lang/std/core/int64.ks
cargo run -- check lang/std/text/string.ks

# Full stdlib check
cargo run -- check lang/std/**/*.ks 2>&1 | grep "^error:" | wc -l
```

---

## Design Decisions

1. **Getter/setter symbol naming** - Use synthetic prefix: `get:isEmpty`, `set:isEmpty`

2. **Computed field behavior** - New `ComputedMemberAccessBehavior` (not reusing MemberAccessBehavior)

3. **Static computed properties in protocols** - Yes, supported: `static var zero: Self { get }`
