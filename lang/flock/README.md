# Flock

Package manager for Kestrel. Manages dependencies, builds packages, and publishes to the package registry.

## Installation

Flock is included with the Kestrel toolchain. Install via jessup:

```
jessup install stable
```

Or build from source using the bootstrap script:

```
cd lang/flock && ./bootstrap.sh
```

## Commands

- `flock build` - build the current package (`--bin <name>` to pick a target)
- `flock run` - build and run the current package (`--bin <name>` to pick a target)
- `flock check` - type-check without building
- `flock install` - build and install a package's binaries into `~/.flock/bin`
- `flock init` - scaffold a new package (binary by default; `--lib` for a library)
- `flock publish` - publish a package to the registry
- `flock update` - update dependency lock file

## Targets: libraries and binaries

Like Cargo, a package has at most one **library** and any number of **binaries**:

- **Binaries** — `src/main.ks` (named after the package), `src/bin/*.ks` (one
  each, named after the file), and `[[bin]] { name, path }` overrides/additions.
- **Library** — everything else under `src/` (the package's importable surface).
  A package that defines *only* binaries has no library.

The two kinds are used in opposite directions, and flock enforces it both ways:

- **Dependencies use the library.** When you depend on a package, flock compiles
  its library sources and never its binaries — so a dependency's `@main` can't
  collide with yours. Depending on a **bin-only** package is an error (it has no
  library to link), just like Cargo's *"no library targets found"*.
- **`flock install` uses the binaries.** Installing a **lib-only** package is an
  error (nothing to install), mirroring Cargo's *"there is nothing to install…
  only libraries"*.

`build`/`run` pick the `src/main.ks` (package-named) binary by default; pass
`--bin <name>` when a package has several. Each binary compiles the whole `src/`
together (the library sources + that one entry).

## Installing tools

```
flock install                       # install the current package's binaries
flock install <org>/<pkg>           # install the latest from the registry
flock install <org>/<pkg>@<version> # install a pinned version
flock install --bin <name>          # install only that target
flock install --force               # overwrite an existing installed binary
flock install --debug               # build unoptimized (default is optimized)
```

Binaries are copied into `~/.flock/bin`. Add it to your `PATH`:

```
export PATH="$HOME/.flock/bin:$PATH"
```

## Manifest (flock.toml)

```toml
[package]
name = "my-package"
version = "0.1.0"
description = "A Kestrel package"
author = "you"
license = "MIT"

[dependencies]
kestrel/quill = "0.1.0"
kestrel/quill-json = "0.1.0"
```

## Publishing

```
FLOCK_ORG=myorg flock publish
```

Requires an auth token saved to `~/.flock/credentials` or set via `FLOCK_TOKEN`.

## Features

- Dependency resolution with lock files
- Registry and path-based dependencies
- Automatic source file discovery
- Compiler invocation and linking
