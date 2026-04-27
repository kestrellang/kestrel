# std.io.libc

## typealias `Fd`

```kestrel
public type Fd = Int32
```

File descriptor type (wraps an int).

_Defined in `lang/std/io/libc.ks`._

## function `MODE_DEFAULT`

```kestrel
public func MODE_DEFAULT() -> Int32
```

Default file mode (rw-r--r-- = 0o644 = 420 decimal).

_Defined in `lang/std/io/libc.ks`._

## function `O_APPEND`

```kestrel
public func O_APPEND() -> Int32
```

Append to end of file.

_Defined in `lang/std/io/libc.ks`._

## function `O_CREAT`

```kestrel
public func O_CREAT() -> Int32
```

_Defined in `lang/std/io/libc.ks`._

## function `O_EXCL`

```kestrel
public func O_EXCL() -> Int32
```

Fail if file exists (with O_CREAT).

_Defined in `lang/std/io/libc.ks`._

## function `O_RDONLY`

```kestrel
public func O_RDONLY() -> Int32
```

Open for reading only.

_Defined in `lang/std/io/libc.ks`._

## function `O_RDWR`

```kestrel
public func O_RDWR() -> Int32
```

Open for reading and writing.

_Defined in `lang/std/io/libc.ks`._

## function `O_TRUNC`

```kestrel
public func O_TRUNC() -> Int32
```

Truncate file to zero length.

_Defined in `lang/std/io/libc.ks`._

## function `O_WRONLY`

```kestrel
public func O_WRONLY() -> Int32
```

Open for writing only.

_Defined in `lang/std/io/libc.ks`._

## function `SEEK_CUR`

```kestrel
public func SEEK_CUR() -> Int32
```

Seek from current position.

_Defined in `lang/std/io/libc.ks`._

## function `SEEK_END`

```kestrel
public func SEEK_END() -> Int32
```

Seek from end of file.

_Defined in `lang/std/io/libc.ks`._

## function `SEEK_SET`

```kestrel
public func SEEK_SET() -> Int32
```

Seek from beginning of file.

_Defined in `lang/std/io/libc.ks`._

## function `STDERR`

```kestrel
public func STDERR() -> Fd
```

Standard error file descriptor.

_Defined in `lang/std/io/libc.ks`._

## function `STDIN`

```kestrel
public func STDIN() -> Fd
```

Standard input file descriptor.

_Defined in `lang/std/io/libc.ks`._

## function `STDOUT`

```kestrel
public func STDOUT() -> Fd
```

Standard output file descriptor.

_Defined in `lang/std/io/libc.ks`._

## function `close`

```kestrel
public func close(Int32) -> Int32
```

Closes a file descriptor. Returns 0 on success, -1 on error.

_Defined in `lang/std/io/libc.ks`._

## function `errno`

```kestrel
public func errno() -> Int32
```

Returns the current errno value.

_Defined in `lang/std/io/libc.ks`._

## function `lseek`

```kestrel
public func lseek(Int32, Int64, Int32) -> Int64
```

Seeks to a position in a file. Returns new position or -1 on error.

_Defined in `lang/std/io/libc.ks`._

## function `open`

```kestrel
public func open(Pointer[UInt8], Int32, Int32) -> Fd
```

Opens a file. Returns file descriptor or -1 on error.

_Defined in `lang/std/io/libc.ks`._

## function `read`

```kestrel
public func read(Int32, Pointer[UInt8], Int64) -> Int64
```

Reads from a file descriptor. Returns bytes read, 0 on EOF, -1 on error.

_Defined in `lang/std/io/libc.ks`._

## function `write`

```kestrel
public func write(Int32, Pointer[UInt8], Int64) -> Int64
```

Writes to a file descriptor. Returns bytes written or -1 on error.

_Defined in `lang/std/io/libc.ks`._

