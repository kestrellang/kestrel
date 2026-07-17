# Kestrel Toolchain Reference

> Verified against `src/main.rs` and `./kestrel --help` on 2026-07-01.
> Only flags that exist in the compiler are documented here.

The toolchain has three layers:

| Tool | Role |
|---|---|
| [`kestrel`](#the-kestrel-cli) | The compiler. Compiles `.ks` files into an executable, and dumps compiler-internal representations for debugging. |
| [`flock`](#flock--package-manager) | Package manager and build tool. `init`/`build`/`run`/`check`/`install`/`publish` for whole packages; invokes `kestrel` under the hood. |
| [`jessup`](#jessup--toolchain-installer) | Toolchain installer and version manager. Installs `kestrel`, `flock`, `kestrel-lsp`, and the stdlib. |

Day-to-day project work goes through `flock`; `kestrel` is what you reach for
when compiling individual files or inspecting the compiler pipeline.

---

## The `kestrel` CLI

```
kestrel <COMMAND> [OPTIONS]

Commands:
  build  Build an executable from source files
  dump   Dump a compiler-internal representation to stdout
```

There is no `kestrel run` or `kestrel check` — those are `flock` commands
(see [flock](#flock--package-manager)). To type-check without producing a
binary, use `kestrel dump diagnostics` or `flock check`.

### Global options

Available on every subcommand:

| Flag | Meaning |
|---|---|
| `--target <TRIPLE>` | Target triple for cross-compilation (e.g. `x86_64-unknown-linux-gnu`). Also selects `@platform` conditionals (darwin/linux). |
| `-v`, `--verbose` | Verbose output (stdlib path, files read, output path). |
| `--std <PATH>` | Path to the standard library (overrides the default lookup below). |
| `--no-std` | Compile without the standard library. |

### Standard library lookup

Unless `--no-std` is given, the stdlib is located in this priority order:

1. `--std <PATH>` flag
2. `KESTREL_STD` environment variable
3. `<exe>/../lib/std` relative to the (symlink-resolved) `kestrel` binary —
   the layout of a jessup-installed toolchain
4. `lang/std/` in the repository (in-repo development builds)

If none exists, the build fails and prints every path it tried, with a hint
to set `KESTREL_STD` or pass `--std <path>`.

### `kestrel build`

```
kestrel build [OPTIONS] <FILES>...
```

Compiles one or more `.ks` files into an executable. An executable build
requires exactly one `@main` function (see diagnostics E615–E618 in
[error-codes.md](error-codes.md)). If any error is reported, no binary is
written and the exit code is nonzero.

| Flag | Meaning |
|---|---|
| `-o`, `--output <OUTPUT>` | Output executable path. Defaults to the basename of the first input file (`.exe` appended on Windows). |
| `-O`, `--opt-level <LEVEL>` | Optimization level: `0` = none (default), `1` = speed, `2` = speed + size. |
| `--backend <BACKEND>` | Code generation backend: `cranelift` (default) or `llvm`. The LLVM backend requires LLVM 18 installed. |
| `-l`, `--link <LIBRARY>` | Link with a library (repeatable; use `:libname.a` for static). |
| `-L`, `--library-path <PATH>` | Add a library search path (repeatable). |
| `--framework <NAME>` | Link a macOS framework (repeatable). |

#### Environment variable overrides

Two environment variables override build flags. They exist so tools that
shell out to `kestrel build` (e.g. `flock`) can select behavior without
passing flags:

| Variable | Overrides | Values |
|---|---|---|
| `KESTREL_BACKEND` | `--backend` | `cranelift` or `llvm` |
| `KESTREL_OPT` | `-O` / `--opt-level` | `0`, `1`, or `2` |
| `KESTREL_STD` | default stdlib lookup (but not `--std`) | path to a stdlib directory |

```sh
# Build with LLVM at -O2 regardless of flags:
KESTREL_BACKEND=llvm KESTREL_OPT=2 kestrel build main.ks -o app
```

### `kestrel dump` — inspecting compiler stages

```
kestrel dump <KIND> [OPTIONS] [FILES]...
```

Prints a compiler-internal representation to **stdout**; diagnostics always
go to **stderr**, so `kestrel dump mir f.ks > out.txt` captures the dump
while errors still show in the terminal.

The available dump kinds (these are the exact names — there are no separate
AST/HIR/type dumps in the current CLI):

| Kind | What it prints |
|---|---|
| `tokens` | Token stream from the lexer (per file; no stdlib, no inference). |
| `cst` | Concrete syntax tree from the parser (per file; no stdlib, no inference). |
| `mir` | The MIR (OSSA) module, at a selectable pipeline stage. |
| `cranelift` | Cranelift IR produced from the monomorphized MIR. |
| `diagnostics` | All accumulated diagnostics (lex, parse, infer, analyze) — nothing on stdout. Useful as a "check" mode. |

| Flag | Meaning |
|---|---|
| `-f`, `--function <SUBSTRING>` | Filter output to functions whose name contains this substring. |
| `-s`, `--stage <STAGE>` | For `mir` only: which pipeline stage to print. Defaults to `verify`. |
| `--list-stages` | List the MIR pipeline stages and exit. |

#### MIR pipeline stages (`-s`/`--stage`)

In pipeline order: `raw`, `drop-fix`, `thunk`, `drop-shim`, `clone-shim`,
`layout`, `verify` (default), then the post-monomorphization stages `mono`,
`copy-prop`, `expand`. The meta-stage `all` prints every stage under
`=== <stage> ===` headers.

The default `verify` stage aborts on a verification error; every other stage
is best-effort — it prints whatever the stage produced and reports verify
problems as warnings on stderr.

```sh
kestrel dump mir main.ks                 # MIR after verify (default)
kestrel dump mir -s mono main.ks         # after monomorphization
kestrel dump mir -s all -f myFunc main.ks  # every stage, one function
kestrel dump cranelift main.ks           # backend IR
kestrel dump diagnostics main.ks         # errors/warnings only
```

---

## `flock` — package manager

Full documentation: [`lang/flock/README.md`](../lang/flock/README.md).

`flock` manages packages: a `flock.toml` manifest, dependency resolution with
lock files, source discovery under `src/`, and compiler invocation. A package
has at most one **library** (the importable sources under `src/`) and any
number of **binaries** (`src/main.ks`, `src/bin/*.ks`, and `[[bin]]`
overrides). Dependencies always use a package's library; `flock install` uses
its binaries.

Common commands:

| Command | Meaning |
|---|---|
| `flock init` | Scaffold a new package (binary by default; `--lib` for a library). |
| `flock build` | Build the current package (`--bin <name>` to pick a target). |
| `flock run` | Build and run the current package. |
| `flock check` | Type-check without building. |
| `flock install` | Build and install a package's binaries into `~/.flock/bin` (`flock install <org>/<pkg>[@version]` installs from the registry; `--force`, `--debug`, `--bin <name>`). |
| `flock publish` | Publish to the registry (`FLOCK_ORG`, token in `~/.flock/credentials` or `FLOCK_TOKEN`). |
| `flock update` | Update the dependency lock file. |

`~/.flock` layout: installed binaries land in `~/.flock/bin` (add it to your
`PATH`), and registry credentials live in `~/.flock/credentials`.

## `jessup` — toolchain installer

Full documentation: [`lang/jessup/README.md`](../lang/jessup/README.md).

`jessup` installs and switches between Kestrel toolchain versions. A
toolchain includes `kestrel`, `flock`, `kestrel-lsp`, and the standard
library. Toolchains are downloaded from GitHub releases into
`~/.jessup/toolchains/`, and the active one is exposed via symlinks in
`~/.jessup/bin/` (the `kestrel` binary finds its stdlib relative to the
resolved symlink — see [stdlib lookup](#standard-library-lookup)).

| Command | Meaning |
|---|---|
| `jessup install <version>` | Install a toolchain (`stable`, `nightly`, or a specific version). |
| `jessup default <version>` | Set the default toolchain. |
| `jessup list` | Show installed toolchains. |
| `jessup show` | Show the active toolchain. |
| `jessup update` | Update installed channels to latest. |
| `jessup remove <version>` | Remove an installed toolchain. |
| `jessup self update` | Update jessup itself. |
