# Memory Model Implementation Plan

This document details the implementation plan for Kestrel's memory model as described in the memory-model documentation.

## Overview

The memory model implementation is divided into 6 phases, designed to build incrementally with each phase providing value on its own.

| Phase | Feature | Description |
|-------|---------|-------------|
| 1 | Parameter Access Modes | `borrow`/`mutating`/`consuming` for parameters + MIR foundation |
| 2 | Copyable / not Copyable | Move semantics for non-copyable types |
| 3 | Drop Semantics (RAII) | `deinit` blocks and automatic cleanup |
| 4 | Cloneable Protocol | Custom copy behavior via `clone()` |
| 5 | Generics Integration | `[T: not Copyable]` bounds |
| 6 | Law of Exclusivity | Borrow checking and conflict detection |

---

## Phase 1: Parameter Access Modes + MIR Foundation

**Goal**: Parameters can have explicit access modes, MIR reflects passing semantics.

### 1.1 Parser Changes

**Files**: `lib/kestrel-lexer/src/lib.rs`, `lib/kestrel-parser/src/common/*.rs`

- [ ] Add `borrow` keyword to lexer (for explicit use if desired, though default)
- [ ] Extend parameter parsing to accept access mode prefix:
  ```kestrel
  func process(consuming p: Point, mutating q: Point, r: Point)
  ```
- [ ] `ReceiverModifier` enum already exists - extend or create `ParameterModifier`

**Syntax**:
```
parameter := (access_mode)? (label)? name ':' type
access_mode := 'borrow' | 'mutating' | 'consuming'
```

### 1.2 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/symbol/parameter.rs`, `lib/kestrel-semantic-tree/src/behavior/callable.rs`

- [ ] Add `AccessMode` enum:
  ```rust
  pub enum AccessMode {
      Borrow,    // Default - immutable reference
      Mutating,  // Mutable reference
      Consuming, // Takes ownership
  }
  ```
- [ ] Add `access_mode: AccessMode` to `ParameterSymbol`
- [ ] Extend `CallableBehavior` to include parameter access modes
- [ ] Update `ParameterBuilder` to extract access mode from syntax

### 1.3 Call-Site Validation

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [ ] Add `VariableState` tracking:
  ```rust
  pub enum VariableState {
      Valid,       // Can be used
      MaybeMoved,  // Moved in some branches
      Moved,       // Definitely moved
  }
  ```
- [ ] Track variable states in `BodyResolver` context
- [ ] Validate `mutating` parameters:
  - Argument must be a mutable place (`var` binding or mutable field)
  - Error: "cannot pass `let` binding to `mutating` parameter"
- [ ] Validate `consuming` parameters:
  - Mark source variable as `Moved` (for now, full tracking in Phase 2)
- [ ] Store `AccessMode` on call expressions for later MIR lowering

### 1.4 Execution Graph Changes

**Files**: `lib/kestrel-execution-graph/src/*.rs`, `lib/kestrel-execution-graph-lowering/src/lowerer/*.rs`

- [ ] Add `PassingMode` enum to MIR:
  ```rust
  pub enum PassingMode {
      Ref,     // Borrow - immutable reference
      MutRef,  // Mutating - mutable reference
      Copy,    // Value copied, original retained (Copyable + consuming)
      Move,    // Value moved, original invalidated (not Copyable + consuming)
  }
  ```
- [ ] Update `Call` instruction:
  ```rust
  Call {
      dest: Option<Place>,
      callee: Operand,
      args: Vec<(Operand, PassingMode)>,
  }
  ```
- [ ] Update function lowering to emit correct passing modes:
  - `borrow` → `Ref`
  - `mutating` → `MutRef`
  - `consuming` → `Copy` or `Move` (depends on type, default to `Copy` for Phase 1)

### 1.5 Diagnostics

**Files**: `lib/kestrel-semantic-tree-binder/src/diagnostics/*.rs`

- [ ] "cannot pass `let` binding `{name}` to `mutating` parameter"
- [ ] "cannot pass immutable field `{name}` to `mutating` parameter"
- [ ] "use of moved value `{name}`" (basic version)
- [ ] "value `{name}` moved here" (secondary span)

### 1.6 Tests

**Files**: `lib/kestrel-test-suite/tests/memory_model/*.rs` (new directory)

- [ ] `parameter_access_modes.rs`:
  - Borrow parameter compiles
  - Mutating parameter requires var
  - Consuming parameter compiles
  - Error: let to mutating
- [ ] `mir_passing_modes.rs`:
  - Verify MIR emits correct PassingMode for each access mode

---

## Phase 2: Copyable / not Copyable

**Goal**: Types can opt-out of copy semantics with `not Copyable`.

