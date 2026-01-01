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

## Phase 2: Attributes ✅ COMPLETE

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

- [x] Add `At` token to lexer (the `@` symbol)
- [x] Add syntax kinds:
  - `SyntaxKind::Attribute`
  - `SyntaxKind::AttributeList`
  - `SyntaxKind::AttributeArgs`
  - `SyntaxKind::AttributeArg`
- [x] Create attribute parser:
  - Parse `@identifier` 
  - Parse optional `(expr, expr, ...)` argument list
- [x] Integrate attribute parsing before declarations:
  - Protocol declarations
  - Struct declarations
  - Enum declarations
  - Function declarations
  - Field declarations
  - Initializer declarations
  - Enum case declarations

**Files modified**:
- `lib/kestrel-parser/src/common/data.rs` - Added `AttributeData`, `AttributeArgsData`, `AttributeArgData`
- `lib/kestrel-parser/src/common/emitters.rs` - Added attribute emitters
- `lib/kestrel-parser/src/attribute/mod.rs` - New module for attribute parsing
- `lib/kestrel-parser/src/protocol/mod.rs` - Accept attributes before protocol
- `lib/kestrel-parser/src/struct/mod.rs` - Accept attributes before struct
- `lib/kestrel-parser/src/common/parsers.rs` - Accept attributes before function, field, initializer
- `lib/kestrel-parser/src/enum_decl/mod.rs` - Accept attributes before enum and enum case

### 2.3 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/`

- [x] Create `AttributeKind` enum for known attributes:
  ```rust
  pub enum AttributeKind {
      Dummy,    // @dummy - placeholder for testing
      Unknown,  // Unrecognized attribute
  }
  ```

- [x] Create `Attribute` struct with name, kind, args, and span

- [x] Create `AttributesBehavior`:
  ```rust
  pub struct AttributesBehavior {
      attributes: Vec<Attribute>,
  }
  
  impl AttributesBehavior {
      pub fn has(&self, name: &str) -> bool { ... }
      pub fn get(&self, name: &str) -> Option<&Attribute> { ... }
      pub fn attributes(&self) -> &[Attribute] { ... }
  }
  ```

### 2.4 Attribute Resolution

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [x] Create attribute resolver (`binders/utils/attributes.rs`):
  - `resolve_attributes()` extracts attributes from syntax
  - Emits warnings for unknown attributes

- [x] Integrate into all 7 binders:
  - `ProtocolBinder` - resolve attributes, add `AttributesBehavior`
  - `StructBinder` - resolve attributes, add `AttributesBehavior`
  - `EnumBinder` - resolve attributes, add `AttributesBehavior`
  - `FunctionBinder` - resolve attributes, add `AttributesBehavior`
  - `FieldBinder` - resolve attributes, add `AttributesBehavior`
  - `InitializerBinder` - resolve attributes, add `AttributesBehavior`
  - `EnumCaseBinder` - resolve attributes, add `AttributesBehavior`

### 2.5 Diagnostics

- [x] "unknown attribute `{name}`" (warning)
- [ ] "attribute `{name}` does not take arguments" (deferred to Phase 3)
- [ ] "attribute `{name}` requires arguments" (deferred to Phase 3)
- [ ] "invalid argument for attribute `{name}`: expected {expected}" (deferred to Phase 3)
- [ ] "duplicate attribute `{name}`" (deferred to Phase 3)

Note: Argument validation and duplicate detection are deferred until specific attributes with requirements are defined in Phase 3.

### 2.6 Tests

**Files**: `lib/kestrel-test-suite/tests/attributes/`

- [x] `parsing.rs` - 109 tests covering:
  - Simple attributes on all declaration types
  - Attributes with arguments (string, int, float, bool, path, implicit member)
  - Multiple attributes on same declaration
  - Labeled and unlabeled arguments
  - Empty parentheses
  - Nested declarations with attributes
