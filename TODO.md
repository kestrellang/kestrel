# Kestrel TODO

## Phase 13: Standard Library & Syntactic Sugar

**Total Errors:** 456 (most are cascading from 18 parse errors and import resolution)

---

### 1. Parse Errors (14 files, blocks everything)

These parse errors cause cascading failures throughout the stdlib.

| File | Root Cause |
|------|------------|
| `collections/array.ks` | `if let` pattern |
| `collections/dictionary.ks` | `if let` pattern |
| `collections/set.ks` | Closure syntax `{ (elem, _) in ... }` |
| `core/protocols.ks` | Extension on protocol not supported |
| `iter/adapters.ks` | Closure syntax `{ item in ... }` |
| `iter/extensions.ks` | Closure syntax `{ (a, b) in ... }` |
| `json/json.ks` | `if let` pattern |
| `memory/buffer.ks` | `ref T` type not supported |
| `ops/range.ks` | Extension on protocol not supported |
| `result/optional.ks` | Extension on protocol not supported |
| `result/result.ks` | Extension on protocol not supported |
| `serde/serde.ks` | `if let` pattern |
| `text/string.ks` | `if let` pattern |
| `text/views.ks` | `if let` pattern |

**Priority features to implement:**
- [ ] Extension on protocols (`extend Iterator { ... }`)
- [ ] Closure syntax (`{ (a, b) in ... }`)
- [ ] `if let` pattern matching

---

### 2. Import/Type Resolution (25+ errors, causes ~200 cascading)

Cross-module type resolution fails. Types like `Equatable`, `Hashable`, `Numeric`, etc. are not found even though modules are declared.

**Error patterns:**
- `undefined name 'lang'` (25 errors)
- `cannot find type 'X' in this scope` (~180 errors for various protocol names)

**Missing types by frequency:**
| Type | Count | Module |
|------|-------|--------|
| `Rhs` | 20 | (type parameter, not import) |
| `FFISafe` | 14 | std.core |
| `Subtractable` | 11 | std.ops |
| `Multipliable` | 11 | std.ops |
| `ExpressibleByIntLiteral` | 11 | std.ops |
| `Divisible` | 11 | std.ops |
| `Addable` | 11 | std.ops |
| `Ordering` | 10 | std.core |
| `Numeric` | 10 | std.core |
| `Hashable` | 10 | std.core |
| `Equatable` | 9 | std.core |
| `Hasher` | 9 | std.core |
| `BitwiseAnd/Or/Xor/Not` | 9 each | std.ops |

**Root cause:** When compiling files individually, imports don't resolve symbols from other modules.

---

### 3. Cascading Errors (~200 errors)

These errors result from parse failures and type resolution failures above:

| Error | Count | Cause |
|-------|-------|-------|
| `type '<error>' is not callable` | 44 | Type resolution failed |
| `member not found on type '<error>'` | 38 | Type resolution failed |
| `cannot access member on type 'Self'` | 10 | Protocol extension parse failed |
| `cannot call 'write' on type 'H'` | 10 | Hasher constraint not resolved |
| `type arguments cannot be applied` | 7 | Type resolution failed |

These will resolve automatically when root causes are fixed.

---

### 4. Literal Protocol Conformance (7+ errors)

Core types need to implement literal protocols:

- [ ] `Bool` does not conform to `ExpressibleByBoolLiteral` (7 errors)
- [ ] `Int64` does not conform to `ExpressibleByIntLiteral`
- [ ] `Float64` does not conform to `ExpressibleByFloatLiteral`
- [ ] `String` does not conform to `ExpressibleByStringLiteral`

**Note:** The protocols exist but the conformance/initializers aren't being recognized.

---

### 5. Self Mutability Errors (16 errors)

| Error | Count | Fix |
|-------|-------|-----|
| `cannot use 'self' in free function` | 6 | Parse error cascade - methods parsed as functions |
| `cannot assign to immutable variable 'self'` | 10 | Add `mutating` to method declarations |

**Files needing `mutating` fixes:**
- `collections/array.ks` - multiple methods

---

### 6. Pointer Type Errors (6 errors)

```
type mismatch: expected `Pointer[T]`, found `Pointer`
```

Generic pointer instantiation issue in `memory/pointer.ks`.

---

## Compiler Features Needed

### High Priority (blocks most files)

- [ ] **Cross-module symbol resolution**
  - Modules compile but can't import symbols from each other
  - Need to load dependencies when checking a file

### Medium Priority (blocks specific features)

- [ ] **Extension on protocols**
  - `extension Iterator { func map[U](...) }`
  - Blocks: `core/protocols.ks`, `ops/range.ks`, `result/result.ks`, `iter/extensions.ks`

- [ ] **Literal protocol recognition**
  - Types with `@builtin(.ExpressibleBy*Literal)` need working conformance

### Completed

- [x] Computed properties (parser, semantic, binder, lowering)
- [x] Protocol property requirements (`{ get }`, `{ get set }`)
- [x] Builtin literal protocol attributes registered
- [x] `@builtin` on type aliases
- [x] Where clauses on associated types (`type Iter: Iterator where Iter.Item = Item`)
- [x] Generic initializers with where clauses (`init[I](from iter: I) where I: Iterator`)

---

## Stdlib Code Status

### Fixed Issues

- [x] Type equality syntax (`==` → `=`)
- [x] Inline constraints to where clauses (`T: A + B` → `where T: A, T: B`)
- [x] `ref` → `mutating` parameter mode
- [x] Hash function constraint syntax
- [x] Allocator constraints on collections
- [x] Keyword conflicts (`and/or/not` → method names)
- [x] Module declarations (37 files)
- [x] Missing imports

### Needs Fixing

- [ ] Add `mutating` to methods that modify `self` in `collections/array.ks`
- [ ] Review parse errors for simple syntax fixes

---

## Future Work

### Standard Library Implementation

- [ ] Core types: Option[T], Result[T, E]
- [ ] Collections: Array, Dictionary, Set
- [ ] String utilities
- [ ] I/O primitives

### Syntactic Sugar

- [ ] `T?` for `Option[T]`
- [ ] `?` operator for error propagation
- [ ] Optional chaining `x?.foo`
- [ ] For loops (iterator protocol)
- [ ] Compound assignment operators (`+=`, `-=`, `*=`, `/=`, etc.)
- [ ] Closure syntax (`{ (a, b) in ... }` and `{ |a, b| ... }`)
- [ ] `try` expression for Result/error handling

### Remaining from Phase 10

- [ ] Analysis infrastructure (CFG, dataflow)
- [ ] Optimization passes (DCE, constant folding, inlining)
- [ ] Thin closure optimization

---

## Quick Reference

```bash
# Check all stdlib
cargo run -- check lang/std/**/*.ks

# Count errors
cargo run -- check lang/std/**/*.ks 2>&1 | grep "^error:" | wc -l

# Count by type
cargo run -- check lang/std/**/*.ks 2>&1 | grep "^error:" | sort | uniq -c | sort -rn
```
