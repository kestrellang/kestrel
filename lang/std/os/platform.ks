// Platform identification

module std.os

import std.text.(String)

/// Returns the current platform identifier.
@platform(.darwin)
public func platform() -> String { "darwin" }

/// Returns the current platform identifier.
@platform(.linux)
public func platform() -> String { "linux" }
