// Semantic versioning for flock packages

module flock.version

import flock.error.(FlockError)

// ============================================================================
// VERSION
// ============================================================================

/// A semantic version with major.minor.patch components.
public struct Version: Cloneable {
    public var major: Int64
    public var minor: Int64
    public var patch: Int64

    public init(major major: Int64, minor minor: Int64, patch patch: Int64) {
        self.major = major;
        self.minor = minor;
        self.patch = patch;
    }

    public func clone() -> Version {
        Version(major: self.major, minor: self.minor, patch: self.patch)
    }

    /// Returns "major.minor.patch" string representation.
    public func toString() -> String {
        var s = String(); s.append(self.major.format()); s.append("."); s.append(self.minor.format()); s.append("."); s.append(self.patch.format()); s
    }

    /// Returns true if this version equals another.
    public func equals(other other: Version) -> Bool {
        self.major == other.major and self.minor == other.minor and self.patch == other.patch
    }

    /// Returns true if this version is less than another.
    public func lessThan(other other: Version) -> Bool {
        if self.major != other.major {
            return self.major < other.major
        }
        if self.minor != other.minor {
            return self.minor < other.minor
        }
        self.patch < other.patch
    }
}

// ============================================================================
// VERSION CONSTRAINT
// ============================================================================

/// A constraint on which versions are acceptable.
public enum VersionConstraint: Cloneable {
    /// Exact match: "=1.2.3" or "1.2.3"
    case Exact(Version)
    /// Compatible: "^1.2.3" (>=1.2.3, <2.0.0)
    case Compatible(Version)
    /// Tilde: "~1.2.3" (>=1.2.3, <1.3.0)
    case TildeCompat(Version)
    /// Any version matches
    case Any

    public func clone() -> VersionConstraint {
        match self {
            .Exact(v) => .Exact(v.clone()),
            .Compatible(v) => .Compatible(v.clone()),
            .TildeCompat(v) => .TildeCompat(v.clone()),
            .Any => .Any
        }
    }
}

/// Returns true if the given version satisfies the constraint.
public func satisfies(version: Version, constraint: VersionConstraint) -> Bool {
    match constraint {
        .Any => true,
        .Exact(required) => version.equals(other: required),
        .Compatible(minimum) => {
            if version.lessThan(other: minimum) {
                return false
            }
            // Must share major version (for major > 0)
            if minimum.major > 0 {
                return version.major == minimum.major
            }
            // For 0.x.y, must share minor version
            if minimum.minor > 0 {
                return version.major == 0 and version.minor == minimum.minor
            }
            // For 0.0.z, must be exact
            version.equals(other: minimum)
        },
        .TildeCompat(minimum) => {
            if version.lessThan(other: minimum) {
                return false
            }
            // Must share major and minor
            version.major == minimum.major and version.minor == minimum.minor
        }
    }
}

// ============================================================================
// PARSING
// ============================================================================

/// Parses a version string like "1.2.3".
public func parseVersion(s s: String) -> Result[Version, FlockError] {
    let parts = splitOnDot(s);
    if parts.count != 3 {
        return .Err(FlockError.InvalidVersion(s))
    }

    let majorStr = parts(unchecked: 0);
    let minorStr = parts(unchecked: 1);
    let patchStr = parts(unchecked: 2);

    match parseInt(majorStr) {
        .Some(major) => {
            match parseInt(minorStr) {
                .Some(minor) => {
                    match parseInt(patchStr) {
                        .Some(patch) => .Ok(Version(major: major, minor: minor, patch: patch)),
                        .None => .Err(FlockError.InvalidVersion(s))
                    }
                },
                .None => .Err(FlockError.InvalidVersion(s))
            }
        },
        .None => .Err(FlockError.InvalidVersion(s))
    }
}

/// Parses a version constraint string like "^1.2.3", "~1.2.3", "1.2.3", or "*".
public func parseConstraint(s s: String) -> Result[VersionConstraint, FlockError] {
    let trimmed = s.trimmed();

    if trimmed.equals("*") {
        return .Ok(VersionConstraint.Any)
    }

    if trimmed.starts(with: "^") {
        let versionStr = trimmed.substringBytes(from: 1, to: trimmed.byteCount);
        match parseVersion(s: versionStr) {
            .Ok(v) => return .Ok(VersionConstraint.Compatible(v)),
            .Err(e) => return .Err(e)
        }
    }

    if trimmed.starts(with: "~") {
        let versionStr = trimmed.substringBytes(from: 1, to: trimmed.byteCount);
        match parseVersion(s: versionStr) {
            .Ok(v) => return .Ok(VersionConstraint.TildeCompat(v)),
            .Err(e) => return .Err(e)
        }
    }

    // Default: exact match
    match parseVersion(s: trimmed) {
        .Ok(v) => .Ok(VersionConstraint.Exact(v)),
        .Err(e) => .Err(e)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Splits a string on '.' characters.
func splitOnDot(s: String) -> Array[String] {
    var result = Array[String]();
    var start: Int64 = 0;
    var i: Int64 = 0;
    let len = s.byteCount;

    while i < len {
        let byte = s.bytes(unchecked: i);
        if byte == 46 { // '.'
            result.append(s.substringBytes(from: start, to: i));
            start = i + 1
        }
        i = i + 1
    }

    // Add the last segment
    if start <= len {
        result.append(s.substringBytes(from: start, to: len))
    }

    result
}

/// Parses a non-negative integer from a string. Returns None on failure.
func parseInt(s: String) -> Optional[Int64] {
    match Int64.parse(s) {
        .Some(n) => {
            if n >= 0 { .Some(n) } else { .None }
        },
        .None => .None
    }
}
