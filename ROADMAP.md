# Kestrel Language Roadmap

This document is the **narrative** roadmap — phase descriptions, version themes, and the design rationale behind major decisions. Active task tracking lives on the [GitHub Project board](https://github.com/orgs/kestrellang/projects). For branch model and release cadence see [docs/contributing/git.md](docs/contributing/git.md).

The history below summarizes work that has shipped. The future sections describe what each version is *about*; the issues that implement them live on the board, grouped by milestone.

---

# Preview 1 (0.1 → 0.15)

The foundation. Every primitive Kestrel needs to be a usable language — types, generics, control flow, pattern matching, an execution graph, a memory model, a standard library, and the basic toolchain.

## Phase 1: Type System Foundation

The first compiler stood up the type system end-to-end: aliases, modules with visibility modifiers, primitive types, structs, fields, protocols (with inheritance and conformance), function declarations, function overloading by arity / types / labels, and first-class function types. Type expressions covered tuples, functions, paths, plus the unit and `!` (never) types.

## Phase 2: Generics

Generic type parameters with defaults and arity checking, generic functions, where-clause constraints, and a substitutions system threaded through complex types.

## Phase 3: Values & Expressions

Literals across the primitives plus arrays and tuples; paths and value-bearing symbols; `let`/`var` bindings and assignment expressions with mutability tracking; function and method calls (including overload resolution by arity + labels and instance methods with auto-injected `self`); chained member access; binary, comparison, logical, and unary operators that desugar to method calls; struct instantiation with implicit memberwise initializers; explicit `init {}` blocks; field access and assignment.

## Phase 4: Control Flow

`if`/`else` as expressions, `while` and `loop` with optional labels, labeled `break`/`continue` resolved to their target loop, and `return` with the never type for control transfer.

## Phase 5: Validation & Type Checking

Initializer verification (every field initialized exactly once before return, with `let` enforcing single-assignment), dead-code detection, exhaustive return analysis, never-type propagation, and full type checking across returns / assignments / calls / array elements / struct equality. Tuple indexing landed here.

> **Note on tuple indexing:** chained access (`t.0.1`) requires intermediate variables. The lexer treats `0.1` as a single float token, and disambiguating after the fact would complicate the lexer in ways we didn't want to pay for. The workaround is small enough that the bare lexer wins.

## Phase 6: Generics & Protocols (deeper)

Generic constraint enforcement (collecting methods from all bounds, Self substitution), static methods on type parameters, the `GenericsBehavior` refactor that eliminated `RwLock<WhereClause>` mutation, associated types (with constraints, qualified bindings, defaults, override support), protocol method linking, extensions with conformances (retroactive, generic, specialized, with priority rules), tighter type-parameter assignability, and where-clause equality constraints.

## Phase 7: Type Inference

Local inference (`let x = 42` → `Int`), generic argument inference at call sites, bidirectional type checking with expected-type propagation, type-parameter substitution for static and generic methods, a Hindley-Milner-style constraint solver, and extension-specialization overlap detection that allows non-overlapping specializations like `Box[Int]` vs. `Box[String]`.

## Phase 8: Closures & First-Class Functions

Closure expressions with by-value capture, function references as values, closure parameter inference (including the implicit `it` for single-arg closures), and Swift-style trailing closure syntax with multi-closure labels.

## Phase 9: Enums & Algebraic Data Types

Simple enums, enums with associated values, recursive enums via `indirect`, generic enums with where clauses, full-path and shorthand instantiation, protocol conformance and methods on enums, enum extensions, and pattern matching with exhaustiveness checking, guard clauses, and `if let` / `guard let`.

## Phase 10: Execution Graph (MIR)

A full mid-level IR: basic blocks with terminators, a control-flow graph, primitive / memory / control / call operations, struct + tuple + array + enum-variant construction, casts, string ops, pointer ops, closure ops (including `FuncToEscaping` and `ApplyPartial`). Item lowering for functions, initializers, structs, enums, protocols, extensions, and auto-generated witnesses. Expression and pattern lowering for everything the language could express. A pass system with `MirPass` / `FunctionPass` traits and fixed-point iteration.

## Phase 11: Memory Model

Parameter access modes (`borrow` / `mutating` / `consuming`), an attribute system, the builtin-protocol registry, `Copyable` / `Cloneable` / `not Copyable` with field-level inference, drop semantics with `deinit` blocks (RAII, conditional drops via flags, early drops, temporary drops at end-of-statement, struct/enum field drop ordering), and generics integration that defaults `[T]` to `[T: Copyable]`.

See [docs/memory-model/](docs/memory-model/) for the full memory model specification.

## Phase 13: Standard Library & Language Features

Computed properties and subscripts, protocol extensions with default implementations, the try operator desugaring through `Tryable` / `FromResidual` / `ControlFlowEnum`, the literal-protocol family (`ExpressibleBy*`), `Matchable` / `BooleanConditional` / `Formattable`, init where clauses, associated types in extensions, language intrinsics (cast / integer / float / pointer / atomic in the `lang` namespace), enum-case shorthand, delegating initializers, full string-escape coverage, multi-file spans, and the `--std` / `--no-std` flags.

The standard library itself shipped: I/O, the pong example, reference counting, formatting, error handling, and the test suite that exercises all of it.

## Phase 14: Syntactic Sugar

Array / dictionary / optional / result type syntax, optional / result promotion, throw expressions, the try operator, for-loops, short-circuiting `and` / `or`, null-coalescing, range operators, compound assignment, character literals, string interpolation, null literals, dictionary literals, range and array patterns, irrefutable patterns in function parameters, `let`/`var` static-variable consistency, default function parameters, expression-bodied functions, hardened parser errors, and a real hash implementation behind the scenes.

## Phase 15: Compiler Infrastructure

Parser rewrite, symbol-mangling refactor, the deferred-resolution inference engine (lookups moved from the binder to type inference), unified type transformations, semantic-model passes, incremental compilation, `HirTy::SelfType`, the LSP, the website, the **Flock** package manager, and the **Jessup** version manager. Doc comments (`///`) with structured sections.

---

# Preview 2: Types & Expressiveness (0.16 → 0.23)

This preview rounds out the type system surface — opaque and existential types, attribute-driven derives, expression sugar that's been deferred, the property model (lazy / observed / `mutating get`), conditional conformance, and the class runtime. Each version is a 3-week train cycle.

## 0.16 — Opaque types & language gaps

`some Protocol` opaque return types, plus the long-tail of small language gaps that have been collecting: computed properties allowed in protocol extensions, name-collision rules between methods and computed properties, keywords usable as labels, `some` patterns, null patterns, chained guards, normal guard, `Self` constructors, prefix/suffix half-open ranges (`..n`, `n..`), and optional / throwing constructors.

## 0.17 — Boxing & existentials

Existential types (`any Protocol`) — boxed via `GlobalAllocator`, with vtables carrying drop / size / align plus the protocol methods. `any P` is non-Copyable; `Cloneable` is conditional on `P: Cloneable`. Escaping closures get the same boxing treatment when a closure outlives its frame. `indirect case` enum variants heap-box their payloads via the same allocator.

## 0.18 — Attribute system

The full attribute pipeline parsed and propagated through AST → HIR → MIR. Auto-derived protocols arrive (`@derive(Equatable, Hashable, Cloneable, Comparable)`), along with built-in attributes (`@inline`, `@deprecated`).

## 0.19 — Expression sugar

Optional chaining, the pipe operator (`|>`), and placeholder arguments (`_` for partial application).

## 0.20 — Properties & conditional conformance

Lazy properties (`lazy let expensive = compute()`), property observers (`willSet` / `didSet`), `mutating get` on computed properties and subscripts, and conditional conformance (`Box[T]: Copyable where T: Copyable`).

> **`mutating get` matters more than it looks.** It lets a getter modify `self` (value types only); the call site requires a `var` receiver. This unblocks insert-on-read APIs like `Dictionary.subscript(key:inserting:)`, which was removed in 0.13's stdlib because without `mutating get` the documented "insert default on miss" contract couldn't be honored. Keep an eye on what other APIs become expressible once this lands.

## 0.21 — Standard library & polish

Standard-library expansion and depth, compiler-speed work, stdlib speedups, and language refinements informed by 0.16 → 0.20 usage.

## 0.22 — Class runtime

Class declarations with reference semantics, reference counting with control blocks, `===` identity, RTTI via extended vtables, `@weak` / `@unowned` reference attributes, and `@final` classes.

## 0.23 — Refinements (Preview 2 milestone)

Bug fixes and rough edges from 0.16 → 0.22 usage. Class runtime hardening informed by real-world adoption. Documentation and stdlib polish. Stabilization of the Preview 2 surface area.

---

# Preview 3: Concurrency

Built on an effects-lite architecture: async, yield, and throw are modeled as capabilities provided by handlers. The internal design is compatible with the `using` / `given` generalization in Preview 4, and prepares the ground for full algebraic effects in 3.0.

The shape of the work: generators (`generator` / `yield`, CPS / state-machine lowering, lazy sequences), then async / await built on the same state-machine infrastructure (executor, runtime, `Future` type), then atomics and a memory model for concurrent access (`send` / `sync` capabilities for thread safety). Structured concurrency (task groups, cancellation), async iteration (async generators and async sequences), and async stdlib APIs sit on top of all of that. Closing out the preview: actors or a refined concurrency model, multithreading primitives, and concurrency testing/debugging tools.

---

# Preview 4: Expressiveness & Ecosystem

Implicits land here: `using` / `given` implicit parameters, with `GlobalAllocator` migrating to `given Allocator`, and async context (executors, cancellation tokens) migrating to `using`. The effect-lite generalization carries implicit propagation, handler blocks, and effect inference — a stepping-stone to 3.0's algebraic effects.

The rest of the preview is shaped by real-world usage: language features informed by what 0.16 → 0.23 made awkward, the standard library's final shape and depth, and ecosystem tooling.

---

# Release Candidate

LLVM backend, WebAssembly target, `const` compile-time evaluation, `unsafe` blocks and escape hatches, and standard-library stabilization.

---

# 2.0: Metaprogramming

User-defined procedural macros (extending `@derive` from 0.18), compile-time reflection (inspecting types, fields, and conformances), `comptime` blocks, and custom attributes that generate code.

---

# 3.0: Algebraic Effects

User-defined `effect` declarations, `handle` blocks, and effect polymorphism (`func map(f: (A) -> B / E) -> Array[B] / E`). The existing async / generators / throw machinery is reframed as built-in effects. Built-in effects: `async`, `throws`, `yield`, `alloc`, `unsafe`, `const` (purity). The whole control-flow story collapses into a single composable model.
