// Directories: creating, removing and listing them.

module std.files

import std.numeric.(Int32, Int64, UInt8)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.result.(Result, Optional)
import std.iter.(Iterator)
import std.memory.(RawPointer, Pointer)
import std.streams.(IoError)
import std.ffi.(malloc, free)

/// Creating, removing and listing directories.
///
/// # Examples
///
/// ```
/// try Directory.createAll("build/out");
/// for entry in try Directory.entries("src") {
///     let entry = try entry;
///     if entry.kind == .File { println(entry.path) }
/// }
/// ```
public enum Directory {
    /// Creates the directory `path`. Its parent must exist.
    public static func create(path: Path) -> Result[(), IoError] {
        withNativePath(path, { (p) in native_mkdir(p, 493) })
    }

    /// Creates `path` and any missing parents (`mkdir -p`). Succeeds if the
    /// directory already exists.
    public static func createAll(path: Path) -> Result[(), IoError] {
        var current = if path.isAbsolute { Path("/") } else { Path("") };
        for part in path.components() {
            if part == "/" {
                continue
            }
            current = current.joined(with: Path(part));
            match Directory.create(current) {
                .Ok(_) => {},
                .Err(e) => {
                    if not (e.kind == .AlreadyExists and current.isDirectory()) {
                        return .Err(e)
                    }
                }
            }
        }
        .Ok(())
    }

    /// Removes the directory `path`, which must be empty.
    public static func remove(path: Path) -> Result[(), IoError] {
        withNativePath(path, { (p) in native_rmdir(p) })
    }

    /// Removes `path` and everything in it (`rm -r`). Symbolic links are
    /// removed, never followed.
    public static func removeAll(path: Path) -> Result[(), IoError] {
        for entry in try Directory.entries(path) {
            let entry = try entry;
            if entry.kind == .Directory {
                try Directory.removeAll(entry.path)
            } else {
                // A bare `remove` here would name `Directory.remove`, so
                // unlink directly.
                try withNativePath(entry.path, { (p) in native_unlink(p) })
            }
        }
        Directory.remove(path)
    }

    /// The entries of `path`, read lazily, without `.` and `..`, in the
    /// order the file system returns them.
    public static func entries(path: Path) -> Result[DirectoryEntries, IoError] {
        let cpath = path.text.toCString();
        let dir = native_directory_open(cpath.raw.asRaw());
        let error = IoError.last();
        cpath.free();
        if dir.isNull {
            return .Err(error)
        }
        .Ok(DirectoryEntries(dir: dir, base: path.clone()))
    }

    /// Creates a new, empty directory with a unique name starting with
    /// `prefix`, in the system's temporary directory (`mkdtemp`). The
    /// caller removes it.
    public static func createTemporary(prefix prefix: String) -> Result[Path, IoError] {
        var template = temporaryRoot();
        if not template.ends(with: "/") {
            template.append("/")
        }
        template.append(prefix);
        template.append("XXXXXX");
        let ctemplate = template.toCString();
        let r = native_make_temporary_directory(ctemplate.raw.asRaw());
        let error = IoError.last();
        let made = textFromNative(ctemplate.raw.asRaw());
        ctemplate.free();
        if r < 0 {
            return .Err(error)
        }
        .Ok(Path(made))
    }
}

/// `$TMPDIR`, or `/tmp`.
internal func temporaryRoot() -> String {
    let name = "TMPDIR".toCString();
    let raw = native_getenv(name.raw.asRaw());
    name.free();
    if raw.isNull { return "/tmp" }
    textFromNative(raw)
}

@extern(.C, mangleName: "getenv")
internal func native_getenv(name: RawPointer) -> RawPointer

/// One entry of a directory listing.
public struct DirectoryEntry {
    /// The entry's path: the listed directory joined with `name`.
    public var path: Path
    /// The entry's name within the directory.
    public var name: String
    /// What the entry is. A symbolic link is `.SymbolicLink`, not what it
    /// points to.
    public var kind: FileKind
}

/// The entries of a directory, read one at a time. Each is a
/// `Result[DirectoryEntry, IoError]`; iteration ends after an error.
public struct DirectoryEntries: Iterator, not Copyable {
    type Item = Result[DirectoryEntry, IoError]

    var dir: RawPointer
    var base: Path

    /// The next entry.
    public mutating func next() -> Optional[Result[DirectoryEntry, IoError]] {
        if self.dir.isNull {
            return .None
        }
        let cells = malloc(16);
        let r = native_directory_next(self.dir, cells, cells.cast[UInt8]().offset(by: 8).asRaw());
        let error = IoError.last();
        if r <= 0 {
            free(cells);
            native_directory_close(self.dir);
            self.dir = RawPointer.nullPointer();
            return if r == 0 { .None } else { .Some(.Err(error)) }
        }
        let name = textFromNative(cells.cast[RawPointer]().read());
        let code = Int64(from: cells.cast[UInt8]().offset(by: 8).cast[Int32]().read());
        free(cells);
        let path = self.base.joined(with: Path(name.clone()));
        // d_type is unknown on some file systems: ask lstat.
        let kind = if code == 4 {
            match symbolicLinkMetadata(path) {
                .Ok(m) => m.kind,
                .Err(e) => return .Some(.Err(e))
            }
        } else {
            FileKind.of(code)
        };
        .Some(.Ok(DirectoryEntry(path: path, name: name, kind: kind)))
    }

    deinit {
        if not self.dir.isNull {
            native_directory_close(self.dir);
        }
    }
}