- [x] `semantic.rs` - Semantic binding tests:
  - Attribute behavior attachment to all symbol types
  - Attribute count verification
  - Argument count verification
  - Unknown attribute warnings
- [x] `declarations.rs` - Declaration-specific tests:
  - All 7 declaration types with attributes and various modifiers

---

## Phase 3: Builtin Protocols ✅ COMPLETE

**Goal**: Define the `@builtin` attribute and language feature protocol system.

### 3.1 Language Feature Enum

**Files**: `lib/kestrel-semantic-tree/src/builtins.rs`

- [x] Create `LanguageFeature` enum:
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

- [x] Create `BuiltinKind` enum and `BuiltinDefinition` struct:
  ```rust
  /// What kind of symbol a builtin expects, with kind-specific configuration.
  pub enum BuiltinKind {
      Protocol { implicit_conformance: bool, must_be_marker: bool },
      Struct,
      Enum,
      Function,
      Variable,
  }
  
  pub struct BuiltinDefinition {
      pub feature: LanguageFeature,
      pub kind: BuiltinKind,
  }
  
  impl LanguageFeature {
      pub fn definition(&self) -> BuiltinDefinition { ... }
  }
  ```

### 3.2 Builtin Registry

**Files**: `lib/kestrel-semantic-tree/src/builtins.rs`, `lib/kestrel-semantic-model/src/model.rs`

- [x] Create `BuiltinRegistry` with separate maps per symbol kind:
  ```rust
  /// Registry for builtin language features.
  /// Maintains separate maps for each symbol kind.
  pub struct BuiltinRegistry {
      protocols: RwLock<HashMap<LanguageFeature, SymbolId>>,
      protocol_features: RwLock<HashMap<SymbolId, LanguageFeature>>,
      structs: RwLock<HashMap<LanguageFeature, SymbolId>>,
      struct_features: RwLock<HashMap<SymbolId, LanguageFeature>>,
      enums: RwLock<HashMap<LanguageFeature, SymbolId>>,
      enum_features: RwLock<HashMap<SymbolId, LanguageFeature>>,
      functions: RwLock<HashMap<LanguageFeature, SymbolId>>,
      function_features: RwLock<HashMap<SymbolId, LanguageFeature>>,
      variables: RwLock<HashMap<LanguageFeature, SymbolId>>,
      variable_features: RwLock<HashMap<SymbolId, LanguageFeature>>,
  }
  
  impl BuiltinRegistry {
      pub fn register_protocol(&self, feature: LanguageFeature, id: SymbolId) -> bool;
      pub fn register_struct(&self, feature: LanguageFeature, id: SymbolId) -> bool;
      pub fn register_enum(&self, feature: LanguageFeature, id: SymbolId) -> bool;
      pub fn register_function(&self, feature: LanguageFeature, id: SymbolId) -> bool;
      pub fn register_variable(&self, feature: LanguageFeature, id: SymbolId) -> bool;
      pub fn copyable_protocol(&self) -> Option<SymbolId>;
      // ... other accessors
  }
  ```

- [x] Integrate into `SemanticModel`:
  ```rust
  impl SemanticModel {
      pub fn builtin_registry(&self) -> &Arc<BuiltinRegistry>;
  }
  ```

### 3.3 Builtin Attribute Processing

**Files**: `lib/kestrel-semantic-tree-binder/src/binders/`

- [x] Add `AttributeKind::Builtin` to recognized attributes

- [x] Create `parse_builtin_attribute()` helper in `binders/utils/attributes.rs`:
  - Validates attribute has arguments
  - Validates argument is implicit member syntax (`.Feature`)
  - Parses feature name and returns `LanguageFeature`
  - Emits appropriate errors for invalid formats

- [x] Update `ProtocolBinder`:
  - After resolving attributes, check for `@builtin`
  - If present, validate:
    - Feature expects a protocol
    - If `must_be_marker`, protocol has no functions/associated types
    - Feature not already registered
  - Register in `BuiltinRegistry`

