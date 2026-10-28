# Kestrel Bug Hunt — 2026-06-10

A multi-agent automated bug hunt run against a **frozen reference compiler** built from
`e80ddead` (branch `feature/115-references`, 0.17-dev). The frozen binary and a matching
stdlib snapshot live at `temp/stable-kestrel/` so every repro replays identically as the
tree changes:

```sh
export REPO=$(git rev-parse --show-toplevel)
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build repro.ks -o repro && ./repro
```

**Method.** 22 finder agents probed feature *intersections* (references x generics,
non-Copyable types x control flow, witness dispatch x statics, backend differentials, ...),
each writing and *running* small self-checking programs. Every claimed finding was then
adversarially re-verified by an independent agent that re-ran the repro from scratch and
re-derived the expected behavior from testdata/docs; semantic claims got a second
language-lawyer review. 129 findings were filed, 119 confirmed, 10 refuted; deduplication
by root cause yields the **82 bugs** below
(12 critical / 39 high / 26 medium / 5 low).

Severity rubric: **critical** = silent miscompile or memory corruption in plausible code;
**high** = ICE on valid code, runtime crash, backend divergence; **medium** = ICE on invalid
code, false/missing diagnostic; **low** = minor.

All repro programs live under `temp/bughunt/<area>/` (minimized versions under
`temp/bughunt/verify/`); commands in each section assume `$REPO` as above. Raw finding
data: `temp/bughunt/harvest.json`, verification verdicts:
`temp/bughunt/verify/opus/verdicts.json`.

Cross-cutting clusters worth reading first:

- **Silent write-loss** (BUG-01, 02, 05, 63, 64, 80): five lowering paths resolve an
  lvalue to an rvalue temp and discard the store — assignments that compile and do nothing.
- **Toolchain integrity** (BUG-13, 14, 15, 49): failed compiles that exit 0 with trap-stub
  binaries, write executables anyway, or fail with zero diagnostics. These multiply every
  other codegen bug.
- **`Self`/static witness dispatch** (BUG-08, 09, 10, 47, 48): static-position requirements
  are never witness-resolved at monomorphization; failure mode depends on which verifier
  catches it (ICE, mangler panic, or silently-skipped function -> SIGILL binary).
- **Non-Copyable ownership** (BUG-03, 04, 07, 16, 17, 18, 25, 26, 41, 70): moves, drops,
  and deinit counts go wrong across inits, aggregates, closures, `try`, and stdlib mutators.
- **References stage 1** (BUG-57-61): the `&mutating` return feature is unusable (E494),
  and witness-dispatched refs return addresses as values.

## Summary

| ID | Sev | Class | Status | Issue | Title |
|----|-----|-------|--------|-------|-------|
| BUG-01 | critical | miscompile | new | #139 | Assignment through a getter-produced intermediate is silently lost |
| BUG-02 | critical | miscompile | new | #140 | Compound assignment (`+=`) silently no-ops for computed-prop / static / global LHS |
| BUG-03 | critical | miscompile | new | #141 | Whole-`self` writes in mutating methods of non-Copyable types are unsound |
| BUG-04 | critical | miscompile | known-variant | #142 | Copyable mono-substitution gap: `Array.retain` corrupts memory; unconstrained generic struct ICEs |
| BUG-05 | critical | miscompile | new | #143 | Mutating-parameter writeback lost for tuple-element writes and destructured params |
| BUG-06 | critical | miscompile | new | #144 | Delegation to a failable init ignores failure |
| BUG-07 | critical | miscompile | new | #145 | Moving a non-Copyable field out of consuming self: silent failed build that still emits a double-dropping binary |
| BUG-38 | critical | miscompile | new | #174 | Escape checker bypassed: capturing closures escape via local binding or struct field; dangling environment at runtime |
| BUG-58 | critical | miscompile | new | #193 | Ref-returning protocol requirement through witness dispatch returns the ADDRESS as the value |
| BUG-63 | critical | miscompile | new | #198 | Tuple-element assignment `t.0 = v` compiles but the store is silently dropped |
| BUG-64 | critical | miscompile | known | #129 | Stores through chained-subscript places silently dropped (tracked as #129) |
| BUG-70 | critical | runtime-crash | new | #204 | `try` error-propagation double-deinits a non-Copyable Err payload (use-after-free) |
| BUG-08 | high | ICE | new | #146 | `Self`-keyed static requirements in protocol default impls never witness-resolved at mono |
| BUG-09 | high | ICE | new | #147 | Stored `static var` witness ICEs through a type parameter |
| BUG-10 | high | ICE | new | #148 | Generic default-parameter expression calling `T.staticMethod()` ICEs post-mono |
| BUG-11 | high | ICE | new | #149 | Default-parameter subscript WRITE with argument omitted: trap binary with exit 0 |
| BUG-12 | high | ICE | new | #150 | Static stored var counted as a memberwise-init field |
| BUG-13 | high | other | new | #151 | Any codegen failure downgraded to a warning: exit 0, trap-stub binary ships |
| BUG-14 | high | other | new | #152 | Failed builds (exit 1) still write the output executable |
| BUG-15 | high | missing-diagnostic | new | #153 | E503 in tail-expression position fails the build silently |
| BUG-16 | high | miscompile | new | #154 | Init bodies don't track initialized fields for drops: overwrite leaks; post-delegation failure leaks |
| BUG-17 | high | other | new | #155 | `Array.clear()` never runs element deinits |
| BUG-18 | high | miscompile | known | #127 | #127: array literal with non-Copyable element fires a spurious deinit at construction |
| BUG-19 | high | miscompile | new | #156 | Formatting any signed `minValue` emits mirrored garbage digits in every radix |
| BUG-20 | high | miscompile | new | #157 | NaN ordered comparisons via `<=` / `>=` return true |
| BUG-21 | high | backend-divergence | new | #158 | Integer division/modulo by zero does not trap on the LLVM backend |
| BUG-22 | high | backend-divergence | new | #159 | `Int64.min / -1`: three different behaviors; docs say wrap |
| BUG-23 | high | runtime-crash | new | #160 | `multiplyChecked/Saturating(minValue, -1)`: trap on cranelift, wrong answer on LLVM -O0 |
| BUG-24 | high | other | new | #161 | Float64 decimal digit generation emits wrong digits (`:.n` paths and large-magnitude shortest print) |
| BUG-39 | high | miscompile | new | #175 | Immediately calling a closure returned by a method (`s.mk()()`) produces garbage |
| BUG-40 | high | ICE | new | #176 | Immediately calling a closure returned by a subscript: Array ICEs; user-defined subscript ships a SIGILL binary |
| BUG-41 | high | missing-diagnostic | new | #177 | Move checker ignores closure captures of non-Copyable values |
| BUG-44 | high | miscompile | new | #180 | `for-in` over `Array[Cloneable]` deep-clones every element twice per iteration and deinits it twice in-loop |
| BUG-46 | high | miscompile | new | #182 | Overlapping generic conformances: only the first-declared extension is consulted (declaration-order-dependent witness selection) |
| BUG-47 | high | ICE | new | #183 | `some P` returned from a generic struct's method: TypeParam leaks past monomorphization (ICE) |
| BUG-48 | high | ICE | new | #184 | Associated-type-projection bound `T.Item: P`: witness call on the projected value ICEs post-mono |
| BUG-49 | high | missing-diagnostic | new | #185 | Extension `where` clause with an assoc-projection bound: build exits 1 with ZERO output |
| BUG-50 | high | miscompile | new | #186 | Integer range-from pattern `N..` never matches; SIGILL when it is the final arm |
| BUG-51 | high | ICE | new | #187 | Or-pattern with bindings (`.A(x) or .B(x) => x`) ICEs in OSSA verify |
| BUG-52 | high | miscompile | new | #188 | Array patterns are broken end-to-end at runtime |
| BUG-54 | high | ICE | new | #190 | ICE `enum case Entity(4294967295) has no Name` (pattern.rs:1344) when a wildcard arm follows a payload split |
| BUG-57 | high | false-diagnostic | new | #192 | E494 falsely rejects every `-> &mutating` return rooted at a `mutating` param or `self` |
| BUG-59 | high | false-diagnostic | new | #194 | Ref result does not decay to a copy in assignment-RHS and return value contexts |
| BUG-62 | high | miscompile | new | #197 | Nested string interpolation splices the inner literal's raw source text instead of evaluating it |
| BUG-65 | high | runtime-crash | new | #199 | `return` inside a closure types the closure `-> Never` and lowers its body to a trap |
| BUG-67 | high | ICE | new | #201 | Labeled `continue` crossing an inner loop: OSSA verify ICE |
| BUG-72 | high | backend-divergence | new | #206 | Shift by >= bit width: LLVM -O2 produces poison garbage (even for constant `1 << 64`) |
| BUG-73 | high | backend-divergence | new | #207 | `f64 -> i64` cast of NaN/inf/out-of-range: LLVM -O2 produces nondeterministic garbage |
| BUG-78 | high | false-diagnostic | new | #212 | `[1,2,3] == [1,2,3]` fails to compile: mono loses the Slice protocol-extension witness's type args |
| BUG-80 | high | miscompile | new | #214 | Member field chained off a static computed var silently drops the projection |
| BUG-82 | high | miscompile | new | #216 | Float64 subnormals print Int64.max-mantissa garbage; digit generation truncates instead of rounds; parse underflows representable values to 0 |
| BUG-25 | medium | ICE (on invalid code) | new | #162 | Use-after-move into an aggregate literal: no E500, OSSA verify ICE |
| BUG-26 | medium | ICE (on invalid code) | new | #163 | Use-after-move on a while-loop back edge: MIR block-arg mismatch ICEs |
| BUG-27 | medium | miscompile | known | #107 | Non-Copyable `let` conditionally consumed in `if` drops at the if-merge (early drop) |
| BUG-28 | medium | false-diagnostic | new | #164 | False E503 reading a Copyable field through a tuple element of non-Copyable type |
| BUG-29 | medium | false-diagnostic | new | #165 | E412 false 'duplicate method' for conformances on disjoint specializations |
| BUG-30 | medium | false-diagnostic | new | #166 | Diagnostics inside string interpolation get bogus file-start spans |
| BUG-31 | medium | false-diagnostic | new | #167 | Assoc-type-returning protocol method on a `some P` value rejected |
| BUG-32 | medium | ICE (on invalid/unsupported code) | new | #168 | `some P` as a struct field type panics mir-lower |
| BUG-33 | medium | missing-diagnostic | new | #169 | Out-of-range typed integer literals silently truncate/wrap |
| BUG-34 | medium | other | new | #170 | `Int64(parsing:)` cannot parse `Int64.minValue` |
| BUG-37 | medium | false-diagnostic | new | #173 | E600: any `it`-closure poisons every zero-param closure in the same function body |
| BUG-42 | medium | false-diagnostic | new | #178 | Closure `mutating` param convention not inferred from a `let` variable annotation |
| BUG-43 | medium | false-diagnostic | new | #179 | Dictionary subscript-assignment does not coerce the RHS to the setter's Optional `newValue` |
| BUG-45 | medium | miscompile | new | #181 | Struct field drop order is declaration order, contradicting the documented reverse order (and the init-failure path) |
| BUG-53 | medium | false-diagnostic | new | #189 | False E305: exhaustive payload-split matches reported non-exhaustive |
| BUG-55 | medium | false-diagnostic | new | #191 | `Int64??` fails to parse: `??` is lexed as one token in type position |
| BUG-56 | medium | false-diagnostic | known | #135 | Throws value-promotion fails for computed/operator-expression returns (known: #135 family) |
| BUG-60 | medium | ICE | new | #195 | Closure whose tail is a bare ref-returning call fails OSSA verify (ICE) |
| BUG-61 | medium | false-diagnostic | new | #196 | `opt ?? refReturningCall()` rejected with E491 + inverted type mismatch |
| BUG-66 | medium | ICE | new | #200 | Error-typed expression inside string interpolation reaches post-mono verify (ICE; real diagnostic swallowed) |
| BUG-68 | medium | false-diagnostic | new | #202 | `guard else` block ending in a never-typed call rejected with E003 |
| BUG-74 | medium | runtime-crash | new | #208 | Deep recursive drop of a 30k-node recursive enum chain SIGSEGVs (synthesized drop glue recursion) |
| BUG-75 | medium | other | new | #209 | Every type-inference diagnostic is emitted twice (uncoded copy + [E100] copy) |
| BUG-77 | medium | false-diagnostic | new | #211 | Int+float literal pair unifies in generic calls but is rejected in array literals and if-branches |
| BUG-79 | medium | false-diagnostic | new | #213 | E454 false: constrained generic-protocol-extension method not recognized as a conformance witness |
| BUG-81 | medium | false-diagnostic | new | #215 | `extend (): P` gaps: static `-> Self` requirement rejected (E458); associated-type bindings not registered |
| BUG-35 | low | other | new | #171 | Zero-pad format spec places the minus sign after the pad zeros |
| BUG-36 | low | false-diagnostic | new | #172 | Literal-default inconsistency: `let q: Float64 = 7 / 2` rejected |
| BUG-69 | low | false-diagnostic | new | #203 | `try` cannot appear in bare `if`/`while` condition position |
| BUG-71 | low | other | new | #205 | `fatalError` discards its message argument; panic path prints nothing at all |
| BUG-76 | low | other | new | #210 | Overload-ambiguity diagnostic renders the receiver of module-level functions as `Error` |

---

## BUG-01 — Assignment through a getter-produced intermediate is silently lost
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#139](https://github.com/kestrellang/kestrel/issues/139)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_accessors_setters_0/min_r2_x_accessors_setters_0.ks`

```
cd $REPO/temp/bughunt/x_accessors_setters
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build m02_chain_write_noop_min.ks -o m02 && ./m02
```

**Expected:** `t1=42 / t2=10` — `o.proxy.x = 42` writes through the computed property's setter; `arr(0).x = 10` writes through the Array subscript setter (or the assignment is rejected).
**Actual:** `t1=1 / t2=1` — both writes compile cleanly (exit 0) and are silently discarded.

MIR shows the root cause: the getter is called, the result is stored into an owned temp, the field store goes into the temp, and the temp is dropped — the corresponding setter is never invoked. Reproduced for computed-property chains, subscript-through-computed-property, two-level computed chains, and `arr(i).field = v`. These are the surviving siblings of the previously fixed `try_lower_setter_assign` no-op class (`obj.field(i) = v`): any chain whose INTERMEDIATE link goes through a getter loses the write. Direct setter writes work. Identical on llvm and -O2. **Suspected subsystem: hir-lower (setter-assign / accessor writeback lowering).**

