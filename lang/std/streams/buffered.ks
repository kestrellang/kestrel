// Buffered streams: BufferedReader, BufferedWriter and Lines.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.collections.(Array, Slice)
import std.text.(String)
import std.core.(Bool)
import std.iter.(Iterator)

// ============================================================================
// BUFFERED READER
// ============================================================================

/// Reads ahead from another reader in large chunks, so small reads and line
/// reading don't cost a system call each.
///
/// # Examples
///
/// ```
/// var input = Cursor(text: "a\nb\n").buffered();
/// try input.readLine()     // .Some("a")
/// ```
public struct BufferedReader[R]: Readable where R: Readable, R: not Copyable, R: not Static {
    var inner: R
    var buffer: Array[UInt8]
    /// Unread bytes are `buffer(start..<end)`.
    var start: Int64
    var end: Int64

    /// @name From Reader
    /// Buffers `inner` with `capacity` bytes of read-ahead.
    public init(consuming inner: R, capacity capacity: Int64 = 8192) {
        self.inner = inner;
        self.buffer = Array[UInt8](repeating: 0, count: if capacity < 1 { 1 } else { capacity });
        self.start = 0;
        self.end = 0;
    }

    /// Bytes read ahead and not yet handed out.
    public var buffered: Int64 { self.end - self.start }

    /// Refills the buffer if it's empty. `false` at end of stream.
    internal mutating func fill() -> Result[Bool, IoError] {
        if self.start < self.end {
            return .Ok(true)
        }
        let n = try self.inner.read(into: self.buffer.asMutableSlice());
        self.start = 0;
        self.end = n;
        .Ok(n > 0)
    }

    /// Hands out buffered bytes first. A read at least as large as the
    /// buffer, with nothing buffered, goes straight to the inner reader.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        if self.start == self.end and buf.count >= self.buffer.count {
            return self.inner.read(into: buf)
        }
        let more = try self.fill();
        if not more {
            return .Ok(0)
        }
        let n = if buf.count < self.buffered { buf.count } else { self.buffered };
        buf(0..<n).assign(from: self.buffer(self.start..<(self.start + n)));
        self.start = self.start + n;
        .Ok(n)
    }

    /// Reads up to and including the next `byte`, appending to `buf`.
    /// Returns the bytes appended; `0` at end of stream.
    public mutating func readUntil(byte byte: UInt8, mutating into buf: Array[UInt8]) -> Result[Int64, IoError] {
        var total: Int64 = 0;
        loop {
            let more = try self.fill();
            if not more {
                break
            }
            var i = self.start;
            while i < self.end and self.buffer(unchecked: i) != byte {
                i = i + 1
            }
            let found = i < self.end;
            let stop = if found { i + 1 } else { self.end };
            buf.append(contentsOf: self.buffer(self.start..<stop));
            total = total + (stop - self.start);
            self.start = stop;
            if found {
                break
            }
        }
        .Ok(total)
    }

    /// The next line without its `"\n"` (or `"\r\n"`); `None` at end of
    /// stream. Invalid UTF-8 becomes U+FFFD.
    public mutating func readLine() -> Result[Optional[String], IoError] {
        var bytes = Array[UInt8]();
        if try self.readUntil(byte: 10, into: bytes) == 0 {
            return .Ok(.None)
        }
        if bytes.count > 0 and bytes(unchecked: bytes.count - 1) == 10 {
            bytes.pop();
            if bytes.count > 0 and bytes(unchecked: bytes.count - 1) == 13 {
                bytes.pop();
            }
        }
        .Ok(.Some(String(fromUtf8Lossy: bytes.asSlice())))
    }

    /// An iterator over the remaining lines.
    public consuming func lines() -> Lines[R] {
        Lines(reader: self)
    }

    /// The inner reader. Bytes already read ahead are lost.
    public consuming func intoInner() -> R {
        self.inner
    }
}

/// The lines of a `BufferedReader`, each as `Result[String, IoError]`.
/// Ends at end of stream or after the first error.
public struct Lines[R]: Iterator where R: Readable, R: not Copyable, R: not Static {
    type Item = Result[String, IoError]

