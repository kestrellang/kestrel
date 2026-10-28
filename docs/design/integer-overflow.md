# Integer overflow — design

**Status:** accepted 2026-10-07, not implemented. Rulings: **O1 (b)** Rust
model — trap when overflow checks are on (default at `-O0`), wrap when off;
**O2** `*Wrapping` methods (operators deferred); **O3** `MIN / -1` traps when
checks are on. Rollout per §4: flag default-off, measure, convert, then flip.
**Why now:** the bidirectional-checker ruling D2 (2026-10-06,
`bidirectional-typechecking.md` § *Maintainer rulings*) chose Rust-style
literals. Under that rule `let x = 100; takes8(x); let w = x * 2` types `x`
as `Int8`, and because integer arithmetic wraps, `w` is silently `-56`. Rust
accepts the same typing because its debug builds panic on overflow. Kestrel
has no such net. This document decides whether to add one.

**Evidence tags:** **[V]** verified by reading the code at `6f838c5c`;
**[R]** reasoned, not measured.

---

## 1. Today

| Operation | Behaviour | Where |
|---|---|---|
| `+ - *`, unary `-` | **wrap**, documented in the stdlib doc comments ("wrapping on overflow") | `lang/std/numeric/int8.ks:405-413` [V]; generated from `integer.ks.template` by `generate.py` |
| `/ %` by zero | **trap** (explicit block in LLVM, native trap in Cranelift) | `kestrel-codegen-llvm/src/inst.rs:823-845` |
| `MIN / -1`, `MIN % -1` | **defined**: wraps to `MIN`, and `0` | llvm `inst.rs:850, 962-990`; cranelift `inst.rs:680-694` |
| `<< >>` | shift amount **masked** mod bit width | llvm `inst.rs:1002-1015` |
| `addChecked`, `…Saturating` | `Optional` / clamp; built on `lang.iN_{signed,unsigned}_{add,sub,mul}_overflows` | `int8.ks:464-520` [V] |
| integer literal out of range | compile error E121, "never silently truncated" | `docs/language/types.md:38-45` |

- The `+ - *` lowering is a plain `build_int_add` / `iadd`, with no flags and no
  check, in both backends [V] (`kestrel-codegen-llvm/src/inst.rs:953-955`).
- MIR's `Op::Add(bits, signedness)` carries a sign, but the intrinsic table
  hardcodes `Signed` for `iN_add` even when unsigned types call it
  (`kestrel-mir-lower/src/body/call/intrinsic.rs:41`). So today the sign means
  nothing, and a check would need it fixed.
- **Build modes:** there is no debug/release distinction. The CLI has
  `-O/--opt-level` (default 0, `src/main.rs:99-107`, env `KESTREL_OPT`). The
  value reaches `CodegenOptions.opt_level` in each backend, but **not** MIR
  lowering (`Compiler::lower_to_mir()` takes no options).
- **Trap path:** `lang.panic()` → `TerminatorKind::Panic` → `llvm.trap` /
  Cranelift `trap(user(1))`. `fatalError(message:)` prints and then panics.
  Array bounds checks are ordinary stdlib code calling `fatalError`.
- **Code that depends on wrapping:**
  - the stdlib hashers (`collections/hashing.ks:97-129`, FNV and Murmur
    multiplies on `UInt64`) [V];
  - `memory/pointer.ks` hashes and `numeric/random.ks` (LCG) [R];
  - the `crypto` package digests [R].

  These are mostly the same 9 constants that the per-statement literal
  measurement surfaced.

## 2. What other languages do

| | Debug | Release | Wrapping spelled |
|---|---|---|---|
| **Rust** | panic | wrap (opt-in `overflow-checks = true`) | `wrapping_add`, `Wrapping<T>`; `checked_*`, `saturating_*` |
| **Swift** | trap | **trap** (only `-Ounchecked` removes it) | `&+ &- &*` operators; `addingReportingOverflow` |
| **Zig** | trap | trap in `ReleaseSafe`, UB in `ReleaseFast` | `+% -% *%` operators |
| **Go / Java / C#** | wrap | wrap | default (`checked {}` in C#) |

Rust and Swift also both panic on `MIN / -1`.

## 3. Decisions

### O1 — what `+ - *` and unary `-` do on overflow

