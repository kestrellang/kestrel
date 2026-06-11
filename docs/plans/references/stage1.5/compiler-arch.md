# Stage 1.5 — Compiler architecture

## Place accessors — anchors (explored 2026-06-10)

- **Accessor entity shape today**: a subscript/computed-property getter
  lives ON the member entity itself (`Callable` + `Body` on the
  `NodeKind::Subscript`/`Field` entity —
  `kestrel-ast-builder/src/builders/subscript.rs:36-78`,
  `builders/field.rs:107-133`); a setter is a separate CHILD entity
  (`NodeKind::Setter`, `spawn_setter` — `subscript.rs:79-116`,
  `field.rs:135-169`) with its own `Callable` (`[index..., newValue]`,
  Mutating receiver). `ref`/`mutating ref` follow the setter pattern:
  new child entity kinds, each with own `Callable` + `TypeAnnotation`
  (the `&T`/`&mutating T` return) — ~50 LOC per kind in the builder.
  Note a `ref`-only member leaves the parent entity bodyless; the
  builder needs a story for that (or migrate ALL accessors to children).
- **`CallableRefReturn`** (`kestrel-hir-lower/src/ty.rs:884-906`)
  dispatches on entity → the new accessor entities work without query
  changes. E481 carve-out site: `ty.rs:795-801` (computed getters
  carved; subscripts deliberately not — that stays, the accessor
  entities get carved instead).
- **Read/write consistency comes free**: `try_lower_setter_assign`
  (`kestrel-mir-lower/src/body/expr.rs:819-1036`) does NOT re-resolve —
  it reads the inference-pinned subscript entity from the `resolutions`
  map (`expr.rs:976-980`) and takes its setter CHILD
  (`find_setter_child`). The provider rule slots into exactly that
  joint: one resolution, then pick the accessor child per operation.
- **Evaluation order**: RHS before LHS ref fabrication, then
  PtrTo/StoreAssign/EndBorrow (`expr.rs:637-664`; RHS at `:649`, target
  at `:657`). Accessor routing must preserve it.

## Subscript resolution: the inherent-vs-extension problem (CLOSED 2026-06-10 — option 2 chosen)

**Decision: the labeled place form (option 2).** Array's in-place
subscript is `subscript(at index: Int64)`; unlabeled forms stay on the
Slice extension. `bind_arguments` is label-strict (an unlabeled arg never
binds a labeled param), so the label-based fallback routes correctly with
ZERO resolve.rs changes — verified by `accessors/
array_at_subscript_routing.ks` (`arr(at:)` inherent; `arr(i)`,
`arr(1..<3)`, `arr(checked:)` extension). The type-aware fallback-tier
machinery below stays UNBUILT; the analysis is kept for the record in
case a future unlabeled inherent subscript is ever wanted. Residual
accepted cost: `arr(at: 1..<3)` is a type error (no range `at:` form),
and `arr(i) += v` takes the writeback copy path rather than in-place.

**Diagnostic-quality bug discovered while verifying (pre-existing, NOT
stage-1.5)**: a missing METHOD on an `AssocProjection`-typed value (e.g.
calling nonexistent `len()` on an ArraySlice from `arr(1..<3)` — the
slice type spells it `count`) reports as "no matching subscript on type
'Int64'" — wrong member kind, wrong type. Worth a follow-up issue.

### The original analysis (superseded by the decision above)

Facts (verified in `kestrel-type-infer/src/resolve.rs`):

