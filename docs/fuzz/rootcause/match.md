# Root causes: enums_match bugs (plus C1 codegen repros)

Base: /home/user/kestrel, commit 3fea2041eef543c99ac62c296e1fbba8b30af401, prebuilt ./target/release/kestrel. Analysis only; no fixes made.
P-M = lib/kestrel-pattern-matching/src, M-L = lib/kestrel-mir-lower/src/body

## Group A: integer patterns use a fixed-width or i64-only domain (C1, M2)

### C1: integer literal and range patterns truncated to 32 bits; unsigned ranges compared as signed
- **Root cause:** `emit_discriminant` always gives the switch value type `i32`, whatever the scrutinee is. Codegen's `discriminant_width` falls back to I32 for any type that is not an enum, and `ireduce`s an Int64 scrutinee to 32 bits. The case immediates are then compared at i32 (`icmp_imm`), so 5000000000 matches 705032704, and the bound 2^32 becomes 0. `i64max_range_trap` is the same bug: exhaustiveness proves the arms are total over i64, but at runtime the truncated compare misses every arm and hits the trap. A second defect: `IntRange` always uses signed predicates (SGE/SLE), so a UInt64 value ≥ 2^63 is compared as negative (`u64_high_range`, `u64_range_pattern` c3).
- **Location:**
  - M-L/mod.rs:2729 (`emit_discriminant`: `ty_arena.i32()`)
  - lib/kestrel-codegen-cranelift/src/inst.rs:233-262 (Discriminant) and :2115 (`discriminant_width`, default I32)
  - lib/kestrel-codegen-cranelift/src/terminator.rs:206-240 (width taken from the i32 result type) and :299 (IntRange signed CC)
  - LLVM twins: lib/kestrel-codegen-llvm/src/inst.rs:237 and :2127, terminator.rs:352 (SGE)
- **Why it's missed:** the Discriminant instruction was written for enum tags and reused for integer switches. Nothing carries the integer width or signedness of the scrutinee.
- **Confidence:** high (raw MIR shows `discriminant %v0 // @owned Int32` for an Int64 scrutinee).
- **Fix shape:** for an integer switch, type the discriminant at the scrutinee's own int width (or skip Discriminant and switch on the scalar), and choose signed or unsigned predicates from the type's signedness.

### M2: a full unsigned range (`0..=255` on UInt8) is not exhaustive
- **Root cause:** `classify_intrinsic` maps every `i8..i64` to `TypeShape::Infinite`, with no width or signedness. `covers_whole_domain` tiles against the hard-coded `[i64::MIN, i64::MAX]`, and `split::int_interval` defaults open bounds to the i64 range. A per-type domain such as `[0, 255]` is never consulted.
- **Location:**
  - P-M/constructor.rs:552 (`classify_intrinsic`)
  - P-M/usefulness.rs:443-458 (`covers_whole_domain`)
  - P-M/split.rs:90-100 (`int_interval`)
  - P-M/flat_pat.rs:416 (`IntLiteral(*n as i64)`; also wraps u64 literals above i64::MAX)
- **Confidence:** high.
- **Fix shape:** add a `TypeShape::IntRange{min,max}` from the scrutinee's width and signedness, and tile or split against it. Literals above i64::MAX need i128.

## Group B: positional binding paths that ignore the real structure (C3, C4, H11, M19)

### C3: tuple rest `(a, .., b)` binds the wrong element
- **Root cause:** `collect_bindings` builds suffix paths as `Index(prefix.len() + j)`. The comment there says these are placeholder indices, and nothing ever fixes them. On a 3-tuple, `b` gets index 1 instead of 2. `flatten` gets the arity right (it uses `actual_arity - suffix.len()`), but the binding paths do not.
- **Location:** P-M/decision_tree.rs:403-418 (`collect_bindings`, `HirPat::Tuple`).
- **Confidence:** high.
- **Fix shape:** pass the tuple arity (from the scrutinee type or the matched Tuple ctor) and use `Index(arity - suffix.len() + j)`.

### C4: labels in enum payload patterns are ignored (fields bind by position)
- **Root cause:** `HirPatArg.label` is never read in pattern lowering.
  - Type-check: `gen_variant_pat` zips args with payload types, and `gen_implicit_variant_pat` / ImplicitPat does the same.
  - Flattening: `flatten`, `Variant` / `ImplicitVariant`.
  - Binding paths: `collect_bindings` pushes `Index(i)` by source position.
