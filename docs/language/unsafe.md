# Unsafe

Some operations can break memory safety if they are used wrong: reading through a raw pointer, freeing a buffer, calling into C. Kestrel lets a declaration say so with `unsafe`, and makes every *use* of it sit inside an `unsafe` expression or block. The caller writes `unsafe` at the point where they take responsibility for the declaration's safety contract, so a reviewer can find every such point with a search.

## Declaring Something Unsafe

`unsafe` is a modifier on functions, initializers, subscripts, computed properties and protocols:

```kestrel
struct RawBuffer {
    private var base: Pointer[UInt8];

    /// Safety: `index` must be in bounds.
    public unsafe func read(at index: Int64) -> UInt8 {
        unsafe self.base.offset(by: index).read()
    }

    public unsafe init(base base: Pointer[UInt8]) {
        self.base = base;
    }

    public unsafe var first: UInt8 {
        get { unsafe self.base.read() }
    }
}
```

Document the safety contract on every unsafe declaration: what the caller must guarantee.

A stored field can't be `unsafe` (E645). Keep the field private and expose an `unsafe` computed property instead.

## Using Something Unsafe

A use is a call, a property read, an initializer, a subscript, or naming the declaration as a value. Every use must be inside an *unsafe context*, or it is an error (E640). There are two forms.

The **`unsafe` expression** covers one expression. It binds like `try`, so it covers the whole postfix chain after it:

```kestrel
let x = unsafe p.read();
unsafe p.write(42);
let r = unsafe &p.value;
```

The **`unsafe` block** covers several statements. Like `loop { … }`, it is a keyword-led block, not a closure, and its value is its last expression:

```kestrel
let sum = unsafe {
    let a = p.read();
    let b = p.offset(by: 1).read();
    a + b
};
```

Rules:

- The body of an `unsafe func` is **not** an unsafe context. Each unsafe operation inside it still needs its own `unsafe`, so the operations that need checking stay visible.
- The rule is lexical. A closure written inside an `unsafe` region is covered by it.
- An `unsafe` that covers no unsafe use is a warning (E641). A use marks only the innermost `unsafe` around it, so an `unsafe` that only wraps another `unsafe` is reported as well.

## What Is Unsafe

In the standard library:

- `Pointer` access: `read`, `take`, `write`, `assign`, `dropInPlace`, `value` and `mutatingValue`.
- `UnsafeSlice.init(pointer:count:)`.
- `Buffer.read(unchecked:)` and `Buffer.write(unchecked:)`.
- `Allocator.deallocate` and `Allocator.reallocate`.
- `Layout.ofValue`.

These `lang` intrinsics are unsafe: `ptr_read`, `ptr_take`, `ptr_write`, `ptr_ref`, `ptr_mut_ref`, `ptr_mut_borrow`, `drop_in_place`, `clone_val_into`, `sizeof_val` and `alignof_val`. Creating, offsetting, casting and comparing pointers is safe, because nothing is read or written.

### `@extern` functions

Every `@extern` function is unsafe to call, because the compiler can't check what the foreign code does. There is no opt-out. To offer a safe API over a C function, wrap it in a Kestrel function whose body checks the contract and uses `unsafe`:

```kestrel
@extern(.C, mangleName: "sqrt")
func libm_sqrt(x: lang.f64) -> lang.f64

extend Float64 {
    public func squareRoot() -> Float64 {
        Float64(raw: unsafe libm_sqrt(self.raw))
    }
}
```

## Unsafe Protocols

A protocol is `unsafe` when the compiler relies on its conformers to keep a promise it can't check. For example, `ArrayMatchable` must report indices that are really in bounds, because array patterns read elements without a bounds check.

```kestrel
public unsafe protocol ArrayMatchable { … }
```

A conformance to an unsafe protocol must be written `unsafe P` (E643). That marks the place where the author vouches for the contract:

```kestrel
struct Ring[T]: unsafe ArrayMatchable { … }
extend MyBuffer: unsafe ArrayMatchable { … }
```

Writing `unsafe P` for a protocol that isn't unsafe is a warning (E644). Using an unsafe protocol, as a bound or an existential, needs no `unsafe`.

### Unsafe requirements and witnesses

A protocol requirement can be `unsafe`; then callers must use `unsafe` whenever they call it, including through generic code. A requirement that is safe can't be satisfied by an `unsafe` witness (E642), because generic code would call it without `unsafe`. Either mark the requirement `unsafe` or make the witness safe.

## Diagnostics

| Code | Severity | Meaning |
|------|----------|---------|
| E640 | Error | An unsafe declaration is used outside an `unsafe` expression or block. |
| E641 | Warning | An `unsafe` covers no unsafe operation. |
| E642 | Error | An `unsafe` witness satisfies a safe requirement. |
| E643 | Error | A conformance to an unsafe protocol is missing `unsafe`. |
| E644 | Warning | `unsafe P` on a protocol that isn't unsafe. |
| E645 | Error | `unsafe` on a stored field. |

See [Error codes](../error-codes.md#e640e645--unsafe) for the messages.
