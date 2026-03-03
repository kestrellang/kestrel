// Toolchain management
//
// Install, remove, list, and link kestrel toolchains.
// Each toolchain lives in ~/.jessup/toolchains/<name>/ with bin/ and lib/std/.

module jessup.toolchain

// fileExists, isDirectory, listDir, getenv, spawn, captureOutput
// are auto-imported from stdlib
import jessup.error.(JessupError)
import jessup.config.(jessupHome, binDir, toolchainsDir, ensureDirectories, readConfig, writeConfig, JessupConfig)
import jessup.platform.(Platform, detectPlatform)
import jessup.github.(Release, fetchRelease, fetchJessupRelease)

// ============================================================================
// INSTALL
// ============================================================================

/// Installs a toolchain for the given channel.
/// channel can be "stable", "nightly", or a specific version like "1.0.0".
public func installToolchain(channel channel: String) -> Result[String, JessupError] {
    // Ensure directories exist
    match ensureDirectories() {
        .Err(e) => return .Err(e),
        .Ok(_) => {}
    }

    // Detect platform
    var platform = Platform(os: "", arch: "");
    match detectPlatform() {
        .Err(e) => return .Err(e),
        .Ok(p) => platform = p
    }

    let _ = println("Fetching release info for " + channel + "...");

    // Fetch release info from GitHub
    var release = Release(tagName: "", assetUrl: "");
    match fetchRelease(channel: channel, platform: platform) {
        .Err(e) => return .Err(e),
        .Ok(r) => release = r
    }

    // Determine toolchain name
    let toolchainName = toolchainDirName(channel: channel, tag: release.tagName);

    // Check if already installed
    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => tcDir = dir + "/" + toolchainName
    }

    if isDirectory(tcDir) {
        let _ = println("Toolchain " + toolchainName + " is already installed");
        return .Ok(toolchainName)
    }

    let _ = println("Downloading " + toolchainName + "...");

    // Create temp directory for download
    let tmpDir = "/tmp/jessup-download-" + toolchainName;
    let _ = spawn("mkdir -p " + tmpDir);
    let archivePath = tmpDir + "/toolchain.tar.gz";

    // Download using curl (handles GitHub redirects)
    let curlCmd = "curl -sL -o " + archivePath + " " + release.assetUrl;
    let exitCode = spawn(curlCmd);
    if exitCode != 0 {
        let _ = spawn("rm -rf " + tmpDir);
        return .Err(JessupError.NetworkError("failed to download toolchain"))
    }

    // Create toolchain directory
    let _ = spawn("mkdir -p " + tcDir);

    // Extract archive (strip the top-level directory from the tarball)
    let tarCmd = "tar xzf " + archivePath + " -C " + tcDir + " --strip-components=1";
    let tarExit = spawn(tarCmd);
    if tarExit != 0 {
        let _ = spawn("rm -rf " + tmpDir);
        let _ = spawn("rm -rf " + tcDir);
        return .Err(JessupError.InstallError("failed to extract toolchain archive"))
    }

    // Clean up temp files
    let _ = spawn("rm -rf " + tmpDir);

    // Make binaries executable
    let _ = spawn("chmod +x " + tcDir + "/bin/kestrel");
    let _ = spawn("chmod +x " + tcDir + "/bin/flock");

    let _ = println("Installed " + toolchainName);

    .Ok(toolchainName)
}

// ============================================================================
// DEFAULT (SYMLINK MANAGEMENT)
// ============================================================================

