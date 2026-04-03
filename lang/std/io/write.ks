// Write trait and utilities

module std.io.write

import std.num.(Int64, UInt8)
import std.result.(Result)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.io.error.(Error, brokenPipe)

// ============================================================================
// WRITE PROTOCOL
// ============================================================================

/// Protocol for types that can be written to as a sink for bytes.
///
/// Implementors provide `write` and `flush` methods. Writers may buffer
/// data internally; call `flush` to ensure all data reaches its destination.
///
/// Example implementation:
///     struct MyWriter: Write {
///         var buffer: [UInt8] = []
///
///         mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
///             buffer.append(contentsOf: buf)
///             return .Ok(buf.count)
///         }
///
///         mutating func flush() -> Result[(), Error] {
///             // send buffer to destination
///             buffer.clear()
///             return .Ok(())
///         }
///     }
public protocol Write {
    /// Writes bytes from the buffer, returning the number of bytes written.
    ///
    /// Behavior:
    /// - Returns Ok(n) where n > 0: successfully wrote n bytes from buf[0..n]
    /// - Returns Ok(0): no bytes written (may indicate full buffer or would block)
    /// - Returns Err: an error occurred
    ///
    /// A write may write fewer bytes than provided. This is not an error -
    /// use writeAll() to ensure all bytes are written, or retry with the
    /// remaining bytes.
    ///
    /// Example:
    ///     let data: [UInt8] = [1, 2, 3, 4, 5]
    ///     let n = try writer.write(from: data.asSlice())
    ///     // n bytes written, may be less than 5
    mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error]

    /// Flushes any buffered data to the underlying destination.
    ///
    /// Call flush to ensure all previously written data has been transmitted.
    /// For unbuffered writers, this may be a no-op.
    ///
    /// Returns Err if flushing fails (e.g., disk full, broken pipe).
    ///
    /// Example:
    ///     try writer.write(from: data.asSlice())
    ///     try writer.flush()  // ensure data is committed
    mutating func flush() -> Result[(), Error]
}

// ============================================================================
// SINK
// ============================================================================

/// A writer that discards all bytes written to it.
///
/// Useful for testing, benchmarking, or suppressing output.
/// Analogous to /dev/null.
///
/// Example:
///     var sink = Sink()
///     try sink.write(from: hugeData.asSlice())  // instantly "succeeds"
///     // data is discarded, nothing stored
public struct Sink: Write {
    /// Creates a sink writer that discards all data.
    ///
    /// Example:
    ///     let devNull = Sink()
    public init() {}

    /// Discards all bytes and returns Ok with the buffer length.
    ///
    /// Always succeeds, always reports all bytes as "written".
    ///
    /// Example:
    ///     var sink = Sink()
    ///     let n = try sink.write(from: data.asSlice())  // n == data.count
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        .Ok(buf.count)
    }

    /// No-op flush, always succeeds.
    ///
    /// Example:
    ///     var sink = Sink()
    ///     try sink.flush()  // Ok(())
    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }
}

// ============================================================================
// BUFFER
// ============================================================================

/// A writer that accumulates bytes in a growable in-memory buffer.
///
/// Useful for building byte sequences, capturing output, or testing.
/// Access the accumulated data via `asSlice()` or `toArray()`.
///
/// Example:
///     var buf = Buffer()
///     try writeStr(writer: buf, s: "Hello, ")
///     try writeStr(writer: buf, s: "World!")
///     let result = buf.toString()  // "Hello, World!"
public struct Buffer: Write {
    var data: Array[UInt8]

    /// Creates an empty buffer writer.
    ///
    /// Example:
    ///     var buf = Buffer()
    public init() {
        self.data = Array[UInt8]()
    }

    /// Creates a buffer writer with the specified initial capacity.
    ///
    /// Pre-allocating capacity avoids reallocations when the approximate
    /// final size is known.
    ///
    /// Example:
    ///     var buf = Buffer(capacity: 4096)
    public init(capacity: Int64) {
        self.data = Array(capacity: capacity)
    }

