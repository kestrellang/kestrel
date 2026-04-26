# Installation

The Kestrel toolchain ships as a single binary with a build tool ([Flock](flock.md)), a compiler, and a language server. Pick your platform below.

## macOS

```sh
curl -fsSL https://kestrel-lang.org/install.sh | sh
```

This installs `kestrel` and `flock` into `~/.kestrel/bin`. Add that directory to your `PATH`:

```sh
echo 'export PATH="$HOME/.kestrel/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

## Linux

Same command:

```sh
curl -fsSL https://kestrel-lang.org/install.sh | sh
```

Tested on recent Ubuntu, Debian, Fedora, and Arch. Requires `glibc` 2.31 or later.

## Windows

PowerShell:

```powershell
irm https://kestrel-lang.org/install.ps1 | iex
```

Adds `kestrel.exe` and `flock.exe` to your PATH automatically.

## Verifying

```sh
kestrel --version
flock --version
```

Both should print a version string. If they don't, your shell hasn't picked up the new PATH — open a new terminal and try again.

## Updating

```sh
kestrel update
```

Pulls the latest stable release. Pass `--channel nightly` to track the development branch.

## Uninstalling

```sh
kestrel uninstall
```

Removes everything the installer added. Your projects are untouched.

---

[← Getting Started](index.md) · [↑ Getting Started](index.md) · [Hello, World →](hello-world.md)
