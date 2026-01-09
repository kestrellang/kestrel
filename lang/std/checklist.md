# Standard Library Compilation Checklist

This checklist tracks compiler features needed for `lang/std/` to compile.

**Current Status:** ~261 errors (up from 49 due to more code now parsing successfully)

Most errors are cascading from import resolution (49 "undefined name 'lang'" errors) and type resolution issues.

Run: `cargo run -- check lang/std/**/*.ks`

---

## 1. Computed Properties

**Status:** COMPLETED
**Priority:** High - blocks most files

Computed properties are now fully implemented:
- Parser supports shorthand `{ expr }` and explicit `{ get { } set { } }` syntax
- Semantic symbols (GetterSymbol, SetterSymbol) created as children of FieldSymbol
- Binder adds CallableBehavior and ExecutableBehavior to getter/setter
- Lowering generates getter/setter calls instead of direct field access
- Assignment to computed properties generates setter calls

```kestrel
// Instance computed property
public var isEmpty: Bool { self.count == 0 }

// Static computed property
public static var zero: Int64 { Int64(value: 0) }

// Getter + setter
public var value: Int {
    get { self._value }
    set { self._value = newValue; }
}
```

---

## 2. Protocol Property Requirements

**Status:** COMPLETED
**Priority:** High - blocks protocol definitions

Protocol property requirements are now fully implemented:
- Parser supports `var` declarations in protocol bodies
- `{ get }` and `{ get set }` requirement syntax supported
- Conformance checking validates properties match requirements
- Stored `var` fields satisfy `{ get set }`, any field satisfies `{ get }`

```kestrel
public protocol Numeric {
    static var zero: Self { get }
    static var one: Self { get }
}
```

---

## 3. Where Clauses on Associated Types (1 error, 1 file)

**Status:** Not implemented
**Priority:** Medium

Parser doesn't support where clauses on associated type declarations.

```kestrel
public protocol Iterable {
    type Iter: Iterator where Iter.Item = Item
}
```

**Files affected:**
- `iter/iterator.ks:13` - `type Iter: Iterator where Iter.Item = Item`

---

## 4. Extension on Protocols (2 errors, 2 files)

**Status:** Not implemented
**Priority:** Medium - blocks default implementations

Parser/binder doesn't support extensions that add methods to protocols.

```kestrel
extension Iterator {
    public func map[U](transform: (Item) -> U) -> MapIterator[Self, U] { ... }
}
```

**Files affected:**
- `iter/extensions.ks:5` - `extension Iterator { ... }`
- `core/protocols.ks:11` - `extension Equatable: Equal[Self] { ... }`

---

## 5. Extension Adding Protocol Conformance (3 errors, 3 files)

**Status:** Not implemented
**Priority:** Medium - blocks default operator implementations

Parser/binder doesn't support extensions that add protocol conformance.

```kestrel
extension Equatable: Equal[Self], NotEqual[Self] {
    type Output = Bool
    func eq(other: Self) -> Bool { self.equals(other) }
}

extension Addable[Rhs] where Output = Self: AddAssign[Rhs] { ... }
```

**Files affected:**
- `core/protocols.ks:11` - `extension Equatable: Equal[Self] { ... }`
- `ops/assign.ks:66` - `extension Addable[Rhs] where Output = Self: AddAssign { ... }`
- `result/error.ks:15` - `extension Tryable { ... }`
- `memory/allocator.ks:12` - `extension Allocator { ... }`

---

## 6. Import Symbol Resolution (6 errors, 2 files)

**Status:** Partially working
**Priority:** High

Module resolution works but symbol lookup within modules fails when compiling files individually or in isolation.

```kestrel
import std.core.(Equatable)  // Module found, symbol not found
import std.core.(Int64, Float64)  // Module found, symbols not found
```

**Files affected:**
- `core/ordering.ks:5` - `import std.core.(Equatable)`
- `ops/literals.ks:5` - `import std.core.(Int64, Float64)`

**Error messages:**
- `symbol 'Equatable' not found in module 'std.core'`
- `symbol 'Int64' not found in module 'std.core'`
- `symbol 'Float64' not found in module 'std.core'`
- `cannot find type 'Equatable' in this scope`
- `cannot find type 'Int64' in this scope`
- `cannot find type 'Float64' in this scope`

**Note:** These errors occur because files are compiled in isolation. A proper module system that loads dependencies would resolve these.

---

## 7. Builtin Literal Protocol Attributes

**Status:** ✅ COMPLETED
**Priority:** Low - only affects literal syntax

The following builtin attributes are now registered:
- `@builtin(.ExpressibleByNilLiteral)`
- `@builtin(.ExpressibleByArrayLiteral)`
- `@builtin(.ExpressibleByDictionaryLiteral)`
- `@builtin(.DefaultIntegerLiteralType)` - for type aliases
- `@builtin(.DefaultFloatLiteralType)` - for type aliases

