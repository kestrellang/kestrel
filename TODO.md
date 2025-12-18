# Kestrel TODO

## Phase 7: Type Inference

- [ ] Extension Specialization Overlap Detection
  - [ ] Allow non-overlapping specialized extensions (`Box[Int]` vs `Box[String]`)
  - [ ] Only reject truly ambiguous cases

## Phase 8: Closures & First-Class Functions

- [ ] Closure Expressions
  - [ ] Closure syntax (e.g., `{ x, y in x + y }` or `func(x, y) { x + y }`)
  - [ ] Capturing variables from enclosing scope
  - [ ] Capture semantics (by value vs by reference)
- [ ] Function References
  - [ ] Reference named functions as values
  - [ ] Pass functions to higher-order functions
- [ ] Closure Type Inference
  - [ ] Infer parameter types from context
  - [ ] `numbers.map({ n in n * 2 })` infers `n: Int`
