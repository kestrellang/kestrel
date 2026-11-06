// Standard input, output and error, and the print and read functions.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.collections.(Array, Slice)
import std.text.(String, Formattable)

// ============================================================================
// STANDARD INPUT
// ============================================================================

/// The process's standard input. Every value reads through one shared
/// buffer, so lines read through one value aren't lost to another.
public struct StandardInput: Readable {
    /// @name Default
    public init() {}

    /// Reads from the shared buffer, refilling it from fd 0.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_stdin_read(buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// The next line without its `"\n"` (or `"\r\n"`); `None` at end of
    /// input. Invalid UTF-8 becomes U+FFFD.
    public mutating func readLine() -> Result[Optional[String], IoError] {
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
        if bytes.count > 0 and unsafe bytes(unchecked: bytes.count - 1) == 13 {
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
public struct StandardOutput: Writable {
    /// @name Default
    public init() {}

    /// Adds `buf` to the shared buffer.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_stdout_write(buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Writes the shared buffer to fd 1.
    public mutating func flush() -> Result[(), IoError] {
        flushStandardOutput()
    }
}

/// The process's standard error. Unbuffered, so diagnostics appear at once.
public struct StandardError: Writable {
    /// @name Default
    public init() {}

    /// Writes `buf` to fd 2.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_stream_write(2, buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Nothing to flush.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }
}

/// Writes standard output's shared buffer to fd 1. Other std modules call
/// it before anything else writes to fd 1, such as a child process.
internal func flushStandardOutput() -> Result[(), IoError] {
    if unsafe native_stdout_flush() < 0 {
        return .Err(IoError.last())
    }
    .Ok(())
}

// ============================================================================
// ACCESSORS
// ============================================================================

/// The process's standard input.
public func standardInput() -> StandardInput { StandardInput() }

/// The process's standard output.
public func standardOutput() -> StandardOutput { StandardOutput() }

/// The process's standard error.
public func standardError() -> StandardError { StandardError() }

// ============================================================================
// PRINTING AND READING
// ============================================================================

/// Writes `value` to standard output.
public func print(value: some Formattable) -> Result[(), IoError] {
    var out = StandardOutput();
    out.write(formatted: value)
}

/// Writes `value` and a `"\n"` to standard output.
public func println(value: some Formattable) -> Result[(), IoError] {
    var out = StandardOutput();
    try out.write(formatted: value);
    out.write(byte: 10)
}

/// Writes a `"\n"` to standard output.
public func println() -> Result[(), IoError] {
    var out = StandardOutput();
    out.write(byte: 10)
}

/// Writes `value` to standard error.
public func eprint(value: some Formattable) -> Result[(), IoError] {
    var err = StandardError();
    err.write(formatted: value)
}

/// Writes `value` and a `"\n"` to standard error.
public func eprintln(value: some Formattable) -> Result[(), IoError] {
    var err = StandardError();
    try err.write(formatted: value);
    err.write(byte: 10)
}

/// Reads a line from standard input. `None` at end of input.
public func readLine() -> Result[Optional[String], IoError] {
    var input = StandardInput();
    input.readLine()
}

/// Writes `message`, flushes standard output so it shows before the user
/// types, then reads a line.
public func prompt(message: String) -> Result[Optional[String], IoError] {
    var out = StandardOutput();
    try out.write(text: message);
    try out.flush();
    readLine()
}