### 2.1 Parser Changes

**Files**: `lib/kestrel-lexer/src/lib.rs`, `lib/kestrel-parser/src/struct/mod.rs`

- [ ] Parse `not Copyable` in struct conformance list:
  ```kestrel
  struct FileHandle: not Copyable { ... }
  ```
- [ ] This is similar to protocol conformance but with negation

**Syntax**:
```
struct_declaration := 'struct' name type_params? (':' conformance_list)? struct_body
conformance_list := conformance (',' conformance)*
conformance := 'not'? type_path
```

### 2.2 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/symbol/struct.rs`, `lib/kestrel-semantic-tree/src/behavior/*.rs`

- [ ] Add `CopySemantics` enum:
  ```rust
  pub enum CopySemantics {
      Copyable,     // Can be copied (default for all-copyable fields)
      NotCopyable,  // Must be moved (explicit or has non-copyable field)
  }
  ```
- [ ] Add `CopySemanticssBehavior` to struct symbols
- [ ] Inference rules:
  - Primitive types (`Int`, `Bool`, etc.) are `Copyable`
  - Struct with explicit `not Copyable` → `NotCopyable`
  - Struct with any `NotCopyable` field → `NotCopyable`
  - Otherwise → `Copyable`

### 2.3 Move Tracking

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/*.rs`

- [ ] Extend `VariableState` tracking from Phase 1
- [ ] On `consuming` parameter pass:
  - If type is `Copyable`: variable remains `Valid`
  - If type is `NotCopyable`: variable becomes `Moved`
- [ ] On assignment `let x = y`:
  - If type is `Copyable`: y remains `Valid`
  - If type is `NotCopyable`: y becomes `Moved`
- [ ] Error on use of `Moved` variable
- [ ] Track `MaybeMoved` for conditionals:
  ```kestrel
  if condition {
      consume(x)  // x moved here
  }
  print(x)  // Error: x may have been moved
  ```

### 2.4 Execution Graph Changes

**Files**: `lib/kestrel-execution-graph-lowering/src/lowerer/*.rs`

- [ ] When lowering `consuming` parameter:
  - Check type's `CopySemantics`
  - Emit `Copy` for `Copyable`, `Move` for `NotCopyable`
- [ ] Add explicit `Copy` and `Move` instructions if needed:
  ```rust
  Copy { dest: Place, src: Operand },  // Bitwise copy
  Move { dest: Place, src: Operand },  // Transfer ownership
  ```

### 2.5 Diagnostics

- [ ] "type `{name}` is not copyable and cannot be used after move"
- [ ] "cannot copy value of type `{name}`; type is `not Copyable`"
- [ ] "value `{name}` used here after move"
- [ ] "adding field `{field}` of type `{type}` makes `{struct}` not copyable" (warning)

### 2.6 Tests

- [ ] `copyable_inference.rs`:
  - Simple struct is Copyable
  - Struct with not Copyable field is not Copyable
  - Explicit not Copyable works
- [ ] `move_semantics.rs`:
  - Use after move error
  - Maybe moved error
  - Copyable allows reuse after consuming

---

## Phase 3: Drop Semantics (RAII)

**Goal**: `deinit` blocks for deterministic resource cleanup.

### 3.1 Parser Changes

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

### 3.2 Semantic Model Changes

**Files**: `lib/kestrel-semantic-tree/src/symbol/*.rs`

- [ ] Add `DeinitSymbol` (similar to `InitializerSymbol`)
- [ ] Add `DeinitBehavior` to struct symbols
- [ ] Validation:
  - At most one `deinit` per struct
  - `deinit` has access to `self` (read-only? or full access?)
  - Warn if `Copyable` type has `deinit`

### 3.3 Execution Graph Changes

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

### 3.4 Drop Intrinsic

**Files**: `lib/kestrel-semantic-tree-binder/src/body_resolver/calls.rs`

- [ ] Add `drop(x)` as built-in intrinsic:
  - Immediately drops the value
  - Marks variable as `Moved`
  - Cannot be called on borrowed value
- [ ] Lower to `Drop` instruction + invalidate variable

### 3.5 Field Drop Order

- [ ] Struct fields dropped in reverse declaration order
- [ ] `deinit` body runs BEFORE fields are dropped
- [ ] `self` is fully valid in `deinit` body

### 3.6 Enum Drop

- [ ] Only drop the active variant's payload
- [ ] Requires runtime discrimination

### 3.7 Diagnostics

- [ ] "struct `{name}` already has a deinit"
- [ ] "deinit cannot return a value"
- [ ] Warning: "struct `{name}` is Copyable but has deinit - deinit will run for each copy"

### 3.8 Tests

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

## Phase 4: Cloneable Protocol

**Goal**: Custom copy behavior via `clone()` method.

### 4.1 Prelude Definition

**Files**: `lang/std/core/cloneable.ks` (new or existing)

```kestrel
protocol Cloneable: Copyable {
    func clone(self) -> Self
}
```

### 4.2 Semantic Model Changes

- [ ] Detect `Cloneable` conformance on types
- [ ] For `Cloneable` types, copy semantics change:
  - Instead of bitwise copy, call `clone()`
- [ ] Track whether a type is:
  - Simple `Copyable` (bitwise copy)
  - `Cloneable` (clone() copy)
  - `NotCopyable` (no copy, only move)

### 4.3 Execution Graph Changes

- [ ] Add `Clone` instruction or emit as method call:
  ```rust
  // Option 1: Explicit instruction
  Clone { dest: Place, src: Operand }
  
  // Option 2: Lower to method call
  Call { dest, callee: clone_method, args: [(src, Ref)] }
  ```
- [ ] When copying a `Cloneable` type:
  - Emit `Clone` instead of `Copy`

### 4.4 Compiler-Derived Cloneable

- [ ] If struct explicitly declares `: Cloneable`:
  - All fields must be `Cloneable` or simple `Copyable`
  - Compiler synthesizes `clone()` that clones each field
- [ ] If any field is `Cloneable`, struct must be `Cloneable` (not simple `Copyable`)

### 4.5 Tests

- [ ] `cloneable_basic.rs`:
  - Custom clone() called on copy
  - Derived clone() works
- [ ] `cloneable_validation.rs`:
  - Cannot be both Cloneable and not Copyable
  - All fields must be cloneable

---

## Phase 5: Generics Integration

**Goal**: `[T: not Copyable]` syntax for generic bounds.

### 5.1 Parser Changes

**Files**: `lib/kestrel-parser/src/common/parsers.rs` (generic bounds)

- [ ] Parse `not Copyable` in generic bounds:
  ```kestrel
  struct List[T: not Copyable] { ... }
  func wrap[T: not Copyable](consuming item: T) -> Box[T]
  ```
- [ ] `[T]` = implicit `Copyable` bound
- [ ] `[T: Copyable]` = explicit `Copyable` bound
- [ ] `[T: not Copyable]` = no copyability requirement

### 5.2 Semantic Model Changes

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

### 5.3 Conditional Conformance

- [ ] `Box[T]` is `Copyable` when `T: Copyable`
- [ ] Requires tracking conditional bounds on generic types
- [ ] Query: "is `Box[Int]` Copyable?" → Yes (Int is Copyable)
- [ ] Query: "is `Box[FileHandle]` Copyable?" → No (FileHandle is not Copyable)

### 5.4 Validation

- [ ] Error: "cannot copy value of type `T`; `T` may not be `Copyable`"
- [ ] Error: "type `FileHandle` does not satisfy bound `Copyable` required by `duplicate[T]`"

### 5.5 Tests

- [ ] `generic_copyability.rs`:
  - Default bound allows copy
  - not Copyable prevents copy
  - Calling with non-copyable type errors if bound requires Copyable
- [ ] `conditional_conformance.rs`:
  - Box[Int] is Copyable
  - Box[FileHandle] is not Copyable

---

## Phase 6: Law of Exclusivity

**Goal**: Prevent simultaneous conflicting accesses.

### 6.1 Borrow Tracking

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

### 6.2 Conflict Detection

- [ ] Before creating a new borrow, check for conflicts:
  - Mutable borrow conflicts with ANY existing borrow of same place
  - Shared borrow conflicts with existing mutable borrow
- [ ] "Overlapping access" includes:
  - Same variable
  - Field of borrowed struct
  - Element of borrowed array

### 6.3 Closure Captures

- [ ] Closure that captures mutable reference:
  - While closure exists, no other mutable access
- [ ] Non-escaping closures:
  - Borrow ends when closure scope ends
  - Can validate statically

### 6.4 Diagnostics

- [ ] "cannot borrow `x` as mutable because it is already borrowed as immutable"
- [ ] "cannot use `x` while mutable borrow is active"
- [ ] "mutable borrow of `x` occurs here" (secondary span)

### 6.5 Tests

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

1. **Phase 1** - Foundation, required by everything else
2. **Phase 2** - Core value proposition, enables move-only types
3. **Phase 3** - RAII is critical per requirements
4. **Phase 5** - Generics before Cloneable (standard library needs this)
5. **Phase 4** - Cloneable builds on Copyable infrastructure
6. **Phase 6** - Can be done in parallel with later phases

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
