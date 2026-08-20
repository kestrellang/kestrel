# F40 — the two backends' `classify_named` disagree on a newtype over an aggregate field

`high` (audit filed it `medium`; corrected — see `decisions.md` §5) ·
`single-source-of-truth` · crates: `kestrel-codegen-cranelift`,
`kestrel-codegen-llvm`

## Symptom

A one-field struct whose field is itself carried by memory — the canonical case
is a payload-carrying enum, e.g. `Optional[Int32]` or the stdlib's
`IoErrorKind` — **silently miscompiles on the default backend**. No diagnostic,
no ICE, no verifier complaint: the program runs and prints wrong numbers, or
segfaults if the reused frame has since been clobbered.

```kestrel
public enum Kind { case A case B case C case Other(code: Int32) }
public struct Wrap { public var kind: Kind }          // ONE field, layout 8 bytes

func k(kind: Kind) -> Int64 { match kind { .A => 1, .B => 2, .C => 3, .Other(c) => Int64(from: c) } }
func makeWrap(code: Int32) -> Wrap { Wrap(kind: Kind.Other(code: code)) }
```

Real output, measured, `kestrel build` at `-O0`:

| program / shape | cranelift (default) | llvm | expected |
| --- | --- | --- | --- |
| `Wrap` (newtype over payload enum), two in an `Array` | `1 1` | `11 22` | `11 22` |
| `Pair` (same field + a second field — control) | `11 22` | `11 22` | `11 22` |
| `WrapInt` (newtype over a **scalar** field — control) | `11 22` | `11 22` | `11 22` |
| `Wrap` as a struct field (`Holder.w`) | `1 1` | `11 22` | `11 22` |
| `Wrap` through `ident[T](v: T) -> T` | `1` | `33` | `33` |
| `Wrap` in `Optional[Wrap]`, `.Some` arm | `777` (the `.A` arm!) | `44` | `44` |
| `Slot { o: Optional[Int32] }` as a field | `999 999` (`.None` arm) | `11 22` | `11 22` |
| `Slot { o: Optional[Int32] }` in an `Array` | `999 999`, **exit 138** | `33 44` | `33 44` |
| `Slot64 { o: Optional[Int64] }`, size 16 (control) | `11 22` | `11 22` | `11 22` |
| **`IoError` (SHIPPED stdlib)**, two in an `Array` | `errno = 1 1` | `2 77` | `2 77` |

Exit 138 is `SIGBUS` — the address-shaped "value" was dereferenced after its
frame died. That is the *lucky* outcome; the rows above it are silent data
corruption.

## Root cause

`lib/kestrel-codegen-cranelift/src/ty.rs` `classify_named` flattened its
single-field collapse into one `&&`-joined `if let` chain:

```rust
if let Some(field_ty) = single_field_ty
    && let TypeRepr::Scalar(t) = self.repr(field_ty, arena, module)
{
    return TypeRepr::Scalar(t);
}
let cl_ty = match size { 1 => I8, 2 => I16, 3..=4 => I32, _ => I64 };
return TypeRepr::Scalar(cl_ty);
```

When the field's repr is **not** `Scalar`, the second `let` fails, the whole
chain is skipped, and control falls into the integer-by-size mapping — which
answers `Scalar(I64)` for an 8-byte struct. The field's classification, already
computed, is **discarded**.

LLVM (`lib/kestrel-codegen-llvm/src/ty.rs:275-302`) nests the two `if let`s, so
the inner failure returns `TypeRepr::Aggregate { size, align }` from the outer
arm. Same intent, different control flow, opposite answer.

### Divergence condition

All four must hold:

1. `MirTy::Named` present in `module.structs` (a struct, not an enum),
2. exactly one field,
3. layout size in `1..=8`,
4. `repr(field.ty)` is not `Scalar` — i.e. the field is an aggregate.

(3) plus (4) means the field is a small aggregate: a payload-carrying enum in
≤ 8 bytes, a small tuple, or a small struct.

### How a wrong classification becomes wrong bytes

`compile_struct` (`cranelift/inst.rs:947-968`) branches on the repr:

```rust
TypeRepr::Scalar(t) => {
    if fields.len() == 1 {
        return Ok(resolve_slot_value(fc, builder, field_ty, fields[0].1));
    }
    …
}
```

For an aggregate field, `resolve_slot_value` yields the field's **stack-slot
address**. So the constructed "value" of a `Wrap` is a pointer — carried in an
`i64` because the repr says `Scalar(I64)`. From there every consumer that copies
by `TypeRepr` (`mem::store_to_repr`, `mem::copy_aggregate`, array element
writes, `Optional` payload packing, `abi::param_pass_mode` / `return_mode`)
moves **8 bytes of address instead of 8 bytes of contents**.

