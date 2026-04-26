# The Kestrel Language

Welcome. Kestrel is a compiled, statically-typed language with algebraic types, protocol-based abstraction, and automatic reference counting. It draws ideas from Swift, Rust, and ML; it's designed to feel familiar to anyone who's written code in any of them, and to compile to a single self-contained binary.

This guide moves from setup, to a hands-on tour, to a top-to-bottom walk through the language, ending with tooling and lookup material. You can read it in order or jump to whatever chapter answers your current question.

```swift
module Main
import std.io.stdio.println

func main() -> Int {
    let names = ["Alice", "Morgana", "Robin"]
    for name in names {
        println("Hello, \(name)!")
    }
    0
}
```

## Where to start

- New to Kestrel? Read [Getting Started](getting-started/index.md) and then [the Tour](tour/index.md). 30 minutes total.
- Coming from another language? Skim the Tour, then jump to whatever chapter answers your "wait, how does X work here?" question.
- Looking up something specific? Hit the search box (top-right) or jump straight to [Reference](reference/index.md).

## Contents

1. [Getting Started](getting-started/index.md) — install, run hello-world, set up your editor
2. [A Tour of Kestrel](tour/index.md) — three small programs that build a feel for the language
3. [Values & Variables](values-and-variables.md) — `let`, `var`, primitives, literals
4. [Functions](functions/index.md) — labels, access modes, methods, closures
5. [Control Flow](control-flow.md) — `if`, loops, `guard`, `match`
6. [Collections](collections/index.md) — `Array`, `Dict`, `Set`, `Tuple`, iterators
7. [Structs](structs/index.md) — fields, methods, initializers, computed variables
8. [Enums](enums/index.md) — variants, payloads, pattern matching
9. [Error Handling](error-handling/index.md) — `Optional`, `Result`, `try`
10. [Protocols](protocols/index.md) — Kestrel's primary abstraction
11. [Generics](generics/index.md) — type parameters, `where` clauses, associated types
12. [Extending Types](extending-types.md) — extensions and type aliases
13. [Organization](organization.md) — modules, visibility, imports
14. [FFI](ffi.md) — calling C, exporting to C
15. [Concepts](concepts/index.md) — type inference, the memory model
16. [Tooling](tooling/index.md) — Flock, the LSP, Jessup
17. [Reference](reference/index.md) — diagnostics, stdlib, operators, builtins

---

[Getting Started →](getting-started/index.md)
