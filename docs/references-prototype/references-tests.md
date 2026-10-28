# References — Stage 1 Test Matrix

## Harness Format Summary

All `.ks` test files under `lib/kestrel-test-suite/testdata/` are auto-discovered by the file-based harness (`tests/file_tests.rs`). The mode is determined by file-header comments.

**Header keys** (must appear before any non-comment, non-blank line):
```
// test: diagnostics | mir | execution
// stdlib: true | false
// expect-exit: <int>           (execution only; default 0)
// expect-stdout: <string>      (execution only; exact match after trim)
// stdout-contains: <string>    (execution only; substring)
// skip: <reason>               (skip this test with reason)
// include: <relative-path>     (pull in an extra .ks file)
```

**Inline annotation syntax** (end-of-line in `diagnostics` tests):
```
// ERROR                  — any error on this line
// ERROR: message text    — error whose message contains the substring
// ERROR(E441)            — error with specific analyzer code
// WARN / WARN: message   — warning equivalents
```

**`execution` test contract:**
- `check_no_errors()` runs first — the test fails if the compiler emits any error.
- The executable is compiled with Cranelift, linked, and run.
- Exit code is checked against `expect-exit` (default 0).
- Stdout is checked against `expect-stdout` (exact, trimmed) or `stdout-contains` (substring), if provided.
- A non-zero exit code, stdout mismatch, or compile failure all count as test failure.
- There is no ASan, Valgrind, or leak detector wired into the harness. **Memory bugs surface only if they cause a crash (SIGSEGV / SIGABRT → non-zero exit) or corrupt a checked value.**

---

## Detecting UAF Under the Harness

The harness has no sanitizer integration. UAF/double-free/leak detection relies entirely on:

1. **OSSA verifier firing before codegen.** `verify_ossa` runs on every body before emission. It catches:
   - Use of a consumed value (double-move).
   - `@owned` value live at block exit but never consumed (leak).
   - Open borrow at block exit without `EndBorrow` or forwarding (dangling borrow).
   - Borrow-while-consumed (via `try_consume` blocking loop, `verify.rs:328-334`).
   - Read-during-mut-borrow (Check 5, `verify.rs:373`).
   - **It does NOT catch:** `@guaranteed` values in `Return` position (no-op, `verify.rs:989`); cross-block liveness of borrow sources (`verify.rs:355,362` return true for cross-block); projections of borrows not tracked in `self.borrows`.

2. **`deinit_count` / drop-counter pattern.** A `not Copyable` struct with a global `var deinit_count: Int64` counter in its `deinit` block lets execution tests assert exact drop counts. Double-free increments the counter past the expected value; use-after-free (read via a held pointer after drop) returns a corrupted or already-decremented value. This is the primary runtime detection mechanism in the existing memory model tests. It does NOT detect: leaks (under-drop increments the counter too few times, which can be checked as `deinit_count != N`).

3. **`Pointer`-backed cell pattern** (as in `aggregate_control_flow_field_no_double_drop.ks`). The `deinit` writes to a heap cell and the test reads back the cell value to detect whether `deinit` ran the wrong number of times. A UAF read after the cell was freed returns garbage.

4. **Non-zero exit code.** SIGSEGV / SIGABRT from a double-free or null-deref causes a non-zero exit. The harness reports: `Expected exit code 0, got -11` (SIGSEGV on macOS returns -11 to the wait code).

