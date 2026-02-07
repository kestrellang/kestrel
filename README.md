# Kestrel

A statically-typed programming language with Swift-inspired syntax, protocol-based polymorphism, and value semantics.

## What It Does

Games built with Kestrel:

<p>
<img src="site/public/pong.gif" alt="Pong game written in Kestrel" width="400">
<img src="site/public/snake.gif" alt="Snake game written in Kestrel" width="400">
<img src="site/public/breakout.gif" alt="Breakout game written in Kestrel" width="400">
</p>

## Features

- **Value semantics by default** - Copy-on-assignment with explicit `not Copyable` for move-only types
- **Protocol-based polymorphism** - Interfaces through protocols with extensions and retroactive conformance
- **Monomorphized generics** - Zero-cost abstractions through compile-time specialization
- **RAII resource management** - Deterministic cleanup via `deinit` blocks
- **First-class functions** - Closures with trailing closure syntax and implicit `it` parameter
- **Pattern matching** - `match`, `if let`, `while let`, range patterns, and array patterns with exhaustiveness checking
- **Type inference** - Bidirectional type inference within function bodies
- **Parameter labels** - Named parameters with default values for readable call sites
- **String interpolation** - `"Hello, \(name)"` with format specifiers
- **Error handling** - `try`/`throw` expressions with `Result` and `Optional` types
- **Type sugar** - `T?` for Optional, `T!` for Result, `[T]` for Array, `[K: V]` for Dictionary
- **For-in loops** - `for item in collection { ... }` with iterator protocol
- **Expression-bodied functions** - `func double(x: Int) -> Int = x * 2`

## Quick Start

```bash
# Build the compiler
cargo build --release

# Run a program
kestrel run examples/pong/pong.ks

# Check for errors without running
kestrel check file.ks

# Build an executable
kestrel build file.ks -o output
```

## Hello World

```kestrel
module Hello

import std.io.stdio.(println)
import std.io.error.(Error)

func main() -> () throws Error {
    let name = "World";
    println("Hello, \(name)!");
}
```

## Language Overview

### Structs and Protocols

```kestrel
protocol Bakeable {
    func bakeTime() -> Int
    func isGoldenBrown(minutes: Int) -> Bool
}

struct Cookie : Bakeable {
    let flavor: String
    let chips: Int

    func bakeTime() -> Int { 12 }

    func isGoldenBrown(minutes: Int) -> Bool {
        minutes >= self.bakeTime()
    }
}
```

### Generics

```kestrel
struct Box[T] {
    var value: T
}

func identity[T](value: T) -> T { value }

func process[T](item: T) where T: Bakeable {
    let time = item.bakeTime();
}
```

### Enums and Pattern Matching

```kestrel
enum Option[T] {
    case Some(T)
    case None
}

func unwrap[T](opt: Option[T], default: T) -> T {
    match opt {
        .Some(value) => value,
        .None => default
    }
}
```

### Closures

```kestrel
let double = { it * 2 }
let add = { x, y in x + y }

// Trailing closure syntax
numbers.map { it * 2 }
```

### For Loops and Iterators

```kestrel
for item in items {
    println("\(item)");
}

// Iterator adapters
let evens = numbers.iter()
    .filter { it % 2 == 0 }
    .map { it * 10 }
    .collect();
```

### Error Handling

```kestrel
func readConfig(path: String) -> String throws Error {
    let file = try File.open(path);
    let contents = try file.readToString();
    contents
}

// Null coalescing
let name = optionalName ?? "default";
```

### Type Sugar

```kestrel
var name: String? = nil;      // Optional[String]
var ids: [Int64] = [];         // Array[Int64]
var scores: [String: Int64];   // Dictionary[String, Int64]
```

### Expression-Bodied Functions

```kestrel
func double(x: Int64) -> Int64 = x * 2
func isEven(n: Int64) -> Bool = n % 2 == 0
```

### Parameter Access Modes

```kestrel
// borrowing (default) - read-only reference
func read(point: Point) -> Int { point.x }

// mutating - mutable reference
mutating func reset(point: Point) { point.x = 0; }

// consuming - takes ownership
consuming func destroy(point: Point) { /* point is moved */ }
```

### Extensions

```kestrel
extend Int {
    func squared() -> Int {
        self * self
    }
}

// Retroactive conformance
extend ExternalType : MyProtocol {
    func requiredMethod() { }
}
```

## CLI Reference

```bash
kestrel <command> [options] <files>

Commands:
  check   Type-check without compiling
  run     Compile and run a program
  build   Compile to an executable

Options:
  --tree           Show semantic tree (use --tree=full for details)
  --symbols        Show symbol table
  --xgraph         Show execution graph (MIR)
  --no-std         Disable standard library
  --std <PATH>     Custom standard library path
  -O, --opt-level  Optimization level (0-2)
  --target         Target triple for cross-compilation
  -v, --verbose    Verbose output
  -o, --output     Output file path (build only)
  -l, --link       Link with library (build/run)
  -L               Library search path (build/run)
```

## Project Structure

```
lib/
  kestrel-lexer/                  # Tokenization (Logos)
  kestrel-parser/                 # Parsing (Chumsky)
  kestrel-syntax-tree/            # Concrete Syntax Tree (Rowan)
  kestrel-semantic-tree/          # Symbol definitions
  kestrel-semantic-model/         # Query system
  kestrel-semantic-tree-builder/  # BUILD phase
  kestrel-semantic-tree-binder/   # BIND phase
  kestrel-semantic-analyzers/     # VALIDATE phase
  kestrel-execution-graph/        # MIR representation
  kestrel-codegen-cranelift/      # Code generation (Cranelift)
lang/
  std/                            # Standard library
examples/                         # Example programs
docs/                             # Documentation
```

## Standard Library

The standard library (`lang/std/`) includes:

- **core/** - Protocols for operators, comparison, copying, error handling
- **collections/** - Array, Dictionary (hash map), Set
- **result/** - Optional and Result types with promotion
- **text/** - String (UTF-8), Char (grapheme), Unicode tables, string formatting
- **io/** - File I/O, stdin/stdout, buffered readers/writers
- **memory/** - Allocator, Buffer, Pointer, reference counting (RcBox)
- **iter/** - Iterator protocol and adapters (map, filter, flatMap, zip, chain, enumerate, etc.)
- **num/** - Numeric types (Int8-64, UInt8-64, Float32/64), math functions, random
- **ffi/** - CString, libc bindings for C interop

## Building from Source

Requirements:
- Rust 2024 edition (1.85+)
- Cargo

```bash
git clone https://github.com/jkpdino/kestrel
cd kestrel
cargo build --release
```

Run tests:

```bash
cargo test
```

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
