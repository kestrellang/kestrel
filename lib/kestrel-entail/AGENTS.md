# kestrel-entail

The one where-clause entailment kernel. See `docs/architecture.md`.

- **Never write a second evaluator.** There used to be five, with three
  different meanings of "unknown", and equality clauses were skipped as true.
  Matching, substitution, the three-valued AND/OR and specificity live only
  here. A layer (`FrontEndLayer`, `SolverLayer`, `MonoLayer`) supplies world
  facts through `EntailLayer` and nothing else.
- **`Tri::Unknown` is never "permit".** It means an open inference variable or
  a depth cut-off blocks the answer. The solver re-queues the goal; callers
  with complete types decide what it means for them, explicitly.
- **`ETy::Poison` holds**, so an error already reported doesn't cascade.
- **`supply` is never called with `ETy::Var` or `Poison`.** The kernel handles
  those before asking the layer.
- A new goal shape or rule needs a kernel test in `tests.rs` over the toy world.
- Depends only on `kestrel-hecs`, so mono can use it. Keep it a leaf.
