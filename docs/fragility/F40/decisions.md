# F40 — decisions

## 1. Point fix now; the duplication that caused it is filed, not bundled

**Decision:** change cranelift's `classify_named` to LLVM's nested form and stop
there. Do **not** unify the two `TypeRepr` implementations in this change.

F40 is a live silent-miscompile on the default backend, reachable from shipped
stdlib code (`IoError` on every `Result[T, IoError]`). The fix is ten lines and
its blast radius is one branch of one function. The unification is a
cross-crate refactor touching two backends' type layers at once — a strictly
larger diff, with a strictly larger chance of introducing a *second* codegen bug,
gating a corruption fix behind it.

The two are also independent: unifying without the fix would have to pick which
implementation to keep (and picking cranelift's would spread the bug to LLVM);
fixing first means unification has one correct implementation to lift.

### The filed follow-up: move `TypeRepr` + `classify` into `kestrel-mir`

`ScalarTy` / `ir::Type`, `TypeRepr`, `classify`, and `classify_named` exist twice
— ~170 duplicated lines across `lib/kestrel-codegen-cranelift/src/ty.rs` and
`lib/kestrel-codegen-llvm/src/ty.rs`, structurally parallel down to the comment
prose. That is the actual defect; F40 was one drift between the copies, and the
next drift will be a different one.

**Home: a new `lib/kestrel-mir/src/repr.rs`.**

* Both backends already depend on `kestrel-mir`, and both `classify` functions
  read *only* `kestrel-mir` inputs — `MirTy`, `TyArena`, `TyId`, `Layout`,
  `StructLayout`, `MonoModule`, `func_thick_words`, `IntBits`, `FloatBits`. There
  is no new dependency edge to add.
* `kestrel-codegen` **cannot** host it: `kestrel-mir` depends on
  `kestrel-codegen`, so putting it there would cycle.
* The backend-specific part is the leaf scalar type only. Parameterise
  `TypeRepr` over the backend's scalar (or return a neutral `ScalarKind` that
  each backend maps to `ir::Type` / its LLVM equivalent at the boundary). Every
  structural decision — the size-0 → `Zst` test, the tuple/`Str`/`FuncThick`
  sizing, the single-field collapse and its `size <= 8` bound — is
  backend-independent and moves wholesale.

After the move, a divergence like F40 stops being *possible* rather than merely
being fixed.

## 2. The `Scalar`-delegation branch is untouched, on purpose

The fix **adds** an arm; it does not modify the existing one. When a one-field
struct's field is itself `Scalar`, the newtype keeps the field's exact scalar
type — an `f64` newtype stays `f64`, a `Pointer[T]` newtype stays `ptr`.

That branch is load-bearing history: collapsing `Float64` to I64-by-size once
made the auto-synthesized clone-shim's *signature* disagree with its *body*, and
cranelift's verifier rejected the function. Anyone reading the fix might
reasonably think "the whole collapse is suspect, delete it" — it isn't, and
`testdata/codegen/structs/newtype_over_scalar_field_control.ks` now fails loudly
if it is deleted.

Corollary for debugging: a clone-shim verifier error appearing after this change
is **not** F40 resurfacing. `KESTREL_DEBUG_CLONE=1` prints the `[classify_named]`
traces from `ty.rs`.

## 3. No other cranelift site needed to move — verified, not assumed

The classification change makes some values `Aggregate` that were `Scalar`. Every
consumer was re-read to confirm it handles the new answer correctly:

* **`compile_struct`'s `Aggregate` arm** loops over `fields` computing each
  offset. It is field-count-agnostic; a 1-field struct is just a loop of length
  one writing at offset 0. No special case needed.
* **`compile_struct_extract`**: the `(Scalar, Scalar)` fast path stops matching
  and control reaches the offset+load branch, where `offset == 0` and
  `mem::load_from_repr(Aggregate, addr, …)` returns `addr` unchanged. For a
  1-field struct the struct's address *is* its field's address — correct.
* **`mem::store_to_repr`'s `Aggregate` arm** is purely size-driven
  (`copy_aggregate(size, …)`); `mem::load_from_repr`'s is the identity above.
* **`abi.rs`** — `param_pass_mode`, `return_mode`, `build_signature`,
  `build_extern_signature` all match on `TypeRepr` alone with no field
  introspection. `Aggregate` uniformly means ByRef / Sret.

The behaviour change is therefore entirely "this type now travels by address",
which is what the other three quarters of the type system already did.

## 3b. A guard goes where the wrong value is minted — and is proven by running it

The inert `compile_struct_extract` guard was first rewritten in place (an
exhaustive `match` instead of a non-matching `if let`). Building a debug compiler
and running the repro showed it **still** never fired: `w.kind` lowers to
`field_addr` + `copy_value`, not `StructExtract`, so that function is not on the
path at all. Fixing the pattern would have produced a second guard that reads
like coverage and provides none.

