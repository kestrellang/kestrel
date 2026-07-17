# Kestrel Internal ABI (MIR)

This document describes the internal, backend-agnostic ABI at the boundary
between MIR lowering and codegen. It is not a stable external ABI. Backends
may choose their own target calling conventions so long as they preserve the
layouts and ownership semantics below.

For the MIR ownership model itself — instruction contracts, operand/result
conventions, borrow scoping, the verifier — see
[docs/contributing/mir-ownership-spec.md](../contributing/mir-ownership-spec.md).
This document only summarizes what codegen needs.

## Scope

- Applies to MIR types (`MirTy`) and call argument passing (`ParamConvention`).
- Layouts are expressed in terms of `TargetConfig` pointer size.
- Type parameters, `Self`, and associated types are resolved at
  monomorphization; codegen only ever sees fully concrete types.

## Ownership Model (OSSA)

MIR is in **ownership SSA** form. Every instruction operand and result is a
`ValueId` carrying an ownership state:

- `@owned` — the value is owned by the current SSA name and must be consumed
  exactly once (by a `Consume`-convention operand: a move, a store, a
  destroy, or a consuming call argument).
- `@guaranteed` — a borrowed view (`BeginBorrow`/`BeginMutBorrow` results,
  `StructExtract`/`TupleExtract`/`EnumPayload` projections, ref-typed call
  results). Valid until the matching `EndBorrow`; never destroyed.
- Stack **addresses** (`Uninit`, `FieldAddr`) — memory slots for mutable
  locals and in-place initialization (`StoreInit`/`StoreAssign`/`Take`).

Every `InstKind` declares what it expects of each operand (`Read`, `Consume`,
`Freeze`, `Addr`) and what it produces (`Owned`, `Guaranteed`, `Addr`, `Void`,
`MultiOwned`) — the contract table lives in the ownership spec, and the MIR
verifier enforces it (consume-exactly-once, no use of frozen values, address
init-state transitions). Copy/clone decisions are made **in MIR lowering**:
`CopyValue` duplicates Copyable values, Cloneable copies are emitted as calls
to the `clone()` witness, and non-Copyable values only ever flow through
`MoveValue`/`Take`. By the time codegen runs, there are no implicit copies
left to decide.

## Call Argument Passing (`ParamConvention`)

Each function parameter (including the method receiver, which is
`params[0]`) carries one of three conventions:

| Convention | Source syntax | Caller side | Callee side |
|------------|--------------|-------------|-------------|
| `Borrow` | (default) | passes `@guaranteed` (BeginBorrow around the call) | read-only view |
| `MutBorrow` | `mutating` | passes the address / exclusive view | reads and writes in place |
| `Consuming` | `consuming` | passes `@owned`; caller's value is gone | owns it; drops or forwards |

The caller adapts what it *has* to what the convention *needs*: an owned
value passed to `Borrow` is borrowed for the duration of the call; a borrowed
value passed to `Consuming` is copied (Copyable), cloned via witness call
(Cloneable), or is a compile-time move error (NotCopyable — caught by the
move checker before MIR verification).

Backends may realize `Borrow`/`MutBorrow` however they like (typically: by
value in registers for small scalars, by pointer for aggregates and all
`MutBorrow`s), provided writes through `MutBorrow` land in the caller's
storage and `Consuming` transfers responsibility for the drop.

## Returns

Return values are `@owned` in MIR. Backends choose by-value vs. hidden sret
passing, but must preserve the layouts below.

**Reference returns** (`-> &T` / `-> &mutating T`) use `MirTy::Ref` — a
*signature-only* type: it appears on `FunctionDef.ret`/`MonoFunction.ret`,
never as a `ValueDef` type. The callee returns a raw address (pointer-sized);
the caller registers the result as an ordinary `@guaranteed` value of the
pointee type — "a borrowed parameter that travels". Which addresses may be
returned is decided earlier by the escape checker (E494 family; see
[diagnostics.md](diagnostics.md)); codegen just passes the pointer.

## Type Layout (Size + Alignment)

Pointer size is `ptr_size` (4 or 8 bytes depending on target).

### Primitives

| Type | Size | Align |
|------|------|-------|
| `i8` / `bool` | 1 | 1 |
| `i16` / `f16` | 2 | 2 |
| `i32` / `f32` | 4 | 4 |
| `i64` / `f64` | 8 | 8 |
| `!` (Never) | 0 | 1 |
| `()` (empty tuple) | 0 | 1 |

Backends may materialize zero-sized values as a single byte, but the logical
layout is size 0, align 1.

### Pointers, References, Strings, Functions

| Type | Size | Align |
|------|------|-------|
| `Pointer(T)` (raw pointer) | `ptr_size` | `ptr_size` |
| `Ref` (`&T`, `&mutating T`) | `ptr_size` | `ptr_size` |
| `str` (fat pointer `{ ptr, len }`) | `2 * ptr_size` | `ptr_size` |
| `FuncThin` (bare function pointer) | `ptr_size` | `ptr_size` |
| `FuncThick` (closure `{ func_ptr, env_ptr }`) | `2 * ptr_size` | `ptr_size` |

References carry no metadata — they are raw addresses in MIR. Closure
environments are currently stack-allocated in the creating frame (which is
why capturing closures cannot escape it — E494).

### Structs and Tuples

Struct fields and tuple elements are laid out in declaration order:

1. Start with `(size=0, align=1)`.
2. For each field: offset = round_up(current size, field align);
   `size = offset + field_size`; `align = max(align, field_align)`.
3. After the last field, pad size up to `align`.

Example (64-bit):

```
struct Pair { a: i8, b: i64 }
// a @ 0, b @ 8, size = 16, align = 8
```

### Enums

Enums are tagged unions:

- **Discriminant** at offset 0. Its width is chosen from the variant count:
  `i8` up to 256 variants, `i16` up to 65,536, `i32` beyond.
- **Payload** at the first offset after the discriminant that satisfies the
  maximum payload alignment; sized to the largest case payload.
- Total size padded to the overall alignment
  (`max(discriminant, payloads)`).

Case payloads are laid out as case-specific structs using the struct rules
above; empty cases have zero-sized payloads.

Note: drop order within a dropped value is **reverse declaration order** for
struct fields and enum payload fields (see
[drop-semantics.md](drop-semantics.md)); this is a semantic rule emitted by
MIR drop elaboration, not a layout property.

## Backend-Specific Notes

Backends may pass compound types indirectly or add target-specific ABI rules,
but should treat this document as the source of truth for layout, and the
MIR ownership spec as the source of truth for ownership semantics at the MIR
boundary. Codegen failures are a hard build error (no binary is written) —
codegen must never paper over a malformed function with a trap stub.
