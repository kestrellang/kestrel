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
                .Some(home) => {
                    var s = String();
                    s.append(home);
                    s.append("/.jessup");
                    .Ok(s)
                },
                .None => .Err(JessupError.ConfigError("HOME environment variable not set"))
            }
        }
    }
}

/// Returns the path to the bin directory (~/.jessup/bin/).
public func binDir() -> Result[String, JessupError] {
    match jessupHome() {
        .Ok(home) => {
            var s = String();
            s.append(home);
            s.append("/bin");
            .Ok(s)
        },
        .Err(e) => .Err(e)
    }
}

/// Returns the path to the toolchains directory (~/.jessup/toolchains/).
public func toolchainsDir() -> Result[String, JessupError] {
    match jessupHome() {
        .Ok(home) => {
            var s = String();
            s.append(home);
            s.append("/toolchains");
            .Ok(s)
        },
        .Err(e) => .Err(e)
    }
}

/// Returns the path to config.toml (~/.jessup/config.toml).
public func configPath() -> Result[String, JessupError] {
    match jessupHome() {
        .Ok(home) => {
            var s = String();
            s.append(home);
            s.append("/config.toml");
            .Ok(s)
        },
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
                    var mkdirCmd = String();
                    mkdirCmd.append("mkdir -p ");
                    mkdirCmd.append(home);
                     spawn(mkdirCmd);
                }
            }

            var content = String();
            content.append("[config]\ndefault_channel = \"");
            content.append(config.defaultChannel);
            content.append("\"\n");
            match writeFileString(path, content) {
                .Ok(_) => .Ok(()),
                .Err(_) => {
                    var ioMsg = String();
                    ioMsg.append("failed to write config: ");
                    ioMsg.append(path);
                    .Err(JessupError.IoError(ioMsg))
                }
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
            var mkBinCmd = String();
            mkBinCmd.append("mkdir -p ");
            mkBinCmd.append(home);
            mkBinCmd.append("/bin");
             spawn(mkBinCmd);
            var mkTcCmd = String();
            mkTcCmd.append("mkdir -p ");
            mkTcCmd.append(home);
            mkTcCmd.append("/toolchains");
             spawn(mkTcCmd);
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
