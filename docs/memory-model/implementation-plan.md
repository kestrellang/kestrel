# Memory Model Implementation Plan

This document details the implementation plan for Kestrel's memory model as described in the memory-model documentation.

## Overview

The memory model implementation is divided into 8 phases, designed to build incrementally with each phase providing value on its own.

| Phase | Feature | Description |
|-------|---------|-------------|
| 1 | Parameter Access Modes | `borrow`/`mutating`/`consuming` for parameters + MIR foundation |
| 2 | Attributes | `@attribute` syntax and semantic processing |
| 3 | Builtin Protocols | `@builtin(.Copyable)` and language feature protocol system |
| 4 | Copyable / not Copyable | Move semantics for non-copyable types |
| 5 | Drop Semantics (RAII) | `deinit` blocks and automatic cleanup |
| 6 | Cloneable Protocol | Custom copy behavior via `clone()` |
| 7 | Generics Integration | `[T: not Copyable]` bounds |
| 8 | Law of Exclusivity | Borrow checking and conflict detection |

---

## Phase 1: Parameter Access Modes + MIR Foundation ✅ COMPLETE

**Goal**: Parameters can have explicit access modes, MIR reflects passing semantics.

### 1.1 Parser Changes

**Files**: `lib/kestrel-lexer/src/lib.rs`, `lib/kestrel-parser/src/common/*.rs`

- [x] Add `mutating` and `consuming` keywords to lexer
- [x] Extend parameter parsing to accept access mode prefix:
  ```kestrel
  func process(consuming p: Point, mutating q: Point, r: Point)
  ```

**Syntax**:
```
parameter := (access_mode)? (label)? name ':' type
access_mode := 'borrow' | 'mutating' | 'consuming'
```

### 1.2 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/symbol/parameter.rs`, `lib/kestrel-semantic-tree/src/behavior/callable.rs`

- [x] Add `ParameterAccessMode` enum (Borrow, Mutating, Consuming)
- [x] Extend `CallableBehavior` to include parameter access modes
- [x] Update parameter binding to extract access mode from syntax

### 1.3 Call-Site Validation

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [x] Validate `mutating` parameters:
  - Argument must be a mutable place (`var` binding or mutable field)
  - Error: "cannot pass `let` binding to `mutating` parameter"

### 1.4 Execution Graph Changes

**Files**: `lib/kestrel-execution-graph/src/*.rs`, `lib/kestrel-execution-graph-lowering/src/*.rs`

- [x] Add `PassingMode` enum to MIR (Ref, MutRef, Copy, Move)
- [x] Add `CallArg` struct with operand and passing mode
- [x] Update function lowering to emit correct passing modes

### 1.5 Diagnostics

- [x] "cannot pass `let` binding `{name}` to `mutating` parameter"
- [x] "cannot pass immutable field `{name}` to `mutating` parameter"

### 1.6 Tests

- [x] Parameter access mode parsing tests
- [x] Call-site validation tests
- [x] MIR passing mode emission tests

---

## Phase 2: Attributes

**Goal**: Add attribute syntax to the language with semantic processing infrastructure.

### 2.1 Syntax

Attributes use the `@` prefix and can optionally take arguments:

```kestrel
@deprecated
public func oldWay() { }

@builtin(.Copyable)
public protocol Copyable {}

@inline(.always)
func hotPath() { }
```

**Grammar**:
```
attribute := '@' identifier attribute_args?
attribute_args := '(' expr_list ')'
expr_list := expression (',' expression)*
```

### 2.2 Parser Changes

**Files**: `lib/kestrel-lexer/src/lib.rs`, `lib/kestrel-syntax-tree/src/lib.rs`, `lib/kestrel-parser/src/`

- [ ] Add `At` token to lexer (the `@` symbol)
- [ ] Add syntax kinds:
  - `SyntaxKind::Attribute`
  - `SyntaxKind::AttributeList`
  - `SyntaxKind::AttributeArgs`
- [ ] Create attribute parser:
  - Parse `@identifier` 
  - Parse optional `(expr, expr, ...)` argument list