- [x] Update `StructBinder`, `EnumBinder`, `FunctionBinder`:
  - Check for `@builtin` attribute
  - Validate feature expects the correct symbol kind
  - Register in appropriate registry map

### 3.4 Standard Library Update

**Files**: `lang/std/core/protocols.ks`

- [x] Add `Copyable` protocol:
  ```kestrel
  @builtin(.Copyable)
  public protocol Copyable {}
  ```

- [x] Remove `NonCopyable` protocol (replaced by `not Copyable` syntax in Phase 4)

### 3.5 Diagnostics

**Files**: `lib/kestrel-semantic-tree-binder/src/diagnostics/builtins.rs`

- [x] `BuiltinRequiresArgumentError` - "@builtin requires a language feature argument"
- [x] `BuiltinInvalidArgumentError` - "@builtin expected implicit member syntax (.Feature)"
- [x] `UnknownLanguageFeatureError` - "unknown language feature `.{name}`"
- [x] `BuiltinWrongKindError` - "@builtin(.{feature}) can only be applied to a {expected_kind}"
- [x] `BuiltinMustBeMarkerError` - "@builtin(.{feature}) must be a marker protocol"
- [x] `DuplicateBuiltinError` - "language feature `.{feature}` is already defined by another symbol"

### 3.6 Tests

**Files**: `lib/kestrel-test-suite/tests/builtins/`

- [x] `protocols.rs` - 17 tests covering:
  - Success: `@builtin(.Copyable)` on marker protocol
  - Success: `@builtin(.ExpressibleByIntLiteral)` on non-marker protocol
  - Success: Multiple builtin protocols in same module
  - Success: Builtin with public visibility
  - Error: `@builtin` without argument
  - Error: `@builtin()` with empty parens
  - Error: Unknown language feature
  - Error: Misspelled language feature
  - Error: Builtin protocol on struct
  - Error: Builtin protocol on enum
  - Error: Builtin protocol on function
  - Error: Copyable on protocol with method (non-marker)
  - Error: Copyable on protocol with associated type (non-marker)
  - Error: Duplicate Copyable builtin
  - Error: String argument instead of implicit member
  - Error: Integer argument instead of implicit member
  - Error: Labeled argument

---

## Phase 4: Copyable / not Copyable ✅ COMPLETE

**Goal**: Types can opt-out of copy semantics with `not Copyable`.

### 4.1 Parser Changes - Negative Conformance ✅ COMPLETE

**Files**: `lib/kestrel-parser/src/`

- [x] Parse `not Protocol` in conformance lists:
  ```kestrel
  struct FileHandle: not Copyable { ... }
  struct Connection: SomeProtocol, not Copyable { ... }
  ```
- [x] Add syntax kinds:
  - `SyntaxKind::NegativeConformance`
- [x] Modify conformance item parsing to accept optional `not` prefix

**Syntax**:
```
conformance_list := conformance (',' conformance)*
conformance := 'not'? type_path
```

### 4.2 Semantic Model Changes ✅ COMPLETE

**Files**: `lib/kestrel-semantic-tree/src/`

- [x] Extend `ConformancesBehavior` to track negative conformances
- [x] Add `CopySemanticsBehavior` with `is_copyable()` method
- [x] Add `Ty::is_copyable()` method that checks:
  - Primitives are copyable
  - Composites are copyable if all parts are
  - Structs/Enums check their `CopySemanticsBehavior`

### 4.3 Copy Semantics Computation ✅ COMPLETE

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [x] Compute `CopySemantics` for structs and enums:
  1. If explicitly `not Copyable` → NotCopyable
  2. If any field is NotCopyable → NotCopyable (silent propagation)
  3. Otherwise → Copyable

### 4.4 Conformance Validation ✅ COMPLETE

**Files**: `lib/kestrel-semantic-tree-binder/src/`

- [x] Update `resolve_conformance_list()`:
  - Track negative conformances separately
  - Validate that negated protocol allows negation (via `BuiltinRegistry`)
  - Error if negating a non-builtin or non-negatable protocol

