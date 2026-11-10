# std.files

## enum `Directory`

```kestrel
public enum Directory
```

Creating, removing and listing directories.

### Examples

```
try Directory.createAll("build/out");
for entry in try Directory.entries("src") {
    let entry = try entry;
    if entry.kind == .File { println(entry.path) }
}
```

_Defined in `lang/std/files/directory.ks`._

### Members

#### function `create`

```kestrel
public static func create(Path) -> Result[(), IoError]
```

Creates the directory `path`. Its parent must exist.

_Defined in `lang/std/files/directory.ks`._

#### function `createAll`

```kestrel
public static func createAll(Path) -> Result[(), IoError]
```

Creates `path` and any missing parents (`mkdir -p`). Succeeds if the
directory already exists.

_Defined in `lang/std/files/directory.ks`._

#### function `createTemporary`

```kestrel
public static func createTemporary(prefix: String) -> Result[Path, IoError]
```

Creates a new, empty directory with a unique name starting with
`prefix`, in the system's temporary directory (`mkdtemp`). The
caller removes it.

_Defined in `lang/std/files/directory.ks`._

#### function `entries`

```kestrel
public static func entries(Path) -> Result[DirectoryEntries, IoError]
```

The entries of `path`, read lazily, without `.` and `..`, in the
order the file system returns them.

_Defined in `lang/std/files/directory.ks`._

#### function `remove`

```kestrel
public static func remove(Path) -> Result[(), IoError]
```

Removes the directory `path`, which must be empty.

_Defined in `lang/std/files/directory.ks`._

#### function `removeAll`

```kestrel
public static func removeAll(Path) -> Result[(), IoError]
```

Removes `path` and everything in it (`rm -r`). Symbolic links are
removed, never followed, and that includes `path` itself: if it's a
link to a directory, the link goes and the directory stays.

_Defined in `lang/std/files/directory.ks`._

## struct `DirectoryEntries`

```kestrel
public struct DirectoryEntries: not Copyable { /* private fields */ }
```

The entries of a directory, read one at a time. Each is a
`Result[DirectoryEntry, IoError]`; iteration ends after an error.

_Defined in `lang/std/files/directory.ks`._

### Members

#### field `base`

```kestrel
var base: Path
```

_Defined in `lang/std/files/directory.ks`._

#### field `dir`

```kestrel
var dir: RawPointer
```

_Defined in `lang/std/files/directory.ks`._

### Implements `Iterator`

#### typealias `Item`

```kestrel
type Item = Result[DirectoryEntry, IoError]
```

_Defined in `lang/std/files/directory.ks`._

#### function `next`

```kestrel
public mutating func next() -> Optional[Result[DirectoryEntry, IoError]]
```

The next entry.

_Defined in `lang/std/files/directory.ks`._

## struct `DirectoryEntry`

```kestrel
public struct DirectoryEntry { /* private fields */ }
```

One entry of a directory listing.

_Defined in `lang/std/files/directory.ks`._

### Members

#### field `kind`

```kestrel
public var kind: FileKind
```

What the entry is. A symbolic link is `.SymbolicLink`, not what it
points to.

_Defined in `lang/std/files/directory.ks`._

#### field `name`

```kestrel
public var name: String
```

The entry's name within the directory.

_Defined in `lang/std/files/directory.ks`._

#### field `path`

```kestrel
public var path: Path
```

The entry's path: the listed directory joined with `name`.

_Defined in `lang/std/files/directory.ks`._

## struct `File`

```kestrel
public struct File: not Copyable { /* private fields */ }
```

An open file. Closed when dropped.

It is `Readable`, `Writable` and `Seekable`, so every stream method
works on it: `readAll`, `readText`, `writeAll`, `buffered()` and the
rest. For a whole file at once, `readText(path)` and `writeText(path,
text)` are shorter.

### Examples

```
var log = try File.open("app.log", OpenOptions.appending);
try log.write(line: "started");

var input = try File.open("data.csv").buffered();
while let .Some(line) = try input.readLine() { ... }
```

_Defined in `lang/std/files/file.ks`._

### Members

#### initializer `From Descriptor`

