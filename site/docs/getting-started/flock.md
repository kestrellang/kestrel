# Flock

Flock is the Kestrel build tool and package manager. It scaffolds projects, resolves dependencies, builds and runs binaries, and produces release artifacts. If you've used Cargo, npm, or go modules, the shape will feel familiar.

## Day-to-day commands

```sh
flock new <name>      # scaffold a new project
flock build           # compile (incremental)
flock run             # build and run
flock test            # build and run tests
flock check           # type-check without producing a binary
flock clean           # delete the build cache
```

`flock run` is what you'll use most. It's incremental — second runs only rebuild what changed.

## Project layout

A new project looks like:

```
my-project/
  flock.toml          — manifest
  src/
    main.ks
  tests/
    (empty)
  target/             — build output, gitignored
```

`flock.toml` declares the project name, version, and dependencies. A small one:

```toml
[project]
name = "my-project"
version = "0.1.0"

[dependencies]
serde = "0.4"
```

## Adding a dependency

```sh
flock add serde
```

Flock fetches the dependency, updates `flock.toml`, and writes a lockfile (`flock.lock`) pinning the resolved version graph. Commit `flock.lock` for applications; libraries usually don't.

## Build profiles

Two by default: `debug` (the default, fast compile, slow runtime) and `release` (slow compile, fast runtime, optimizations on).

```sh
flock build --release
flock run --release
```

You can define your own profiles in `flock.toml` for things like benchmarks or instrumented builds.

## More

For deeper Flock usage — workspaces, custom build scripts, publishing libraries — see [Tooling → Flock](../tooling/flock.md). For first-time setup, you've already done it: `flock` is included in the install.

---

[← Hello, World](hello-world.md) · [↑ Getting Started](index.md) · [Kestrel Skill →](kestrel-skill.md)
