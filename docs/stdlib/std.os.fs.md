# std.os.fs

## function `chmod`

```kestrel
public func chmod(String, Int32) -> Result[(), IoError]
```

Changes the mode bits of `path` to `mode`.

Wraps `chmod(2)`. `mode` is the raw POSIX mode bits (e.g.
`0o755`); pass it as `Int32`. Resolves symlinks (use `lchmod`
equivalent if you need to change a link itself — not currently
exposed).

### Errors

Returns `Err(IoError)` on any libc failure; `errno` is captured.

### Examples

```
chmod(path: "/tmp/script.sh", mode: Int32(intLiteral: 0o755));
```

_Defined in `lang/std/os/fs.ks`._

## function `fileExists`

```kestrel
public func fileExists(String) -> Bool
```

Returns true if any filesystem entry exists at `path`.

Wraps `access(path, F_OK)`. Does not distinguish files from
directories or symlinks (a dangling symlink reports as
nonexistent because `access` follows symlinks). For the type,
follow up with `isFile` / `isDirectory`.

### Examples

```
if fileExists(path: "/tmp/foo") {
    // ...
}
```

_Defined in `lang/std/os/fs.ks`._

## function `getcwd`

```kestrel
public func getcwd() -> String
```

Returns the calling process's current working directory.

Wraps `getcwd(2)` with a 1 KiB buffer. Returns the empty string if
the cwd has been deleted, is longer than 1 KiB, or any other
`getcwd` failure occurs — the function does not surface the
error code.

### Examples

```
let here = getcwd();
```

_Defined in `lang/std/os/fs.ks`._

## function `isDirectory`

```kestrel
public func isDirectory(String) -> Bool
```

Returns true if `path` exists and is a directory.

Resolves symlinks (uses `stat`, not `lstat`). Returns `false` for
nonexistent paths or any non-directory file type.

### Examples

```
isDirectory(path: "/tmp");      // true
isDirectory(path: "/etc/hosts"); // false
```

_Defined in `lang/std/os/fs.ks`._

## function `isFile`

```kestrel
public func isFile(String) -> Bool
```

Returns true if `path` exists and is a regular file.

Resolves symlinks. Returns `false` for directories, sockets,
FIFOs, devices, and nonexistent paths.

_Defined in `lang/std/os/fs.ks`._

## function `listDir`

```kestrel
public func listDir(String) -> Array[String]
```

Returns the names of the entries inside `path`, excluding `.` and `..`.

Wraps `opendir`/`readdir`/`closedir`. The returned names are
relative to `path`; join with `path` yourself if you need full
paths. On failure to open the directory (missing path, permission
denied, etc.), returns an empty array — the function does not
distinguish "empty directory" from "open failed".

### Examples

```
for entry in listDir(path: "/tmp") {
    print(entry);
}
```

_Defined in `lang/std/os/fs.ks`._

## function `mkdir`

```kestrel
public func mkdir(String) -> Result[(), IoError]
```

Creates a single directory at `path` with mode `0o755`.

Wraps `mkdir(2)`. Fails if `path` already exists or any parent
component is missing — use `mkdirAll` to create intermediate
directories.

### Errors

Returns `Err(IoError)` on any libc failure; `errno` is captured and
surfaced via the error's `kind`. Common cases: `EEXIST` (path exists),
`ENOENT` (missing parent), `EACCES` (permission denied).

### Examples

```
match mkdir(path: "/tmp/foo") {
    .Ok(_)  => print("created"),
    .Err(e) => print(e.message)
}
```

_Defined in `lang/std/os/fs.ks`._

## function `mkdirAll`

```kestrel
public func mkdirAll(String) -> Result[(), IoError]
```

Creates `path` and any missing parent directories.

Walks back from the deepest non-existent component, recursing on
the parent first. If `path` already exists and is a directory, the
call is a no-op success; if it exists and is **not** a directory,
returns `Err(IoError(kind: .AlreadyExists))` (`EEXIST`) without disturbing the file.
Each created intermediate uses the default mode `0o755`.

### Errors

Forwards any `mkdir` failure verbatim. Specific to this function:
`Err(IoError(kind: .AlreadyExists))` when `path` exists as a non-directory.

### Examples

```
mkdirAll(path: "/tmp/foo/bar/baz");  // creates all three levels
```

_Defined in `lang/std/os/fs.ks`._

## function `readlink`

```kestrel
public func readlink(String) -> Result[String, IoError]
```

Returns the target stored in the symlink at `path`.

Wraps `readlink(2)` with a 1 KiB buffer. `readlink` does not null-
terminate, so this function copies exactly the returned byte
count into the result string. Targets longer than 1 KiB are
silently truncated — the syscall returns a partial result rather
than failing.

### Errors

Returns `Err(IoError)` if `path` is not a symlink (`EINVAL`),
missing (`ENOENT`), or any other libc failure.

_Defined in `lang/std/os/fs.ks`._

## function `remove`

```kestrel
public func remove(String) -> Result[(), IoError]
```

Deletes the file at `path`.

Wraps `unlink(2)`. Does not work on directories — use `removeDir`
for those. If `path` is the last link to an open file, the file's
blocks remain allocated until every descriptor is closed.

### Errors

Returns `Err(IoError)` on any libc failure; `errno` is captured.

_Defined in `lang/std/os/fs.ks`._

## function `removeDir`

```kestrel
public func removeDir(String) -> Result[(), IoError]
```

Removes an empty directory at `path`.

Wraps `rmdir(2)`. Fails with `ENOTEMPTY` if the directory still
has entries — list and remove its contents first if you need a
recursive remove.

### Errors

Returns `Err(IoError)` on any libc failure; `errno` is captured.

_Defined in `lang/std/os/fs.ks`._

## function `rename`

```kestrel
public func rename(String, String) -> Result[(), IoError]
```

Renames or moves `from` to `to`.

Wraps `rename(2)`. Atomic within a single filesystem; cross-
filesystem moves return `EXDEV` and require a copy + delete
instead. If `to` exists, it is replaced (subject to type-match
rules — file replaces file, directory replaces empty directory).

### Errors

Returns `Err(IoError)` on any libc failure; `errno` is captured.

_Defined in `lang/std/os/fs.ks`._

## function `symlink`

```kestrel
public func symlink(String, String) -> Result[(), IoError]
```

Creates a symbolic link at `path` pointing to `target`.

Wraps `symlink(2)`. The target is stored verbatim — it is not
resolved or validated, so dangling links are allowed and relative
targets resolve relative to the directory containing the link.

### Errors

Returns `Err(IoError)` on any libc failure; `errno` is captured.

_Defined in `lang/std/os/fs.ks`._

