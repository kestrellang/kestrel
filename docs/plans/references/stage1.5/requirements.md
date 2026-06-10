# Stage 1.5 — Requirements

Ergonomics follow-ons, scheduled **on demand** after stage 1 ships. Items
are independently shippable; do not bundle.

1. **Call-as-place → place accessors — IMPLEMENTED 2026-06-10**:
   `ref` / `mutating ref` accessor kinds on subscripts and computed
   properties (`syntax.md`), per-operation provider routing with
   `get`/`set` writeback as the permanent fallback (`semantics.md`),
   decl rules E619–E622. The subscript-resolution question was settled
   by DECISION (2026-06-10): the **labeled place form** — Array's
   in-place subscript is `subscript(at index: Int64)` (inherent, ref
   pair), the unlabeled/`checked:`/… forms stay on `extend Slice[T]`
   get/set, and distinct labels route through the existing label-based
   fallback with ZERO resolution changes (the type-aware fallback-tier
   option from `compiler-arch.md` stays unbuilt). `arr(i) += v` works
   via writeback (COW copy path); `arr(at: i) += v` is the in-place
   fast path. `Array.at`/`mutableAt` removed; references tests migrated
   to `arr(at: i)`.
2. **Named ref bindings**: `let r = &expr;` with the visible-`&` cue
   (`references-syntax.md` §2 Option C), block-local. Fine semantics
   proposed in `syntax.md` (store-through, no rebind, copy-on-rebind);
   not ratified. Unlocks **`&` pattern bindings** (decided spelling,
   `syntax.md`) and with them in-place enum-payload access.
3. **Dangle lint**: same-function `Pointer(to: local)` returned as a ref
   (`references-gaps.md` §10.3).
4. **Shared-read projection sugar — DISSOLVED (ratified 2026-06-10),
   interim SHIPPED**: no new projection construct. The interim
   `Dictionary.modify(key) { (mutating v) in … } -> R?` landed
   (bucket read → mutate → write-back under the hood; one probe, COW
   only on hit; upgrades silently to in-place access later — no API
   change). The conditional-lookup shape revisits as
   `if let r = &dict.find(key)` once item 2's bindings meet
   `Optional[&T]` — items 2 and 4 converge there.
5. **Arm-value decay — IMPLEMENTED 2026-06-10** (arms; literal elements
   remain the follow-up): `match c { 1 => b.peek(), _ => 0 }` decays to
   owned. Mechanism CORRECTION: stage 1 never shipped a
   `Constraint::Decay` — scrutinee/binding/assign-target decay is
   expr-id SETS (`ctx.rs`) consulted in `bind_call_result`, and arm
   values are simply the 4th set (`arm_value_exprs`, marked for
   `UserMatch`/`IfLet` arms and if-else branch tails; `GuardLet` is
   deliberately unmarked — its pattern arm is the CPS continuation).
   Merge `Equal`s stay fully bidirectional. MIR: `capture_arm_exit`
   already copied @guaranteed arm results; it now also ends the ref's
   borrow (no false E497), with arm-value spans threaded for the E503
   copy-guard. Arms ALWAYS decay to owned — all-refs included; a
   match/if in return position of a `-> &T` fn errs as E494 (was E497),
   pinned. **Literal elements LANDED in the follow-up commit
   (2026-06-10)**: array/tuple/dict elements mark the same set (renamed
   `always_decay_exprs`) and the literal lowerings run the binding-decay
   copy (`decay_if_ref`). Two consequences: E492's INFERENCE surface
   dissolved (`[h.peek()]` now legally infers `Array[Int64]`; borrow-
   convention generic args see through refs per the §10.5 amendment, so
   no expressible program leaks a ref into an inferred type argument —
   that diagnostics pin was removed; the E492 validation stays as a
   backstop), and a PRE-EXISTING per-literal clone of Cloneable elements
   at Array-literal construction surfaced (storage-init cost, unrelated
   to refs — the decay pin measures the DELTA over a plain element).
   #127 untouched.

## Dictionary deferral (DECIDED 2026-06-09)

**Dict's stdlib adoption of refs waits for `Optional[&T]`.** The useful
Dict ref API is conditional (`find(key) -> Optional[&V]`); the only
shape expressible under stage-1 rules is a panicking unconditional
accessor, which isn't worth an API change. Revisit when `Optional[&T]`
becomes expressible (stage-2 territory, or a dedicated narrow carve in
the Rust-`Option<&T>` style — niche-able: a ref is never null, so the
optional is free at runtime). Interim: closure-based
`dict.modify(key) { ... }` is implementable today with writeback under
the hood and upgrades silently to in-place access later — no API change.

**Enum-payload projection is wanted regardless (100%, decided) — via
`&` pattern bindings, NOT an intrinsic.** The MIR + codegen mechanism
already exists: `InstKind::EnumPayload` on a `@guaranteed` operand
returns the payload field's in-place ADDRESS in both backends; pattern
bindings then copy out of the projected address, which is the only
missing piece. The match proves the tag before the projection — safe by
construction, unlike an `enum_payload_ptr` intrinsic (unchecked-tag
contract, no near-term customer). Anchors in `compiler-arch.md`.

The **parallel-array restructure** (Swiss-table shape) is demoted to a
pure performance project — never required for refs. Rehash invalidation
was never a blocker: stage-1 refs are expression-scoped, so no
insert/remove/rehash can intervene while one is live.

Entry: stage 1 shipped + concrete demand. ~4-6 wk total.
