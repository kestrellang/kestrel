# std.streams

## struct `Buffer`

```kestrel
public struct Buffer { /* private fields */ }
```

Collects written bytes in memory.

### Examples

```
var b = Buffer();
try b.write(text: "hi");
b.toText()           // .Some("hi")
```

_Defined in `lang/std/streams/memory.ks`._

### Members

#### initializer `Empty`

```kestrel
public init()
```

An empty buffer.

_Defined in `lang/std/streams/memory.ks`._

#### initializer `With Capacity`

```kestrel
public init(capacity: Int64)
```

An empty buffer with room for `capacity` bytes before it grows.

_Defined in `lang/std/streams/memory.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> &Slice[UInt8]
```

The written bytes, borrowed from the buffer.

_Defined in `lang/std/streams/memory.ks`._

#### function `clear`

```kestrel
public mutating func clear()
```

Drops the bytes and keeps the capacity.

_Defined in `lang/std/streams/memory.ks`._

#### field `count`

```kestrel
public var count: Int64 { get }
```

Bytes written so far.

_Defined in `lang/std/streams/memory.ks`._

#### field `data`

```kestrel
var data: Array[UInt8]
```

_Defined in `lang/std/streams/memory.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` until something is written.

_Defined in `lang/std/streams/memory.ks`._

#### function `toArray`

```kestrel
public consuming func toArray() -> Array[UInt8]
```

The written bytes, moved out.

_Defined in `lang/std/streams/memory.ks`._

#### function `toText`

```kestrel
public func toText() -> Optional[String]
```

The written bytes as text, or `None` if they aren't valid UTF-8.

_Defined in `lang/std/streams/memory.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Nothing to flush.

_Defined in `lang/std/streams/memory.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Appends all of `buf`.

_Defined in `lang/std/streams/memory.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Buffer
```

Copies the bytes.

_Defined in `lang/std/streams/memory.ks`._

## struct `BufferedReader`

```kestrel
public struct BufferedReader[R] where R: Readable, R: not Copyable, R: not Static { /* private fields */ }
```

Reads ahead from another reader in large chunks, so small reads and line
reading don't cost a system call each.

### Examples

```
var input = Cursor(text: "a\nb\n").buffered();
try input.readLine()     // .Some("a")
```

_Defined in `lang/std/streams/buffered.ks`._

### Members

#### initializer `From Reader`

```kestrel
public init(consuming R, capacity: Int64)
```

Buffers `inner` with `capacity` bytes of read-ahead.

_Defined in `lang/std/streams/buffered.ks`._

#### field `buffer`

```kestrel
var buffer: Array[UInt8]
```

_Defined in `lang/std/streams/buffered.ks`._

#### field `buffered`

```kestrel
public var buffered: Int64 { get }
```

Bytes read ahead and not yet handed out.

_Defined in `lang/std/streams/buffered.ks`._

#### field `end`

```kestrel
var end: Int64
```

_Defined in `lang/std/streams/buffered.ks`._

#### function `fill`

```kestrel
internal mutating func fill() -> Result[Bool, IoError]
```

Refills the buffer if it's empty. `false` at end of stream.

_Defined in `lang/std/streams/buffered.ks`._

#### field `inner`

```kestrel
var inner: R
```

_Defined in `lang/std/streams/buffered.ks`._

#### function `intoInner`

```kestrel
public consuming func intoInner() -> R
```

The inner reader. Bytes already read ahead are lost.

_Defined in `lang/std/streams/buffered.ks`._

#### function `lines`

```kestrel
public consuming func lines() -> Lines[R]
```

An iterator over the remaining lines.

_Defined in `lang/std/streams/buffered.ks`._

#### function `readLine`

```kestrel
public mutating func readLine() -> Result[Optional[String], IoError]
```

The next line without its `"\n"` (or `"\r\n"`); `None` at end of
stream. Invalid UTF-8 becomes U+FFFD.

_Defined in `lang/std/streams/buffered.ks`._

#### function `readUntil`

```kestrel
public mutating func readUntil(byte: UInt8, into: mutating Array[UInt8]) -> Result[Int64, IoError]
```

Reads up to and including the next `byte`, appending to `buf`.
Returns the bytes appended; `0` at end of stream.

_Defined in `lang/std/streams/buffered.ks`._

#### field `start`