- [ ] Integrate attribute parsing before declarations:
  - Protocol declarations
  - Struct declarations
  - Enum declarations
  - Function declarations
  - Field declarations (future)

**Files to modify**:
- `lib/kestrel-parser/src/common/data.rs` - Add `AttributeData`, `AttributeListData`
- `lib/kestrel-parser/src/common/emitters.rs` - Add attribute emitters
- `lib/kestrel-parser/src/attribute/mod.rs` - New module for attribute parsing
- `lib/kestrel-parser/src/protocol/mod.rs` - Accept attributes before protocol
- `lib/kestrel-parser/src/struct/mod.rs` - Accept attributes before struct
- `lib/kestrel-parser/src/func.rs` - Accept attributes before func

### 2.3 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/`

- [ ] Create `AttributeKind` enum for known attributes:
  ```rust
  /// Known attribute types that the compiler understands.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum AttributeKind {
      /// @builtin(.Feature) - marks a protocol as a language feature
      Builtin,
      /// @deprecated - marks a declaration as deprecated
      Deprecated,
      /// @inline(.always | .never) - inlining hints
      Inline,
      // Future attributes...
  }
  ```

- [ ] Create `Attribute` struct:
  ```rust
  /// A resolved attribute on a declaration.
  #[derive(Debug, Clone)]
  pub struct Attribute {
      /// The kind of attribute
      pub kind: AttributeKind,
      /// The resolved arguments (attribute-specific)
      pub args: AttributeArgs,
      /// Source span
      pub span: Span,
  }
  
  /// Attribute arguments, specific to each attribute kind.
  #[derive(Debug, Clone)]
  pub enum AttributeArgs {
      /// No arguments
      None,
      /// @builtin(.Feature)
      Builtin { feature: LanguageFeature },
      /// @deprecated or @deprecated("message")
      Deprecated { message: Option<String> },
      /// @inline(.always) or @inline(.never)
      Inline { mode: InlineMode },
  }
  ```

- [ ] Create `AttributesBehavior`:
  ```rust
  /// Behavior that stores resolved attributes on a symbol.
  #[derive(Debug, Clone)]
  pub struct AttributesBehavior {
      attributes: Vec<Attribute>,
  }
  
  impl AttributesBehavior {
      pub fn has(&self, kind: AttributeKind) -> bool { ... }
      pub fn get(&self, kind: AttributeKind) -> Option<&Attribute> { ... }
  }
  ```

### 2.4 Attribute Resolution

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [ ] Create `AttributeResolver`:
  ```rust
  /// Resolves and validates attributes from syntax.
  pub struct AttributeResolver<'a> {
      ctx: &'a BindingContext<'a>,
  }
  
  impl AttributeResolver {
      /// Resolve an attribute list from syntax.
      pub fn resolve(&self, syntax: &SyntaxNode) -> Vec<Attribute> { ... }
      
      /// Parse arguments for a specific attribute kind.
      fn parse_args(&self, kind: AttributeKind, args: &[Expression]) 
          -> Result<AttributeArgs, Diagnostic> { ... }
  }
  ```

- [ ] Integrate into binders:
  - `ProtocolBinder` - resolve attributes, add `AttributesBehavior`
  - `StructBinder` - resolve attributes, add `AttributesBehavior`
  - `EnumBinder` - resolve attributes, add `AttributesBehavior`
  - `FunctionBinder` - resolve attributes, add `AttributesBehavior`

### 2.5 Diagnostics

- [ ] "unknown attribute `{name}`"
- [ ] "attribute `{name}` does not take arguments"
- [ ] "attribute `{name}` requires arguments"
- [ ] "invalid argument for attribute `{name}`: expected {expected}"
- [ ] "duplicate attribute `{name}`" (for non-repeatable attributes)

### 2.6 Tests

**Files**: `lib/kestrel-test-suite/tests/attributes/`

- [ ] `attribute_parsing.rs`:
  - Simple attribute `@deprecated`
  - Attribute with arguments `@builtin(.Copyable)`
  - Multiple attributes on same declaration
  - Unknown attribute error
- [ ] `attribute_validation.rs`:
  - Missing required arguments
  - Invalid argument types
  - Duplicate attributes

