// Platform identification

module std.os

import std.text.(String)

/// Returns a short identifier for the host operating system.
///
/// One of `"darwin"`, `"linux"` or `"emscripten"` — the string is fixed at compile
/// time via `@platform` selection of distinct definitions, so the
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
@platform(.Darwin)
public func platform() -> String { "darwin" }

/// Linux-specific definition of `platform()`. Selected by `@platform(.Linux)`.
@platform(.Linux)
public func platform() -> String { "linux" }

/// Emscripten (WebAssembly in a browser) definition of `platform()`.
@platform(.Emscripten)
public func platform() -> String { "emscripten" }