```kestrel
var start: Int64
```

Unread bytes are `buffer(start..<end)`.

_Defined in `lang/std/streams/buffered.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Hands out buffered bytes first. A read at least as large as the
buffer, with nothing buffered, goes straight to the inner reader.

_Defined in `lang/std/streams/buffered.ks`._

### Implements `Seekable`

_Conditional: `where R: Seekable`._

#### function `seek`

```kestrel
public mutating func seek(to: Seek) -> Result[Int64, IoError] where R: Seekable
```

Drops the read-ahead, then seeks the inner reader. A `.Current`
offset counts from the position the caller sees, not the inner one.

_Defined in `lang/std/streams/buffered.ks`._

## struct `BufferedWriter`

```kestrel
public struct BufferedWriter[W]: not Copyable where W: Writable, W: not Copyable, W: not Static { /* private fields */ }
```

Batches small writes into large ones on another writer.

Dropping it flushes, but an error from that flush is lost: call
`flush()` (or `intoInner()`) to see it.

### Examples

```
var out = Buffer().buffered();
try out.write(line: "hello");
try out.flush();
```

_Defined in `lang/std/streams/buffered.ks`._

### Members

#### initializer `From Writer`

```kestrel
public init(consuming W, capacity: Int64)
```

Buffers up to `capacity` bytes before writing to `inner`.

_Defined in `lang/std/streams/buffered.ks`._

#### field `buffer`

```kestrel
var buffer: Array[UInt8]
```

_Defined in `lang/std/streams/buffered.ks`._

#### field `buffered`

```kestrel
public var buffered: Int64 { get }
```

Bytes waiting to be written.

_Defined in `lang/std/streams/buffered.ks`._

#### field `capacity`

```kestrel
var capacity: Int64
```

_Defined in `lang/std/streams/buffered.ks`._

#### function `drain`

```kestrel
internal mutating func drain() -> Result[(), IoError]
```

Writes the buffered bytes to the inner writer, without flushing it.

_Defined in `lang/std/streams/buffered.ks`._

#### field `inner`

```kestrel
var inner: Optional[W]
```

_Defined in `lang/std/streams/buffered.ks`._

#### function `intoInner`

```kestrel
public consuming func intoInner() -> Result[W, IoError]
```

Flushes, then returns the inner writer.

_Defined in `lang/std/streams/buffered.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Writes the buffer out and flushes the inner writer.

_Defined in `lang/std/streams/buffered.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Buffers `buf`, writing the buffer out first if `buf` won't fit. A
write at least as large as the buffer goes straight through.

_Defined in `lang/std/streams/buffered.ks`._

### Implements `Seekable`

_Conditional: `where W: Seekable`._

#### function `seek`

```kestrel
public mutating func seek(to: Seek) -> Result[Int64, IoError] where W: Seekable
```

Writes the buffer out, then seeks the inner writer.

_Defined in `lang/std/streams/buffered.ks`._

## struct `ChainReader`

```kestrel
public struct ChainReader[A, B] where A: Readable, B: Readable, A: not Copyable, B: not Copyable, A: not Static, B: not Static { /* private fields */ }
```

Reads one reader to its end, then another. Built by
`first.chained(with: second)`.

_Defined in `lang/std/streams/readable.ks`._

### Members

#### initializer `From Readers`

```kestrel
public init(first: consuming A, second: consuming B)
```

Reads `first`, then `second`. Prefer `first.chained(with: second)`.

_Defined in `lang/std/streams/readable.ks`._

#### field `first`

```kestrel
var first: A
```

_Defined in `lang/std/streams/readable.ks`._

#### field `firstDone`

```kestrel
var firstDone: Bool
```

_Defined in `lang/std/streams/readable.ks`._

#### field `second`

```kestrel
var second: B
```

_Defined in `lang/std/streams/readable.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads from the first reader until it ends, then from the second.

_Defined in `lang/std/streams/readable.ks`._

## struct `Cursor`

```kestrel
public struct Cursor { /* private fields */ }
```

Reads from bytes it owns, with a position that can move. Use it to hand
an in-memory byte array or string to code that takes a `Readable`.

### Examples

```
var c = Cursor(text: "abc");
try c.readByte()     // .Some(97)
c.position           // 1
```

_Defined in `lang/std/streams/memory.ks`._

