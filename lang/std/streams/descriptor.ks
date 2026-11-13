// FileDescriptor: the system's number for an open file or socket.

module std.streams

import std.numeric.(Int32)
import std.text.(Formatter, Formattable, FormatOptions)
import std.core.(Bool, Equatable, Hashable, Hasher)

/// The system's number for an open file, pipe or socket.
///
/// Holding one doesn't own anything: whoever handed it out (a `File`, a
/// `TcpStream`) still closes it. Use it to pass a handle to C code or an
/// event loop.
public struct FileDescriptor: Equatable, Hashable, Formattable {
    /// The raw number.
    public var value: Int32

    /// @name From Number
    /// Wraps a raw descriptor number.
    public init(value: Int32) {
        self.value = value
    }

    /// Same number.
    public func isEqual(to other: FileDescriptor) -> Bool { self.value == other.value }

    /// Hashes the number.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.value.hash(into: hasher)
    }

    /// Writes the number.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        self.value.format(into: writer, options)
    }
}