---

## Phase 3: Builtin Protocols

**Goal**: Define the `@builtin` attribute and language feature protocol system.

### 3.1 Language Feature Enum

**Files**: `lib/kestrel-semantic-tree/src/`

- [ ] Create `LanguageFeature` enum:
  ```rust
  /// Built-in language features that protocols can represent.
  /// These are special protocols with compiler-known semantics.
  #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
  pub enum LanguageFeature {
      /// Copy semantics - types conforming can be implicitly copied.
      /// All types implicitly conform unless marked `not Copyable`.
      Copyable,
      
      // Future features:
      // /// Thread-safe types that can be sent across threads.
      // Sendable,
      // /// Types that can escape from the current scope.
      // Escapable,
  }
  ```

- [ ] Create `LanguageFeatureConfig`:
  ```rust
  /// Configuration for a language feature protocol.
  #[derive(Debug, Clone)]
  pub struct LanguageFeatureConfig {
      /// The feature this protocol represents.
      pub feature: LanguageFeature,
      /// Whether types implicitly conform (true for Copyable).
      pub implicit_conformance: bool,
      /// Whether `not` is allowed in conformance lists.
      pub allows_negation: bool,
  }
  
  impl LanguageFeature {
      pub fn config(&self) -> LanguageFeatureConfig {
          match self {
              LanguageFeature::Copyable => LanguageFeatureConfig {
                  feature: LanguageFeature::Copyable,
                  implicit_conformance: true,
                  allows_negation: true,
              },
          }
      }
  }
  ```

### 3.2 Builtin Protocol Registry

**Files**: `lib/kestrel-semantic-tree/src/`, `lib/kestrel-semantic-model/src/`

- [ ] Create `BuiltinRegistry`:
  ```rust
  /// Registry that tracks which protocols are language features.
  pub struct BuiltinRegistry {
      /// Map from language feature to the protocol symbol ID.
      features: HashMap<LanguageFeature, SymbolId>,
      /// Reverse map from protocol symbol ID to feature.
      protocols: HashMap<SymbolId, LanguageFeature>,
  }
  
  impl BuiltinRegistry {
      /// Register a protocol as a language feature.
      pub fn register(&mut self, feature: LanguageFeature, protocol_id: SymbolId);
      
      /// Get the protocol for a language feature.
      pub fn protocol_for(&self, feature: LanguageFeature) -> Option<SymbolId>;
      
      /// Check if a protocol is a language feature.
      pub fn feature_for(&self, protocol_id: SymbolId) -> Option<LanguageFeature>;
      
      /// Check if a protocol allows negation in conformance lists.
      pub fn allows_negation(&self, protocol_id: SymbolId) -> bool;
  }
  ```

- [ ] Integrate into `SemanticModel`:
  ```rust
  impl SemanticModel {
      pub fn builtin_registry(&self) -> &BuiltinRegistry;
      
      /// Convenience: get the Copyable protocol.
      pub fn copyable_protocol(&self) -> Option<SymbolId> {
          self.builtin_registry().protocol_for(LanguageFeature::Copyable)
      }
  }
  ```

### 3.3 Builtin Attribute Processing

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [ ] Extend `AttributeResolver` to handle `@builtin`:
  - Parse `.Copyable` (or other feature variants) as argument
  - Validate protocol shape (marker protocols only for now)
  - Return `AttributeArgs::Builtin { feature }`

- [ ] Update `ProtocolBinder`:
  - After resolving attributes, check for `@builtin`
  - If present, validate:
    - Protocol must be a marker (no required methods)
    - Feature must not already be registered
  - Register in `BuiltinRegistry`

### 3.4 Standard Library Update

**Files**: `lang/std/core/protocols.ks`

- [ ] Add `Copyable` protocol:
  ```kestrel
  @builtin(.Copyable)
  public protocol Copyable {}
  ```

- [ ] Deprecate or remove `NonCopyable`:
  - Option A: Remove entirely (breaking change)
  - Option B: Keep as alias, emit deprecation warning

### 3.5 Diagnostics

