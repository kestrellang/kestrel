# Flock

This page covers Flock beyond the [Getting Started introduction](../getting-started/flock.md) — workspaces, custom profiles, build scripts, and the publishing flow.

## Workspaces

A workspace groups multiple Kestrel projects under one `flock.toml`, with shared dependencies and a single `target/` directory:

```toml
[workspace]
members = [
    "crates/core",
    "crates/cli",
    "crates/server",
]
```

Run `flock build` from the workspace root to build all members. Run `flock build -p core` to build just one.

Workspaces are useful when projects share code but have different shipping shapes (a library + CLI + service, all developed together). For single-binary projects, you don't need one.

## Profiles

Two are built in (`debug`, `release`). Define more in `flock.toml` for workflows that want different optimization or instrumentation:

```toml
[profile.bench]
inherits = "release"
debug = true             # keep symbols for profiling
optimization = "aggressive"

[profile.size]
inherits = "release"
optimize-for = "size"
```

Build them with `--profile`:

```sh
flock build --profile bench
flock run --profile size
```

## Build scripts

A project that needs codegen at build time (e.g. generating bindings from a schema) ships a `build.ks`:

```swift
module Build

func main() -> Int {
    // run codegen, write to target/generated/...
    0
}
```

Flock runs `build.ks` before compiling the rest of the project. Files written under `target/generated/` are picked up automatically as part of the build.

## Publishing

To publish a library to the registry:

```sh
flock publish
```

Requires you've authenticated (`flock login`) and your `flock.toml` has a `[package]` section with name, version, license, and description.

Versions follow semver. Once published, a version is immutable — bump and re-publish for fixes.

---

[← Tooling](index.md) · [↑ Tooling](index.md) · [Kestrel LSP →](kestrel-lsp.md)
