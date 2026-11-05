// Removing, renaming, copying and linking files.

module std.files

import std.numeric.(Int32, Int64, UInt8)
import std.collections.(Array)
import std.text.(String)
import std.result.(Result)
import std.memory.(RawPointer)
import std.streams.(IoError, copy as copyStream)
import std.ffi.(malloc, free)

/// Removes the file or symbolic link `path`. Use `Directory.remove` for a
/// directory.
public func remove(path: Path) -> Result[(), IoError] {
    withNativePath(path, { (p) in native_unlink(p) })
}

/// Renames `source` to `destination`, replacing a file already there.
/// Both must be on the same file system (`CrossesDevices` otherwise).
public func rename(source: Path, to destination: Path) -> Result[(), IoError] {
    let from = source.text.toCString();
    let to = destination.text.toCString();
    let r = native_rename(from.raw.asRaw(), to.raw.asRaw());
    let error = IoError.last();
    from.free();
    to.free();
    if r < 0 { .Err(error) } else { .Ok(()) }
}

/// Copies the file `source` to `destination`, with its permissions.
/// Returns the number of bytes copied.
public func copy(source: Path, to destination: Path) -> Result[Int64, IoError] {
    var input = try File.open(source);
    let permissions = (try input.metadata()).permissions;
    var options = OpenOptions.writing;
    options.permissions = permissions;
    var output = try File.open(destination, options);
    let n = try copyStream(from: input, to: output);
    try setPermissions(destination, permissions);
    .Ok(n)
}

/// Creates a symbolic link at `link` that points to `target`. `target`
/// isn't checked; it may not exist.
public func createSymbolicLink(at link: Path, to target: Path) -> Result[(), IoError] {
    let t = target.text.toCString();
    let l = link.text.toCString();
    let r = native_symlink(t.raw.asRaw(), l.raw.asRaw());
    let error = IoError.last();
    t.free();
    l.free();
    if r < 0 { .Err(error) } else { .Ok(()) }
}

/// Where the symbolic link `path` points, as stored in the link.
public func readSymbolicLink(path: Path) -> Result[Path, IoError] {
    var capacity: Int64 = 256;
    let cpath = path.text.toCString();
    loop {
        let buffer = malloc(capacity);
        let n = native_read_link(cpath.raw.asRaw(), buffer, capacity);
        let error = IoError.last();
        if n < 0 {
            free(buffer);
            cpath.free();
            return .Err(error)
        }
        if n < capacity {
            buffer.cast[UInt8]().offset(by: n).write(0);
            let text = textFromNative(buffer);
            free(buffer);
            cpath.free();
            return .Ok(Path(text))
        }
        free(buffer);
        capacity = capacity * 2
    }
}

/// Sets the permission bits of `path` (`chmod`).
public func setPermissions(path: Path, permissions: Permissions) -> Result[(), IoError] {
    withNativePath(path, { (p) in native_chmod(p, Int32(from: permissions.mode)) })
}
