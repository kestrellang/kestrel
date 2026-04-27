# std.io.read

## struct `Cursor`

```kestrel
public struct Cursor { /* private fields */ }
```

`Read` over an in-memory `Array[UInt8]` with a movable position.

Mirrors the role of Rust's `io::Cursor` — useful for tests, parsers, and
any place a byte buffer needs to be presented as a `Read` stream. The
position is clamped to `[0, count]` by `setPosition`. `Cursor` clones
share the underlying COW array.

### Examples

```
var c = Cursor(data: [10, 20, 30].asArray());
var buf = Array[UInt8](repeating: 0, count: 2);
try c.read(into: buf.asSlice());     // .Ok(2); buf == [10, 20]
c.position()                         // 2
```

_Defined in `lang/std/io/read.ks`._

### Members

#### initializer `From Bytes`

```kestrel
public init(data: Array[UInt8])
```

Builds a cursor positioned at byte 0 over `data`.

_Defined in `lang/std/io/read.ks`._

#### field `data`

```kestrel
var data: Array[UInt8]
```

_Defined in `lang/std/io/read.ks`._

#### field `pos`

```kestrel
var pos: Int64
```

_Defined in `lang/std/io/read.ks`._

#### function `position`

```kestrel
public func position() -> Int64
```

Current byte offset into the underlying array.

_Defined in `lang/std/io/read.ks`._

#### function `setPosition`

```kestrel
public mutating func setPosition(to: Int64)
```

Sets the position. Negative values clamp to `0`; values past the
end clamp to `count`.

_Defined in `lang/std/io/read.ks`._

### Implements `Read`

#### function `read`

```kestrel
public mutating func read(into: Slice[UInt8]) -> Result[Int64, IoError]
```

Reads from the current position; returns `.Ok(0)` at EOF and
advances the position by the byte count returned.

_Defined in `lang/std/io/read.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Cursor
```

Deep-clones the underlying byte array and copies the position.

_Defined in `lang/std/io/read.ks`._

## struct `Empty`

```kestrel
public struct Empty { /* private fields */ }
```

`Read` that always returns `0` (EOF). Useful as a placeholder reader
or in tests that need to assert how a consumer handles an empty source.

### Representation

Zero-sized — no fields.

_Defined in `lang/std/io/read.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds the empty reader.

_Defined in `lang/std/io/read.ks`._

### Implements `Read`

#### function `read`

```kestrel
public mutating func read(into: Slice[UInt8]) -> Result[Int64, IoError]
```

Always returns `.Ok(0)`.

_Defined in `lang/std/io/read.ks`._

## protocol `Read`

```kestrel
public protocol Read
```

Protocol for byte-source streams.

A single `read(into:)` call reads up to `buf.count` bytes into the
provided slice and returns how many actually landed. A return of `0`
means end-of-stream; a partial read (`n < buf.count`) is *not* an
error — the caller is expected to loop, or to use `readExact` /
`readAll` when a specific shape is required.

### Examples

```
public struct DigitsReader: Read {
    var next: UInt8
    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        if buf.count == 0 { return .Ok(0) }
        buf.pointer.write(self.next);
        self.next = self.next + 1;
        .Ok(1)
    }
}
```

_Defined in `lang/std/io/read.ks`._

### Members

#### function `read`

```kestrel
mutating func read(into: Slice[UInt8]) -> Result[Int64, IoError]
```

Reads up to `buf.count` bytes; returns the number of bytes
actually written, with `0` signalling EOF.

_Defined in `lang/std/io/read.ks`._

## struct `Repeat`

```kestrel
public struct Repeat { /* private fields */ }
```

`Read` that yields the same byte forever — analogous to `/dev/zero`
(with `byte: 0`) or `yes(1)`. Each `read` fills the entire destination.

### Representation

One `UInt8` field for the repeated byte.

_Defined in `lang/std/io/read.ks`._

### Members

#### initializer `From Byte`

```kestrel
public init(byte: UInt8)
```

Builds a reader that yields `byte` indefinitely.

_Defined in `lang/std/io/read.ks`._

#### field `byte`

```kestrel
var byte: UInt8
```

_Defined in `lang/std/io/read.ks`._

### Implements `Read`

#### function `read`

```kestrel
public mutating func read(into: Slice[UInt8]) -> Result[Int64, IoError]
```

Fills `buf` with the repeated byte; returns `.Ok(buf.count)`.

_Defined in `lang/std/io/read.ks`._

## function `readAll`

```kestrel
public func readAll[R](mutating R, into: mutating Array[UInt8]) -> Result[Int64, IoError] where R: Read
```

Drains `reader` into `buf`, appending every byte until EOF. Reads in
4 KiB chunks. Returns the total number of bytes appended.

### Examples

```
var bytes = Array[UInt8]();
var file = try File.open("input.bin");
let total = try readAll(file, into: bytes);
```

_Defined in `lang/std/io/read.ks`._

## function `readByte`

```kestrel
public func readByte[R](mutating R) -> Result[Optional[UInt8], IoError] where R: Read
```

Reads exactly one byte. Returns `.Ok(.None)` on EOF, `.Ok(.Some(b))`
on success, or propagates a reader error.

### Examples

```
match try readByte(reader) {
    .Some(b) => use(b),
    .None => /* EOF */ break
}
```

_Defined in `lang/std/io/read.ks`._

## function `readExact`

```kestrel
public func readExact[R](mutating R, into: Slice[UInt8]) -> Result[(), IoError] where R: Read
```

Reads exactly `buf.count` bytes; treats a short read (EOF reached
early) as an error rather than a quiet truncation. Use when the
caller wants binary fidelity — e.g. reading a fixed-width header.

### Errors

Returns `.Err(invalidInput())` if EOF is reached before `buf.count`
bytes have been collected.

### Examples

```
var header = Array[UInt8](repeating: 0, count: 16);
try readExact(file, into: header.asSlice());   // must read 16 bytes
```

_Defined in `lang/std/io/read.ks`._