```kestrel
public init(unsafeDescriptor: FileDescriptor)
```

Takes ownership of an open descriptor: the `File` closes it.

_Defined in `lang/std/files/file.ks`._

#### function `appendBytes`

```kestrel
public static func appendBytes(Path, Slice[UInt8]) -> Result[(), IoError]
```

Adds `bytes` to the end of the file, creating the file if needed.

_Defined in `lang/std/files/contents.ks`._

#### function `appendText`

```kestrel
public static func appendText(Path, String) -> Result[(), IoError]
```

Adds `text` to the end of the file, creating the file if needed.

_Defined in `lang/std/files/contents.ks`._

#### function `close`

```kestrel
public consuming func close() -> Result[(), IoError]
```

Closes the file and reports an error from `close`, which dropping
the file can't.

_Defined in `lang/std/files/file.ks`._

#### function `copy`

```kestrel
public static func copy(Path, to: Path) -> Result[Int64, IoError]
```

Copies the file `source` to `destination`, with its permissions.
Returns the number of bytes copied.

Fails with `InvalidInput` when both name the same file, through the
same path or a symbolic or hard link: opening the destination
truncates it, which would empty the source before it's read.

_Defined in `lang/std/files/operations.ks`._

#### function `create`

```kestrel
public static func create(Path) -> Result[File, IoError]
```

Opens `path` for writing, creating it or emptying it.

_Defined in `lang/std/files/file.ks`._

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/files/file.ks`._

#### function `intoRawDescriptor`

```kestrel
public consuming func intoRawDescriptor() -> FileDescriptor
```

Gives up ownership: the descriptor stays open and the caller closes
it.

_Defined in `lang/std/files/file.ks`._

#### function `metadata`

```kestrel
public func metadata() -> Result[Metadata, IoError]
```

The open file's metadata (`fstat`).

_Defined in `lang/std/files/file.ks`._

#### function `open`

```kestrel
public static func open(Path) -> Result[File, IoError]
```

Opens `path` for reading.

_Defined in `lang/std/files/file.ks`._

#### function `open`

```kestrel
public static func open(Path, OpenOptions) -> Result[File, IoError]
```

Opens `path` as `options` say.

##### Errors

`NotFound` if the file doesn't exist and `options` don't create it,
`AlreadyExists` if `createNew` is set and it does.

_Defined in `lang/std/files/file.ks`._

#### field `rawDescriptor`

```kestrel
public var rawDescriptor: FileDescriptor { get }
```

The descriptor, still owned by this file.

_Defined in `lang/std/files/file.ks`._

#### function `readBytes`

```kestrel
public static func readBytes(Path) -> Result[Array[UInt8], IoError]
```

The whole file's bytes.

_Defined in `lang/std/files/contents.ks`._

#### function `readText`

```kestrel
public static func readText(Path) -> Result[String, IoError]
```

The whole file as UTF-8 text.

##### Errors

`InvalidData` if the file isn't valid UTF-8.

##### Examples

```
let config = try File.readText("config.toml");
```

_Defined in `lang/std/files/contents.ks`._

#### function `remove`

```kestrel
public static func remove(Path) -> Result[(), IoError]
```

Removes the file or symbolic link `path`. Use `Directory.remove` for a
directory.

_Defined in `lang/std/files/operations.ks`._

#### function `rename`

```kestrel
public static func rename(Path, to: Path) -> Result[(), IoError]
```

Renames `source` to `destination`, replacing a file already there.
Both must be on the same file system (`CrossesDevices` otherwise).

_Defined in `lang/std/files/operations.ks`._

#### function `setLength`

```kestrel
public mutating func setLength(Int64) -> Result[(), IoError]
```

Cuts the file to `length` bytes, or extends it with zeros.

_Defined in `lang/std/files/file.ks`._

#### function `setPermissions`

```kestrel
public static func setPermissions(Path, Permissions) -> Result[(), IoError]
```

Sets the permission bits of `path` (`chmod`).

_Defined in `lang/std/files/operations.ks`._

#### function `sync`

```kestrel
public mutating func sync() -> Result[(), IoError]
```

Waits until the contents reach the storage device (`fsync`).

_Defined in `lang/std/files/file.ks`._

#### function `writeBytes`

```kestrel
public static func writeBytes(Path, Slice[UInt8]) -> Result[(), IoError]
```

Replaces the file's contents with `bytes`, creating the file if needed.

_Defined in `lang/std/files/contents.ks`._

#### function `writeText`

```kestrel
public static func writeText(Path, String) -> Result[(), IoError]
```

Replaces the file's contents with `text`, creating the file if needed.

_Defined in `lang/std/files/contents.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

