# Visibility

Visibility controls access to declarations. Kestrel has four visibility levels inspired by Swift.

## Syntax

```
Visibility → PUBLIC | PRIVATE | INTERNAL | FILEPRIVATE
```

### Tokens
- `PUBLIC` - The `public` keyword
- `PRIVATE` - The `private` keyword
- `INTERNAL` - The `internal` keyword
- `FILEPRIVATE` - The `fileprivate` keyword

## Visibility Levels

### Public

```kestrel
public struct API { }
public func process() { }
```

- **Accessible from:** Everywhere
- **Use case:** Public API, library interfaces
- **Numeric value:** 4 (most visible)

### Internal (Default)

```kestrel
internal struct Helper { }
struct Helper { }           // Same as above (default)
func utility() { }         // Default is internal
```

- **Accessible from:** Within the same module
- **Use case:** Module-internal implementation
- **Numeric value:** 3
- **Note:** This is the default if no modifier is specified

### Fileprivate

```kestrel
fileprivate struct FileHelper { }
fileprivate func fileUtility() { }
```

- **Accessible from:** Within the same source file
- **Use case:** File-local helpers, related types in same file
- **Numeric value:** 2

### Private

```kestrel
private struct Secret { }
private func internalImpl() { }
```

- **Accessible from:** Within the enclosing declaration and its nested types
- **Use case:** Implementation details, encapsulated state
- **Numeric value:** 1 (least visible)

## Visibility Hierarchy

```
Public (4)      Most visible - accessible everywhere
   ↑
Internal (3)    Accessible within module
   ↑
Fileprivate (2) Accessible within file
   ↑
Private (1)     Least visible - accessible within declaring scope
```

## Visibility Scope

Each symbol has a **visibility scope** that determines where private access is allowed:

```kestrel
struct Outer {
    private let secret: Int    // visibility_scope = Outer

    struct Inner {
        func useSecret() {
            // Can access Outer.secret because Inner is inside Outer
        }
    }
}

struct Other {
    func tryAccess(o: Outer) {
        // Cannot access o.secret - Other is not inside Outer
    }
}
```

### Scope Rules

| Declaration Context | Visibility Scope |
|--------------------|------------------|
| Module-level | The module |
| Inside struct | The struct |
| Inside struct | The struct |
| Inside protocol | The protocol |
| Nested type | The enclosing type |

## Visibility Checking

### Is Visible From

A symbol `S` is visible from context `C` if:

```
is_visible(S, C):
    match S.visibility:
        Public:
            return true

        Internal:
            return same_module(S, C)  // TODO: not fully implemented

        Fileprivate:
            return same_file(S, C)    // TODO: not fully implemented

        Private:
            visibility_scope = S.visibility_scope
            return C == visibility_scope OR is_descendant(C, visibility_scope)
```

### Descendant Check

```
is_descendant(C, scope):
    current = C
    while current is not None:
        if current == scope:
            return true
        current = current.parent
    return false
```

## Visibility Consistency Rules

Public declarations cannot expose less-visible types.

### Rule 1: Public Function Return Type

```
ERROR: VisibilityConsistencyPass error
WHEN: Public function returns a less-visible type
WHY: Callers couldn't use the return value's type
```

**Example (invalid):**
```kestrel
private struct Secret { }

public func getSecret() -> Secret { }    // ERROR: exposes private type
```

### Rule 2: Public Function Parameter Types

```
ERROR: VisibilityConsistencyPass error
WHEN: Public function has parameter of less-visible type
WHY: Callers couldn't provide arguments of the required type
```

**Example (invalid):**
```kestrel
private struct Config { }

public func configure(c: Config) { }    // ERROR: exposes private type in parameter
```

### Rule 3: Public Type Alias Target

```
ERROR: VisibilityConsistencyPass error
WHEN: Public type alias aliases a less-visible type
WHY: Users of the alias couldn't access the underlying type
```

