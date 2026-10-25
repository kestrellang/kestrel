# Entry Points

An executable Kestrel program starts at the one function marked with the `@main` attribute. The attribute — not the function's name — is what makes it the entry point, and the function's return type determines the process exit code: return `()` for a plain success, an `ExitCode` or integer for an explicit status, or use a throwing `main` and let errors print themselves and exit non-zero. Anything that conforms to the `Exitable` protocol works, including your own types.

## The `@main` Attribute

```kestrel
@main
func main() {
    println("Hello!");
}
```

Rules:

- `@main` must sit on a **free (module-level) function** — not a method or static member (E615). The name does not have to be `main`; a bare `func main()` without the attribute is *not* an entry point.
- An executable build requires **exactly one** `@main`: none is E618, more than one is E617.
- Library builds don't need a `@main`; but a malformed one (wrong position or return type) is an error in any build.
- The function takes no parameters. Command-line argument access is separate library functionality, not `main` parameters.

## Return Types and Exit Codes

`@main` may return `()`, `!` (Never), or any type conforming to `Exitable` (E616 otherwise). The returned value's `report()` produces the process exit code.

| Return type | Exit code |
|---|---|
| `()` (or no return type) | 0 |
| `!` (never returns) | — (the program doesn't exit normally) |
| `ExitCode` | the wrapped code |
| `Int8`–`Int64`, `UInt8`–`UInt64` | the value (low 8 bits are what survives on POSIX) |
| `Result[T, E]` (throwing main) | `T`'s exit code on `.Ok`; prints the error and exits 1 on `.Err` |
| Your own `Exitable` type | whatever its `report()` returns |

### ExitCode

```kestrel
@main
func main() -> ExitCode {
    if somethingWrong() {
        return ExitCode.failure;   // 1
    }
    ExitCode.success               // 0
}
```

`ExitCode` lives in `std.os` (available without an import). Construct one from any `UInt8`: `ExitCode(3)`. The meaningful range is 0–255.

### Integer Returns

```kestrel
@main
func main() -> Int64 {
    42
}
// $ ./program; echo $?   →   42
```

All the standard integer types conform to `Exitable`. Values are truncated to the 8 bits the OS actually reports.

## Throwing Main

Declare `main` with `throws` (the return type must be written explicitly — `-> () throws E`):

```kestrel
struct AppError {
    var message: String;
}

extend AppError: Formattable {
    public func format(mutating into writer: some Formatter, options: FormatOptions) {
        writer.append(self.message);
    }
}

@main
func main() -> () throws AppError {
    println("starting");
    throw AppError(message: "disk on fire");
}
```

Running this prints `starting` to stdout, `disk on fire` to **stderr**, and exits with code 1. The mechanics: `-> () throws E` is `Result[(), E]`, and `Result[T, E]` conforms to `Exitable` whenever `T: Exitable` and `E: Formattable` — on `.Ok` it reports the success value's code, on `.Err` it prints the error via `eprintln` and reports failure. So the error type must be printable (`Formattable`), and any `Exitable` success type works: `-> Int32 throws AppError` exits with the `Int32` on success.

`throw`, `try`, and everything else from [Error Handling](error-handling.md) work in `main`'s body as in any throwing function.

## Custom Exitable Types

`Exitable` is an ordinary protocol:

```kestrel
public protocol Exitable {
    consuming func report() -> ExitCode
}
```

Conform your own status type and return it from `main`:

```kestrel
struct Status: Exitable {
    var code: UInt8;
    consuming func report() -> ExitCode { ExitCode(self.code) }
}

@main
func main() -> Status {
    Status(code: 3)   // exits with 3
}
```

`report()` is `consuming` so move-only types (like `Result` carrying a non-copyable payload) can report by moving out of themselves.

## Diagnostics You May Hit

| Code | Meaning |
|---|---|
| E615 | `@main` on something that isn't a free (module-level) function |
| E616 | `@main` return type isn't `()`, `!`, or `Exitable`-conforming |
| E617 | More than one `@main` in the build |
| E618 | Executable build with no `@main` anywhere |

## See Also

- [Error Handling](error-handling.md) — `throws`, `throw`, `try`, and `Result`
- [Functions](functions.md) — free functions and declaration syntax
- [Protocols](protocols.md) — conforming your own types to `Exitable`
- [String Interpolation](string-interpolation.md) — `Formattable`, which throwing main's error type must implement
