// In-memory streams: Cursor, Buffer, Sink, Empty and Repeat.

module std.streams

import std.numeric.(Int64, UInt8)
import std.result.(Result, Optional)
import std.collections.(Array, Slice)
import std.text.(String)
import std.core.(Bool, Cloneable)

// ============================================================================
// CURSOR
// ============================================================================

/// Reads from bytes it owns, with a position that can move. Use it to hand
/// an in-memory byte array or string to code that takes a `Readable`.
///
/// # Examples
///
/// ```
/// var c = Cursor(text: "abc");
/// try c.readByte()     // .Some(97)
/// c.position           // 1
/// ```
public struct Cursor: Readable, Seekable, Cloneable {
    var data: Array[UInt8]
    var offset: Int64

    /// @name From Bytes
    /// Reads `bytes`, starting at the first.
    public init(bytes bytes: Array[UInt8]) {
        self.data = bytes;
        self.offset = 0;
    }

    /// @name From Text
    /// Reads the UTF-8 bytes of `text`, starting at the first.
    public init(text text: String) {
        let raw = text.asByteSlice();
        self.data = unsafe Slice.lend(start: raw.pointer, count: raw.count).toOwned();
        self.offset = 0;
    }

    /// The offset of the next byte to read. Setting it past the end makes
    /// reads return end of stream; it never fails, unlike `seek(to:)`.
    public var position: Int64 {
        get { self.offset }
        set { self.offset = if newValue < 0 { 0 } else { newValue } }
    }

    /// Bytes left to read.
    public var remaining: Int64 {
        if self.offset >= self.data.count { 0 } else { self.data.count - self.offset }
    }

    /// The bytes being read, all of them.
    public func asSlice() -> &Slice[UInt8] {
        self.data.asSlice()
    }

    /// Copies from the current position and advances it.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let left = self.remaining;
        let n = if buf.count < left { buf.count } else { left };
        if n > 0 {
            buf(0..<n).assign(from: self.data(self.offset..<(self.offset + n)));
            self.offset = self.offset + n
        }
        .Ok(n)
    }

    /// Moves the position. Past the end is allowed; before the start is
    /// `InvalidInput`.
    public mutating func seek(to position: Seek) -> Result[Int64, IoError] {
        let target = match position {
            .Start(n) => n,
            .Current(n) => self.offset + n,
            .End(n) => self.data.count + n
        };
        if target < 0 {
            return .Err(IoError(kind: .InvalidInput))
        }
        self.offset = target;
        .Ok(target)
    }

    /// Copies the bytes and the position.
    public func clone() -> Cursor {
        var c = Cursor(bytes: self.data.clone());
        c.offset = self.offset;
        c
    }
}

// ============================================================================
// BUFFER
// ============================================================================

/// Collects written bytes in memory.
///
/// # Examples
///
/// ```
/// var b = Buffer();
/// try b.write(text: "hi");
/// b.toText()           // .Some("hi")
/// ```
public struct Buffer: Writable, Cloneable {
    var data: Array[UInt8]

    /// @name Empty
    /// An empty buffer.
    public init() {
        self.data = Array[UInt8]()
    }

    /// @name With Capacity
    /// An empty buffer with room for `capacity` bytes before it grows.
    public init(capacity capacity: Int64) {
        self.data = Array[UInt8](capacity: capacity)
    }

    /// Bytes written so far.
    public var count: Int64 { self.data.count }

    /// `true` until something is written.
    public var isEmpty: Bool { self.data.count == 0 }

    /// The written bytes, borrowed from the buffer.
    public func asSlice() -> &Slice[UInt8] {
        self.data.asSlice()
    }

    /// The written bytes, moved out.
    public consuming func toArray() -> Array[UInt8] {
        self.data
    }

    /// The written bytes as text, or `None` if they aren't valid UTF-8.
    public func toText() -> Optional[String] {
        String(fromUtf8: self.data.asSlice())
    }

    /// Drops the bytes and keeps the capacity.
    public mutating func clear() {
        self.data.clear()
    }

    /// Appends all of `buf`.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        self.data.append(contentsOf: buf);
        .Ok(buf.count)
    }

    /// Nothing to flush.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }

    /// Copies the bytes.
    public func clone() -> Buffer {
        var b = Buffer();
        b.data = self.data.clone();
        b
    }
}

// ============================================================================
// SINK, EMPTY, REPEAT
// ============================================================================

/// Accepts and discards every write, like `/dev/null`.
public struct Sink: Writable {
    /// @name Default
    public init() {}

    /// Reports all of `buf` written.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] { .Ok(buf.count) }

    /// Nothing to flush.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }
}

/// Always at end of stream.
public struct Empty: Readable {
    /// @name Default
    public init() {}

    /// Reads nothing.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] { .Ok(0) }
}

/// The same byte forever, like `/dev/zero` for `byte: 0`.
public struct Repeat: Readable {
    var byte: UInt8

    /// @name From Byte
    /// Reads `byte`, endlessly.
    public init(byte byte: UInt8) {
        self.byte = byte
    }

    /// Fills all of `buf`.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        buf.fill(with: self.byte);
        .Ok(buf.count)
    }
}