- [ ] "@builtin requires a language feature argument"
- [ ] "unknown language feature `.{name}`"
- [ ] "@builtin can only be applied to protocols"
- [ ] "protocol `{name}` cannot be @builtin: must be a marker protocol"
- [ ] "language feature `{feature}` is already defined by protocol `{other}`"

### 3.6 Tests

**Files**: `lib/kestrel-test-suite/tests/builtins/`

- [ ] `builtin_protocol.rs`:
  - `@builtin(.Copyable)` on protocol
  - Query `copyable_protocol()` returns the right symbol
  - Error on non-marker protocol with @builtin
  - Error on duplicate @builtin for same feature

---

## Phase 4: Copyable / not Copyable

**Goal**: Types can opt-out of copy semantics with `not Copyable`.

### 4.1 Parser Changes - Negative Conformance

**Files**: `lib/kestrel-parser/src/`

- [ ] Parse `not Protocol` in conformance lists:
  ```kestrel
  struct FileHandle: not Copyable { ... }
  struct Connection: SomeProtocol, not Copyable { ... }
  ```
- [ ] Add syntax kinds:
  - `SyntaxKind::NegativeConformance`
- [ ] Modify conformance item parsing to accept optional `not` prefix

**Syntax**:
```
conformance_list := conformance (',' conformance)*
conformance := 'not'? type_path
```

### 4.2 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/`

- [ ] Extend `ConformancesBehavior` to track negative conformances:
  ```rust
  pub struct ConformancesBehavior {
      /// Positive conformances (protocols this type conforms to)
      conformances: Vec<Ty>,
      /// Negative conformances (protocols this type explicitly does NOT conform to)
      /// Only valid for language feature protocols that allow negation.
      negative_conformances: Vec<Ty>,
  }
  ```

- [ ] Add `CopySemantics` enum:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum CopySemantics {
      /// Type can be copied (bitwise copy, original remains valid)
      Copyable,
      /// Type cannot be copied, only moved (original becomes invalid)
      NotCopyable,
  }
  ```

- [ ] Add `CopySemanticsBehavior`:
  ```rust
  #[derive(Debug, Clone)]
  pub struct CopySemanticsBehavior {
      semantics: CopySemantics,
  }
  ```

- [ ] Add `Ty::is_copyable()` method:
  ```rust
  impl Ty {
      pub fn is_copyable(&self) -> bool {
          match self.kind() {
              // Primitives are copyable
              TyKind::Unit | TyKind::Never | TyKind::Bool | 
              TyKind::String | TyKind::Int(_) | TyKind::Float(_) => true,
              
              // Composites are copyable if all parts are
              TyKind::Tuple(elems) => elems.iter().all(|e| e.is_copyable()),
              TyKind::Array(elem) => elem.is_copyable(),
              TyKind::Function { .. } => true,
              
              // Structs/Enums check their CopySemanticsBehavior
              TyKind::Struct { symbol, .. } => {
                  symbol.metadata()
                      .get_behavior::<CopySemanticsBehavior>()
                      .map(|b| b.is_copyable())
                      .unwrap_or(true)
              }
              TyKind::Enum { symbol, .. } => { /* same */ }
              
              _ => true,
          }
      }
  }
  ```

### 4.3 Copy Semantics Computation

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [ ] Compute `CopySemantics` for structs and enums:
  1. If explicitly `not Copyable` → NotCopyable
  2. If any field is NotCopyable → NotCopyable (silent propagation)
  3. Otherwise → Copyable

- [ ] Handle cycles using Tarjan's algorithm or similar:
  - Build dependency graph of struct → field types
  - Find strongly connected components
  - A cycle is copyable if:
    - No member has explicit `not Copyable`
    - No field outside the cycle is not copyable

### 4.4 Conformance Validation

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [ ] Update `resolve_conformance_list()`:
  - Track negative conformances separately
  - Validate that negated protocol allows negation (via `BuiltinRegistry`)
  - Error if negating a non-builtin or non-negatable protocol

### 4.5 Move Tracking

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/`