Three consequences follow, all observed above:

* **Aliasing.** Two calls to the same constructing function return the same
  reused slot address, so the second silently rewrites the first result.
* **Dangling.** The address outlives the frame that owns the slot — hence the
  `.None`/`.A` arms being taken (a stale tag byte) and the `SIGBUS`.
* **Type-punned reads.** An `Optional[Wrap]` payload holds a pointer where the
  match arm expects the enum's tag + payload.

### Why it shipped

`IoError` — a newtype over `IoErrorKind` — is the error type of every
`Result[T, IoError]` in `Read`, `Write`, `File`, `stdio`, and `os.fs`. The bug
was live in shipped stdlib code the whole time. Both existing `IoError` tests
(`testdata/stdlib/io/io_error_types.ks`, `io_error_formattable.ks`) construct
and consume a single instance **inside one frame** — the one shape where the
address is still valid and nothing else has reused the slot. The stdlib's
`OptionalIterator[T]` / `ResultIterator[T]` at small `T` are the same shape.

The comment above the branch asserted the superseded rule verbatim ("Pure-
discriminant enums and one-field structs over a non-scalar field fall through to
the integer-by-size mapping below"), so code and comment agreed — and were both
wrong. Nothing in the file pointed at LLVM as the disagreeing party.

## Why both guards were inert

Two debug-only checks existed that read like coverage for exactly this class of
bug. Neither could fire.

### 1. `inst.rs` `compile_struct_extract` — the pattern that falls through

```rust
if let (TypeRepr::Scalar(base_cl), TypeRepr::Scalar(field_cl)) = (operand_repr, field_repr) {
    debug_assert_eq!(base_cl, field_cl,
        "single-field newtype repr must equal its field repr (classify_named delegates)");
    return Ok(base);
}
```

Its own comment says "if it ever fires, a layout authority has diverged from
`classify_named` again". But an F40 divergence produces `(Scalar, Aggregate)` —
which the `if let` pattern simply **does not match**. Control falls silently into
the offset+load path below, where `offset == 0` and
`mem::load_from_repr(Aggregate, …)` returns the address unchanged. That is why
the corruption produced *plausible* wrong answers instead of an assertion: the
guard's shape guaranteed it would look away from precisely the case it named.

An `if let` used as an assertion is a no-op on every input that does not match.

There is a second reason this site could never have caught F40, found only by
building a debug compiler and watching the guard stay silent on a known-bad
program: **field access does not lower to `StructExtract`**. The MIR for
`w.kind` is `field_addr` + `begin_borrow_addr` + `copy_value` (confirmed with
`kestrel dump mir`), so `compile_struct_extract` is not even on the path for the
shapes that miscompile.

The guard was therefore moved to where the wrong value is actually **minted** —
`compile_struct`'s `Scalar` + one-field branch, the line that returns
`resolve_slot_value(field_ty)`. If the struct's repr is `Scalar` while the
field's is `Aggregate`, that return value *is* a stack-slot address. Verified
with teeth: a debug compiler carrying the new guard but the **old** `ty.rs`
aborts on the repro with

```
thread 'main' panicked at lib/kestrel-codegen-cranelift/src/inst.rs:975:17:
scalar-repr struct over an aggregate field: the constructed value would be a
stack-slot address (F40)
error: 2 function(s) failed to compile: Main.mk, Main.main
```

and the same compiler with the fixed `ty.rs` is silent and returns 0.
`compile_struct_extract` keeps a cheap consistency `debug_assert!`, now placed
**above** the `is_borrowed` early return so it is at least reachable on both
ownership paths.

### 2. `func.rs` `verify_value_repr` — blind by construction

```rust
if cl_ty == ptr_ty && expected_scalar != ptr_ty { eprintln!("VERIFY: …") }
```

Two independent reasons it can never catch F40:

* **It is self-consistent, not correct.** It compares a value against
  `classify_named`'s own answer. When classification is the thing that is wrong,
  the value agrees with it perfectly. Every F40 value satisfied this check.
* **`ptr_ty == I64` on a 64-bit target.** `expected_scalar != ptr_ty` is false
  for every I64-sized scalar, so a smuggled pointer and a genuine `Int64` are
  indistinguishable here even in principle.

This one was **not** made into a general pointer-leak detector — it cannot become
one. It gained the single honest check available to it (an `Aggregate` value's
cranelift IR type must be `ptr_ty`), and its doc comment now states the
self-consistency-only limitation, so a future reader does not mistake it for
classification coverage. Cross-backend execution tests are what actually cover
this axis; see `decisions.md` §3.