### 4.5 Move Tracking ✅ COMPLETE

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/`

- [x] Add `MoveTracker` with `MoveState` enum (`Valid`, `Moved`, `MaybeMoved`)
- [x] Integrate into `BodyResolutionContext`
- [x] Track moves on:
  - `consuming` parameter with non-copyable type
  - Return of non-copyable value
- [x] Check on variable use:
  - Error if `Moved`
  - Error if `MaybeMoved`
- [x] Handle control flow:
  - Fork tracker for if/else branches
  - Join trackers after (union of moved sets → MaybeMoved)

### 4.6 MIR Lowering ✅ COMPLETE

**Files**: `lib/kestrel-execution-graph-lowering/src/`

- [x] Update `access_mode_to_passing_mode` to check copyability:
  - `Consuming` + copyable → `PassingMode::Copy`
  - `Consuming` + not copyable → `PassingMode::Move`

### 4.7 Diagnostics ✅ COMPLETE

- [x] "cannot use `not` with protocol `{name}`: not a language feature protocol"
- [x] "use of moved value `{name}`"
- [x] "value `{name}` used here after move"
- [x] "value moved here" (secondary span)
- [x] "value may have been moved" (for MaybeMoved)

### 4.8 Tests ✅ COMPLETE

**Files**: `lib/kestrel-test-suite/tests/memory_model/`

- [x] `copy_semantics.rs`:
  - Struct copy semantics (copyable by default, not Copyable, with protocol)
  - Enum copy semantics (copyable by default, not Copyable)
  - Field propagation (non-copyable field makes struct non-copyable)
  - Loop move tests (while loop maybe moved, infinite loop definitely moved)
  - Maybe moved tests (if/else branches)
  - MIR tests (Copy vs Move emission)
  - Use after move tests
- [x] `negative_conformance.rs`:
  - Parsing tests
  - Semantic tests
  - Validation error tests

---

## Phase 5: Drop Semantics (RAII)

**Goal**: `deinit` blocks for deterministic resource cleanup.

### 5.1 Parser Changes ✅ COMPLETE

**Files**: `lib/kestrel-parser/src/struct/mod.rs`, `lib/kestrel-parser/src/block/mod.rs`

- [x] Parse `deinit { ... }` blocks in struct body:
  ```kestrel
  struct FileHandle: not Copyable {
      var fd: Int
      
      deinit {
          close(self.fd)
      }
  }
  ```
- [x] Parse `deinit x;` statement in function bodies

**Syntax**:
```
struct_body := '{' struct_member* '}'
struct_member := field | function | init | deinit
deinit := 'deinit' block