    /// Appends bytes to the internal buffer.
    ///
    /// Always succeeds and writes all bytes. Returns Ok(buf.count).
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     let data: [UInt8] = [1, 2, 3]
    ///     try buf.write(from: data.asSlice())
    ///     buf.count()  // 3
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        var i: Int64 = 0;
        while i < buf.count {
            self.data.append(buf.pointer.offset(by: i).read());
            i = i + 1
        }
        .Ok(buf.count)
    }

    /// No-op flush, always succeeds.
    ///
    /// Buffer keeps data in memory, so flush has no effect.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     try buf.flush()  // Ok(())
    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }

    // ========================================================================
    // BUFFER-SPECIFIC METHODS
    // ========================================================================

    /// Returns the number of bytes in the buffer.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     try buf.write(from: [1, 2, 3].asSlice())
    ///     buf.count()  // 3
    public func count() -> Int64 {
        self.data.count
    }

    /// Returns true if the buffer is empty.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     buf.isEmpty()  // true
    ///     try buf.write(from: [1].asSlice())
    ///     buf.isEmpty()  // false
    public func isEmpty() -> Bool {
        self.data.count == 0
    }

    /// Clears all bytes from the buffer.
    ///
    /// Capacity is retained for reuse.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     try buf.write(from: data.asSlice())
    ///     buf.clear()
    ///     buf.count()  // 0
    public mutating func clear() {
        self.data.clear()
    }

    /// Returns a slice view of the buffer contents.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     try buf.write(from: [1, 2, 3].asSlice())
    ///     let slice = buf.asSlice()  // view of [1, 2, 3]
    public func asSlice() -> Slice[UInt8] {
        self.data.asSlice()
    }

    /// Returns the buffer contents as an owned array.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     try buf.write(from: [1, 2, 3].asSlice())
    ///     let arr = buf.toArray()  // [1, 2, 3]
    public func toArray() -> Array[UInt8] {
        self.data.clone()
    }

    /// Interprets the buffer contents as a UTF-8 string.
    ///
    /// Assumes the buffer contains valid UTF-8. Behavior is undefined
    /// if the buffer contains invalid UTF-8 sequences.
    ///
    /// Example:
    ///     var buf = Buffer()
    ///     try writeStr(writer: buf, s: "Hello")
    ///     buf.toString()  // "Hello"
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

/// Writes all bytes from a slice to a writer.
///
/// Retries partial writes until all bytes are written or an error occurs.
/// Use this when you need to ensure the entire slice is written.
///
/// Example:
///     var file = try File.create(path: "output.bin")
///     try writeAll(writer: file, from: data.asSlice())
///     // all bytes guaranteed written (or error)
public func writeAll[W](mutating writer: W, from buf: Slice[UInt8]) -> Result[(), Error] where W: Write {
    var written: Int64 = 0;
    while written < buf.count {
        let remaining = Slice(pointer: buf.pointer.offset(by: written), count: buf.count - written);
        let n = try writer.write(from: remaining);
        if n == 0 {
            return .Err(brokenPipe())
        }
        written = written + n
    };
    .Ok(())
}

/// Writes a single byte to a writer.
///
/// Example:
///     var file = try File.create(path: "output.bin")
///     try writeByte(writer: file, byte: 0xFF)
public func writeByte[W](mutating writer: W, byte: UInt8) -> Result[(), Error] where W: Write {
    var buf = Array[UInt8](capacity: 1);
    buf.append(byte);
    let slice = Slice(pointer: buf.asPointer(), count: 1);
    writeAll(writer, from: slice)
}

/// Writes a string as UTF-8 bytes to a writer.
///
/// Example:
///     var file = try File.create(path: "greeting.txt")
///     try writeStr(writer: file, s: "Hello, World!")
public func writeStr[W](mutating writer: W, s: String) -> Result[(), Error] where W: Write {
    // Get the byte count and pointer from string
    let byteCount = s.byteCount;
    if byteCount == 0 {
        return .Ok(())
    }
    // Create a slice from the string's internal bytes
    // Note: String stores bytes internally, we need to access them
    var i: Int64 = 0;
    while i < byteCount {
        let byte = s.byteAtUnchecked(i);
        try writeByte(writer, byte);
        i = i + 1
    };
    .Ok(())
}

/// Writes a string followed by a newline to a writer.
///
/// Appends a single '\n' character after the string.
///
/// Example:
///     var file = try File.create(path: "lines.txt")
///     try writeLine(writer: file, s: "First line")
///     try writeLine(writer: file, s: "Second line")
public func writeLine[W](mutating writer: W, s: String) -> Result[(), Error] where W: Write {
    try writeStr(writer, s);
    writeByte(writer, 10)  // '\n'
}
