// Platform detection for jessup
//
// Detects the current OS and architecture using uname, then maps
// to the release asset naming convention used by GitHub releases.

module jessup.platform

// captureOutput is auto-imported from stdlib
import jessup.error.(JessupError)

// ============================================================================
// PLATFORM INFO
// ============================================================================

/// Represents the current platform (OS + architecture).
public struct Platform: Cloneable {
    public var os: String
    public var arch: String

    public init(os os: String, arch arch: String) {
        self.os = os;
        self.arch = arch;
    }

    public func clone() -> Platform {
        Platform(os: self.os.clone(), arch: self.arch.clone())
    }

    /// Returns the asset suffix for this platform.
    /// e.g., "aarch64-apple-darwin" or "x86_64-unknown-linux"
    public func assetTarget() -> String {
        self.arch + "-" + self.os
    }
}

// ============================================================================
// DETECTION
// ============================================================================

/// Detects the current platform from uname.
public func detectPlatform() -> Result[Platform, JessupError] {
    let rawOs = captureOutput("uname -s");
    let rawArch = captureOutput("uname -m");

    let os = trimWhitespace(rawOs);
    let arch = trimWhitespace(rawArch);

    // Map OS
    var mappedOs = "";
    if os.equals("Darwin") {
        mappedOs = "apple-darwin"
    } else if os.equals("Linux") {
        mappedOs = "unknown-linux"
    } else {
        return .Err(JessupError.InstallError("unsupported operating system: " + os))
    };

    // Map architecture
    var mappedArch = "";
    if arch.equals("arm64") or arch.equals("aarch64") {
        mappedArch = "aarch64"
    } else if arch.equals("x86_64") {
        mappedArch = "x86_64"
    } else {
        return .Err(JessupError.InstallError("unsupported architecture: " + arch))
    };

    return .Ok(Platform(os: mappedOs, arch: mappedArch))
}

// ============================================================================
// HELPERS
// ============================================================================

/// Trims trailing whitespace (newlines, spaces) from a string.
func trimWhitespace(s: String) -> String {
    let len = s.byteCount;
    var end = len;
    while end > 0 {
        let b = s.byteAtUnchecked(end - 1);
        // space=32, tab=9, newline=10, carriage return=13
        if b == UInt8(intLiteral: 32) or b == UInt8(intLiteral: 9) or b == UInt8(intLiteral: 10) or b == UInt8(intLiteral: 13) {
            end = end - 1
        } else {
            return s.substringBytes(from: 0, to: end)
        }
    }
    ""
}
