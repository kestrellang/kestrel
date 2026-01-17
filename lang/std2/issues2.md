# std2 + io Module Compiler Issues

Issues discovered when compiling std2 and io modules together (25 errors total).

## 1. Static Method Calls on Generic Enum Types

**Location**: `lang/io/read.ks` (8 occurrences)

**Problem**: Calling static methods like `Result.ok(value: x)` fails with "no matching overload for 1 arguments with labels (value:)".

**Example**:
```kestrel
public func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
    Result.ok(value: 0)  // ERROR: no matching overload
}
```

**Cause**: `Result` is a generic type `Result[T, E]`. The compiler can't find the static method `ok(value: T)` because it needs concrete type parameters to resolve the method.

**Workaround**: Use enum shorthand `.Ok(0)` instead, which infers types from return context. However, this also has cross-module issues (see issues.md).

---

## 2. Generic Function Resolution with Protocol Constraints

**Location**: `lang/io/write.ks`, `lang/io/stdio.ks`, `lang/io/file.ks` (9 occurrences)

**Problem**: Generic functions with `where` clauses are not found when called.

**Example**:
```kestrel
// Definition in write.ks
public func writeAll[W](writer: W, from buf: Slice[UInt8]) -> Result[(), Error] where W: Write {
    // ...
}

// Call site - ERROR: no matching overload for 2 arguments
writeAll(writer: writer, from: slice)
```

**Affected functions**:
- `writeAll(writer:, from:)`
- `writeByte(writer:, byte:)`
- `writeStr(writer:, s:)`
- `writeLine(writer:, s:)`

**Cause**: The compiler isn't resolving generic functions with protocol constraints across module boundaries.

---

## 3. Pointer Type Casting Issues

**Location**: `lang/std2/memory/pointer.ks`, `lang/io/libc.ks` (5 occurrences)

**Problem**: Type mismatches when casting between pointer types and `lang.i8`.

**Examples**:
```kestrel
// pointer.ks:36 - expected `I8`, found `T`
Pointer(raw: lang.cast_ptr[T](self.raw))

// pointer.ks:94 - expected `T`, found `I8`
RawPointer(raw: lang.cast_ptr[lang.i8](self._raw))

// libc.ks:60,68,72 - expected `UInt8`, found `I8`
lang.cast_ptr[lang.i8](path.raw)  // path.raw is Pointer[UInt8]
```

**Cause**: `lang.cast_ptr` expects specific pointer element types but is receiving mismatched types.

---

## 4. Integer Conversion Issues

**Location**: `lang/io/error.ks` (2 occurrences)

**Problem**: Type mismatch in integer conversion.

**Example**:
```kestrel
// error.ks:25
let code64 = Int64(from: self.code);  // self.code is Int8, expects Int32
```

**Cause**: `Int64(from:)` initializer expects `Int32`, not `Int8`.

---

## 5. Missing `mutating` on Methods That Modify Self

**Location**: `lang/io/read.ks:71` (1 occurrence)

**Problem**: `cannot assign to immutable field 'pos'`

**Example**:
```kestrel
public struct Cursor: Read {
    var pos: Int64

    // Missing `mutating` keyword
    public func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        // ...
        self.pos = self.pos + n  // ERROR: cannot assign to immutable field
        // ...
    }
}
```

**Fix needed**: Add `mutating` to the method signature. However, this may conflict with the `Read` protocol definition if it doesn't declare the method as mutating.

---

## Summary

| Issue | Count | Severity |
|-------|-------|----------|
| Static method on generic enum | 8 | High |
| Generic function with where clause | 9 | High |
| Pointer type casting | 5 | High |
| Integer conversion | 2 | Medium |
| Missing mutating keyword | 1 | Medium |

**Total**: 25 errors

## Fixed Issues

### ~~Cascade Errors from Unresolved Types~~ (FIXED)

Previously, once a type became `<error>` due to earlier failures, all subsequent operations on that value failed with unhelpful messages like:
- `member not found: 'equals' on type '<error>'`
- `if condition must conform to 'BooleanConditional', found '_'`

**Fix**: Added `is_poison()` helper to `Ty` and early-exit checks in:
- `desugar_binary_op` / `desugar_unary_op` in operators.rs
- `check_if_condition` / `check_while_condition` / `check_if_branches` / `check_array_elements` in type_check/mod.rs

This reduced errors from 37 to 25 by eliminating cascade noise.