statement := ... | deinit_statement
deinit_statement := 'deinit' identifier ';'
```

### 5.2 Semantic Model Changes ✅ COMPLETE

**Files**: `lib/kestrel-semantic-tree/src/symbol/*.rs`

- [x] Add `DeinitSymbol` (similar to `InitializerSymbol`)
- [x] Add `DeinitBehavior` to struct symbols
- [x] Validation:
  - [x] At most one `deinit` per struct
  - [x] `deinit` has access to `self` (read-only)
  - [x] Warn if `Copyable` type has `deinit`

### 5.3 Execution Graph Changes ✅ COMPLETE

**Files**: `lib/kestrel-execution-graph/src/*.rs`, `lib/kestrel-execution-graph-lowering/src/*.rs`

- [x] Add `Deinit` instruction (named to match Kestrel's `deinit` keyword):
  ```rust
  Deinit { place: Place }           // Unconditional deinit
  DeinitIf { place: Place, flag: Id<Local> }  // Conditional deinit
  SetDeinitFlag { flag: Id<Local>, value: bool }  // Set a deinit flag
  ```
- [x] Insert deinit calls:
  - **Scope exit**: Deinit locals in reverse declaration order
  - **Return**: Deinit all in-scope locals before return
  - **Break/Continue**: Deinit locals between current scope and target loop
  - **Temporaries**: Deinit at end of statement
- [x] Handle conditional deinits (branch merging):
  - Track `DeinitStatus`: `Valid`, `Moved`, `MaybeMoved { flag }`
  - When variable moved in one if-branch but not other:
    - Create deinit flag
    - Emit `SetDeinitFlag(flag, false)` in moving branch
    - Emit `SetDeinitFlag(flag, true)` in non-moving branch
    - Emit `DeinitIf { place, flag }` at scope exit
- [x] Do NOT deinit moved values:
  - Track moves via `mark_moved()` when passing with `PassingMode::Move`
  - Check deinit status before emitting deinit at scope exit
- [x] Temporary tracking:
  - Track temps created during expression evaluation
  - Deinit temps at statement end if not consumed

### 5.4 Deinit Statement ✅ COMPLETE

**Files**: `lib/kestrel-parser/src/block/mod.rs`, `lib/kestrel-semantic-tree-binder/src/body_resolver/`

- [x] Add `deinit x;` statement syntax (reuses `deinit` keyword)
- [x] Marks variable as `Moved` after deinit
- [x] Cannot deinit already-moved value (use-after-move error)
- [x] Lowered to `Deinit` statement in execution graph

### 5.5 Field Drop Order

- [ ] Struct fields dropped in reverse declaration order
- [ ] `deinit` body runs BEFORE fields are dropped
- [ ] `self` is fully valid in `deinit` body

### 5.6 Enum Drop

- [ ] Only drop the active variant's payload
- [ ] Requires runtime discrimination

### 5.7 Diagnostics ✅ COMPLETE

- [x] "struct `{name}` already has a deinit"
- [ ] "deinit cannot return a value"
- [x] Warning: "struct `{name}` is Copyable but has deinit - deinit will run for each copy"

### 5.8 Tests ✅ COMPLETE

- [x] `deinit.rs` in memory_model tests:
  - [x] Basic deinit parsing and binding
  - [x] DeinitBehavior attachment
  - [x] Duplicate deinit error
  - [x] Copyable + deinit warning
  - [x] Deinit with protocol conformance
  - [x] `deinit x;` statement compiles
  - [x] Use-after-deinit is error
  - [x] Double deinit is error
  - [x] Deinit on copyable type allowed
- [x] Automatic deinit MIR tests:
  - [x] `basic_scope_exit_deinit` - Non-copyable local gets Deinit at scope exit
  - [x] `deinit_in_reverse_order` - Multiple locals deinited in reverse order
  - [x] `explicit_deinit_emits_mir_statement` - `deinit x;` emits MIR Deinit
  - [x] `return_emits_deinits` - Return emits deinits for in-scope locals
  - [x] `break_emits_deinits` - Break emits deinits for loop-scoped locals
  - [x] `if_branch_deinits` - Each branch deinits its own locals
  - [x] `moved_value_not_double_deinited` - Moved value not deinited at scope exit
  - [x] `conditional_move_uses_deinit_if` - Uses DeinitIf for conditional moves
  - [x] `conditional_move_sets_flags` - SetDeinitFlag statements emitted
  - [x] `both_branches_move_no_conditional_deinit` - Both branches move → no conditional
  - [x] `neither_branch_moves_uses_regular_deinit` - Neither moves → regular Deinit
  - [x] `temporary_in_nested_call_deinited` - Temps passed by ref get deinited
  - [x] `temporary_consumed_not_deinited` - Consumed temps not deinited

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
2. **Phase 2** - Attributes: foundation for builtin system ✅ COMPLETE
3. **Phase 3** - Builtin protocols: defines `@builtin(.Copyable)` ✅ COMPLETE
4. **Phase 4** - Copyable/not Copyable: core value proposition ✅ COMPLETE
5. **Phase 5** - Drop semantics: RAII is critical per requirements ✅ MOSTLY COMPLETE (5.5, 5.6 remaining)
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
