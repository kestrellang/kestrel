// File and OpenOptions.

module std.files

import std.numeric.(Int32, Int64, UInt8, UInt32)
import std.collections.(Array, Slice)
import std.core.(Bool)
import std.result.(Result, Optional)
import std.streams.(IoError, Readable, Writable, Seekable, Seek, FileDescriptor)
import std.ffi.(malloc, free)

/// An open file. Closed when dropped.
///
/// It is `Readable`, `Writable` and `Seekable`, so every stream method
/// works on it: `readAll`, `readText`, `writeAll`, `buffered()` and the
/// rest. For a whole file at once, `readText(path)` and `writeText(path,
/// text)` are shorter.
///
/// # Examples
///
/// ```
/// var log = try File.open("app.log", OpenOptions.appending);
/// try log.write(line: "started");
///
/// var input = try File.open("data.csv").buffered();
/// while let .Some(line) = try input.readLine() { ... }
/// ```
public struct File: Readable, Writable, Seekable, not Copyable {
    var fd: Int32

    /// Opens `path` for reading.
    public static func open(path: Path) -> Result[File, IoError] {
        File.open(path, OpenOptions.reading)
    }

    /// Opens `path` for writing, creating it or emptying it.
    public static func create(path: Path) -> Result[File, IoError] {
        File.open(path, OpenOptions.writing)
    }

    /// Opens `path` as `options` say.
    ///
    /// # Errors
    ///
    /// `NotFound` if the file doesn't exist and `options` don't create it,
    /// `AlreadyExists` if `createNew` is set and it does.
    public static func open(path: Path, options: OpenOptions) -> Result[File, IoError] {
        let cpath = path.text.toCString();
        let fd = unsafe native_open(cpath.raw.asRaw(), options.nativeFlags(), Int32(from: options.permissions.mode));
        let error = IoError.last();
        cpath.free();
        if fd < 0 {
            return .Err(error)
        }
        .Ok(File(unsafeDescriptor: FileDescriptor(fd)))
    }

    /// @name From Descriptor
    /// Takes ownership of an open descriptor: the `File` closes it.
    public init(unsafeDescriptor descriptor: FileDescriptor) {
        self.fd = descriptor.value
    }

    /// The descriptor, still owned by this file.
    public var rawDescriptor: FileDescriptor { FileDescriptor(self.fd) }

    /// Reads up to `buf.count` bytes from the current position.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_read(self.fd, buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Writes up to `buf.count` bytes at the current position (or the end,
    /// when opened to append).
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_write(self.fd, buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Nothing to flush: `File` doesn't buffer. Use `sync()` to reach the
    /// disk, or `buffered()` to batch small writes.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }

    /// Waits until the contents reach the storage device (`fsync`).
    public mutating func sync() -> Result[(), IoError] {
        if unsafe native_sync(self.fd) < 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// Moves the position; returns the new offset from the start. Seeking
    /// past the end is allowed, and a write there extends the file.
    public mutating func seek(to position: Seek) -> Result[Int64, IoError] {
        let (offset, whence) = match position {
            .Start(n) => (n, 0),
            .Current(n) => (n, 1),
            .End(n) => (n, 2)
        };
        let at = unsafe native_seek(self.fd, offset, whence);
        if at < 0 {
            return .Err(IoError.last())
        }
        .Ok(at)
    }

    /// Cuts the file to `length` bytes, or extends it with zeros.
    public mutating func setLength(length: Int64) -> Result[(), IoError] {
        if unsafe native_set_length(self.fd, length) < 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// The open file's metadata (`fstat`).
    public func metadata() -> Result[Metadata, IoError] {
        let out = unsafe malloc(40);
        let r = unsafe native_file_metadata(self.fd, out);
        let error = IoError.last();
        let result = if r < 0 { .Err(error) } else { .Ok(metadataFrom(out)) };
        unsafe free(out);
        result
    }

    /// Closes the file and reports an error from `close`, which dropping
    /// the file can't.
    public consuming func close() -> Result[(), IoError] {
        let fd = self.fd;
        self.fd = -1;
        if unsafe native_close(fd) < 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// Gives up ownership: the descriptor stays open and the caller closes
    /// it.
    public consuming func intoRawDescriptor() -> FileDescriptor {
        let fd = self.fd;
        self.fd = -1;
        FileDescriptor(fd)
    }

    deinit {
        if self.fd >= 0 {
            unsafe native_close(self.fd);
        }
    }
}

/// How `File.open(path, options)` opens a file.
///
/// Start from one of the presets and change fields as needed:
///
/// ```
/// var options = OpenOptions.writing;
/// options.createNew = true;          // fail if the file exists
/// var file = try File.open("out.txt", options);
/// ```
public struct OpenOptions {
    /// Allow reading.
    public var read: Bool
    /// Allow writing.
    public var write: Bool
    /// Write at the end, whatever the position.
    public var append: Bool
    /// Empty the file on open.
    public var truncate: Bool
    /// Create the file if it doesn't exist.
    public var create: Bool
    /// Create the file, failing with `AlreadyExists` if it exists.
    public var createNew: Bool
    /// Permissions for a newly created file (before the umask).
    public var permissions: Permissions

    /// @name All Off
    /// Every option off, permissions `0o644`.
    public init() {
        self.read = false;
        self.write = false;
        self.append = false;
        self.truncate = false;
        self.create = false;
        self.createNew = false;
        self.permissions = Permissions.fileDefault;
    }

    /// Read only.
    public static var reading: OpenOptions {
        var o = OpenOptions();
        o.read = true;
        o
    }

    /// Write, creating the file or emptying it.
    public static var writing: OpenOptions {
        var o = OpenOptions();
        o.write = true;
        o.create = true;
        o.truncate = true;
        o
    }

    /// Append, creating the file if needed.
    public static var appending: OpenOptions {
        var o = OpenOptions();
        o.append = true;
        o.create = true;
        o
    }

    /// Read and write an existing file, from the start.
    public static var readingAndWriting: OpenOptions {
        var o = OpenOptions();
        o.read = true;
        o.write = true;
        o
    }

    /// The option bits `kestrel_file_open` takes.
    func nativeFlags() -> Int32 {
        var bits: Int32 = 0;
        if self.read { bits = bits + 1 }
        if self.write { bits = bits + 2 }
        if self.append { bits = bits + 4 }
        if self.truncate { bits = bits + 8 }
        if self.create { bits = bits + 16 }
        if self.createNew { bits = bits + 32 }
        bits
    }
}
