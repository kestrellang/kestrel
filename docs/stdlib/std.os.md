# std.os

## function `captureOutput`

```kestrel
public func captureOutput(String) -> String
```

Runs `command` through the system shell and returns its captured stdout.

Reads from `popen(command, "r")` 1 KiB at a time until EOF, then
trims a single run of trailing ASCII whitespace (space, tab, LF,
CR) so callers don't have to chomp the newline themselves. Stderr
is **not** captured — it goes to the parent's stderr. Returns the
empty string if `popen` fails.

### Examples

```
let branch = captureOutput(command: "git rev-parse --abbrev-ref HEAD");
// "main"
```

_Defined in `lang/std/os/proc.ks`._

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

## function `exit`

```kestrel
public func exit(Int32)
```

Terminates the calling process immediately with the given exit code.

Wraps `libc::exit`. Runs `atexit` handlers and flushes stdio
buffers; does **not** unwind Kestrel's stack or run deinits on
values still in scope. Conventionally `0` means success and any
non-zero value means failure; a few codes have specific meanings
(`2` is shells' "misuse of builtins", `126`/`127` are `exec`
errors, `>128` typically encodes a fatal signal).

### Examples

```
exit(code: 0);   // success — does not return
```

_Defined in `lang/std/os/proc.ks`._

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

## function `getenv`

```kestrel
public func getenv(String) -> Optional[String]
```

Looks up the value of the environment variable `name`.

Returns `Some(value)` if the variable is set (including the empty
string), `None` if it is unset. Wraps `libc::getenv`, which returns
a pointer into the `environ` block — this function copies the bytes
into a Kestrel `String` immediately, so the result is safe to keep
across subsequent `setenv` / `unsetenv` calls.

### Examples

```
match getenv(name: "HOME") {
    .Some(path) => print(path),
    .None      => print("HOME not set")
}
```

_Defined in `lang/std/os/env.ks`._

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

## function `platform`

```kestrel
public func platform() -> String
```

Returns a short identifier for the host operating system.

One of `"darwin"` or `"linux"` — the string is fixed at compile
time via `@platform` selection of two distinct definitions, so the
call is effectively a constant. Use this for one-off platform
branches; for repeated checks consider `@platform` on your own
functions instead.

### Examples

```
if platform() == "darwin" {
    // macOS-specific path
}
```

_Defined in `lang/std/os/platform.ks`._

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

## function `spawn`

```kestrel
public func spawn(String) -> Int32
```

Runs `command` through the system shell and returns its exit code.

Wraps `libc::system`, which on POSIX runs `/bin/sh -c <command>`
and returns a packed status word; this function shifts off the
signal/coredump bits and returns just the exit code (0–255 in
normal cases). The child's stdout and stderr are inherited from
the parent process — they go straight to the terminal. For
captured output, use `captureOutput`.

### Examples

```
let code = spawn(command: "ls -la");
if code != 0 {
    print("ls failed");
}
```

_Defined in `lang/std/os/proc.ks`._

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