    var reader: BufferedReader[R]
    var done: Bool

    /// @name From Reader
    /// Prefer `reader.lines()`.
    public init(consuming reader reader: BufferedReader[R]) {
        self.reader = reader;
        self.done = false;
    }

    /// The next line, or the error that ended the iteration.
    public mutating func next() -> Optional[Result[String, IoError]] {
        if self.done {
            return .None
        }
        match self.reader.readLine() {
            .Ok(.Some(line)) => .Some(.Ok(line)),
            .Ok(.None) => {
                self.done = true;
                .None
            },
            .Err(e) => {
                self.done = true;
                .Some(.Err(e))
            }
        }
    }
}

// ============================================================================
// BUFFERED WRITER
// ============================================================================

/// Batches small writes into large ones on another writer.
///
/// Dropping it flushes, but an error from that flush is lost: call
/// `flush()` (or `intoInner()`) to see it.
///
/// # Examples
///
/// ```
/// var out = Buffer().buffered();
/// try out.write(line: "hello");
/// try out.flush();
/// ```
public struct BufferedWriter[W]: Writable, not Copyable where W: Writable, W: not Copyable, W: not Static {
    var inner: Optional[W]
    var buffer: Array[UInt8]
    var capacity: Int64

    /// @name From Writer
    /// Buffers up to `capacity` bytes before writing to `inner`.
    public init(consuming inner: W, capacity capacity: Int64 = 8192) {
        self.inner = .Some(inner);
        self.capacity = if capacity < 1 { 1 } else { capacity };
        self.buffer = Array[UInt8](capacity: self.capacity);
    }

    /// Bytes waiting to be written.
    public var buffered: Int64 { self.buffer.count }

    /// Writes the buffered bytes to the inner writer, without flushing it.
    internal mutating func drain() -> Result[(), IoError] {
        if self.buffer.count == 0 {
            return .Ok(())
        }
        match self.inner.chainMutate() {
            .Some(w) => try w.writeAll(from: self.buffer.asSlice()),
            .None => {}
        };
        self.buffer.clear();
        .Ok(())
    }

    /// Buffers `buf`, writing the buffer out first if `buf` won't fit. A
    /// write at least as large as the buffer goes straight through.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        if self.buffer.count + buf.count > self.capacity {
            try self.drain()
        }
        if buf.count >= self.capacity {
            return match self.inner.chainMutate() {
                .Some(w) => w.write(from: buf),
                .None => .Ok(0)
            }
        }
        self.buffer.append(contentsOf: buf);
        .Ok(buf.count)
    }

    /// Writes the buffer out and flushes the inner writer.
    public mutating func flush() -> Result[(), IoError] {
        try self.drain();
        match self.inner.chainMutate() {
            .Some(w) => w.flush(),
            .None => .Ok(())
        }
    }

    /// Flushes, then returns the inner writer.
    public consuming func intoInner() -> Result[W, IoError] {
        try self.flush();
        match self.inner.take() {
            .Some(w) => .Ok(w),
            .None => .Err(IoError(kind: .BrokenPipe))
        }
    }

    deinit {
        let _ = self.flush();
    }
}

// ============================================================================
// SEEKING THROUGH A BUFFER
// ============================================================================

extend BufferedReader[R]: Seekable where R: Seekable {
    /// Drops the read-ahead, then seeks the inner reader. A `.Current`
    /// offset counts from the position the caller sees, not the inner one.
    public mutating func seek(to position: Seek) -> Result[Int64, IoError] {
        let target = match position {
            .Current(n) => Seek.Current(n - self.buffered),
            other => other
        };
        self.start = 0;
        self.end = 0;
        self.inner.seek(to: target)
    }
}

extend BufferedWriter[W]: Seekable where W: Seekable {
    /// Writes the buffer out, then seeks the inner writer.
    public mutating func seek(to position: Seek) -> Result[Int64, IoError] {
        try self.drain();
        match self.inner.chainMutate() {
            .Some(w) => w.seek(to: position),
            .None => .Err(IoError(kind: .BrokenPipe))
        }
    }
}