- Member candidates are tiered (`resolve.rs:296-322`): direct +
  own-extension members **compete equally**; protocol-extension members
  (which is what `extend Slice[T]`'s unified subscript is, reached via
  Array's conformance) are a **fallback tier** that fires only when no
  direct/own-ext candidate matches the call's **labels**
  (`resolve.rs:332-361`) — the check is `matches_labels`, i.e.
  **type-blind**.
- Among same-tier candidates with matching labels there is **no
  type-aware disambiguation at all** (`resolve.rs:456-474`): protocol
  requirement resolution is tried, then candidates are ranked by
  *extension-target* specificity only and returned as
  `MemberError::Ambiguous`. Direct members all rank 0. (Consistent with
  the standing design stance: type-based overloads need labels.)

Consequence for item 1: giving Array an inherent
`subscript(index: Int64) -> T { ref ... }`:

- `arr(5)` → direct candidate label-matches → fallback suppressed →
  inherent wins. ✓
- `arr(1..<3)` → the inherent candidate ALSO label-matches (one
  unlabeled arg, type-blind) → fallback suppressed → only candidate is
  `Int64` → **type error. Range subscripts break.** ✗
- `arr(checked: i)` etc. keep working (distinct labels → fallback
  fires). ✓

Options (decision needed before item 1 ships on Array):

1. **(leaning) Type-aware fallback-tier boundary**: the fallback fires
   when no direct/own-ext candidate label-matches *and type-binds* the
   call. NOT general type-based overloading — peers in one tier still
   never disambiguate by type; only the tier boundary becomes
   bindability-aware. Needs a conservative "could-bind" check usable on
   partially-resolved args (an unresolved Int literal could bind
   `Int64`, can never bind `Range[Int64]`) and a defer path when args
   are still unresolved — same deferral discipline solve_member already
   uses for unresolved receivers.
2. Labeled place form (`arr(at: i)` ref subscript, `arr(i)` stays
   get/set) — works today, but two user spellings and the flagship
   `arr(i) += 1` ergonomics goal dies.
3. General type-aware overload resolution among same-label peers —
   rejected; reverses the deliberate no-type-overloads stance.

Note: moving range forms inherent doesn't dodge the problem — two
unlabeled DIRECT subscripts (Int64 + Range/generic) tie at rank 0 and
hard-ambiguate today; option 1 is needed either way.

## Enum-payload projection (capability decided; vehicle = `&` patterns)

`InstKind::EnumPayload` with a `@guaranteed` operand already produces
the payload field's in-place address in both backends (cranelift
`inst.rs:1157` `compile_enum_payload` returns the raw addr when
`ownership == Guaranteed`; LLVM `inst.rs:1218` mirrors it — payload
enums are always carried by address). Pattern bindings then copy out of
the projected address — that copy is the only missing piece.

**No intrinsic** (decided 2026-06-09): an `enum_payload_ptr` intrinsic
would carry an unchecked-tag unsafe contract, and its only near-term
customer (Dict) is deferred until `Optional[&T]`. The surface is **`&`
pattern bindings** (`syntax.md`) — the match proves the tag before the
projection runs: safety by construction. Manual offsets are not an
option for enums (`RcBox.valuePtr` hand-computes its header offset only
because the stdlib BUILT that layout; enum layout is compiler-owned).

## Named ref bindings — SHIPPED shape (2026-06-10)

- **Multi-use registry**: `ref_binding_vals: HashMap<ValueId, LocalId>`
  (+ `ref_binding_remaining` use budgets, decremented once per read
  expr). Members are ALSO in `ref_results`; the registry marks them
  multi-use. Every "single use — end it" decay site funnels through
  `end_ref_if_single_use`; `end_stale_refs_since` excludes them;
  `set_terminator` ends them silently at zero remaining uses or emits
  the binding-worded E497.
- **`lower_borrow_init`** delegates the place matrix wholesale to
  `prepare_call_arg_for_expr` (var slots → BorrowAddr — writes stay
  visible, may-alias; accessor elements → the `mutating ref` child;
  in-place borrows without spurious clones; ref-call pass-through);
  re-borrows of an existing binding get a fresh sub-borrow.
- **Place-mode matches**: the scrutinee place's raw address (`PtrTo` of
  the place view — an OWNED pointer scalar) threads through the
  decision tree's existing block-param machinery; tests/leaves read
  through intra-block `BeginBorrowAddr` views; `&v` leaf bindings are
  REAL nested borrows (`emit_begin_borrow(proj)`) so they're
  scope-tracked — arm-internal control flow hits the binding E497
  policy instead of an untracked-value verify error. No verifier
  changes; `new_block_with_params` never sees a Guaranteed desc.
- ~~`add_guaranteed_block_param` is still only a panic-string
  aspiration~~ IMPLEMENTED 2026-06-11 ("1.75"): bindings thread through
  all control flow as @guaranteed block args via the LiveTracker
  machinery — see `stage2/requirements.md` prerequisite 3 for the
  shipped shape.
- **Dangle lint home**: analyze body check (`body/dangle_ref.rs`), NOT
  verify — it shares `RetRefPointerDerived` (moved mir-lower →
  kestrel-type-infer so analyze can reach it; hir-lower can't host it,
  the impl needs `InferBody`). Gotcha: init-call resolutions key on the
  CALL expr, not the callee expr.