## BUG-02 — Compound assignment (`+=`) silently no-ops for computed-prop / static / global LHS
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#140](https://github.com/kestrellang/kestrel/issues/140)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_accessors_setters_1/min_r2_x_accessors_setters_1.ks`

```
cd $REPO/temp/bughunt/x_accessors_setters
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build m01_compound_noop_min.ks -o m01 && ./m01
```

**Expected:** `t1=1 t2=1 t3=1 t4=1` — instance computed prop, static STORED var, global STORED var, and static computed prop each increment by 1 (or get a diagnostic like the E202 issued for subscript-LHS compound).
**Actual:** all four print 0 — every update silently dropped, exit 0.

Subscript-LHS compound correctly errors E202 (known stage-1.5 gap), but these four LHS forms sail through. Crucially the global/static stored cases need no accessor writeback at all — direct `=` to them works — so the compound-assignment desugar appears to resolve every non-local LHS root to an rvalue temp. Also reproduces for enum static computed and global computed props. Identical on llvm and -O2. Real-world impact is vicious: `drops += 1` against a global counter silently never increments. **Suspected subsystem: hir-lower (compound-assignment desugar place resolution).**

## BUG-03 — Whole-`self` writes in mutating methods of non-Copyable types are unsound
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 1 (round-1 `Optional.take()/replace()` finding)
**Issue:** [#141](https://github.com/kestrellang/kestrel/issues/141)
**Repro:** `$REPO/temp/bughunt/verify/r2_copy_move_r2_1/min_r2_copy_move_r2_1.ks`

```
cd $REPO/temp/bughunt/copy_move_r2
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build min_self_assign_leak.ks -o /tmp/msal && /tmp/msal
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p02_manual_take.ks -o /tmp/p02 && /tmp/p02
# stdlib surface (round-1 finding, merged here):
cd $REPO/temp/bughunt/copy_move
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p11_take_miscompile_min.ks -o p11 && ./p11
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p12_replace.ks -o p12 && ./p12
```

**Expected:** `after_reset=1 / final=2` (old value drops at `self = Res(id: 9)`); p02 `t2=7 / t2b=1`; stdlib `Optional.take()` returns the payload and the old value's deinit runs.
**Actual:** `after_reset=0 / final=1` — the old self value NEVER deinits (leak). In `let old = self; self = .N; return old;` the binding `old` ALIASES the self slot and observes the freshly-stored value (p02 prints `t2=NONE / t2b=0`). Identical on llvm and -O2.

One broken primitive, two observable defects: `self = new` in a mutating method of a non-Copyable type is a raw overwrite with no drop of the old value, and `let old = self` is a by-ref alias rather than a move. This directly miscompiles stdlib `Optional[T].take()`/`replace()` for non-Copyable payloads (`take()` returns None and leaks the payload; `replace()` returns the NEW value as the old one — std/result/optional.ks:443 uses exactly this pattern). Field stores through mutating self (`self.r = n`) and plain local `var` reassignment are both correct — only the whole-`self` place is broken. The hand-written pattern on some concrete shapes is rejected with E503 while the generic body is accepted, so per-instantiation copy gating inside generic bodies is also implicated. **Suspected subsystem: mir-lower (lowering of the whole-`self` place in mutating methods); secondary: per-instantiation E503 gating at mono-expand.**

## BUG-04 — Copyable mono-substitution gap: `Array.retain` corrupts memory; unconstrained generic struct ICEs
**Severity:** critical | **Class:** miscompile | **Status:** known-variant (matches the internal memory note "Copyable mono-substitution gap"; not on the provided known-issue list — the silent-corruption surface is new) | **Merged:** 1 (round-1 `Box[T]` post-mono ICE finding)
**Issue:** [#142](https://github.com/kestrellang/kestrel/issues/142)
**Repro:** `$REPO/temp/bughunt/verify/r2_copy_move_r2_2/min_r2_copy_move_r2_2.ks`

```
cd $REPO/temp/bughunt/copy_move_r2
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p22b_retain_string_payload.ks -o /tmp/p22b && /tmp/p22b; echo "run-rc=$?"
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p22_retain_noncopyable.ks -o /tmp/p22 && /tmp/p22
# ICE surface (round-1, merged here):
cd $REPO/temp/bughunt/copy_move
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p04a_box_ice.ks -o /tmp/p04a
```

**Expected:** retain with non-Copyable elements either works move-correctly (p22b `t1=2`, p22 `t1=1 / t1b=3`) or is rejected at the instantiation site; `struct Box[T] { var value: T }` + `Box(value: Res(id: 3))` compiles per-instantiation (unconstrained `Optional[T]` works) or gets a frontend diagnostic.
**Actual:** p22b SIGSEGVs deterministically on both backends; p22 shows 8 deinits for 3 logical values (double-free class). The Box program ICEs in post-mono verify ("Copyable type MonoStruct(...) contains non-Copyable field 'value'") with the span pointing at an unrelated stdlib function (`dictionary.ks:64 nextPowerOfTwo`).

One root cause, two surfaces: the frontend never rejects (or re-folds) a non-Copyable type substituted into Copyable-assuming generic code at monomorphization. When the post-mono verifier's invariant happens to trip you get an ICE with garbage span attribution (Box); when it doesn't (stdlib `Array.retain`'s `ptr.read()` loop, array.ks:920) the bit-copy of non-Copyable elements monomorphizes silently and corrupts the heap. Other `ptr.read()`-based Array mutators (removeAll(consuming where:), sort, reverse, swap) are likely equally affected; append/pop/remove(at:) are correct. **Suspected subsystem: mono-expand / type-infer (per-instantiation Copyable bound enforcement), with stdlib Array mutators as the dangerous surface.**

## BUG-05 — Mutating-parameter writeback lost for tuple-element writes and destructured params
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#143](https://github.com/kestrellang/kestrel/issues/143)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_init_paths_4/min_r2_x_init_paths_4.ks`

```
cd $REPO/temp/bughunt/x_init_paths
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p21_tuple_writeback_variants.ks -o p21 && ./p21
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p10_mutating_destructure.ks -o p10 && ./p10
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p14_mutating_writeback_controls.ks -o p14 && ./p14
```

**Expected:** p21 `t1=10,2 / t2=30,40`; p10 `t1=10,20 / t2=6`; controls p14 `t1=99 / t2=10,2 / t3=7`.
**Actual:** tuple ELEMENT assignment through `mutating t: (Int64,Int64)` (`t.0 = 10`) silently discarded (p21 t1=1,2); ALL writes through destructured mutating params (`mutating (a, b): (...)`, `mutating Point { x, .. }: Point`) silently discarded (p10). Whole-tuple reassignment, scalar assignment, and struct-field assignment write back correctly.

Two adjacent gaps with the same observable: tuple-element projection stores through a mutating param are lowered against a local copy, and destructured params bind fresh locals with no writeback epilogue. All forms compile cleanly — the suite's `mutating_mode_tuple_is_mutable.ks` only asserts compilation, never the value. Identical on llvm and -O2. Distinct from the known subscript-compound stage-1.5 gap (no subscripts/compound involved). Likely shares "tuple projections aren't places" DNA with BUG-28. **Suspected subsystem: mir-lower (mutating-param writeback epilogue; tuple element place projection).**

## BUG-06 — Delegation to a failable init ignores failure
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#144](https://github.com/kestrellang/kestrel/issues/144)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_init_paths_3/min_r2_x_init_paths_3.ks`

```
cd $REPO/temp/bughunt/x_init_paths
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p18_failable_deleg_failable_min.ks -o p18 && ./p18
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p19_nonfailable_deleg_failable.ks -o p19 && ./p19
```

**Expected:** p18 `t1=none d=1` (outer `init?` propagates the inner `init?`'s failure as None); p19 should be a compile error (non-failable init cannot delegate to a failable init).
**Actual:** p18 prints `t1=some id=11 d=1` — the outer init returns `Some(self)` even though the inner init failed; `self.a` is the STALE already-deinited value, which is then deinited AGAIN at scope exit (double-drop; the two-field variant shows 3 deinits where 1 is correct). p19 compiles and exhibits the same stale read from a non-failable outer init.

`self.init(innerFailable: ...)` is lowered as an unconditional delegation call; the inner init's Optional result is discarded and the outer body continues with `self` never (re)initialized. Both failable->failable (must propagate None) and non-failable->failable (should be rejected) are affected; with heap fields this is use-after-free/double-free. No testdata exercises failable delegation — the surface is silently accepted but unimplemented. Identical on llvm and -O2. **Suspected subsystem: hir-lower/mir-lower (init delegation lowering + missing diagnostic in analyzer).**

## BUG-07 — Moving a non-Copyable field out of consuming self: silent failed build that still emits a double-dropping binary
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0 (the silent-diagnostic and binary-emission components are tracked as BUG-15 / BUG-14)
**Issue:** [#145](https://github.com/kestrellang/kestrel/issues/145)
**Repro:** `$REPO/temp/bughunt/verify/r2_copy_move_r2_0/min_r2_copy_move_r2_0.ks`

```
cd $REPO/temp/bughunt/copy_move_r2
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build min_into_field.ks -o /tmp/mif; echo "build-rc=$?"
/tmp/mif; echo "run-rc=$?"
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build min_into_field_heap.ks -o /tmp/mifh; /tmp/mifh
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel dump mir min_into_field.ks | grep -A8 intoInner
```

**Expected:** either a rendered diagnostic and no binary, or (preferably, since `self` is consuming) a successful build printing `id=8 / drops=1`; heap variant `name=omega_omega_omega_omega_omega_omega`.
**Actual:** build exits rc=1 with NO output at all, yet the binary IS written and runs: `drops=2` (one logical value deinits twice); the String-payload variant prints corrupted output (`oame=`) from a double-freed heap buffer.

MIR shows the lowering defect directly: `begin_borrow self; struct_extract .0 (@guaranteed); move_value (bit-copy out of the borrow); destroy_value %v0` — the extracted field is bit-copied while the WHOLE self (including that field) is destroyed. Same for a free function with a `consuming` param. The non-consuming version correctly renders E503; the consuming version's error is counted but never rendered (BUG-15 shape) and codegen proceeds anyway (BUG-14 shape). The enum counterpart (consuming method matching on self, returning the payload) is correct — this is specific to struct field projection out of consuming self. Identical on llvm and -O2. **Suspected subsystem: mir-lower (consuming-self field move lowered as borrow + bit-copy + whole destroy); kestrel-analyze should also permit field moves out of consuming self rather than counting an E503.**

## BUG-08 — `Self`-keyed static requirements in protocol default impls never witness-resolved at mono
**Severity:** high | **Class:** ICE | **Status:** new (identical symptom string to the known overlapping-conformance "Callee::Witness not resolved" issue, but verified disjoint trigger: no overlapping conformances involved) | **Merged:** 5
**Issue:** [#146](https://github.com/kestrellang/kestrel/issues/146)
**Repro:** `$REPO/temp/bughunt/verify/r1_generics_protocols_0/r12_min.ks`

```
cd $REPO/temp/bughunt/generics_protocols
export KESTREL_STD=$REPO/temp/stable-kestrel/std
# (a) static FUNC requirement -> post-mono ICE, build fails:
$REPO/temp/stable-kestrel/kestrel build r12_min.ks -o r12_min
# (b) static VAR requirement -> function skipped, build EXITS 0, binary traps at runtime:
cd $REPO/temp/bughunt/generics_protocols_r2
$REPO/temp/stable-kestrel/kestrel build g42b_custom.ks -o g42b; echo build_exit=$?; ./g42b; echo run_exit=$?
# (c) init() requirement -> mangler panic:
cd $REPO/temp/bughunt/x_opaque_static_dispatch
$REPO/temp/stable-kestrel/kestrel build r09a_self_init_direct.ks -o r09a
```

**Expected:** all of these are textbook protocol-extension code (`extend P { func m() { ... Self.staticReq() ... } }`, `Self.staticVarReq`, `Self()`) and should compile and run.
**Actual:** three failure modes by requirement kind: **static func** -> `post-mono verify failed ... Callee::Witness not resolved`, build fails (r12_min; g36 static-default variant; r09c/r09e direct and generic-bound calls); **init()** -> compiler panic `mangle_type: TypeParam(... name=Test.Springy) reached the mangler`, exit 101 (r09a); **static var** -> the unresolved witness ESCAPES the verifier, the function is "skipped" at codegen, build exits 0 with only a warning, and the binary traps SIGILL/SIGTRAP at runtime with no output (g42a/g42b, r13a/b/c, p13 — read and write, get-only and settable, instance and static default impls). Both backends.

Merged findings (6 total -> 1): round-1 r12 instance-default ICE; r2 g36 static-default ICE; r2 x_opaque r09c/r09e `Self.staticMethod()` ICE (verified identical shape to r12); r2 g42 static-var trap; r2 x_opaque r13 static-var trap (identical shape to g42); r09a `Self()` mangler panic. Delimiters that work: `T.staticReq()` from generic free functions, instance requirements from default bodies, conformers that override the default. One fix should cover all: substitute/witness-resolve `Self` static-position requirements when expanding default-impl bodies at monomorphization. The static-var mode additionally rides BUG-13 (exit-0 trap-stub policy) into shipped binaries. **Suspected subsystem: mono-expand (witness resolution for Self static requirements in protocol-extension bodies).**

## BUG-09 — Stored `static var` witness ICEs through a type parameter
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#147](https://github.com/kestrellang/kestrel/issues/147)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_opaque_static_dispatch_3/r16_stored_static_witness_min.ks`

```
cd $REPO/temp/bughunt/x_opaque_static_dispatch
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build r16_stored_static_witness_min.ks -o r16
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build r16b_stored_static_direct.ks -o r16b && ./r16b
```

**Expected:** `protocol Counter { static var count: Int64 { get set } }` witnessed by `static var count: Int64 = 0` and read via `func peek[T]() -> Int64 where T: Counter { T.count }` builds and prints `t1=0`.
**Actual:** `post-mono verify failed ... Callee::Witness not resolved` (writes ICE twice, get+set). Both backends. Direct non-generic `C.count` access works (r16b).

Distinct from BUG-08: no protocol extensions are involved. The frontend accepts a stored static var as a witness for a `static var { get set }` requirement, but witness lowering only knows how to handle computed accessor pairs (the testdata-style explicit get/set witness works through `T`). Fix: synthesize accessors for stored static witnesses, or diagnose the conformance. **Suspected subsystem: mono-expand / witness lowering.**

## BUG-10 — Generic default-parameter expression calling `T.staticMethod()` ICEs post-mono
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#148](https://github.com/kestrellang/kestrel/issues/148)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_init_paths_0/p16_ice_min.ks`

```
cd $REPO/temp/bughunt/x_init_paths
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p16_ice_min.ks -o p16
```

**Expected:** `func pick[T](x x: T = T.makeDefault()) -> T where T: Defaultable` called as `let v: Int64 = pick();` builds and prints `t1=42`.
**Actual:** `post-mono verify failed in '_K0N4_Test4_mainERi4': TypeParam(Entity(4782)) in value 0` pointing at `T.makeDefault()`, plus a second ICE at the call site. Same on llvm.

The T-dependent default expression is type-checked/lowered once against the generic TypeParam and spliced into the caller without mono substitution — providing the argument explicitly (`pick(x: 7)`) compiles and runs fine, so the ICE fires only when the default is actually instantiated. Related in spirit to BUG-08 (unsubstituted generic context at mono) but a different mechanism (default-arg splicing) and panic. **Suspected subsystem: mono-expand (default-argument expression substitution).**

## BUG-11 — Default-parameter subscript WRITE with argument omitted: trap binary with exit 0
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#149](https://github.com/kestrellang/kestrel/issues/149)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_accessors_setters_2/c07_default_subscript_write_omitted.ks`

```
cd $REPO/temp/bughunt/x_accessors_setters
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build c07_default_subscript_write_omitted.ks -o c07; echo "build_exit=$?"
./c07; echo "run_exit=$?"
```

**Expected:** clean build, `t1=10` (`c() = 10` invokes the setter with the defaulted index `i=0`).
**Actual:** build prints `warning: 1 of 2523 functions failed to compile (skipped)` but exits 0; the binary prints nothing and dies SIGILL (132 cranelift / 133 llvm).

MIR shows the setter has 3 params (self, i, newValue) but the call site emits only 2 args — the defaulted index is never materialized on the WRITE path. The read path with the arg omitted works, and the write with the arg provided works. The trap-stub-on-success behavior is BUG-13. **Suspected subsystem: hir-lower / arg-binding on the subscript setter write path (defaulted args not bound).**

## BUG-12 — Static stored var counted as a memberwise-init field
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#150](https://github.com/kestrellang/kestrel/issues/150)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_accessors_setters_3/m04b_static_memberwise_provided.ks`

```
cd $REPO/temp/bughunt/x_accessors_setters
# arm 1: valid memberwise call falsely rejected
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build m04_static_in_memberwise.ks -o m04
# arm 2: appeasing the bogus arity -> codegen panic + exit-0 trap binary
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build m04b_static_memberwise_provided.ks -o m04b; echo "build_exit=$?"
./m04b; echo "run_exit=$?"
```

**Expected:** `struct S { var v: Int64; static var sv: Int64 = 5; }` has a one-field memberwise init: `S(v: 0)` compiles; `S(v: 0, sv: 77)` is rejected normally.
**Actual:** `S(v: 0)` -> false `error: struct 'S' has 2 field(s), but 1 argument(s) were provided [E100]`. `S(v: 0, sv: 77)` -> codegen panic (`inst.rs:1701 index out of bounds: the len is 1 but the index is 1`; llvm panics at its inst.rs:1722), downgraded to a "skipped" warning, build exits 0, binary SIGILLs.

Single root cause: memberwise-initializer synthesis includes static stored vars in the field list while the type layout (correctly) has only instance fields, so the synthesized init indexes field 1 of a 1-field struct. Workaround: declare any explicit init. Static var machinery itself (reads, direct writes, computed accessors) works. **Suspected subsystem: ast-builder / analyzer (memberwise init synthesis must exclude statics).**

## BUG-13 — Any codegen failure downgraded to a warning: exit 0, trap-stub binary ships
**Severity:** high | **Class:** other | **Status:** new | **Merged:** 0 (cross-cutting policy defect independently flagged by four findings)
**Issue:** [#151](https://github.com/kestrellang/kestrel/issues/151)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_accessors_setters_2/c07_default_subscript_write_omitted.ks` (also g42b, r13c, m04b)

```
cd $REPO/temp/bughunt/x_accessors_setters
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build c07_default_subscript_write_omitted.ks -o c07; echo "build_exit=$?"   # 0
./c07; echo "run_exit=$?"   # 132, no output
```

**Expected:** a function that fails to compile is a build error: nonzero exit, no binary (or at minimum an error, not a warning).
**Actual:** `warning: N of M functions failed to compile (skipped): ...` + exit 0; the failed function becomes a trap stub and the binary crashes at runtime with no output.

This policy converts every codegen-stage bug (BUG-08 static-var mode, BUG-11, BUG-12 — including outright backend PANICS in m04b) into a "successful" build that ships a crashing executable — the worst possible failure mode for CI and for users. Reachable-function codegen failure should fail the build. **Suspected subsystem: driver / codegen failure policy (both backends).**

## BUG-14 — Failed builds (exit 1) still write the output executable
**Severity:** high | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#152](https://github.com/kestrellang/kestrel/issues/152)
**Repro:** `$REPO/temp/bughunt/verify/r2_generics_protocols_r2_1/g17b_return.ks`

```
cd $REPO/temp/bughunt/generics_protocols_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
rm -f g17b
$REPO/temp/stable-kestrel/kestrel build g17b_return.ks -o g17b; echo build_exit=$?   # 1, E503 printed
ls -la g17b   # binary exists despite failed build
./g17b; echo run_exit=$?
```

**Expected:** a build that exits nonzero must not leave a runnable executable at the `-o` path (frontend type errors correctly emit no binary — g18_type_error.ks).
**Actual:** E503 is reported, `kestrel build` exits 1, yet a fresh 1.1MB executable is written and runs — and it exhibits exactly the unsound semantics the checker rejected (g13c: the moved-out field deinits twice; double-free class for heap payloads).

Late kestrel-analyze (move checker) errors do not gate codegen/link/emission; only frontend errors do. Combined with BUG-15, a failed build can be completely indistinguishable from a successful one. Also the emission path for BUG-07's binaries. **Suspected subsystem: driver (analysis errors must gate codegen/link/emit).**

## BUG-15 — E503 in tail-expression position fails the build silently
**Severity:** high | **Class:** missing-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#153](https://github.com/kestrellang/kestrel/issues/153)
**Repro:** `$REPO/temp/bughunt/verify/r2_generics_protocols_r2_2/g17a_tail.ks`

```
cd $REPO/temp/bughunt/generics_protocols_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build g17a_tail.ks -o g17a 2>/tmp/a.err; echo exit=$?; wc -c /tmp/a.err   # exit=1, 0 bytes
$REPO/temp/stable-kestrel/kestrel build g17b_return.ks -o g17b 2>/tmp/b.err; echo exit=$?; cat /tmp/b.err  # exit=1, E503 printed
```

**Expected:** g17a and g17b are identical except `{ self.inner }` vs `{ return self.inner; }` — both should print the same E503.
**Actual:** the return-statement version prints E503 with a correct span; the tail-expression version exits 1 with COMPLETELY EMPTY stdout/stderr — the error is counted (build fails) but never rendered. BUG-07's repro exhibits the same silence (`kestrel dump diagnostics` is also empty).

Likely the diagnostic is attached to a synthetic/desugared span that the renderer drops while the error count still fails the build (cf. the load-bearing matcher file_id filter). Combined with BUG-14 the failure is invisible unless the exit code is checked. **Suspected subsystem: analyzer-diagnostics (move-checker span anchoring / diagnostic rendering).**

## BUG-16 — Init bodies don't track initialized fields for drops: overwrite leaks; post-delegation failure leaks
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 1 (failable-after-delegation leak merged with field-overwrite leak: same initialized-state gap)
**Issue:** [#154](https://github.com/kestrellang/kestrel/issues/154)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_init_paths_2/min_r2_x_init_paths_2.ks`

```
cd $REPO/temp/bughunt/x_init_paths
# (a) overwrite of an already-initialized field inside init leaks the old value:
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p13_init_field_overwrite_control.ks -o p13 && ./p13
# (b) init? failing AFTER self.init(...) delegation leaks all delegated-initialized fields:
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p17_failable_fail_after_full_init.ks -o p17 && ./p17
```

**Expected:** p13 `t1=1 / t2=2` (first `self.a = Res(id:1)` drops when overwritten); p17 `t2=1 none=true` (failure after delegation drops the delegated-initialized field).
**Actual:** p13 `t1=0 / t2=1` — the first value is never deinited; p17 `t2=0` — the failure path drops nothing after delegation. Direct-assign-then-fail in the same file drops correctly (t1=1). Identical on llvm and -O2.

One state gap, two leaks: the init body's "fields initialized so far" tracking does not (a) treat assignment to an already-initialized field as overwrite-with-drop (it is lowered as raw first-initialization — var-field reassignment in inits is intended-legal per testdata, and works correctly OUTSIDE inits), nor (b) set drop flags when initialization happens through `self.init(...)` delegation, so the failable-init partial-drop machinery (recently fixed for direct assignment) sees nothing to drop. Edge of the failable_init_partial_drop fix. **Suspected subsystem: mir-lower (init definite-initialization / drop-flag state).**

## BUG-17 — `Array.clear()` never runs element deinits
**Severity:** high | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#155](https://github.com/kestrellang/kestrel/issues/155)
**Repro:** `$REPO/temp/bughunt/verify/r2_copy_move_r2_3/min_array_clear_leak.ks`

```
cd $REPO/temp/bughunt/copy_move_r2
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build min_array_clear_leak.ks -o /tmp/acl && /tmp/acl
```

**Expected:** `after_clear=2 / after_scope=2`.
**Actual:** `after_clear=0 / after_scope=0` — neither `clear()` nor the later array drop ever deinits the 2 elements; they leak forever.

Pure stdlib bug: `temp/stable-kestrel/std/collections/array.ks:902` `clear()` is `self.makeUnique(); self.storage.modify { (mutating s) in s.len = 0 }` — no element destruction, and `ArrayStorage.deinit` only drops `0..len`, which is now 0. Affects Copyable-with-heap elements (String arrays leak their buffers) as well as non-Copyable. Same on llvm/-O2. **Suspected subsystem: stdlib (array.ks clear()).**

## BUG-18 — #127: array literal with non-Copyable element fires a spurious deinit at construction
**Severity:** high | **Class:** miscompile | **Status:** known — tracked as #127 | **Merged:** 0
**Repro:** `$REPO/temp/bughunt/verify/r1_copy_move_6/min_r1_copy_move_6.ks`

```
cd $REPO/temp/bughunt/copy_move
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p14_array_wildcard.ks -o p14 && ./p14
```

**Expected:** `t1=0 t1b=1 t2=2` — building `[Res(id:1)]` is a move: zero deinits during construction, exactly one when the array drops.
**Actual:** `t1=1 t1b=2 t2=2` — one spurious deinit of the moved-from temp at construction, then the stored element deinits again when the array drops: 2 deinits for 1 logical value (double-free class).

Matches the skipped suite test `memory_model/copy_semantics/array_literal_notcopyable_no_spurious_deinit.ks` exactly. The wildcard tuple-destructure case in the same file is clean. **Suspected subsystem: mir-lower (array-literal element ownership).**

## BUG-19 — Formatting any signed `minValue` emits mirrored garbage digits in every radix
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 1 (round-2 `:x/:b/:o` radix finding merged: same negate-wrap root cause)
**Issue:** [#156](https://github.com/kestrellang/kestrel/issues/156)
**Repro:** `$REPO/temp/bughunt/verify/r1_numerics_0/r01_min_print.ks`

```
cd $REPO/temp/bughunt/numerics
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build r01_min_print.ks -o r01_min_print && ./r01_min_print
# radix flavor:
cd $REPO/temp/bughunt/numerics_r2
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build r05_min_radix_format.ks -o r05 && ./r05
```

**Expected:** `t1=-128 t2=-32768 t3=-2147483648 t4=-9223372036854775808 t5=-127`; radix: `-80`, `-8000000000000000`, `-10000000`, `-100000`.
**Actual:** every digit d of |min| is emitted as ASCII `0x30 - d` instead of `0x30 + d` (`t1=-/.(` etc.; hex `-(0`, binary `-/00000`); all non-min negatives print fine in all radices.

Root cause read from `std/numeric/int64.ks` format() (~line 1016): `if isNegative { n = n.negate() }` wraps at minValue (negate(min) == min, still negative); the digit loop then computes negative remainders and `digitVal + 48` produces bytes `48 - d`. Identical on cranelift/LLVM, -O0/-O2 — stdlib, not backend. One fix (format via unsigned magnitude or special-case minValue) covers the decimal print path and all `:x/:b/:o` format-spec paths. Any wrapped-to-min arithmetic result silently emits corrupt text. **Suspected subsystem: stdlib (integer format/radix digit generation).**

## BUG-20 — NaN ordered comparisons via `<=` / `>=` return true
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#157](https://github.com/kestrellang/kestrel/issues/157)
**Repro:** `$REPO/temp/bughunt/verify/r1_numerics_4/r02_nan_cmp.ks`

```
cd $REPO/temp/bughunt/numerics
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build r02_nan_cmp.ks -o r02 && ./r02
```

**Expected:** all eight comparisons false (IEEE 754; float64.ks's own header docs: "every ordered comparison against NaN is false").
**Actual:** `nan<=nan`, `nan>=nan`, `nan<=1.0`, `1.0<=nan` all true; strict `<`/`>`/`==` correct. Both backends.

Root cause: Float64's `<=`/`>=` are derived from `Comparable.compare()` (std/numeric/float64.ks ~line 375), which falls through to `.Equal` when neither `f64_lt` nor `f64_gt` holds — NaN is treated as equal to everything for the derived operators, while `<`/`>`/`==` use the raw IEEE intrinsics. Deriving a total-order three-way compare for a partial order is unsound; Float64/Float32 need direct `f64_le`/`f64_ge` operator impls (the compare() doc-comment itself admits the fall-through "is wrong"). **Suspected subsystem: stdlib (float Comparable-derived operators).**

## BUG-21 — Integer division/modulo by zero does not trap on the LLVM backend
**Severity:** high | **Class:** backend-divergence | **Status:** new | **Merged:** 0
**Issue:** [#158](https://github.com/kestrellang/kestrel/issues/158)
**Repro:** `$REPO/temp/bughunt/verify/r1_numerics_1/t03_div_zero.ks`

```
cd $REPO/temp/bughunt/numerics
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build t03_div_zero.ks -o t03_cl && ./t03_cl; echo "cl exit=$?"
KESTREL_STD=$REPO/temp/stable-kestrel/std KESTREL_BACKEND=llvm $REPO/temp/stable-kestrel/kestrel build t03_div_zero.ks -o t03_ll && ./t03_ll; echo "ll exit=$?"
# also t03b_mod_zero.ks (5 % 0) and t16_hex_range_udiv.ks (unsigned 7 / 0)
```

**Expected:** both backends print `before=ok` then trap (divide() docs: "Traps on division by zero"). Cranelift does exactly this (exit 132).
**Actual:** LLVM never traps: `5 / zero()` -> 0, exit 0; `5 % zero()` -> 5 at -O0 but 0 at -O2 (LLVM diverging from itself); unsigned `7 / 0` -> 0.

LLVM sdiv/udiv/srem by zero is UB; AArch64 hardware happens to return 0 (msub gives lhs for rem), but this is real UB the optimizer already exploits. The codegen-llvm intrinsic lowering needs an explicit zero-check trap to match the docs and cranelift. **Suspected subsystem: codegen-llvm (division intrinsic lowering).**

## BUG-22 — `Int64.min / -1`: three different behaviors; docs say wrap
**Severity:** high | **Class:** backend-divergence | **Status:** new | **Merged:** 0
**Issue:** [#159](https://github.com/kestrellang/kestrel/issues/159)
**Repro:** `$REPO/temp/bughunt/numerics/t02_div_min.ks`

```
cd $REPO/temp/bughunt/numerics
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build t02_div_min.ks -o t02_cl && ./t02_cl; echo "cl exit=$?"
KESTREL_STD=$REPO/temp/stable-kestrel/std KESTREL_BACKEND=llvm $REPO/temp/stable-kestrel/kestrel build t02_div_min.ks -o t02_ll && ./t02_ll; echo "ll exit=$?"
KESTREL_STD=$REPO/temp/stable-kestrel/std KESTREL_BACKEND=llvm $REPO/temp/stable-kestrel/kestrel build t02_div_min.ks -O2 -o t02_ll2 && ./t02_ll2; echo "ll -O2 exit=$?"
```

**Expected:** per divide() docs ("For signed types, minValue / -1 wraps"): wraps to min, exit 0, identical everywhere.
**Actual:** cranelift traps immediately (exit 132, nothing printed, -O0 and -O2); LLVM -O0 wraps as documented (by accident); LLVM -O2 constant-folds the poison to 0.

Cranelift sdiv has a built-in INT_MIN/-1 overflow trap; LLVM sdiv is UB on that input. The documented wrap semantics are matched only by LLVM -O0 by luck. Same intrinsic family as BUG-21 but a distinct edge with a distinct documented contract (wrap, not trap), needing fixes on BOTH backends; also the trigger inside BUG-23's checked-math bug. `min % -1` alone is consistent (0); `divideChecked` correctly returns None. **Suspected subsystem: codegen-cranelift + codegen-llvm (signed division overflow contract).**

## BUG-23 — `multiplyChecked/Saturating(minValue, -1)`: trap on cranelift, wrong answer on LLVM -O0
**Severity:** high | **Class:** runtime-crash | **Status:** new | **Merged:** 0
**Issue:** [#160](https://github.com/kestrellang/kestrel/issues/160)
**Repro:** `$REPO/temp/bughunt/verify/r2_numerics_r2_0/min_r2_numerics_r2_0.ks`

```
cd $REPO/temp/bughunt/numerics_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
K=$REPO/temp/stable-kestrel/kestrel
$K build r01_mulchecked_min.ks -o r01_cl && ./r01_cl; echo "exit=$?"
KESTREL_BACKEND=llvm $K build r01_mulchecked_min.ks -o r01_ll && ./r01_ll; echo "exit=$?"
KESTREL_BACKEND=llvm $K build r01_mulchecked_min.ks -O2 -o r01_llO2 && ./r01_llO2; echo "exit=$?"
```

**Expected:** `t1=none / t2=127`, exit 0 (multiplyChecked detects the overflow of -128 x -1 = 128; multiplySaturating clamps to maxValue).
**Actual:** cranelift: trap, exit 132, no output. LLVM -O0: returns `Some(-128)` / `-128` (wrong). LLVM -O2: correct. Three behaviors for one documented-safe API; all signed widths share the template.

Root cause: `std/numeric/int8.ks` multiplyChecked validates overflow via divide-back (`result.divide(other)`); for min x -1 the wrapped product is min, so the check evaluates minValue / -1 — which traps on cranelift and wraps to min on LLVM, making the check wrongly pass. Even with consistent divide semantics (BUG-22), divide-back is logically incapable of detecting min x -1 overflow, so this stdlib bug needs its own fix. **Suspected subsystem: stdlib (checked/saturating multiply validation), compounded by BUG-22.**

## BUG-24 — Float64 decimal digit generation emits wrong digits (`:.n` paths and large-magnitude shortest print)
**Severity:** high | **Class:** other | **Status:** new | **Merged:** 1 (round-1 large-magnitude shortest-print finding merged: shared digit-generation core in float64.ks)
**Issue:** [#161](https://github.com/kestrellang/kestrel/issues/161)
**Repro:** `$REPO/temp/bughunt/verify/r2_numerics_r2_2/r03_precision_digits.ks`

```
cd $REPO/temp/bughunt/numerics_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
K=$REPO/temp/stable-kestrel/kestrel
$K build r03_precision_digits.ks -o r03 && ./r03; echo "exit=$?"
# large-magnitude shortest-print flavor:
cd $REPO/temp/bughunt/numerics
$K build r03_float_digits.ks -o r03b && ./r03b
```

**Expected:** `1/3 :.4` -> 0.3333; `0.125 :.2` -> 0.12 or 0.13 (exactly representable; never 0.11); `0.35 :.1` -> 0.3 (stored value is below 0.35); shortest print of 2^63 -> 9.223372e18 (true digits 9223372...).
**Actual:** `0.3332`, `0.11`, `0.4`, `...333332`; and 2^63 -> 9.223371e18, 2^53 -> 9.007198e15, 2^62 -> 4.611685e18 — the printed decimal denotes a DIFFERENT f64 than the one stored. Identical on cranelift and LLVM, -O0 and -O2.

Multiple wrongnesses with one shared signature ("last digit one low") plus precision-path-specific failures (0.125:.2 -> 0.11 is off by a full hundredth on an exact input; 0.35:.1 rounds the decimal numeral half-up instead of the stored binary value). The digit-generation core extracts digits via floating-point divides by powers of 10, accumulating error (float64.ks ~lines 1076-1210). Notably `:.17` of 0.1+0.2 prints the fully correct 0.30000000000000004, so correct digits are obtainable internally; the n<17 and shortest-print paths corrupt them. **Suspected subsystem: stdlib (float64.ks format digit generation; needs integer-based / dragon-ryu-style digit extraction).**

## BUG-25 — Use-after-move into an aggregate literal: no E500, OSSA verify ICE
**Severity:** medium | **Class:** ICE (on invalid code) | **Status:** new | **Merged:** 0
**Issue:** [#162](https://github.com/kestrellang/kestrel/issues/162)
**Repro:** `$REPO/temp/bughunt/verify/r1_copy_move_1/d01_move_struct_then_use.ks`

```
cd $REPO/temp/bughunt/copy_move
for f in d01_move_struct_then_use d02_move_tuple_then_use d03_move_enum_then_use d04_move_array_then_use; do KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build $f.ks -o /tmp/d; done
```

**Expected:** `error: use of moved value 'r' [E500]` — exactly what the consuming-method and let-rebind cases already get.
**Actual:** `internal compiler error: OSSA verify failed ... use of consumed value ValueId(5)` for all four aggregate forms: `Wrap(inner: r)`, `(r, 10)`, `Holder.Full(r)`, `[r]`.

Single root cause across all four: the HIR move checker tracks consuming calls/methods and let-rebinds but NOT moves into aggregate-literal construction, so invalid reuse sails through to MIR and the OSSA verifier panics. **Suspected subsystem: kestrel-analyze (move checker aggregate-literal consumption tracking).**

## BUG-26 — Use-after-move on a while-loop back edge: MIR block-arg mismatch ICEs
**Severity:** medium | **Class:** ICE (on invalid code) | **Status:** new | **Merged:** 0
**Issue:** [#163](https://github.com/kestrellang/kestrel/issues/163)
**Repro:** `$REPO/temp/bughunt/copy_move/d06_loop_maybe_moved.ks`

```
cd $REPO/temp/bughunt/copy_move
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build d06_loop_maybe_moved.ks -o /tmp/d
```

**Expected:** a "may have been moved" diagnostic for `consume(r)` reachable on the loop back edge (matching the suite exemplar's wording for post-loop uses).
**Actual:** TWO ICEs: `terminator passes 7 args to BlockId(1) but block expects 8 params` and `@owned value ... live at block exit but never consumed`.

Program: `let r = Res(...); while i < 3 { consume(r); i = i + 1; }`. The upstream gap is again the move checker (back-edge use not flagged), but the crash site is distinct from BUG-25: mir-lower's loop merge-slot bookkeeping produces a block-argument arity mismatch, suggesting an independent loop-lowering bug for consumed-in-loop values that would need fixing even after the checker diagnoses this. **Suspected subsystem: kestrel-analyze (back-edge move tracking) + mir-lower (loop merge-slot bookkeeping).**

## BUG-27 — Non-Copyable `let` conditionally consumed in `if` drops at the if-merge (early drop)
**Severity:** medium | **Class:** miscompile | **Status:** known — tracked as #107 (if-merge drop timing; `var` is correct) | **Merged:** 0
**Repro:** `$REPO/temp/bughunt/verify/r1_copy_move_5/min_r1_copy_move_5.ks`

```
cd $REPO/temp/bughunt/copy_move
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p01_branch_timing.ks -o p01 && ./p01
```

**Expected:** `t1=1 t1b=1 t2=0 t2b=1 t3=0` — with cond=false, `r` is lexically alive at the post-if probe.
**Actual:** `t2=1` — the deinit fires at the if-merge even though the branch did not consume it; total count is still exactly 1; the `var` variant has correct timing. Same on llvm.

Hits the documented backlog item exactly (diamond conditional-move let drop timing); observable whenever deinit has side effects ordered against post-merge code. Documented fix direction: address+drop-flags for the consumed `let` (the consume-half of non-Copyable var-read lowering). **Suspected subsystem: mir-lower.**

## BUG-28 — False E503 reading a Copyable field through a tuple element of non-Copyable type
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#164](https://github.com/kestrellang/kestrel/issues/164)
**Repro:** `$REPO/temp/bughunt/copy_move_r2/p06b_tuple_field_read.ks`

```
cd $REPO/temp/bughunt/copy_move_r2
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p06b_tuple_field_read.ks -o /tmp/p06b; echo "build-rc=$?"
```

**Expected:** compiles and prints `t1=1` — `t.0.id` only reads a Copyable Int64 through a borrow of the tuple element; the isomorphic struct path (`w.inner.id`) compiles and runs.
**Actual:** `error[E503]: cannot move non-copyable value of type 'Test.Res' out of a borrow` — the tuple element projection `t.0` is treated as a by-value move instead of a place borrow.

Tuple element access is not place-aware the way struct field access is; this also blocks borrowing method calls through tuple elements. Destructuring and whole-tuple drops work. Likely shares a root with BUG-05's tuple-element store-path failures ("tuple projections aren't places"). **Suspected subsystem: kestrel-analyze / hir-lower (tuple-element place projection).**

## BUG-29 — E412 false 'duplicate method' for conformances on disjoint specializations
**Severity:** medium | **Class:** false-diagnostic | **Status:** new (sibling/dual of the FIXED E412 multi-conformance bug; not the listed E426-intended case) | **Merged:** 0
**Issue:** [#165](https://github.com/kestrellang/kestrel/issues/165)
**Repro:** `$REPO/temp/bughunt/verify/r2_generics_protocols_r2_3/min_r2_generics_protocols_r2_3.ks`

```
cd $REPO/temp/bughunt/generics_protocols_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build g39_disjoint_spec.ks -o g39; echo exit=$?
$REPO/temp/stable-kestrel/kestrel build g41_array_spec.ks -o g41; echo exit=$?
```

**Expected:** `extend Box[Int64]: Show` + `extend Box[String]: Show` are disjoint instantiations — both conformances compile (`t1=int:5 / t2=str:hi`).
**Actual:** `error: duplicate method 'show' in extensions of 'Box' [E412]` — the dedup key uses the bare nominal, ignoring the extension's self-type arguments. Same for imported generics (`Array[Int64]`/`Array[String]` -> duplicate 'tally'); with static Self-returning requirements it cascades into a bogus E458.

The fixed E412 bug covered same-self + different protocol type args; the dual — different SELF instantiations, same protocol — is still keyed only by (nominal, method name, labels). Practical impact: a generic type can conform a protocol on at most ONE specialization, which also breaks the documented "ship specialized-only" workaround for the overlap ICE whenever two specializations are needed. **Suspected subsystem: analyzer-diagnostics (E412 extension-method dedup key must include extension self-type args).**

## BUG-30 — Diagnostics inside string interpolation get bogus file-start spans
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#166](https://github.com/kestrellang/kestrel/issues/166)
**Repro:** `$REPO/temp/bughunt/verify/r2_generics_protocols_r2_4/g38_interp_span.ks`

```
cd $REPO/temp/bughunt/generics_protocols_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build g38_interp_span.ks -o g38
```

**Expected:** "no member 'bogusMember' on type 'String'" with the span on `s.bogusMember` inside `print("t1=\(s.bogusMember)")` (line 6).
**Actual:** the span points at lines 1-3 (module header) — file start — with the highlight width matching the offending expression's length: the expression's segment-relative offset is rendered as file-absolute. Reproduced for three different error kinds; the same call outside an interpolation gets a perfect span.

Affects every diagnostic raised inside `\(...)` — extremely common since print debugging goes through interpolation. The diagnosis TEXT is correct; only the span anchoring is wrong. **Suspected subsystem: parser / diagnostics (interpolation segment span offset mapping).**

## BUG-31 — Assoc-type-returning protocol method on a `some P` value rejected
**Severity:** medium | **Class:** false-diagnostic | **Status:** new (possibly a staged limitation of opaque types, but the rejection is unactionable and mis-anchored) | **Merged:** 0
**Issue:** [#167](https://github.com/kestrellang/kestrel/issues/167)
**Repro:** `$REPO/temp/bughunt/verify/r2_generics_protocols_r2_5/min_r2_generics_protocols_r2_5.ks`

```
cd $REPO/temp/bughunt/generics_protocols_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build g20_opaque_assoc.ks -o g20
$REPO/temp/stable-kestrel/kestrel build g20a_opaque_assoc_min.ks -o g20a && ./g20a
```

**Expected:** `let s = makeSrc(4); s.produce()` — where `makeSrc -> some Producer` and Producer has `type Item; func produce() -> Item` — compiles and prints `t1=4` (the core opaque-type use case).
**Actual:** `cannot infer type parameter __opaque_0` at the call site plus duplicated `no associated type Item` whose span points INTO the protocol declaration. Holding the opaque value without touching Item works; non-assoc methods on `some P` work.

No testdata under types/opaque/ exercises associated types, so this may be a staged limitation (E464/E466/E467 deferred) rather than a regression — but valid-looking code is rejected with an unactionable diagnostic anchored at the protocol itself; the unsupported case deserves a clean targeted error or an implementation. **Suspected subsystem: type-infer (opaque-type associated-type projection).**

## BUG-32 — `some P` as a struct field type panics mir-lower
**Severity:** medium | **Class:** ICE (on invalid/unsupported code) | **Status:** new | **Merged:** 0
**Issue:** [#168](https://github.com/kestrellang/kestrel/issues/168)
**Repro:** `$REPO/temp/bughunt/verify/r2_x_opaque_static_dispatch_4/min_r2_x_opaque_static_dispatch_4.ks`

```
cd $REPO/temp/bughunt/x_opaque_static_dispatch
KESTREL_STD=$REPO/temp/stable-kestrel/std $REPO/temp/stable-kestrel/kestrel build p15c_diag_some_field.ks -o p15c
```

**Expected:** a diagnostic rejecting `some Shape` in struct-field position (the E464/E466/E467 family is deferred, but invalid input must get a diagnostic, not a panic).
**Actual:** panic exit 101: `lib/kestrel-mir-lower/src/ty.rs:252:21: ICE: opaque type origin Entity(4782) has no concrete type`.

`struct Holder { var s: some Shape; }` + memberwise use. Locals (`let x: some Shape = ...`) are accepted and run correctly — only the field position panics. **Suspected subsystem: mir-lower (opaque origin resolution), with the real fix being a frontend rejection of `some` in field position.**

## BUG-33 — Out-of-range typed integer literals silently truncate/wrap
**Severity:** medium | **Class:** missing-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#169](https://github.com/kestrellang/kestrel/issues/169)
**Repro:** `$REPO/temp/bughunt/verify/r2_numerics_r2_5/r06_literal_range_silent.ks`

```
cd $REPO/temp/bughunt/numerics_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
K=$REPO/temp/stable-kestrel/kestrel
$K build r06_literal_range_silent.ks -o r06; echo "build exit=$?"
./r06; echo "run exit=$?"
```

**Expected:** a range diagnostic on each out-of-range literal: `Int8 = 200`, `Int64 = 9223372036854775808`, `Int8 = -129`, `UInt8 = 300` (precedent: out-of-range char escapes ARE diagnosed).
**Actual:** builds clean on both backends; prints `a=-56`, Int64 max+1 becomes minValue, etc. — silent two's-complement wrap of every out-of-range literal. Only `UInt8 = -1` is caught, incidentally via `UInt8 !: Negatable` (with a cascading bogus follow-up error).

`init(intLiteral value: lang.i64)` truncates via `lang.cast_i64_i8` with no range check, and no frontend check exists. A typo'd constant (port number into a UInt8 field) silently corrupts. The Negatable-based rejection of negative unsigned literals shows intent to reject impossible literals, making this look like an oversight rather than design. **Suspected subsystem: type-infer (literal range validation) + stdlib intLiteral inits.**

## BUG-34 — `Int64(parsing:)` cannot parse `Int64.minValue`
**Severity:** medium | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#170](https://github.com/kestrellang/kestrel/issues/170)
**Repro:** `$REPO/temp/bughunt/verify/r2_numerics_r2_1/r02_parse_int64_min.ks`

```
cd $REPO/temp/bughunt/numerics_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build r02_parse_int64_min.ks -o r02 && ./r02; echo "exit=$?"
```

**Expected:** `t1=some / t2=true` — "-9223372036854775808" fits exactly in Int64 per the documented contract.
**Actual:** `t1=none / t2=false` on both backends, -O0 and -O2.

`std/numeric/int64.ks` `init(parsing:)` accumulates the positive magnitude in Int64 and rejects when `result > maxValue - digit` BEFORE the negation step, so the magnitude 2^63 needed for minValue is unreachable. Int8/16/32 round-trip their minValue fine (they accumulate in Int64); UInt64 max parses fine. Breaks minValue round-tripping (the print direction is already broken by BUG-19). **Suspected subsystem: stdlib (int64.ks parsing — accumulate negative or special-case min).**

## BUG-35 — Zero-pad format spec places the minus sign after the pad zeros
**Severity:** low | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#171](https://github.com/kestrellang/kestrel/issues/171)
**Repro:** `$REPO/temp/bughunt/numerics_r2/r04_zeropad_sign.ks`

```
cd $REPO/temp/bughunt/numerics_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build r04_zeropad_sign.ks -o r04 && ./r04; echo "exit=$?"
```

**Expected:** `\(-5:08)` -> `-0000005`; `\(-255:05x)` -> `-000ff` (printf/Rust/Python convention: sign precedes the zero pad).
**Actual:** `000000-5`, `000-ff` — zeros are prepended before the sign. Positive values pad correctly; space-pad right-align is correct.

The 0-flag pads the fully rendered string (including its sign) on the left instead of padding between sign and digits. Combines with BUG-19 at minValue (`\(Int8.minValue:08x)` -> `00000-(0`). **Suspected subsystem: stdlib (format-spec zero-padding).**

## BUG-36 — Literal-default inconsistency: `let q: Float64 = 7 / 2` rejected
**Severity:** low | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#172](https://github.com/kestrellang/kestrel/issues/172)
**Repro:** `$REPO/temp/bughunt/numerics_r2/r07_float_ctx_literal.ks`

```
cd $REPO/temp/bughunt/numerics_r2
export KESTREL_STD=$REPO/temp/stable-kestrel/std
$REPO/temp/stable-kestrel/kestrel build r07_float_ctx_literal.ks -o r07; echo "exit=$?"
```

**Expected:** the Float64 annotation flows into the all-literal operand expression (q = 3.5), exactly as it already does for the bare literal (`let a: Float64 = 7;`) and through `+` for the mixed case (`let m: Float64 = 1 + 2.5;`), both verified compiling in the same build.
**Actual:** `E100 expected Float64 got Int64` on `let q: Float64 = 7 / 2;` only.

When BOTH operands of an arithmetic operator are int literals, they default to Int64 before the annotation's expected type is consulted; if either operand anchors to Float64 the int literal adapts. The asymmetry points at defaulting order in apply_literal_defaults rather than intended semantics (lower confidence the intended design requires `7 / 2` to work, hence low severity — but the inconsistency is real and reproducible). **Suspected subsystem: type-infer (apply_literal_defaults graduated relaxation ordering).**

## BUG-37 — E600: any `it`-closure poisons every zero-param closure in the same function body
**Severity:** medium | **Class:** false-diagnostic | **Status:** new (related to #136's `it`-tracking, distinct symptom) | **Merged:** 3 (independently found by 4 areas)
**Issue:** [#173](https://github.com/kestrellang/kestrel/issues/173)
**Repro:** `temp/bughunt/closures/bug_e600_or_plus_map_it.ks` (minimized: `temp/bughunt/verify/opus/backend_diff_work/r6_e600_min.ks`)

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_e600_or_plus_map_it.ks -o t
```

**Expected:** compiles — `{ a * a }` contains no `it`; the `{ it * 2 }` closure is a separate literal. Each closure compiles in isolation, and both compile when split across two functions.
**Actual:** `error: implicit 'it' parameter used in closure expecting 0 parameters [E600]` on the zero-param closure. Order-independent. Also fires on the invisible thunk of `a or b` / `a and b` desugar whenever a `.map { it * 2 }` exists in the same body, making the two most idiomatic constructs in the language mutually exclusive per function.

The "uses `it`" fact is tracked per function body, not per closure literal. Any function mixing a 0-param closure with an `it`-closure is uncompilable. **Suspected subsystem: type-infer / hir-lower (closure `it`-usage tracking granularity).**

## BUG-38 — Escape checker bypassed: capturing closures escape via local binding or struct field; dangling environment at runtime
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#174](https://github.com/kestrellang/kestrel/issues/174)
**Repro:** `temp/bughunt/closures/bug_escape_via_local_binding.ks`

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_escape_via_local_binding.ks -o t && ./t
```

**Expected:** the "cannot return a closure that captures variables" diagnostic — which correctly fires when the capturing closure literal is returned directly.
**Actual:** binding the closure to a local first (`let f = { ...captures... }; return f;`) or storing it in a struct field compiles with no diagnostic and prints nondeterministic stack garbage that changes per run (e.g. `t1=43935336960`). Same on cranelift, llvm, and -O2.

The escape check is syntactic on the return expression only; any indirection (local binding, struct field) defeats it, producing a closure whose environment points into a dead frame. **Suspected subsystem: kestrel-analyze (closure escape check must be data-flow-aware, or closure envs need heap promotion).**

## BUG-39 — Immediately calling a closure returned by a method (`s.mk()()`) produces garbage
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#175](https://github.com/kestrellang/kestrel/issues/175)
**Repro:** `temp/bughunt/closures/bug_method_result_immediate_call.ks`

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_method_result_immediate_call.ks -o t && ./t
```

**Expected:** `t2=7` — the returned closure `{ () in 7 }` captures nothing; `let g = s.mk(); g()` correctly prints 7, and the free-function analogue `id({ () in 7 })()` works.
**Actual:** nondeterministic garbage (`t2=6165487096`, varies per run) on cranelift, llvm, and -O2 — frontend/MIR lowering bug, not backend.

Only the method-call-then-immediate-call shape is broken; binding the result first works. **Suspected subsystem: mir-lower (FuncThick result of a method call invoked in place).**

## BUG-40 — Immediately calling a closure returned by a subscript: Array ICEs; user-defined subscript ships a SIGILL binary
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#176](https://github.com/kestrellang/kestrel/issues/176)
**Repro:** `temp/bughunt/closures/bug_subscript_result_call_ice.ks`

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_subscript_result_call_ice.ks -o t
```

**Expected:** both compile and run: `fs(0)()` -> `t1=42` (Array of closures), user-subscript case `t3=7`. Binding the subscript result first works in both.
**Actual:** Array case: `internal compiler error: post-mono verify failed ... Callee::Witness not resolved`. User-subscript case: function silently skipped at codegen ("N of M functions failed to compile"), build exits 0, binary SIGILLs (BUG-13 policy rides along).

Sibling of BUG-39 (immediate invocation of a call-result closure), but through the subscript path the failure is an ICE / trap-stub instead of garbage. **Suspected subsystem: mir-lower / mono-expand (callee resolution for closures produced by subscript calls).**

## BUG-41 — Move checker ignores closure captures of non-Copyable values
**Severity:** high | **Class:** missing-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#177](https://github.com/kestrellang/kestrel/issues/177)
**Repro:** `temp/bughunt/closures/bug_noncopyable_capture_dup.ks`

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_noncopyable_capture_dup.ks -o t && ./t
```

**Expected:** a diagnostic — the closure returns its captured non-Copyable by value, so calling it twice duplicates the value (needs a FnOnce-style restriction or a move error); using the source after capture should be E500.
**Actual:** calling the closure twice compiles silently and prints `deinit 3` TWICE — one logical non-Copyable value deinitialized twice (double-free class for heap payloads). Using the source after the capture ICEs in OSSA verify instead of producing a move diagnostic.

The move checker does not model closure captures as consumption at all. Sibling of BUG-25/BUG-26 (move-checker gaps whose downstream symptom is an ICE), but with an additional unsound-accept mode. **Suspected subsystem: kestrel-analyze (move checker closure-capture consumption tracking).**

## BUG-42 — Closure `mutating` param convention not inferred from a `let` variable annotation
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#178](https://github.com/kestrellang/kestrel/issues/178)
**Repro:** `temp/bughunt/closures/bug_convention_let_annotation.ks`

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_convention_let_annotation.ks -o t
```

**Expected:** `let f: (mutating Counter) -> () = { (x) in x.n = x.n + 10; };` compiles — the annotation supplies the expected type, exactly as argument position does (where convention inference works, per the #106 mutating-closure-params feature).
**Actual:** `error: cannot assign to immutable field 'n' [E201]`. Writing `{ (mutating x) in ... }` explicitly compiles and runs.

Convention inference from expected type only runs at call-argument position, not through variable-annotation expected types. **Suspected subsystem: type-infer (closure convention inference sources).**

## BUG-43 — Dictionary subscript-assignment does not coerce the RHS to the setter's Optional `newValue`
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#179](https://github.com/kestrellang/kestrel/issues/179)
**Repro:** `temp/bughunt/closures/bug_dict_subscript_assign_coercion.ks`

```
cd temp/bughunt/closures
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_dict_subscript_assign_coercion.ks -o t
```

**Expected:** `d("a") = 3` compiles and prints `t1=3` — the stdlib Dictionary subscript (`subscript(key: K) -> V?`) doc-comment explicitly documents this form; assigning `T` where `Optional[T]` is expected coerces everywhere else.
**Actual:** `error: type mismatch: expected Int64 got Optional[Int64] [E100]` (note the inverted direction in the message). Workarounds compile and run: `d("a") = .Some(3)` and `d.insert("a", 3)`.

Subscript-setter assignment unifies `newValue` with the RHS using Equal instead of Coerce, so the documented optional-subscript idiom is unusable. **Suspected subsystem: type-infer (subscript-assign RHS coercion).**

## BUG-44 — `for-in` over `Array[Cloneable]` deep-clones every element twice per iteration and deinits it twice in-loop
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#180](https://github.com/kestrellang/kestrel/issues/180)
**Repro:** `temp/bughunt/drop_deinit/r04_forin_double_clone.ks`

```
cd temp/bughunt/drop_deinit
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r04_forin_double_clone.ks -o t && ./t
```

**Expected:** `t1=[K1<i1>K2<i2>]<i1><i2>` — at most one yield-copy (clone) per element per iteration.
**Actual:** `t1=[K1K1<i1><i1>K2K2<i2><i2>]<i1><i2>` — two clones and two deinits per element per iteration, with an EMPTY loop body.

Behavior is correct (count-wise: each clone is dropped) but pathological: 2N clones of every element for any read-only iteration over a Cloneable element type — a major performance landmine for `Array[String]` loops, and a semantic surprise for deinit side effects. The iterator yield path plus the loop-binding copy each insert a clone. **Suspected subsystem: mir-lower / mono-expand (for-in desugar yield copy + binding copy both cloning).**

## BUG-45 — Struct field drop order is declaration order, contradicting the documented reverse order (and the init-failure path)
**Severity:** medium | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#181](https://github.com/kestrellang/kestrel/issues/181)
**Repro:** `temp/bughunt/drop_deinit/p20_drop_intrinsic_field_order.ks`

```
cd temp/bughunt/drop_deinit
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build p20_drop_intrinsic_field_order.ks -o t && ./t
```

**Expected:** per `docs/memory-model/drop-semantics.md` "Struct Field Drop Order": second-declared field drops first (`t1=[D]<9><8>`), consistently on every drop path.
**Actual:** whole-struct drop runs declaration order (`t1=[D]<8><9>`), while the `init?`-failure partial-drop path runs REVERSE order — the two drop paths disagree, and the normal one contradicts the docs.

Either the docs or the drop elaboration is wrong; the inconsistency between the normal and partial-drop paths is a bug regardless of which order is intended. **Suspected subsystem: mir-lower (drop elaboration field order; align with docs/memory-model/drop-semantics.md).**

## BUG-46 — Overlapping generic conformances: only the first-declared extension is consulted (declaration-order-dependent witness selection)
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#182](https://github.com/kestrellang/kestrel/issues/182)
**Repro:** `temp/bughunt/generics_protocols/r02_order_a.ks` (and `r02_order_b.ks` with the declarations swapped)

```
cd temp/bughunt/generics_protocols
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r02_order_a.ks -o a && ./a
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r02_order_b.ks -o b && ./b
```

**Expected:** most-specific-wins (the documented overlapping-conformance semantics): `Box[Int64]` picks the `where T: Show` impl in BOTH declaration orders.
**Actual:** the first-declared extension wins and later ones are never consulted: one order silently chooses the less-specific witness; the other order falsely rejects a conformance that holds.

The most-specific-wins selector (recently implemented for specialized-vs-generic overlap) is not applied here — candidate collection stops at the first matching extension. **Suspected subsystem: type-infer / conformance selection (candidate enumeration across all matching extensions).**

## BUG-47 — `some P` returned from a generic struct's method: TypeParam leaks past monomorphization (ICE)
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#183](https://github.com/kestrellang/kestrel/issues/183)
**Repro:** `temp/bughunt/generics_protocols/r28_min.ks`

```
cd temp/bughunt/generics_protocols
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r28_min.ks -o t
```

**Expected:** builds and prints `t1=5` — the equivalent opaque return from a generic FREE FUNCTION (`func wrap[T](x: T) -> some Counter where T: Counter { x }`) compiles and runs.
**Actual:** `internal compiler error: post-mono verify failed ... TypeParam(Entity(4782)) in value 8` (twice).

The opaque-origin resolution handles free-function type params but not the enclosing struct's params when the method's `some P` underlier mentions them. **Suspected subsystem: mono-expand (opaque origin substitution with container type params).**

## BUG-48 — Associated-type-projection bound `T.Item: P`: witness call on the projected value ICEs post-mono
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 1 (independent witness_dispatch find: `rn03a_assoc_bound_min.ks`)
**Issue:** [#184](https://github.com/kestrellang/kestrel/issues/184)
**Repro:** `temp/bughunt/generics_protocols/r26_c.ks` (also `temp/bughunt/witness_dispatch/rn03a_assoc_bound_min.ks`)

```
cd temp/bughunt/generics_protocols
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r26_c.ks -o t
```

**Expected:** builds and prints `t1=i` — `func render[T](x: T) -> String where T: Producer, T.Item: Show { x.produce().show() }` with a concrete conformer is core protocol machinery.
**Actual:** `internal compiler error: post-mono verify failed ... TypeParam(...) in value 2` + `Callee::Witness not resolved` — the witness call on the projected `T.Item` value never gets the concrete substitution.

Related family: #132 (blanket-witness TypeParam leak). The projection-bound witness table is keyed by the unsubstituted projection. **Suspected subsystem: mono-expand (witness resolution through associated-type projections).**

## BUG-49 — Extension `where` clause with an assoc-projection bound: build exits 1 with ZERO output
**Severity:** high | **Class:** missing-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#185](https://github.com/kestrellang/kestrel/issues/185)
**Repro:** `temp/bughunt/generics_protocols/r26_a.ks`

```
cd temp/bughunt/generics_protocols
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r26_a.ks -o t 2>/tmp/err; echo exit=$?; wc -c /tmp/err
```

**Expected:** either a successful build or SOME diagnostic. A compiler must never exit nonzero with no output.
**Actual:** exit 1 having written 0 bytes to stdout and stderr (verified with separate redirection and RUST_BACKTRACE=full). Deterministic on both backends.

Same silent-failure presentation as BUG-15 but a different trigger (extension `where T.Item: P` instead of move-checker tail position) — the error-counted-but-never-rendered class has at least two sources. **Suspected subsystem: analyzer-diagnostics / type-infer (diagnostic with unrenderable span swallowed; where-clause projection bound validation).**

## BUG-50 — Integer range-from pattern `N..` never matches; SIGILL when it is the final arm
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#186](https://github.com/kestrellang/kestrel/issues/186)
**Repro:** `temp/bughunt/patterns/bug_rangefrom.ks`

```
cd temp/bughunt/patterns
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_rangefrom.ks -o t && ./t; echo exit=$?
```

**Expected:** `t1=ge t2=ge exit=0` — `match x { 10.. => "ge", _ => "other" }` with x=42 takes the range arm.
**Actual:** `t1=other` then exit 132 (SIGILL; llvm exits 133) — `10..` never matches, falls to the wildcard; when `N..` is the FINAL arm the match falls off the end into a trap. `..<10` prefix patterns work.

Range-from patterns lower to an always-false test. **Suspected subsystem: mir-lower (range-from pattern test emission).**

## BUG-51 — Or-pattern with bindings (`.A(x) or .B(x) => x`) ICEs in OSSA verify
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#187](https://github.com/kestrellang/kestrel/issues/187)
**Repro:** `temp/bughunt/patterns/bug_orpattern_binding.ks`

```
cd temp/bughunt/patterns
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_orpattern_binding.ks -o t
```

**Expected:** builds and prints `t1=3 t1b=7` — binding or-patterns are pinned valid by testdata (`expressions/match/or_patterns` uses `.Add(left, right) or .Sub(left, right) => ...`, but as a diagnostics-only test that never reaches MIR).
**Actual:** `internal compiler error: OSSA verify failed in 'Test.bind' at bb1[2]: operand ValueId(5) used but never defined`.

The per-alternative binding values are not merged into a block param at the arm join, so the arm body reads a value defined on only one path. Same one-armed-binding DNA as the fixed #126/#121 cluster, in the or-pattern joiner. **Suspected subsystem: mir-lower (or-pattern binding join in pattern.rs).**

## BUG-52 — Array patterns are broken end-to-end at runtime
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 3 (four facets, one feature)
**Issue:** [#188](https://github.com/kestrellang/kestrel/issues/188)
**Repro:** `temp/bughunt/patterns/a1_empty.ks` (+ `a2_pair.ks`, `a3_first_rest.ks`, `a4_rest_count.ks`)

```
cd temp/bughunt/patterns
for f in a1_empty a2_pair a3_first_rest a4_rest_count; do KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build $f.ks -o $f && ./$f; echo "$f exit=$?"; done
```

**Expected:** `a1: e=empty n=nonempty` | `a2: x=5 m=0` | `a3: f=10` | `a4: r=3`.
**Actual:** a1: `[]` MATCHES a 2-element array (wrong arm taken — miscompile); a2: `[x, y]` SIGILLs with no output; a3: `[first, ..]` ICEs in OSSA verify; a4: `..rest` binds a garbage count.

Every array-pattern form mis-executes at runtime; existing testdata for them is diagnostics-only so MIR/codegen was never exercised (matches the "MIR witness-call TODO" note from the array-rest-pattern port). Either finish the lowering or reject array patterns until they work — silently taking the wrong arm is the worst of the four modes. **Suspected subsystem: mir-lower (array pattern lowering; witness calls for count/element access).**

## BUG-53 — False E305: exhaustive payload-split matches reported non-exhaustive
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 1 (enums_recursive generic-enum/Bool payload-field variant)
**Issue:** [#189](https://github.com/kestrellang/kestrel/issues/189)
**Repro:** `temp/bughunt/patterns/bug_e305_payload_split.ks` (also `temp/bughunt/enums_recursive/p26_e305_false_positive.ks`)

```
cd temp/bughunt/patterns
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_e305_payload_split.ks -o t
```

**Expected:** compiles — `.Has(.Some(b))` + `.Has(.None)` + `.Nothing` jointly cover `enum W3 { Has(Optional[Int64]), Nothing }`; same for splits on Bool-typed payload fields.
**Actual:** `error: non-exhaustive match: missing _ [E305]` on provably exhaustive matches whenever a payload FIELD whose type is a generic-enum instantiation (Optional) or Bool is split across arms.

The exhaustiveness analyzer doesn't expand the case-space of generic-enum-instantiation / Bool payload fields. Drives users straight into BUG-54 (adding the suggested `_` arm panics the compiler). **Suspected subsystem: kestrel-analyze (match pattern analyzer payload-field case expansion, E310-E315 family).**

## BUG-54 — ICE `enum case Entity(4294967295) has no Name` (pattern.rs:1344) when a wildcard arm follows a payload split
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 1 (enums_recursive generic-payload variant `p13_ice_wildcard.ks`)
**Issue:** [#190](https://github.com/kestrellang/kestrel/issues/190)
**Repro:** `temp/bughunt/patterns/c2_wildcard_after_split.ks` (also `temp/bughunt/verify/opus/enums_recursive_work/p13_ice_wildcard.ks`)

```
cd temp/bughunt/patterns
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build c2_wildcard_after_split.ks -o t
```

**Expected:** builds — the wildcard arm is legal (merely redundant), and it is EXACTLY what BUG-53's E305 help text tells the user to add.
**Actual:** `thread 'main' panicked at lib/kestrel-mir-lower/src/body/pattern.rs:1344:36: ICE: enum case Entity(4294967295) has no Name` — a raw Rust panic with no rendered diagnostic.

The decision-tree builder materializes a sentinel case id (u32::MAX) for the wildcard row and later looks up its name. BUG-53 + BUG-54 together form a trap: the false diagnostic's suggested fix panics the compiler. **Suspected subsystem: mir-lower (pattern decision tree wildcard sentinel).**

## BUG-55 — `Int64??` fails to parse: `??` is lexed as one token in type position
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#191](https://github.com/kestrellang/kestrel/issues/191)
**Repro:** `temp/bughunt/patterns/q_optopt.ks`

```
cd temp/bughunt/patterns
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build q_optopt.ks -o t
```

**Expected:** `let p: Int64?? = .Some(.Some(2));` parses as `Optional[Optional[Int64]]` (the `T?` sugar applied twice); the spelled-out form works fully.
**Actual:** `error: expected 'try', '-', or 27 others, found '??'` — the nil-coalescing `??` token is not split into two postfix `?` in type position.

Standard lexer/parser fix: split `??` in type contexts (same treatment other languages give `>>` in generics). **Suspected subsystem: parser (type-position token splitting).**

## BUG-56 — Throws value-promotion fails for computed/operator-expression returns (known: #135 family)
**Severity:** medium | **Class:** false-diagnostic | **Status:** known — tracked as #135 (and the in-repo characterization test `types/optional/value_promotion_computed_expr_known_bug.ks`, bug "3-i") | **Merged:** 2 (found independently by patterns, control_flow_main, optionals_errors)
**Repro:** `temp/bughunt/patterns/bug_throws_promotion.ks` (also `temp/bughunt/optionals_errors/r13_promotion_computed_expr.ks`)

```
cd temp/bughunt/patterns
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build bug_throws_promotion.ks -o t
```

**Expected:** `return x + 1;` in `-> Int64 throws Err` promotes to `.Ok(x + 1)` like bare values do.
**Actual:** `error: type mismatch: expected Result[Int64, Err] got Int64 [E100]` — the expected Result type is pushed INTO the `+` expression instead of promoting its result. Interpolated-string returns additionally produce a bogus `no associated type 'Interpolation'` cascade (`Result[String, E].Interpolation`).

Three finder areas hit this independently; the bug-hunt evidence adds the diagnostic-cascade detail to #135's existing description. No new issue filed.

## BUG-57 — E494 falsely rejects every `-> &mutating` return rooted at a `mutating` param or `self`
**Severity:** high | **Class:** false-diagnostic | **Status:** new | **Merged:** 1 (refs_core + refs_cross independent finds)
**Issue:** [#192](https://github.com/kestrellang/kestrel/issues/192)
**Repro:** `temp/bughunt/refs_core/r01_mut_param_ref_bogus_e494.ks` (also `temp/bughunt/refs_cross/r03_mutref_decl_only.ks`)

```
cd temp/bughunt/refs_core
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r01_mut_param_ref_bogus_e494.ks -o t
```

**Expected:** builds — `mutating func mutPeek() -> &mutating Int64 { self.v }` is the documented stage-1 feature: `self`/a `mutating` param is parameter-rooted, exactly the legal root for `-> &mutating` returns (the mutable-root rule).
**Actual:** `error[E494]: cannot return this reference: it borrows local, which does not outlive the call` (pointing at `self.v`, naming no local), followed by an `unsupported` cascade. Every user-written `&mutating` return shape hits it.

The stdlib's own `Array.mutableAt` works, so the escape checker's root classification treats user `mutating` receivers/params differently from the blessed stdlib path — the user-facing half of the feature is unusable. **Suspected subsystem: mir-lower escape checker (root classification of mutating params/self in ret_borrow functions).**

## BUG-58 — Ref-returning protocol requirement through witness dispatch returns the ADDRESS as the value
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 1 (refs_core + refs_cross independent finds)
**Issue:** [#193](https://github.com/kestrellang/kestrel/issues/193)
**Repro:** `temp/bughunt/refs_core/r03_witness_ref_garbage.ks` (also `temp/bughunt/refs_cross/r14_witness_operand.ks`)

```
cd temp/bughunt/refs_core
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r03_witness_ref_garbage.ks -o t && ./t
KESTREL_STD=$PWD/../../stable-kestrel/std KESTREL_BACKEND=llvm ../../stable-kestrel/kestrel build r03_witness_ref_garbage.ks -o t2 && ./t2
```

**Expected:** `l1=21 l2=21` — `Box(v:21).peek()` through a `T: Peekable` generic bound reads the pointee exactly like a direct call.
**Actual:** cranelift prints ASLR-varying addresses (different each run); llvm is PARTIALLY correct (one of two call shapes right) — the two backends diverge. String pointees produce stale/garbage strings.

The witness thunk for a ret_borrow requirement doesn't apply the ref-return ABI (or applies it on one side of the seam only), so the caller treats the returned address as the value. Silent wrong answers in the core protocol+refs combination. **Suspected subsystem: mono-expand / codegen (ret_borrow ABI through witness thunks — the ABI is implemented per backend, and they disagree).**

## BUG-59 — Ref result does not decay to a copy in assignment-RHS and return value contexts
**Severity:** high | **Class:** false-diagnostic | **Status:** new | **Merged:** 1 (refs_core assignment-RHS + refs_cross return-position finds)
**Issue:** [#194](https://github.com/kestrellang/kestrel/issues/194)
**Repro:** `temp/bughunt/refs_core/r02_ref_no_decay_value_ctx.ks` (also `temp/bughunt/refs_cross/r10_return_decay.ks`)

```
cd temp/bughunt/refs_core
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r02_ref_no_decay_value_ctx.ks -o t
```

**Expected:** `x = b.peek();` and `func f() -> Int64 { return arr.at(index: 0); }` decay-copy the pointee — `let x = b.peek();` already works (binding decay, pinned by `ret_borrow/binding_decay_copies.ks`), and `docs/plans/references/stage1/semantics.md` explicitly lists `return` of `T` as a copy-out context.
**Actual:** `error: type mismatch: expected Int64 got &Int64 [E100]` for plain assignment RHS, tail-expression returns, and `return` statements (plain and throws).

The solve_coerce ref-decay arm covers bindings and call arguments but not assignment/return expected-type seams. **Suspected subsystem: type-infer (ref decay coverage of value contexts).**

## BUG-60 — Closure whose tail is a bare ref-returning call fails OSSA verify (ICE)
**Severity:** medium | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#195](https://github.com/kestrellang/kestrel/issues/195)
**Repro:** `temp/bughunt/refs_cross/r09_closure_bare_ref_tail.ks`

```
cd temp/bughunt/refs_cross
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r09_closure_bare_ref_tail.ks -o t
```

**Expected:** compiles (a closure return is a value context; the `&Int64` from `arr.at` should decay), prints `t1=20` — or a proper diagnostic.
**Actual:** `internal compiler error: OSSA verify failed in 'Test.main.closure.458' at bb0: function returns @guaranteed value ... without the ret_borrow convention`.

Closure bodies miss both the decay path (BUG-59) and the E481-style "refs don't cross closure boundaries" rejection, so the raw ref reaches MIR verification. **Suspected subsystem: type-infer / hir-lower (ref decay or rejection at closure return seams).**

## BUG-61 — `opt ?? refReturningCall()` rejected with E491 + inverted type mismatch
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#196](https://github.com/kestrellang/kestrel/issues/196)
**Repro:** `temp/bughunt/refs_cross/r31_coalesce_ref_rhs.ks`

```
cd temp/bughunt/refs_cross
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r31_coalesce_ref_rhs.ks -o t
```

**Expected:** `t1=11` — the `??` RHS thunk's result is a value use; the `&Int64` from `h.peek()` should decay to `Int64` exactly as in plain argument position (which works).
**Actual:** `error[E491]: a reference-returning function cannot be used as a value` + `error: type mismatch: expected &Int64 got Int64` (inverted).

The `??` desugar's RHS thunk has a `() -> T` expected type; the decay should apply inside the thunk body but the ref-misuse check fires first. Same family as the known `or`-RHS thunk-capture wart, new diagnostic shape. **Suspected subsystem: type-infer (ref decay inside coalescing-thunk bodies).**

## BUG-62 — Nested string interpolation splices the inner literal's raw source text instead of evaluating it
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 1 (strings + backend_diff independent finds)
**Issue:** [#197](https://github.com/kestrellang/kestrel/issues/197)
**Repro:** `temp/bughunt/strings/r01_nested_interp.ks` (also `temp/bughunt/verify/opus/backend_diff_work/r2_nested_interp.ks`)

```
cd temp/bughunt/strings
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r01_nested_interp.ks -o t && ./t
```

**Expected:** `t1=aXb` — `"a\("X")b"` and `"\("v=\(inner * 7)")"` evaluate the inner literal; or a diagnostic if nesting is unsupported.
**Actual:** when the hole is EXACTLY a nested-interpolating literal, the output contains the inner literal's raw source text including quotes and backslash (`t1="a\(inner)b"`). Compiles clean on both backends.

The interpolation lexer does not recursively lex a nested literal that itself contains `\(...)`; the un-lexed segment is emitted as literal text. Mixed holes (`"x \(s) y"` where s is a plain variable) work. **Suspected subsystem: lexer/parser (recursive interpolation segment lexing).**

## BUG-63 — Tuple-element assignment `t.0 = v` compiles but the store is silently dropped
**Severity:** critical | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#198](https://github.com/kestrellang/kestrel/issues/198)
**Repro:** `temp/bughunt/collections_cow/r01_tuple_field_assign.ks` (minimized: `temp/bughunt/verify/opus/collections_cow_work/min_0.ks`)

```
cd temp/bughunt/collections_cow
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r01_tuple_field_assign.ks -o t && ./t
```

**Expected:** `var t = (1, 2); t.0 = 9;` reads back 9 — `tuple_index_mutability.ks` testdata pins `t.0 = 10` as valid with no ERROR annotation.
**Actual:** reads back 1 — every `*.N = v` store is a no-op in every context tried (locals, fields, nested). Whole-tuple reassignment works. Identical on cranelift/llvm/-O2, zero diagnostics.

Tuple-index projections are not lowered as places on the assignment path (the rvalue temp absorbs the store) — the same "tuple projections aren't places" DNA as BUG-05 and BUG-28, on the simplest possible surface. **Suspected subsystem: hir-lower/mir-lower (tuple-element place lowering in assignment).**

## BUG-64 — Stores through chained-subscript places silently dropped (tracked as #129)
**Severity:** critical | **Class:** miscompile | **Status:** known — tracked as #129 | **Merged:** 1 (collections_cow + backend_diff finds)
**Repro:** `temp/bughunt/collections_cow/r02_nested_subscript_assign.ks` (minimized: `temp/bughunt/verify/opus/collections_cow_work/min_1.ks`; also `temp/bughunt/backend_diff/r3b_subscript_field_assign.ks`)

```
cd temp/bughunt/collections_cow
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r02_nested_subscript_assign.ks -o t && ./t
```

**Expected:** `grid(1)(0) = 30` writes through; `arr(0).n = 5` writes through (or both are rejected like compound `+=` is with E202).
**Actual:** the write lands in a temporary copy of the inner collection and is discarded — zero diagnostics, exit 0. Terminal-subscript writes (`flat(1) = 99`) work.

Matches existing issue #129 (chained subscript assignment silently no-ops); the hunt adds two repro shapes (`dd(unwrap: k)(0) = v` labeled subscripts, and subscript-then-field `arr(0).n = 5`, which is also BUG-01's accessor-intermediate class). Verified identical on llvm and -O2. No new issue filed — evidence attached to the doc for #129.

## BUG-65 — `return` inside a closure types the closure `-> Never` and lowers its body to a trap
**Severity:** high | **Class:** runtime-crash | **Status:** new | **Merged:** 0
**Issue:** [#199](https://github.com/kestrellang/kestrel/issues/199)
**Repro:** `temp/bughunt/collections_cow/r09_closure_return_runtime.ks` (minimized: `temp/bughunt/verify/opus/collections_cow_work/min_2.ks`)

```
cd temp/bughunt/collections_cow
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r09_closure_return_runtime.ks -o t && ./t; echo exit=$?
```

**Expected:** either a diagnostic rejecting `return` in a closure, or closure-local return semantics (`result=10`).
**Actual:** compiles with zero warnings, prints nothing, and traps on the first call of the closure: exit 132 (SIGILL, cranelift) / 133 (SIGTRAP, llvm), same at -O2.

`return` inside the closure body is type-checked against the closure as if diverging, the closure infers `-> Never`, and the body is lowered to `unreachable`. A one-token user mistake (or Swift habit) becomes a clean-building crash. **Suspected subsystem: type-infer / hir-lower (`return` binding inside closure bodies — diagnose or give it closure-return semantics).**

## BUG-66 — Error-typed expression inside string interpolation reaches post-mono verify (ICE; real diagnostic swallowed)
**Severity:** medium | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#200](https://github.com/kestrellang/kestrel/issues/200)
**Repro:** `temp/bughunt/collections_cow/r10_nested_tuple_projection.ks`

```
cd temp/bughunt/collections_cow
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r10_nested_tuple_projection.ks -o t
```

**Expected:** the normal syntax/type diagnostic for `n.0.0` — outside interpolation it gets `expected identifier after '.'`.
**Actual:** `internal compiler error: post-mono verify failed ... Callee::Direct not resolved (callee='appendInterpolation', type_args=[Error])` and the real diagnostic is swallowed.

Inside `\(...)`, a parse-failed expression becomes an `Error` type that flows into the interpolation's `appendInterpolation` call and on into monomorphization, instead of poisoning the statement. **Suspected subsystem: hir-lower / type-infer (interpolation error recovery; Error type args must not reach mono).**

## BUG-67 — Labeled `continue` crossing an inner loop: OSSA verify ICE
**Severity:** high | **Class:** ICE | **Status:** new | **Merged:** 0
**Issue:** [#201](https://github.com/kestrellang/kestrel/issues/201)
**Repro:** `temp/bughunt/control_flow_main/ice_labeled_continue_min.ks`

```
cd temp/bughunt/control_flow_main
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build ice_labeled_continue_min.ks -o t
```

**Expected:** compiles — `mid: while c < 2 { c = c + 1; while true { continue mid; } }` is valid (the `continue outer` pattern appears in testdata, but only in a diagnostics-kind test that never reaches MIR).
**Actual:** `internal compiler error: OSSA verify failed in 'Test.main' at bb10: value ValueId(136) consumed more than once`, both backends.

The labeled back-edge from inside the inner loop re-enters the outer loop header without re-establishing the outer loop's merge-slot bookkeeping. Same loop merge-slot family as BUG-26. **Suspected subsystem: mir-lower (labeled continue scope unwinding / loop merge slots).**

## BUG-68 — `guard else` block ending in a never-typed call rejected with E003
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#202](https://github.com/kestrellang/kestrel/issues/202)
**Repro:** `temp/bughunt/control_flow_main/r16_guard_fatalerror.ks`

```
cd temp/bughunt/control_flow_main
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r16_guard_fatalerror.ks -o t
```

**Expected:** compiles — `guard x > 0 else { fatalError("negative"); }`: the else block ends in a call of type `!`, which diverges; never-typed calls ARE accepted as diverging if-arms (verified).
**Actual:** `error: guard else block must diverge (return, break, continue, or throw) [E003]`; same for a user-defined `func boom() -> !`.

The guard divergence check is a syntactic statement-kind whitelist and ignores `!`-typed terminal expressions, inconsistent with the type system's own divergence handling elsewhere. **Suspected subsystem: kestrel-analyze (guard divergence check should consult the type of the trailing expression).**

## BUG-69 — `try` cannot appear in bare `if`/`while` condition position
**Severity:** low | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#203](https://github.com/kestrellang/kestrel/issues/203)
**Repro:** `temp/bughunt/control_flow_main/r12_if_try_cond.ks`

```
cd temp/bughunt/control_flow_main
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r12_if_try_cond.ks -o t
```

**Expected:** `if try check() { ... }` parses — `try expr` is an ordinary expression, and the parenthesized `if (try check())` compiles and runs.
**Actual:** `error: expected 'let', '-', or 9 others, found 'try'` with a parse-error cascade; same for `while try ...`.

`try` is missing from the restricted no-struct-literal condition grammar that other prefix forms (`not`, unary minus) are in. **Suspected subsystem: parser (condition-position expression grammar).**

## BUG-70 — `try` error-propagation double-deinits a non-Copyable Err payload (use-after-free)
**Severity:** critical | **Class:** runtime-crash | **Status:** new | **Merged:** 0
**Issue:** [#204](https://github.com/kestrellang/kestrel/issues/204)
**Repro:** `temp/bughunt/optionals_errors/r02_try_err_string_payload.ks` (Int-payload counter variant: `r01`)

```
cd temp/bughunt/optionals_errors
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r02_try_err_string_payload.ks -o t && ./t; echo exit=$?
```

**Expected:** exactly one `edeinit` — the Err payload propagated by `try` deinits once, when the caller's Result drops.
**Actual:** `edeinit` runs TWICE — once on the try-propagation path inside the callee's caller, once at the caller's scope exit. With a heap String payload the double-free produces nondeterministic SIGILL/SIGTRAP or garbage output, on both backends at -O0 and -O2.

Controls isolate it precisely: holding `.Err` directly is clean, explicit `throw` is clean, Copyable payloads are clean — only the non-Copyable-Err `try` propagation path duplicates ownership of the payload while re-wrapping the Result. **Suspected subsystem: mir-lower (try/throw rethrow lowering — Err payload ownership when re-wrapping).**

## BUG-71 — `fatalError` discards its message argument; panic path prints nothing at all
**Severity:** low | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#205](https://github.com/kestrellang/kestrel/issues/205)
**Repro:** `temp/bughunt/optionals_errors/t15_panic_message.ks`

```
cd temp/bughunt/optionals_errors
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build t15_panic_message.ks -o t && ./t; echo exit=$?
```

**Expected:** `fatalError("custom-message-123")` writes something containing the message to stderr before aborting.
**Actual:** nothing is written to stdout or stderr on either backend; the process dies bare SIGILL/SIGTRAP. Two layers: (1) `std/core/panic.ks:24` literally discards the `message` parameter and hardcodes `lang.panic_unwind("fatal error")` — violating its own doc comment; (2) the panic runtime path emits no output at all (the silent-trap part matches the pinned force-unwrap behavior, so the stdlib message-drop is the clear defect).

One-line stdlib fix for layer 1; layer 2 (whether traps should print) is a design question worth deciding deliberately. **Suspected subsystem: stdlib (panic.ks) + runtime panic plumbing.**

## BUG-72 — Shift by >= bit width: LLVM -O2 produces poison garbage (even for constant `1 << 64`)
**Severity:** high | **Class:** backend-divergence | **Status:** new | **Merged:** 0
**Issue:** [#206](https://github.com/kestrellang/kestrel/issues/206)
**Repro:** `temp/bughunt/backend_diff/p15_shift_width.ks`

```
cd temp/bughunt/backend_diff
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build p15_shift_width.ks -o a && ./a
KESTREL_STD=$PWD/../../stable-kestrel/std KESTREL_BACKEND=llvm ../../stable-kestrel/kestrel build p15_shift_width.ks -O2 -o b && ./b
```

**Expected:** all configs agree; cranelift (-O0/-O2) and llvm -O0 mask the shift amount mod width (`1 << 64 == 1`).
**Actual:** llvm -O2 prints run-to-run-varying pointer-like garbage, and the CONSTANT `1 << 64` flips from 1 to 0 — unguarded LLVM `shl`/`lshr` poison, exploited by the optimizer.

An earlier verifier refuted a variant of this claim; re-verification reproduced run-varying outputs, settling it. Decide the language semantics (mask, like cranelift implements) and guard the LLVM lowering to match. **Suspected subsystem: codegen-llvm (shift intrinsic lowering needs amount masking).**

## BUG-73 — `f64 -> i64` cast of NaN/inf/out-of-range: LLVM -O2 produces nondeterministic garbage
**Severity:** high | **Class:** backend-divergence | **Status:** new | **Merged:** 0
**Issue:** [#207](https://github.com/kestrellang/kestrel/issues/207)
**Repro:** `temp/bughunt/backend_diff/r5_cast_f64_i64_divergence.ks`

```
cd temp/bughunt/backend_diff
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r5_cast_f64_i64_divergence.ks -o a && ./a
KESTREL_STD=$PWD/../../stable-kestrel/std KESTREL_BACKEND=llvm ../../stable-kestrel/kestrel build r5_cast_f64_i64_divergence.ks -O2 -o b && ./b && ./b
```

**Expected:** all configs agree with the cranelift/llvm -O0 saturating behavior (NaN -> 0, +inf -> i64.max, 1e19 -> i64.max).
**Actual:** llvm -O2 prints pointer-looking values that CHANGE between runs of the same binary — raw `fptosi` poison folded under UB.

Use LLVM's `fptosi.sat` intrinsic (or explicit range guards) to match the saturating semantics the other three configs already implement. **Suspected subsystem: codegen-llvm (float->int cast lowering).**

## BUG-74 — Deep recursive drop of a 30k-node recursive enum chain SIGSEGVs (synthesized drop glue recursion)
**Severity:** medium | **Class:** runtime-crash | **Status:** new | **Merged:** 0
**Issue:** [#208](https://github.com/kestrellang/kestrel/issues/208)
**Repro:** `temp/bughunt/enums_recursive/p09e_30k.ks`

```
cd temp/bughunt/enums_recursive
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build p09e_30k.ks -o t && ./t; echo exit=$?
```

**Expected:** `t1=built` then `t2=dropped` — or at minimum a graceful stack-overflow message.
**Actual:** `t1=built` then SIGSEGV (exit 139) during the scope-exit drop of a 30,000-node linked list; 20k nodes drop cleanly. Deterministic.

Unlike user-written recursion (where stack exhaustion is the user's problem), the recursion here is compiler-synthesized drop glue: building the list iteratively is fine, but dropping it recurses per node. Linked structures of this size are ordinary; drop glue for self-recursive boxed enums should be iterative. **Suspected subsystem: mir-lower / mono-expand (drop-shim generation for recursive boxed enums).**

## BUG-75 — Every type-inference diagnostic is emitted twice (uncoded copy + [E100] copy)
**Severity:** medium | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#209](https://github.com/kestrellang/kestrel/issues/209)
**Repro:** `temp/bughunt/overloads_inference/p20_span_check.ks` (minimized: `temp/bughunt/verify/opus/overloads_inference_work/min_f1_dup_diag.ks`)

```
cd temp/bughunt/overloads_inference
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build p20_span_check.ks -o t
```

**Expected:** one error block per mistake.
**Actual:** each inference diagnostic renders once WITHOUT a code and again WITH `[E100]` at the identical span — a single bad call produces four error blocks (2 errors x 2 copies).

Two reporters drain the solver's diagnostics (an uncoded path and the coded path). Halves the signal-to-noise of every inference error a user ever sees. **Suspected subsystem: kestrel-reporting / type-infer diagnostic sink (deduplicate the two emission paths).**

## BUG-76 — Overload-ambiguity diagnostic renders the receiver of module-level functions as `Error`
**Severity:** low | **Class:** other | **Status:** new | **Merged:** 0
**Issue:** [#210](https://github.com/kestrellang/kestrel/issues/210)
**Repro:** `temp/bughunt/overloads_inference/p22_ambiguity_outside_interp.ks`

```
cd temp/bughunt/overloads_inference
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build p22_ambiguity_outside_interp.ks -o t
```

**Expected:** a message naming the overload set plainly, e.g. `ambiguous call to 'f': 2 candidates`.
**Actual:** `error: ambiguous member 'f': Error.f ambiguous [E100]` — free functions have no receiver, and the internal `Error` type leaks into the message (a member call with the same overload set correctly renders `S.f ambiguous`).

This is the same `Error.xxx ambiguous` artifact that pollutes LSP diagnostics. **Suspected subsystem: type-infer (ambiguity message formatting for receiver-less calls).**

## BUG-77 — Int+float literal pair unifies in generic calls but is rejected in array literals and if-branches
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#211](https://github.com/kestrellang/kestrel/issues/211)
**Repro:** `temp/bughunt/overloads_inference/r4_literal_mixed_reject.ks`

```
cd temp/bughunt/overloads_inference
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r4_literal_mixed_reject.ks -o t
```

**Expected:** `{integer-literal, float-literal}` treated uniformly across contexts — `pick(1, 2.5)` unifies T=Float64 and `let c: [Float64] = [1, 2.5]` compiles, so `let xs = [1, 2.5];` and `if cond { 1 } else { 2.5 }` should too.
**Actual:** the array-literal and if-branch forms are rejected; the if-branch message is also SWAPPED — it points at the float `2.5` but says "got integer literal".

Element/branch unification uses Equal between the two literal kinds and skips the int->float relaxation that call-argument unification applies. Sibling of BUG-36 (annotation-flow asymmetry) in the literal-defaults family. **Suspected subsystem: type-infer (apply_literal_defaults / join unification for literal kinds).**

## BUG-78 — `[1,2,3] == [1,2,3]` fails to compile: mono loses the Slice protocol-extension witness's type args
**Severity:** high | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#212](https://github.com/kestrellang/kestrel/issues/212)
**Repro:** `temp/bughunt/witness_dispatch/r19c_array_eq_concrete.ks`

```
cd temp/bughunt/witness_dispatch
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r19c_array_eq_concrete.ks -o t
```

**Expected:** builds and prints `t1=true t2=false` — stdlib declares `extend Array[T]: Equatable where T: Equatable` with `isEqual` provided by `extend Slice[T] where T: Equatable` (array.ks:1434, slice.ks:1098); `xs.isEqual(to: ys)` called directly works.
**Actual:** `error: unsupported: monomorphization failed with 1 error(s): type arg arity mismatch for std.collections.Slice.isEqual: expected 1, got 0`.

The conformance witness points at a constrained protocol-extension method on a DIFFERENT generic container (Slice) than the conforming type (Array); mono drops the extension's type args when instantiating the witness. Array equality — about as basic as stdlib surface gets — is unusable via `==`. **Suspected subsystem: mono-expand (witness instantiation for cross-container protocol-extension methods).**

## BUG-79 — E454 false: constrained generic-protocol-extension method not recognized as a conformance witness
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#213](https://github.com/kestrellang/kestrel/issues/213)
**Repro:** `temp/bughunt/witness_dispatch/n02_container_ext_witness.ks`

```
cd temp/bughunt/witness_dispatch
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build n02_container_ext_witness.ks -o t
```

**Expected:** compiles — `BoxC: Container[Int64]`; `extend Container[T] where T: Equatable` provides `isEqual(to: Self)`; `Int64: Equatable` holds; so `extend BoxC: Equatable` is satisfied. This is the exact user-level shape of the stdlib Array/Slice pattern.
**Actual:** `error: type 'BoxC' does not implement method 'isEqual' from protocol 'Equatable' [E454]`.

The conformance checker doesn't look through constrained protocol-extension members when the constraint is satisfied by the concrete conformer — the frontend half of BUG-78 (which is the mono half of the same pattern). Users cannot replicate the stdlib's own conformance idiom. **Suspected subsystem: kestrel-analyze / type-infer (witness lookup through bounded protocol extensions).**

## BUG-80 — Member field chained off a static computed var silently drops the projection
**Severity:** high | **Class:** miscompile | **Status:** new | **Merged:** 0
**Issue:** [#214](https://github.com/kestrellang/kestrel/issues/214)
**Repro:** `temp/bughunt/witness_dispatch/r17b_static_var_chain_runtime.ks` (minimized: `temp/bughunt/verify/opus/witness_dispatch_work/min_17.ks`)

```
cd temp/bughunt/witness_dispatch
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r17b_static_var_chain_runtime.ks -o t && ./t
```

**Expected:** `let c = Money.seven.cents` gives `c: Int64 = 77`; passing `c` to a `(m: Money)` parameter is a type error.
**Actual:** compiles and prints `t1=money(77)` — `c` is typed AND valued as the whole `Money(cents: 77)`; the `.cents` field projection is silently ignored. Static func chains (`Money.make().cents`) and methods off static vars work; only field-access-off-static-computed-var drops the projection.

A type-level wrong answer that survives to runtime: the member resolution for `Type.staticVar.field` resolves the chain to the static var and discards the field link. **Suspected subsystem: type-infer (member resolution of field access on static computed var results).**

## BUG-81 — `extend (): P` gaps: static `-> Self` requirement rejected (E458); associated-type bindings not registered
**Severity:** medium | **Class:** false-diagnostic | **Status:** new | **Merged:** 0
**Issue:** [#215](https://github.com/kestrellang/kestrel/issues/215)
**Repro:** `temp/bughunt/witness_dispatch/rn05a_unit_static_self.ks` (and `rn05b_unit_assoc.ks`)

```
cd temp/bughunt/witness_dispatch
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build rn05a_unit_static_self.ks -o a
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build rn05b_unit_assoc.ks -o b
```

**Expected:** both compile — in `extend (): Ident`, `static func make() -> ()` implements `static func make() -> Self` (Self = ()); in `extend (): Marked`, `type Mark = ...` registers the binding. Nominal-struct analogs of both constructs build and run; unit conformance itself is a landed, tested feature.
**Actual:** rn05a: `error: method 'make' has wrong return type for protocol 'Ident' [E458]` (`()` not recognized as Self); rn05b: `error: no associated type 'Mark': ().Mark no assoc type [E100]`.

The synthetic `lang.()` entity (from the structural-extend feature) misses two of the Entity-keyed integration points the feature's design notes call out: Self-equality in requirement signature matching, and the assoc-type binding registry. **Suspected subsystem: type-infer / conformance (structural-entity Self matching + assoc bindings).**

## BUG-82 — Float64 subnormals print Int64.max-mantissa garbage; digit generation truncates instead of rounds; parse underflows representable values to 0
**Severity:** high | **Class:** miscompile | **Status:** new (same digit-generation engine as BUG-24 — one rewrite should fix both) | **Merged:** 0
**Issue:** [#216](https://github.com/kestrellang/kestrel/issues/216)
**Repro:** `temp/bughunt/strings/r05_float_format.ks`

```
cd temp/bughunt/strings
KESTREL_STD=$PWD/../../stable-kestrel/std ../../stable-kestrel/kestrel build r05_float_format.ks -o t && ./t
```

**Expected:** `5e-324` (min positive subnormal — the literal itself is stored correctly, `d > 0.0` is true) prints a faithful representation; `9.2e-305` round-trips; `Float64(parsing: "9223372036854775807e-323")` returns the representable ~9.22e-305.
**Actual:** the subnormal prints `9223372036854775807e-323` — the mantissa digits are literally Int64.max from a saturated float->int cast in the digit extractor (~19 orders of magnitude off); `9.2e-305` prints `9.199999e-305` (truncated, not rounded — re-parses to a DIFFERENT double); parse returns 0 for the long-mantissa form. All on both backends and -O2.

Three more faces of the BUG-24 digit-generation engine: saturation on subnormal scaling, truncation instead of round-to-nearest, and the inverse (parse) path underflowing. Fixing BUG-24 with integer-based digit extraction (ryu/dragon) should be validated against these cases too. **Suspected subsystem: stdlib (float64.ks format/parse digit pipeline).**