- [ ] Add `MoveTracker`:
  ```rust
  pub struct MoveTracker {
      states: HashMap<LocalId, MoveState>,
  }
  
  #[derive(Clone, Debug)]
  pub enum MoveState {
      Valid,
      Moved { span: Span },
      MaybeMoved { spans: Vec<Span> },
  }
  ```

- [ ] Integrate into `BodyResolutionContext`

- [ ] Track moves on:
  - `consuming` parameter with non-copyable type
  - Assignment `let x = y` with non-copyable type
  - Return of non-copyable value

- [ ] Check on variable use:
  - Error if `Moved`
  - Error if `MaybeMoved`

- [ ] Handle control flow:
  - Fork tracker for if/else branches
  - Join trackers after (union of moved sets → MaybeMoved)

### 4.6 MIR Lowering

**Files**: `lib/kestrel-execution-graph-lowering/src/`

- [ ] Update `access_mode_to_passing_mode` to check copyability:
  ```rust
  fn access_mode_to_passing_mode(mode: ParameterAccessMode, ty: &Ty) -> PassingMode {
      match mode {
          ParameterAccessMode::Borrow => PassingMode::Ref,
          ParameterAccessMode::Mutating => PassingMode::MutRef,
          ParameterAccessMode::Consuming => {
              if ty.is_copyable() {
                  PassingMode::Copy
              } else {
                  PassingMode::Move
              }
          }
      }
  }
  ```

### 4.7 Diagnostics

- [ ] "cannot use `not` with protocol `{name}`: not a language feature protocol"
- [ ] "use of moved value `{name}`"
- [ ] "value `{name}` used here after move"
- [ ] "value moved here" (secondary span)
- [ ] "value may have been moved" (for MaybeMoved)

### 4.8 Tests

**Files**: `lib/kestrel-test-suite/tests/memory_model/`

- [ ] `copyable_inference.rs`:
  - Simple struct is Copyable
  - Struct with `not Copyable` is not Copyable
  - Struct with not-copyable field is not Copyable
  - Cyclic structs handled correctly
- [ ] `move_semantics.rs`:
  - Use after move error
  - Maybe moved error (conditional)
  - Copyable allows reuse after consuming
  - Assignment moves non-copyable
- [ ] `mir_copy_move.rs`:
  - Consuming copyable → PassingMode::Copy
  - Consuming not-copyable → PassingMode::Move

---

## Phase 5: Drop Semantics (RAII)

**Goal**: `deinit` blocks for deterministic resource cleanup.

### 5.1 Parser Changes

**Files**: `lib/kestrel-parser/src/struct/mod.rs`

- [ ] Parse `deinit { ... }` blocks in struct body:
  ```kestrel
  struct FileHandle: not Copyable {
      var fd: Int
      
      deinit {
          close(self.fd)
      }
  }
  ```

**Syntax**:
```
struct_body := '{' struct_member* '}'
struct_member := field | function | init | deinit
deinit := 'deinit' block
```

### 5.2 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/symbol/*.rs`

- [ ] Add `DeinitSymbol` (similar to `InitializerSymbol`)
- [ ] Add `DeinitBehavior` to struct symbols
- [ ] Validation:
  - At most one `deinit` per struct
  - `deinit` has access to `self` (read-only? or full access?)
  - Warn if `Copyable` type has `deinit`

### 5.3 Execution Graph Changes

**Files**: `lib/kestrel-execution-graph/src/*.rs`, `lib/kestrel-execution-graph-lowering/src/*.rs`

- [ ] Add `Drop` instruction:
  ```rust
  Drop { place: Place }
  ```
- [ ] Lower `deinit` as a special function `__deinit__` or similar
- [ ] Insert drop calls:
  - **Scope exit**: Drop locals in reverse declaration order
  - **Reassignment**: Drop old value before assigning new
  - **Temporaries**: Drop at end of statement
- [ ] Handle conditional drops:
  ```rust
  // For maybe-moved variables, emit:
  if drop_flag {
      Drop { place }
  }
  ```
- [ ] Do NOT drop moved values (drop at destination only)

### 5.4 Drop Intrinsic

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [ ] Add `drop(x)` as built-in intrinsic:
  - Immediately drops the value
  - Marks variable as `Moved`
  - Cannot be called on borrowed value
