# kestrel-analyze — Agent Guide

## Analyzer File Structure

Every analyzer file MUST follow this structure:

### 1. Doc Comment (required)

The file starts with a doc comment describing the analyzer, followed by a section for each diagnostic it produces. Each diagnostic section documents the ID, name, severity, category, message, labels (with span sources), and notes.

```rust
//! # <Analyzer Name>
//!
//! <Brief description of what this analyzer checks.>
//!
//! ## Diagnostics
//!
//! ### <ID> — `<name>` (<Severity>, <Category>)
//!
//! **Message:** "<message template, use {name} for interpolated values>"
//!
//! **Labels:**
//! - Primary: <what this label points to>
//!   - Span source: <which util function and what HIR node it's called on>
//!   - Message: "<label message>"
//! - Secondary: <what this label points to> (if any)
//!   - Span source: <which util function and what HIR node/entity>
//!   - Message: "<label message>"
//!
//! **Notes:**
//! - "<note text>" (or "(none)" if no notes)
```

#### Span Source Documentation

Each label's span source MUST specify:
- Which `util::` function extracts the span (`util::expr_span`, `util::stmt_span`, `util::pat_span`, `util::entity_name`)
- What HIR node or entity it's called on (e.g., "the unreachable `HirStmtId`", "the `HirExprId` of the assignment target")
- For declaration spans: whether it's the **usage site**, the **declaration name**, or the **declaration signature**

Examples of good span source documentation:
```
- Span source: `util::stmt_span` on the last `HirStmtId` in the function body
- Span source: `util::expr_span` on the assignment target `HirExprId`
- Span source: `util::pat_span` on the refutable `HirPatId` in the let binding
- Span source: entity span from `Callable` component on the protocol method declaration (name span)
```

### 2. Descriptor Statics

```rust
static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E<NNN>",
        name: "<snake_case_name>",
        default_severity: Severity::<Error|Warning|Info>,
        category: Category::<Correctness|Style|Performance|Usage>,
    },
    // Add more descriptors if the analyzer produces multiple diagnostic kinds
];
```

### 3. Analyzer Struct (ZST)

```rust
pub struct <Name>Analyzer;
```

### 4. Trait Implementations

```rust
impl Describe for <Name>Analyzer {
    fn id(&self) -> AnalyzerId { AnalyzerId::<Name> }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] { DESCRIPTORS }
}

impl BodyCheck for <Name>Analyzer {  // or DeclCheck or CompilationCheck
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Pure analysis logic
    }
}
```

### 5. Helper Functions — private by default, shared when a *fact* is shared

Logic that only one analyzer needs stays a private function in that analyzer's
file. That is still the default, and most helpers should be private.

But a **control-flow fact that two or more analyzers ask about is not a private
helper** — it is one fact with several consumers, and per "One analyzer per
fact" below, duplicating it guarantees drift. Such a fact goes in
`body/control_flow.rs` as a `pub(crate)` **pure predicate**: `&HirBody` in,
`bool` or plain data out. No `TypedBody`, no `BodyContext`, no `QueryContext`,
no diagnostics — if a helper needs any of those it is analyzer logic, not a
shared fact.

**Precedent:** `control_flow::block_contains_break_for`. Four analyzers
(`dead_code`, `exhaustive_return`, `definite_assignment`, `move_tracking`) each
carried a private `block_contains_break` triad, `guard.rs` carried a fifth
degenerate version, and all of them ignored `break`'s label while only one ever
grew a `Sugar` arm. They now share one walk keyed on the loop's label, over the
one label rule in `kestrel_hir::label_selects_loop`.

**Tier 2 — typed divergence, the one named exception.** "Does this diverge?"
cannot be answered from `&HirBody` alone: a call to a `-> !` function is an
ordinary `HirExpr::Call` and only its *type* says it never returns. So
`control_flow.rs` also holds `expr_diverges` / `stmt_diverges` /
`block_diverges` / `block_parts_diverge`, which take `&BodyContext<'_>`. They
share the file because the `Loop` case of "does this diverge" **is**
`block_contains_break_for`. This is an exception by name, not a new category:
anything else that wants `BodyContext` in here is analyzer logic and stays in
its analyzer. Six analyzers had copy-pasted the divergence rule with three
different `Loop` mechanisms and a seventh consulted no types at all; see
`docs/fragility/G12/`. **Structure is matched before the Never type** in those
functions — reversing it lets inference overrule the structural `Loop` verdict,
the same hazard G8 removed.