Type aliases can now use `@builtin` attributes (parser updated to support attributes on type aliases).

---

## 7b. Literal Protocol Conformance (4 errors, 1 file)

**Status:** Not implemented
**Priority:** Medium - blocks literal syntax usage

Core types don't conform to literal protocols yet.

```
error: type `Bool` does not conform to protocol `ExpressibleByBoolLiteral`
```

**Files affected:**
- `core/bool.ks` - Bool needs to conform to `ExpressibleByBoolLiteral`

**Note:** This is a new error that appears now that the literal protocol infrastructure is working. The conformance declarations exist but the types don't have the required `init(boolLiteral:)` initializers implemented correctly.

---

## 8. Subscript Declarations (1 error, 1 file)

**Status:** Not implemented
**Priority:** Medium - blocks collection indexing

Parser doesn't support subscript declarations.

```kestrel
public subscript(index: Int) -> T { get set }
public subscript(safe index: Int) -> Optional[T] { get }
```

**Files affected:**
- `collections/array.ks:63` - `subscript[index: Int]`

---

## 9. Cascade Errors (6 errors, 1 file)

These errors are caused by earlier parse failures in the same file.

**Files affected:**
- `collections/array.ks` - 6 "cannot use 'self' in free function" errors

These will resolve when the root causes (subscripts, builtin attributes) are fixed.

---

## 10. Other Parse Issues (5 errors, 5 files)

Various parse errors from unsupported syntax:

| File | Line | Issue |
|------|------|-------|
| `core/bool.ks:56` | `Boolean` token | Unknown cause |
| `collections/dictionary.ks:50` | `.` in wrong context | Tuple access or method chain |
| `collections/set.ks:26` | `where` clause | Where on struct |
| `iter/adapters.ks:114` | `}` expecting `:` | Pattern or match issue |
| `memory/buffer.ks:13` | `let` in wrong context | Constant in struct |
| `memory/layout.ks:20` | `(` expecting `:` | Function syntax |

---

## Summary by Priority

### High Priority (blocks most files)
1. **Import symbol resolution** - 6 errors

### Medium Priority (blocks specific features)
2. **Extension on protocols** - 2 errors
3. **Extension adding conformance** - 3 errors
4. **Where on associated types** - 1 error
5. **Subscripts** - 1 error
6. **Literal protocol conformance** - 4 errors (new)

### Low Priority (minor features)
7. **Cascade errors** - 6 errors (auto-fix)
8. **Other parse issues** - ~5 errors

### Completed
- **Computed properties** - Full implementation (parser, semantic, binder, lowering)
- **Protocol property requirements** - Parser + conformance checking
- **Builtin literal attributes** - All literal protocol builtins registered
- **@builtin on type aliases** - Parser now supports attributes on type aliases

---

## Error Count by File

| File | Errors | Root Cause |
|------|--------|------------|
| `collections/array.ks` | 10 | Subscripts, builtins, cascade |
| `core/int*.ks` (4 files) | 4 | Computed properties |
| `core/uint*.ks` (4 files) | 4 | Computed properties |
| `core/float*.ks` (2 files) | 2 | Computed properties |
| `core/protocols.ks` | 1 | Extension on protocol |
| `core/numeric.ks` | 1 | Protocol vars |
| `core/ordering.ks` | 2 | Import resolution |
| `iter/iterator.ks` | 1 | Where on associated type |
| `iter/extensions.ks` | 1 | Extension on protocol |
| `iter/adapters.ks` | 1 | Parse error |
| `ops/assign.ks` | 1 | Extension conformance |
| `ops/range.ks` | 1 | Computed property |
| `ops/literals.ks` | 4 | Import resolution |
| `result/optional.ks` | 1 | Computed property |
| `result/result.ks` | 1 | Computed property |
| `result/error.ks` | 1 | Extension on protocol |
| `memory/allocator.ks` | 1 | Extension on protocol |
| `memory/buffer.ks` | 1 | Parse error |
| `memory/layout.ks` | 1 | Parse error |
| `memory/pointer.ks` | 1 | Computed property |
| `text/char.ks` | 1 | Computed property |
| `text/string.ks` | 1 | Computed property |
| `text/views.ks` | 1 | Computed property |
| `serde/serde.ks` | 1 | Computed property |
| `json/json.ks` | 1 | Computed property |
| `collections/dictionary.ks` | 1 | Parse error |
| `collections/set.ks` | 1 | Where clause |
| `core/bool.ks` | 5 | Parse error, literal conformance |

---

## Testing

```bash
# Check all stdlib files
cargo run -- check lang/std/**/*.ks

# Check specific file
cargo run -- check lang/std/core/int64.ks

# Count errors
cargo run -- check lang/std/**/*.ks 2>&1 | grep "^error:" | wc -l
```
