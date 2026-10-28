# G3 — the thunk pass decided "is this an environment pointer?" by parameter NAME

`high` · `silent miscompile` · crate: `kestrel-mir` (`src/passes/thunk.rs`)

## Symptom

A named function whose first parameter happens to be called `env` returns the
wrong answer — silently, with no diagnostic, on both the Cranelift and the LLVM
backend — as soon as it is used as a **function value**:

```kestrel
module Test

func combine(env: Int64, x: Int64) -> Int64 {
    env * 100 + x
}

func apply(f: (Int64, Int64) -> Int64, a: Int64, b: Int64) -> Int64 {
    f(a, b)
}

@main
func main() -> lang.i64 {
    let r = apply(combine, 3, 7);
    print("result=\(r)");
    if r != 307 { return 1 }
    0
}
```

```
$ kestrel build a_env_first.ks -o a && ./a
result=3
$ echo $?
1
```

`307` was expected. `3` came out, with a clean exit from the compiler and no
warning of any kind. Calling `combine(3, 7)` **directly** in the same program
gives the correct `307` — only the function-value path is wrong.

## Reproduction table

Six variants, all `apply(combine, 3, 7)` expecting `307` unless noted. "Before"
is the shipped compiler at `e5667910`; "after" is with this fix.

| # | first parameter | before | after |
|---|---|---|---|
| a | `func combine(env: Int64, x: Int64)` | `result=3`, exit 1 — **silent miscompile** | `307`, exit 0 |
| b | `func combine(_env: Int64, x: Int64)` | `result=3`, exit 1 — **silent miscompile** | `307`, exit 0 |
| c | `func combine(x: Int64, env: Int64)` — `env` at position 2 | backend verifier error: `mismatched argument count … got 1, expected 2` | `307`, exit 0 |
| d | `func combine(e: Int64, x: Int64)` — control | `307`, exit 0 | `307`, exit 0 |
| e | `combine(3, 7)` called directly, never as a value — control | `307`, exit 0 | `307`, exit 0 |
| f | `func combine(self: Int64, x: Int64)` | backend verifier error: `mismatched argument count … got 1, expected 2` | `307`, exit 0 |

Variant (f) matters because `self` is **not** a lexer or parser keyword.
`func combine(self: Int64, x: Int64)` is legal Kestrel today and parses,
name-resolves and type-checks; only the thunk pass mangled it.

Both backends were affected identically. `KESTREL_BACKEND=llvm` on (a) also
produced `result=3` before and `307` after.

## Root cause

`lib/kestrel-mir/src/passes/thunk.rs` synthesizes a wrapper ("thunk") for every
function reached by an `ApplyPartial`, so that a plain function and a closure
present the same `(env_ptr, args…)` ABI behind a thick function value. Building
that wrapper needs one fact about the target: **does its `params[0]` hold a
synthesized environment pointer, or is it a real user parameter?**

It answered that question with string comparisons on the parameter's name:

```rust
let needs_env = target_func
    .params
    .first()
    .is_some_and(|p| p.name == "env" || p.name == "_env");

// Non-self, non-env params from the target
let target_params: Vec<_> = target_func
    .params
    .iter()
    .filter(|p| p.name != "self" && p.name != "env" && p.name != "_env")
    .cloned()
    .collect();
```

`"env"` and `"_env"` are the names the *producers of generated code* give their
own synthesized parameter — mir-lower's `closure.rs` pushes `"env"`, this very
pass pushes `"_env"`. They are facts about generated code. Nothing stopped a
user from spelling a real parameter the same way, and when one did, the pass
misread it as generated.

### Why (a) is a same-count, wrong-type miscompile

For `combine(env: Int64, x: Int64)` both name rules fired on the *same*
parameter:

- `needs_env` was `true`, so the thunk's own env pointer was pushed as forward
  argument 0;
- the filter dropped `env` from `target_params`, so only `x` survived.

The generated thunk was therefore:

```mir
; before
; function: Test.combine.thunk
bb0(%v0: @owned Pointer[()], %v1: @owned std.numeric.Int64):
    %v2 = call Test.combine(%v0, %v1)
    return %v2
```