**What still stays local:** a fact that needs analyzer-specific state carried
*alongside* the walk. `initializer.rs`'s `loop_break_stack` collects an
`InitState` at each reachable break — reachability-aware, strictly stronger
than the syntactic predicate, and meaningless to any other analyzer. It keeps
its own stack and shares only the `label_selects_loop` predicate.
`control_flow.rs` is for shared *facts*, not a dumping ground for walks.

`util.rs` is unchanged and unrelated: **span extraction and entity info
helpers** only.

### 6. Registration

Add a variant to `AnalyzerId` in `traits.rs` (plus its `as_str` arm), then add
the analyzer to `default_analyzers()` in `lib.rs`. The `Analyze` query panics
on IDs that aren't registered as body or decl checks, so both steps are
required:
```rust
pub fn default_analyzers() -> AnalyzerRegistry {
    let mut r = AnalyzerRegistry::new();
    r.add_body_check(MyNewAnalyzer);  // ← add here
    r
}
```

## Shared Utilities (`util.rs`)

Use these utilities in all analyzers. **Do not create local span extraction or entity name helpers — use the shared ones.** If you need a new utility, add it to `util.rs` and update this table.

### Span Extraction

| Function | Input | Returns | Description |
|----------|-------|---------|-------------|
| `util::expr_span(hir, id)` | `&HirBody, HirExprId` | `Span` | Span of any `HirExpr` variant |
| `util::stmt_span(hir, id)` | `&HirBody, HirStmtId` | `Span` | Span of any `HirStmt` variant |
| `util::pat_span(hir, id)` | `&HirBody, HirPatId` | `Span` | Span of any `HirPat` variant |

### Entity Info

| Function | Input | Returns | Description |
|----------|-------|---------|-------------|
| `util::entity_name(ctx, entity)` | `&QueryContext, Entity` | `String` | Name from `Name` component, or `"<anonymous>"` |

### Child Walks

| Function | Input | Returns | Description |
|----------|-------|---------|-------------|
| `util::children_of_kind(ctx, parent, kind)` | `&QueryContext, Entity, NodeKind` | `Vec<Entity>` | Direct children with the given `NodeKind`, in declaration order |
| `util::children_named_of_kind(ctx, parent, name, kind)` | `&QueryContext, Entity, &str, NodeKind` | `Vec<Entity>` | Direct children matching both `NodeKind` and `Name` (multiple for overloads); nameless entities never match — `init`/`subscript` sentinel or `"<anonymous>"` matching needs `children_of_kind` + a custom filter |

## Diagnostic ID Allocation

IDs follow the pattern `E<NNN>`:
- **E001–E099**: Control flow (exhaustive return, dead code, guard-let divergence)
- **E100–E199**: Type checking (branch mismatch, condition not bool, argument type)
- **E200–E299**: Mutability and assignment (immutable assignment, captured variable)
- **E300–E399**: Patterns (exhaustiveness, refutable in let, irrefutable in match)
- **E400–E499**: Declarations (conformance, duplicates, cycles, visibility)
- **E500–E599**: Memory semantics (use-after-move, cloneable fields)
- **E600–E699**: Functions and closures (missing body, wrong arity, FFI safety)
- **E700–E799**: Literals and lexing (escape sequences, malformed literals)

