# std.io.file

## struct `File`

```kestrel
public struct File { /* private fields */ }
```

RAII-owned POSIX file handle.

The wrapped file descriptor is closed automatically when the `File`
goes out of scope, so explicit `close` is never necessary. `File` is
`not Copyable` to keep the descriptor uniquely owned — pass by
reference or move it instead. Conforms to both `Read` and `Write`,
although calls fail with `EBADF` if the open mode does not permit the
direction (e.g. writing to a file opened with `open()`).

### Examples

```
// Read whole file in 4 KiB chunks.
var file = try File.open("input.txt");
var buf = Array[UInt8](repeating: 0, count: 4096);
while true {
    let n = try file.read(into: buf.asSlice());
    if n == 0 { break }
    // process buf[0..n]
}
```

### Representation

One `libc.Fd` (32-bit signed integer) field.

### Memory Model

Owning, unique. The `deinit` calls `close(fd)` if `fd >= 0`; close
errors are silently ignored — there's no caller to surface them to.

_Defined in `lang/std/io/file.ks`._

### Members

#### initializer `From Fd`

```kestrel
init(libc.Fd)
```

Internal init wrapping a raw descriptor; not for general use.

_Defined in `lang/std/io/file.ks`._

#### function `create`

```kestrel
public static func create(String) -> Result[File, IoError]
```

Creates (or truncates) `path` for writing with mode `0644`.
Existing contents are discarded.

##### Examples

```
var file = try File.create("output.txt");
try writeStr(file, "New content");
```

_Defined in `lang/std/io/file.ks`._

#### function `createNew`

```kestrel
public static func createNew(String) -> Result[File, IoError]
```

Creates a new file, failing if the path already exists. The
canonical pattern for cooperative locking via lockfiles.

##### Errors

Returns `Err` carrying `EEXIST` if the path already exists.

##### Examples

```
match File.createNew("lock.pid") {
    .Ok(f) => /* we hold the lock */ holdLock(f),
    .Err(e) => /* somebody else has it */ retryLater()
}
```

_Defined in `lang/std/io/file.ks`._

#### field `fd`

```kestrel
var fd: libc.Fd
```

_Defined in `lang/std/io/file.ks`._

#### function `open`

```kestrel
public static func open(String) -> Result[File, IoError]
```

Opens an existing file for reading. The file must exist; missing
paths surface as `Err(IoError.last())` carrying `ENOENT`, and
permission failures as `EACCES`.

_Defined in `lang/std/io/file.ks`._

#### function `openAppend`

```kestrel
public static func openAppend(String) -> Result[File, IoError]
```

Opens (or creates) a file in append mode. Every write atomically
lands at the current end of file regardless of where `seek` last
left the cursor — the standard idiom for log files and any
concurrent appender.

_Defined in `lang/std/io/file.ks`._

#### function `openReadWrite`

```kestrel
public static func openReadWrite(String) -> Result[File, IoError]
```

Opens an existing file for both reading and writing. Use for
in-place modification of a file that already exists; for "create
or open" semantics combine with `create` / `createNew` as
appropriate.

_Defined in `lang/std/io/file.ks`._

#### function `position`

```kestrel
public mutating func position() -> Result[Int64, IoError]
```

Convenience for `seek(.Current(0))`.

_Defined in `lang/std/io/file.ks`._

#### function `rawFd`

```kestrel
public func rawFd() -> libc.Fd
```

Returns the underlying libc file descriptor for direct FFI use.
Ownership stays with the `File`; do not call `close` on the
returned value or the `deinit` will hit `EBADF`.

_Defined in `lang/std/io/file.ks`._

#### function `rewind`

```kestrel
public mutating func rewind() -> Result[(), IoError]
```

Convenience for `seek(.Start(0))` that drops the returned offset.

_Defined in `lang/std/io/file.ks`._

#### function `seek`

```kestrel
public mutating func seek(to: Seek) -> Result[Int64, IoError]
```

Calls `lseek(2)` with the requested anchor and offset. Returns
the new absolute position from the start of the file. Seeking
past EOF is allowed; a subsequent write extends the file (with a
hole on filesystems that support sparse files).