### Members

#### initializer `From Bytes`

```kestrel
public init(bytes: Array[UInt8])
```

Reads `bytes`, starting at the first.

_Defined in `lang/std/streams/memory.ks`._

#### initializer `From Text`

```kestrel
public init(text: String)
```

Reads the UTF-8 bytes of `text`, starting at the first.

_Defined in `lang/std/streams/memory.ks`._

#### function `asSlice`

```kestrel
public func asSlice() -> &Slice[UInt8]
```

The bytes being read, all of them.

_Defined in `lang/std/streams/memory.ks`._

#### field `data`

```kestrel
var data: Array[UInt8]
```

_Defined in `lang/std/streams/memory.ks`._

#### field `offset`

```kestrel
var offset: Int64
```

_Defined in `lang/std/streams/memory.ks`._

#### field `position`

```kestrel
public var position: Int64 { get set }
```

The offset of the next byte to read. Setting it past the end makes
reads return end of stream; it never fails, unlike `seek(to:)`.

_Defined in `lang/std/streams/memory.ks`._

#### field `remaining`

```kestrel
public var remaining: Int64 { get }
```

Bytes left to read.

_Defined in `lang/std/streams/memory.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Copies from the current position and advances it.

_Defined in `lang/std/streams/memory.ks`._

### Implements `Seekable`

#### function `seek`

```kestrel
public mutating func seek(to: Seek) -> Result[Int64, IoError]
```

Moves the position. Past the end is allowed; before the start is
`InvalidInput`.

_Defined in `lang/std/streams/memory.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Cursor
```

Copies the bytes and the position.

_Defined in `lang/std/streams/memory.ks`._

## struct `Empty`

```kestrel
public struct Empty { /* private fields */ }
```

Always at end of stream.

_Defined in `lang/std/streams/memory.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

_Defined in `lang/std/streams/memory.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads nothing.

_Defined in `lang/std/streams/memory.ks`._

## struct `FileDescriptor`

```kestrel
public struct FileDescriptor { /* private fields */ }
```

The system's number for an open file, pipe or socket.

Holding one doesn't own anything: whoever handed it out (a `File`, a
`TcpStream`) still closes it. Use it to pass a handle to C code or an
event loop.

_Defined in `lang/std/streams/descriptor.ks`._

### Members

#### initializer `From Number`

```kestrel
public init(Int32)
```

Wraps a raw descriptor number.

_Defined in `lang/std/streams/descriptor.ks`._

#### field `value`

```kestrel
public var value: Int32
```

The raw number.

_Defined in `lang/std/streams/descriptor.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: FileDescriptor) -> Bool
```

Same number.

_Defined in `lang/std/streams/descriptor.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the number.