| Option | Rule | N4 (`w = -56`) | Cost |
|---|---|---|---|
| (a) | Keep wrapping everywhere (today) | silent in every build | 0; document it in `docs/language/` |
| **(b) Rust** (recommended) | **Trap when overflow checks are on; wrap when off.** Checks are on by default at `-O0` and off at `-O1+`. `--overflow-checks=on/off` overrides either way. | trap in development builds; wraps in optimized builds | stdlib code that relies on wrapping must say so (O2) |
| (c) Swift | Always trap. An explicit unchecked flag is the only opt-out. | trap everywhere | same as (b), plus a small runtime cost in optimized builds |

**Why (b):**
- It is the counterpart of the D2 ruling. Rust-style literals are
  acceptable *because* overflow is caught where people test.
- It keeps optimized builds exactly as fast as today.
- (c) is the safer end state. Moving from (b) to (c) later only means changing
  a default. Moving back would break programs that now rely on traps being off.

### O2 — how wrapping is spelled

Under (b) and (c), `add` traps when checks are on. Code that wants wrapping
must ask for it:
- **Methods** `addWrapping`, `subtractWrapping`, `multiplyWrapping` and
  `negateWrapping`, next to the existing `*Checked` / `*Saturating`.
  Recommended: no new syntax, it matches Rust, and the stdlib already has the
  `*Checked` family.
- Optionally, **operators** `&+ &- &*` (Swift), as sugar for those methods,
  through `BINARY_OP_PROTOCOLS`. This can be decided later.

### O3 — the neighbours, for consistency

| Case | Today | Proposed with (b) |
|---|---|---|
| `MIN / -1`, `MIN % -1` | defined, wraps | trap when checks are on (Rust and Swift panic in every mode) |
| shift by ≥ bit width | masked | unchanged (documented masking; Rust panics in debug) — lowest priority |
| `Int8(truncating:)`-style conversions | — | unchanged; explicit by name |
| constant expressions that overflow (`Int8.maxValue + 1` folded at compile time) | wraps | a compile-time error is the natural follow-up; not part of this design |

## 4. Implementation sketch (for (b) + method spelling) [R]

1. **Intrinsics** (`kestrel-ast-builder/src/lang_module.rs:301-415`): add
   `iN_{signed,unsigned}_{add,sub,mul}_checked` and `iN_neg_checked` ("checked"
   meaning *trap if checks are on*). The existing `iN_add`/`sub`/`mul`/`neg`
   become the explicitly wrapping ones.
2. **MIR** (`kestrel-mir/src/op.rs`): `Add/Sub/Mul/Neg` gain an
   `Overflow { Wrap, Check }` field. Lowering also fixes the hardcoded
   `Signed` (intrinsic.rs:41), because a check needs the real sign.
3. **Codegen**: `CodegenOptions` gains `overflow_checks: bool`, derived from
   `opt_level` unless the flag overrides it.
   - For `Check` ops, LLVM uses `llvm.{s,u}{add,sub,mul}.with.overflow` (the
     helper already exists, `emit_overflow_check`), and Cranelift uses the
     existing widen-and-compare helper. Both branch to the backend's existing
     `Panic` lowering.
   - Deciding in codegen, not MIR, keeps MIR (and its cache) independent of
     the build mode. The query graph needs no new input.
4. **Message**: `Panic` drops its message today (`llvm.trap` only). Overflow
   should report "attempt to add with overflow" and the source location. This
   is the same work bounds checks would also benefit from.
5. **Stdlib**: `integer.ks.template`:
   - `add`/`subtract`/`multiply`/`negate` call the checked intrinsics;
   - new `*Wrapping` methods call the plain ones;
   - `*Checked` is rebuilt on the plain ones (unchanged semantics).

   Then regenerate with `generate.py`, and move the wrap-reliant code in §1 to
   `*Wrapping`.
6. **Tests**:
   - `stdlib/uintN/uintN_overflow_behavior.ks` and
     `codegen/arithmetic/int_div_min_value_wraps.ks` pin today's wrapping.
     Under (b) their expectations change by design, so they need your explicit
     OK per CLAUDE.md.
   - New: overflow traps at `-O0`, wraps with `--overflow-checks=off`,
     `*Wrapping` never traps.
   - N4 (`w = -56`) becomes a trap test.

**Measuring the cost before committing to it:**
- Land steps 1–3 behind the flag with the default **off**.
- Turn it on, run the execution corpus plus the `lang/` package tests, and
  list every trap. That list is the exact set of wrap-reliant code the stdlib
  and packages must convert.
- Only then flip the default.

## 5. Open questions for the ruling

1. O1: (a), (b) or (c)? Recommended: (b).
2. O2: methods only, or also `&+ &- &*`? Recommended: methods now, operators
   later if wanted.
3. O3: should `MIN / -1` trap under checks? Recommended: yes.
