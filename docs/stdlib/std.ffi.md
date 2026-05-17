# std.ffi

## struct `CString`

```kestrel
public struct CString { /* private fields */ }
```

A null-terminated, non-owning byte pointer suitable for `@extern(.C)` boundaries.

`CString` is an FFI shim — it carries a `Pointer[UInt8]` that
the C side will treat as `const char *`, but it does **not** own
the memory. The pointer's lifetime, validity, and disposal are
entirely the caller's responsibility. Two common ownership
patterns: (1) the C side returns a pointer into static or
`environ` memory — wrap it in a `CString` and read, but never
free; (2) the Kestrel side allocates via `String.toCString()` —
the caller must `free()` the result.

### Examples

```
@extern(.C, mangleName: "puts")
func puts(s: CString) -> Int32

let cstr = "Hello, C!".toCString();
let _ = puts(cstr);
cstr.free();
```

### Safety

- The pointer must remain valid for as long as the `CString` is used.
- The pointed-to bytes must end with a `0` terminator.
- `length` is computed by scanning to the terminator — quadratic
  if you build long strings by repeated reads of `length`.
- The caller chooses whether `free()` is appropriate (yes for
  self-allocated, no for borrowed pointers).

### Representation

A single `Pointer[UInt8]` field. No length is cached.

### Memory Model

Non-owning. Conforms to `FFISafe` so it passes through
`@extern(.C)` signatures unchanged.

_Defined in `lang/std/ffi/cstring.ks`._

### Members

#### initializer `From Pointer`

```kestrel
public init(raw: Pointer[UInt8])
```

Wraps an existing pointer as a `CString`.

Performs no validation — the caller affirms that the pointer
is null or points at null-terminated memory.

##### Safety

- `rawPtr` must be null or point at a null-terminated byte
  sequence.
- The pointed-to bytes must remain valid for the lifetime of
  the `CString`.
- The caller decides whether `free()` is later appropriate.

_Defined in `lang/std/ffi/cstring.ks`._

#### function `free`

```kestrel
public func free()
```

Frees the buffer pointed to by this `CString` via libc `free`.

No-op when the pointer is null. After this call the `CString`
is dangling — do not read its bytes or call any other method
that touches the pointer.

##### Safety

Only call this on `CString`s whose pointer was produced by a
prior `malloc` (e.g. via `String.toCString()`). Calling on a
borrowed pointer (returned by `getenv`, a string literal,
etc.) is undefined behaviour.

_Defined in `lang/std/ffi/cstring.ks`._

#### field `isNull`

```kestrel
public var isNull: Bool { get }
```

True if the wrapped pointer is null.

A null `CString` should not be passed to a C function that
expects a string; check this before calling.

_Defined in `lang/std/ffi/cstring.ks`._

#### field `length`

```kestrel
public var length: Int64 { get }
```

Length of the string in bytes, **excluding** the null terminator.

Computed by linear scan — O(n). Cache the result if you
need it more than once. Returns `0` for a null pointer
(defensive: avoids dereferencing).

_Defined in `lang/std/ffi/cstring.ks`._

#### field `raw`

```kestrel
public var raw: Pointer[UInt8]
```

The underlying pointer to the null-terminated bytes.

_Defined in `lang/std/ffi/cstring.ks`._

## protocol `FFISafe`

```kestrel
public protocol FFISafe
```

Marker protocol for types that can cross an `@extern(.C)` boundary.

`FFISafe` is empty — conformance is purely a contract that the
type's in-memory layout matches what a C function expects.
Conformance is *not* automatically transitive at the type level:
the compiler checks `FFISafe` constraints separately on each
generic instantiation. The conformance rules:

  - Primitive numeric types and `Bool` conform automatically.
  - `Pointer[T]` conforms iff `T: FFISafe`.
  - Tuples of `FFISafe` types conform.
  - User structs may opt in (`struct Foo: FFISafe`); every field
    must itself be `FFISafe`.

String and other heap-managed types do **not** conform — convert
at the boundary with `String.toCString()` and pass `CString`.

### Examples

```
struct Point: FFISafe { var x: Int32; var y: Int32 }

@extern(.C, mangleName: "process_point")
func processPoint(p: Point) -> Int32
```

_Defined in `lang/std/ffi/ffi.ks`._

## function `free`

```kestrel
public func free(consuming RawPointer)
```

Wraps `free(3)` — releases memory previously returned by `malloc` / `realloc`.