Reads up to `buf.count` bytes from the current position.

_Defined in `lang/std/files/file.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

Nothing to flush: `File` doesn't buffer. Use `sync()` to reach the
disk, or `buffered()` to batch small writes.

_Defined in `lang/std/files/file.ks`._

#### function `write`

```kestrel
public mutating func write(from: Slice[UInt8]) -> Result[Int64, IoError]
```

Writes up to `buf.count` bytes at the current position (or the end,
when opened to append).

_Defined in `lang/std/files/file.ks`._

### Implements `Seekable`

#### function `seek`

```kestrel
public mutating func seek(to: Seek) -> Result[Int64, IoError]
```

Moves the position; returns the new offset from the start. Seeking
past the end is allowed, and a write there extends the file.

_Defined in `lang/std/files/file.ks`._

## enum `FileKind`

```kestrel
public enum FileKind
```

What a path names.

_Defined in `lang/std/files/metadata.ks`._

### Members

#### case `Directory`

```kestrel
case Directory
```

A directory.

_Defined in `lang/std/files/metadata.ks`._

#### case `File`

```kestrel
case File
```

A regular file.

_Defined in `lang/std/files/metadata.ks`._

#### case `Other`

```kestrel
case Other
```

Anything else: a device, socket or pipe.

_Defined in `lang/std/files/metadata.ks`._

#### case `SymbolicLink`

```kestrel
case SymbolicLink
```

A symbolic link (only reported when links aren't followed).

_Defined in `lang/std/files/metadata.ks`._

#### function `code`

```kestrel
func code() -> Int64
```

_Defined in `lang/std/files/metadata.ks`._

#### function `of`

```kestrel
static func of(Int64) -> FileKind
```

The kind for the shims' kind number.

_Defined in `lang/std/files/metadata.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: FileKind) -> Bool
```

Same kind.

_Defined in `lang/std/files/metadata.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

`file`, `directory`, `symbolic link` or `other`.

_Defined in `lang/std/files/metadata.ks`._

## struct `Metadata`

```kestrel
public struct Metadata { /* private fields */ }
```

Facts about a file, directory or link.

_Defined in `lang/std/files/metadata.ks`._

### Members

#### field `accessed`

```kestrel
public var accessed: Int64
```

When it was last read, in nanoseconds since the Unix epoch.

_Defined in `lang/std/files/metadata.ks`._

#### field `device`

```kestrel
public var device: UInt64
```

The device the file system lives on (`st_dev`).

_Defined in `lang/std/files/metadata.ks`._

#### field `inode`

```kestrel
public var inode: UInt64
```

The file's number on its device (`st_ino`). Two paths name the same
file exactly when `device` and `inode` both match.

_Defined in `lang/std/files/metadata.ks`._

#### field `isDirectory`

```kestrel
public var isDirectory: Bool { get }
```

`true` for a directory.

_Defined in `lang/std/files/metadata.ks`._

#### field `isFile`

```kestrel
public var isFile: Bool { get }
```

`true` for a regular file.

_Defined in `lang/std/files/metadata.ks`._

#### function `isSameFile`

```kestrel
public func isSameFile(as: Metadata) -> Bool
```

`true` when both describe the same file: the same inode on the
same device, whatever paths or links led to them.

_Defined in `lang/std/files/metadata.ks`._

#### field `isSymbolicLink`

```kestrel
public var isSymbolicLink: Bool { get }
```

`true` for a symbolic link.

_Defined in `lang/std/files/metadata.ks`._

#### field `kind`

```kestrel
public var kind: FileKind
```

What it is.

_Defined in `lang/std/files/metadata.ks`._

#### field `modified`

