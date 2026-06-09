# Stage 0.5 — Errors

Codes TBD — allocate a contiguous E-block per the analyzer AGENTS.md
conventions before implementation; placeholders below.

| # | Trigger | Message sketch |
|---|---|---|
| E-REF-01 | ref type in parameter position (function/method/init params, function-type param lists) | "parameters are not reference-typed; spell the convention instead — `x: T` borrows, `mutating x: T` mutably borrows". **Permanent rule** — word it as "is not", never "not yet" |
| E-REF-02 | ref type in return position | "reference return types are not supported yet" |
| E-REF-03 | ref type in `var`/`let` annotation | "references cannot be stored in bindings" |
| E-REF-04 | ref type in a struct/enum field | "references cannot be stored in fields" |
| E-REF-05 | ref type in a tuple element | same family |
| E-REF-06 | ref type as a generic type argument (`Array[&T]`, `Optional[&T]`, `&T?`) | "references cannot be used as type arguments" |
| E-REF-07 | ref type as a function-type **return** (`() -> &T`) | not supported yet |
| E-REF-08 | nested reference (`&&T`, `&mutating &T`) | "a reference cannot reference a reference" |
| E-REF-09 | `&` in expression position | parse recovery + "borrow expressions are not written; the signature decides" |
| (existing) | immutable argument to `Pointer(mutating:)` or a `mutating` param | reuse the E200-class place-mutability diagnostics — same check as `mutating` params today |

Notes:

- E-REF-01..07 are one **type-position walk** at HIR lowering (pattern:
  `contains_opaque`), not per-feature checks. Keep it one function so stage1
  can carve the return position out of it. E-REF-01 is the only row that is
  never carved out.
- Extern functions need no dedicated row — extern params hit E-REF-01 like
  any other parameter position.
- Parser recovery: illegal ref positions must still produce a parse tree
  (LSP rule — `parser_recovery_pattern`).
