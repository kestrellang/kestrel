# Stdlib

The standard library reference is **auto-generated** from doc comments in the stdlib source. Each module has its own page, with one section per public type and per type a list of methods, conformances, and protocol implementations.

## Top-level modules

- **`std.num`** — `Int`, `Int32`, `Int64`, `UInt32`, `UInt64`, `Float`, `Float32`, `Float64`, numeric protocols (`Numeric`, `Integer`, `Floating`)
- **`std.bool`** — `Bool`
- **`std.str`** — `String`, `Char`, string operations
- **`std.collections`** — `Array`, `Dict`, `Set`, `Tuple`, `Iterator`, `Iterable`
- **`std.optional`** — `Optional[T]`
- **`std.result`** — `Result[T, E]`
- **`std.io`** — `stdio`, `File`, `Path`, `IOError`
- **`std.fmt`** — formatting, `Display`, `Debug`
- **`std.cmp`** — `Comparable`, `Equatable`, `Ordering`
- **`std.hash`** — `Hashable`, hash-builders
- **`std.math`** — `sqrt`, `pow`, trig, `Pi`
- **`std.time`** — `Instant`, `Duration`
- **`std.os`** — process, env, args
- **`std.ffi`** — `Ptr`, `OpaquePtr`, `FFISafe`

## How to read a generated page

Every type page has the same structure:

1. **Summary.** One sentence describing what the type is.
2. **Conformances.** Which protocols the type implements.
3. **Variants** (for enums) or **Fields** (for structs).
4. **Methods.** Signatures, doc comments, and example code where present.
5. **Operators.** What `+`, `==`, `<` etc. do on this type, if anything.

Methods inherited from protocol defaults link to the protocol page. Methods added by an extension link to the file declaring the extension.

## Searching

The docs site has search built in (top-right corner). It indexes type names, method names, and the body text of each generated page — fastest way to find "what's the method that does X" without remembering which type owns it.

---

[← Diagnostics](diagnostics.md) · [↑ Reference](index.md) · [Operators →](operators.md)
