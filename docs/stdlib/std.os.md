# std.os

## Submodules

- [`std.os.env`](std.os.env.md)
- [`std.os.fs`](std.os.fs.md)
- [`std.os.proc`](std.os.proc.md)

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

