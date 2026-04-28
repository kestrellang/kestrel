<p align="center">
<img src="site/public/kestrel-bird.png" alt="Kestrel" width="200">
</p>

# Kestrel

[kestrel-lang.com](https://kestrel-lang.com)

Kestrel is a programming language with clean syntax, a powerful type system and deterministic memory management. Kestrel is currently in its first preview release, and is able to be used to write 2d games and web apps. It compiles to native code via Cranelift, and ships with a full ecosystem: package manager, web framework, http client, vscode extension, and more - many written in Kestrel themselves.

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
            throw CafeError(reason: "not enough beans for \(order.drink)")
        };
        self.beansLeft -= order.shots;
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

- **Value semantics** — copy-on-assignment, `not Copyable` for move-only types, copy-on-write collections
- **Protocols and extensions** — polymorphism with retroactive conformance
- **Monomorphized generics** — zero-cost abstractions with `where` clause constraints
- **Algebraic data types** — enums with associated values and exhaustive pattern matching (`match`, `if let`, `while let`)
- **Error handling** — `throws` / `try` sugar over `Result[T, E]`
- **RAII** — deterministic cleanup via `deinit`
- **Closures** — trailing closure syntax, implicit `it` parameter
- **Type inference** — bidirectional constraint-based inference
- **String interpolation** — `"\(expr)"` via the Formattable protocol
- **Iterators** — `for`-`in` loops with 20+ adapters (map, filter, zip, scan, take, ...)
- **C interop** — `@extern(.C)` for calling C functions and linking native libraries
- **Parameter labels** — named parameters for readable call sites

## Ecosystem

Every tool below is written in Kestrel:

| Tool                              | Description                                                                   |
| --------------------------------- | ----------------------------------------------------------------------------- |
| [**Flock**](lang/flock)           | Package manager — dependency resolution, registry, lock files, TOML manifests |
| [**Jessup**](lang/jessup)         | Toolchain version manager (like rustup)                                       |
| [**Perch**](lang/perch)           | Web framework — routing, middleware, generic context                          |
| [**Swoop**](lang/swoop)           | HTTP/HTTPS client                                                             |
| [**Clutch**](lang/clutch)         | CLI argument parser                                                           |
| [**Quill**](lang/quill)           | Serialization framework                                                       |
| [**Quill JSON**](lang/quill-json) | JSON support for Quill                                                        |
| [**Quill TOML**](lang/quill-toml) | TOML support for Quill                                                        |
| [**HTTP**](lang/http)             | Shared HTTP types                                                             |
| [**Plume**](lang/plume)           | Template engine                                                               |

### Example apps

- [**Weather Dashboard**](examples/weather) — full-stack web app using Perch, htmx, and the Open-Meteo API
- [**Pokédex**](examples/pokedex) — Kanto Pokédex using PokéAPI, Perch, and Plume templates
- [**Wordle**](examples/wordle) — Wordle clone with shareable URL state
- [**APOD**](examples/apod) — NASA Astronomy Picture of the Day viewer
- [**Counter**](examples/counter) — HTMX counter app with Perch
- [**Game of Life**](examples/life) — Conway's Game of Life rendered with SDL2
- [**Breakout**](examples/breakout) — terminal brick breaker with Iterator-based game loop
- [**Snake**](examples/snake) — terminal snake with RAII terminal management
- [**Pong**](examples/pong) — terminal pong with AI opponent
- [**SDL Pong**](examples/sdl_pong) — graphical pong via SDL2 FFI bindings

## Quick Start

```bash
# Install the compiler
cargo install --git https://github.com/jkpdino/kestrel

# Run a program
kestrel run file.ks

# Check for errors without running
kestrel check file.ks

# Build an executable
kestrel build file.ks -o output
```

### Using Flock (package manager)

```bash
flock new myproject
cd myproject
flock run
```

## Editor Support

Kestrel ships with a language server (`kestrel-lsp`) and a VS Code extension.

**Features:** diagnostics, go-to-definition, hover, completions, signature help, rename, code actions, document symbols, semantic highlighting, and inlay hints.

```bash
# Install the language server
cargo install --git https://github.com/jkpdino/kestrel kestrel-lsp
```

The [VS Code extension](editors/vscode) picks up `kestrel-lsp` from PATH automatically. You can also point it at a custom binary via the `kestrel.lsp.path` setting.

## Standard Library

All public stdlib types are auto-imported — no `import` statements needed for common types.

| Module          | Contents                                                            |
| --------------- | ------------------------------------------------------------------- |
| **core**        | Protocols (Equatable, Comparable, Hashable, Cloneable, Formattable) |
| **collections** | Array, Dictionary, Set with copy-on-write semantics                 |
| **result**      | Optional (`T?`) and Result types with `try` operator                |
| **text**        | String with Unicode support                                         |
| **io**          | File I/O, stdin/stdout, networking                                  |
| **memory**      | Allocator, Buffer, Pointer, reference counting                      |
| **iter**        | Iterator protocol and 20+ adapters                                  |
| **num**         | Int8–64, UInt8–64, Float32/64                                       |

## Building from Source

Requirements:

- Rust 2024 edition (1.85+)

```bash
git clone https://github.com/jkpdino/kestrel
cd kestrel
cargo install --path .
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
