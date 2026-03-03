// Platform identification (macOS)

module std.os

import std.text.(String)

/// Returns the current platform identifier.
public func platform() -> String { "darwin" }
