# Error Handling

Kestrel errors are ordinary values. A function that can fail returns `Result[T, E]` — usually written with the `T throws E` sugar — and failure travels through the type system rather than through unwinding. Three pieces of syntax make this ergonomic: `throw` produces the failure and returns it in one step, `try` unwraps a success or propagates the failure to the caller, and failable initializers (`init(...)?` / `init(...) throws E`) extend the same model to construction. All of it is driven by ordinary protocols (`Tryable`, `FromResidual`), so `try` works on `Optional`, `Result`, and your own types alike.

## Result[T, E]

`Result` is a two-case enum in the standard library:

```kestrel
public enum Result[T, E] {
    case Ok(T)
    case Err(E)
}
```

The case names are `.Ok` and `.Err`. Handle one with `match`, or use the API surface:

```kestrel
let r: Result[Int64, String] = .Ok(42);

r.isOk();             // true
r.unwrap();           // 42 — panics if Err
r.unwrap(or: 0);      // 42, or the default on Err
r.map({ it * 2 });    // Result[Int64, String] = .Ok(84)
r.mapErr({ (e: String) in "wrapped: \(e)" });
r.ok();               // Optional[Int64] — Ok becomes Some, Err becomes None
r.err();              // Optional[String] — the reverse
```

Also available: `flatMap`/`andThen`, `orElse`, `unwrapErr()`, `unwrap(orElse:)`, and conditional `Equatable`/`Formattable` conformances (a `Result` prints as `Ok(...)`/`Err(...)`).

## Throwing Functions: `-> T throws E`

`T throws E` is type-operator sugar for `Result[T, E]`. A "throwing function" is simply a function whose return type is a `Result`:

```kestrel
enum ParseError { case TooBig }

func checked(n: Int64) -> Int64 throws ParseError {
    if n > 100 { throw ParseError.TooBig; }
    .Ok(n * 2)
}
```

Because the return type really is `Result[Int64, ParseError]`, **success values must be wrapped**: the tail expression (or a `return`) produces `.Ok(value)`. A bare `n * 2` in tail position is a type mismatch. (One exception: an annotated binding promotes a bare success value — `let r: Int64 throws MyError = 42;` produces `.Ok(42)`.)

Callers see a plain `Result` and may `match` it, query it, or `try` it:

```kestrel
match checked(3) {
    .Ok(v)  => println("got \(v)"),
    .Err(e) => println("failed")
};
```

## `throw` Expressions

`throw expr` wraps `expr` as the failure case of the enclosing function's return type and returns it immediately:

```kestrel
func openPort(n: Int64) -> Int64 throws ParseError {
    if n > 100 { throw ParseError.TooBig; }   // returns .Err(.TooBig)
    .Ok(n)
}
```

- `throw` is an expression of type `!` (Never), like `return` — so it composes anywhere an expression fits: `guard cond else { throw E.Bad; }`, match arms, the branches of an `if`.
- It targets the **innermost** function or closure.
- The enclosing return type must know how to absorb the error — formally, it must conform to `FromResidual[E]`. For `Result[T, E]`, `throw e` is equivalent to `return .Err(e)`.
- A bare `throw` with no operand is a parse error, as is `throw` outside a function body.

## The `try` Operator

`try expr` extracts the success value of a fallible expression, or **early-returns the failure** from the enclosing function:

```kestrel
func chain(n: Int64) -> Int64 throws ParseError {
    let v = try checked(n);   // Ok: v is an Int64. Err: chain returns that Err.
    .Ok(v + 1)
}
```

`try` binds tightly: `try foo() + bar()` parses as `(try foo()) + bar()`. Inside a closure, `try` returns from the closure, not the outer function.

`try` also works on `Optional` — `.None` propagates:

```kestrel
func halfOfFirstEven(xs: [Int64]) -> Int64? {
    let e = try firstEven(xs);   // .None here returns .None from halfOfFirstEven
    .Some(e / 2)
}
```

Mixing types under `try` is allowed exactly when the enclosing return type can absorb the inner failure (`FromResidual` again). Trying a `Result[T, E1]` inside a function returning `Result[T, E2]` requires converting the error first (e.g. with `.mapErr`); the compiler reports the missing `fromResidual` conversion otherwise.

### Defaults Instead of Propagation

`try f() ?? default` is **not supported** — `??` is the `Optional`-coalescing operator, and `Result` does not conform to `Coalesce`. To fall back to a default instead of propagating:

```kestrel
let a = checked(200).unwrap(or: -1);   // default on Err
let b = checked(200).ok() ?? -1;       // convert to Optional, then coalesce
```

## The Tryable Protocol

`try` is protocol-driven, so any type can participate:

```kestrel
public protocol Tryable {
    type Output      // the value `try` yields on success
    type Residual    // the failure payload that propagates
    consuming func tryExtract() -> ControlFlow[Output, Residual]
}
```