_Defined in `lang/std/streams/descriptor.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

Writes the number.

_Defined in `lang/std/streams/descriptor.ks`._

## struct `IoError`

```kestrel
public struct IoError { /* private fields */ }
```

A failed I/O call: its `kind`, plus the system error code when the
system reported one.

### Examples

```
match File.open("data.bin") {
    .Ok(file) => use(file),
    .Err(e) => eprintln("can't open data.bin: \(e)")
}
```

_Defined in `lang/std/streams/error.ks`._

### Members

#### initializer `From Errno`

```kestrel
public init(errno: Int32)
```

The error for a system error code.

_Defined in `lang/std/streams/error.ks`._

#### initializer `From Kind`

```kestrel
public init(kind: IoErrorKind)
```

An error of `kind` with no system error code.

_Defined in `lang/std/streams/error.ks`._

#### field `alreadyExists`

```kestrel
public static var alreadyExists: IoError { get }
```

`AlreadyExists`.

_Defined in `lang/std/streams/error.ks`._

#### field `brokenPipe`

```kestrel
public static var brokenPipe: IoError { get }
```

`BrokenPipe`.

_Defined in `lang/std/streams/error.ks`._

#### field `code`

```kestrel
var code: Int32
```

The system error code, or -1 for an error std made itself.

_Defined in `lang/std/streams/error.ks`._

#### function `description`

```kestrel
public func description() -> String
```

A short message: the kind's description, or the system's message
for an error of kind `.Other`.

_Defined in `lang/std/streams/error.ks`._

#### field `errno`

```kestrel
public var errno: Optional[Int32] { get }
```

The system error code, if the system reported this error.

_Defined in `lang/std/streams/error.ks`._

#### field `interrupted`

```kestrel
public static var interrupted: IoError { get }
```

`Interrupted`.

_Defined in `lang/std/streams/error.ks`._

#### field `invalidData`

```kestrel
public static var invalidData: IoError { get }
```

`InvalidData`.

_Defined in `lang/std/streams/error.ks`._

#### field `invalidInput`

```kestrel
public static var invalidInput: IoError { get }
```

`InvalidInput`.

_Defined in `lang/std/streams/error.ks`._

#### field `kind`

```kestrel
public var kind: IoErrorKind
```

What kind of failure this is.

_Defined in `lang/std/streams/error.ks`._

#### function `last`

```kestrel
public static func last() -> IoError
```

The error for the calling thread's current `errno`. Call it right
after the failing system call, before anything else can change it.

_Defined in `lang/std/streams/error.ks`._

#### field `notFound`

```kestrel
public static var notFound: IoError { get }
```

`NotFound`.

_Defined in `lang/std/streams/error.ks`._

#### field `permissionDenied`

```kestrel
public static var permissionDenied: IoError { get }
```

`PermissionDenied`.

_Defined in `lang/std/streams/error.ks`._

#### field `timedOut`

```kestrel
public static var timedOut: IoError { get }
```

`TimedOut`.

_Defined in `lang/std/streams/error.ks`._

#### field `unexpectedEnd`

```kestrel
public static var unexpectedEnd: IoError { get }
```

`UnexpectedEnd`.

_Defined in `lang/std/streams/error.ks`._

#### field `unsupported`

```kestrel
public static var unsupported: IoError { get }
```

`Unsupported`.

_Defined in `lang/std/streams/error.ks`._

#### field `wouldBlock`

```kestrel
public static var wouldBlock: IoError { get }
```

`WouldBlock`.

_Defined in `lang/std/streams/error.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: IoError) -> Bool
```

Same kind and same system error code.

_Defined in `lang/std/streams/error.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

Writes `description()`.

_Defined in `lang/std/streams/error.ks`._

## enum `IoErrorKind`

```kestrel
public enum IoErrorKind
```

What kind of failure an `IoError` is. Match on it to handle the cases
that matter; everything unclassified is `.Other`, and `IoError.errno`
still has the system's code.

### Examples

```
match readText("config.toml") {
    .Ok(text) => parse(text),
    .Err(e) if e.kind == .NotFound => defaults(),
    .Err(e) => fail(e)
}
```

_Defined in `lang/std/streams/error.ks`._

### Members

#### case `AddressInUse`

```kestrel
case AddressInUse
```

The address is already in use.

_Defined in `lang/std/streams/error.ks`._

#### case `AddressNotAvailable`

```kestrel
case AddressNotAvailable
```

The address isn't available on this machine.

_Defined in `lang/std/streams/error.ks`._

#### case `AlreadyExists`

```kestrel
case AlreadyExists
```

The path already exists.

_Defined in `lang/std/streams/error.ks`._

#### case `BadFileDescriptor`

```kestrel
case BadFileDescriptor
```

The descriptor is closed or invalid.

_Defined in `lang/std/streams/error.ks`._

#### case `BrokenPipe`

```kestrel
case BrokenPipe
```

The other end of a pipe or socket was closed.

_Defined in `lang/std/streams/error.ks`._

#### case `ConnectionAborted`

```kestrel
case ConnectionAborted
```

The connection was aborted on this side.

_Defined in `lang/std/streams/error.ks`._

#### case `ConnectionRefused`

```kestrel
case ConnectionRefused
```

The remote host refused the connection.

_Defined in `lang/std/streams/error.ks`._

#### case `ConnectionReset`

```kestrel
case ConnectionReset
```

The remote host reset the connection.

_Defined in `lang/std/streams/error.ks`._

#### case `CrossesDevices`

```kestrel
case CrossesDevices
```

A rename or link would cross file systems.

_Defined in `lang/std/streams/error.ks`._

#### case `DirectoryNotEmpty`

```kestrel
case DirectoryNotEmpty
```

The directory isn't empty.

_Defined in `lang/std/streams/error.ks`._

#### case `HostNotFound`

```kestrel
case HostNotFound
```

