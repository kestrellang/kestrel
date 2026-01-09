# Standard Library Fixes

This document tracks stdlib **code issues** (not compiler features) that have been fixed.

## Fixed Issues (2025-01-08)

### 1. Type Equality Syntax

**Problem:** Used `==` instead of `=` for type equality in where clauses.

**Fix:** `Output == Self` → `Output = Self`

**Files fixed:**
- `ops/assign.ks` - 9 occurrences

---

### 2. Inline Constraints to Where Clauses

**Problem:** Used inline `+` syntax for multiple constraints instead of where clauses.

**Fix:** `T: Steppable + Comparable` → `where T: Steppable, T: Comparable`

**Files fixed:**
- `ops/range.ks` - 2 occurrences
- `iter/adapters.ks` - `I: Iterator + Cloneable` → `I: Iterator, I: Cloneable`
- `collections/array.ks` - `T: Comparable + Cloneable` → `T: Comparable, T: Cloneable`
- `serde/serde.ks` - ~50 occurrences
- `json/json.ks` - ~15 occurrences

---

### 3. `ref` → `mutating` Parameter Mode

**Problem:** Used `ref` keyword for mutable reference parameters, but Kestrel uses `mutating`.

**Fix:** `ref H` → `mutating H`

**Files fixed:**
- `core/protocols.ks`
- `core/int8.ks`, `core/int16.ks`, `core/int32.ks`, `core/int64.ks`
- `core/uint8.ks`, `core/uint16.ks`, `core/uint32.ks`, `core/uint64.ks`
- `text/char.ks`, `text/string.ks`
- `collections/array.ks`, `collections/set.ks`
- `serde/serde.ks` - ~60 occurrences
- `json/json.ks` - multiple occurrences

---

### 4. Hash Function Constraint Syntax

**Problem:** Used inline constraint on generic parameter instead of where clause.

**Fix:** `hash[H: Hasher]` → `hash[H] where H: Hasher`

**Files fixed:**
- All files with `Hashable` implementations (same as #3)

---

### 5. Added `where A: Allocator` Constraints

**Problem:** Generic allocator parameter `A` was missing constraint.

**Fix:** Added `where A: Allocator` to type declarations.

**Files fixed:**
- `collections/array.ks` - `Array[T, A]`
- `collections/dictionary.ks` - `Dictionary[K, V, A]`
- `collections/set.ks` - `Set[T, A]`
- `text/string.ks` - `String[A]`

---

### 6. Keyword Conflicts in Method Names

**Problem:** `and`, `or`, `not` are keywords in Kestrel but were used as method names.

**Fix:** Renamed methods to avoid keyword conflicts.

**Files fixed:**
- `ops/logical.ks` - Renamed protocol methods:
  - `And.and()` → `And.logicalAnd()`
  - `Or.or()` → `Or.logicalOr()`
  - `Not.not()` → `Not.logicalNot()`
- `core/bool.ks` - Renamed implementations to match
- `result/optional.ks` - Renamed combinators:
  - `and()` → `andValue()`
  - `or()` → `orValue()`
- `result/result.ks` - Renamed combinators:
  - `and()` → `andValue()`
  - `or()` → `orValue()`
- `docs/overview.md` - Updated documentation

---

### 7. Missing Module Declarations

**Problem:** Stdlib files were missing `module` declarations, causing `module 'std.core' not found` errors when importing.

**Files fixed (37 total):**

**core/ (14 files):**
- `bool.ks` - Added `module std.core`
- `float32.ks` - Added `module std.core`
- `float64.ks` - Added `module std.core`
- `int8.ks` - Added `module std.core`
- `int16.ks` - Added `module std.core`
- `int32.ks` - Added `module std.core`
- `int64.ks` - Added `module std.core`
- `uint8.ks` - Added `module std.core`
- `uint16.ks` - Added `module std.core`
- `uint32.ks` - Added `module std.core`
- `uint64.ks` - Added `module std.core`
- `numeric.ks` - Added `module std.core`
- `ordering.ks` - Added `module std.core`
- `protocols.ks` - Added `module std.core`

**ops/ (7 files):**
- `arithmetic.ks` - Added `module std.ops`
- `assign.ks` - Added `module std.ops`
- `bitwise.ks` - Added `module std.ops`
- `comparison.ks` - Added `module std.ops`
- `literals.ks` - Added `module std.ops`
- `logical.ks` - Added `module std.ops`
- `range.ks` - Added `module std.ops`

**result/ (3 files):**
- `error.ks` - Added `module std.result`
- `optional.ks` - Added `module std.result`
- `result.ks` - Added `module std.result`

**iter/ (3 files):**
- `adapters.ks` - Added `module std.iter`
- `extensions.ks` - Added `module std.iter`
- `iterator.ks` - Added `module std.iter`

**memory/ (4 files):**
- `allocator.ks` - Added `module std.memory`
- `buffer.ks` - Added `module std.memory`
- `layout.ks` - Added `module std.memory`
- `pointer.ks` - Added `module std.memory`

**text/ (3 files):**
- `char.ks` - Added `module std.text`
- `string.ks` - Added `module std.text`
- `views.ks` - Added `module std.text`

**collections/ (3 files):**
- `array.ks` - Added `module std.collections`
- `dictionary.ks` - Added `module std.collections`
- `set.ks` - Added `module std.collections`

**Result:** Module resolution now finds modules. Errors changed from "module not found" to "symbol not found in module" (remaining issue is compiler-side symbol resolution).

---

### 8. Missing Imports

**Problem:** Some files referenced types from other modules without importing them.

**Files fixed:**
- `core/ordering.ks` - Added `import std.core.(Equatable)`
- `ops/literals.ks` - Added `import std.core.(Float64)`

**Note:** These imports now reach the module but symbol resolution within modules is a compiler issue.

---

## Known Issues (Documented in `docs/stdlib-issues.md`)

These issues are documented but intentionally not fixed yet:

1. **Type parameter constraints** - Use `where` instead of `:`
2. **`~` operator** - Use `.bitwiseNot()` method instead
3. **`$0` shorthand** - Use explicit closure parameters
4. **`null` as function name** - Use `nilPointer` instead

---

## Remaining Issues

### Stdlib Code Issues:
1. `ref` fields in struct types (e.g., `private var serializer: ref JsonSerializer`) - these are reference-typed fields, not parameters

### Compiler Issues (Not Stdlib):
All other remaining errors require compiler features:

1. **Computed properties** (~20 errors) - Parser doesn't support `var x: Type { ... }`
2. **Where clauses on extensions** (2 errors) - Parser doesn't support
3. **Subscripts** (1 error) - Parser doesn't support subscript declarations
4. **Symbol resolution in imports** (2 errors) - Module found but symbols not resolved
5. **Unknown builtin attributes** (3 errors) - `.ExpressibleByNilLiteral`, etc.
6. **Protocol vars** (1 error) - Parser doesn't support `var` in protocols
7. **Cascading errors** (~15) - From the above parse failures

---

## Summary

| Category | Occurrences Fixed |
|----------|-------------------|
| Type equality syntax (`==` → `=`) | 9 |
| Inline constraints to where clauses | ~70 |
| `ref` → `mutating` parameters | ~80 |
| Hash function constraints | ~15 |
| Allocator constraints | 4 |
| Keyword conflicts | 5 files |
| Module declarations | 37 files |
| Missing imports | 2 files |
