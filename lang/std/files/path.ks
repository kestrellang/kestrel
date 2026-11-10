// Path: a file system path, and lexical operations on it.

module std.files

import std.numeric.(Int64, UInt8)
import std.collections.(Array, Slice)
import std.text.(String, Formatter, Formattable, FormatOptions, _writePadded)
import std.core.(Bool, Equatable, Hashable, Hasher, Cloneable, ExpressibleByStringLiteral, Divisible)
import std.result.(Optional, Result)
import std.streams.(IoError)
import std.ffi.(CString, free, malloc, memcpy)
import std.memory.(RawPointer)

/// A file system path.
///
/// A `Path` is the bytes as written. Paths are byte strings, usually but
/// not necessarily UTF-8, so non-UTF-8 file names round-trip. Everything
/// here except the file system shorthands at the end is lexical: it looks
/// at the text and never touches the disk. A string literal is a `Path`
/// wherever one is expected, and `/` joins two.
///
/// # Examples
///
/// ```
/// let config = Environment.homeDirectory() / ".config" / "app.toml";
/// config.name          // .Some("app.toml")
/// config.extension     // .Some("toml")
/// if config.exists() { ... }
/// ```
public struct Path: Equatable, Hashable, Cloneable, Formattable, ExpressibleByStringLiteral, Divisible[Path] {
    type Output = Path

    // The bytes as written: usually UTF-8, but not necessarily, so a
    // non-UTF-8 name read from the system opens again unchanged.
    var data: Array[UInt8]

    /// @name From Text
    /// The path `text`, as written.
    public init(text: String) {
        self.data = text.asByteSlice().toOwned()
    }

    /// @name From Bytes
    /// The path made of exactly these bytes, which needn't be UTF-8.
    public init(bytes bytes: Array[UInt8]) {
        self.data = bytes
    }

