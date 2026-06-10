# Stage 1.5 — Errors

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

## `&` patterns

- `&mutating v` against a non-mutable scrutinee place — same predicate
  family as E495 (mutable root).
- `&` binding in a match whose scrutinee is an rvalue that cannot be
  pinned for the match's duration — expected to be representable as a
  match-scoped temp, so likely NO error; confirm during semantics work.

## Named ref bindings (open — follows `semantics.md`)

Known candidates: binding a ref past its referent's scope; `&` of a
temporary in a let-initializer; store-through (`r = v`) on a shared-`&`
binding (E208 family); rebinding spelling rejected (no `var r = &x`).
