# Builtins

Types and functions the compiler treats specially. Most are also documented in the [Stdlib reference](stdlib.md) — this page is the lookup table.

## Primitive types

| Type | Description | Range / Notes |
|---|---|---|
| `Int` | Default integer type | Platform-sized signed (64-bit on most targets) |
| `Int8` `Int16` `Int32` `Int64` | Sized signed integers | -2^(n-1) to 2^(n-1) - 1 |
| `UInt8` `UInt16` `UInt32` `UInt64` | Sized unsigned integers | 0 to 2^n - 1 |
| `Float` | Default floating point | Alias for `Float64` |
| `Float32` `Float64` | IEEE-754 floats | Single / double precision |
| `Bool` | Boolean | `true` or `false` |
| `Char` | Unicode scalar value | 32-bit, valid Unicode codepoint |
| `String` | UTF-8 string | Heap-backed, immutable |
| `Ptr` `OpaquePtr` | Raw pointers | FFI use only |
| `!` | Never type | Bottom type — see [Functions → Return Types](../functions/index.md#return-types) |

## Compiler-provided functions

A handful of names that look like functions but are compiler intrinsics:

| Function | Purpose |
|---|---|
| `panic(message: String) -> !` | Abort the program with a message. Returns `!`. |
| `assert(condition: Bool, message: String)` | Panic if condition is false (in debug builds). |
| `unreachable() -> !` | Mark a code path as impossible. Compile-time hint and runtime panic. |
| `sizeof[T]() -> Int` | Static size of a type, in bytes. |
| `alignof[T]() -> Int` | Static alignment of a type, in bytes. |
| `typeof(value) -> TypeId` | Runtime type identifier of a value. |

Use these sparingly. Most code shouldn't reach for `sizeof` or `typeof`; if it does, that's a sign you wanted FFI or generics rather than reflection.

## Special literals

| Literal | Type | Notes |
|---|---|---|
| `42` | `Int` | Decimal integer |
| `0xFF` | `Int` | Hex |
| `0b1010` | `Int` | Binary |
| `0o755` | `Int` | Octal |
| `3.14` | `Float` | Float (the `.` is required) |
| `true` `false` | `Bool` | |
| `'A'` | `Char` | Single-quoted character |
| `"text"` | `String` | Double-quoted string |
| `"\(expr)"` | `String` | String interpolation |

---

[← Operators](operators.md) · [↑ Reference](index.md)
