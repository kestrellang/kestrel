<p align="center">
<img src="external/kestrel-website/public/kestrel-hovering.png" alt="Kestrel" width="180">
</p>

<h1 align="center">Kestrel</h1>

<p align="center">
Clean syntax. Powerful types. Deterministic memory.
</p>

<p align="center">
<a href="https://kestrel-lang.com"><img src="https://img.shields.io/badge/docs-kestrel--lang.com-blue" alt="Docs"></a>
<a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green" alt="License"></a>
</p>

<p align="center">
<img src="docs/images/weather.jpeg" alt="Weather Dashboard" width="260">
<img src="docs/images/pokedex.jpeg" alt="Pokédex" width="260">
<img src="docs/images/apod.jpeg" alt="APOD Viewer" width="260">
</p>
<table align="center"><tr>
<td align="center" valign="middle"><img src="docs/images/wordle.jpeg" alt="Wordle" width="260"></td>
<td align="center" valign="middle"><img src="docs/images/life.png" alt="Game of Life" width="260"></td>
</tr></table>

## What is Kestrel?

Kestrel is a compiled programming language with deterministic memory management — no garbage collector, no borrow checker. It compiles to native code via Cranelift and ships with a full ecosystem: package manager, web framework, HTTP client, VS Code extension, and more — many written in Kestrel itself.

Currently in its first preview release, Kestrel can be used to write 2D games, CLI tools, and web apps.

## Quick Start

```bash
# Install the compiler
cargo install --git https://github.com/kestrellang/kestrel

# Run a program
kestrel run hello.ks

# Or use Flock (package manager)
flock new myproject && cd myproject && flock run
```

## Contents

- [A Taste of Kestrel](#a-taste-of-kestrel)
- [Features](#features)
- [Ecosystem](#ecosystem)
- [Examples](#examples)
- [Editor Support](#editor-support)
- [Standard Library](#standard-library)
- [Building from Source](#building-from-source)

## A Taste of Kestrel

```kestrel
module Cafe

protocol Describable {
    func describe() -> String
}

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

struct Order {
    let drink: String
    let roast: Roast
    let shots: Int64

    var price: Int64 { self.shots * 250 }

    func receipt() -> String {
        "\(self.drink) (\(self.roast.describe())) — $\((self.price / 100).format())"
    }
}
```

```kestrel
// Move-only types with deterministic cleanup
struct Register : not Copyable {
    var orders: Array[Order]
    var beansLeft: Int64

    init(beans: Int64) {
        self.orders = Array[Order]();
        self.beansLeft = beans;
    }

    mutating func ring(order: Order) -> () throws CafeError {
        if self.beansLeft < order.shots {
            throw CafeError(reason: "not enough beans for \(order.drink)")
        };
        self.beansLeft -= order.shots;
        self.orders.append(order);
    }

    // Runs automatically when Register goes out of scope
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

    for order in orders {
        try register.ring(order);
        println(order.receipt());
    }

    // Trailing closures with implicit `it` parameter
    orders.filter { it.shots > 2 }.forEach { println("Big order: " + it.drink) };
}
```

## Features

### Type System

- **Protocols and extensions** — polymorphism with retroactive conformance
- **Monomorphized generics** — zero-cost abstractions with `where` clause constraints
- **Algebraic data types** — enums with associated values and exhaustive pattern matching
- **Type inference** — bidirectional constraint-based inference

### Memory Model

- **Value semantics** — copy-on-assignment, `not Copyable` for move-only types
- **Copy-on-write collections** — Array, Dictionary, Set
- **RAII** — deterministic cleanup via `deinit`
- **No GC, no borrow checker** — ownership is simple and predictable

### Ergonomics

- **Error handling** — `throws` / `try` sugar over `Result[T, E]`
- **Closures** — trailing closure syntax, implicit `it` parameter
- **String interpolation** — `"\(expr)"` via the Formattable protocol
- **Iterators** — `for`-`in` loops with 20+ adapters (map, filter, zip, scan, take, ...)
- **C interop** — `@extern(.C)` for calling C functions and linking native libraries
- **Parameter labels** — named parameters for readable call sites

## Ecosystem

Every tool below is written in Kestrel:

| Tool | Description |
| --- | --- |
| [**Flock**](lang/flock) | Package manager — dependency resolution, registry, lock files |
| [**Jessup**](lang/jessup) | Toolchain version manager (like rustup) |
| [**Perch**](lang/perch) | Web framework — routing, middleware, generic context |
| [**Swoop**](lang/swoop) | HTTP/HTTPS client |
| [**Clutch**](lang/clutch) | CLI argument parser |
| [**Quill**](lang/quill) | Serialization framework |
| [**Quill JSON**](lang/quill-json) | JSON support for Quill |
| [**Quill TOML**](lang/quill-toml) | TOML support for Quill |
| [**HTTP**](lang/http) | Shared HTTP types |
| [**Plume**](lang/plume) | Template engine |

## Examples

| Example | Description | Complexity |
| --- | --- | --- |
| [Weather Dashboard](examples/weather) | Full-stack web app with Perch, htmx, and Open-Meteo API | Advanced |
| [Pokédex](examples/pokedex) | Kanto Pokédex using PokéAPI, Perch, and Plume | Advanced |
| [Wordle](examples/wordle) | Wordle clone with shareable URL state | Intermediate |
| [APOD](examples/apod) | NASA Astronomy Picture of the Day viewer | Intermediate |
| [Counter](examples/counter) | HTMX counter app with Perch | Beginner |
| [Game of Life](examples/life) | Conway's Game of Life with SDL2 | Intermediate |
| [Breakout](examples/breakout) | Terminal brick breaker with Iterator-based game loop | Intermediate |
| [Snake](examples/snake) | Terminal snake with RAII terminal management | Intermediate |
| [Pong](examples/pong) | Terminal pong with AI opponent | Intermediate |
| [SDL Pong](examples/sdl_pong) | Graphical pong via SDL2 FFI bindings | Intermediate |

## Editor Support

Kestrel ships with a language server (`kestrel-lsp`) providing:

- Diagnostics
- Go-to-definition
- Hover info
- Completions
- Signature help
- Rename
- Code actions
- Document symbols
- Semantic highlighting
- Inlay hints

The [VS Code extension](https://github.com/kestrellang/vscode-kestrel) picks up `kestrel-lsp` from PATH automatically, or configure a custom path via `kestrel.lsp.path`.

```bash
# Installed with the toolchain
jessup install stable
kestrel-lsp --version
```

## Standard Library

All public stdlib types are auto-imported — no `import` statements needed.

| Module | Contents |
| --- | --- |
| **core** | Protocols (Equatable, Comparable, Hashable, Cloneable, Formattable), Bool, Range |
| **collections** | Array, Dictionary, Set with copy-on-write semantics |
| **result** | Optional (`T?`) and Result types with `try` operator |
| **text** | String with Unicode support |
| **io** | File I/O, stdin/stdout |
| **net** | Sockets, networking |
| **memory** | Allocator, Buffer, Pointer, reference counting |
| **iter** | Iterator protocol and 20+ adapters |
| **numeric** | Int8–64, UInt8–64, Float32/64 |
| **os** | Environment, process |
| **ffi** | C interop utilities |

## Building from Source

Requires Rust 2024 edition (1.85+).

```bash
git clone https://github.com/kestrellang/kestrel
cd kestrel
cargo install --path .
```

## License

Apache-2.0 — see [LICENSE](LICENSE).