Current allocations:
- E001: `missing_return` (exhaustive_return.rs)
- E002: `unreachable_code` (dead_code.rs)
- E121: `integer_literal_out_of_range` (body/integer_literal_range.rs) — post-inference range check on integer literals against their resolved fixed-width type; a unary-`negate` over a literal is checked as the negated value (so `Int8` accepts `-128` but rejects `-129`)
- E301: `refutable_for_loop_pattern` (for_loop_pattern.rs)
- E302: `irrefutable_if_let` (exhaustiveness.rs)
- E303: `irrefutable_match_arm` (exhaustiveness.rs) — reserved; currently not emitted (E306 subsumes)
- E304: `empty_match` (exhaustiveness.rs)
- E305: `non_exhaustive_match` (exhaustiveness.rs)
- E306: `unreachable_pattern` (exhaustiveness.rs)
- E307: `overlapping_range` (exhaustiveness.rs)
- E308: `irrefutable_while_let` (exhaustiveness.rs)
- E309: `irrefutable_guard_let` (exhaustiveness.rs)
- E310: `duplicate_match_binding` (match_pattern.rs)
- E311: `float_literal_in_pattern` (match_pattern.rs)
- E312: `unknown_enum_case` (match_pattern.rs)
- E313: `wrong_variant_arity` (match_pattern.rs)
- E314: `wrong_tuple_arity_in_pattern` (match_pattern.rs)
- E315: `or_pattern_inconsistent_bindings` (match_pattern.rs)
- E422: `disallowed_enum_conformance` (decl/conformance_rules.rs)
- E423: `conflicting_copyable_opt_out` (decl/conformance_rules.rs)
- E424: `negative_conformance_requires_language_feature` (decl/conformance_rules.rs)
- E425: `copyable_with_non_copyable_field` (decl/conformance_rules.rs)
- E430: `return_type_less_visible` (decl/visibility.rs)
- E431: `parameter_type_less_visible` (decl/visibility.rs)
- E432: `aliased_type_less_visible` (decl/visibility.rs)
- E433: `field_type_less_visible` (decl/visibility.rs)
- E447: `circular_type_alias` (compilation/type_alias_cycles.rs)
- E448: `type_alias_contains_infer` (compilation/type_alias_cycles.rs) — reserved; not emitted
- E449: `self_containing_struct` (compilation/struct_cycles.rs)
- E450: `circular_struct_containment` (compilation/struct_cycles.rs)
- E451: `circular_constraint` (compilation/constraint_cycles.rs)
- E459: `circular_protocol_inheritance` (compilation/protocol_cycles.rs)
  — was double-allocated with conformance_completeness's receiver-kind
  check; resolved 2026-07 by moving that check to E477.
- E454–E458, E460, E462–E465: conformance completeness + indirect-enum
  checks (compilation/conformance_completeness.rs, indirect_enum.rs) —
  this list is stale for that range.
