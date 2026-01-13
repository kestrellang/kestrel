// Read trait and utilities

module io.read

import std.(Optional, Array, Slice, UInt8)
import io.error.(Error, Result)

// Read trait - source of bytes
public protocol Read {
    // Read bytes into buffer, return number of bytes read.
    // Returns 0 on EOF.
    func read(into buf: Slice[UInt8]) -> Result[Int]
}

// Extension methods for all readers
extension Read {
    // Read until buffer is full or EOF
    public func readExact(into buf: Slice[UInt8]) -> Result[Unit] {
        var filled = 0
        while filled < buf.count {
            let remaining = buf.slice(from: filled, to: buf.count).unwrap()
            let n = try self.read(into: remaining)
            if n == 0 {
                return .Err(Error.custom(message: "unexpected eof"))
            }
            filled += n
        }
        .Ok(())
    }

    // Read all bytes to end into array
    public func readAll(into buf: ref Array[UInt8]) -> Result[Int] {
        var total = 0
        var chunk = Array[UInt8](capacity: 4096)
        chunk.resize(to: 4096, default: 0)

        while true {
            let n = try self.read(into: chunk.asSlice())
            if n == 0 { break }
            for i in 0..<n {
                buf.append(chunk(unchecked: i))
            }
            total += n
        }
        .Ok(total)
    }

    // Read single byte
    public func readByte() -> Result[Optional[UInt8]] {
        var buf = Array[UInt8](capacity: 1)
        buf.resize(to: 1, default: 0)
        let n = try self.read(into: buf.asSlice())
        if n == 0 {
            .Ok(.None)
        } else {
            .Ok(.Some(buf(unchecked: 0)))
        }
    }

    // Create an iterator over bytes
    public func bytes() -> Bytes[Self] {
        Bytes(inner: self)
    }

    // Limit bytes read
    public func take(n: Int) -> Take[Self] {
        Take(inner: self, remaining: n)
    }

    // Chain two readers
    public func chain[R: Read](other: R) -> Chain[Self, R] {
        Chain(first: self, second: other, firstDone: false)
    }
}

// Bytes iterator
public struct Bytes[R: Read]: Iterator {
    type Item = Result[UInt8]

    var inner: R

    public mutating func next() -> Optional[Result[UInt8]] {
        match self.inner.readByte() {
            .Ok(.Some(let b)) => .Some(.Ok(b)),
            .Ok(.None) => .None,
            .Err(let e) => .Some(.Err(e))
        }
    }
}

// Take adapter - limits bytes read
public struct Take[R: Read]: Read {
    var inner: R
    var remaining: Int

    public func read(into buf: Slice[UInt8]) -> Result[Int] {
        if self.remaining == 0 {
            return .Ok(0)
        }
        let max = if buf.count < self.remaining { buf.count } else { self.remaining }
        let limited = buf.slice(from: 0, to: max).unwrap()
        let n = try self.inner.read(into: limited)
        self.remaining -= n
        .Ok(n)
    }

    public var limit: Int { self.remaining }
}

// Chain adapter - reads from first, then second
public struct Chain[R1: Read, R2: Read]: Read {
    var first: R1
    var second: R2
    var firstDone: Bool

    public mutating func read(into buf: Slice[UInt8]) -> Result[Int] {
        if not self.firstDone {
            let n = try self.first.read(into: buf)
            if n > 0 { return .Ok(n) }
            self.firstDone = true
        }
        self.second.read(into: buf)
    }
}

// Empty reader - always returns EOF
public struct Empty: Read {
    public init() {}

    public func read(into buf: Slice[UInt8]) -> Result[Int] {
        .Ok(0)
    }
}

// Repeat reader - infinite stream of a byte
public struct Repeat: Read {
    var byte: UInt8

    public init(byte: UInt8) {
        self.byte = byte
    }

    public func read(into buf: Slice[UInt8]) -> Result[Int] {
        for i in 0..<buf.count {
            buf(unchecked: i) = self.byte
        }
        .Ok(buf.count)
    }
}

// Cursor - read from a byte slice
public struct Cursor: Read {
    var data: Slice[UInt8]
    var pos: Int

    public init(data: Slice[UInt8]) {
        self.data = data
        self.pos = 0
    }

    public func read(into buf: Slice[UInt8]) -> Result[Int] {
        let available = self.data.count - self.pos
        if available == 0 { return .Ok(0) }

        let n = if buf.count < available { buf.count } else { available }
        for i in 0..<n {
            buf(unchecked: i) = self.data(unchecked: self.pos + i)
        }
        self.pos += n
        .Ok(n)
    }

    public var position: Int { self.pos }

    public func setPosition(to pos: Int) {
        self.pos = if pos < 0 { 0 }
            else if pos > self.data.count { self.data.count }
            else { pos }
    }
}
