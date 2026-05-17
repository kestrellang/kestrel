// Writable trait and utilities

module std.io.write

import std.numeric.(Int64, UInt8)
import std.result.(Result)
import std.memory.(ArraySlice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.io.error.(IoError, brokenPipe)

// ============================================================================
// WRITE PROTOCOL
// ============================================================================

/// Protocol for byte-sink streams.
///
/// Conformers expose a single-shot `write(from:)` and a `flush()` for
/// buffered implementations. As with `Read`, a single `write` may move
/// fewer bytes than supplied — this is not an error; use `writeAll` to
/// loop until the whole slice is consumed.
///
/// # Examples
///
/// ```
/// public struct CountingSink: Writable {
///     var written: Int64 = 0
///     public mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
///         self.written = self.written + buf.count;
///         .Ok(buf.count)
///     }
///     public mutating func flush() -> Result[(), IoError] { .Ok(()) }
/// }
/// ```
public protocol Writable {
    /// Writes up to `buf.count` bytes; returns how many actually moved.
    /// `.Ok(0)` indicates the sink could accept no more right now (full
    /// buffer / would-block); other amounts are partial successes that
    /// the caller may retry.
    mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError]

    /// Pushes any internally buffered bytes to the underlying destination.
    /// Unbuffered writers may implement this as a no-op. Errors here can
    /// surface conditions deferred from earlier `write` calls (broken
    /// pipe, disk full).
    mutating func flush() -> Result[(), IoError]
}

// ============================================================================
// SINK
// ============================================================================

/// `Writable` that swallows everything — analogous to `/dev/null`. Useful
/// for tests, benchmarks, and code paths where output is suppressed.
///
/// # Representation
///
/// Zero-sized — no fields.
public struct Sink: Writable {
    /// @name Default
    /// Builds the discarding sink.
    public init() {}

    /// Returns `.Ok(buf.count)` without storing the bytes.
    public mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        .Ok(buf.count)
    }

    /// No-op; always succeeds.
    public mutating func flush() -> Result[(), IoError] {
        .Ok(())
    }
}

// ============================================================================
// BUFFER
// ============================================================================

/// `Writable` that appends bytes to a growable `Array[UInt8]` — the in-memory
/// counterpart to writing to a file. Useful for capturing output, building
/// byte sequences before flushing to a real sink, or testing formatters.
///
/// `Buffer` clones share the underlying COW array.
///
/// # Examples
///
/// ```
/// var b = Buffer();
/// try writeString(b, "Hello, ");
/// try writeString(b, "World!");
/// b.toString()       // "Hello, World!"
/// ```
///
/// # Representation
///
/// One `Array[UInt8]` field; capacity grows on demand.
public struct Buffer: Writable, Cloneable {
    var data: Array[UInt8]

    /// @name Default
    /// Builds an empty buffer.
    public init() {
        self.data = Array[UInt8]()
    }

    /// Deep-clones the underlying byte array.
    public func clone() -> Buffer {
        var b = Buffer();
        b.data = self.data.clone();
        b
    }

    /// @name With Capacity
    /// Builds an empty buffer pre-sized to `capacity` bytes. Use when the
    /// approximate final size is known to skip intermediate growth.
    public init(capacity: Int64) {
        self.data = Array(capacity: capacity)
    }

    /// Appends every byte from `buf`. Always succeeds with `.Ok(buf.count)`.
    public mutating func write(from buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        var i: Int64 = 0;
        while i < buf.count {
            self.data.append(buf.pointer.offset(by: i).read());
            i = i + 1
        }
        .Ok(buf.count)
    }

    /// No-op; bytes are already "in" the buffer.
    public mutating func flush() -> Result[(), IoError] {
        .Ok(())
    }

    // ========================================================================
    // BUFFER-SPECIFIC METHODS
    // ========================================================================

    /// Bytes currently held.
    public var count: Int64 {
        self.data.count
    }

    /// `true` when no bytes have been written.
    public var isEmpty: Bool {
        self.data.count == 0
    }

    /// Drops every byte but keeps the allocated capacity for reuse.
    public mutating func clear() {
        self.data.clear()
    }

    /// Returns a non-owning slice view over the buffered bytes. The slice
    /// dangles once the buffer is mutated again — copy via `toArray` if
    /// you need to outlive the next write.
    public func asSlice() -> ArraySlice[UInt8] {
        self.data.asSlice()
    }

    /// Returns an owned copy of the buffered bytes.
    public func toArray() -> Array[UInt8] {
        self.data.clone()
    }

    /// Interprets the buffered bytes as UTF-8 and returns the resulting
    /// `String`. Behaviour for invalid UTF-8 is currently undefined —
    /// validate upstream if untrusted bytes are involved.
    public func toString() -> String {
        var result = String();
        var i: Int64 = 0;
        let count = self.data.count;
        while i < count {
            result.appendByte(self.data(unchecked: i));
            i = i + 1
        }
        result
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Writes every byte in `buf`, looping until the full slice has been
/// consumed. Returns `.Err(brokenPipe())` if the writer reports `0` bytes
/// written before the slice is exhausted (matches Rust's
/// `WriteAll`/`ErrorKind::WriteZero`).
///
/// # Examples
///
/// ```
/// var file = try File.create("output.bin");
/// try writeAll(file, from: data.asSlice());
/// ```
public func writeAll[W](mutating writer: W, from buf: ArraySlice[UInt8]) -> Result[(), IoError] where W: Writable {
    var written: Int64 = 0;
    while written < buf.count {
        let remaining = ArraySlice(pointer: buf.pointer.offset(by: written), count: buf.count - written);
        let n = try writer.write(from: remaining);
        if n == 0 {
            return .Err(brokenPipe())
        }
        written = written + n
    };
    .Ok(())
}

/// Writes a single byte, looping internally until it lands.
public func writeByte[W](mutating writer: W, byte: UInt8) -> Result[(), IoError] where W: Writable {
    var buf = Array[UInt8](capacity: 1);
    buf.append(byte);
    let slice = ArraySlice(pointer: buf.asPointer(), count: 1);
    writeAll(writer, from: slice)
}

/// Writes the UTF-8 encoding of `s`. Empty strings short-circuit. Currently
/// emits one byte per call into the writer — fine for buffered sinks like
/// `Buffer`, expensive for raw `File`/`Stdout` (TODO: collect into a slice
/// first).
public func writeString[W](mutating writer: W, s: String) -> Result[(), IoError] where W: Writable {
    // Get the byte count and pointer from string
    let byteCount = s.byteCount;
    if byteCount == 0 {
        return .Ok(())
    }
    // Create a slice from the string's internal bytes
    // Note: String stores bytes internally, we need to access them
    var i: Int64 = 0;
    while i < byteCount {
        let byte = s.bytes(unchecked: i);
        try writeByte(writer, byte);
        i = i + 1
    };
    .Ok(())
}

/// Writes `s` followed by a single `\n`. Does not append `\r` on any
/// platform — Kestrel writes Unix line endings everywhere by default.
public func writeLine[W](mutating writer: W, s: String) -> Result[(), IoError] where W: Writable {
    try writeString(writer, s);
    writeByte(writer, 10)  // '\n'
}
