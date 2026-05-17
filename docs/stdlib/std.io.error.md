# std.io.error

## struct `IoError`

```kestrel
public struct IoError { /* private fields */ }
```

Structured I/O error: a classified `kind` plus the originating POSIX
errno.

Returned in the `Err` arm of every `Result` produced by `Read`, `Write`,
`File`, `stdio`, and the `os.fs` helpers. Pattern-match on `kind` for
programmatic dispatch; call `description()` for a short human-readable
phrase. The convenience constructors at the bottom (`notFound()`,
`permissionDenied()`, etc.) build common kinds without spelling the enum.

### Examples

```
match File.open("missing.txt") {
    .Ok(f) => use(f),
    .Err(e) => match e.kind {
        .NotFound          => createDefault(),
        .PermissionDenied  => requestAccess(),
        _                  => log(e.description())
    }
}
```

_Defined in `lang/std/io/error.ks`._

### Members

#### initializer `From Code`

```kestrel
public init(code: Int32)
```

Builds an error from a raw POSIX errno; classifies the kind.

_Defined in `lang/std/io/error.ks`._

#### initializer `From Kind`

```kestrel
public init(kind: IoErrorKind)
```

Builds an error for a categorized kind.

_Defined in `lang/std/io/error.ks`._

#### function `description`

```kestrel
public func description() -> String
```

Returns a short human-readable phrase for the error kind. Unknown
codes yield `"unknown error"`; for full coverage use `errno()`
with a platform `strerror`.

_Defined in `lang/std/io/error.ks`._

#### function `errno`

```kestrel
public func errno() -> Int32
```

The raw POSIX error code. Use for programmatic dispatch when
pattern-matching on `kind` is too coarse — e.g. distinguishing
between `.Other` codes.

_Defined in `lang/std/io/error.ks`._

#### field `kind`

```kestrel
public var kind: IoErrorKind
```

_Defined in `lang/std/io/error.ks`._

#### function `last`

```kestrel
public static func last() -> IoError
```

Snapshots the current value of the platform's `errno` thread-local.
Call immediately after a failed libc call — any other libc activity
in between can clobber the value.

_Defined in `lang/std/io/error.ks`._

## enum `IoErrorKind`

```kestrel
public enum IoErrorKind
```

Categorical classification of an I/O error.

`IoError` carries one of these alongside its raw `errno`. The named
variants cover the common categories applications dispatch on; everything
else falls into `.Other` carrying the original POSIX code so no
information is lost. Built from a code via `IoErrorKind.fromErrno(code:)`,
or matched directly in error-handling code:

```
match e.kind {
    .NotFound          => createDefault(),
    .PermissionDenied  => promptForElevation(),
    .Other(c)          => log("unhandled errno: " + c.toString())
}
```

_Defined in `lang/std/io/error.ks`._

### Members

#### case `AlreadyExists`

```kestrel
case AlreadyExists
```

`EEXIST` — the path already exists (e.g. `O_CREAT | O_EXCL`).

_Defined in `lang/std/io/error.ks`._

#### case `BadFileDescriptor`

```kestrel
case BadFileDescriptor
```

`EBADF` — file descriptor is invalid or closed.

_Defined in `lang/std/io/error.ks`._

#### case `BrokenPipe`

```kestrel
case BrokenPipe
```

`EPIPE` — write to a pipe with no reader.

_Defined in `lang/std/io/error.ks`._

#### case `Interrupted`

```kestrel
case Interrupted
```

`EINTR` — operation interrupted by a signal.

_Defined in `lang/std/io/error.ks`._

#### case `InvalidInput`

```kestrel
case InvalidInput
```

`EINVAL` — invalid argument to a libc call.

_Defined in `lang/std/io/error.ks`._

#### case `IoFailure`

```kestrel
case IoFailure
```

`EIO` — generic kernel-reported I/O failure.

_Defined in `lang/std/io/error.ks`._

#### case `IsADirectory`

```kestrel
case IsADirectory
```

`EISDIR` — operation expected a file but got a directory.

_Defined in `lang/std/io/error.ks`._

#### case `NoSpaceLeft`

```kestrel
case NoSpaceLeft
```

`ENOSPC` — no space left on device.

_Defined in `lang/std/io/error.ks`._

#### case `NotADirectory`

```kestrel
case NotADirectory
```

`ENOTDIR` — a path component is not a directory.

_Defined in `lang/std/io/error.ks`._

#### case `NotFound`

```kestrel
case NotFound
```

`ENOENT` — the path does not exist.

_Defined in `lang/std/io/error.ks`._

#### case `NotPermitted`

```kestrel
case NotPermitted
```

`EPERM` — operation not permitted.

_Defined in `lang/std/io/error.ks`._

#### case `Other`

```kestrel
case Other(Int32)
```

Any other POSIX errno — keeps the original code so callers can
still dispatch on the raw value.

_Defined in `lang/std/io/error.ks`._

#### case `OutOfMemory`

```kestrel
case OutOfMemory
```

`ENOMEM` — kernel allocation failed.

_Defined in `lang/std/io/error.ks`._

#### case `PermissionDenied`

```kestrel
case PermissionDenied
```

`EACCES` — caller lacks permission for the operation.

_Defined in `lang/std/io/error.ks`._

#### case `WouldBlock`

```kestrel
case WouldBlock
```

`EAGAIN` — non-blocking call would have blocked.

_Defined in `lang/std/io/error.ks`._

#### function `description`

```kestrel
public func description() -> String
```

Short human-readable phrase, locale-independent.

_Defined in `lang/std/io/error.ks`._

#### function `errno`

```kestrel
public func errno() -> Int32
```

The POSIX errno corresponding to this kind. Lossless round-trip
for all named variants and `.Other`.

_Defined in `lang/std/io/error.ks`._

#### function `fromErrno`

```kestrel
public static func fromErrno(Int32) -> IoErrorKind
```

Classifies a POSIX errno. Unknown codes fall through to `.Other(c)`.

_Defined in `lang/std/io/error.ks`._

## function `alreadyExists`

```kestrel
public func alreadyExists() -> IoError
```

`EEXIST` — the path already exists (e.g. `O_CREAT | O_EXCL`).

_Defined in `lang/std/io/error.ks`._

## function `brokenPipe`

```kestrel
public func brokenPipe() -> IoError
```

`EPIPE` — write to a pipe with no reader.

_Defined in `lang/std/io/error.ks`._

## function `interrupted`

```kestrel
public func interrupted() -> IoError
```

`EINTR` — operation interrupted by a signal.

_Defined in `lang/std/io/error.ks`._

## function `invalidInput`

```kestrel
public func invalidInput() -> IoError
```

`EINVAL` — invalid argument to a libc call.

_Defined in `lang/std/io/error.ks`._

## function `notFound`

```kestrel
public func notFound() -> IoError
```

`ENOENT` — the path does not exist.

_Defined in `lang/std/io/error.ks`._

## function `permissionDenied`

```kestrel
public func permissionDenied() -> IoError
```

`EACCES` — caller lacks permission for the operation.

_Defined in `lang/std/io/error.ks`._

## function `wouldBlock`

```kestrel
public func wouldBlock() -> IoError
```

`EAGAIN` — non-blocking call would have blocked.

_Defined in `lang/std/io/error.ks`._

