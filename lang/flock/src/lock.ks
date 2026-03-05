// Lock file support for reproducible builds
//
// flock.lock records every resolved dependency with its exact version,
// source type, and checksum. This ensures reproducible builds.
//
// Format (TOML):
//   [[package]]
//   name = "kestrel/swoop"
//   version = "1.1.0"
//   checksum = "sha256:abcdef..."
//   source = "registry"
//
//   [[package]]
//   name = "quill"
//   version = "0.1.0"
//   source = "path"
//   path = "../quill"

module flock.lock

import quill.value.(Value)
import quill.toml.parser.(parseToml)
import flock.error.(FlockError)
import flock.version.(Version, parseVersion)
import flock.source.(joinPath)

// ============================================================================
// LOCK ENTRY
// ============================================================================

/// A single resolved dependency in the lock file.
public struct LockEntry: Cloneable {
    public var name: String
    public var version: Version
    /// "registry" or "path"
    public var source: String
    /// Checksum for registry packages (e.g., "sha256:abcdef...")
    public var checksum: Optional[String]
    /// Relative path for path dependencies
    public var path: Optional[String]

    public init(name name: String, version version: Version, source source: String, checksum checksum: Optional[String], path path: Optional[String]) {
        self.name = name;
        self.version = version;
        self.source = source;
        self.checksum = checksum;
        self.path = path;
    }

    public func clone() -> LockEntry {
        LockEntry(
            name: self.name.clone(),
            version: self.version.clone(),
            source: self.source.clone(),
            checksum: cloneOptStr(self.checksum),
            path: cloneOptStr(self.path)
        )
    }
}

// ============================================================================
// LOCK FILE
// ============================================================================

/// A parsed flock.lock file.
public struct LockFile: Cloneable {
    public var packages: Array[LockEntry]

    public init() {
        self.packages = Array[LockEntry]();
    }

    public init(packages packages: Array[LockEntry]) {
        self.packages = packages;
    }

    public func clone() -> LockFile {
        LockFile(packages: self.packages.clone())
    }

    /// Finds a locked entry by package name.
    public func find(name name: String) -> Optional[LockEntry] {
        var i: Int64 = 0;
        while i < self.packages.count {
            let entry = self.packages(unchecked: i);
            if entry.name.equals(name) {
                return .Some(entry)
            }
            i = i + 1
        }
        .None
    }
}

// ============================================================================
// PARSING
// ============================================================================

/// Parses a flock.lock file from its TOML source.
public func parseLockFile(source source: String) -> Result[LockFile, FlockError] {
    match parseToml(source) {
        .Err(e) => {
            var msg = String(); msg.append("invalid lock file: "); msg.append(e.description());
            .Err(FlockError.ManifestParse(msg))
        },
        .Ok(root) => {
            match root.value(forKey: "package") {
                .None => .Ok(LockFile()),
                .Some(pkgVal) => {
                    match pkgVal.asArray() {
                        .None => .Err(FlockError.ManifestParse("lock file: 'package' is not an array")),
                        .Some(arr) => {
                            var entries = Array[LockEntry]();
                            var i: Int64 = 0;
                            while i < arr.count {
                                match parseLockEntry(val: arr(unchecked: i)) {
                                    .Err(e) => return .Err(e),
                                    .Ok(entry) => entries.append(entry)
                                }
                                i = i + 1
                            }
                            .Ok(LockFile(packages: entries))
                        }
                    }
                }
            }
        }
    }
}

/// Parses a single [[package]] entry from the lock file.
func parseLockEntry(val val: Value) -> Result[LockEntry, FlockError] {
    // Required: name
    var name = "";
    match val.value(forKey: "name") {
        .Some(v) => {
            match v.asString() {
                .Some(s) => name = s,
                .None => return .Err(FlockError.ManifestParse("lock entry: name is not a string"))
            }
        },
        .None => return .Err(FlockError.ManifestParse("lock entry: missing name"))
    }

    // Required: version
    var version = Version(major: 0, minor: 0, patch: 0);
    match val.value(forKey: "version") {
        .Some(v) => {
            match v.asString() {
                .Some(s) => {
                    match parseVersion(s: s) {
                        .Ok(ver) => version = ver,
                        .Err(e) => return .Err(e)
                    }
                },
                .None => return .Err(FlockError.ManifestParse("lock entry: version is not a string"))
            }
        },
        .None => return .Err(FlockError.ManifestParse("lock entry: missing version"))
    }

    // Required: source
    var source = "path";
    match val.value(forKey: "source") {
        .Some(v) => {
            match v.asString() {
                .Some(s) => source = s,
                .None => {}
            }
        },
        .None => {}
    }

    // Optional: checksum
    var checksum: Optional[String] = .None;
    match val.value(forKey: "checksum") {
        .Some(v) => {
            match v.asString() {
                .Some(s) => checksum = .Some(s),
                .None => {}
            }
        },
        .None => {}
    }

    // Optional: path
    var entryPath: Optional[String] = .None;
    match val.value(forKey: "path") {
        .Some(v) => {
            match v.asString() {
                .Some(s) => entryPath = .Some(s),
                .None => {}
            }
        },
        .None => {}
    }

    .Ok(LockEntry(name: name, version: version, source: source, checksum: checksum, path: entryPath))
}

// ============================================================================
// GENERATION
// ============================================================================

/// Generates flock.lock TOML content from a list of lock entries.
public func generateLockFile(entries entries: Array[LockEntry]) -> String {
    var buf = "# This file is auto-generated by flock. Do not edit.\n";

    for entry in entries {
        buf.append("\n[[package]]\n");
        buf.append("name = \""); buf.append(entry.name); buf.append("\"\n");
        buf.append("version = \""); buf.append(entry.version.toString()); buf.append("\"\n");
        buf.append("source = \""); buf.append(entry.source); buf.append("\"\n");

        match entry.checksum {
            .Some(cs) => {
                buf.append("checksum = \""); buf.append(cs); buf.append("\"\n")
            },
            .None => {}
        }

        match entry.path {
            .Some(p) => {
                buf.append("path = \""); buf.append(p); buf.append("\"\n")
            },
            .None => {}
        }
    }

    buf
}

// ============================================================================
// HELPERS
// ============================================================================

func cloneOptStr(opt: Optional[String]) -> Optional[String] {
    match opt {
        .Some(s) => .Some(s.clone()),
        .None => .None
    }
}