##### Examples

```
var file = try File.openReadWrite("data.bin");
try file.seek(.Start(0));        // rewind
try file.seek(.Current(100));    // skip 100 bytes
let size = try file.seek(.End(0));   // size of file
```

_Defined in `lang/std/io/file.ks`._

### Implements `Read`

#### function `read`

```kestrel
public mutating func read(into: Slice[UInt8]) -> Result[Int64, IoError]
```

Calls `read(2)`. Advances the file position by the byte count
returned. Short reads (`n < buf.count`) are normal — keep calling
until `0` is returned (EOF) or an error fires. Use `readAll`/
`readExact` from `std.io.read` when looping by hand isn't wanted.

_Defined in `lang/std/io/file.ks`._

### Implements `Write`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op; `File` does no internal buffering. Reaches the kernel as
soon as `write` returns, but does not call `fsync` — durability
across power loss requires a separate, currently-unwrapped libc
call.

_Defined in `lang/std/io/file.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Calls `write(2)`. May write fewer bytes than supplied — wrap with
`writeAll` from `std.io.write` to loop until done.

_Defined in `lang/std/io/file.ks`._

## enum `Seek`

```kestrel
public enum Seek
```

Anchor + offset pair passed to `File.seek`. The three variants match
POSIX `SEEK_SET`, `SEEK_CUR`, and `SEEK_END`; the payload is the
offset in bytes (signed, so backwards seeks work).

### Examples

```
try file.seek(.Start(0));        // beginning
try file.seek(.Current(-10));    // back 10 bytes
try file.seek(.End(0));          // end of file
```

_Defined in `lang/std/io/file.ks`._

### Members

#### case `Current`

```kestrel
case Current(Int64)
```

Seek by `n` bytes from the current position. Negative values move
backwards.

_Defined in `lang/std/io/file.ks`._

#### case `End`

```kestrel
case End(Int64)
```

Seek by `n` bytes from EOF. Use `0` to land exactly at EOF;
negative values move backwards from the end.

_Defined in `lang/std/io/file.ks`._

#### case `Start`

```kestrel
case Start(Int64)
```

Seek to an absolute byte offset from the start of the file.

_Defined in `lang/std/io/file.ks`._

## function `appendFileBytes`

```kestrel
public func appendFileBytes(String, Array[UInt8]) -> Result[(), IoError]
```

Appends bytes to `path`, creating if absent. Binary counterpart to
`appendFileString`.

_Defined in `lang/std/io/file.ks`._

## function `appendFileString`

```kestrel
public func appendFileString(String, String) -> Result[(), IoError]
```

Appends `content` to `path` as UTF-8, creating the file if absent.
Atomic per-write under POSIX `O_APPEND` semantics — safe to call from
multiple writers without intermediate locking, though writes longer
than `PIPE_BUF` may interleave.

_Defined in `lang/std/io/file.ks`._

## function `readFileBytes`

```kestrel
public func readFileBytes(String) -> Result[Array[UInt8], IoError]
```

Reads `path` into an `Array[UInt8]`. The binary counterpart to
`readFileString` — does no UTF-8 decoding.

_Defined in `lang/std/io/file.ks`._

## function `readFileString`

```kestrel
public func readFileString(String) -> Result[String, IoError]
```

Reads `path` into a `String`, decoding the bytes as UTF-8. Convenient
for config files, source files, and other small/medium text. Slurps
the entire file into memory — for huge inputs prefer streaming via
`File` + `readAll`.

### Examples

```
let cfg = try readFileString("config.json");
```

_Defined in `lang/std/io/file.ks`._

## function `writeFileBytes`

```kestrel
public func writeFileBytes(String, Array[UInt8]) -> Result[(), IoError]
```

Writes `content` to `path`, creating or truncating as needed.
Binary equivalent of `writeFileString`.

_Defined in `lang/std/io/file.ks`._

## function `writeFileString`

```kestrel
public func writeFileString(String, String) -> Result[(), IoError]
```

Writes `content` to `path`, creating or truncating as needed. Bytes
are the UTF-8 encoding of the string. The mirror of `readFileString`.

_Defined in `lang/std/io/file.ks`._

