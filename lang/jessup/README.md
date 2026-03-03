# Jessup

Version manager for the Kestrel toolchain. Install, manage, and switch between compiler versions.

## Installation

Download the latest jessup binary from GitHub releases, or build from source:

```
cd lang/jessup && flock build
```

## Commands

- `jessup install <version>` - install a toolchain (stable, nightly, or specific version)
- `jessup default <version>` - set the default toolchain
- `jessup list` - show installed toolchains
- `jessup update` - update installed channels to latest
- `jessup remove <version>` - remove an installed toolchain
- `jessup show` - show the active toolchain
- `jessup self update` - update jessup itself

## Usage

```
# Install the stable toolchain
jessup install stable

# Install a nightly build
jessup install nightly

# Switch default version
jessup default stable

# See what's installed
jessup list

# Update all channels
jessup update
```

## How It Works

Jessup downloads prebuilt Kestrel toolchains from GitHub releases and manages them in `~/.kestrel/toolchains/`. It supports channel-based versions (stable, nightly) that track the latest release in each channel.
