# Common Workflows

Step-by-step guides for the tasks that come up most often in lib2. For architectural background see [Architecture](architecture.md); for file paths by task see [Quick Reference](quick-reference.md).

---

## Adding a new language feature

A "feature" here means new syntax or a new semantic construct that spans several pipeline stages — a keyword, declaration form, expression, or statement. The general order of operations, from front of pipeline to back:

| Stage | Where the work lives |
|-------|---------------------|
| 1. Tokenize | `lib2/kestrel-lexer/src/lib.rs` |
| 2. Parse | `lib2/kestrel-parser/src/` |
| 3. Syntax node | `lib2/kestrel-syntax-tree/src/` |
| 4. AST type | `lib2/kestrel-ast/src/` (if the feature is body-local) |
| 5. Component / `NodeKind` | `lib2/kestrel-ast-builder/src/components.rs` (if the feature is a declaration) |
| 6. Build from CST | `lib2/kestrel-ast-builder/src/` |
| 7. Name resolution | `lib2/kestrel-name-res/src/` |
| 8. HIR shape | `lib2/kestrel-hir/src/body.rs` |
| 9. HIR lowering | `lib2/kestrel-hir-lower/src/` |
| 10. Inference | `lib2/kestrel-type-infer/src/constraint.rs`, `solver.rs`, `generate.rs` |
| 11. Analyzers | `lib2/kestrel-analyze/src/body|decl|compilation/` |
| 12. MIR lowering | `lib2/kestrel-mir-lower/src/` |
| 13. Codegen | `lib2/kestrel-codegen-cranelift/src/` |
| 14. Tests | `lib2/kestrel-test-suite/testdata/<category>/` |

You won't touch every stage for every feature — a purely syntactic change (e.g. new keyword for an existing construct) may only need 1–3. A new declaration form touches 1–12.

### Recommended order

1. **Write the test first.** Create a `.ks` file under the most relevant testdata directory with the desired behavior. Run `triage <pattern>` — it should fail, and the failure tells you the nearest pipeline stage that doesn't understand the new input yet.
2. **Move stage-by-stage**, re-running the test. Each stage typically fails with a specific error from the next stage downstream.
3. **Add analyzers last**, once the feature compiles and runs. Analyzers encode rules; rules are easier to write when you have a working feature to reason about.

For non-trivial features prefer the `feature` agent skill — it enforces design/plan/test gates so you don't skip stages.

---

## Changing an existing feature

For targeted changes (tweak a diagnostic, adjust inference for one case, modify lowering for a specific pattern), use the `change` skill as a reference — the short form:

1. Identify the two or three pipeline stages involved (use the `kestrel-pipeline` skill if routing is unclear).
2. Update the code.
3. Update the tests that cover it — every `.ks` file under `testdata/` with an `// ERROR:` annotation that touches this rule.
4. Run the relevant testdata subdirectory before running the whole suite: `triage <subdir>` is faster.

If a test has to change, make the change deliberate. `CLAUDE.md` forbids changing a test to cajole it into passing — if the test was right, the code is wrong.

---

## Adding a diagnostic

All user-visible diagnostics are emitted by analyzers or by the inference solver. The two paths differ.

### From an analyzer

