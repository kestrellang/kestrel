# FFI

Kestrel can call C, and C can call Kestrel. The interface is **C ABI** — there's no automatic binding to C++ classes or other higher-level constructs, but anything you can `extern "C"` from C, you can talk to.

## Calling C

Declare an external function with `@extern(.C)` and a body-less signature:

```swift
@extern(.C)
func malloc(size: Int) -> Ptr

@extern(.C)
func free(ptr: Ptr)

@extern(.C)
func write(fd: Int, buf: Ptr, count: Int) -> Int
```

Kestrel emits no body — the linker resolves the symbol against libc, or whatever object file you're linking. Call them like ordinary functions:

```swift
let buf = malloc(size: 1024)
defer { free(ptr: buf) }
```

## Exporting to C

Going the other direction, expose a Kestrel function to C with `@export(.C)`:

```swift
@export(.C)
public func add(a: Int, b: Int) -> Int {
    a + b
}
```

The compiler emits a symbol named `add` with C calling convention, callable from C as:

```c
extern int64_t add(int64_t a, int64_t b);
```

## FFI-Safe Types

Not every Kestrel type can cross the C boundary. Things that *can*:

- Primitive numeric types (`Int`, `Float`, `Bool`, `Char`)
- Raw pointers (`Ptr`, `OpaquePtr`)
- Structs whose fields are themselves FFI-safe and that conform to the `FFISafe` protocol
- Function pointers with FFI-safe argument and return types

Things that *can't*: generics, protocols-as-existentials, closures with captures, enums with payloads, `Optional[T]` (use `Ptr` and check for null).

You make a struct FFI-safe by conforming it to `FFISafe` and ensuring its layout is plain:

```swift
struct Vec3: FFISafe {
    var x: Float
    var y: Float
    var z: Float
}
```

The protocol carries no requirements — it's a marker the compiler uses to verify layout and prevent you from accidentally passing a non-FFI-safe type across the boundary.

## When to reach for FFI

The usual cases: calling system libraries (libc, OpenGL, SQLite), wrapping a C library you already have, exposing Kestrel as a plug-in target for a host program written in another language. For pure Kestrel-to-Kestrel code, stay on the Kestrel side — protocols and generics give you everything FFI doesn't.

---

[← Organization](organization.md) · [↑ The Kestrel Language](index.md) · [Concepts →](concepts/index.md)
