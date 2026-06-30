// Flock error types

module flock.error

/// All error conditions that can occur in flock operations.
public enum FlockError: Cloneable {
    case ManifestNotFound(String)
    case ManifestParse(String)
    case InvalidVersion(String)
    case DependencyCycle(Array[String])
    case DependencyNotFound(String)
    case CompilerFailed(Int32)
    case IoError(String)
    case RegistryError(String)
    case ChecksumMismatch(String)
    case CacheError(String)
    /// The package defines no binary targets (no src/main.ks, src/bin/*.ks, or [[bin]]).
    case NoBinaryTargets(String)
    /// A dependency exposes no library (it only defines binaries) — like Cargo's
    /// "no library targets found". Payload is the dependency name.
    case NoLibraryTarget(String)
    /// An installed binary of this name already exists (pass --force to overwrite).
    case BinaryExists(String)
    /// --bin named a target that doesn't exist; payload is the requested name.
    case BinNotFound(String)
    /// Multiple binary targets and no default (src/main.ks) to pick; payload is a
    /// comma-separated list of candidate names.
    case AmbiguousBinary(String)
    /// Two binary targets resolved to the same name; payload is that name.
    case DuplicateBinary(String)

    public func clone() -> FlockError {
        match self {
            .ManifestNotFound(s) => .ManifestNotFound(s.clone()),
            .ManifestParse(s) => .ManifestParse(s.clone()),
            .InvalidVersion(s) => .InvalidVersion(s.clone()),
            .DependencyCycle(a) => .DependencyCycle(a.clone()),
            .DependencyNotFound(s) => .DependencyNotFound(s.clone()),
            .CompilerFailed(c) => .CompilerFailed(c),
            .IoError(s) => .IoError(s.clone()),
            .RegistryError(s) => .RegistryError(s.clone()),
            .ChecksumMismatch(s) => .ChecksumMismatch(s.clone()),
            .CacheError(s) => .CacheError(s.clone()),
            .NoBinaryTargets(s) => .NoBinaryTargets(s.clone()),
            .NoLibraryTarget(s) => .NoLibraryTarget(s.clone()),
            .BinaryExists(s) => .BinaryExists(s.clone()),
            .BinNotFound(s) => .BinNotFound(s.clone()),
            .AmbiguousBinary(s) => .AmbiguousBinary(s.clone()),
            .DuplicateBinary(s) => .DuplicateBinary(s.clone())
        }
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self {
            .ManifestNotFound(path) => "flock.toml not found: \(path)",
            .ManifestParse(msg) => "failed to parse manifest: \(msg)",
            .InvalidVersion(msg) => "invalid version: \(msg)",
            .DependencyCycle(names) => "dependency cycle detected: \(names.joined(" -> "))",
            .DependencyNotFound(name) => "dependency not found: \(name)",
            .CompilerFailed(code) => "compiler exited with code \(Int64(from: code))",
            .IoError(msg) => "I/O error: \(msg)",
            .RegistryError(msg) => "registry error: \(msg)",
            .ChecksumMismatch(msg) => "checksum mismatch: \(msg)",
            .CacheError(msg) => "cache error: \(msg)",
            .NoBinaryTargets(pkg) => "package '\(pkg)' has no binary targets (add src/main.ks, src/bin/*.ks, or a [[bin]] entry)",
            .NoLibraryTarget(name) => "dependency '\(name)' has no library target — it only defines binaries, so it cannot be depended on",
            .BinaryExists(name) => "'\(name)' is already installed in ~/.flock/bin (pass --force to overwrite)",
            .BinNotFound(name) => "no binary target named '\(name)' in this package",
            .AmbiguousBinary(names) => "multiple binary targets (\(names)); pass --bin <name> to choose one",
            .DuplicateBinary(name) => "duplicate binary target name '\(name)' (each binary must have a unique name)"
        }
    }
}
