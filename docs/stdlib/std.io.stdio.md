# std.io.stdio

## struct `Stderr`

```kestrel
public struct Stderr { /* private fields */ }
```

`Writable` over the process's standard error (file descriptor `2`).

Mirrors `Stdout` but writes to `STDERR_FILENO`. Conventionally used
for diagnostics, log lines, and anything that should not be captured
by a downstream pipe consuming `stdout`.

### Representation

Zero-sized.

_Defined in `lang/std/io/stdio.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds a stderr handle.

_Defined in `lang/std/io/stdio.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op; stderr is unbuffered at this layer.

_Defined in `lang/std/io/stdio.ks`._

#### function `write`

```kestrel
public mutating func write(from: ArraySlice[UInt8]) -> Result[Int64, IoError]
```

Calls `write(2)` on `STDERR_FILENO`.

_Defined in `lang/std/io/stdio.ks`._

## struct `Stdin`

```kestrel
public struct Stdin { /* private fields */ }
```

`Readable` over the process's standard input (file descriptor `0`).

Construct via `Stdin()` or the `stdin()` accessor. Stateless — every
instance shares the same descriptor; concurrent readers race on the
same pipe.

### Representation

Zero-sized — operations dispatch directly on `libc.STDIN()`.

_Defined in `lang/std/io/stdio.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds a stdin handle.

_Defined in `lang/std/io/stdio.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: ArraySlice[UInt8]) -> Result[Int64, IoError]
```

Calls `read(2)` on `STDIN_FILENO`. Returns `0` on EOF (e.g. after
the user types Ctrl-D in a terminal).

_Defined in `lang/std/io/stdio.ks`._

## struct `Stdout`

```kestrel
public struct Stdout { /* private fields */ }
```

`Writable` over the process's standard output (file descriptor `1`).

As with `Stdin`, stateless — `flush` is a no-op because writes go
straight to libc; line buffering / TTY behaviour is handled by libc
or the terminal.

### Representation

Zero-sized.

_Defined in `lang/std/io/stdio.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds a stdout handle.

_Defined in `lang/std/io/stdio.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op; stdout does no internal buffering at this layer.

_Defined in `lang/std/io/stdio.ks`._

#### function `write`

```kestrel
public mutating func write(from: ArraySlice[UInt8]) -> Result[Int64, IoError]
```

Calls `write(2)` on `STDOUT_FILENO`.

_Defined in `lang/std/io/stdio.ks`._

## function `eprint`

```kestrel
public func eprint[F](F) -> Result[(), IoError] where F: Formattable
```

Stderr counterpart to `print`. Useful for diagnostics that must not
pollute a piped stdout.

_Defined in `lang/std/io/stdio.ks`._

## function `eprintln`

```kestrel
public func eprintln[F](F) -> Result[(), IoError] where F: Formattable
```

Stderr counterpart to `println`.

_Defined in `lang/std/io/stdio.ks`._

## function `print`

```kestrel
public func print[F](F) -> Result[(), IoError] where F: Formattable
```

Formats `value` with its default `FormatOptions` and writes the
result to stdout. No trailing newline.

### Examples

```
try print("count: ");
try println(42);
```

_Defined in `lang/std/io/stdio.ks`._

## function `println`

```kestrel
public func println[F](F) -> Result[(), IoError] where F: Formattable
```

Like `print`, plus a trailing `\n`.

_Defined in `lang/std/io/stdio.ks`._

## function `printlnEmpty`

```kestrel
public func printlnEmpty() -> Result[(), IoError]
```

Writes a single newline to stdout — the no-argument form of `println`.

_Defined in `lang/std/io/stdio.ks`._

## function `prompt`

```kestrel
public func prompt(String) -> Result[String, IoError]
```

Writes `message` to stdout, flushes, then reads a line from stdin.
The flush matters for line-buffered terminals — without it the
prompt would appear after the user's keystrokes.

### Examples

```
let name = try prompt("Name: ");
try println("Hello, " + name);
```

_Defined in `lang/std/io/stdio.ks`._

## function `readLine`

```kestrel
public func readLine() -> Result[String, IoError]
```

Reads a single line from stdin, stripping the trailing `\n` (and
`\r` if present, for tolerance with Windows-style line endings).
Returns an empty string on immediate EOF.

TODO: the trailing-bytes are collected but the returned `String` is
currently empty — see the comment in the body about
`String.fromUtf8Bytes`.

_Defined in `lang/std/io/stdio.ks`._

## function `stderr`

```kestrel
public func stderr() -> Stderr
```

Convenience constructor — equivalent to `Stderr()`.

_Defined in `lang/std/io/stdio.ks`._

## function `stdin`

```kestrel
public func stdin() -> Stdin
```

Convenience constructor — equivalent to `Stdin()`.

_Defined in `lang/std/io/stdio.ks`._

## function `stdout`

```kestrel
public func stdout() -> Stdout
```

Convenience constructor — equivalent to `Stdout()`.

_Defined in `lang/std/io/stdio.ks`._