The host name couldn't be resolved.

_Defined in `lang/std/streams/error.ks`._

#### case `Interrupted`

```kestrel
case Interrupted
```

A signal interrupted the call; it can be retried.

_Defined in `lang/std/streams/error.ks`._

#### case `InvalidData`

```kestrel
case InvalidData
```

The bytes weren't in the expected format, such as UTF-8.

_Defined in `lang/std/streams/error.ks`._

#### case `InvalidInput`

```kestrel
case InvalidInput
```

An argument was invalid.

_Defined in `lang/std/streams/error.ks`._

#### case `IoFailure`

```kestrel
case IoFailure
```

The device reported a low-level I/O failure.

_Defined in `lang/std/streams/error.ks`._

#### case `IsADirectory`

```kestrel
case IsADirectory
```

A file was expected, but the path is a directory.

_Defined in `lang/std/streams/error.ks`._

#### case `NoSpaceLeft`

```kestrel
case NoSpaceLeft
```

The device is full.

_Defined in `lang/std/streams/error.ks`._

#### case `NotADirectory`

```kestrel
case NotADirectory
```

A path component isn't a directory.

_Defined in `lang/std/streams/error.ks`._

#### case `NotConnected`

```kestrel
case NotConnected
```

The socket isn't connected.

_Defined in `lang/std/streams/error.ks`._

#### case `NotFound`

```kestrel
case NotFound
```

The file or directory doesn't exist.

_Defined in `lang/std/streams/error.ks`._

#### case `NotPermitted`

```kestrel
case NotPermitted
```

The operation isn't permitted.

_Defined in `lang/std/streams/error.ks`._

#### case `Other`

```kestrel
case Other
```

Not one of the kinds below; see `IoError.errno`.

_Defined in `lang/std/streams/error.ks`._

#### case `OutOfMemory`

```kestrel
case OutOfMemory
```

Memory ran out.

_Defined in `lang/std/streams/error.ks`._

#### case `PermissionDenied`

```kestrel
case PermissionDenied
```

The process lacks permission.

_Defined in `lang/std/streams/error.ks`._

#### case `TimedOut`

```kestrel
case TimedOut
```

The operation ran out of time.

_Defined in `lang/std/streams/error.ks`._

#### case `UnexpectedEnd`

```kestrel
case UnexpectedEnd
```

The stream ended before the expected number of bytes.

_Defined in `lang/std/streams/error.ks`._

#### case `Unsupported`

```kestrel
case Unsupported
```

The system doesn't support the operation.

_Defined in `lang/std/streams/error.ks`._

#### case `WouldBlock`

```kestrel
case WouldBlock
```

A non-blocking call would have had to wait.

_Defined in `lang/std/streams/error.ks`._

#### function `description`

```kestrel
public func description() -> String
```

A short lowercase phrase, the same on every platform.

_Defined in `lang/std/streams/error.ks`._

#### function `of`

```kestrel
public static func of(errno: Int32) -> IoErrorKind
```

The kind for a system error code.

_Defined in `lang/std/streams/error.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: IoErrorKind) -> Bool
```

Same kind.

_Defined in `lang/std/streams/error.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

Writes `description()`.

_Defined in `lang/std/streams/error.ks`._

## struct `Lines`

```kestrel
public struct Lines[R] where R: Readable, R: not Copyable, R: not Static { /* private fields */ }
```

The lines of a `BufferedReader`, each as `Result[String, IoError]`.
Ends at end of stream or after the first error.

_Defined in `lang/std/streams/buffered.ks`._

### Members

#### initializer `From Reader`

```kestrel
public init(reader: consuming BufferedReader[R])
```

Prefer `reader.lines()`.

_Defined in `lang/std/streams/buffered.ks`._

#### field `done`

```kestrel
var done: Bool
```

_Defined in `lang/std/streams/buffered.ks`._

#### field `reader`

```kestrel
var reader: BufferedReader[R]
```

_Defined in `lang/std/streams/buffered.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Result[String, IoError]
```

_Defined in `lang/std/streams/buffered.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[Result[String, IoError]]
```

The next line, or the error that ended the iteration.

_Defined in `lang/std/streams/buffered.ks`._

## protocol `Readable`

```kestrel
public protocol Readable
```

A source of bytes.

One `read(into:)` moves up to `buf.count` bytes into `buf` and returns
how many it moved. `0` means end of stream. A short read is not an
error: loop, or use `readExact` / `readAll` when the shape matters.

### Examples

```
struct Zeros: Readable {
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        buf.fill(with: 0);
        .Ok(buf.count)
    }
}
```

_Defined in `lang/std/streams/readable.ks`._

### Members

#### function `buffered`

```kestrel
public consuming func buffered() -> BufferedReader[Self]
```

Wraps the stream in a `BufferedReader`, for line reading and fewer
system calls.

_Defined in `lang/std/streams/readable.ks`._

#### function `chained`

```kestrel
public consuming func chained[R](with: consuming R) -> ChainReader[Self, R] where R: Readable, R: not Copyable, R: not Static
```

A reader that reads this one to its end, then `next`.

_Defined in `lang/std/streams/readable.ks`._

#### function `read`

```kestrel
mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads up to `buf.count` bytes into `buf`; `0` at end of stream.