**Example (invalid):**
```kestrel
private struct Impl { }

public type API = Impl;    // ERROR: exposes private type
```

### Rule 4: Public Field Type

```
ERROR: VisibilityConsistencyPass error
WHEN: Public field has a less-visible type
WHY: Users couldn't work with the field's value
```

**Example (invalid):**
```kestrel
private struct Data { }

public struct Container {
    public let data: Data    // ERROR: exposes private type
}
```

## Visibility and Imports

### Import Visibility Checking

When importing symbols, visibility is checked:

```
ERROR: SymbolNotVisibleError
WHEN: Trying to import a symbol with insufficient visibility
```

**Example:**
```kestrel
// In module A:
private struct Secret { }
public struct Public { }

// In module B:
import A.(Public)    // OK
import A.(Secret)    // ERROR: 'Secret' is not accessible
```

### Whole-Module Import Filtering

When doing `import M`, only visible symbols are imported:

```kestrel
// In module Lib:
public struct PublicClass { }
private struct PrivateStruct { }

// In module App:
import Lib    // Only PublicClass is imported, not PrivateClass
```

## Examples

### Module Design

```kestrel
module MyLib

// Public API
public struct Client {
    public func connect() { }
    public func disconnect() { }

    // Internal implementation detail
    private var socket: Socket
    private func sendPacket(data: Data) { }
}

// Internal helper, not part of public API
internal struct Socket {
    func open() { }
    func close() { }
}

// File-local utilities
fileprivate func log(message: String) { }
```

### Encapsulation

```kestrel
public struct Account {
    // Public read-only identifier
    public let id: Int

    // Internal state
    private var balance: Int
    private var transactions: (Transaction)

    // Public interface
    public func getBalance() -> Int { }
    public func deposit(amount: Int) { }
    public func withdraw(amount: Int) -> Bool { }

    // Private implementation
    private func recordTransaction(t: Transaction) { }
    private func validateAmount(amount: Int) -> Bool { }
}
```

### Nested Types

```kestrel
public struct Outer {
    // Private nested type
    private struct PrivateInner {
        let value: Int
    }

    // Public nested type
    public struct PublicInner {
        let data: String
    }

    // Private field using private type (OK)
    private let inner: PrivateInner

    // Public field using public type (OK)
    public let publicInner: PublicInner

    // ERROR: public field using private type
    // public let exposed: PrivateInner
}
```

## Formal Semantics

### Visibility Ordering

```
Public > Internal > Fileprivate > Private

visibility_value(Public) = 4
visibility_value(Internal) = 3
visibility_value(Fileprivate) = 2
visibility_value(Private) = 1
```

### Visibility Constraint

For public symbol `S` with type `T`:

```
visibility(S) = Public implies visibility(T) >= visibility(S)

∀ public symbol S:
    ∀ type T referenced by S:
        T must be public
```

### Access Check

```
can_access(S, from_context):
    v = S.visibility
    scope = S.visibility_scope

    if v == Public:
        return true
    if v == Private:
        return from_context == scope OR ancestor(from_context, scope)
    if v == Fileprivate:
        return same_file(from_context, scope)
    if v == Internal:
        return same_module(from_context, scope)
```

## VisibilityBehavior

Symbols store visibility information in a `VisibilityBehavior`:

```rust
struct VisibilityBehavior {
    visibility: Visibility,
    visibility_scope: SymbolId,  // The scope that defines "private" access
}
```

## Default Visibility

If no visibility modifier is specified, the default is `Internal`:

```kestrel
struct MyStruct { }          // internal
func myFunc() { }          // internal
struct MyStruct { }        // internal
let myField: Int           // internal (inside a type)
```

## Source Location

- **Behavior:** `lib/kestrel-semantic-tree/src/behavior/visibility.rs`
- **Access checking:** `lib/kestrel-semantic-model/src/queries/is_visible_from.rs`
- **Validate (consistency):** `lib/kestrel-semantic-analyzers/src/analyzers/visibility_consistency/mod.rs`
