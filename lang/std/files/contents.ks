// Reading and writing a whole file in one call.

module std.files

import std.numeric.(Int64, UInt8)
import std.collections.(Array, Slice)
import std.text.(String)
import std.result.(Result)
import std.streams.(IoError, Writable, Readable)

extend File {
    /// The whole file as UTF-8 text.
    ///
    /// # Errors
    ///
    /// `InvalidData` if the file isn't valid UTF-8.
    ///
    /// # Examples
    ///
    /// ```
    /// let config = try File.readText("config.toml");
    /// ```
    public static func readText(path: Path) -> Result[String, IoError] {
        var file = try File.open(path);
        file.readText()
    }

    /// The whole file's bytes.
    public static func readBytes(path: Path) -> Result[Array[UInt8], IoError] {
        var file = try File.open(path);
        var bytes = Array[UInt8]();
        try file.readAll(into: bytes);
        .Ok(bytes)
    }

    /// Replaces the file's contents with `text`, creating the file if needed.
    public static func writeText(path: Path, text: String) -> Result[(), IoError] {
        var file = try File.create(path);
        file.write(text: text)
    }

    /// Replaces the file's contents with `bytes`, creating the file if needed.
    public static func writeBytes(path: Path, bytes: Slice[UInt8]) -> Result[(), IoError] {
        var file = try File.create(path);
        file.writeAll(from: bytes)
    }

    /// Adds `text` to the end of the file, creating the file if needed.
    public static func appendText(path: Path, text: String) -> Result[(), IoError] {
        var file = try File.open(path, OpenOptions.appending);
        file.write(text: text)
    }

    /// Adds `bytes` to the end of the file, creating the file if needed.
    public static func appendBytes(path: Path, bytes: Slice[UInt8]) -> Result[(), IoError] {
        var file = try File.open(path, OpenOptions.appending);
        file.writeAll(from: bytes)
    }
}