    /// @name String Literal
    /// A path literal, as in `File.open("data.txt")`.
    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.data = String(stringLiteral: ptr, length).asByteSlice().toOwned()
    }

    /// The path as text. Bytes that aren't UTF-8 become U+FFFD, so use
    /// `bytes` when the exact name matters.
    public var string: String { textOf(self.data.asSlice()) }

    /// The path's bytes, exactly, borrowed from the path. Call
    /// `toOwned()` for an array of your own.
    public var bytes: &Slice[UInt8] { self.data.asSlice() }

    /// `true` for the empty path.
    public var isEmpty: Bool { self.data.isEmpty }

    /// `true` if the path starts at the root (`/`).
    public var isAbsolute: Bool {
        self.data.count > 0 and unsafe self.data(unchecked: 0) == slash()
    }

    /// The path without its last component: `/a/b` → `/a`, `/a` → `/`.
    /// `None` for `/`, the empty path, or a single relative component.
    public var parent: Optional[Path] {
        let b = self.data.asSlice();
        let count = self.trimmedCount();
        var i = count - 1;
        while i >= 0 and unsafe b(unchecked: i) != slash() {
            i = i - 1
        }
        if i < 0 or count == 1 {
            return .None
        }
        var end = i;
        while end > 0 and unsafe b(unchecked: end - 1) == slash() {
            end = end - 1
        }
        if end == 0 {
            return .Some(Path("/"))
        }
        .Some(Path(bytes: b(0..<end).toOwned()))
    }

    /// The last component: `b.tar.gz` for `/a/b.tar.gz`. `None` for `/`,
    /// the empty path, or a path ending in `.` or `..`.
    public var name: Optional[String] {
        let (start, end) = match self.nameRange() { .Some(r) => r, .None => return .None };
        .Some(textOf(self.data(start..<end)))
    }

    /// The name without its extension: `b.tar` for `b.tar.gz`.
    public var stem: Optional[String] {
        let (start, end) = match self.nameRange() { .Some(r) => r, .None => return .None };
        let stop = match self.extensionDot(start, end) { .Some(dot) => dot, .None => end };
        .Some(textOf(self.data(start..<stop)))
    }

    /// What follows the name's last dot: `gz` for `b.tar.gz`. A leading dot
    /// (`.profile`) doesn't start an extension.
    public var extension: Optional[String] {
        let (start, end) = match self.nameRange() { .Some(r) => r, .None => return .None };
        let dot = match self.extensionDot(start, end) { .Some(d) => d, .None => return .None };
        .Some(textOf(self.data((dot + 1)..<end)))
    }

    /// The components in order. An absolute path starts with `/`; empty
    /// components and `.` are dropped.
    public func components() -> Array[String] {
        var parts = Array[String]();
        if self.isAbsolute {
            parts.append("/")
        }
        for (start, end) in self.componentRanges() {
            parts.append(textOf(self.data(start..<end)))
        }
        parts
    }

    /// `other` appended as a further component. An absolute `other`
    /// replaces this path, as in Rust and Python.
    public func joined(with other: Path) -> Path {
        if other.isAbsolute or self.data.isEmpty {
            return other.clone()
        }
        var joined = Array[UInt8](capacity: self.data.count + 1 + other.data.count);
        joined.append(contentsOf: self.data);
        if unsafe self.data(unchecked: self.data.count - 1) != slash() {
            joined.append(slash())
        }
        joined.append(contentsOf: other.data);
        Path(bytes: joined)
    }

    /// `self / other` is `self.joined(with: other)`.
    public consuming func divide(consuming other: Path) -> Path {
        self.joined(with: other)
    }

    /// The path with its extension replaced by `ext`, or added if it has
    /// none. An empty `ext` removes the extension.
    public func replacingExtension(with ext: String) -> Path {
        let (start, end) = match self.nameRange() { .Some(r) => r, .None => return self.clone() };
        let stop = match self.extensionDot(start, end) { .Some(dot) => dot, .None => end };
        // Everything up to the stem stays byte for byte; trailing slashes go.
        var result = Array[UInt8]();
        result.append(contentsOf: self.data(0..<stop));
        if not ext.isEmpty {
            result.append(46);
            result.append(contentsOf: ext.asByteSlice())
        }
        Path(bytes: result)
    }

    /// The path with `.` dropped and `..` folded into the component before
    /// it, without touching the file system. A symbolic link can make this
    /// differ from `canonical()`.
    public func normalized() -> Path {
        var kept = Array[(Int64, Int64)]();
        for (start, end) in self.componentRanges() {
            if self.isDotDot(start, end) {
                if kept.count > 0 {
                    let (ls, le) = unsafe kept(unchecked: kept.count - 1);
                    if not self.isDotDot(ls, le) {
                        kept.popLast();
                        continue
                    }
                }
                if self.isAbsolute {
                    continue
                }
            }
            kept.append((start, end))
        }
        var result = Array[UInt8]();
        if self.isAbsolute {
            result.append(slash())
        }
        var first = true;
        for (start, end) in kept {
            if not first {
                result.append(slash())
            }
            result.append(contentsOf: self.data(start..<end));
            first = false
        }
        if result.isEmpty {
            result.append(46)
        }
        Path(bytes: result)
    }

    // ========================================================================
    // FILE SYSTEM SHORTHANDS
    // ========================================================================

    /// `true` if something exists at this path (following symbolic links).
    public func exists() -> Bool {
        Metadata.of(self).isOk
    }

    /// `true` if this path is a regular file (following symbolic links).
    public func isFile() -> Bool {
        match Metadata.of(self) {
            .Ok(m) => m.kind == .File,
            .Err(_) => false
        }
    }

    /// `true` if this path is a directory (following symbolic links).
    public func isDirectory() -> Bool {
        match Metadata.of(self) {
            .Ok(m) => m.kind == .Directory,
            .Err(_) => false
        }
    }

    /// The metadata of what's at this path, following symbolic links.
    public func metadata() -> Result[Metadata, IoError] {
        Metadata.of(self)
    }

    /// The absolute path with every symbolic link, `.` and `..` resolved
    /// (`realpath(3)`). The path must exist.
    public func canonical() -> Result[Path, IoError] {
        let cpath = try self.toNativePath();
        let resolved = unsafe native_canonical(cpath.raw.asRaw());
        let error = IoError.last();
        cpath.free();
        if resolved.isNull {
            return .Err(error.at(path: self.string))
        }
        let result = pathFromNative(resolved);
        unsafe free(resolved);
        .Ok(result)
    }

    // ========================================================================
    // CONFORMANCES
    // ========================================================================

    /// Same bytes. `a/b` and `a//b` are different paths; compare
    /// `normalized()` paths to ignore that.
    public func isEqual(to other: Path) -> Bool { self.data == other.data }

    /// Hashes the bytes.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.data.hash(into: hasher)
    }

    /// Copies the path.
    public func clone() -> Path { Path(bytes: self.data.clone()) }

    /// Writes the path as text (non-UTF-8 bytes as U+FFFD).
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, self.string, options)
    }

    // ========================================================================
    // INTERNALS
    // ========================================================================

    /// The path as a NUL-terminated C string for a system call;
    /// `InvalidInput` if it holds a NUL byte. The caller frees it.
    public func toNativePath() -> Result[CString, IoError] {
        let count = self.data.count;
        for i in 0..<count {
            if unsafe self.data(unchecked: i) == 0 {
                return .Err(IoError(kind: .InvalidInput))
            }
        }
        let raw = unsafe malloc(count + 1);
        if raw.isNull {
            return .Err(IoError(kind: .OutOfMemory))
        }
        if count > 0 {
            let _ = unsafe memcpy(raw, self.data.asSlice().pointer.asRaw(), count);
        }
        unsafe raw.cast[UInt8]().offset(by: count).write(0);
        .Ok(CString(raw: raw.cast[UInt8]()))
    }

    /// The byte count without trailing slashes, except a lone `/`.
    func trimmedCount() -> Int64 {
        var n = self.data.count;
        while n > 1 and unsafe self.data(unchecked: n - 1) == slash() {
            n = n - 1
        }
        n
    }

    /// The byte range of the last component, unless it's `.` or `..`.
    func nameRange() -> Optional[(Int64, Int64)] {
        let count = self.trimmedCount();
        var i = count;
        while i > 0 and unsafe self.data(unchecked: i - 1) != slash() {
            i = i - 1
        }
        if i == count or self.isDot(i, count) or self.isDotDot(i, count) {
            return .None
        }
        .Some((i, count))
    }

    /// The index of the dot that starts the extension of the name in
    /// `[start, end)`. A dot at the name's first byte (`.profile`) doesn't count.
    func extensionDot(start: Int64, end: Int64) -> Optional[Int64] {
        var i = end - 1;
        while i > start {
            if unsafe self.data(unchecked: i) == 46 {
                return .Some(i)
            }
            i = i - 1
        }
        .None
    }

    /// Byte ranges of the components, skipping empty ones and `.`.
    func componentRanges() -> Array[(Int64, Int64)] {
        var ranges = Array[(Int64, Int64)]();
        let count = self.data.count;
        var start: Int64 = 0;
        var i: Int64 = 0;
        while i <= count {
            if i == count or unsafe self.data(unchecked: i) == slash() {
                if i > start and not self.isDot(start, i) {
                    ranges.append((start, i))
                }
                start = i + 1
            }
            i = i + 1
        }
        ranges
    }

    func isDot(start: Int64, end: Int64) -> Bool {
        end - start == 1 and unsafe self.data(unchecked: start) == 46
    }

    func isDotDot(start: Int64, end: Int64) -> Bool {
        end - start == 2 and unsafe self.data(unchecked: start) == 46 and unsafe self.data(unchecked: start + 1) == 46
    }
}

/// `/`.
internal func slash() -> UInt8 { 47 }

/// `bytes` as a `String`, with bytes that aren't UTF-8 as U+FFFD.
internal func textOf(bytes: Slice[UInt8]) -> String {
    String(fromUtf8Lossy: bytes)
}

/// The text of a NUL-terminated C string, with invalid UTF-8 replaced by
/// U+FFFD.
internal func textFromNative(raw: RawPointer) -> String {
    String(from: CString(raw: raw.cast[UInt8]()))
}

/// The path spelled by a NUL-terminated C string, byte for byte.
internal func pathFromNative(raw: RawPointer) -> Path {
    let ptr = raw.cast[UInt8]();
    let count = CString(raw: ptr).length;
    Path(bytes: unsafe Slice.lend(start: ptr, count: count).toOwned())
}
