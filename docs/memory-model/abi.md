# Kestrel Internal ABI (MIR)

This document describes the internal, backend-agnostic ABI used between MIR
lowering and codegen. It is not a stable external ABI. Backends may choose
their own target calling conventions so long as they preserve the layouts and
ownership semantics below.

## Scope

- Applies to MIR types (`MirTy`) and call argument passing (`PassingMode`).
- Layouts are expressed in terms of `TargetConfig` pointer size.
- Type parameters, `Self`, and associated types are resolved at monomorphization;
  until then they are treated as opaque pointer-sized values.

## Type Layout (Size + Alignment)

Pointer size is `ptr_size` (4 or 8 bytes depending on target).

### Primitives

| Type | Size | Align |
|------|------|-------|
| `i8` | 1 | 1 |
| `i16` | 2 | 2 |
| `i32` | 4 | 4 |
| `i64` | 8 | 8 |
| `f16` | 2 | 2 |
| `f32` | 4 | 4 |
| `f64` | 8 | 8 |
| `bool` | 1 | 1 |
| `()` (Unit) | 0 | 1 |
| `!` (Never) | 0 | 1 |
| `<error>` | 0 | 1 |

Note: Backends may materialize zero-sized values as a single byte for codegen,
but the logical layout is size 0, align 1.

### Pointers and References

- `p[T]`, `&T`, and `&var T` are all pointer-sized (`ptr_size`, align `ptr_size`).
- References have no extra metadata; they are raw addresses in MIR.

### Strings

`str` is a fat pointer: `{ ptr, len }`.

- Size: `2 * ptr_size`
- Alignment: `ptr_size`

### Functions and Closures

- `func(...) -> ...` (thin): pointer-sized (function pointer).
- `func escaping(...) -> ...` (thick): two words `{ func_ptr, env_ptr }`.

### Named Types

- Structs and enums use the layouts described below.
- Protocol/opaque named types fall back to pointer-sized layout.

## Struct and Tuple Layout

Struct fields and tuple elements are laid out in declaration order.

Algorithm:
1. Start with `(size=0, align=1)`.
2. For each field, compute its layout `(field_size, field_align)`.
3. Field offset = round_up(current_size, field_align).
4. Update size and alignment:
   - `size = offset + field_size`
   - `align = max(align, field_align)`
5. After the last field, pad size up to `align`.

Example (64-bit):

```
struct Pair { a: i8, b: i64 }
// a @ 0, b @ 8, size = 16, align = 8
```

Tuples follow the same rules.

## Enum Layout

Enums are tagged unions:

- Discriminant: `i32` at offset 0.
- Payload: max of all case payload layouts.

Layout = `discriminant` appended with `max_payload`, then padded to alignment.

Payloads are represented as case-specific structs, so they use the struct layout
rules above. Empty cases use a zero-sized payload.

## Reference vs Value Representation

Kestrel has no user-facing reference types, but MIR introduces them for ABI:

Access mode lowering:
- `borrow`   -> parameter type `&T`
- `mutating` -> parameter type `&var T`
- `consuming` -> parameter type `T`

`Rvalue::Ref` and `Rvalue::RefMut` create references when needed.

## Call Argument Passing (`PassingMode`)

`PassingMode` describes how the argument value is passed:

- `Ref` / `MutRef`: callee receives a reference value.
- `Copy`: bitwise copy, caller retains ownership.
- `Move`: ownership transfers, caller invalidated.

Mapping for `consuming` parameters:
- Copyable -> `Copy`
- Cloneable but not Copyable -> emit `Cloneable.clone`, then `Move`
- Otherwise -> `Move`

For `borrow` and `mutating` parameters, lowering typically creates a reference
value and passes it with `Copy` (copying the pointer). Some call sites pass an
existing reference with `Ref`/`MutRef`; both represent the same semantics.

## Returns

Return types are plain MIR types. Backends choose by-value vs. hidden sret
passing, but must preserve the layouts above.

## Backend-Specific Notes

Backends may pass compound types indirectly or add target-specific ABI rules,
but should treat this document as the source of truth for layout and ownership
semantics at the MIR boundary.
