# Type Inference

Kestrel infers the types of most things, most of the time. You don't have to write `let x: Int = 5` — `let x = 5` is enough. But the rules aren't magic; once you've seen this page, you'll know when to expect inference to do its job and when to give it a hand.

## What gets inferred

- **Local bindings.** `let` and `var` get their type from the right-hand side.
- **Closure parameters and returns**, when the closure is being passed to a function whose parameter type is known.
- **Generic type arguments.** When you call `identity(42)`, Kestrel infers `T = Int` from the argument type.
- **Enum cases via `.Foo` shorthand.** When the expected type is `Color`, `.Red` is enough; you don't have to write `Color.Red`.

## What doesn't get inferred

- **Function parameter types.** Always written.
- **Function return types**, *unless* the function uses the expression-bodied `=` form, in which case the return is inferred.
- **Top-level declarations.** Module-level constants need an explicit type. (Inference inside a single binding works, but cross-file inference is intentionally out of scope.)
- **Empty collection literals.** `let xs = []` has no signal — write `let xs: [Int] = []`.

## Literal defaults

When a literal has no surrounding context, Kestrel picks a default:

- Integer literal → `Int`
- Float literal → `Float`
- Boolean literal → `Bool`
- String literal → `String`
- Character literal → `Char`

When there *is* context, the literal takes that type instead:

```swift
let port: UInt16 = 8080   // 8080 is a UInt16 here
```

## When inference can't decide

Sometimes there genuinely isn't enough information — a generic call where the type parameter doesn't appear in the arguments, or a literal that has to be one of several compatible types. The compiler tells you with an "ambiguous type" error and asks for a hint:

```swift
let x = []                // error: ambiguous
let x: [Int] = []         // ok
let x = [Int]()           // also ok
```

These cases are uncommon in practice; when they show up, write the type and move on.

## A useful mental model

Think of inference as the compiler propagating type information *out* from constants, literals, and function signatures, then unifying everything in the middle. When inference works, it's because that propagation has a single consistent answer. When it doesn't, it's because the propagation either has too many answers or too few — and the fix is to add a type annotation that resolves the ambiguity.

For the deeper semantics — how the solver handles where-clauses, associated types, default conformances — see the compiler internals docs. Day to day, the surface rules above are what you need.

---

[← Concepts](index.md) · [↑ Concepts](index.md) · [Memory Model →](memory-model.md)
