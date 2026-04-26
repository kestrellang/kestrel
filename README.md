# Kestrel

A compiled programming language with Swift-inspired syntax, value semantics, and monomorphized generics. The compiler, standard library, package manager, web framework, and tooling were built in ~3 months by one developer using AI-assisted development.

**The ecosystem is self-hosting** — the [package manager](#ecosystem), [version manager](#ecosystem), and several libraries are written in Kestrel itself.

<p>
<img src="site/public/breakout.gif" alt="Breakout game written in Kestrel" width="280">
<img src="site/public/snake.gif" alt="Snake game written in Kestrel" width="280">
<img src="site/public/pong.gif" alt="Pong game written in Kestrel" width="280">
</p>

## A Taste of Kestrel

```kestrel
module Cafe

// Protocols define shared behavior
protocol Describable {
    func describe() -> String
}

// Enums with associated values
enum Roast : Describable {
    case Light
    case Dark
    case Custom(String)

    func describe() -> String {
        match self {
            .Light => "light roast",
            .Dark => "dark roast",
            .Custom(name) => name
        }
    }
}

// Structs with computed properties
struct Order {
    let drink: String
    let roast: Roast
    let shots: Int64

    var price: Int64 { self.shots * 250 }

    func receipt() -> String {
        "\(self.drink) (\(self.roast.describe())) — $\((self.price / 100).format())"
    }
}

// Custom error type for throws
struct CafeError { let reason: String }

// Move-only types with deterministic cleanup
struct Register : not Copyable {
    var orders: Array[Order]
    var beansLeft: Int64

    init(beans: Int64) {
        self.orders = Array[Order]();
        self.beansLeft = beans;
    }

    // throws desugars to Result[(), CafeError]
    mutating func ring(order: Order) -> () throws CafeError {
        if self.beansLeft < order.shots {
            return .Err(CafeError(reason: "not enough beans for \(order.drink)"))
        };
        self.beansLeft = self.beansLeft - order.shots;
        self.orders.append(order);
        .Ok(())
    }

    // RAII: runs automatically when Register goes out of scope
    deinit {
        println("Register closed. \(self.orders.count) orders today.");
    }
}

func main() -> () throws CafeError {
    var register = Register(beans: 10);

    let orders = [
        Order(drink: "Cortado", roast: .Dark, shots: 2),
        Order(drink: "Oat Latte", roast: .Light, shots: 3),
        Order(drink: "Red Eye", roast: .Custom("house blend"), shots: 4),
    ];

    // try unwraps Result, propagating errors
    for order in orders {
        try register.ring(order);
        println(order.receipt());
    }

    // Trailing closures with implicit `it` parameter
    let bigOrders = orders.filter { it.shots > 2 };
    bigOrders.forEach { println("Big order: " + it.drink) };
}
```

## Features

- **Value semantics** — copy-on-assignment with `not Copyable` for move-only types and copy-on-write collections
- **Protocols and extensions** — protocol-based polymorphism with retroactive conformance
- **Monomorphized generics** — zero-cost abstractions with `where` clause constraints
- **Enums with associated values** — algebraic data types with exhaustive pattern matching
- **Error handling** — `throws` / `try` sugar over `Result[T, E]` types
- **RAII** — deterministic cleanup via `deinit` blocks
- **Closures** — trailing closure syntax, implicit `it` parameter
- **Pattern matching** — `match`, `if let`, `while let` with exhaustiveness checking
- **Type inference** — bidirectional constraint-based inference within function bodies
- **String interpolation** — `"\(expr)"` with formattable protocol support
- **Computed properties** — `var name: Type { expression }`
- **Iterators** — `for`-`in` loops with 20+ adapters (map, filter, zip, scan, take, ...)
- **C interop** — `@extern(.C)` for calling C functions and linking native libraries
- **Parameter labels** — named parameters for readable call sites

## Ecosystem

The tooling is written in Kestrel, proving the language works for real software:

| Tool | Description |
|------|-------------|
| [**Flock**](lang/flock) | Package manager — dependency resolution, registry, lock files, TOML manifests |
| [**Jessup**](lang/jessup) | Toolchain version manager (like rustup) |
| [**Perch**](lang/perch) | Web framework — routing, middleware, generic context |
| [**Swoop**](lang/swoop) | HTTP/HTTPS client |
| [**Clutch**](lang/clutch) | CLI argument parser |
| [**Quill**](lang/quill) | JSON and TOML parsing |
| [**Plume**](lang/plume) | Template engine |

### Example apps

- [**Weather Dashboard**](examples/weather) — full-stack web app using Perch, htmx, and the Open-Meteo API
- [**Counter**](examples/counter) — HTMX counter app with Perch
- [**Breakout**](examples/breakout) — terminal brick breaker with Iterator-based game loop
- [**Snake**](examples/snake) — terminal snake with RAII terminal management
- [**Pong**](examples/pong) — terminal pong with AI opponent
- [**SDL Pong**](examples/sdl_pong) — graphical pong via SDL2 FFI bindings

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a program
kestrel run file.ks

# Check for errors without running
kestrel check file.ks

# Build an executable
kestrel build file.ks -o output
```

### Using Flock (package manager)

```bash
flock init myproject
cd myproject
flock run
```

## Standard Library

All public stdlib types are auto-imported — no `import` statements needed for common types.

| Module | Contents |
|--------|----------|
| **core** | Protocols (Equatable, Comparable, Hashable, Cloneable, Formattable) |
| **collections** | Array, Dictionary, Set with copy-on-write semantics |
| **result** | Optional (`T?`) and Result types with `try` operator |
| **text** | String with Unicode support |
| **io** | File I/O, stdin/stdout, networking |
| **memory** | Allocator, Buffer, Pointer, reference counting |
| **iter** | Iterator protocol and 20+ adapters |
| **num** | Int8–64, UInt8–64, Float32/64 |

## Building from Source

Requirements:
- Rust 2024 edition (1.85+)

```bash
git clone https://github.com/jkpdino/kestrel
cd kestrel
cargo build --release
cargo test
```

## Architecture

The compiler is a 6-phase pipeline:

```
Source → Lex → Parse → Semantic Tree → Type Inference → MIR → Cranelift → Native
         │       │          │               │             │        │
       Logos  Chumsky    BUILD/BIND    Constraint      Execution  Codegen
                          phases       Solver          Graph
```

See [docs/contributing/architecture.md](docs/contributing/architecture.md) for details.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