_Defined in `lang/std/streams/readable.ks`._

#### function `readAll`

```kestrel
public mutating func readAll(into: mutating Array[UInt8]) -> Result[Int64, IoError]
```

Reads to end of stream, appending to `buf`. Returns the number of
bytes appended. Retries reads interrupted by a signal.

_Defined in `lang/std/streams/readable.ks`._

#### function `readByte`

```kestrel
public mutating func readByte() -> Result[Optional[UInt8], IoError]
```

Reads one byte; `None` at end of stream.

_Defined in `lang/std/streams/readable.ks`._

#### function `readExact`

```kestrel
public mutating func readExact(into: mutating Slice[UInt8]) -> Result[(), IoError]
```

Fills all of `buf`. A stream that ends first is an error, not a
short read: use it for fixed-size records and headers.

##### Errors

`UnexpectedEnd` if the stream ends before `buf` is full.

_Defined in `lang/std/streams/readable.ks`._

#### function `readText`

```kestrel
public mutating func readText() -> Result[String, IoError]
```

Reads to end of stream as UTF-8 text.

##### Errors

`InvalidData` if the bytes aren't valid UTF-8.

_Defined in `lang/std/streams/readable.ks`._

#### function `take`

```kestrel
public consuming func take(bytes: Int64) -> TakeReader[Self]
```

A reader that stops after `bytes` bytes of this one.

_Defined in `lang/std/streams/readable.ks`._

## struct `Repeat`

```kestrel
public struct Repeat { /* private fields */ }
```

The same byte forever, like `/dev/zero` for `byte: 0`.

_Defined in `lang/std/streams/memory.ks`._

### Members

#### initializer `From Byte`

```kestrel
public init(byte: UInt8)
```

Reads `byte`, endlessly.

_Defined in `lang/std/streams/memory.ks`._

#### field `byte`

```kestrel
var byte: UInt8
```

_Defined in `lang/std/streams/memory.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Fills all of `buf`.

_Defined in `lang/std/streams/memory.ks`._

## enum `Seek`

```kestrel
public enum Seek
```

Where `seek(to:)` moves a stream.

_Defined in `lang/std/streams/seekable.ks`._

### Members

#### case `Current`

```kestrel
case Current(Int64)
```

This many bytes from the current position; negative moves back.

_Defined in `lang/std/streams/seekable.ks`._

#### case `End`

```kestrel
case End(Int64)
```

This many bytes from the end; usually `0` or negative.

_Defined in `lang/std/streams/seekable.ks`._

#### case `Start`

```kestrel
case Start(Int64)
```

This many bytes from the start.

_Defined in `lang/std/streams/seekable.ks`._

## protocol `Seekable`

```kestrel
public protocol Seekable
```

A stream whose position can move: files and in-memory cursors, but not
pipes, sockets or the standard streams.

It is separate from `Readable` and `Writable` because most streams
can't seek. Code that needs to says so: `where R: Readable, R: Seekable`.

### Examples

```
var c = Cursor(text: "hello");
try c.seek(to: .End(-2));
try c.readText()      // "lo"
```

_Defined in `lang/std/streams/seekable.ks`._

### Members

#### function `currentPosition`

```kestrel
public mutating func currentPosition() -> Result[Int64, IoError]
```

The offset from the start.

_Defined in `lang/std/streams/seekable.ks`._

#### function `length`

```kestrel
public mutating func length() -> Result[Int64, IoError]
```

The stream's length in bytes. Seeks to the end and back, so the
position is unchanged.

_Defined in `lang/std/streams/seekable.ks`._

#### function `rewind`

```kestrel
public mutating func rewind() -> Result[(), IoError]
```

Moves back to the start.

_Defined in `lang/std/streams/seekable.ks`._

#### function `seek`

```kestrel
mutating func seek(to: Seek) -> Result[Int64, IoError]
```

Moves to `position`; returns the new offset from the start.

##### Errors

`InvalidInput` if the position would be before the start.

_Defined in `lang/std/streams/seekable.ks`._

## struct `Sink`

```kestrel
public struct Sink { /* private fields */ }
```

Accepts and discards every write, like `/dev/null`.

_Defined in `lang/std/streams/memory.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

