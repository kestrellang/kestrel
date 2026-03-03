// Platform identification (Linux)

module std.os

import std.text.(String)

/// Returns the current platform identifier.
public func platform() -> String { "linux" }
