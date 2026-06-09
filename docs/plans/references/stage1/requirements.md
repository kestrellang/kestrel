# Stage 1 — Requirements

**Goal:** returnable references — `-> &T` and `-> &mutating T` — under the
uniform root rule, plus the Pointer bridge accessors that make stdlib
collection access real.

## Deliverables

1. Reference return types, both mutabilities (the `&mutating` ban is lifted
   — `references-gaps.md` §10.4), gated by the **root rule**: the returned
   ref's provenance root ∈ {Param (a *mutable* one for `&mutating`), Static,
   PointerDerived}.
2. The escape checker: `root_provenance` stamp
   (`Param(idx)/Static/Local/PointerDerived`) copied O(1) through
   projections; return-site check in `verify.rs`.
3. The `ret_borrow` return convention: `MonoFunction` bit, the two
   copy-guard gates, the `set_terminator` carve-out, codegen in **both**
   backends.
4. `Pointer.value` / `.mutatingValue` (public, `# Safety` docs) → stdlib
   accessors (`Array.first/at`, Dict internals).
5. `&mutating T → &T` coercion (one-way, `solve_coerce` arm).
6. **Transparent place** (Q8 = (a), `semantics.md`): the `solve_member`
   receiver peel, the **convention-aware** `&T → T` copy-out coercion arm
   (borrow-convention argument positions are place contexts — no copy;
   decided 2026-06-09), the ref-aware `classify_mutability` extension
   (E-REF-20), and the `codegen_byref_scalar_deref` bug fix it depends on.
7. Stage-1 negative rules (`errors.md`): no cross-merge ref *places*, no
   function-value / `throws` / generic-arg leakage. (Named bindings and
   `match` scrutinees are no longer errors — they decay; `semantics.md`.)

## Entry criteria

- Stage 0.5 landed.
- ~~Q8 decided~~ **done 2026-06-09: transparent place (a)** —
  `references-gaps.md` §10.5.

## Success criteria

- Full test matrix green via `/triage`, **including one
  `KESTREL_BACKEND=llvm` execution lane** — a missed LLVM twin is a
  wrong-ABI miscompile invisible to the Cranelift-only harness.
- The `return_borrow_of_local` class is rejected at compile time; it never
  reaches runtime as a UAF.

## Effort / risk

~14-20 wk after 0.5. Dominant risk: the conditional-`@guaranteed`-return
surgery (`references.md` §5 #1) — the verifier no-ops on `@guaranteed`, so
mistakes here are silent miscompiles. **Land the escape checker first.**