- **Location:**
  - lib/kestrel-type-infer/src/generate.rs:3430-3440 and 3449-3474
  - P-M/flat_pat.rs:267-312
  - P-M/decision_tree.rs:419-433
- **Confidence:** high.
- **Fix shape:** at HIR lowering (one place), normalize labeled payload args into declaration order, filling omitted fields with wildcards and diagnosing unknown or duplicate labels.

### M19: a wrong or duplicate payload label is accepted (`.A(z: n)` for `A(x:)`)
- **Root cause:** same as C4. No stage checks pattern-arg labels against the case's payload field names. The parser treats a bare identifier as a label, and nobody validates it.
- **Location:** generate.rs:3386 (`gen_variant_pat`), generate.rs:3449 and solver/member.rs ImplicitPat (around line 1307).
- **Confidence:** high.
- **Fix shape:** the same normalization step as C4 emits an error for an unknown or duplicate label.

### H11: struct patterns in match arms ICE with "TupleExtract on non-tuple"
- **Root cause:** `build_specialized_paths` gives `Constructor::Struct` children `PathElement::Index(i)`. In `apply_access_path`, `Index` on a non-enum, non-array value lowers to `emit_tuple_extract`. Bindings in `collect_bindings` use `Field(name)`, but switch tests on struct sub-fields (here `x: true`) go through the Index path. A second risk: the index order comes from `collect_fields` and may not match the layout.
- **Location:** P-M/decision_tree.rs:336-338; M-L/pattern.rs:1464-1486 (`apply_access_path`).
- **Confidence:** high.
- **Fix shape:** emit `PathElement::Field(field_name)` for Struct ctor children, taking the names from `collect_fields`.

## Group C: usefulness and decision-tree algorithm defects (M1, H25, M3)

### M1: an exhaustive Bool or enum tuple match is reported as missing `_`
- **Root cause:** `is_wildcard_useful` drops straight to the default matrix whenever any row has a wildcard in the column (`has_catch_all`). It never checks whether the head constructors already form a complete signature. With `(true,false) | (_,true) | (false,false)`, it discards the true/false rows and asks only about `(_,true)`, so it misses (·,false).
- **Location:** P-M/usefulness.rs:236-248.
- **Why it's missed:** standard Maranget uses the default matrix only when the signature is incomplete; this shortcut is a non-standard optimization.
- **Confidence:** high.
- **Fix shape:** delete the shortcut, or apply it only when `all_for_type` is None or the heads are incomplete.

### H25: Bool-tuple match compile time doubles per element
- **Root cause:** `compile_matrix` has no "first row is all wildcards → leaf" cutoff. It only stops at width 0. Each specialization keeps the irrefutable row 0 and still splits every remaining column, so the tree has 2^n−1 switches. Measured: n=11 gives 2047 `switch` instructions in raw MIR.
- **Location:** P-M/decision_tree.rs:145-212 (`compile_matrix`). Base cases are at 156-163.
- **Confidence:** high.
- **Fix shape:** if `rows[0]` is unguarded and all of its pats are wildcard-like, return `compile_leaf` immediately.

### M3: an empty match on an empty enum gives E304 "inhabited"
- **Root cause:** the empty-arms check treats only `ResolvedTy::Never` as uninhabited. A zero-case enum (`all_for_type` == `Some([])`) is never considered.
- **Location:** lib/kestrel-analyze/src/body/exhaustiveness.rs:192-208; P-M/usefulness.rs:85-97 (`check_match`).
- **Confidence:** high.
- **Fix shape:** treat the scrutinee as uninhabited when it is Never or `Constructor::all_for_type` returns an empty set.

## Group D: parser gaps (M4, L14)

### M4: `.Some(P { x, y })` is a parse error
- **Root cause:** `enum_pattern` treats any leading Identifier as a label or shorthand binding (`bump Identifier; eat ':'`). It never checks `nth_at(1, LBrace)` for a struct pattern.
- **Location:** lib/kestrel-parser/src/grammar/patterns.rs:165-190 (`enum_pattern`).
- **Confidence:** high.
- **Fix shape:** if `Identifier` is followed by `{`, parse it with `pat(p)` instead of as a label.

