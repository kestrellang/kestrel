# Diagnostics

Every error and warning the compiler emits has a code — `E0001`, `W0042`, etc. — and a permanent home on this page. Clicking through from your editor's diagnostic, or grepping the source for an E-code you saw in CI, lands here.

## How to read a diagnostic

A typical message looks like:

```
error[E0203]: cannot infer type of empty array
  --> src/main.ks:14:13
   |
14 |     let xs = []
   |             ^^ no element type given
   |
   = help: write `let xs: [Int] = []`, or use `[Int]()`
```

- **`E0203`** — the diagnostic code. Permanent identity for this kind of error.
- **`error`** — the severity. Some codes can be downgraded to warnings or upgraded to errors via project config.
- **The location and span** — where the compiler saw the problem.
- **The note and help** — what the compiler thinks went wrong, and a likely fix.

The fix isn't always right — it's the compiler's best guess. Read the rule on this page if you're unsure.

## Categories

Diagnostics are grouped by phase:

| Range | Phase |
|---|---|
| E0001–E0099 | Lexer |
| E0100–E0199 | Parser |
| E0200–E0299 | Name resolution |
| E0300–E0399 | Type inference |
| E0400–E0499 | Trait & protocol resolution |
| E0500–E0599 | Borrow / access mode checking |
| E0600–E0699 | Lowering & MIR |
| E0700–E0799 | Codegen |
| W0001–W0099 | Style and dead-code warnings |
| W0100–W0199 | Suspicious-but-legal patterns |

This page lists each code, what triggers it, and the canonical fix. Use the search at the top of the docs to jump straight to a code.

## Auto-generation

This page is generated from the compiler's diagnostic registry on each build. Anything missing here is missing from the source — file an issue if you hit a code that has no documentation.

---

[← Reference](index.md) · [↑ Reference](index.md) · [Stdlib →](stdlib.md)
