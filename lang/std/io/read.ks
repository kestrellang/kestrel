// Readable trait and utilities

module std.io.read

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.memory.(ArraySlice, Pointer)
import std.collections.(Array)
import std.core.(Bool)
import std.io.error.(IoError, invalidInput)

// ============================================================================
// READ PROTOCOL
// ============================================================================

/// Protocol for byte-source streams.
///
/// A single `read(into:)` call reads up to `buf.count` bytes into the
/// provided slice and returns how many actually landed. A return of `0`
/// means end-of-stream; a partial read (`n < buf.count`) is *not* an
/// error — the caller is expected to loop, or to use `readExact` /
/// `readAll` when a specific shape is required.
///
/// # Examples
///
/// ```
/// public struct DigitsReader: Readable {
///     var next: UInt8
///     public mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
///         if buf.count == 0 { return .Ok(0) }
///         buf.pointer.write(self.next);
///         self.next = self.next + 1;
///         .Ok(1)
///     }
/// }
/// ```
public protocol Readable {
    /// Reads up to `buf.count` bytes; returns the number of bytes
    /// actually written, with `0` signalling EOF.
    mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError]
}

// ============================================================================
// EMPTY READER
// ============================================================================

/// `Readable` that always returns `0` (EOF). Useful as a placeholder reader
/// or in tests that need to assert how a consumer handles an empty source.
///
/// # Representation
///
/// Zero-sized — no fields.
public struct Empty: Readable {
    /// @name Default
    /// Builds the empty reader.
    public init() {}

    /// Always returns `.Ok(0)`.
    public mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        .Ok(0)
    }
}

// ============================================================================
// REPEAT READER
// ============================================================================

/// `Readable` that yields the same byte forever — analogous to `/dev/zero`
/// (with `byte: 0`) or `yes(1)`. Each `read` fills the entire destination.
///
/// # Representation
///
/// One `UInt8` field for the repeated byte.
public struct Repeat: Readable {
    var byte: UInt8

    /// @name From Byte
    /// Builds a reader that yields `byte` indefinitely.
    public init(byte byte: UInt8) {
        self.byte = byte
    }

    /// Fills `buf` with the repeated byte; returns `.Ok(buf.count)`.
    public mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        var i: Int64 = 0;
        while i < buf.count {
            buf.pointer.offset(by: i).write(self.byte);
            i = i + 1
        }
        .Ok(buf.count)
    }
}

// ============================================================================
// CURSOR
// ============================================================================

/// `Readable` over an in-memory `Array[UInt8]` with a movable position.
///
/// Mirrors the role of Rust's `io::Cursor` — useful for tests, parsers, and
/// any place a byte buffer needs to be presented as a `Readable` stream. The
/// position is clamped to `[0, count]` by `setPosition`. `Cursor` clones
/// share the underlying COW array.
///
/// # Examples
///
/// ```
/// var c = Cursor(data: [10, 20, 30].asArray());
/// var buf = Array[UInt8](repeating: 0, count: 2);
/// try c.read(into: buf.asSlice());     // .Ok(2); buf == [10, 20]
/// c.position()                         // 2
/// ```
public struct Cursor: Readable, Cloneable {
    var data: Array[UInt8]
    public var position: Int64

    /// @name From Bytes
    /// Builds a cursor positioned at byte 0 over `data`.
    public init(data data: Array[UInt8]) {
        self.data = data;
        self.position = 0;
    }

    /// Deep-clones the underlying byte array and copies the position.
    public func clone() -> Cursor {
        var c = Cursor(data: self.data.clone());
        c.position = self.position;
        c
    }

    /// Reads from the current position; returns `.Ok(0)` at EOF and
    /// advances the position by the byte count returned.
    public mutating func read(into buf: ArraySlice[UInt8]) -> Result[Int64, IoError] {
        let available = self.data.count - self.position;
        if available == 0 {
            return .Ok(0)
        }

        var n: Int64 = buf.count;
        if n > available {
            n = available
        }
        var i: Int64 = 0;
        while i < n {
            buf.pointer.offset(by: i).write(self.data(unchecked: self.position + i));
            i = i + 1
        }
        self.position = self.position + n;
        .Ok(n)
    }

    /// Sets the position. Negative values clamp to `0`; values past the
    /// end clamp to `count`.
    public mutating func setPosition(to pos: Int64) {
        let count = self.data.count;
        if pos < 0 {
            self.position = 0
        } else if pos > count {
            self.position = count
        } else {
            self.position = pos
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Reads exactly one byte. Returns `.Ok(.None)` on EOF, `.Ok(.Some(b))`
/// on success, or propagates a reader error.
///
/// # Examples
///
/// ```
/// match try readByte(reader) {
///     .Some(b) => use(b),
///     .None => /* EOF */ break
/// }
/// ```
public func readByte[R](mutating reader: R) -> Result[Optional[UInt8], IoError] where R: Readable, R: not Copyable {
    var buf = Array[UInt8](capacity: 1);
    buf.append(0);
    let slice = ArraySlice(pointer: buf.asPointer(), count: 1);
    let n = try reader.read(into: slice);
    if n == 0 {
        .Ok(.None)
    } else {
        .Ok(.Some(buf(unchecked: 0)))
    }
}

/// Drains `reader` into `buf`, appending every byte until EOF. Reads in
/// 4 KiB chunks. Returns the total number of bytes appended.
///
/// # Examples
///
/// ```
/// var bytes = Array[UInt8]();
/// var file = try File.open("input.bin");
/// let total = try readAll(file, into: bytes);
/// ```
public func readAll[R](mutating reader: R, mutating into buf: Array[UInt8]) -> Result[Int64, IoError] where R: Readable, R: not Copyable {
    var total: Int64 = 0;
    var chunk = Array[UInt8](capacity: 4096);
    // Initialize chunk with zeros
    var i: Int64 = 0;
    while i < 4096 {
        chunk.append(0);
        i = i + 1
    }

    loop {
        let slice = ArraySlice(pointer: chunk.asPointer(), count: 4096);
        let n = try reader.read(into: slice);
        if n == 0 {
            break
        }
        var j: Int64 = 0;
        while j < n {
            buf.append(chunk(unchecked: j));
            j = j + 1
        }
        total = total + n
    }
    .Ok(total)
}

/// Reads exactly `buf.count` bytes; treats a short read (EOF reached
/// early) as an error rather than a quiet truncation. Use when the
/// caller wants binary fidelity — e.g. reading a fixed-width header.
///
/// # Errors
///
/// Returns `.Err(invalidInput())` if EOF is reached before `buf.count`
/// bytes have been collected.
///
/// # Examples
///
/// ```
/// var header = Array[UInt8](repeating: 0, count: 16);
/// try readExact(file, into: header.asSlice());   // must read 16 bytes
/// ```
public func readExact[R](mutating reader: R, into buf: ArraySlice[UInt8]) -> Result[(), IoError] where R: Readable, R: not Copyable {
    var filled: Int64 = 0;
    while filled < buf.count {
        let remaining = ArraySlice(pointer: buf.pointer.offset(by: filled), count: buf.count - filled);
        let n = try reader.read(into: remaining);
        if n == 0 {
            return .Err(invalidInput())
        }
        filled = filled + n
    }
    .Ok(())
}