The assertion therefore lives in `compile_struct`'s `Scalar` + one-field branch —
the single line that mints the bad value — and it was **verified by running**: a
debug compiler with the new guard and the OLD `ty.rs` aborts with
`scalar-repr struct over an aggregate field … (F40)` on both functions of the
repro; the same compiler with the fixed `ty.rs` is silent. `compile_struct_extract`
keeps a cheap consistency assert, moved above its `is_borrowed` early return.

Rule this generalises to: **a guard is not verified until it has been observed
firing on the bug it names.** Both F40 guards read plausibly and neither could
fire; the only way to tell was to reintroduce the defect under a debug build.

Caveat for whoever tries to repeat this: at the time of writing, a **debug**
build of the compiler cannot compile any stdlib program at all — a pre-existing
`debug_assert!` in `lib/kestrel-mir/src/passes/thunk.rs:210` fires on
`std.collections.Array.subscript.closure.3.thunk` for every input (reproduced on
a clean `HEAD` worktree, so it is not F40's and not a work-in-progress artifact).
The verification above had to use `--no-std` programs. Release builds — which is
what `triage` runs — are unaffected.

## 4. Tests are cross-backend execution tests, and they cover the *cross-frame*
shapes specifically

Every new file carries `// backends: cranelift,llvm`. A same-backend expectation
cannot catch a divergence; the whole bug class is "the backends disagree", so the
test must be the comparison.

The shapes were chosen against how the bug hid, not just how it manifests:

| file | what it pins |
| --- | --- |
| `newtype_over_payload_enum_struct_field.ks` | newtype as a field of another struct, built in a callee |
| `newtype_over_payload_enum_in_array.ks` | heap storage, incl. read-back after a realloc |
| `newtype_over_payload_enum_through_generic.ks` | ByVal/Direct ABI decision via `ident[T]` |
| `newtype_over_payload_enum_in_optional.ks` | enum-payload packing of the newtype |
| `newtype_over_optional_int32.ks` | `Slot { o: Optional[Int32] }` — the `OptionalIterator[T]`/`ResultIterator[T]` shape at small `T` |
| `newtype_over_payload_enum_return_and_alias.ks` | two/three live values from one callee — the stack-slot aliasing witness |
| `newtype_over_aggregate_size_gt_8_control.ks` | NEGATIVE: `Optional[Int64]`, size 16, always agreed — pins the `size <= 8` boundary |
| `newtype_over_scalar_field_control.ks` | the delegation branch of §2 |
| `pair_two_field_control.ks` | 2+-field structs are unaffected |
| `stdlib/io/io_error_cross_frame_repr.ks` | the shipped type, across frames and in an `Array` |

`testdata/stdlib/io/io_error_types.ks` was **left alone**. It exercises only the
construct-and-consume-in-one-frame shape — the narrow case that always worked —
and keeping it distinct records exactly which coverage existed before and why it
was insufficient.

## 5. Severity corrected: `medium` → `high`

The audit filed F40 as `medium` `single-source-of-truth`, which reads it as a
duplication smell. It is that, but the duplication has already produced a **live
silent data corruption on the default backend, in shipped stdlib code, with no
diagnostic**. Measured effects: wrong values, wrong match arms taken, and a
`SIGBUS` (exit 138). Anything that silently produces wrong bytes at runtime is
`high` regardless of how tidy the underlying cause is.

## 6. Separately filed: aggregate-by-value across `@extern(.C)` is broken in
BOTH backends — independent of F40

While verifying that F40 could not corrupt an FFI boundary, a **different,
pre-existing** bug was measured. It is not caused by F40 and is not fixed by this
change; filing it here so the evidence is not lost.

A plain **two-field** `FFISafe` struct passed by value to a C function returns
garbage under **both** backends, independently:

```kestrel
public struct Inner: FFISafe { public var a: Int32  public var b: Int32 }
@extern(.C) func f40_take_inner(i: Inner) -> Int32     // C: return i.a*10 + i.b
```

```
plain aggregate  -> 875371297   (cranelift)      expect 34
plain aggregate  -> 1428004377  (llvm)           expect 34
plain scalar     -> 70          (both)           expect 70   ← scalars are fine
```

Two backends producing two *different* wrong answers is the signature of neither
one implementing the platform ABI. `abi.rs::build_extern_signature` passes every
`TypeRepr::Aggregate` as a bare `ptr_ty` parameter — that is Kestrel's internal
"manual ABI", **not** AAPCS64 (which would decompose a 8-byte two-int struct into
registers, or pass ≤16-byte aggregates in `x0`/`x1`) and not SysV. Closing it
means implementing real aggregate classification per target triple, which is a
project of its own.

Also measured, and relevant to the F40 blast radius: **`enum`s cannot conform to
`FFISafe`** — `E422 'FFISafe' only allows struct conformance`. So `IoError`, a
newtype over an enum, can never cross an `@extern` boundary at all, and F40's
corruption was confined to Kestrel-internal calls. That containment is luck, not
design.