5. **Wrong output value.** A miscompile (wrong value returned by a `@guaranteed` scalar read — the `resolve_scalar` path documented in the design doc's §5 #1) will produce a wrong numeric result, which fails an `expect-stdout` or a numeric comparison in `main`.

**The headline gap:** the escape checker (§5 #2 in the design doc) is purely new analysis — it does not exist in the compiler today. Until it is built, a "return borrow of local" test will compile cleanly, run, and produce a dangling-pointer dereference at runtime. The runtime failure path is: the borrowed local is dropped at function scope exit, the caller receives a pointer to freed stack, and any read of it is a UAF. On most runs this either crashes (SIGSEGV) or returns garbage, both of which fail an exit-0 or value-check test. Neither failure mode is guaranteed — the stack slot may still hold the value after scope exit in a debug build, making the UAF silent. **These tests must be marked `// skip: escape checker not yet implemented` until the checker lands; they document the required invariant, not current behavior.**

**Proposed harness extension (not in current code):** wire `ASAN_OPTIONS=detect_leaks=1` and pass `-fsanitize=address` to the Cranelift linking step (via a `KESTREL_ASAN=1` env flag). This would catch: heap use-after-free, double-free, and heap leaks without any Kestrel-level instrumentation. Until then, the `deinit_count` pattern is the only reliable detector.

---

## Negative Tests Must Reject, Not Miscompile

The following table entries marked "diagnostics" test must produce a compile-time error. The critical property for each is that the compiler **rejects the program** rather than accepting it and generating unsafe code. Each rejection is tied to a specific verifier or checker anchor:

- **Return-borrow-of-local** must be rejected by the escape checker (§5 #2). Without the checker, the compiler accepts the program and emits code that compiles clean and UAFs silently. The test must be `skip`ped until the checker is implemented.
- **`&mutating` return** must be rejected by a guard at the return-convention check. Without the guard, `set_terminator` force-ends the borrow before `Return`, so the value is unusable — but the compiler may ICE or silently mis-lower rather than emit a clean diagnostic.
- **Reference crossing if/loop merge** must be rejected because `add_guaranteed_block_param` is not implemented (only a panic-string aspiration). Without the rejection, the code will ICE in lowering.
- **`&mutating` into RcBox-backed field** must be rejected because it cannot be proven safe at Stage 1.
- **Dropped referent before use (intra-block)** is caught by the OSSA verifier's `try_consume` blocking loop (`verify.rs:328-334`), which prevents consuming an `@owned` source while a borrow is live.

---

## Test Matrix

| Test name | Kind | Pins | Expected | UAF/miscompile path it provokes |
|---|---|---|---|---|
| `ref_param_shared_pass` | execution | Shared `&T` param accepted; caller value not moved; callee reads correctly | Compiles, runs, exit 0; value printed is correct | Baseline — ensures `@guaranteed` pass-by-ref convention works for params (already functional) |
| `ref_param_mutating_pass` | execution | `&mutating T` param accepted; callee mutates through ref; caller sees updated value | Compiles, runs, exit 0; mutated value verified | Baseline for `&mutating` param (already functional via `ParamConvention::MutBorrow`) |
| `intra_block_borrow_basic` | execution | `let r = &x; use(r)` in same block; `x` not moved; ref dropped before `x` | Compiles, runs, exit 0; no double-drop | In-function named borrow over single expression; validates `BeginBorrow`/`EndBorrow` pair is correctly emitted |
| `intra_block_borrow_dropped_before_referent` | diagnostics | `let r = &x; drop(x)` — consuming `x` while borrow `r` is live | ERROR: cannot consume while borrowed | OSSA `try_consume` blocking loop (`verify.rs:328-334`): consuming `@owned` source while `@guaranteed` borrow is tracked in `self.borrows` |
| `return_borrow_of_param_shared` | execution | `func first(&arr: Array[Int64]) -> &Int64 { &arr(0) }` | Compiles, runs, exit 0; returned ref reads correct value | The only fully-safe return-ref case in v1: param outlives the call; `borrow_source` chain bottoms out in a `Param`; tests the `ret_borrow` gate on the two copy guards (`mod.rs:476`, `expr.rs:258`) |
| `return_borrow_of_local_REJECTED` | diagnostics | `func bad() -> &Int64 { let x = 42; &x }` — borrow of local returned | ERROR: returned reference does not outlive the call | Escape checker (`verify.rs` return-site `borrow_source` walk): root provenance is `Local`, not `Param`/`Static`; **silent UAF if checker is absent** — caller gets a pointer to freed stack |
| `return_borrow_param_scalar_value_correct` | execution | `func peek(&val: Int64) -> &Int64 { &val }; let x = 7; let r = peek(&x); assert *r == 7` | exit 0; printed value is 7 | Pins the `resolve_scalar` miscompile path (`func.rs:45-62`): a `@guaranteed`+`Scalar` return must use `get_value` (the raw pointer) not `resolve_scalar` (which loads through the ptr, returning the pointee by value and breaking the returned-reference contract) |
| `return_mutating_borrow_REJECTED` | diagnostics | `func mut_peek(&mutating x: Int64) -> &mutating Int64 { &mutating x }` | ERROR: cannot return a mutable reference | `&mutating` return is deferred in v1; rejected at the return-convention check; Check 5 (`verify.rs:373`) cannot enforce across the return boundary |
| `ref_crossing_if_merge_REJECTED` | diagnostics | `let r = if cond { &a } else { &b }; use(r)` — reference surviving an if-merge | ERROR: reference cannot cross control flow merge | `add_guaranteed_block_param` is unimplemented (panic-aspiration only); the verifier would need fixpoint cross-block liveness it does not have (`verify.rs:1012-1033` accepts syntactically only) |
| `ref_crossing_loop_REJECTED` | diagnostics | `let r = &x; loop { use(r) }` — borrow forwarded through loop back-edge | ERROR: reference cannot persist across loop | Same unimplemented cross-block borrow forwarding; a loop back-edge is a block successor the verifier's per-block BFS cannot prove liveness for |
| `mutating_alias_within_block` | execution | Two `&mutating` refs to the same value in the same block; both write; last write wins | Compiles, exit 0; correct final value | The aliasing-allowed decision: `&mutating` does NOT have exclusivity; Check 5 (`verify.rs:373`) is NOT triggered; documents the deliberate soundness relaxation |
| `mutating_into_rcbox_REJECTED` | diagnostics | Deriving `&mutating` from an `RcBox`/shared-heap-backed field | ERROR: cannot take a mutable reference to shared heap storage | Guards the `RcBox.setValue` / COW unsoundness: a `&mutating` through `getValue` can alias concurrent `RcBox` readers; rejected in v1 |
| `ref_of_struct_field_param` | execution | `func name(&person: Person) -> &String { &person.name }` — field projection | Compiles, runs, exit 0; returned field ref reads correct value | `root_provenance` propagation through `StructExtract` / `BeginBorrowAddr` field projections: tests that the escape checker correctly stamps provenance through projections, not just direct params |
| `intra_block_deinit_order` | execution | Borrow `r` of `x` ends before `x` is dropped; both resources deinit exactly once | exit 0; `deinit_count == 2` | Validates that `EndBorrow` is emitted before the scope-exit `DestroyValue` for the source; ensures `destroy_scope_except` ordering is correct |
| `scalar_return_via_ref_no_load` | execution | `func id(&x: Int64) -> &Int64 { &x }; let v = id(&42); assert *v == 42` | exit 0; value is 42 not garbage | Direct pin of the `resolve_scalar` miscompile: if `resolve_scalar` loads through the ByRef pointer, it dereferences a pointer-to-scalar and returns the integer value rather than the pointer, silently breaking the borrow contract and making `*v` a second load of a now-dead address |

---

## Fully-Written Test Files

### 1. `ref_param_shared_pass.ks`

```
// test: execution
// stdlib: true
// expect-exit: 0
//
// Pins: shared &T parameter — value is not moved, callee reads correctly,
// caller retains ownership after the call.

module Test

import std.numeric.Int64

public var deinit_count: Int64 = 0;

struct Token: not Copyable {
    var value: Int64
    deinit { deinit_count = deinit_count + 1; }
}

func read_value(&tok: Token) -> Int64 {
    tok.value
}

func main() -> lang.i64 {
    let t = Token(value: 99);
    // Pass by shared reference — t is not moved.
    let v = read_value(&t);
    if v != 99 { return 1; }
    // t is still alive here; deinit has not run yet.
    if deinit_count != 0 { return 2; }
    // t drops here: exactly one deinit.
    0
}
// After main returns, t drops → deinit_count == 1.
// The execution test only checks exit code; deinit on exit is verified
// by the expect-exit: 0 (a double-free/crash would produce non-zero).
```

### 2. `ref_param_mutating_pass.ks`

```
// test: execution
// stdlib: true
// expect-exit: 0
//
// Pins: &mutating T parameter — callee mutates through the ref,
// caller observes the updated value. Aliasing is permitted.

module Test

import std.numeric.Int64

func increment(&mutating counter: Int64) {
    counter = counter + 1;
}

func add_to(&mutating total: Int64, amount: Int64) {
    total = total + amount;
}

func main() -> lang.i64 {
    var x: Int64 = 0;
    increment(&mutating x);
    if x != 1 { return 1; }
    add_to(&mutating x, 41);
    if x != 42 { return 2; }
    0
}
```

### 3. `return_borrow_of_param_shared.ks`

```
// test: execution
// stdlib: true
// expect-exit: 0
// skip: return-ref codegen not yet implemented (ret_borrow gate missing)
//
// Pins: returning a borrow of a PARAMETER — the only safe return-ref case in
// Stage 1. The caller owns the Array; the callee returns &arr(0); the caller
// dereferences the returned ref. Requires:
//   - ret_borrow gate on the copy guards (mod.rs:476, expr.rs:258)
//   - escape checker: borrow_source root == Param → accepted
//   - codegen: compile_return uses get_value (pointer), not resolve_scalar

module Test

import std.numeric.Int64
import std.collections.Array

// Returns a shared reference to the first element of the passed array.
// The array is passed by reference so its lifetime exceeds the call.
func first(&arr: Array[Int64]) -> &Int64 {
    &arr(0)
}

func main() -> lang.i64 {
    var data = Array[Int64]();
    data.append(42);
    data.append(7);
    let r = first(&data);
    // Dereference the returned reference: must be 42, not garbage.
    if *r != 42 { return 1; }
    0
}
```

### 4. `return_borrow_of_local_REJECTED.ks`

```
// test: diagnostics
// stdlib: true
//
// SAFETY: this program must be REJECTED at compile time.
// Accepting it and compiling it would produce a silent UAF:
//   - `x` is a local with function scope.
//   - `&x` is a borrow whose borrow_source root is `Local`, not `Param`.
//   - The caller would receive a pointer to freed stack after `bad` returns.
//   - No ICE, no crash at compile time — only silent heap/stack corruption
//     at runtime, possibly on the very next call that reuses the stack slot.
//
// The escape checker (verify.rs return-site borrow_source walk) must reject
// this with an error. Until the checker is implemented this test will
// compile clean and fail at runtime — mark it skip until then.
//
// skip: escape checker not yet implemented (Stage 1 safety gate)

module Test

import std.numeric.Int64

func bad() -> &Int64 {
    let x: Int64 = 42;
    &x  // ERROR: returned reference does not outlive the call
}

func main() -> lang.i64 {
    let r = bad();
    *r
}
```

### 5. `return_mutating_borrow_REJECTED.ks`

```
// test: diagnostics
// stdlib: false
//
// SAFETY: returning a &mutating reference is deferred in Stage 1.
// A returned mut-ref keeps the source frozen for mutation across the call
// boundary, which assert_readable / Check 5 (verify.rs:373) cannot enforce
// past Return (per-block, no fixpoint). The aliasing-allowed decision is
// sound within a block but does NOT extend across return.

module Test

func peek_mut(&mutating val: lang.i64) -> &mutating lang.i64 {
    &mutating val  // ERROR: cannot return a mutable reference
}

func main() -> lang.i64 {
    var x: lang.i64 = 0;
    let r = peek_mut(&mutating x);
    *r = 99;
    x
}
```

### 6. `intra_block_borrow_dropped_before_referent.ks`

```
// test: diagnostics
// stdlib: true
//
// Pins: OSSA verifier try_consume blocking loop (verify.rs:328-334).
// Consuming an @owned value while a @guaranteed borrow derived from it is
// still live must be rejected by the verifier. This is an intra-block case
// where both the borrow and the attempted consume are in the same block.
//
// Without this check, the borrow outlives its source: the caller of `consume`
// would execute deinit on the Token, then use `r` to read freed memory.

module Test

import std.numeric.Int64

struct Token: not Copyable {
    var id: Int64
    deinit {}
}

func consume(consuming t: Token) {}

func bad(t: Token) {
    let r = &t;
    consume(t);  // ERROR: cannot consume while borrowed
    _ = r;
}

func main() -> lang.i64 { 0 }
```

---

## Negative Tests Must Reject, Not Miscompile

Each negative test below is tied to a specific verifier anchor. The critical invariant is that the program is **rejected with a compile-time error**, not silently accepted and miscompiled into a UAF.

**`return_borrow_of_local_REJECTED`** — The escape checker (net-new, §5 #2) must walk `borrow_source` from the `Return` value back to its root. If the root is `Local` (not `Param`/`Static`), the checker emits an error. **Without this checker the program compiles clean.** The only runtime backstop is that the freed stack slot is re-used and the read returns garbage (or crashes), both non-deterministic. This test is skip until the checker lands. Verifier anchor: new `root_provenance` stamp check at the `Return` site.

**`return_mutating_borrow_REJECTED`** — Rejected by a gate at the return-convention site, before the `ret_borrow` path is engaged. `&mutating` returns are not permitted in v1. Verifier anchor: return-convention validation added alongside the `ret_borrow` bit.

**`ref_crossing_if_merge_REJECTED`** — `add_guaranteed_block_param` is unimplemented (only a panic-string in the codebase). The lowering must detect that a borrow-value is live at a block merge and reject with a diagnostic rather than panicking. Verifier anchor: `set_terminator`'s force-EndBorrow for non-escaping borrows (`mod.rs:~1820`); any borrow that would be forwarded through a merge without `add_guaranteed_block_param` infrastructure must be caught here.

**`ref_crossing_loop_REJECTED`** — Same as above; loop back-edges are block successors. Verifier anchor: same `set_terminator` / cross-block borrow forwarding check.

**`intra_block_borrow_dropped_before_referent`** — Already caught by the existing OSSA `try_consume` blocking loop (`verify.rs:328-334`). This is a pre-existing safety gate that Stage 1 inherits for free; it is the only negative test that does not require new checker infrastructure.

**`mutating_into_rcbox_REJECTED`** — Rejected by a new semantic check that the `&mutating` source must not be derived from a heap-shared/Rc-backed storage location. Without this guard, the Stage 1 aliasing-allowed relaxation (Check 5 disengaged) opens a concurrent-read/write UAF through `RcBox.getValue`/`setValue`. Verifier anchor: new `BeginMutBorrow` source classification (provenance must be stack/param, not heap-Rc interior).

---

## Essential Files for Stage 1 Implementation

- `/Users/dino/Documents/Projects/kestrel/docs/references-prototype/references.md` — full feasibility analysis and MVP cut
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/verify.rs` — OSSA verifier; `try_consume` (line ~321), `assert_readable`/Check 5 (line ~373), `Return` handling (line ~988), Check 4 block-arg forwarding (line ~1012)
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/value.rs` — `ValueDef.ownership`, `ValueDef.borrow_source` (lines 14-16)
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir/src/inst.rs` — `BeginBorrow`/`EndBorrow`/`BeginMutBorrow`/`EndMutBorrow`/`*Addr` instruction set
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/body/mod.rs` — copy guards at return (lines ~476-489), `alloc_guaranteed` (line ~663), `set_terminator` force-EndBorrow (line ~1820)
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-mir-lower/src/body/expr.rs` — second copy-at-return site (lines ~257-265)
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/annotation.rs` — test format parser; `TestMode`, header keys, `// ERROR` annotation syntax
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/src/runner.rs` — execution harness; no sanitizer integration documented here
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/tests/file_tests.rs` — file discovery, dispatch to diagnostics/mir/execution modes
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/testdata/memory_model/deinit/aggregate_control_flow_field_no_double_drop.ks` — canonical `deinit_count` + `Pointer`-cell UAF detection pattern
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/testdata/memory_model/deinit/partial_drop_one_of_two_initialized.ks` — canonical partial-drop / exact-count pattern
- `/Users/dino/Documents/Projects/kestrel/lib/kestrel-test-suite/testdata/codegen/keyword_labels.ks` — canonical execution test format with `main() -> lang.i64` returning non-zero on failure
