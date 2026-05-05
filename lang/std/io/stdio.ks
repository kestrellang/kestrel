// Standard I/O streams

module std.io.stdio

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.memory.(ArraySlice, Pointer)
import std.collections.(Array)
import std.text.(String, Formattable)
import std.core.(Bool)
import std.io.libc
import std.io.error.(IoError)
import std.io.read.(Readable)
import std.io.write.(Writable, writeString, writeByte, writeLine)

// ============================================================================
// STANDARD INPUT
// ============================================================================

/// `Readable` over the process's standard input (file descriptor `0`).
///
/// Construct via `Stdin()` or the `stdin()` accessor. Stateless — every
/// instance shares the same descriptor; concurrent readers race on the
/// same pipe.
///
/// # Representation
///
/// Zero-sized — operations dispatch directly on `libc.STDIN()`.
public struct Stdin: Readable {
    /// @name Default
    /// Builds a stdin handle.
    public init() {}

    /// Calls `read(2)` on `STDIN_FILENO`. Returns `0` on EOF (e.g. after
    /// the user types Ctrl-D in a terminal).
    public mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        let n = libc.read(libc.STDIN(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }
}

// ============================================================================
// STANDARD OUTPUT
// ============================================================================

/// `Writable` over the process's standard output (file descriptor `1`).
///
/// As with `Stdin`, stateless — `flush` is a no-op because writes go
/// straight to libc; line buffering / TTY behaviour is handled by libc
/// or the terminal.
///
/// # Representation
///
/// Zero-sized.
public struct Stdout: Writable {
    /// @name Default
    /// Builds a stdout handle.
    public init() {}

    /// Calls `write(2)` on `STDOUT_FILENO`.
    public mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        let n = libc.write(libc.STDOUT(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// No-op; stdout does no internal buffering at this layer.
    public mutating func flush() -> Result[(), IoError] {
        .Ok(())
    }
}

// ============================================================================
// STANDARD ERROR
// ============================================================================

/// `Writable` over the process's standard error (file descriptor `2`).
///
/// Mirrors `Stdout` but writes to `STDERR_FILENO`. Conventionally used
/// for diagnostics, log lines, and anything that should not be captured
/// by a downstream pipe consuming `stdout`.
///
/// # Representation
///
/// Zero-sized.
public struct Stderr: Writable {
    /// @name Default
    /// Builds a stderr handle.
    public init() {}

    /// Calls `write(2)` on `STDERR_FILENO`.
    public mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        let n = libc.write(libc.STDERR(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// No-op; stderr is unbuffered at this layer.
    public mutating func flush() -> Result[(), IoError] {
        .Ok(())
    }
}

// ============================================================================
// HANDLE ACCESSORS
// ============================================================================

/// Convenience constructor — equivalent to `Stdin()`.
public func stdin() -> Stdin {
    Stdin()
}

/// Convenience constructor — equivalent to `Stdout()`.
public func stdout() -> Stdout {
    Stdout()
}

/// Convenience constructor — equivalent to `Stderr()`.
public func stderr() -> Stderr {
    Stderr()
}

// ============================================================================
// PRINT FUNCTIONS
// ============================================================================

/// Formats `value` with its default `FormatOptions` and writes the
/// result to stdout. No trailing newline.
///
/// # Examples
///
/// ```
/// try print("count: ");
/// try println(42);
/// ```
public func print[F](value: F) -> Result[(), IoError] where F: Formattable {
    var out = stdout();
    writeString(out, value.formatted())
}

/// Like `print`, plus a trailing `\n`.
public func println[F](value: F) -> Result[(), IoError] where F: Formattable {
    var out = stdout();
    writeLine(out, value.formatted())
}

/// Writes a single newline to stdout — the no-argument form of `println`.
public func printlnEmpty() -> Result[(), IoError] {
    var out = stdout();
    writeByte(out, 10)
}

/// Stderr counterpart to `print`. Useful for diagnostics that must not
/// pollute a piped stdout.
public func eprint[F](value: F) -> Result[(), IoError] where F: Formattable {
    var err = stderr();
    writeString(err, value.formatted())
}

/// Stderr counterpart to `println`.
public func eprintln[F](value: F) -> Result[(), IoError] where F: Formattable {
    var err = stderr();
    writeLine(err, value.formatted())
}

// ============================================================================
// INPUT FUNCTIONS
// ============================================================================

/// Reads a single line from stdin, stripping the trailing `\n` (and
/// `\r` if present, for tolerance with Windows-style line endings).
/// Returns an empty string on immediate EOF.
///
/// TODO: the trailing-bytes are collected but the returned `String` is
/// currently empty — see the comment in the body about
/// `String.fromUtf8Bytes`.
public func readLine() -> Result[String, IoError] {
    var input = stdin();
    var bytes = Array[UInt8]();

    loop {
        var buf = Array[UInt8](capacity: 1);
        buf.append(0);
        let slice = ArraySlice(pointer: buf.asPointer(), count: 1);
        let n = try input.read(into: slice);
        if n == 0 {
            break  // EOF
        }
        let b = buf(unchecked: 0);
        if b == 10 {  // newline
            break
        }
        bytes.append(b)
    }

    // Strip trailing \r if present (Windows line endings)
    let count = bytes.count;
    if count > 0 {
        let lastByte = bytes(unchecked: count - 1);
        if lastByte == 13 {
            let _ = bytes.pop();
        }
    }

    // Build string from bytes (inefficient but works)
    // TODO: Add proper String.fromUtf8Bytes()
    var result = "";
    .Ok(result)
}

/// Writes `message` to stdout, flushes, then reads a line from stdin.
/// The flush matters for line-buffered terminals — without it the
/// prompt would appear after the user's keystrokes.
///
/// # Examples
///
/// ```
/// let name = try prompt("Name: ");
/// try println("Hello, " + name);
/// ```
public func prompt(message: String) -> Result[String, IoError] {
    var out = stdout();
    try writeString(out, message);
    try out.flush();
    readLine()
}
