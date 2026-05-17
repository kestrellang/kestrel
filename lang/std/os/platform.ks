// Platform identification

module std.os

import std.text.(String)

/// Returns a short identifier for the host operating system.
///
/// One of `"darwin"` or `"linux"` — the string is fixed at compile
/// time via `@platform` selection of two distinct definitions, so the
/// call is effectively a constant. Use this for one-off platform
/// branches; for repeated checks consider `@platform` on your own
/// functions instead.
///
/// # Examples
///
/// ```
/// if platform() == "darwin" {
///     // macOS-specific path
/// }
/// ```
@platform(.darwin)
public func platform() -> String { "darwin" }

/// Linux-specific definition of `platform()`. Selected by `@platform(.linux)`.
@platform(.linux)
public func platform() -> String { "linux" }
