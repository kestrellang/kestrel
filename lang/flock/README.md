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

- `flock build` - build the current package
- `flock run` - build and run the current package
- `flock check` - type-check without building
- `flock init` - create a new flock.toml manifest
- `flock publish` - publish a package to the registry
- `flock update` - update dependency lock file

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

Requires an auth token saved to `~/.kestrel/credentials` or set via `FLOCK_TOKEN`.

## Features

- Dependency resolution with lock files
- Registry and path-based dependencies
- Automatic source file discovery
- Compiler invocation and linking