1. Pick an unused diagnostic id (search `static DESCRIPTORS` across `lib2/kestrel-analyze/src/` to see what's taken).
2. In the analyzer file, extend the `DESCRIPTORS` slice:
   ```rust
   DiagnosticDescriptor {
       id: "E123",
       name: "your_rule",
       default_severity: Severity::Error,
       category: Category::Correctness,
   }
   ```
3. In the analyzer's `check` method, build an `AnalyzeDiagnostic` with the message, primary label, secondary labels, and optional notes.
4. Add a `// test: diagnostics` `.ks` file under `testdata/` with an `// ERROR:` annotation that matches your full message.
5. Document the diagnostic at the top of the analyzer file (message template, label sources, cascading behavior).

### From the type-inference solver

Inference diagnostics go through `InferError`. Adding a variant requires changes in **five** files — `lib2/kestrel-type-infer/AGENTS.md` has the canonical list. The short version:

| File | Change |
|------|--------|
| `kestrel-type-infer/src/error.rs` | Add the variant + its span arm. |
| `kestrel-type-infer/src/result.rs` | `describe_error()` match arm. |
| `kestrel-compiler/src/diagnostic.rs` | `InferError` → `Diagnostic` match arm. |
| `kestrel-analyze/src/body/type_check.rs` | `format_error()` match arm. |
| `kestrel-compiler-driver/src/lib.rs` | `describe()` and `format_error()` arms. |

Missing any one of these produces a non-exhaustive-match error only when the dependent crate is compiled — so do the whole set in one pass.

To report the error from the solver, call `ctx.report_error(InferError::YourVariant { ... })`. It returns an Error TyVar that you use as the result of the constraint-generation branch, so cascades get absorbed.

---

## Adding an analyzer

1. Decide granularity:
   - **`BodyCheck`** — per function / init body. You receive the HIR body and the typed body.
   - **`DeclCheck`** — per declaration entity. You filter by `NodeKind`.
   - **`CompilationCheck`** — once over the whole compilation. Use for cycle detection or cross-entity conflicts.
2. Create the file under the matching subdirectory:
   - `lib2/kestrel-analyze/src/body/<name>.rs`
   - `lib2/kestrel-analyze/src/decl/<name>.rs`
   - `lib2/kestrel-analyze/src/compilation/<name>.rs`
3. Follow the analyzer skeleton in [Patterns](patterns.md#analyzer-shape):
   - `static DESCRIPTORS` with the diagnostic ids.
   - ZST struct.
   - `Describe` impl (id + descriptors).
   - The relevant check trait impl.
4. Export it from the parent `mod.rs`.
5. Register it in `default_analyzers()` in `lib2/kestrel-analyze/src/lib.rs`, in the correct section. Analyzers run in registration order — place yours after any analyzer it depends on.
6. Suppress on upstream errors: if `cx.typed.errors.is_empty()` is false (for body checks), return `vec![]`.
7. Add diagnostic tests under `testdata/validation/<your_rule>/`.

---

## Adding a stdlib method

The standard library is Kestrel source in `lang/std/`, one module per directory.

1. **Implement the method** in the appropriate `.ks` file. Match the patterns in the surrounding code — visibility, mutating annotations, COW conventions.
2. **Add an execution test** under `lib2/kestrel-test-suite/testdata/stdlib/<type>/<test_name>.ks`:
   ```kestrel
   // test: runs
   // stdlib: true

   module Main

   func main() -> lang.i64 {
       let arr = Array[lang.i64]()
       arr.append(5)
       if arr.count != 1 { return 1 }
       0
   }
   ```
   Non-zero exit codes surface as test failures; each check returns a unique non-zero value so a regression points at the exact assertion.
3. **Run it:** `triage <test_name>`.

---

## Adding a `Constraint` variant

Type-inference constraints live in `lib2/kestrel-type-infer/src/constraint.rs`.

1. Add the variant with the minimum data it needs, and a doc comment explaining the rule ("`a = b` — structural type equality", "`ty : Protocol` — protocol conformance", etc.).
2. Decide whether the variant is **eager** (solves on first visit) or **deferred** (requires concrete input, re-checked each solver round). Document this in the variant's doc comment.
3. Add a `try_solve_*` function in `lib2/kestrel-type-infer/src/solver.rs` and wire it into the solver loop.
4. Generate the constraint where the language construct is lowered — typically in `generate.rs`.
5. Add focused tests under `testdata/inference/`.

---

## Debugging a failing test

See the `debug` skill for the full protocol. The headline rules:

1. **Reproduce first.** `triage <test>` — look at the actual error.
2. **One hypothesis at a time.** Form a specific hypothesis, test it, evaluate the result before forming the next one.
3. **Stop after 3 failed attempts of the same class.** List what you tried, what you ruled out, ask for guidance.
4. **Use `debug_trace!`** in compiler source and rerun with `VERBOSE_DEBUG_OUTPUT=1`. Do not use `eprintln!` / `println!`.
5. **`kestrel dump`** can print intermediate representations (CST, AST, HIR, types, MIR) for a `.ks` file — handy for narrowing the stage where the bug lives.

---

## Committing and opening a PR

See [Git](git.md) for the full branching model. Before you commit:

```bash
cargo fmt
cargo clippy
triage            # full suite — only before commits, not after every edit
```

Commit message prefixes: `feature:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`.

Small, focused commits beat big sweeping ones. A feature commit should include its tests.
