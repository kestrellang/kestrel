# std.io.write

## struct `Buffer`

```kestrel
public struct Buffer { /* private fields */ }
```

`Write` that appends bytes to a growable `Array[UInt8]` — the in-memory
counterpart to writing to a file. Useful for capturing output, building
byte sequences before flushing to a real sink, or testing formatters.

`Buffer` clones share the underlying COW array.

### Examples

```
var b = Buffer();
try writeStr(b, "Hello, ");
try writeStr(b, "World!");
b.toString()       // "Hello, World!"
```

### Representation

One `Array[UInt8]` field; capacity grows on demand.

_Defined in `lang/std/io/write.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds an empty buffer.

_Defined in `lang/std/io/write.ks`._

#### initializer `With Capacity`

```kestrel
public init(Int64)
```

Builds an empty buffer pre-sized to `capacity` bytes. Use when the
approximate final size is known to skip intermediate growth.

_Defined in `lang/std/io/write.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> Slice[UInt8]
```

Returns a non-owning slice view over the buffered bytes. The slice
dangles once the buffer is mutated again — copy via `toArray` if
you need to outlive the next write.

_Defined in `lang/std/io/write.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Drops every byte but keeps the allocated capacity for reuse.

_Defined in `lang/std/io/write.ks`._

#### function `count`

```kestrel
public func count() -> Int64
```

Bytes currently held.

_Defined in `lang/std/io/write.ks`._

#### field `data`

```kestrel
var data: Array[UInt8]
```

_Defined in `lang/std/io/write.ks`._

#### function `isEmpty`

```kestrel
public func isEmpty() -> Bool
```

`true` when no bytes have been written.

_Defined in `lang/std/io/write.ks`._

#### function `toArray`

```kestrel
public func toArray() -> Array[UInt8]
```

Returns an owned copy of the buffered bytes.

_Defined in `lang/std/io/write.ks`._

#### function `toString`

```kestrel
public func toString() -> String
```

Interprets the buffered bytes as UTF-8 and returns the resulting
`String`. Behaviour for invalid UTF-8 is currently undefined —
validate upstream if untrusted bytes are involved.

_Defined in `lang/std/io/write.ks`._

### Implements `Write`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op; bytes are already "in" the buffer.

_Defined in `lang/std/io/write.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Appends every byte from `buf`. Always succeeds with `.Ok(buf.count)`.

_Defined in `lang/std/io/write.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Buffer
```

Deep-clones the underlying byte array.

_Defined in `lang/std/io/write.ks`._

## struct `Sink`

```kestrel
public struct Sink { /* private fields */ }
```

`Write` that swallows everything — analogous to `/dev/null`. Useful
for tests, benchmarks, and code paths where output is suppressed.

### Representation

Zero-sized — no fields.

_Defined in `lang/std/io/write.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

Builds the discarding sink.

_Defined in `lang/std/io/write.ks`._

### Implements `Write`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op; always succeeds.

_Defined in `lang/std/io/write.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Returns `.Ok(buf.count)` without storing the bytes.

_Defined in `lang/std/io/write.ks`._

## protocol `Write`

```kestrel
public protocol Write
```

Protocol for byte-sink streams.

Conformers expose a single-shot `write(from:)` and a `flush()` for
buffered implementations. As with `Read`, a single `write` may move
fewer bytes than supplied — this is not an error; use `writeAll` to
loop until the whole slice is consumed.

### Examples

```
public struct CountingSink: Write {
    var written: Int64 = 0
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        self.written = self.written + buf.count;
        .Ok(buf.count)
    }
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }
}
```

_Defined in `lang/std/io/write.ks`._

### Members

#### function `flush`

```kestrel
mutating func flush() -> Result[(), IoError]
```

Pushes any internally buffered bytes to the underlying destination.
Unbuffered writers may implement this as a no-op. Errors here can
surface conditions deferred from earlier `write` calls (broken
pipe, disk full).

_Defined in `lang/std/io/write.ks`._

#### function `write`

```kestrel
mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Writes up to `buf.count` bytes; returns how many actually moved.
`.Ok(0)` indicates the sink could accept no more right now (full
buffer / would-block); other amounts are partial successes that
the caller may retry.

_Defined in `lang/std/io/write.ks`._

## function `writeAll`

```kestrel
public func writeAll[W](mutating W, from: Slice[UInt8]) -> Result[(), IoError] where W: Write
```

Writes every byte in `buf`, looping until the full slice has been
consumed. Returns `.Err(brokenPipe())` if the writer reports `0` bytes
written before the slice is exhausted (matches Rust's
`WriteAll`/`ErrorKind::WriteZero`).

### Examples

```
var file = try File.create("output.bin");
try writeAll(file, from: data.asSlice());
```

_Defined in `lang/std/io/write.ks`._

## function `writeByte`

```kestrel
public func writeByte[W](mutating W, UInt8) -> Result[(), IoError] where W: Write
```

Writes a single byte, looping internally until it lands.

_Defined in `lang/std/io/write.ks`._

## function `writeLine`

```kestrel
public func writeLine[W](mutating W, String) -> Result[(), IoError] where W: Write
```

Writes `s` followed by a single `\n`. Does not append `\r` on any
platform — Kestrel writes Unix line endings everywhere by default.

_Defined in `lang/std/io/write.ks`._

## function `writeStr`

```kestrel
public func writeStr[W](mutating W, String) -> Result[(), IoError] where W: Write
```

Writes the UTF-8 encoding of `s`. Empty strings short-circuit. Currently
emits one byte per call into the writer — fine for buffered sinks like
`Buffer`, expensive for raw `File`/`Stdout` (TODO: collect into a slice
first).

_Defined in `lang/std/io/write.ks`._

