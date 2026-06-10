# Stage 1.5 — Errors

> **E492 note (2026-06-10)**: with literal-element decay landed, E492's
> inference-side surface is empty — literal elements decay, consuming
> positions copy out, borrow-convention generic args see through refs
> (§10.5 amendment), and ref-returning callees are E491. The validation
> remains as a backstop; annotation-position rejections are the
> stage-0.5 E480–E489 walks.

Codes ALLOCATED 2026-06-10 (table in `lib/kestrel-analyze/AGENTS.md`):
the accessor decl rules took **E619–E622** in the E6xx decl block where
the subscript decl errors (E607/E608) already live.

## Place accessors (IMPLEMENTED — `decl/place_accessor.rs`)

- **E619 `duplicate_read_provider`** (`get` + `ref`) / **E620
  `duplicate_write_provider`** (`set` + `mutating ref`) on one
  subscript/property — decl-time.
- **E621 `ref_accessor_in_protocol`** — ref accessor on a protocol
  member or protocol-extension member (witness ref-returns out of
  scope; `semantics.md` scope restriction).
- **E622 `accessor_missing_read_provider`** — write provider with no
  `get`/`ref` (set-only and mutating-ref-only blocks, which the
  clause-list grammar now parses).
- **NotCopyable RMW through `get`/`set`** (`x(i).mutate()` with no
  `mutating ref`) — E503-coded copy-guard from the writeback lowering,
  message "cannot mutate a non-copyable `T` element through get/set
  accessors", note "add a `mutating ref` accessor to mutate elements in
  place". (A SETTABLE NotCopyable get/set member usually dies earlier:
  storing the borrowed `newValue` is its own E503 in the setter body.)
- **Write to a read-only place** (`x(i) = v`, ref-only member) — the
  existing E201 no-setter error; RMW on a ref-only member is E207 (the
  read provider's `&T` is shared).
- **RMW through an immutable base** (`let arr; arr(i) += 1`) — E203 at
  the desugared receiver (the member is a place projection; the old
  blanket E202 no longer fires for members with a write provider).
- **Declared `-> &T` subscript** — stays E481 (unchanged; the accessor
  form is the only spelling — pinned in
  `rejected_syntax/return_ref_rejected.ks`).

## Named ref bindings + `&` patterns (IMPLEMENTED — codes allocated)

- **E209 `ref_binding_requires_let`** (hir-lower stmt.rs) — `&`
  initializer on `var` or a destructuring pattern; recovery drops the
  `&`.
- **E210 `mutable_borrow_of_immutable`** (access_mode.rs
  `check_borrow_init` + the match arm) — `&mutating expr` of a let
  local/field, a shared-`&` reach, or a get/set-only member (no
  `mutating ref` accessor to lend a place); also `&mutating v` patterns
  on a non-mutable scrutinee place.
- **E211 `ref_pattern_position`** (hir-lower pat.rs) — `&` binder
  outside match-arm position (let/for destructures, if/while-let,
  params); degrades to a plain binding.
- **E212 `ref_binding_captured`** (closure.rs) — closure captures a ref
  binding.
- **E499 `borrow_of_temporary`** (access_mode.rs) — `let r = &<rvalue>`.
- **E504 `dangling_pointer_ref`** (WARNING, dangle_ref.rs) — item 3's
  lint; see requirements.
- **E497 second wording** (mir-lower) — a binding still used after an
  inside-fn terminator: "ref binding 'r' cannot stay live across a
  control-flow merge"; re-borrow inside the branch or bind the value.
- Match-scoped temp pinning means an rvalue SCRUTINEE is legal in
  place-mode matches (no error) — only `&mutating` needs real mutable
  storage (E210).
- `let r: &T = …` annotations stay E482 (inference-only this stage).