```kestrel
public var modified: Int64
```

When its contents last changed, in nanoseconds since the Unix epoch.

_Defined in `lang/std/files/metadata.ks`._

#### function `of`

```kestrel
public static func of(Path) -> Result[Metadata, IoError]
```

The metadata of what's at `path`, following symbolic links (`stat`).

_Defined in `lang/std/files/metadata.ks`._

#### function `ofSymbolicLink`

```kestrel
public static func ofSymbolicLink(Path) -> Result[Metadata, IoError]
```

The metadata of `path` itself, without following a symbolic link
(`lstat`).

_Defined in `lang/std/files/metadata.ks`._

#### field `permissions`

```kestrel
public var permissions: Permissions
```

Its permission bits.

_Defined in `lang/std/files/metadata.ks`._

#### field `size`

```kestrel
public var size: Int64
```

Its size in bytes.

_Defined in `lang/std/files/metadata.ks`._

## struct `OpenOptions`

```kestrel
public struct OpenOptions { /* private fields */ }
```

How `File.open(path, options)` opens a file.

Start from one of the presets and change fields as needed:

```
var options = OpenOptions.writing;
options.createNew = true;          // fail if the file exists
var file = try File.open("out.txt", options);
```

_Defined in `lang/std/files/file.ks`._

### Members

#### initializer `All Off`

```kestrel
public init()
```

Every option off, permissions `0o644`.

_Defined in `lang/std/files/file.ks`._

#### field `append`

```kestrel
public var append: Bool
```

Write at the end, whatever the position.

_Defined in `lang/std/files/file.ks`._

#### field `appending`

```kestrel
public static var appending: OpenOptions { get }
```

Append, creating the file if needed.

_Defined in `lang/std/files/file.ks`._

#### field `create`

```kestrel
public var create: Bool
```

Create the file if it doesn't exist.

_Defined in `lang/std/files/file.ks`._

#### field `createNew`

```kestrel
public var createNew: Bool
```

Create the file, failing with `AlreadyExists` if it exists.

_Defined in `lang/std/files/file.ks`._

#### function `nativeFlags`

```kestrel
func nativeFlags() -> Int32
```

The option bits `kestrel_file_open` takes.

_Defined in `lang/std/files/file.ks`._

#### field `permissions`

```kestrel
public var permissions: Permissions
```

Permissions for a newly created file (before the umask).

_Defined in `lang/std/files/file.ks`._

#### field `read`

```kestrel
public var read: Bool
```

Allow reading.

_Defined in `lang/std/files/file.ks`._

#### field `reading`

```kestrel
public static var reading: OpenOptions { get }
```

Read only.

_Defined in `lang/std/files/file.ks`._

#### field `readingAndWriting`

```kestrel
public static var readingAndWriting: OpenOptions { get }
```

Read and write an existing file, from the start.

_Defined in `lang/std/files/file.ks`._

#### field `truncate`

```kestrel
public var truncate: Bool
```

Empty the file on open.

_Defined in `lang/std/files/file.ks`._

#### field `write`

```kestrel
public var write: Bool
```

Allow writing.

_Defined in `lang/std/files/file.ks`._

#### field `writing`

```kestrel
public static var writing: OpenOptions { get }
```

Write, creating the file or emptying it.

_Defined in `lang/std/files/file.ks`._

## struct `Path`

```kestrel
public struct Path { /* private fields */ }
```

A file system path.

A `Path` is the bytes as written. Paths are byte strings, usually but
not necessarily UTF-8, so non-UTF-8 file names round-trip. Everything
here except the file system shorthands at the end is lexical: it looks
at the text and never touches the disk. A string literal is a `Path`
wherever one is expected, and `/` joins two.

### Examples

```
let config = Environment.homeDirectory() / ".config" / "app.toml";
config.name          // .Some("app.toml")
config.extension     // .Some("toml")
if config.exists() { ... }
```

_Defined in `lang/std/files/path.ks`._

### Members

#### initializer `From Bytes`

```kestrel
public init(bytes: Array[UInt8])
```

The path made of exactly these bytes, which needn't be UTF-8.

_Defined in `lang/std/files/path.ks`._

#### initializer `From Text`