_Defined in `lang/std/streams/memory.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Nothing to flush.

_Defined in `lang/std/streams/memory.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Reports all of `buf` written.

_Defined in `lang/std/streams/memory.ks`._

## struct `StandardError`

```kestrel
public struct StandardError { /* private fields */ }
```

The process's standard error. Unbuffered, so diagnostics appear at once.

_Defined in `lang/std/streams/standard.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

_Defined in `lang/std/streams/standard.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Nothing to flush.

_Defined in `lang/std/streams/standard.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Writes `buf` to fd 2.

_Defined in `lang/std/streams/standard.ks`._

## struct `StandardInput`

```kestrel
public struct StandardInput { /* private fields */ }
```

The process's standard input. Every value reads through one shared
buffer, so lines read through one value aren't lost to another.

_Defined in `lang/std/streams/standard.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

_Defined in `lang/std/streams/standard.ks`._

#### function `readLine`

```kestrel
public mutating func readLine() -> Result[Optional[String], IoError]
```

The next line without its `"\n"` (or `"\r\n"`); `None` at end of
input. Invalid UTF-8 becomes U+FFFD.

_Defined in `lang/std/streams/standard.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads from the shared buffer, refilling it from fd 0.

_Defined in `lang/std/streams/standard.ks`._

## struct `StandardOutput`

```kestrel
public struct StandardOutput { /* private fields */ }
```

The process's standard output: line-buffered on a terminal,
block-buffered otherwise. The buffer is shared by every value and
flushed when the process exits normally.

_Defined in `lang/std/streams/standard.ks`._

### Members

#### initializer `Default`

```kestrel
public init()
```

_Defined in `lang/std/streams/standard.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Writes the shared buffer to fd 1.

_Defined in `lang/std/streams/standard.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Adds `buf` to the shared buffer.

_Defined in `lang/std/streams/standard.ks`._

## struct `TakeReader`

```kestrel
public struct TakeReader[R] where R: Readable, R: not Copyable, R: not Static { /* private fields */ }
```

Reads at most a fixed number of bytes from another reader, then reports
end of stream. Built by `reader.take(bytes:)`.

_Defined in `lang/std/streams/readable.ks`._

### Members

#### initializer `From Reader`

```kestrel
public init(inner: consuming R, bytes: Int64)
```

Limits `inner` to `bytes` more bytes. Prefer `inner.take(bytes:)`.

_Defined in `lang/std/streams/readable.ks`._

#### field `inner`

```kestrel
var inner: R
```

_Defined in `lang/std/streams/readable.ks`._

#### function `intoInner`

```kestrel
public consuming func intoInner() -> R
```

The inner reader, positioned after what was read through this one.

_Defined in `lang/std/streams/readable.ks`._

#### field `left`

```kestrel
var left: Int64
```

_Defined in `lang/std/streams/readable.ks`._

#### field `limit`

```kestrel
public var limit: Int64 { get }
```

Bytes still allowed before end of stream.

_Defined in `lang/std/streams/readable.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads from the inner reader, never past the limit.

_Defined in `lang/std/streams/readable.ks`._

## protocol `Writable`

```kestrel
public protocol Writable
```

A destination for bytes.

One `write(from:)` moves up to `buf.count` bytes and returns how many it
moved; a short write is not an error, and `writeAll` loops until
everything is written. `flush` pushes out whatever the writer buffered.

### Examples

