// Write trait and utilities

module io.write

import std.num.(Int64, UInt8)
import std.result.(Result)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import io.error.(Error, brokenPipe)

// Write trait - sink for bytes
public protocol Write {
    // Write bytes from buffer, return number of bytes written.
    func write(from buf: Slice[UInt8]) -> Result[Int64, Error]

    // Flush buffered data
    func flush() -> Result[(), Error]
}

// Sink - discards all bytes
public struct Sink: Write {
    public init() {}

    public func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        .Ok(buf.count)
    }

    public func flush() -> Result[(), Error] {
        .Ok(())
    }
}

// Buffer writer - writes to a growable array
public struct Buffer: Write {
    var data: Array[UInt8]

    public init() {
        self.data = Array[UInt8]()
    }

    public init(capacity: Int64) {
        self.data = Array(capacity: capacity)
    }

    public func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        var i: Int64 = 0;
        while i < buf.count {
            self.data.append(buf.pointer.offset(by: i).read());
            i = i + 1
        }
        .Ok(buf.count)
    }

    public func flush() -> Result[(), Error] {
        .Ok(())
    }

    public func count() -> Int64 {
        self.data.count()
    }

    public mutating func clear() {
        self.data.clear()
    }
}

// Helper functions for writers

// Write all bytes from a slice
public func writeAll[W](writer: W, from buf: Slice[UInt8]) -> Result[(), Error] where W: Write {
    var written: Int64 = 0;
    while written < buf.count {
        let remaining = Slice(pointer: buf.pointer.offset(by: written), count: buf.count - written);
        let n = try writer.write(from: remaining);
        if n == 0 {
            return .Err(brokenPipe())
        }
        written = written + n
    }
    .Ok(())
}

// Write a single byte
public func writeByte[W](writer: W, byte: UInt8) -> Result[(), Error] where W: Write {
    var buf = Array[UInt8](capacity: 1);
    buf.append(byte);
    let slice = Slice(pointer: buf.pointer(), count: 1);
    writeAll(writer: writer, from: slice)
}

// Write a string as UTF-8
public func writeStr[W](writer: W, s: String) -> Result[(), Error] where W: Write {
    // Get the byte count and pointer from string
    let byteCount = s.byteCount();
    if byteCount == 0 {
        return .Ok(())
    }
    // Create a slice from the string's internal bytes
    // Note: String stores bytes internally, we need to access them
    var i: Int64 = 0;
    while i < byteCount {
        let byte = s.byteAtUnchecked(i);
        try writeByte(writer: writer, byte: byte);
        i = i + 1
    }
    .Ok(())
}

// Write string with newline
public func writeLine[W](writer: W, s: String) -> Result[(), Error] where W: Write {
    try writeStr(writer: writer, s: s);
    writeByte(writer: writer, byte: 10)  // '\n'
}
