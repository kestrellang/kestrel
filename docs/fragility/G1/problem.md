# G1 — init bodies read `DropBehavior` before the pass that decides it has run

`high` · `silent memory leak` · crate: `kestrel-mir-lower` (`src/items/mod.rs`)

## Symptom

A struct field whose type is droppable but has **no `deinit` of its own** is
never released when an initializer overwrites it, or when a failable
initializer abandons a partially-built `self`. No diagnostic, no crash, clean
exit — the memory is just gone.

```kestrel
module Test

import std.numeric.Int64
import std.text.String

struct Holder: not Copyable {
    var s: String
    init(n n: Int64) {
        self.s = makeBig(n);        // allocates a buffer
        self.s = makeBig(n + 1);    // the first buffer must be released here
    }
}

func makeBig(n: Int64) -> String {
    var b = "";
    for i in 0..<64 { b = b + "0123456789abcdef"; }
    b + "\(n)"
}

@main
func main() -> lang.i64 {
    var total: Int64 = 0;
    for i in 0..<200 {
        let h = Holder(n: i);
        if h.s.isEmpty { total = total + 1; }
    }
    print("total = \(total)");
    0
}
```

```
$ leaks --atExit -- ./string_leak
Process 67700: 596 nodes malloced for 276 KB
Process 67700: 400 leaks for 265600 total leaked bytes.
```

The single-assignment control on the same program shape: `0 leaks for 0 total
leaked bytes`. The `init?`-that-`return null`s shape leaks the identical
400 / 265600.

## Measured, before → after

| shape | before | after |
|---|---|---|
| `String` field reassigned in `init`, ×200 | **400 leaks / 265600 bytes** | 0 leaks / 0 bytes |
| `String` field live at an `init?` failure return, ×200 | **400 leaks / 265600 bytes** | 0 leaks / 0 bytes |
| single-assignment `String` control | 0 leaks | 0 leaks |
| `Cloneable` wrapper reassigned, `deinit` counter | `1` (control: `2`) | `2` |
| `Cloneable` wrapper at `init?` failure, `deinit` counter | `0` (control: `1`) | `1` |
| default-`Copyable` wrapper reassigned, `deinit` counter | `1` (expected `2`) | `2` |

## Root cause — a reader that runs strictly before its writer

`lower_items` (`kestrel-mir-lower/src/items/mod.rs`) is two-pass, and its doc
comment claimed the split

> ensures all TypeInfo (CopyBehavior, DropBehavior) is available when function
> bodies are lowered

That is true for `CopyBehavior` and **false for `DropBehavior`**. The two halves
of `TypeInfo` are not symmetric:

- `lower_copy_behavior` asks `NominalCopySemantics`, which folds the whole type.
  Nothing fixes it up later — pass 1 is final.
- `lower_drop_behavior` reports **only a user `deinit`**. A struct with no
  `deinit` but a droppable field comes out of pass 1 as `DropBehavior::None`.
  The pass that promotes it, `drop_fix::fix_drop_behaviors`, had exactly one
  call site: `kestrel-mir/src/passes/mod.rs`, gated on `stop >= Stage::DropFix`
  — i.e. after `Stage::Raw`, after **all** of `lower_functions`.

Pass 2 calls `lower_function_sig` → `lower_function_body` → `setup_init_field_flags`
inline, and that function decides which `self` fields get a drop flag with
(`body/mod.rs`):

```rust
let droppable = kestrel_mir::ty_query::needs_drop(..., field_ty)
    || self.is_non_copyable(field_ty);
```

`needs_drop`'s `Named` arm is `type_info.drop != DropBehavior::None` — which
`fix_drop_behaviors` has not yet written. **The reader ran strictly before the
writer.** No flag was allocated, so:

- a second assignment lowered to `store_init` where it needed `store_assign`
  (only `StoreAssign` gets the destroy-old expansion in `mono/expand.rs`);
- a failable init's failure block was missing the entire guarded-destroy
  diamond (`field_addr` / `load` the flag / `branch` to `destroy_addr`) that the
  `deinit`-bearing control emits.

Both confirmed by diffing `kestrel dump mir -s verify` against the control.

The `is_non_copyable` disjunct never covered the gap: the leaking wrapper is
`Cloneable` (or plain default-`Copyable`) precisely *because* it has no `deinit`
— a `deinit` does not affect copy semantics.

## The new fixtures, verified against a pre-fix compiler

Built from an isolated worktree at `bf96598d` (pre-fix) and run:

| fixture | pre-fix | post-fix |
|---|---|---|
| `init_field_reassign_transitive_cloneable.ks` | exit **1** | exit 0 |
| `partial_drop_on_init_failure_transitive_cloneable.ks` | exit **2** | exit 0 |
| `init_field_reassign_default_copyable_field_drop.ks` | exit **1** | exit 0 |
| `partial_drop_on_init_failure_default_copyable_field_drop.ks` | exit **2** | exit 0 |
| `init_field_reassign_string_no_crash.ks` | exit 0, **400 leaks / 265600 B** | exit 0, 0 leaks |
| `partial_drop_on_init_failure_string_no_crash.ks` | exit 0, **200 leaks / 132800 B** | exit 0, 0 leaks |

The two `String` files exit 0 on both sides — nothing inside a Kestrel program
can observe a freed `String` buffer. They ship as **regression backstops** (each
says so at the top), not as leak assertions: they loop the shape 200× and assert
the surviving content is exactly the second assignment / the failing inits
return null, which is what catches a *double*-free once the fix arms the drop
path for a real stdlib `Cloneable` heap type. The four `deinit`-counter fixtures
are the ones that actually fail pre-fix.

## Why the test suite never saw it

All eight pre-existing fixtures under
`lib/kestrel-test-suite/testdata/memory_model/deinit/` that exercise init-field
drops declare `struct …: not Copyable` **and** give it a `deinit`. That
satisfies *both* disjuncts of the droppability test at once, so no fixture could
distinguish "droppable because `needs_drop` said so" from "droppable because
`is_non_copyable` said so". The suite was structurally blind to a wrapper that
is droppable but copyable.
