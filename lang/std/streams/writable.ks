// The Writable protocol and its extension methods.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result)
import std.collections.(Array, Slice)
import std.text.(String, Formattable)
import std.io.error.(IoError)

// ============================================================================
// WRITABLE
// ============================================================================

/// A destination for bytes.
///
/// One `write(from:)` moves up to `buf.count` bytes and returns how many it
/// moved; a short write is not an error, and `writeAll` loops until
/// everything is written. `flush` pushes out whatever the writer buffered.
///
/// # Examples
///
/// ```
/// struct Counter: Writable {
///     var total: Int64
///     internal mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
///         self.total = self.total + buf.count;
///         .Ok(buf.count)
///     }
///     internal mutating func flush() -> Result[(), IoError] { .Ok(()) }
/// }
/// ```
internal protocol Writable {
    /// Writes up to `buf.count` bytes; returns how many moved.
    mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError]

    /// Pushes buffered bytes to the destination. Unbuffered writers return
    /// `.Ok(())`. An error here can be one deferred from an earlier write.
    mutating func flush() -> Result[(), IoError]
}

extend Writable {
    /// Writes all of `buf`, looping over short writes and retrying writes
    /// interrupted by a signal.
    ///
    /// # Errors
    ///
    /// `BrokenPipe` if the writer accepts zero bytes before `buf` is done.
    internal mutating func writeAll(from buf: Slice[UInt8]) -> Result[(), IoError] {
        var written: Int64 = 0;
        while written < buf.count {
            let n = match self.write(from: buf(written..)) {
                .Ok(n) => n,
                .Err(e) => {
                    if isInterrupted(e) { continue }
                    return .Err(e)
                }
            };
            if n == 0 {
                return .Err(IoError(kind: .BrokenPipe))
            }
            written = written + n
        }
        .Ok(())
    }

    /// Writes one byte.
    internal mutating func write(byte byte: UInt8) -> Result[(), IoError] {
        let one = Array[UInt8](repeating: byte, count: 1);
        self.writeAll(from: one.asSlice())
    }

    /// Writes the UTF-8 bytes of `text`.
    internal mutating func write(text text: String) -> Result[(), IoError] {
        let raw = text.asByteSlice();
        self.writeAll(from: Slice.lend(start: raw.pointer, count: raw.count))
    }

    /// Writes `text` and a `"\n"`.
    internal mutating func write(line line: String) -> Result[(), IoError] {
        try self.write(text: line);
        self.write(byte: 10)
    }

    /// Writes `value` formatted with its default options.
    internal mutating func write(formatted value: some Formattable) -> Result[(), IoError] {
        self.write(text: value.formatted())
    }

    /// Wraps the writer in a `BufferedWriter`, so small writes are batched
    /// into fewer system calls.
    internal consuming func buffered() -> BufferedWriter[Self] {
        BufferedWriter(self)
    }
}