### L14: negative literal patterns `-1 =>` are unsupported
- **Root cause:** `base()` has no `Minus` arm. Integer and Float literal patterns and range bounds accept only unsigned tokens, and `at_pattern_start` omits Minus.
- **Location:** lib/kestrel-parser/src/grammar/patterns.rs:36-56, 74-150 (`base`), and range bounds at 86-106. HIR literal lowering must then negate.
- **Confidence:** high.
- **Fix shape:** accept `'-' (Integer|Float)` as a literal or range bound in the parser and fold the sign in HIR lowering.

## Group E: OSSA ownership in match lowering (H6, H7, H8, H9, H10)

### H6: string literal patterns on an owned scrutinee ICE
- **Root cause:** in the string-literal chain in non-place mode, `test_val` is computed once before the chain as `apply_access_path(scrutinee, path).0`.
  - With an empty path, it is the owned scrutinee itself. The first test's `branch` forwards it as a block arg, which consumes it. The miss block then re-tests the stale pre-branch ValueId, so the verifier reports "use of consumed / consumed more than once" (raw MIR: bb2 calls `matches` on `%v0` after `bb0` passed `%v0` to `bb2(%v18)`).
  - With a non-empty path (tuple/array element), the owned projection is never destroyed or threaded, so the verifier reports "never consumed".
- **Location:** M-L/pattern.rs:504-590 (string chain; `test_val` at 508). The bool path at 428 has the same shape but only for Copyable Bool.
- **Confidence:** high for the temp-scrutinee case; medium-high for tuple and array.
- **Fix shape:** compute the test through a fresh `begin_borrow` of the current (rebound) scrutinee for each test, ended before the branch, the way the general switch does at 596-635.

### H7: matching a not-Copyable value twice gives an OSSA ICE instead of E500
- **Root cause:** move tracking analyzes a `match` scrutinee as a plain read and never records a move. MIR, however, consumes an owned scrutinee when an arm binds a not-Copyable payload by value (`.Some(r)`). The second match then reuses the consumed value.
- **Location:** lib/kestrel-analyze/src/body/move_tracking.rs:528-563 (`HirExpr::Match`).
- **Confidence:** high.
- **Fix shape:** when any arm moves a not-Copyable sub-value out of an owned scrutinee place, `record_operand_move` the scrutinee (`effective_move_place`), keeping the lockstep with MIR `emit_moveout`.

### H8: a `.Some(var r)` binding of a not-Copyable payload ICEs
- **Root cause:** pattern bindings are always registered as `LocalBinding::Ssa`. `Binding.is_mutable` is ignored. `let` and `var` statements use a `Var` stack slot (stmt.rs:55-68, #107). A field assign on the SSA binding lowers to `move_value %r` plus a mut-borrow, which consumes it, and the next read of `r` uses the consumed value.
- **Location:** M-L/pattern.rs:1209-1213 (`emit_moveout`), 1051 (`bind_path_copy`), 1004/1017 (place mode) and 778. Compare M-L/stmt.rs:55-68.
- **Confidence:** high.
- **Fix shape:** for `is_mutable` bindings (or all bindings, matching #107), store the bound value into an `emit_uninit` slot and register `LocalBinding::Var` with `track_var`.

### H9 and H10: a not-Copyable match where every arm returns, inside `for` (optionally under `if`)
- **Root cause (shared):** when every arm diverges, `lower_match` still switches to a fresh merge block that has no predecessors and is not terminated. The enclosing `capture_arm_exit` (if arm, or the for-desugar's outer match arm) therefore sees a reaching exit in which the scrutinee `o` has been consumed. The merge mask drops `o`, and the live else edge `destroy_value`s it (raw MIR: bb6 `destroy_value %v99`, then bb9 is a Never-param block). At the loop latch, `o` is no longer tracked, so the back-edge jump passes 10 args to a header that expects 11.
  - H9 surfaces as the Cranelift error "got 10, expected 11".
  - H10 surfaces as OSSA "never consumed", because args are shifted onto the wrong params.
- **Location:**
  - M-L/pattern.rs:232-260 (`lower_match` merge when `exits` is empty; comment at 246)
  - M-L/mod.rs:2006 (`capture_arm_exit` checks only `is_terminated`)
  - M-L/control.rs:111-150 (`lower_if` merge), and the back edge in `lower_loop` at control.rs:245
- **Confidence:** medium-high for H10 (MIR inspected). Medium for H9, which is the same arity mismatch without the `if`; its MIR was not dumped.
- **Fix shape:** when `exits` is empty, terminate the merge with `unreachable`, or mark the builder diverged so the outer `capture_arm_exit` returns None. Optionally, make loop back-edges reconcile against the header params.
