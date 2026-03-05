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
            .CacheError(s) => .CacheError(s.clone())
        }
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self {
            .ManifestNotFound(path) => {
                var s = String(); s.append("flock.toml not found: "); s.append(path); s
            },
            .ManifestParse(msg) => {
                var s = String(); s.append("failed to parse manifest: "); s.append(msg); s
            },
            .InvalidVersion(msg) => {
                var s = String(); s.append("invalid version: "); s.append(msg); s
            },
            .DependencyCycle(names) => {
                var s = String(); s.append("dependency cycle detected: "); s.append(joinNames(names)); s
            },
            .DependencyNotFound(name) => {
                var s = String(); s.append("dependency not found: "); s.append(name); s
            },
            .CompilerFailed(code) => {
                var s = String(); s.append("compiler exited with code "); s.append(Int64(from: code).format()); s
            },
            .IoError(msg) => {
                var s = String(); s.append("I/O error: "); s.append(msg); s
            },
            .RegistryError(msg) => {
                var s = String(); s.append("registry error: "); s.append(msg); s
            },
            .ChecksumMismatch(msg) => {
                var s = String(); s.append("checksum mismatch: "); s.append(msg); s
            },
            .CacheError(msg) => {
                var s = String(); s.append("cache error: "); s.append(msg); s
            }
        }
    }
}

func joinNames(names: Array[String]) -> String {
    var result = String();
    var i: Int64 = 0;
    while i < names.count {
        if i > 0 {
            result.append(" -> ")
        }
        result.append(names(unchecked: i));
        i = i + 1
    }
    result
}
