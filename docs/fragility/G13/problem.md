# G13 — a `where T: Copyable` clause was answered by two different mechanisms, and both were wrong

`high` · `false reject` + `silent miscompile` · crates: `kestrel-type-infer`
(`src/conformance.rs`), `kestrel-analyze`
(`src/compilation/conformance_completeness.rs`)

One clause spelling — `where T: Copyable` (or `Cloneable`) on an extension —
reached two different evaluators, and each got it wrong in the **opposite**
direction. Rejecting programs that are fine, and accepting programs that trap.

## Bug 1 — false reject (E454)

`conformance_completeness.rs:1642-1649` calls `type_satisfies` to decide whether
a constrained protocol extension supplies a witness. It was the **only**
unguarded call site: every other consumer of a `Copyable` bound skipped it, this
one did not.

```kestrel
module Test

import std.numeric.Int64

protocol Dup {
    func dup() -> Self
}

protocol Container[T] {
    func item() -> T
}

extend Container[T] where T: Copyable {
    public func dup() -> Self { self }
}

struct BoxC: Container[Int64] {
    var v: Int64;
    func item() -> Int64 { self.v }
}

extend BoxC: Dup { }

@main
func main() -> lang.i32 {
    let a = BoxC(v: 4);
    let b = a.dup();
    if b.item() == 4 { 0 } else { 1 }
}
```

```
$ kestrel build copyable_ext.ks -o copyable_ext
error[E454]: type 'BoxC' does not implement method 'dup' from protocol 'Dup'
   ┌─ copyable_ext.ks:22:1
   │
22 │ extend BoxC: Dup { }
   │ ^^^^^^^^^^^^^^^^^^^^ missing method 'dup'
```

`Int64` is Copyable. The bound holds. Three controls prove the analyzer was the
only component that disagreed:

| variant | before | after |
|---|---|---|
| `extend Container[T] where T: Copyable` | `error[E454] … missing method 'dup'` | exit 0 |
| `extend Container[T] where T: Equatable` (control) | exit 0 | exit 0 |
| same program, `Dup` conformance dropped, `a.dup()` called directly | exit 0 | exit 0 |
| `struct Box[U]: Container[U]` — param-to-param, not concrete | exit 0 | exit 0 |

The `Equatable` control compiling is the tell: the shape is fine, only the
protocol named in the bound mattered. The direct-call control compiling is the
second tell: the **solver** happily routes `a.dup()` through that same
extension. Two components, one question, two answers.

### Why

`type_satisfies` dispatched `Copyable` into its `match ty`, whose arms all route
to `nominal_satisfies` → `ConformingProtocols`. `ConformingProtocols` only
materializes *explicit* conformances plus inheritance, so it never reports the
implicit `Copyable` that every default type carries — `BoxC` does not write
`: Copyable`, so the query says no. The param-to-param variant escaped because
it never reaches `type_satisfies` at all: `conformance_completeness` routes
those through `constraint_entailed_by`, which matches clauses by protocol
`Entity` and never asks about declarations.

The `HirTy::Struct` shape is what the repro actually hits, which is why fixing
this inside the `match` would have needed a per-arm patch rather than one arm.

## Bug 2 — unsound accept, live in the shipped stdlib

`conformance.rs:295-300`, inside `extension_bounds_hold_impl`:

```rust
// Copyable / Cloneable are copy-semantics, not declared conformances;
// `type_satisfies` (which goes through `ConformingProtocols`) can't
// answer them. Skip — copyability is enforced by the move checker / mono.
if is_copy_builtin(ctx, *pb, root) {
    continue;
}
```

`continue` means the clause gates **nothing**. A `where T: Copyable` bound on an
extension did not participate in member selection at all. This is not
hypothetical — two pieces of public stdlib surface depend on it for memory
safety.

### `RcBox[T].getValue()`

`lang/std/memory/rcbox.ks:180-183` puts `getValue` and `deepClone` on a
constrained extension precisely because they bitwise-copy the payload out of
heap storage, and says so:

> The two operations that copy the payload OUT of storage. They need
> `T: Copyable` and therefore cannot sit in the struct body, which is relaxed to
> `T: not Copyable`. […] only a box over a non-Copyable payload loses these two
> methods.

It did not lose them.

```kestrel
module Test
import std.numeric.Int64
import std.memory.RcBox
struct NC: not Copyable { var v: Int64 }
@main
func main() -> lang.i32 {
    let b = RcBox[NC](NC(v: 5));
    let got = b.getValue();   // gated `extend RcBox[T] where T: Copyable`
    if got.v == 5 { 0 } else { 1 }
}
```

```
$ kestrel build rcbox_nc_probe.ks -o rcbox_nc_probe   # no diagnostics
$ ./rcbox_nc_probe ; echo $?
132
```

Exit **132** is SIGILL. It compiled clean and trapped.

### `Pointer[T].pointee`

`lang/std/memory/pointer.ks:351` — same shape, getter is a raw `lang.ptr_read`.

```kestrel
let p = Pointer[NC].nullPointer();
let got = p.pointee;
```

```
$ kestrel build stdlib_pointer_nc.ks -o stdlib_pointer_nc   # no diagnostics
$ ./stdlib_pointer_nc ; echo $?
132
```

### Control

The identical shape with `where T: Equatable` rejects cleanly, which is what
`Copyable` should always have done:

```
error[E100]: no member … on type …
```

## After

```
$ kestrel build rcbox_nc_probe.ks -o rcbox_nc_probe
error[E100]: no member 'getValue' on type 'RcBox[NC]'
  ┌─ rcbox_nc_probe.ks:8:15
  │
8 │     let got = b.getValue();
  │               ^^^^^^^^^^^^ no member 'getValue' on type 'RcBox[NC]'

$ kestrel build stdlib_pointer_nc.ks -o stdlib_pointer_nc
error[E100]: no member 'pointee' on type 'Pointer[NC]'
   ┌─ stdlib_pointer_nc.ks:10:17
   │
10 │     let got = p.pointee;
   │                 ^^^^^^^ no member 'pointee' on type 'Pointer[NC]'
```

`no member` rather than `does not conform` is the right shape: once the bound is
enforced the extension does not apply, so the member was never a candidate —
exactly what the `Equatable` control already produced.

## Not this bug

`memory_model/copy_semantics/subscript_read_notcopyable_traps.ks`
(`expect-exit: -1`) covers `Pointer.read()`, whose bound is **method-level** and
never routes through `extension_bounds_hold`. Separately tracked; unaffected by
this fix.

Reaching a `where T: Copyable`-gated member through a *generic protocol bound*
still fails at mono — `type_conforms_at_mono`
(`kestrel-mir/src/mono/witness.rs`) evaluates the constraint by searching the
witness table for a `Copyable` witness, which structurally never exists. That is
mono's own independent answer to the same question, on a code path this fix does
not touch, and the `Equatable` spelling of the same program works. Also
separately tracked; see `decisions.md` §4.
