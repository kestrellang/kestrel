// The Seekable protocol, for streams with a position that can move.

module std.streams

import std.numeric.(Int64)
import std.result.(Result)
import std.io.error.(IoError)

/// Where `seek(to:)` moves a stream.
internal enum Seek {
    /// This many bytes from the start.
    case Start(Int64)
    /// This many bytes from the current position; negative moves back.
    case Current(Int64)
    /// This many bytes from the end; usually `0` or negative.
    case End(Int64)
}

/// A stream whose position can move: files and in-memory cursors, but not
/// pipes, sockets or the standard streams.
///
/// It is separate from `Readable` and `Writable` because most streams
/// can't seek. Code that needs to says so: `where R: Readable, R: Seekable`.
///
/// # Examples
///
/// ```
/// var c = Cursor(text: "hello");
/// try c.seek(to: .End(-2));
/// try c.readText()      // "lo"
/// ```
internal protocol Seekable {
    /// Moves to `position`; returns the new offset from the start.
    ///
    /// # Errors
    ///
    /// `InvalidInput` if the position would be before the start.
    mutating func seek(to position: Seek) -> Result[Int64, IoError]
}

extend Seekable {
    /// The offset from the start.
    internal mutating func currentPosition() -> Result[Int64, IoError] {
        self.seek(to: .Current(0))
    }

    /// Moves back to the start.
    internal mutating func rewind() -> Result[(), IoError] {
        try self.seek(to: .Start(0));
        .Ok(())
    }

    /// The stream's length in bytes. Seeks to the end and back, so the
    /// position is unchanged.
    internal mutating func length() -> Result[Int64, IoError] {
        let here = try self.currentPosition();
        let end = try self.seek(to: .End(0));
        try self.seek(to: .Start(here));
        .Ok(end)
    }
}
