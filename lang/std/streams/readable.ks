// The Readable protocol, its extension methods, and the Take and Chain
// adapters.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.collections.(Array, Slice)
import std.text.(String)
import std.core.(Bool)

// ============================================================================
// READABLE
// ============================================================================

/// A source of bytes.
///
/// One `read(into:)` moves up to `buf.count` bytes into `buf` and returns
/// how many it moved. `0` means end of stream. A short read is not an
/// error: loop, or use `readExact` / `readAll` when the shape matters.
///
/// # Examples
///
/// ```
/// struct Zeros: Readable {
///     public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
///         buf.fill(with: 0);
///         .Ok(buf.count)
///     }
/// }
/// ```
public protocol Readable {
    /// Reads up to `buf.count` bytes into `buf`; `0` at end of stream.
    mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError]
}

extend Readable {
    /// Reads to end of stream, appending to `buf`. Returns the number of
    /// bytes appended. Retries reads interrupted by a signal.
    public mutating func readAll(mutating into buf: Array[UInt8]) -> Result[Int64, IoError] {
        var chunk = Array[UInt8](repeating: 0, count: 8192);
        var total: Int64 = 0;
        loop {
            let n = match self.read(into: chunk.asMutableSlice()) {
                .Ok(n) => n,
                .Err(e) => {
                    if isInterrupted(e) { continue }
                    return .Err(e)
                }
            };
            if n == 0 {
                break
            }
            buf.append(contentsOf: chunk(0..<n));
            total = total + n
        }
        .Ok(total)
    }

    /// Fills all of `buf`. A stream that ends first is an error, not a
    /// short read: use it for fixed-size records and headers.
    ///
    /// # Errors
    ///
    /// `UnexpectedEnd` if the stream ends before `buf` is full.
    public mutating func readExact(mutating into buf: Slice[UInt8]) -> Result[(), IoError] {
        var filled: Int64 = 0;
        while filled < buf.count {
            let n = match self.read(into: buf(filled..)) {
                .Ok(n) => n,
                .Err(e) => {
                    if isInterrupted(e) { continue }
                    return .Err(e)
                }
            };
            if n == 0 {
                return .Err(IoError(kind: .UnexpectedEnd))
            }
            filled = filled + n
        }
        .Ok(())
    }

    /// Reads one byte; `None` at end of stream.
    public mutating func readByte() -> Result[Optional[UInt8], IoError] {
        var one = Array[UInt8](repeating: 0, count: 1);
        if try self.read(into: one.asMutableSlice()) == 0 {
            return .Ok(.None)
        }
        .Ok(.Some(unsafe one(unchecked: 0)))
    }

    /// Reads to end of stream as UTF-8 text.
    ///
    /// # Errors
    ///
    /// `InvalidData` if the bytes aren't valid UTF-8.
    public mutating func readText() -> Result[String, IoError] {
        var bytes = Array[UInt8]();
        try self.readAll(into: bytes);
        match String(fromUtf8: bytes.asSlice()) {
            .Some(text) => .Ok(text),
            .None => .Err(IoError(kind: .InvalidData))
        }
    }

    /// Wraps the stream in a `BufferedReader`, for line reading and fewer
    /// system calls.
    public consuming func buffered() -> BufferedReader[Self] {
        BufferedReader(self)
    }

    /// A reader that stops after `bytes` bytes of this one.
    public consuming func take(bytes bytes: Int64) -> TakeReader[Self] {
        TakeReader(inner: self, bytes: bytes)
    }

    /// A reader that reads this one to its end, then `next`.
    public consuming func chained[R](consuming with next: R) -> ChainReader[Self, R] where R: Readable, R: not Copyable, R: not Static {
        ChainReader(first: self, second: next)
    }
}

/// The call was cut short by a signal and can be retried.
internal func isInterrupted(error: IoError) -> Bool {
    error.kind == .Interrupted
}

// ============================================================================
// TAKE
// ============================================================================

/// Reads at most a fixed number of bytes from another reader, then reports
/// end of stream. Built by `reader.take(bytes:)`.
public struct TakeReader[R]: Readable where R: Readable, R: not Copyable, R: not Static {
    var inner: R
    var left: Int64

    /// @name From Reader
    /// Limits `inner` to `bytes` more bytes. Prefer `inner.take(bytes:)`.
    public init(consuming inner inner: R, bytes bytes: Int64) {
        self.inner = inner;
        self.left = bytes;
    }

    /// Bytes still allowed before end of stream.
    public var limit: Int64 { self.left }

    /// Reads from the inner reader, never past the limit.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        if self.left <= 0 {
            return .Ok(0)
        }
        let n = if buf.count > self.left {
            try self.inner.read(into: buf(0..<self.left))
        } else {
            try self.inner.read(into: buf)
        };
        self.left = self.left - n;
        .Ok(n)
    }

    /// The inner reader, positioned after what was read through this one.
    public consuming func intoInner() -> R {
        self.inner
    }
}

// ============================================================================
// CHAIN
// ============================================================================

/// Reads one reader to its end, then another. Built by
/// `first.chained(with: second)`.
public struct ChainReader[A, B]: Readable where A: Readable, B: Readable, A: not Copyable, B: not Copyable, A: not Static, B: not Static {
    var first: A
    var second: B
    var firstDone: Bool

    /// @name From Readers
    /// Reads `first`, then `second`. Prefer `first.chained(with: second)`.
    public init(consuming first first: A, consuming second second: B) {
        self.first = first;
        self.second = second;
        self.firstDone = false;
    }

    /// Reads from the first reader until it ends, then from the second.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        if not self.firstDone {
            let n = try self.first.read(into: buf);
            if n > 0 or buf.count == 0 {
                return .Ok(n)
            }
            self.firstDone = true
        }
        self.second.read(into: buf)
    }
}
