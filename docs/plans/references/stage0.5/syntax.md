# Stage 0.5 — Syntax

## Reference types (parsed, legal nowhere yet)

- `&T` — shared borrow. `&mutating T` — mutable borrow. `&` is the sigil;
  `mutating` is the existing keyword. No `mut`, no `inout`, no
  `&borrowing T`.
- The grammar lands this stage, but **no position accepts a ref type yet**;
  returns open in stage 1. Parsing-then-rejecting (vs. not parsing) buys
  real diagnostics + LSP recovery and front-loads the parser risk.
- `&` does not exist in expression position. No new lexer tokens
  (`Token::Ampersand` and `Token::Mutating` both exist; `&=` longest-match
  is unaffected).

## Parameters: conventions only (permanent)

- `x: T` = borrow (the default) · `mutating x: T` (labeled form:
  `mutating on x: T`) = mutable borrow · `consuming x: T` = owned by-value.
- `x: &T` and `x: &mutating T` are **rejected** (`errors.md`) — and not
  "yet": parameter position never takes a ref type. `x: &T` would duplicate
  `x: T` and `x: &mutating T` would duplicate `mutating`; one spelling per
  convention (`references-gaps.md` §10.6).
- Function types identically: `(T) -> R` / `(mutating T) -> R` (shipped,
  #106) are the forms; `(&T) -> R` and `(&mutating T) -> R` are rejected.

## CST shape (the one trap)

Emit `TyRef { amp, mutating?, inner }` as a **single atomic `Ty` node** so a
`mutating` token inside a `TyList` is never visible at list level
(`references-syntax.md` §1; `ast_type.rs:85-99` scan). Still load-bearing
even though every ref type is rejected — rejection happens at HIR lowering,
after the positional `Mutating`-scan has already run over the list.

## Call sites

Unchanged and convention-blind — `f(x)` whether the param is borrow,
`mutating`, or `consuming`; the signature decides. LSP inlay-hint surfaces
the convention (shipped Design-B infra).

## Pointer bridge (this stage: one init)

```kestrel
init(to value: T)   // address of the borrowed place
```

One init, no `to:`/`mutating:` pair (`references-gaps.md` §10.2, revised):
`Pointer[T]` is uniformly write-capable, so a `mutating:` twin would have
been two spellings of the same capture carrying only an advisory
place-mutability check. A borrow param accepts any place — `var` or `let`;
writing through a pointer captured from an immutable place is the
documented const-cast footgun (`# Safety` on the init and on `write`).
Dropping the twin also removes the need for `mutating` as an argument
label.

## Grammar notes

- `&` parses prefix in type position before the postfix loop: `&T?` parses
  as `&(T?)`. Moot this stage (all positions rejected); recorded for
  stage 1.
- Nested refs (`&&T`, `&mutating &T`) parse and are rejected.
