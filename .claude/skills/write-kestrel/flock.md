# Flock: packages and builds

Flock is Kestrel's package manager. It ships with the toolchain (`jessup
install stable`). For a single file, `kestrel build file.ks -o out` is enough.
Use flock once you have dependencies or more than one file.

## Commands

| Command | Does |
|---|---|
| `flock init` | Scaffold a binary package: `flock.toml` + `src/main.ks` |
| `flock init --lib` | Scaffold a library: `flock.toml` + `src/lib.ks` |
| `flock check` | Type-check without building |
| `flock build` | Build; the binary lands in the package root, named after the package |
| `flock run` | Build and run (`--bin <name>` picks a target) |
| `flock update` | Refresh `flock.lock` |
| `flock install` | Build optimized binaries into `~/.flock/bin` (`<org>/<pkg>[@ver]` installs from the registry) |
| `flock publish` | Publish to the registry (needs `FLOCK_TOKEN` or `~/.flock/credentials`) |

## Manifest

```toml
[package]
name = "greet"
version = "0.1.0"
description = "Says hello"
license = "MIT"

[dependencies]
kestrel/quill-json = "0.1.0"     # registry: org/name = version
util = { path = "../util" }      # local path

[build]                          # only for packages that wrap C
c-sources = ["helpers.c"]
c-flags-cmd = "sdl2-config --cflags"
link-cmd = "sdl2-config --libs"
```

Browse packages at https://kestrel-lang.com/flock.

## Layout and modules

- **Binaries:** `src/main.ks` (named after the package), plus one per file in
  `src/bin/*.ks`. Each binary has its own `@main`.
- **Library:** every other file under `src/`. Dependents compile only the
  library, never the binaries, so a dependency's `@main` can't clash with yours.
- **Every file needs a `module` line.** The module path is free-form, not
  tied to the file path; by convention it starts with the package name
  (`module greet.main`, `module util.lib`, `module quill.json.parser`).
- **Import a dependency's items by its module path:**

```kestrel
// src/main.ks (needs the `util` dependency, so not a standalone program)
module greet.main

import util.lib.(greeting)       // `util` is the path dependency above

@main
func main() -> Int64 {
    println(greeting());
    0
}
```

The std needs no import; dependencies always do.

A library is everything except the binaries, so a package with only
`src/main.ks` has no library, and depending on it is an error. A package
with only a library has nothing to `flock install`.
