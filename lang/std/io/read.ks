// Read trait and utilities

module std.io.read

import std.num.(Int64, UInt8)
import std.result.(Result, Optional)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.core.(Bool)
import std.io.error.(Error)

// ============================================================================
// READ PROTOCOL
// ============================================================================

/// Protocol for types that can be read from.
///
/// Implementors provide a source of bytes that can be read into a buffer.
public protocol Read {
    /// Reads bytes into the buffer.
    ///
    /// Returns the number of bytes read, or 0 on EOF.
    mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error]
}

// ============================================================================
// EMPTY READER
// ============================================================================

/// A reader that always returns EOF immediately.
public struct Empty: Read {
    /// Creates an empty reader.
    public init() {}

    /// Always returns 0 (EOF).
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        .Ok(0)
    }
}

// ============================================================================
// REPEAT READER
// ============================================================================

/// A reader that produces an infinite stream of a single byte.
public struct Repeat: Read {
    var byte: UInt8

    /// Creates a repeat reader that yields the given byte forever.
    public init(byte: UInt8) {
        self.byte = byte
    }

    /// Fills the entire buffer with the repeated byte.
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
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

/// A reader that reads from a byte array with a movable position.
public struct Cursor: Read {
    var data: Array[UInt8]
    var pos: Int64

    /// Creates a cursor that reads from the given data.
    public init(data: Array[UInt8]) {
        self.data = data;
        self.pos = 0;
    }

    /// Reads bytes from the current position.
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let available = self.data.count - self.pos;
        if available == 0 {
            return .Ok(0)
        }

        var n: Int64 = buf.count;
        if n > available {
            n = available
        }
        var i: Int64 = 0;
        while i < n {
            buf.pointer.offset(by: i).write(self.data.getUnchecked(self.pos + i));
            i = i + 1
        }
        self.pos = self.pos + n;
        .Ok(n)
    }

    /// Returns the current position.
    public func position() -> Int64 { self.pos }

    /// Sets the position, clamping to valid range.
    public mutating func setPosition(to pos: Int64) {
        let count = self.data.count;
        if pos < 0 {
            self.pos = 0
        } else if pos > count {
            self.pos = count
        } else {
            self.pos = pos
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Reads a single byte from a reader.
///
/// Returns None on EOF.
public func readByte[R](reader: R) -> Result[Optional[UInt8], Error] where R: Read {
    var buf = Array[UInt8](capacity: 1);
    buf.append(0);
    let slice = Slice(pointer: buf.asPointer(), count: 1);
    let n = try reader.read(into: slice);
    if n == 0 {
        .Ok(.None)
    } else {
        .Ok(.Some(buf.getUnchecked(0)))
    }
}

/// Reads all bytes from a reader into an array.
///
/// Returns the total number of bytes read.
public func readAll[R](reader: R, into buf: Array[UInt8]) -> Result[Int64, Error] where R: Read {
    var total: Int64 = 0;
    var chunk = Array[UInt8](capacity: 4096);
    // Initialize chunk with zeros
    var i: Int64 = 0;
    while i < 4096 {
        chunk.append(0);
        i = i + 1
    }

    loop {
        let slice = Slice(pointer: chunk.asPointer(), count: 4096);
        let n = try reader.read(into: slice);
        if n == 0 {
            break
        }
        var j: Int64 = 0;
        while j < n {
            buf.append(chunk.getUnchecked(j));
            j = j + 1
        }
        total = total + n
    }
    .Ok(total)
}