`try expr` desugars to calling `tryExtract()`: `.Continue(value)` yields the value; `.Break(residual)` early-returns `R.fromResidual(residual)` where `R` is the enclosing return type. Stdlib conformances: `Result[T, E]` (`Output = T`, `Residual = E`) and `Optional[T]` (`Output = T`, `Residual = ()`).

A custom conformance:

```kestrel
enum Checked: Tryable {
    case Valid(Int64)
    case Invalid(String)

    type Output = Int64
    type Residual = String

    consuming func tryExtract() -> ControlFlow[Int64, String] {
        match self {
            .Valid(v) => .Continue(v),
            .Invalid(why) => .Break(why)
        }
    }
}

func describe(n: Int64) -> String throws String {
    let v = try validate(n);   // Checked's String residual feeds the Err
    .Ok("valid: \(v)")
}
```

The receiving side of propagation is `FromResidual`: `Result[T, E]: FromResidual[E]` (produces `.Err`) and `Optional[T]: FromResidual[()]` (produces `.None`).

## Failable and Throwing Initializers

An initializer can carry an effect after its parameter list — `?` for failable, `throws E` for throwing. The two are mutually exclusive.

### `init(...)?`

Calling a failable init on `T` yields `T?`. Inside the body, `return null` fails; completing normally (with all fields initialized) succeeds:

```kestrel
struct Port {
    var number: Int64;

    init(raw n: Int64)? {
        if n < 0 or n > 65535 { return null; }
        self.number = n;
    }
}

match Port(raw: 8080) {
    .Some(p) => println("port \(p.number)"),
    .None    => println("invalid")
};
```

If a failure path has already assigned some fields, the compiler cleans them up — you don't leak partially built values.

### Delegation

A failable init may delegate to another failable init with `self.init(...)`. If the inner init fails, the outer init fails too:

```kestrel
init(doubled n: Int64)? {
    self.init(raw: n * 2);   // inner .None propagates as this init's .None
}
```

### `init(...) throws E`

Calling a throwing init on `T` yields `Result[T, E]`; `throw` and `try` work in the body as in any throwing function:

```kestrel
enum ConfigError { case BadPort }

struct Config {
    var port: Int64;

    init(port p: Int64) throws ConfigError {
        if p < 0 { throw ConfigError.BadPort; }
        self.port = p;
    }
}

match Config(port: -1) {
    .Ok(c)  => println("ok \(c.port)"),
    .Err(_) => println("bad")
};
```

For protocol conformance, effects widen one way: a plain `init` satisfies a failable or throwing requirement, but a failable/throwing init cannot satisfy a plain one (E464).

## Optional and Result Interplay

- `??` — coalescing, via the `Coalesce` protocol. `opt ?? fallback` yields the payload or the fallback; the fallback is **lazy** (evaluated only on `.None`).
- `value!` — force-unwrap; traps on `.None`.
- `opt.okOr(e)` — `Optional[T]` → `Result[T, E]` (with `okOrElse` for a lazy error).
- `res.ok()` / `res.err()` — `Result` → `Optional` in either direction.
- `try` composes across them when the enclosing return type conforms to `FromResidual` of the inner residual: trying an `Optional` works inside `Optional`-returning functions (residual `()` → `.None`).

A throwing `main` is also supported — see [Entry Points](entry-points.md).

## Not Yet Supported

- `try expr ?? throw OtherError` and `Result ?? default` — `Result` has no `Coalesce` conformance; use `.unwrap(or:)` or `.ok() ?? d`.
- `catch { }` blocks and `try await` — listed as future extensions in the design; not implemented.

## Diagnostics You May Hit

| Code | Meaning |
|---|---|
| E464 | Initializer effect mismatch with a protocol requirement (expected non-failable/failable/throwing, found another) |
| E490 | Reference return inside effect sugar (`-> &T throws E` would form `Result[&T, E]`) |
| — | `type mismatch: expected Result[...] got ...` — a bare success value in a throws function; wrap it in `.Ok(...)` |
| — | `does not conform ... Tryable` — `try` applied to a non-Tryable expression |
| — | `.fromResidual not found` / `does not implement FromResidual[...]` — the enclosing return type can't absorb the propagated failure; convert the error (e.g. `.mapErr`) or change the return type |
| — | `found 'throw'` / `expected expression` — `throw` at module level or with no operand (parse errors) |

## See Also

- [Entry Points](entry-points.md) — throwing `main` and exit codes
- [Enums](enums.md) — `Result` and your error types are ordinary enums
- [Pattern Matching](pattern-matching.md) — `match`, `if let`, `guard let` over `.Ok`/`.Err`/`.Some`
- [Protocols](protocols.md) — how `Tryable`/`FromResidual` conformances work