```kestrel
public init(String)
```

The path `text`, as written.

_Defined in `lang/std/files/path.ks`._

#### field `bytes`

```kestrel
public var bytes: &Slice[UInt8] { get }
```

The path's bytes, exactly, borrowed from the path. Call
`toOwned()` for an array of your own.

_Defined in `lang/std/files/path.ks`._

#### function `canonical`

```kestrel
public func canonical() -> Result[Path, IoError]
```

The absolute path with every symbolic link, `.` and `..` resolved
(`realpath(3)`). The path must exist.

_Defined in `lang/std/files/path.ks`._

#### function `componentRanges`

```kestrel
func componentRanges() -> Array[(Int64, Int64)]
```

Byte ranges of the components, skipping empty ones and `.`.

_Defined in `lang/std/files/path.ks`._

#### function `components`

```kestrel
public func components() -> Array[String]
```

The components in order. An absolute path starts with `/`; empty
components and `.` are dropped.

_Defined in `lang/std/files/path.ks`._

#### field `data`

```kestrel
var data: Array[UInt8]
```

_Defined in `lang/std/files/path.ks`._

#### function `exists`

```kestrel
public func exists() -> Bool
```

`true` if something exists at this path (following symbolic links).

_Defined in `lang/std/files/path.ks`._

#### field `extension`

```kestrel
public var extension: Optional[String] { get }
```

What follows the name's last dot: `gz` for `b.tar.gz`. A leading dot
(`.profile`) doesn't start an extension.

_Defined in `lang/std/files/path.ks`._

#### function `extensionDot`

```kestrel
func extensionDot(Int64, Int64) -> Optional[Int64]
```

The index of the dot that starts the extension of the name in
`[start, end)`. A dot at the name's first byte (`.profile`) doesn't count.

_Defined in `lang/std/files/path.ks`._

#### field `isAbsolute`

```kestrel
public var isAbsolute: Bool { get }
```

`true` if the path starts at the root (`/`).

_Defined in `lang/std/files/path.ks`._

#### function `isDirectory`

```kestrel
public func isDirectory() -> Bool
```

`true` if this path is a directory (following symbolic links).

_Defined in `lang/std/files/path.ks`._

#### function `isDot`

```kestrel
func isDot(Int64, Int64) -> Bool
```

_Defined in `lang/std/files/path.ks`._

#### function `isDotDot`

```kestrel
func isDotDot(Int64, Int64) -> Bool
```

_Defined in `lang/std/files/path.ks`._

#### field `isEmpty`

```kestrel
public var isEmpty: Bool { get }
```

`true` for the empty path.

_Defined in `lang/std/files/path.ks`._

#### function `isFile`

```kestrel
public func isFile() -> Bool
```

`true` if this path is a regular file (following symbolic links).

_Defined in `lang/std/files/path.ks`._

#### function `joined`

```kestrel
public func joined(with: Path) -> Path
```

`other` appended as a further component. An absolute `other`
replaces this path, as in Rust and Python.

_Defined in `lang/std/files/path.ks`._

#### function `metadata`

```kestrel
public func metadata() -> Result[Metadata, IoError]
```

The metadata of what's at this path, following symbolic links.

_Defined in `lang/std/files/path.ks`._

#### field `name`

```kestrel
public var name: Optional[String] { get }
```

The last component: `b.tar.gz` for `/a/b.tar.gz`. `None` for `/`,
the empty path, or a path ending in `.` or `..`.

_Defined in `lang/std/files/path.ks`._

#### function `nameRange`

```kestrel
func nameRange() -> Optional[(Int64, Int64)]
```

The byte range of the last component, unless it's `.` or `..`.

_Defined in `lang/std/files/path.ks`._

#### function `normalized`

```kestrel
public func normalized() -> Path
```

The path with `.` dropped and `..` folded into the component before
it, without touching the file system. A symbolic link can make this
differ from `canonical()`.

_Defined in `lang/std/files/path.ks`._

#### field `parent`

```kestrel
public var parent: Optional[Path] { get }
```

The path without its last component: `/a/b` → `/a`, `/a` → `/`.
`None` for `/`, the empty path, or a single relative component.