```
struct Counter: Writable {
    var total: Int64
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        self.total = self.total + buf.count;
        .Ok(buf.count)
    }
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }
}
```

_Defined in `lang/std/streams/writable.ks`._

### Members

#### function `buffered`

```kestrel
public consuming func buffered() -> BufferedWriter[Self]
```

Wraps the writer in a `BufferedWriter`, so small writes are batched
into fewer system calls.

_Defined in `lang/std/streams/writable.ks`._

#### function `flush`

```kestrel
mutating func flush() -> Result[(), IoError]
```

Pushes buffered bytes to the destination. Unbuffered writers return
`.Ok(())`. An error here can be one deferred from an earlier write.

_Defined in `lang/std/streams/writable.ks`._

#### function `write`

```kestrel
mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Writes up to `buf.count` bytes; returns how many moved.

_Defined in `lang/std/streams/writable.ks`._

#### function `write`

```kestrel
public mutating func write(byte: UInt8) -> Result[(), IoError]
```

Writes one byte.

_Defined in `lang/std/streams/writable.ks`._

#### function `write`

```kestrel
public mutating func write(text: String) -> Result[(), IoError]
```

Writes the UTF-8 bytes of `text`.

_Defined in `lang/std/streams/writable.ks`._

#### function `write`

```kestrel
public mutating func write(line: String) -> Result[(), IoError]
```

Writes `text` and a `"\n"`.

_Defined in `lang/std/streams/writable.ks`._

#### function `write`

```kestrel
public mutating func write(formatted: some Formattable) -> Result[(), IoError]
```

Writes `value` formatted with its default options.

_Defined in `lang/std/streams/writable.ks`._

#### function `writeAll`

```kestrel
public mutating func writeAll(from: Slice[UInt8]) -> Result[(), IoError]
```

Writes all of `buf`, looping over short writes and retrying writes
interrupted by a signal.

##### Errors

`BrokenPipe` if the writer accepts zero bytes before `buf` is done.

_Defined in `lang/std/streams/writable.ks`._

## function `copy`

```kestrel
public func copy[R, W](from: mutating R, to: mutating W) -> Result[Int64, IoError] where R: Readable, R: not Copyable, R: not Static, W: Writable, W: not Copyable, W: not Static
```

Reads `reader` to its end and writes everything to `writer`. Returns
the number of bytes copied. Doesn't flush `writer`.

### Examples

```
var out = Buffer();
try copy(from: Cursor(text: "hi"), to: out);
```

_Defined in `lang/std/streams/copy.ks`._

## function `eprint`

```kestrel
public func eprint(some Formattable) -> Result[(), IoError]
```

Writes `value` to standard error.

_Defined in `lang/std/streams/standard.ks`._

## function `eprintln`

```kestrel
public func eprintln(some Formattable) -> Result[(), IoError]
```

Writes `value` and a `"\n"` to standard error.

_Defined in `lang/std/streams/standard.ks`._

## function `print`

```kestrel
public func print(some Formattable) -> Result[(), IoError]
```

Writes `value` to standard output.

_Defined in `lang/std/streams/standard.ks`._

## function `println`

```kestrel
public func println(some Formattable) -> Result[(), IoError]
```

Writes `value` and a `"\n"` to standard output.

_Defined in `lang/std/streams/standard.ks`._

## function `println`

```kestrel
public func println() -> Result[(), IoError]
```

Writes a `"\n"` to standard output.

_Defined in `lang/std/streams/standard.ks`._

## function `prompt`

```kestrel
public func prompt(String) -> Result[Optional[String], IoError]
```

Writes `message`, flushes standard output so it shows before the user
types, then reads a line.

_Defined in `lang/std/streams/standard.ks`._

## function `readLine`

```kestrel
public func readLine() -> Result[Optional[String], IoError]
```

Reads a line from standard input. `None` at end of input.

_Defined in `lang/std/streams/standard.ks`._

## function `standardError`

```kestrel
public func standardError() -> StandardError
```

The process's standard error.

_Defined in `lang/std/streams/standard.ks`._

## function `standardInput`

```kestrel
public func standardInput() -> StandardInput
```

The process's standard input.

_Defined in `lang/std/streams/standard.ks`._

## function `standardOutput`

```kestrel
public func standardOutput() -> StandardOutput
```

The process's standard output.

_Defined in `lang/std/streams/standard.ks`._

