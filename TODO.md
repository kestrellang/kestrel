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

- [x] Enum Declarations
  - [x] Simple enums: `enum Color { case Red, Green, Blue }`
  - [x] Enums with associated values: `enum Option[T] { case Some(T), None }`
  - [x] Recursive enums with `indirect` keyword
  - [x] Indirect recursion detection through structs
  - [x] Generic enums with type parameters and where clauses
  - [x] Enum instantiation (full path `Color.Red` and shorthand `.Red`)
  - [x] Protocol conformance for enums
  - [x] Instance methods in enums
  - [x] Static methods in enums
  - [x] Enum extensions (`extend Color { ... }`)
- [xr] Pattern Matching
  - [x] `match` expressions
  - [x] Exhaustiveness checking
  - [x] Patterns: literals, bindings, enum variants, wildcards
  - [x] Guard clauses in patterns
  - [x] `if let` / `guard let`
