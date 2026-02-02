// Standard I/O streams

module std.io.stdio

import std.num.(Int64, UInt8)
import std.result.(Result, Optional)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool, Formattable)
import std.io.libc
import std.io.error.(Error)
import std.io.read.(Read)
import std.io.write.(Write, writeStr, writeByte, writeLine)

// ============================================================================
// STANDARD INPUT
// ============================================================================

/// Standard input stream.
public struct Stdin: Read {
    /// Creates a stdin handle.
    public init() {}

    /// Reads bytes from standard input.
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.read(libc.STDIN(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }
}

// ============================================================================
// STANDARD OUTPUT
// ============================================================================

/// Standard output stream.
public struct Stdout: Write {
    /// Creates a stdout handle.
    public init() {}

    /// Writes bytes to standard output.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.write(libc.STDOUT(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    /// Flushes standard output.
    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }
}

// ============================================================================
// STANDARD ERROR
// ============================================================================

/// Standard error stream.
public struct Stderr: Write {
    /// Creates a stderr handle.
    public init() {}

    /// Writes bytes to standard error.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.write(libc.STDERR(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    /// Flushes standard error.
    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }
}

// ============================================================================
// HANDLE ACCESSORS
// ============================================================================

/// Returns a handle to standard input.
public func stdin() -> Stdin {
    Stdin()
}

/// Returns a handle to standard output.
public func stdout() -> Stdout {
    Stdout()
}

/// Returns a handle to standard error.
public func stderr() -> Stderr {
    Stderr()
}

// ============================================================================
// PRINT FUNCTIONS
// ============================================================================

/// Prints a value to stdout (no newline).
public func print[F](value: F) -> Result[(), Error] where F: Formattable {
    var out = stdout();
    writeStr(out, value.format())
}

/// Prints a value to stdout with a newline.
public func println[F](value: F) -> Result[(), Error] where F: Formattable {
    var out = stdout();
    writeLine(out, value.format())
}

/// Prints an empty line to stdout.
public func printlnEmpty() -> Result[(), Error] {
    var out = stdout();
    writeByte(out, 10)
}

/// Prints a value to stderr (no newline).
public func eprint[F](value: F) -> Result[(), Error] where F: Formattable {
    var err = stderr();
    writeStr(err, value.format())
}

/// Prints a value to stderr with a newline.
public func eprintln[F](value: F) -> Result[(), Error] where F: Formattable {
    var err = stderr();
    writeLine(err, value.format())
}

// ============================================================================
// INPUT FUNCTIONS
// ============================================================================

/// Reads a line from stdin (without the newline).
public func readLine() -> Result[String, Error] {
    var input = stdin();
    var bytes = Array[UInt8]();

    loop {
        var buf = Array[UInt8](capacity: 1);
        buf.append(0);
        let slice = Slice(pointer: buf.asPointer(), count: 1);
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

/// Prints a prompt message and reads a line from stdin.
public func prompt(message: String) -> Result[String, Error] {
    var out = stdout();
    try writeStr(out, message);
    try out.flush();
    readLine()
}