_Defined in `lang/std/files/path.ks`._

#### function `replacingExtension`

```kestrel
public func replacingExtension(with: String) -> Path
```

The path with its extension replaced by `ext`, or added if it has
none. An empty `ext` removes the extension.

_Defined in `lang/std/files/path.ks`._

#### field `stem`

```kestrel
public var stem: Optional[String] { get }
```

The name without its extension: `b.tar` for `b.tar.gz`.

_Defined in `lang/std/files/path.ks`._

#### field `string`

```kestrel
public var string: String { get }
```

The path as text. Bytes that aren't UTF-8 become U+FFFD, so use
`bytes` when the exact name matters.

_Defined in `lang/std/files/path.ks`._

#### function `toNativePath`

```kestrel
public func toNativePath() -> Result[CString, IoError]
```

The path as a NUL-terminated C string for a system call;
`InvalidInput` if it holds a NUL byte. The caller frees it.

_Defined in `lang/std/files/path.ks`._

#### function `trimmedCount`

```kestrel
func trimmedCount() -> Int64
```

The byte count without trailing slashes, except a lone `/`.

_Defined in `lang/std/files/path.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Path) -> Bool
```

Same bytes. `a/b` and `a//b` are different paths; compare
`normalized()` paths to ignore that.

_Defined in `lang/std/files/path.ks`._

### Implements `Hashable`

#### function `hash`

```kestrel
public func hash[H](into: mutating H) where H: Hasher
```

Hashes the bytes.

_Defined in `lang/std/files/path.ks`._

### Implements `Cloneable`

#### function `clone`

```kestrel
public func clone() -> Path
```

Copies the path.

_Defined in `lang/std/files/path.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

Writes the path as text (non-UTF-8 bytes as U+FFFD).

_Defined in `lang/std/files/path.ks`._

### Implements `ExpressibleByStringLiteral`

### Implements `Divisible`

#### typealias `Output`

```kestrel
type Output = Path
```

_Defined in `lang/std/files/path.ks`._

#### function `divide`

```kestrel
public consuming func divide(consuming Path) -> Path
```

`self / other` is `self.joined(with: other)`.

_Defined in `lang/std/files/path.ks`._

## struct `Permissions`

```kestrel
public struct Permissions { /* private fields */ }
```

Unix permission bits, such as `0o644`.

_Defined in `lang/std/files/metadata.ks`._

### Members

#### initializer `From Mode`

```kestrel
public init(UInt32)
```

The permissions `mode`, such as `Permissions(0o755)`.

_Defined in `lang/std/files/metadata.ks`._

#### field `directoryDefault`

```kestrel
public static var directoryDefault: Permissions { get }
```

`rwxr-xr-x`, the default for new directories.

_Defined in `lang/std/files/metadata.ks`._

#### field `fileDefault`

```kestrel
public static var fileDefault: Permissions { get }
```

`rw-r--r--`, the default for new files.

_Defined in `lang/std/files/metadata.ks`._

#### field `isReadOnly`

```kestrel
public var isReadOnly: Bool { get }
```

`true` if no one may write (no write bit is set).

_Defined in `lang/std/files/metadata.ks`._

#### field `mode`

```kestrel
public var mode: UInt32
```

The permission bits (`0o7777` at most).

_Defined in `lang/std/files/metadata.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Permissions) -> Bool
```

Same bits.

_Defined in `lang/std/files/metadata.ks`._

## enum `SymbolicLink`

```kestrel
public enum SymbolicLink
```

Symbolic links: creating them and reading where they point. Removing one
is `File.remove`; `Metadata.ofSymbolicLink` describes the link itself.

_Defined in `lang/std/files/operations.ks`._

### Members

#### function `create`

```kestrel
public static func create(at: Path, to: Path) -> Result[(), IoError]
```

Creates a symbolic link at `link` that points to `target`. `target`
isn't checked; it may not exist.

_Defined in `lang/std/files/operations.ks`._

#### function `read`

```kestrel
public static func read(Path) -> Result[Path, IoError]
```

Where the symbolic link `path` points, as stored in the link.

_Defined in `lang/std/files/operations.ks`._