- E466: `some_in_field_type` (decl/field.rs) — `some P` rejected in field
  position (#168).
  E458 (`wrong_method_return_type`) carries the stage-2d ref-shape rule:
  a witness's reference return must match the requirement EXACTLY in
  shape and mutability (`-> T` never witnesses `-> &T` and vice versa —
  the ABIs differ: raw pointer vs owned value; `&` never matches
  `&mutating`). The normalization is `kestrel-type-infer` compare.rs
  (`ResolvedTy::Ref`); a mismatch with a ref on either side gets the
  ABI-explainer note.
- E461: `unknown_attribute` (compilation/unknown_attribute.rs)
- E467: `method_shadows_field` (decl/field_method_collision.rs) — #130
- E468–E472: RESERVED by the opaque-types plan
  (docs/plans/opaque-types/implementation-spec.md); not yet emitted.
- E473–E478: allocated 2026-07 resolving the eight double-allocated
  E-codes (each pair's registered/externally-referenced owner kept the
  old code; the other side moved here). Uniqueness is now enforced by
  `registry.rs::descriptor_ids_are_unique_across_all_analyzers`.
  **E4xx is allocated up to E479.** E480 and above are already emitted
  elsewhere (`kestrel-hir-lower/src/ty.rs`, `kestrel-semantics/src/staticness.rs`,
  `body/access_mode.rs`). The only free E4xx codes are the gaps
  **E400–E410 and E414**. As of 2026-09-23 they are not emitted, documented or
  reserved anywhere. Take a new E4xx from those gaps.
  - E479: `ambiguous_where_clause_associated_type` (decl/generics.rs) — G29,
    `6ef0782c`. It is reported at the clause when the associated type in a
    where-clause equality (`Item.Out = X`) is declared by two or more of the
    protocols bound on `Item`. Its sibling, where no bound protocol declares
    the segment, reuses **E440**. E479 had been informally earmarked for F29
    (`@builtin` argument validation, never built), so F29 needs a code from
    the gaps above.
  - E473: `duplicate_deinit` (decl/duplicate_deinit.rs) — was E423
  - E474: `duplicate_symbol_same_kind` (decl/duplicate_symbol.rs) — was E424
  - E475: `duplicate_symbol_different_kind` (decl/duplicate_symbol.rs) — was E425
  - E476: `unresolved_type_in_annotation` (compilation/type_annotation_resolution.rs) — was E436
  - E477: `wrong_method_receiver_kind` (compilation/conformance_completeness.rs) — was E459
  - E478: `global_property_already_static` (decl/field.rs) — was E417
- E480–E489: reference-type rejections (stage 0.5 of references). NOT
  analyzer descriptors — emitted from HIR lowering via codespan
  `with_code` (kestrel-hir-lower `ty.rs::reject_ref_types` +
  `desugar.rs` for E488); the test matcher passes codespan codes
  through. E480 is PERMANENT (params never take ref types — conventions
  are the only spelling, references-gaps.md §10.6); E481 was carved out
  in stage 1; **stage 2b carved out E483/E484/E485 under
  `RefPolicy::AllowAggregate`** — those codes now fire only from STRICT
  entry points (alias RHS, protocol/extension-target args, where-clause
  types). Enum case payloads classify as Field (E483's position), NOT
  Param, despite living in the `Callable` component.
  - E480: ref type in parameter position (incl. function-type params, closure params)
  - E481: ref type in return position (legal since stage 1)
  - E482: ref type in a `var`/`let` annotation (aggregates wrapping refs are legal)
  - E483: ref type in a struct/enum field — LEGAL since 2b except Strict entries
  - E484: ref type in a tuple element — LEGAL since 2b except Strict entries
  - E485: ref type as a generic type argument — LEGAL since 2b except Strict entries
  - E486: ref type as a function-type return
  - E487: nested reference (`&&T`, `&mutating &T`)
  - E488: `&` in expression position (desugar.rs, `UnaryOp::Borrow`)
  - E489: ref type in any other position (alias RHS, where-clause, bound).
    **Stage 2d carved out two RHS shapes** via
    `reject_ref_types_allowing_top_ref`: TRIVIAL member aliases
    (`type Item = &T` in a struct/enum/extension — the assoc-binding
    shape) and where-clause EQUALITY RHS (`where I.Item = &Int64`).
    Protocol-parented assoc defaults, non-trivial aliases, and
    protocol-bound args stay Strict. Anti-smuggling rides the eager
    trivial-alias expansion: a named USE re-applies the use-site position
    rules, so `let x: Foo.Item` is E482 — but the diagnostic ANCHORS at
    the alias RHS span (the expansion reuses the alias's AST), one error
    per illegal use.
- E490–E498: stage-1 reference rules (returnable refs). Mixed homes — E490
  is a hir-lower codespan code; E491/E492 are solver `InferError`s; E493 is
  an analyzer descriptor; E494–E498 are coded MIR diagnostics
  (`VerifyError.diag` from the escape checker / `set_terminator` /
  `try_consume`, rendered by kestrel-compiler; surfaced to diagnostics
  tests by the harness's MIR-stage pass).
  - E490: ref inside effect sugar (`-> &T throws E` → `Result[&T, E]`) — hir-lower
  - E491: ref-returning function used as a value / captured / stored — type-infer
  - E492: ref leaked into a generic type argument via inference — type-infer
  - E493: `ambiguous_borrow_source` (decl/ref_return.rs) — free fn with ≥2
    non-consuming params returning a ref; methods root at the receiver
  - E494: returned ref roots at a local — escape error (mir verify::check_escapes).
    Since 2b also the owned-return CARRIER variant: a ref-BEARING aggregate
    return (`-> Optional[&T]`) whose taint roots at a local ("cannot return
    this value: it carries a reference that borrows local …"). Since #174 also
    the CLOSURE variant: a returned capturing closure (gate on
    `contains_closure(ret)`) is rooted at the join over its captures in
    `emit_apply_partial` (stack-allocated env ⇒ frame-bound), so the same
    local-root rule rejects it through every escape route — `return {literal}`,
    `let f = {..}; f`, etc. ("cannot return this closure: it captures local …").
    This SUBSUMES and replaces the old syntactic E605 analyze check (retired),
    which only saw the closure literal in return position
  - E495: `-> &mutating` without a mutable root (mir verify::check_escapes;
    2b carrier variant: a return TYPE carrying `&mutating` demands a mutable root)
  - E496: ref rooted at a consuming param/receiver (mir verify::check_escapes;
    2b carrier variant for ref-bearing aggregate returns)
  - E497: ref live across a control-flow merge (mir-lower set_terminator)
  - E498: consume-while-borrowed (mir verify `try_consume`) — only when a
    LIVE ref (@guaranteed call result) chains to the consumed value; an
    unattributable blocking borrow stays an uncoded ICE (lowering bug)
- E207: `mutating_through_shared_ref` (body/access_mode.rs) — E-REF-20, the
  const-cast guard; lives in the E203–E206 family. `util::ref_place` is the
  single classifier (also consulted by body/assignment.rs); it must run
  BEFORE the syntactic walk — the receiver check accepts temporaries, so a
  shared-ref receiver would otherwise silently pass.
- E209: `ref_binding_requires_let` — hir-lower codespan code (stmt.rs): a `&`/`&mutating` initializer on `var` or a destructuring pattern (named ref bindings are simple `let`s only; recovery drops the `&`)
- E210: `mutable_borrow_of_immutable` (body/access_mode.rs `check_borrow_init`) — `&mutating expr` of a non-mutable place: let local/field, shared-`&` reach, or a get/set-only member (no `mutating ref` accessor to lend a place)
- E211: `ref_pattern_position` — hir-lower codespan code (pat.rs): `&`/`&mutating` binder pattern outside its supported position (match-arm support = stage 1.5 item 2 place-mode lowering)
- E212: **RETIRED** (closure kinds, Phase F / plan lockstep 6). Was `non_static_capture` (body/closure.rs): a closure could not capture a ref binding or any other non-`Static` value. docs/design/closures.md's Diagnostics table retires it — a VIEW-kind (`normal`/`mutating`) environment holds addresses into the frame that built it and can never escape it (E494), so those captures are sound; mir-lower's view tier captures a ref binding's TARGET address (`view_addr_of_local`). The rejection survives for the OWNING tier only, under E624 (`owning_capture_rejected`). **Do not reuse E212** — it stays retired so old diagnostics/docs keep resolving.
- E499: `borrow_of_temporary` (body/access_mode.rs `check_borrow_init`) — `let r = &<rvalue>`; a borrow names an existing place
- E497 has a SECOND wording (mir-lower `emit_binding_across_merge_error`): a named ref binding still used after an inside-fn terminator that did NOT forward it. Since 2026-06-11 ("1.75") bindings thread through all control flow as @guaranteed block args, so this is a defensive FALLBACK for jumps emitted outside the LiveTracker pattern — no user-reachable shape is known to trigger it
- E208: `assign_through_shared_ref` (body/assignment.rs) — plain assignment
  through a `&T`-returning call/getter (`arr.at(index: i) = v`,
  `cell.value = v`). The compound form (`+=`) is E207 instead (the
  desugared `addAssign` receiver is a mutating use). Division of labor:
  assignment.rs admits `&mutating`-returning targets and rejects plain-value
  call targets with E202 ("left-hand side of compound assignment is not
  assignable" for the Sugar walk); access_mode owns all `&T` mutating-USE
  errors.
- E203/E204/E207 SECOND SITE (body/access_mode.rs `check_mutating_kind_call`): CALLING a `mutating`-KIND closure is an exclusive use of whatever holds it (docs/design/closures.md; plan D8 routes it through the mutability band, not the kind machinery). `let`-held → E203, immutable field → E204, shared-`&` reach → E207; an inline literal (Temporary) and a `var`/`mutating`-param holder pass. This is about CALLING — PASSING a closure-typed argument to a `mutating` parameter is exempt (`arg_is_closure_value`), because there the convention means "the callee calls it exclusively", not "the callee writes back into your binding".
- E500: `use_after_move` (body/move_tracking.rs)
- E501: `maybe_moved` (body/move_tracking.rs)
- E502: `cloneable_field_requires_conformance` (decl/cloneable_field.rs)
- E503: `move_out_of_borrow` (body/move_tracking.rs) — moving a non-Copyable value bound from a borrowed scrutinee; backstopped in MIR lowering by `emit_copy_value` (kestrel-mir-lower `body/mod.rs`), which emits the same code E503 for shapes the front-end can't see (e.g. binding decay of a ref to a NotCopyable pointee)
- E504: `dangling_pointer_ref` (body/dangle_ref.rs) — WARNING: ref-returning body returns `Pointer(to: <same-fn local>).value`/`.mutatingValue` (traced through single-assignment `let` pointers); the storage dies at return. Claims nothing beyond that shape (references-gaps.md §10.3). Wrapper recognition shares `kestrel_type_infer::RetRefPointerDerived` (moved there from mir-lower so both can reach it)
- E505: `static_requires_static_type` (decl/static_value_type.rs) — references 2a: a module-level value decl or `static` member whose type is non-Static (globals live for the whole program; only reference-free types may be stored). Selection mirrors MIR `lower_static` (module-parent Field without Callable, or `Static`-marked member); Computed skipped; inert without the Static builtin. Predicate = `kestrel_semantics::hir_type_is_static` (the staticness kernel — single source of truth; solver + analyze mirrors route through `instance_is_static`)
- E506: `move_captured_out_of_closure` (body/move_tracking.rs) — #177: a closure captures a non-Copyable value BY VALUE (whole-local Read capture, moved into the env since it can't be copied) and then moves it OUT of the body (return/tail/consume/rebind-and-escape). A closure may be called more than once but owns a single value, so this would double-deinit. Borrowing a captured value across calls is fine (a borrow records no move). The capture itself is recorded as a move of the root in the *enclosing* scope (so later use of the root is a clean E500, not an OSSA ICE). Capture plan from `kestrel_type_infer::ClosureCaptures` (place-based, single source of truth)
- E507: `freeze_violation` (body/move_tracking.rs) — THE FREEZE RULE (docs/design/closures.md §"The freeze rule"; plan D8), the closure analogue of E498. While a live VIEW-kind (`normal`/`mutating`) closure carries a view of a place, that place is frozen against DESTRUCTION: moving it, passing it to a `consuming` parameter/receiver, or `deinit`ing it is E507; plain reassignment stays legal (a write through a live view). PLACE-GRANULAR: `State.frozen: HashMap<PlaceKey, FreezeInfo>` from the `ClosureCaptures` plan, overlap tested with `PlaceKey::is_prefix_of` in BOTH directions, so `{ self.data }` freezes `self.data` and not all of `self`. Four wordings (`FreezeReason`): move / consume / deinit / outlives.
  - LEXICAL ENDPOINTS: `analyze_block` is the single snapshot/restore point (`restore_frozen`) — a freeze introduced inside a block survives it only if a binding declared OUTSIDE still carries it.
  - SCOPE-DEPTH (outlives) HALF: `FreezeInfo.captured_scope_depth` + `State.local_depth`; storing a view-CARRYING value into a shallower binding is E507 even though nothing is moved. That is the only rule that catches the silent scope-exit dangle (`g = { r.v }` inside a block).
  - CARRIERS: `State.carriers` propagates the frozen set through closure copies and aggregate construction only. A general call result is deliberately NOT a carrier (a view closure can never be returned — E494), which keeps `let n = sink(f)` from manufacturing a false outlives error.
  - SEPARATE REPORTED SET: `State.freeze_reported` is NOT `State.reported` — the latter is shared across E500/E501/E503/E506, so reusing it would let an unrelated move diagnostic swallow the freeze error. It threads through the loop back-edge re-analysis alongside `reported`, or the second pass double-emits.
  - `record_move` consults it FIRST and pre-empts E506/E503; `HirStmt::Deinit` needs its OWN consult (it bypasses `record_move`), and `record_operand_move` needs a PLACE-aware one (`rhs_local` returns `None` for `sink(p.b)`). **Next free E5xx is E508.**
- E615: `main_not_free_function` (compilation/entry_point.rs) — `@main` must be a free (module-level) function
- E616: `invalid_main_return_type` (compilation/entry_point.rs) — `@main` must return `()` or a `lang` primitive integer (i8/i16/i32/i64), not a stdlib `IntN` struct
- E617: `multiple_main` (compilation/entry_point.rs) — more than one `@main` in the build
- E618: `missing_main` (compilation/entry_point.rs) — executable build with no `@main`; gated on `CompilationContext::is_executable` (set by the driver's `analyze_all(is_executable)`), so it fires only for `kestrel build` / execution tests, never for libraries / `kestrel check` / LSP / diagnostics tests
- E619: `duplicate_read_provider` (decl/place_accessor.rs) — `get` + `ref` on one subscript/computed property (stage-1.5 place accessors)
- E620: `duplicate_write_provider` (decl/place_accessor.rs) — `set` + `mutating ref` on one member
- E621: `ref_accessor_in_protocol` (decl/place_accessor.rs) — ref accessors are concrete-inherent-only; rejected in protocols and protocol extensions
- E622: `accessor_missing_read_provider` (decl/place_accessor.rs) — write provider with no `get`/`ref` (set-only / mutating-ref-only accessor blocks)
- E623: `function_missing_body` (decl/function_body.rs) — was E606, moved 2026-07 (E606 stays `cannot_infer_closure_type`, body/closure.rs). The dead `capturing_closure_escape` descriptor that shared E605 with extern_ffi_safe was deleted (check lives in MIR as E494); E605 is FFISafe-only.
- E624: `closure_kind_mismatch` (`InferError::KindMismatch`, solver — reported from `solve_coerce`/`solve_equal`, rendered in kestrel-compiler `diagnostic.rs`) — the closure passing table of docs/design/closures.md. ONLY the table reports here: the signature-level kind/convention pairing is E625 and a non-exclusive `mutating` call is the E203 mutability family. E624 is SHARED with `body/closure.rs`'s `owning_capture_rejected` (plan D4): an `escaping`/`consuming` literal capturing a ref binding or other `not Static` value. Both are "this closure kind cannot be formed from this", so they deliberately share the code; the analyzer descriptor and the solver error must keep the same id
- E625: `closure_kind_convention_pairing` (decl/closure_kind_convention.rs) — a `mutating`-kind function-type parameter must have the `mutating` access mode, a `consuming`-kind one the `consuming` mode. Signature-level (a bodiless decl never generates a `Coerce`, so it can never reach E624), exact pairing, top-level param type only. **Next free E6xx is E626.**
- E700: `invalid_escape_sequence` (body/string_escape.rs)
- E701: `ascii_escape_out_of_range` (body/string_escape.rs)
- E702: `invalid_unicode_escape` (body/string_escape.rs)
- E703: `incomplete_escape_sequence` (body/string_escape.rs)

## Key Conventions

- Analyzers are **stateless ZSTs** — no fields, no mutable state
- Use `cx.query` to read ECS components (`NodeKind`, `Name`, `Callable`, `TypeAnnotation`, etc.)
- A `CompilationCheck` may gate on `cx.is_executable` (true only when building a binary) for whole-program requirements that must not fire on libraries / `kestrel check` / the LSP — e.g. the entry-point requirement E618. Module entities carry **no `DeclSpan`** (and no `FileId`), so anchor whole-program diagnostics on a declaration's span, not a module's.
- Use `cx.hir` to iterate the HIR body, `cx.typed` for resolved types
- Return `Vec<AnalyzeDiagnostic>` — the framework handles accumulation and memoization
- Select descriptors **by code**, never by position: `descriptor("E210").id`,
  not `DESCRIPTORS[5].id`. A positional index ties a code to an array slot with
  nothing to enforce it, so inserting a descriptor in the middle silently
  reassigns every later code, its severity and its docs link. Copy the two-line
  `fn descriptor(id: &str)` helper (see `body/access_mode.rs`)
- Prefer early returns for inapplicable entities (wrong NodeKind, no return type, empty body, etc.)

## Diagnostic codes: the four things that are enforced

`lib.rs::assert_owned` runs on **every** analyzer result, and `registry.rs` has
the compile-time half. Together they hold four invariants — none of which used
to be checked beyond descriptor-id uniqueness (F16):

1. **Ownership.** An analyzer may only emit codes it declares. Declare it in
   your own `DESCRIPTORS`, or — when you legitimately report a code another
   analyzer owns, from a different position — list the owner's descriptor in
   `Describe::borrowed_descriptors()`. Precedent: `GenericsAnalyzer` borrows
   E476 for an unresolved *where-clause* bound, the same "cannot find type 'X'
   in this scope" fact `type_annotation_resolution` reports for annotations.
   This is what stopped E436 from meaning two unrelated things at once.
2. **Uniqueness.** Descriptor ids AND names are unique across all analyzers;
   `AnalyzerId`s are unique within each list (`find_body_check` is linear
   first-match, so a duplicate id makes the second analyzer *never run*).
3. **Reservations are explicit.** A registered code with no emit site goes in
   `registry.rs::RESERVED_UNEMITTED` with a reason. Reserving is fine;
   reserving silently is how E600 and E602 came to be documented as live
   diagnostics with worked examples the compiler cannot produce. The list is
   checked both ways — a reserved code that fires is an error too.
4. **Documentation.** Every registered code appears in `docs/error-codes.md`
   (`every_registered_code_is_documented`). Codespan-emitted codes (E100, the
   E48x reference-position family from hir-lower, E49x/E5xx from mir-lower) are
   *not* registry descriptors and are not covered by that test — document them
   by hand.

## One analyzer per fact

If two analyzers ask the same question (e.g. irrefutable-pattern and
exhaustiveness both run Maranget), merge them. Two analyzers computing the
same thing drift — one gets updated, the other doesn't, diagnostics
disagree at the edges. Precedent: E302 / E303 / E306 all describe the
same pattern-matrix fact and live in `exhaustiveness.rs`. The old
`irrefutable_pattern.rs` was deleted when its logic duplicated the
exhaustiveness walk.

A single analyzer can own multiple diagnostic codes. Use a lookup helper
(`fn descriptor(id: &str) -> &'static DiagnosticDescriptor`) when
selecting by code at emit time.

## Pick one diagnostic per fact, and pick the one that labels the fix

When two codes describe the same situation (cause vs effect, umbrella vs
specific), emit only one. Prefer the diagnostic whose label points at the
code the user needs to change. Precedent: E306 (unreachable pattern —
labels the dead code) beats E303 (irrefutable-cause — labels the arm that
*caused* the dead code); they describe the same fact, E306 is actionable.

## Source-based dispatch for desugared constructs

When analyzing `HirExpr::Match`, branch on `source` first. `UserMatch`
gets the full diagnostic suite; desugared sources get source-specific
codes or are skipped entirely. See `MatchSource::is_desugared()` and the
per-source dispatch in `exhaustiveness.rs`.

## Errors-as-data on HIR nodes (when lowering must be the source of truth)

Some checks need information that only the lowering pass has — escape
decoding, integer parsing, etc. — and the resulting *value* must be
canonical (codegen consumes it; we can't decode twice). Don't push
diagnostic emission into lowering. Instead:

1. Define a typed error variant in `kestrel-hir` next to the literal /
   node it pertains to (e.g. `EscapeError` next to `HirLiteral::String`).
2. Have lowering return `(value, Vec<Error>)` purely — no `&mut sink`,
   no `ctx.accumulate`. Store the error list as a field on the HIR node
   itself, not as a side-table on `HirBody` (side-tables drift from the
   nodes they describe; see "Source-based dispatch" above for the same
   anti-pattern).
3. Write an analyzer that walks the relevant arena (`cx.hir.exprs`,
   `cx.hir.pats`) and translates the per-node error list into
   `AnalyzeDiagnostic`s.

Hash impls on the carrying enum should hash the *value*, not the error
list — errors are a derived property of the source and including them
breaks identity-based memo keys.

Precedent: `HirLiteral::String { value, escape_errors }` →
`body/string_escape.rs` (E700-E703). Decoder lives in
`kestrel-hir-lower/src/literal.rs`.

Do not check for desugared-ness via side-tables on `HirBody`
(`for_loop_matches` was removed for this reason). Use the enum on the
node.

## Read the flags on `BuiltinKind::Protocol` — don't hardcode the protocol

`ProtocolFieldConformanceAnalyzer` used to resolve `Builtin::FFISafe` directly,
which left `requires_fields_conform` and `tuple_conformance_propagation` as
fields with **zero readers** while the module doc advertised a data-driven rule
that did not exist. A second protocol setting either flag would have been
silently ignored.

The pattern to follow is `builtin_marker_protocol.rs` and `conformance_rules.rs`:
query `EntityBuiltin`, pattern-match the flag you care about, and let the answer
come from the table. A flag with no reader is not documentation — it is a lie
that compiles.