- [ ] Lower to `Drop` instruction + invalidate variable

### 5.5 Field Drop Order

- [ ] Struct fields dropped in reverse declaration order
- [ ] `deinit` body runs BEFORE fields are dropped
- [ ] `self` is fully valid in `deinit` body

### 5.6 Enum Drop

- [ ] Only drop the active variant's payload
- [ ] Requires runtime discrimination

### 5.7 Diagnostics

- [ ] "struct `{name}` already has a deinit"
- [ ] "deinit cannot return a value"
- [ ] Warning: "struct `{name}` is Copyable but has deinit - deinit will run for each copy"

### 5.8 Tests

- [ ] `deinit_basic.rs`:
  - deinit called at scope exit
  - deinit called in reverse order
- [ ] `deinit_moved.rs`:
  - Moved value not dropped at source
  - Dropped at destination
- [ ] `drop_intrinsic.rs`:
  - Explicit drop works
  - Use after drop is error

---

## Phase 6: Cloneable Protocol

**Goal**: Custom copy behavior via `clone()` method.

### 6.1 Prelude Definition

**Files**: `lang/std/core/protocols.ks`

```kestrel
protocol Cloneable: Copyable {
    func clone(self) -> Self
}
```

### 6.2 Semantic Model Changes

- [ ] Detect `Cloneable` conformance on types
- [ ] For `Cloneable` types, copy semantics change:
  - Instead of bitwise copy, call `clone()`
- [ ] Track whether a type is:
  - Simple `Copyable` (bitwise copy)
  - `Cloneable` (clone() copy)
  - `NotCopyable` (no copy, only move)

### 6.3 Execution Graph Changes

- [ ] Add `Clone` instruction or emit as method call:
  ```rust
  // Option 1: Explicit instruction
  Clone { dest: Place, src: Operand }
  
  // Option 2: Lower to method call
  Call { dest, callee: clone_method, args: [(src, Ref)] }
  ```
- [ ] When copying a `Cloneable` type:
  - Emit `Clone` instead of `Copy`

### 6.4 Compiler-Derived Cloneable

- [ ] If struct explicitly declares `: Cloneable`:
  - All fields must be `Cloneable` or simple `Copyable`
  - Compiler synthesizes `clone()` that clones each field
- [ ] If any field is `Cloneable`, struct must be `Cloneable` (not simple `Copyable`)

### 6.5 Tests

- [ ] `cloneable_basic.rs`:
  - Custom clone() called on copy
  - Derived clone() works
- [ ] `cloneable_validation.rs`:
  - Cannot be both Cloneable and not Copyable
  - All fields must be cloneable

---

## Phase 7: Generics Integration

**Goal**: `[T: not Copyable]` syntax for generic bounds.

### 7.1 Parser Changes

**Files**: `lib/kestrel-parser/src/common/parsers.rs` (generic bounds)

- [ ] Parse `not Copyable` in generic bounds:
  ```kestrel
  struct List[T: not Copyable] { ... }
  func wrap[T: not Copyable](consuming item: T) -> Box[T]
  ```
- [ ] `[T]` = implicit `Copyable` bound
- [ ] `[T: Copyable]` = explicit `Copyable` bound
- [ ] `[T: not Copyable]` = no copyability requirement

### 7.2 Semantic Model Changes

- [ ] Add `CopyabilityBound` to generic parameters:
  ```rust
  pub enum CopyabilityBound {
      Copyable,     // T: Copyable (default)
      NoCopyBound,  // T: not Copyable (relaxed)
  }
  ```
- [ ] Validate operations on type parameters:
  - In `[T]` context: can copy T values
  - In `[T: not Copyable]` context: cannot copy T values

### 7.3 Conditional Conformance

- [ ] `Box[T]` is `Copyable` when `T: Copyable`
- [ ] Requires tracking conditional bounds on generic types
- [ ] Query: "is `Box[Int]` Copyable?" -> Yes (Int is Copyable)
- [ ] Query: "is `Box[FileHandle]` Copyable?" -> No (FileHandle is not Copyable)

### 7.4 Validation

