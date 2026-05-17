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
        var s = String();
        s.append(self.arch);
        s.append("-");
        s.append(self.os);
        s
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
    if os == "Darwin" {
        mappedOs = "apple-darwin"
    } else if os == "Linux" {
        mappedOs = "unknown-linux"
    } else {
        var errMsg = String();
        errMsg.append("unsupported operating system: ");
        errMsg.append(os);
        return .Err(JessupError.InstallError(errMsg))
    };

    // Map architecture
    var mappedArch = "";
    if arch == "arm64" or arch == "aarch64" {
        mappedArch = "aarch64"
    } else if arch == "x86_64" {
        mappedArch = "x86_64"
    } else {
        var errMsg = String();
        errMsg.append("unsupported architecture: ");
        errMsg.append(arch);
        return .Err(JessupError.InstallError(errMsg))
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
        let b = s.bytes(unchecked: end - 1);
        // space=32, tab=9, newline=10, carriage return=13
        if b == 32 or b == 9 or b == 10 or b == 13 {
            end = end - 1
        } else {
            return s.asSlice().subslice(from: 0, to: end).toOwned()
        }
    }
    ""
}
