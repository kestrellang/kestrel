# std.os.proc

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

