// Removing, renaming, copying and linking files.

module std.files

import std.numeric.(Int32, Int64, UInt8)
import std.collections.(Array)
import std.text.(String)
import std.result.(Result)
import std.memory.(RawPointer)
import std.streams.(IoError, copy as copyStream)
import std.ffi.(malloc, free)

extend File {
    /// Removes the file or symbolic link `path`. Use `Directory.remove` for a
    /// directory.
    public static func remove(path: Path) -> Result[(), IoError] {
        withNativePath(path, { (p) in unsafe native_unlink(p) })
    }

    /// Renames `source` to `destination`, replacing a file already there.
    /// Both must be on the same file system (`CrossesDevices` otherwise).
    public static func rename(source: Path, to destination: Path) -> Result[(), IoError] {
        let from = try source.toNativePath();
        let to = match destination.toNativePath() {
            .Ok(c) => c,
            .Err(e) => { from.free(); return .Err(e) }
        };
        let r = unsafe native_rename(from.raw.asRaw(), to.raw.asRaw());
        let error = IoError.last();
        from.free();
        to.free();
        if r < 0 { .Err(error.at(path: source.string)) } else { .Ok(()) }
    }

    /// Copies the file `source` to `destination`, with its permissions.
    /// Returns the number of bytes copied.
    ///
    /// Fails with `InvalidInput` when both name the same file, through the
    /// same path or a symbolic or hard link: opening the destination
    /// truncates it, which would empty the source before it's read.
    public static func copy(source: Path, to destination: Path) -> Result[Int64, IoError] {
        var input = try File.open(source);
        let sourceInfo = try input.metadata();
        let permissions = sourceInfo.permissions;
        if let .Ok(destinationInfo) = Metadata.of(destination) {
            if sourceInfo.isSameFile(as: destinationInfo) {
                return .Err(IoError(kind: .InvalidInput))
            }
        }
        var options = OpenOptions.writing;
        options.permissions = permissions;
        var output = try File.open(destination, options);
        let n = try copyStream(from: input, to: output);
        try File.setPermissions(destination, permissions);
        .Ok(n)
    }

    /// Sets the permission bits of `path` (`chmod`).
    public static func setPermissions(path: Path, permissions: Permissions) -> Result[(), IoError] {
        withNativePath(path, { (p) in unsafe native_chmod(p, Int32(bitPattern: permissions.mode)) })
    }
}

/// Symbolic links: creating them and reading where they point. Removing one
/// is `File.remove`; `Metadata.ofSymbolicLink` describes the link itself.
public enum SymbolicLink {}

extend SymbolicLink {
    /// Creates a symbolic link at `link` that points to `target`. `target`
    /// isn't checked; it may not exist.
    public static func create(at link: Path, to target: Path) -> Result[(), IoError] {
        let t = try target.toNativePath();
        let l = match link.toNativePath() {
            .Ok(c) => c,
            .Err(e) => { t.free(); return .Err(e) }
        };
        let r = unsafe native_symlink(t.raw.asRaw(), l.raw.asRaw());
        let error = IoError.last();
        t.free();
        l.free();
        if r < 0 { .Err(error.at(path: link.string)) } else { .Ok(()) }
    }

    /// Where the symbolic link `path` points, as stored in the link.
    public static func read(path: Path) -> Result[Path, IoError] {
        var capacity: Int64 = 256;
        let cpath = try path.toNativePath();
        loop {
            let buffer = unsafe malloc(capacity);
            let n = unsafe native_read_link(cpath.raw.asRaw(), buffer, capacity);
            let error = IoError.last();
            if n < 0 {
                unsafe free(buffer);
                cpath.free();
                return .Err(error.at(path: path.string))
            }
            if n < capacity {
                unsafe buffer.cast[UInt8]().offset(by: n).write(0);
                let target = pathFromNative(buffer);
                unsafe free(buffer);
                cpath.free();
                return .Ok(target)
            }
            unsafe free(buffer);
            capacity = capacity * 2
        }
    }
}
