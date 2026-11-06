// Path: a file system path, and lexical operations on it.

module std.files

import std.numeric.(Int64, UInt8)
import std.collections.(Array, Slice)
import std.text.(String, Formatter, Formattable, FormatOptions, _writePadded)
import std.core.(Bool, Equatable, Hashable, Hasher, Cloneable, ExpressibleByStringLiteral, Divisible)
import std.result.(Optional, Result)
import std.streams.(IoError)
import std.ffi.(CString, free)
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

    var text: String

    /// @name From Text
    /// The path `text`, as written.
    public init(text: String) {
        self.text = text
    }

    /// @name String Literal
    /// A path literal, as in `File.open("data.txt")`.
    public init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64) {
        self.text = String(stringLiteral: ptr, length)
    }

    /// The path as text.
    public var string: String { self.text.clone() }

    /// `true` for the empty path.
    public var isEmpty: Bool { self.text.isEmpty }

    /// `true` if the path starts at the root (`/`).
    public var isAbsolute: Bool { self.text.starts(with: "/") }

    /// The path without its last component: `/a/b` → `/a`, `/a` → `/`.
    /// `None` for `/`, the empty path, or a single relative component.
    public var parent: Optional[Path] {
        let b = self.trimmed();
        var i = b.count - 1;
        while i >= 0 and unsafe b(unchecked: i) != slash() {
            i = i - 1
        }
        if i < 0 or b.count == 1 {
            return .None
        }
        var end = i;
        while end > 0 and unsafe b(unchecked: end - 1) == slash() {
            end = end - 1
        }
        if end == 0 {
            return .Some(Path("/"))
        }
        .Some(Path(textOf(b(0..<end))))
    }

    /// The last component: `b.tar.gz` for `/a/b.tar.gz`. `None` for `/`,
    /// the empty path, or a path ending in `..`.
    public var name: Optional[String] {
        let b = self.trimmed();
        var i = b.count;
        while i > 0 and unsafe b(unchecked: i - 1) != slash() {
            i = i - 1
        }
        if i == b.count {
            return .None
        }
        let last = textOf(b(i..<b.count));
        if last == "." or last == ".." { .None } else { .Some(last) }
    }

    /// The name without its extension: `b.tar` for `b.tar.gz`.
    public var stem: Optional[String] {
        let name = match self.name { .Some(n) => n, .None => return .None };
        match extensionDot(name) {
            .Some(dot) => .Some(textOf(textBytes(name)(0..<dot))),
            .None => .Some(name)
        }
    }

    /// What follows the name's last dot: `gz` for `b.tar.gz`. A leading dot
    /// (`.profile`) doesn't start an extension.
    public var extension: Optional[String] {
        let name = match self.name { .Some(n) => n, .None => return .None };
        let dot = match extensionDot(name) { .Some(d) => d, .None => return .None };
        let b = textBytes(name);
        .Some(textOf(b((dot + 1)..<b.count)))
    }

    /// The components in order. An absolute path starts with `/`; empty
    /// components and `.` are dropped.
    public func components() -> Array[String] {
        var parts = Array[String]();
        if self.isAbsolute {
            parts.append("/")
        }
        let b = textBytes(self.text);
        var start: Int64 = 0;
        var i: Int64 = 0;
        while i <= b.count {
            if i == b.count or unsafe b(unchecked: i) == slash() {
                if i > start {
                    let part = textOf(b(start..<i));
                    if part != "." {
                        parts.append(part)
                    }
                }
                start = i + 1
            }
            i = i + 1
        }
        parts
    }

    /// `other` appended as a further component. An absolute `other`
    /// replaces this path, as in Rust and Python.
    public func joined(with other: Path) -> Path {
        if other.isAbsolute or self.text.isEmpty {
            return other.clone()
        }
        var text = self.text.clone();
        if not text.ends(with: "/") {
            text.append("/")
        }
        text.append(other.text);
        Path(text)
    }

    /// `self / other` is `self.joined(with: other)`.
    public consuming func divide(consuming other: Path) -> Path {
        self.joined(with: other)
    }

    /// The path with its extension replaced by `ext`, or added if it has
    /// none. An empty `ext` removes the extension.
    public func replacingExtension(with ext: String) -> Path {
        let stem = match self.stem {
            .Some(s) => s,
            .None => return self.clone()
        };
        var name = stem;
        if not ext.isEmpty {
            name.append(".");
            name.append(ext)
        }
        match self.parent {
            .Some(parent) => parent.joined(with: Path(name)),
            .None => if self.isAbsolute { Path("/" + name) } else { Path(name) }
        }
    }

    /// The path with `.` dropped and `..` folded into the component before
    /// it, without touching the file system. A symbolic link can make this
    /// differ from `canonical()`.
    public func normalized() -> Path {
        var kept = Array[String]();
        for part in self.components() {
            if part == "/" {
                continue
            }
            if part == ".." {
                if kept.count > 0 and unsafe kept(unchecked: kept.count - 1) != ".." {
                    kept.pop();
                    continue
                }
                if self.isAbsolute {
                    continue
                }
            }
            kept.append(part)
        }
        var text: String = if self.isAbsolute { "/" } else { "" };
        var first = true;
        for part in kept {
            if not first {
                text.append("/")
            }
            text.append(part);
            first = false
        }
        if text.isEmpty and not self.isAbsolute {
            text.append(".")
        }
        Path(text)
    }

    // ========================================================================
    // FILE SYSTEM SHORTHANDS
    // ========================================================================

    /// `true` if something exists at this path (following symbolic links).
    public func exists() -> Bool {
        metadata(self).isOk
    }

    /// `true` if this path is a regular file (following symbolic links).
    public func isFile() -> Bool {
        match metadata(self) {
            .Ok(m) => m.kind == .File,
            .Err(_) => false
        }
    }

    /// `true` if this path is a directory (following symbolic links).
    public func isDirectory() -> Bool {
        match metadata(self) {
            .Ok(m) => m.kind == .Directory,
            .Err(_) => false
        }
    }

    /// The metadata of what's at this path, following symbolic links.
    public func metadata() -> Result[Metadata, IoError] {
        metadata(self)
    }

    /// The absolute path with every symbolic link, `.` and `..` resolved
    /// (`realpath(3)`). The path must exist.
    public func canonical() -> Result[Path, IoError] {
        let cpath = try self.text.toNativeString();
        let resolved = unsafe native_canonical(cpath.raw.asRaw());
        let error = IoError.last();
        cpath.free();
        if resolved.isNull {
            return .Err(error)
        }
        let text = textFromNative(resolved);
        unsafe free(resolved);
        .Ok(Path(text))
    }

    // ========================================================================
    // CONFORMANCES
    // ========================================================================

    /// Same bytes. `a/b` and `a//b` are different paths; compare
    /// `normalized()` paths to ignore that.
    public func isEqual(to other: Path) -> Bool { self.text == other.text }

    /// Hashes the bytes.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.text.hash(into: hasher)
    }

    /// Copies the path.
    public func clone() -> Path { Path(self.text.clone()) }

    /// Writes the path as text.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, self.text.clone(), options)
    }

    /// The bytes without trailing slashes, except a lone `/`.
    func trimmed() -> Array[UInt8] {
        var b = textBytes(self.text);
        while b.count > 1 and unsafe b(unchecked: b.count - 1) == slash() {
            b.pop();
        }
        b
    }
}

/// `/`.
internal func slash() -> UInt8 { 47 }

/// The bytes of `text`.
internal func textBytes(text: String) -> Array[UInt8] {
    let raw = text.asByteSlice();
    unsafe Slice.lend(start: raw.pointer, count: raw.count).toArray()
}

/// `bytes` as a `String`, kept as is even when not UTF-8.
internal func textOf(bytes: Slice[UInt8]) -> String {
    String(fromUtf8Unchecked: bytes)
}

/// The text of a NUL-terminated C string, with invalid UTF-8 replaced by
/// U+FFFD.
internal func textFromNative(raw: RawPointer) -> String {
    String(from: CString(raw: raw.cast[UInt8]()))
}

/// The index of the dot that starts `name`'s extension, if any. A dot at
/// index 0 (`.profile`) doesn't count.
internal func extensionDot(name: String) -> Optional[Int64] {
    let b = textBytes(name);
    var i = b.count - 1;
    while i > 0 {
        if unsafe b(unchecked: i) == 46 {
            return .Some(i)
        }
        i = i - 1
    }
    .None
}