Two forward arguments into a two-parameter target — the **count matched**. Only
the types did not: `combine`'s `env: Int64` received a `Pointer[()]`, and the
thunk had one fewer parameter than the thick type `(Int64, Int64) -> Int64` it
was standing in for, so the caller's second real argument (`7`) was never
passed at all.

The arithmetic confirms the mechanism exactly: a capture-free thunk's env
pointer is a null `Pointer[()]`, so `env` read as `0`, `x` received the caller's
first argument `3`, and `0 * 100 + 3 = 3` — the observed output. `7` was
dropped on the floor.

This is why **a pure argument-count check would not have caught variant (a)**.
It is also why every arity-based defence already in the pipeline stayed quiet
while variants (c) and (f) — where only the filter fired, leaving a genuine
count mismatch — were caught by the backend's own verifier as an unexplained
internal failure.

After the fix:

```mir
; after
; function: Test.combine.thunk
bb0(%v0: @owned Pointer[()], %v1: @owned std.numeric.Int64, %v2: @owned std.numeric.Int64):
    destroy_value %v0
    %v3 = call Test.combine(%v1, %v2)
    return %v3
```

The env pointer is destroyed rather than forwarded, and both user parameters
are forwarded positionally.

## Reachability

`run_thunk_pass` scans **every** function in the module for
`ApplyPartial { callee: Callee::Direct { .. } }` and applies no `FunctionKind`
filter whatsoever. Plain free functions arrive there through `lower_def` as
soon as inference gives them a `MirTy::FuncThick` type. The trigger is
therefore just "a named function is used as a function value" — passing it to a
higher-order function, storing it in a variable or a struct field, returning it.
No closures, generics or protocols are needed.

## The fix

Whether `params[0]` is a synthesized env pointer is a property of the
function's **kind**, and both producers already record it:

- `kestrel-mir-lower/src/body/closure.rs` sets `FunctionKind::ClosureCall`
  (capturing) or `FunctionKind::Closure` (capture-free) inside a match, and then
  pushes the `"env"` `ParamDef` at index 0 **outside** that match — so both arms
  get it, unconditionally.
- `passes/thunk.rs` sets `FunctionKind::Thunk` and then pushes its `"_env"`
  `ParamDef` at index 0, unconditionally.

So `Closure | ClosureCall | Thunk` ⟺ "params[0] is the env pointer", with no
gap in either direction. That is now spelled once, as
`FunctionKind::takes_env_param()` in `lib/kestrel-mir/src/item/function.rs`, and
the thunk pass reads it back instead of the names. `mono/collect.rs`'s
`detect_implicit_protocol`, which was already asking the identical question with
an inline `matches!`, calls the same method.

Parameter forwarding became purely positional — `.skip(1)` when the target takes
an env pointer, `.skip(0)` otherwise. No name-based filtering of any kind
survives in the pass. A `debug_assert!` pins the invariant from the other side:
when the kind promises an env parameter, `params[0].name` must still be `"env"`
or `"_env"`, so a future reordering in `closure.rs` trips in debug builds rather
than silently resurrecting a variant of this bug.

Two fail-loud backstops were added:

1. In `thunk.rs`, after the forward-argument list is complete: a
   `debug_assert_eq!` on the count **and** a per-position type comparison against
   the target's own `ParamDef`s. The type half is the one that catches variant
   (a). It is exact rather than heuristic — these are literally the same `TyId`s
   cloned off `target_func.params`, pre-monomorphization, so there is no
   substitution or ref-decay gap.
2. In `verify_ossa`, a general argument-**count** check on `InstKind::Call` for
   any statically resolvable callee, via a new `VerifyModule::callee_declared_arity`
   (implemented for `MirModule` on `Callee::Direct` and for `MonoModule` on
   `Callee::Resolved`). This turns the class of failure behind variants (c) and
   (f) from an opaque backend verifier message into a located compiler error.
   Count only — per-argument type verification at this level would need
   substitution, ref-decay and ByVal/ByRef reasoning and would false-positive
   across the corpus.