Calling `free` on a null pointer is defined as a no-op. Calling
it on any other pointer that was not produced by these
allocators (or has already been freed) is undefined behaviour.

### Safety

`ptr` must be either null or the original pointer returned by a
previous `malloc` / `realloc`. After `free`, the pointer is
dangling — do not read, write, or free it again.

_Defined in `lang/std/ffi/libc.ks`._

## function `malloc`

```kestrel
public func malloc(consuming Int64) -> RawPointer
```

Wraps `malloc(3)` — allocates `size` bytes of uninitialised memory.

Returns a pointer to the start of the block, or null on failure.
The memory is **uninitialised** — read it only after writing
every byte you intend to use, or follow up with `memset` /
`calloc` (not exposed here). Free with `free`.

### Safety

The returned pointer is raw. Callers are responsible for not
reading uninitialised bytes, not exceeding `size`, and pairing
every successful call with exactly one `free`.

### Examples

```
let buf = malloc(1024);
// ... use buf ...
free(buf);
```

_Defined in `lang/std/ffi/libc.ks`._

## function `memcmp`

```kestrel
public func memcmp(consuming RawPointer, consuming RawPointer, consuming Int64) -> Int32
```

Wraps `memcmp(3)` — compares the first `n` bytes of `a` and `b`.

Returns a negative value if the first differing byte in `a` is less
than the corresponding byte in `b`, zero if all bytes are equal,
positive otherwise. Comparison is unsigned, byte-by-byte.

### Safety

Both `a` and `b` must be valid for `n` bytes.

_Defined in `lang/std/ffi/libc.ks`._

## function `memcpy`

```kestrel
public func memcpy(consuming RawPointer, consuming RawPointer, consuming Int64) -> RawPointer
```

Wraps `memcpy(3)` — copies `n` bytes from `src` to `dest`.

**Source and destination must not overlap** — use `memmove` if
they might. Returns `dest` for convenience. No bounds checking;
the caller must ensure both regions are valid for `n` bytes.

### Safety

`src` and `dest` must be valid for `n` bytes each, and the two
regions must not overlap.

_Defined in `lang/std/ffi/libc.ks`._

## function `memmem`

```kestrel
public func memmem(consuming RawPointer, consuming Int64, consuming RawPointer, consuming Int64) -> RawPointer
```

Wraps `memmem(3)` — locates the first occurrence of the `needleLen`-byte
`needle` in the `haystackLen`-byte `haystack`.

Returns a pointer to the start of the match, or null if not found.
`needleLen == 0` returns `haystack` (per glibc/macOS conventions —
callers should check this before calling). Available on Linux and
macOS; not on Windows.

### Safety

`haystack` must be valid for `haystackLen` bytes; `needle` must be
valid for `needleLen` bytes.

_Defined in `lang/std/ffi/libc.ks`._

## function `memmove`

```kestrel
public func memmove(consuming RawPointer, consuming RawPointer, consuming Int64) -> RawPointer
```

Wraps `memmove(3)` — copies `n` bytes from `src` to `dest`, allowing overlap.

Slightly slower than `memcpy` because it has to detect direction;
use `memcpy` when you know the regions are disjoint. Returns
`dest`.

### Safety

`src` and `dest` must each be valid for `n` bytes. Overlap is
permitted.

_Defined in `lang/std/ffi/libc.ks`._

## function `memset`

```kestrel
public func memset(consuming RawPointer, consuming Int64, consuming Int64) -> RawPointer
```

Wraps `memset(3)` — fills `n` bytes starting at `dest` with the low byte of `c`.

`c` is widened to `i64` to match the libc signature, but only
the low 8 bits are used; pass `0` to zero the region. Returns
`dest`.

### Safety

`dest` must be valid for `n` bytes of writes.

_Defined in `lang/std/ffi/libc.ks`._

## function `realloc`

```kestrel
public func realloc(consuming RawPointer, consuming Int64) -> RawPointer
```

Wraps `realloc(3)` — resizes a previously-`malloc`'d block.

May return the same pointer or a new one; either way, the original
pointer becomes invalid. Returns null on failure, in which case
the original block is **not** freed — capture the return value
before reassigning. Bytes beyond the old size in a grown block
are uninitialised.

### Safety

`ptr` must be null or a pointer from a previous `malloc` /
`realloc`. After a successful call, only the returned pointer is
valid; after a failed call, only the original pointer is valid.

### Examples

```
var buf = malloc(64);
let bigger = realloc(buf, 256);
// buf is no longer valid; use bigger
```

_Defined in `lang/std/ffi/libc.ks`._

