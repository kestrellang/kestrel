# std.os.env

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

