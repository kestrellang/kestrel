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

    public func clone() -> FlockError {
        match self {
            .ManifestNotFound(s) => .ManifestNotFound(s.clone()),
            .ManifestParse(s) => .ManifestParse(s.clone()),
            .InvalidVersion(s) => .InvalidVersion(s.clone()),
            .DependencyCycle(a) => .DependencyCycle(a.clone()),
            .DependencyNotFound(s) => .DependencyNotFound(s.clone()),
            .CompilerFailed(c) => .CompilerFailed(c),
            .IoError(s) => .IoError(s.clone())
        }
    }

    /// Returns a human-readable description of the error.
    public func description() -> String {
        match self {
            .ManifestNotFound(path) => "flock.toml not found: " + path,
            .ManifestParse(msg) => "failed to parse manifest: " + msg,
            .InvalidVersion(msg) => "invalid version: " + msg,
            .DependencyCycle(names) => "dependency cycle detected: " + joinNames(names),
            .DependencyNotFound(name) => "dependency not found: " + name,
            .CompilerFailed(code) => "compiler exited with code " + Int64(from: code).format(),
            .IoError(msg) => "I/O error: " + msg
        }
    }
}

func joinNames(names: Array[String]) -> String {
    var result = String();
    var i: Int64 = 0;
    while i < names.count {
        if i > 0 {
            result = result + " -> "
        }
        result = result + names(unchecked: i);
        i = i + 1
    }
    result
}