/// Sets the default toolchain by updating symlinks in ~/.jessup/bin/.
public func setDefault(toolchainName toolchainName: String) -> Result[(), JessupError] {
    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => tcDir = dir + "/" + toolchainName
    }

    if not isDirectory(tcDir) {
        return .Err(JessupError.NotFound("toolchain not installed: " + toolchainName))
    }

    var binPath = "";
    match binDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => binPath = dir
    }

    let _ = spawn("mkdir -p " + binPath);

    // Remove existing symlinks and create new ones
    let _ = spawn("rm -f " + binPath + "/kestrel");
    let _ = spawn("rm -f " + binPath + "/flock");

    let _ = spawn("ln -s " + tcDir + "/bin/kestrel " + binPath + "/kestrel");
    let _ = spawn("ln -s " + tcDir + "/bin/flock " + binPath + "/flock");

    // Update config with the channel name
    var config = readConfig();
    config.defaultChannel = toolchainName;
    match writeConfig(config: config) {
        .Err(e) => return .Err(e),
        .Ok(_) => {}
    }

    let _ = println("Default toolchain set to " + toolchainName);

    .Ok(())
}

// ============================================================================
// LIST
// ============================================================================

/// Lists all installed toolchains. Marks the active one.
public func listToolchains() -> Result[(), JessupError] {
    var tcDirPath = "";
    match toolchainsDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => tcDirPath = dir
    }

    if not isDirectory(tcDirPath) {
        let _ = println("No toolchains installed");
        return .Ok(())
    }

    let entries = listDir(tcDirPath);
    if entries.count == 0 {
        let _ = println("No toolchains installed");
        return .Ok(())
    }

    // Determine the active toolchain from config
    let config = readConfig();
    let activeChannel = config.defaultChannel;

    let _ = println("Installed toolchains:");
    let _ = println("");

    var i: Int64 = 0;
    while i < entries.count {
        let name = entries(unchecked: i);
        // Skip hidden files
        if name.byteCount > 0 and name.byteAtUnchecked(0) != UInt8(intLiteral: 46) {
            if name.equals(activeChannel) {
                let _ = println("  " + name + " (active)");
            } else {
                let _ = println("  " + name);
            }
        }
        i = i + 1
    }

    .Ok(())
}

// ============================================================================
// REMOVE
// ============================================================================

/// Removes an installed toolchain.
public func removeToolchain(toolchainName toolchainName: String) -> Result[(), JessupError] {
    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => tcDir = dir + "/" + toolchainName
    }

    if not isDirectory(tcDir) {
        return .Err(JessupError.NotFound("toolchain not installed: " + toolchainName))
    }

    // Check if this is the active toolchain
    let config = readConfig();
    if config.defaultChannel.equals(toolchainName) {
        let _ = println("Warning: removing the active toolchain. Run 'jessup default <version>' to set a new default.");
        // Remove symlinks
        match binDir() {
            .Ok(bp) => {
                let _ = spawn("rm -f " + bp + "/kestrel");
                let _ = spawn("rm -f " + bp + "/flock");
            },
            .Err(_) => {}
        }
    }

    let _ = spawn("rm -rf " + tcDir);
    let _ = println("Removed toolchain " + toolchainName);

    .Ok(())
}

// ============================================================================
// SHOW
// ============================================================================

/// Shows the active toolchain and its path.
public func showActive() -> Result[(), JessupError] {
    let config = readConfig();
    let activeChannel = config.defaultChannel;

    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => tcDir = dir + "/" + activeChannel
    }

    if isDirectory(tcDir) {
        let _ = println("Active toolchain: " + activeChannel);
        let _ = println("Location: " + tcDir);

        // Show kestrel version if available
        let kestrelBin = tcDir + "/bin/kestrel";
        if fileExists(kestrelBin) {
            let version = captureOutput(kestrelBin + " --version");
            let _ = println("Version: " + version);
        }
    } else {
        let _ = println("No active toolchain. Run 'jessup install stable' to get started.");
    };

    .Ok(())
}

// ============================================================================
// UPDATE
// ============================================================================

