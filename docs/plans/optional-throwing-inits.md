# Optional and Throwing Constructors

**Status**: Design
**Issue**: [#28](https://github.com/kestrellang/kestrel/issues/28)
**Target**: 0.16

## Summary

Allow initializers to declare failable or throwing effects, using the same `?` and `throws E` type operators that apply to regular types:

```kestrel
struct ParsedInt {
    var value: Int64

    init(from source: String)? {
        guard let v = Int64(from: source) else {
            return nil
        }
        self.value = v
    }
}

struct TcpStream {
    var fd: Int32

    init(host: String, port: UInt16) throws IoError {
        self.fd = try connectSocket(host, port)
    }
}
```

Call sites look identical to regular init calls — the result type changes based on the init's declared effect:

```kestrel
let a = ParsedInt(from: "42")           // a: Optional[ParsedInt]
let b = try ParsedInt(from: "42")       // b: ParsedInt (or early return)

let c = TcpStream(host: h, port: 80)        // c: Result[TcpStream, IoError]
let d = try TcpStream(host: h, port: 80)    // d: TcpStream (or early return)
```

## Motivation

Today, initializers that can fail must be written as static factory methods returning `Optional[T]` or `Result[T, E]`:

```kestrel
// Current workaround (net/socket.ks)
public static func connect(host: String, port: UInt16) -> Result[TcpStream, IoError] {
    // ... error handling ...
    .Ok(TcpStream(fd))
}
```

This breaks the `Type(args)` construction idiom and forces callers to know that construction lives behind a different API shape. Failable and throwing inits restore uniform construction syntax while expressing failure in the type system.

## Syntax

The effect modifier appears after the parameter list, before the body — the same position where `throws E` appears on functions:

```
init(<params>)? { <body> }
init(<params>) throws <ErrorType> { <body> }
```

These are **mutually exclusive** — an init is regular, failable, or throwing, never a combination.

The `?` and `throws E` here are the same type operators used in types like `Int64?` and `Int64 throws IoError`. They are not new syntax — they are applied to the init's implicit `()` return type.

### Grammar

```
initializer_decl = attribute* visibility? 'init' type_params? '(' params ')' init_effect? where_clause? block
init_effect      = '?' | 'throws' type
```

## Semantics

### Return type model

An init's body implicitly returns `()` — the receiver `self` is the constructed value, not an explicit return. The effect modifier wraps this implicit return through the type operator:

| Declaration | Body return type | Call-site result type |
|---|---|---|
| `init(x: Int64)` | `()` | `Self` |
| `init(x: Int64)?` | `()?` | `Self?` |
| `init(x: Int64) throws E` | `() throws E` | `Self throws E` |

The `?` and `throws E` are not hardcoded to `Optional` and `Result` — they flow through the same type operators used everywhere else.

### Body semantics

**Failable init (`init()?`):**
- `return nil` — fail, the init produces no value
- `return` (bare) — succeed early, self must be fully initialized
- Falling off the end — succeed, self must be fully initialized

**Throwing init (`init() throws E`):**
- `throw error` — fail with the given error
- `try expr` — propagate failure from a throwing sub-expression
- `return` (bare) — succeed early, self must be fully initialized
- Falling off the end — succeed, self must be fully initialized

### Definite initialization

Self must be fully initialized on every **success** path. Failure paths (`return nil`, `throw`, `try` propagation) may leave self partially initialized.

### Partial drop on failure

When an init fails after some fields have been assigned, the compiler must drop exactly the fields that were initialized. The definite-initialization analysis already tracks which fields are assigned at each program point — this information drives emission of partial-drop sequences on failure paths.

Example:

```kestrel
init()? {
    self.handle = acquireResource()   // handle is now live
    guard let v = riskyOperation() else {
        return nil   // handle is dropped; value is not (never assigned)
    }
    self.value = v
}
```

### Init delegation

A failable or throwing init may delegate to another init via `self.init(...)`. The delegated init's effect must be handled:

```kestrel
init(x: Int64)? {
    guard let _ = self.init(raw: x) else {  // delegate to init(raw:)?
        return nil
    }
}

init(host: String) throws IoError {
    try self.init(host: host, port: 443)   // delegate to init(host:port:) throws IoError
}
```

The delegated call returns the body return type (`()?` or `() throws E`), which can be unwrapped with `guard let` / `try` as usual.

### Overloading

Failability and throwing are **not** part of overload resolution. You cannot have both `init(x: Int64)` and `init(x: Int64)?` — same labels means same overload, conflict.

### Protocol conformance (widening)

A non-failable init satisfies a failable protocol requirement. A non-throwing init satisfies a throwing protocol requirement. An init that always succeeds is strictly more capable than one that might fail — no wrapping needed at the conformance boundary.

The reverse is not true: a failable init cannot satisfy a non-failable requirement.

| Requirement | `init()` conforms? | `init()?` conforms? | `init() throws E` conforms? |
|---|---|---|---|
| `init()` | Yes | No | No |
| `init()?` | Yes (widening) | Yes | No |
| `init() throws E` | Yes (widening) | No | Yes |

### Memberwise inits

The auto-generated memberwise init is always non-failable, non-throwing. It coexists with explicit failable/throwing inits since they have different signatures.

## Pipeline Trace

| Stage | What happens | Changes needed |
|-------|-------------|----------------|
| **Parser** | Parse `init`, params, then optional `?` or `throws Type` after `)` | Add `init_effect` to `InitializerDeclarationData` |
| **AST builder** | Store init effect as a component on the init entity | New `InitEffect` component (`None`, `Optional`, `Throwing(TypeId)`) |
| **HIR lowering** | Set body return type to `()?` or `() throws E`; wrap implicit success return | Insert `.Some(())` / `.Ok(())` wrapping on success paths |
| **Type inference** | When resolving a struct call to a failable/throwing init, result type is `Self?` / `Self throws E` instead of `Self` | Modify init-call constraint generation in `generate.rs` and `solver.rs` |
| **Definite init** | Track field assignments per path; only require completeness on success paths | Extend analysis to distinguish success/failure paths |
| **MIR lowering** | Emit partial-drop sequences on failure paths using definite-init info | New codegen for field-wise drop on init failure |
| **Codegen** | Init function signature uses wrapped return type | Return type is `()?` or `() throws E` at the ABI level |

## Diagnostics

| ID | Name | Description |
|----|------|-------------|
| New | `return_nil_in_non_failable_init` | `return nil` used in a non-failable init |
| New | `throw_in_non_throwing_init` | `throw` used in a non-throwing init |
| New | `failable_and_throwing_init` | Init declared with both `?` and `throws` (mutually exclusive) |
| Existing | Definite init errors | Extended to report "field X not initialized on success path" |

## Examples

### Failable init

```kestrel
struct Color {
    var r: UInt8
    var g: UInt8
    var b: UInt8

    init(hex: String)? {
        guard hex.count == 7, hex.first == "#" else {
            return nil
        }
        guard let r = UInt8(from: hex.substring(1, 3)),
              let g = UInt8(from: hex.substring(3, 5)),
              let b = UInt8(from: hex.substring(5, 7)) else {
            return nil
        }
        self.r = r
        self.g = g
        self.b = b
    }
}

let color = Color(hex: "#FF8800")       // Optional[Color]
guard let c = Color(hex: input) else {
    return defaultColor
}
```

### Throwing init

```kestrel
struct Config {
    var port: UInt16
    var host: String

    init(path: String) throws ParseError {
        let contents = try File.read(path: path)
        let parsed = try Json.parse(contents)
        guard let port = parsed.get("port")?.toUInt16() else {
            throw ParseError(message: "missing port")
        }
        guard let host = parsed.get("host")?.toString() else {
            throw ParseError(message: "missing host")
        }
        self.port = port
        self.host = host
    }
}

let config = try Config(path: "config.json")
```

### Protocol with failable init requirement

```kestrel
protocol Parseable {
    init(from source: String)?
}

struct Wrapper[T: Parseable] {
    var inner: T

    init(from source: String)? {
        guard let inner = T(from: source) else {
            return nil
        }
        self.inner = inner
    }
}
```

### Widening — non-failable satisfies failable

```kestrel
protocol Buildable {
    init(default: Bool)?
}

struct Simple: Buildable {
    var flag: Bool

    // Non-failable init satisfies the failable requirement
    init(default flag: Bool) {
        self.flag = flag
    }
}
```

## Resolved Questions

1. **Effect position.** After the parameter list, not on the `init` keyword. `init()?` and `init() throws E` — consistent with where `throws` sits on functions.

2. **No combining `?` and `throws`.** Mutually exclusive. If you need both failure modes, use `throws` — the error type can represent "no value" as one of its cases.

3. **Not hardcoded to Optional/Result.** The `?` and `throws E` operators determine the wrapping type, same as they do for `T?` and `T throws E` everywhere else.

4. **Widening allowed.** A non-failable init satisfies a failable protocol requirement; a non-throwing init satisfies a throwing requirement.

5. **Partial drop on failure.** The compiler tracks field initialization and emits drops for assigned fields when the init fails. Required because Kestrel has drop semantics.

6. **Overload conflict.** Failability is not part of overload identity. `init(x: Int64)` and `init(x: Int64)?` conflict.

## Open Questions

None — ready for implementation.
