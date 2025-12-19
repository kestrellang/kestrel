# Kestrel TODO

## Phase 8: Closures & First-Class Functions

- [x] Closure Expressions
  - [x] Closure syntax (e.g., `{ x, y in x + y }` or `{ body }`)
  - [x] Capturing variables from enclosing scope
  - [x] Capture semantics (by value vs by reference)
- [x] Function References
  - [x] Reference named functions as values
  - [x] Pass functions to higher-order functions
- [x] Closure Type Inference
  - [x] Infer parameter types from context
  - [x] `numbers.map({ n in n * 2 })` infers `n: Int`
  - [x] Implicit `it` parameter for single-parameter closures
- [x] Trailing Closure Syntax
  - [x] Swift-style trailing closures
  - [x] Multiple trailing closures with labels

## Phase 9: Enums & Algebraic Data Types

- [ ] Enum Declarations
  - [ ] Simple enums: `enum Color { Red, Green, Blue }`
  - [ ] Enums with associated values: `enum Option[T] { Some(T), None }`
  - [ ] Recursive enums
- [ ] Pattern Matching
  - [ ] `match` expressions
  - [ ] Exhaustiveness checking
  - [ ] Patterns: literals, bindings, enum variants, wildcards
  - [ ] Guard clauses in patterns
  - [ ] `if let` / `guard let`