/// Updates all installed channel toolchains (stable, nightly) to their latest versions.
public func updateToolchains() -> Result[(), JessupError] {
    var tcDirPath = "";
    match toolchainsDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => tcDirPath = dir
    }

    if not isDirectory(tcDirPath) {
        let _ = println("No toolchains installed");
        return .Ok(())
    }

    let entries = listDir(tcDirPath);
    var updated = false;

    var i: Int64 = 0;
    while i < entries.count {
        let name = entries(unchecked: i);
        // Update channels (stable-*, nightly-*)
        if name.starts(with: "stable") or name.starts(with: "nightly") {
            let channel = if name.starts(with: "stable") { "stable" } else { "nightly" };
            let _ = println("Updating " + channel + "...");

            // Remove old version
            let _ = spawn("rm -rf " + tcDirPath + "/" + name);

            // Install latest
            match installToolchain(channel: channel) {
                .Ok(newName) => {
                    updated = true;
                    // Re-link if this was the active toolchain
                    let config = readConfig();
                    if config.defaultChannel.equals(name) {
                        match setDefault(toolchainName: newName) {
                            .Ok(_) => {},
                            .Err(e) => {
                                let _ = eprintln(e.description());
                            }
                        }
                    }
                },
                .Err(e) => {
                    let _ = eprintln("Failed to update " + channel + ": " + e.description());
                }
            }
        }
        i = i + 1
    }

    if not updated {
        let _ = println("No updatable channels found (install stable or nightly first)");
    }

    .Ok(())
}

// ============================================================================
// SELF UPDATE
// ============================================================================

/// Updates jessup itself to the latest version.
public func selfUpdate() -> Result[(), JessupError] {
    var platform = Platform(os: "", arch: "");
    match detectPlatform() {
        .Err(e) => return .Err(e),
        .Ok(p) => platform = p
    }

    let _ = println("Checking for jessup updates...");

    var downloadUrl = "";
    match fetchJessupRelease(platform: platform) {
        .Err(e) => return .Err(e),
        .Ok(url) => downloadUrl = url
    }

    var bp = "";
    match binDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => bp = dir
    }

    let tmpDir = "/tmp/jessup-self-update";
    let _ = spawn("mkdir -p " + tmpDir);
    let archivePath = tmpDir + "/jessup.tar.gz";

    let curlCmd = "curl -sL -o " + archivePath + " " + downloadUrl;
    let exitCode = spawn(curlCmd);
    if exitCode != 0 {
        let _ = spawn("rm -rf " + tmpDir);
        return .Err(JessupError.NetworkError("failed to download jessup update"))
    }

    // Extract and strip top-level directory
    let _ = spawn("tar xzf " + archivePath + " -C " + tmpDir + " --strip-components=1");
    let _ = spawn("chmod +x " + tmpDir + "/jessup");
    let _ = spawn("mv " + tmpDir + "/jessup " + bp + "/jessup");
    let _ = spawn("rm -rf " + tmpDir);

    let _ = println("jessup has been updated");

    .Ok(())
}

// ============================================================================
// HELPERS
// ============================================================================

/// Generates a toolchain directory name from channel and release tag.
/// e.g., "stable" + "v1.0.0" -> "stable-1.0.0"
/// e.g., "nightly" + "nightly" -> "nightly-2026-03-02"
func toolchainDirName(channel channel: String, tag tag: String) -> String {
    if channel.equals("nightly") {
        // Use current date for nightly
        let date = captureOutput("date +%Y-%m-%d");
        let trimmed = trimTrailingNewline(date);
        "nightly-" + trimmed
    } else if channel.equals("stable") {
        // Strip leading 'v' from tag if present
        if tag.byteCount > 0 and tag.byteAtUnchecked(0) == UInt8(intLiteral: 118) {
            "stable-" + tag.substringBytes(from: 1, to: tag.byteCount)
        } else {
            "stable-" + tag
        }
    } else {
        // Specific version — use as-is
        channel
    }
}

func trimTrailingNewline(s: String) -> String {
    let len = s.byteCount;
    var end = len;
    while end > 0 {
        let b = s.byteAtUnchecked(end - 1);
        if b == UInt8(intLiteral: 10) or b == UInt8(intLiteral: 13) {
            end = end - 1
        } else {
            return s.substringBytes(from: 0, to: end)
        }
    }
    ""
}
