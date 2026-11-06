// Metadata, FileKind and Permissions.

module std.files

import std.numeric.(Int32, Int64, UInt32, UInt64)
import std.core.(Bool, Equatable)
import std.result.(Result)
import std.text.(String, Formatter, Formattable, FormatOptions, _writePadded)
import std.streams.(IoError)
import std.ffi.(malloc, free)
import std.memory.(RawPointer)

/// What a path names.
public enum FileKind: Equatable, Formattable {
    /// A regular file.
    case File
    /// A directory.
    case Directory
    /// A symbolic link (only reported when links aren't followed).
    case SymbolicLink
    /// Anything else: a device, socket or pipe.
    case Other

    /// Same kind.
    public func isEqual(to other: FileKind) -> Bool {
        self.code() == other.code()
    }

    /// `file`, `directory`, `symbolic link` or `other`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        let text = match self {
            .File => "file",
            .Directory => "directory",
            .SymbolicLink => "symbolic link",
            .Other => "other"
        };
        _writePadded(into: writer, text, options)
    }

    func code() -> Int64 {
        match self { .File => 0, .Directory => 1, .SymbolicLink => 2, .Other => 3 }
    }

    /// The kind for the shims' kind number.
    static func of(code: Int64) -> FileKind {
        match code { 0 => .File, 1 => .Directory, 2 => .SymbolicLink, _ => .Other }
    }
}

/// Unix permission bits, such as `0o644`.
public struct Permissions: Equatable {
    /// The permission bits (`0o7777` at most).
    public var mode: UInt32

    /// @name From Mode
    /// The permissions `mode`, such as `Permissions(0o755)`.
    public init(mode: UInt32) {
        self.mode = mode
    }

    /// `rw-r--r--`, the default for new files.
    public static var fileDefault: Permissions { Permissions(420) }

    /// `rwxr-xr-x`, the default for new directories.
    public static var directoryDefault: Permissions { Permissions(493) }

    /// `true` if no one may write (no write bit is set).
    public var isReadOnly: Bool { self.mode & 146 == 0 }

    /// Same bits.
    public func isEqual(to other: Permissions) -> Bool { self.mode == other.mode }
}

/// Facts about a file, directory or link.
public struct Metadata {
    /// What it is.
    public var kind: FileKind
    /// Its size in bytes.
    public var size: Int64
    /// Its permission bits.
    public var permissions: Permissions
    /// When its contents last changed, in nanoseconds since the Unix epoch.
    public var modified: Int64
    /// When it was last read, in nanoseconds since the Unix epoch.
    public var accessed: Int64
    /// The device the file system lives on (`st_dev`).
    public var device: UInt64
    /// The file's number on its device (`st_ino`). Two paths name the same
    /// file exactly when `device` and `inode` both match.
    public var inode: UInt64

    /// `true` for a regular file.
    public var isFile: Bool { self.kind == .File }
    /// `true` for a directory.
    public var isDirectory: Bool { self.kind == .Directory }
    /// `true` for a symbolic link.
    public var isSymbolicLink: Bool { self.kind == .SymbolicLink }

    /// `true` when both describe the same file: the same inode on the
    /// same device, whatever paths or links led to them.
    public func isSameFile(as other: Metadata) -> Bool {
        self.device == other.device and self.inode == other.inode
    }
}

/// The metadata of what's at `path`, following symbolic links (`stat`).
public func metadata(path: Path) -> Result[Metadata, IoError] {
    readMetadata(path, true)
}

/// The metadata of `path` itself, without following a symbolic link
/// (`lstat`).
public func symbolicLinkMetadata(path: Path) -> Result[Metadata, IoError] {
    readMetadata(path, false)
}

internal func readMetadata(path: Path, follow: Bool) -> Result[Metadata, IoError] {
    let out = unsafe malloc(metadataCellCount * 8);
    let cpath = try path.text.toNativeString();
    let r = unsafe native_path_metadata(cpath.raw.asRaw(), if follow { 1 } else { 0 }, out);
    let error = IoError.last();
    cpath.free();
    let result = if r < 0 { .Err(error) } else { .Ok(metadataFrom(out)) };
    unsafe free(out);
    result
}

/// How many int64s the metadata shims write (`fill_metadata` in
/// files_shims.c).
internal let metadataCellCount: Int64 = 7

/// Builds `Metadata` from the shims' int64s.
internal func metadataFrom(out: RawPointer) -> Metadata {
    let cells = out.cast[Int64]();
    Metadata(
        kind: FileKind.of(unsafe cells.offset(by: 0).read()),
        size: unsafe cells.offset(by: 1).read(),
        permissions: Permissions(UInt32(from: unsafe cells.offset(by: 2).read())),
        modified: unsafe cells.offset(by: 3).read(),
        accessed: unsafe cells.offset(by: 4).read(),
        device: UInt64(truncating: unsafe cells.offset(by: 5).read()),
        inode: UInt64(truncating: unsafe cells.offset(by: 6).read())
    )
}
