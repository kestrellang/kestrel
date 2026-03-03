// Jessup configuration
//
// Manages the ~/.jessup/ directory layout and config.toml reading/writing.

module jessup.config

// getenv, fileExists, isDirectory, spawn are auto-imported from stdlib
import quill.value.(Value)
import quill.toml.parser.(parseToml)
import jessup.error.(JessupError)

// ============================================================================
// PATHS
// ============================================================================

/// Returns the jessup home directory.
/// Uses JESSUP_HOME env var if set, otherwise defaults to ~/.jessup.
public func jessupHome() -> Result[String, JessupError] {
    match getenv("JESSUP_HOME") {
        .Some(home) => .Ok(home),
        .None => {
            match getenv("HOME") {
                .Some(home) => .Ok(home + "/.jessup"),
                .None => .Err(JessupError.ConfigError("HOME environment variable not set"))
            }
        }
    }
}

/// Returns the path to the bin directory (~/.jessup/bin/).
public func binDir() -> Result[String, JessupError] {
    match jessupHome() {
        .Ok(home) => .Ok(home + "/bin"),
        .Err(e) => .Err(e)
    }
}

/// Returns the path to the toolchains directory (~/.jessup/toolchains/).
public func toolchainsDir() -> Result[String, JessupError] {
    match jessupHome() {
        .Ok(home) => .Ok(home + "/toolchains"),
        .Err(e) => .Err(e)
    }
}

/// Returns the path to config.toml (~/.jessup/config.toml).
public func configPath() -> Result[String, JessupError] {
    match jessupHome() {
        .Ok(home) => .Ok(home + "/config.toml"),
        .Err(e) => .Err(e)
    }
}

// ============================================================================
// CONFIG
// ============================================================================

/// Configuration stored in ~/.jessup/config.toml.
public struct JessupConfig: Cloneable {
    /// The default toolchain channel (e.g., "stable", "nightly").
    public var defaultChannel: String

    public init(defaultChannel defaultChannel: String) {
        self.defaultChannel = defaultChannel;
    }

    public func clone() -> JessupConfig {
        JessupConfig(defaultChannel: self.defaultChannel.clone())
    }
}

/// Reads the jessup config from disk. Returns a default config if the file
/// doesn't exist or can't be parsed.
public func readConfig() -> JessupConfig {
    match configPath() {
        .Err(_) => JessupConfig(defaultChannel: "stable"),
        .Ok(path) => {
            if not fileExists(path) {
                return JessupConfig(defaultChannel: "stable")
            }
            match readFileString(path) {
                .Err(_) => JessupConfig(defaultChannel: "stable"),
                .Ok(source) => {
                    match parseToml(source) {
                        .Err(_) => JessupConfig(defaultChannel: "stable"),
                        .Ok(root) => parseConfig(root: root)
                    }
                }
            }
        }
    }
}

/// Writes the config to disk.
public func writeConfig(config config: JessupConfig) -> Result[(), JessupError] {
    match configPath() {
        .Err(e) => .Err(e),
        .Ok(path) => {
            // Ensure jessup home exists
            match jessupHome() {
                .Err(e) => return .Err(e),
                .Ok(home) => {
                    let _ = spawn("mkdir -p " + home);
                }
            }

            let content = "[config]\ndefault_channel = \"" + config.defaultChannel + "\"\n";
            match writeFileString(path, content) {
                .Ok(_) => .Ok(()),
                .Err(_) => .Err(JessupError.IoError("failed to write config: " + path))
            }
        }
    }
}

// ============================================================================
// DIRECTORY SETUP
// ============================================================================

/// Ensures all jessup directories exist.
public func ensureDirectories() -> Result[(), JessupError] {
    match jessupHome() {
        .Err(e) => .Err(e),
        .Ok(home) => {
            let _ = spawn("mkdir -p " + home + "/bin");
            let _ = spawn("mkdir -p " + home + "/toolchains");
            .Ok(())
        }
    }
}

// ============================================================================
// PARSING
// ============================================================================

func parseConfig(root root: Value) -> JessupConfig {
    var channel = "stable";
    match root.value(forKey: "config") {
        .Some(configVal) => {
            match configVal.value(forKey: "default_channel") {
                .Some(channelVal) => {
                    match channelVal.asString() {
                        .Some(s) => channel = s,
                        .None => {}
                    }
                },
                .None => {}
            }
        },
        .None => {}
    }
    JessupConfig(defaultChannel: channel)
}