- [ ] Error: "cannot copy value of type `T`; `T` may not be `Copyable`"
- [ ] Error: "type `FileHandle` does not satisfy bound `Copyable` required by `duplicate[T]`"

### 7.5 Tests

- [ ] `generic_copyability.rs`:
  - Default bound allows copy
  - not Copyable prevents copy
  - Calling with non-copyable type errors if bound requires Copyable
- [ ] `conditional_conformance.rs`:
  - Box[Int] is Copyable
  - Box[FileHandle] is not Copyable

---

## Phase 8: Law of Exclusivity

**Goal**: Prevent simultaneous conflicting accesses.

### 8.1 Borrow Tracking

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/*.rs`

- [ ] Track active borrows during expression evaluation:
  ```rust
  struct BorrowSet {
      borrows: Vec<ActiveBorrow>,
  }
  
  struct ActiveBorrow {
      place: Place,      // What's borrowed
      kind: BorrowKind,  // Shared or Mutable
      span: Span,        // Where borrow started
  }
  ```
- [ ] Borrow lifetime:
  - Starts when passed to function
  - Ends when function returns (for non-escaping)

### 8.2 Conflict Detection

- [ ] Before creating a new borrow, check for conflicts:
  - Mutable borrow conflicts with ANY existing borrow of same place
  - Shared borrow conflicts with existing mutable borrow
- [ ] "Overlapping access" includes:
  - Same variable
  - Field of borrowed struct
  - Element of borrowed array

### 8.3 Closure Captures

- [ ] Closure that captures mutable reference:
  - While closure exists, no other mutable access
- [ ] Non-escaping closures:
  - Borrow ends when closure scope ends
  - Can validate statically

### 8.4 Diagnostics

- [ ] "cannot borrow `x` as mutable because it is already borrowed as immutable"
- [ ] "cannot use `x` while mutable borrow is active"
- [ ] "mutable borrow of `x` occurs here" (secondary span)

### 8.5 Tests

- [ ] `exclusivity_basic.rs`:
  - Two shared borrows OK
  - Mutable + shared conflict
  - Mutable + mutable conflict
- [ ] `exclusivity_fields.rs`:
  - Borrowing field conflicts with borrowing struct
- [ ] `exclusivity_closures.rs`:
  - Closure capture creates borrow

---

## Implementation Order

Recommended order of implementation:

1. **Phase 1** - Foundation, required by everything else ✅ COMPLETE
2. **Phase 2** - Attributes: foundation for builtin system
3. **Phase 3** - Builtin protocols: defines `@builtin(.Copyable)`
4. **Phase 4** - Copyable/not Copyable: core value proposition
5. **Phase 5** - Drop semantics: RAII is critical per requirements
6. **Phase 7** - Generics before Cloneable (standard library needs this)
7. **Phase 6** - Cloneable builds on Copyable infrastructure
8. **Phase 8** - Can be done in parallel with later phases

---

## Design Decisions

### Temporaries Drop at End of Statement

```kestrel
process(FileHandle("a.txt"))  // Dropped after this line
print("file closed")          // File is already closed
```

### Destructuring Moves Allowed

```kestrel
struct Pair { var first: Resource, var second: Resource }

let Pair(a, b) = pair  // Moves both fields out, pair is now invalid
```

### `deinit` + `Copyable` Allowed with Warning

```kestrel
struct Counter: Copyable {  // Warning but allowed
    var count: Int
    deinit { print("dropped") }
}

let a = Counter(count: 1)
let b = a  // Copy - both have deinit
// Both a.deinit and b.deinit will be called
```

### Panic Behavior

Start with abort-on-panic. Unwinding and drop during panic is complex and can be added later.

### Attribute Syntax

Attributes use `@name` or `@name(args)` syntax, similar to Rust/Swift. Arguments are expressions, allowing for flexible attribute definitions. For builtin features, we use enum-like syntax: `@builtin(.Copyable)`.

### Language Feature Protocols

Protocols marked with `@builtin(.Feature)` are special:
- They have compiler-known semantics
- They may have implicit conformance (like Copyable)
- They may allow `not` negation in conformance lists
- They must be marker protocols (no required methods)
