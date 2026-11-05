// Standard input, output and error, and the print and read functions.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.collections.(Array, Slice)
import std.text.(String, Formattable)
import std.io.error.(IoError)

// ============================================================================
// STANDARD INPUT
// ============================================================================

/// The process's standard input. Every value reads through one shared
/// buffer, so lines read through one value aren't lost to another.
internal struct StandardInput: Readable {
    /// @name Default
    internal init() {}

    /// Reads from the shared buffer, refilling it from fd 0.
    internal mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = native_stdin_read(buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// The next line without its `"\n"` (or `"\r\n"`); `None` at end of
    /// input. Invalid UTF-8 becomes U+FFFD.
    internal mutating func readLine() -> Result[Optional[String], IoError] {
        var bytes = Array[UInt8]();
        var sawAny = false;
        while let .Some(b) = try self.readByte() {
            sawAny = true;
            if b == 10 {
                break
            }
            bytes.append(b)
        }
        if not sawAny {
            return .Ok(.None)
        }
        if bytes.count > 0 and bytes(unchecked: bytes.count - 1) == 13 {
            bytes.pop();
        }
        .Ok(.Some(String(fromUtf8Lossy: bytes.asSlice())))
    }
}

// ============================================================================
// STANDARD OUTPUT AND ERROR
// ============================================================================

/// The process's standard output: line-buffered on a terminal,
/// block-buffered otherwise. The buffer is shared by every value and
/// flushed when the process exits normally.
internal struct StandardOutput: Writable {
    /// @name Default
    internal init() {}

    /// Adds `buf` to the shared buffer.
    internal mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = native_stdout_write(buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Writes the shared buffer to fd 1.
    internal mutating func flush() -> Result[(), IoError] {
        flushStandardOutput()
    }
}

/// The process's standard error. Unbuffered, so diagnostics appear at once.
internal struct StandardError: Writable {
    /// @name Default
    internal init() {}

    /// Writes `buf` to fd 2.
    internal mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = native_stream_write(2, buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Nothing to flush.
    internal mutating func flush() -> Result[(), IoError] { .Ok(()) }
}

/// Writes standard output's shared buffer to fd 1. Other std modules call
/// it before anything else writes to fd 1, such as a child process.
internal func flushStandardOutput() -> Result[(), IoError] {
    if native_stdout_flush() < 0 {
        return .Err(IoError.last())
    }
    .Ok(())
}

// ============================================================================
// ACCESSORS
// ============================================================================

/// The process's standard input.
internal func standardInput() -> StandardInput { StandardInput() }

/// The process's standard output.
internal func standardOutput() -> StandardOutput { StandardOutput() }

/// The process's standard error.
internal func standardError() -> StandardError { StandardError() }

// ============================================================================
// PRINTING AND READING
// ============================================================================

/// Writes `value` to standard output.
internal func print(value: some Formattable) -> Result[(), IoError] {
    var out = StandardOutput();
    out.write(formatted: value)
}

/// Writes `value` and a `"\n"` to standard output.
internal func println(value: some Formattable) -> Result[(), IoError] {
    var out = StandardOutput();
    try out.write(formatted: value);
    out.write(byte: 10)
}

/// Writes a `"\n"` to standard output.
internal func println() -> Result[(), IoError] {
    var out = StandardOutput();
    out.write(byte: 10)
}

/// Writes `value` to standard error.
internal func eprint(value: some Formattable) -> Result[(), IoError] {
    var err = StandardError();
    err.write(formatted: value)
}

/// Writes `value` and a `"\n"` to standard error.
internal func eprintln(value: some Formattable) -> Result[(), IoError] {
    var err = StandardError();
    try err.write(formatted: value);
    err.write(byte: 10)
}

/// Reads a line from standard input. `None` at end of input.
internal func readLine() -> Result[Optional[String], IoError] {
    var input = StandardInput();
    input.readLine()
}

/// Writes `message`, flushes standard output so it shows before the user
/// types, then reads a line.
internal func prompt(message: String) -> Result[Optional[String], IoError] {
    var out = StandardOutput();
    try out.write(text: message);
    try out.flush();
    readLine()
}
