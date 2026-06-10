# Stage 0.5 — Errors

**Codes allocated (implemented): E480–E489**, recorded in
`lib/kestrel-analyze/AGENTS.md`. Emitted from HIR lowering via codespan
`with_code` (`kestrel-hir-lower/src/ty.rs::reject_ref_types`; E488 from
`desugar.rs`), not analyzer descriptors; the test matcher passes codespan
codes through.

| # | Code | Trigger | Message sketch |
|---|---|---|---|
| E-REF-01 | E480 | ref type in parameter position (function/method/init params, function-type param lists, closure params) | "parameters are not reference-typed; spell the convention instead — `x: T` borrows, `mutating x: T` mutably borrows". **Permanent rule** — word it as "is not", never "not yet" |
| E-REF-02 | E481 | ref type in return position | "reference return types are not supported yet" |
| E-REF-03 | E482 | ref type in `var`/`let` annotation | "references cannot be stored in bindings" |
| E-REF-04 | E483 | ref type in a struct/enum field (incl. enum case payload) | "references cannot be stored in fields" |
| E-REF-05 | E484 | ref type in a tuple element | "references cannot be stored in tuples" |
| E-REF-06 | E485 | ref type as a generic type argument (`Array[&T]`, `Optional[&T]`, `&T?`) | "references cannot be used as type arguments" |
| E-REF-07 | E486 | ref type as a function-type **return** (`() -> &T`) | not supported yet |
| E-REF-08 | E487 | nested reference (`&&T`, `&mutating &T`) | "a reference cannot reference a reference" — one diagnostic per cluster; the positional error surfaces once the nesting is fixed |
| E-REF-09 | E488 | `&` in expression position | parse recovery (`UnaryOp::Borrow`) + "borrow expressions are not written; the signature decides" |
| — | E489 | ref type anywhere else (alias RHS, where-clause, protocol bound) | "reference types cannot be used here" |

No new place-mutability diagnostics: `Pointer(to:)` is a plain borrow and
accepts any place (`references-gaps.md` §10.2, revised — no `mutating:`
init), and `mutating`-param checks (E200-class) are pre-existing.

Notes:

- E-REF-01..07 are one **type-position walk** at HIR lowering (pattern:
  `contains_opaque`), not per-feature checks. Keep it one function so stage1
  can carve the return position out of it. E-REF-01 is the only row that is
  never carved out.
- Extern functions need no dedicated row — extern params hit E-REF-01 like
  any other parameter position.
- Parser recovery: illegal ref positions must still produce a parse tree
  (LSP rule — `parser_recovery_pattern`).
