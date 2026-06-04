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
import jessup.github.(Release, fetchRelease, fetchJessupRelease, fetchVsixRelease)

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

    var fetchMsg = String();
    fetchMsg.append("Fetching release info for ");
    fetchMsg.append(channel);
    fetchMsg.append("...");
    let _ = println(fetchMsg);

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
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(toolchainName);
            tcDir = s
        }
    }

    if isDirectory(tcDir) {
        var alreadyMsg = String();
        alreadyMsg.append("Toolchain ");
        alreadyMsg.append(toolchainName);
        alreadyMsg.append(" is already installed");
        let _ = println(alreadyMsg);
        return .Ok(toolchainName)
    }

    var dlMsg = String();
    dlMsg.append("Downloading ");
    dlMsg.append(toolchainName);
    dlMsg.append("...");
    let _ = println(dlMsg);

    // Create temp directory for download
    var tmpDir = String();
    tmpDir.append("/tmp/jessup-download-");
    tmpDir.append(toolchainName);
    var mkdirCmd = String();
    mkdirCmd.append("mkdir -p ");
    mkdirCmd.append(tmpDir);
    let _ = spawn(mkdirCmd);
    var archivePath = String();
    archivePath.append(tmpDir);
    archivePath.append("/toolchain.tar.gz");

    // Download using curl (handles GitHub redirects)
    var curlCmd = String();
    curlCmd.append("curl -sL ");
    match getenv("GITHUB_TOKEN") {
        .Some(token) => {
            if token.byteCount > 0 {
                curlCmd.append("-H 'Authorization: Bearer ");
                curlCmd.append(token);
                curlCmd.append("' -H 'Accept: application/octet-stream' ");
            }
        },
        .None => {}
    }
    curlCmd.append("-o ");
    curlCmd.append(archivePath);
    curlCmd.append(" ");
    curlCmd.append(release.assetUrl);
    let exitCode = spawn(curlCmd);
    if exitCode != 0 {
        var rmCmd = String();
        rmCmd.append("rm -rf ");
        rmCmd.append(tmpDir);
        let _ = spawn(rmCmd);
        return .Err(JessupError.NetworkError("failed to download toolchain"))
    }

    // Create toolchain directory
    var mkdirTcCmd = String();
    mkdirTcCmd.append("mkdir -p ");
    mkdirTcCmd.append(tcDir);
    let _ = spawn(mkdirTcCmd);

    // Extract archive (strip the top-level directory from the tarball)
    var tarCmd = String();
    tarCmd.append("tar xzf ");
    tarCmd.append(archivePath);
    tarCmd.append(" -C ");
    tarCmd.append(tcDir);
    tarCmd.append(" --strip-components=1");
    let tarExit = spawn(tarCmd);
    if tarExit != 0 {
        var rmTmpCmd = String();
        rmTmpCmd.append("rm -rf ");
        rmTmpCmd.append(tmpDir);
        let _ = spawn(rmTmpCmd);
        var rmTcCmd = String();
        rmTcCmd.append("rm -rf ");
        rmTcCmd.append(tcDir);
        let _ = spawn(rmTcCmd);
        return .Err(JessupError.InstallError("failed to extract toolchain archive"))
    }

    // Clean up temp files
    var rmCleanCmd = String();
    rmCleanCmd.append("rm -rf ");
    rmCleanCmd.append(tmpDir);
    let _ = spawn(rmCleanCmd);

    // Make binaries executable
    var chmodKestrel = String();
    chmodKestrel.append("chmod +x ");
    chmodKestrel.append(tcDir);
    chmodKestrel.append("/bin/kestrel");
    let _ = spawn(chmodKestrel);
    var chmodFlock = String();
    chmodFlock.append("chmod +x ");
    chmodFlock.append(tcDir);
    chmodFlock.append("/bin/flock");
    let _ = spawn(chmodFlock);
    var lspBin = String();
    lspBin.append(tcDir);
    lspBin.append("/bin/kestrel-lsp");
    if fileExists(lspBin) {
        var chmodLsp = String();
        chmodLsp.append("chmod +x ");
        chmodLsp.append(lspBin);
        let _ = spawn(chmodLsp);
    }
    var docBin = String();
    docBin.append(tcDir);
    docBin.append("/bin/kestrel-doc");
    if fileExists(docBin) {
        var chmodDoc = String();
        chmodDoc.append("chmod +x ");
        chmodDoc.append(docBin);
        let _ = spawn(chmodDoc);
    }

    var installedMsg = String();
    installedMsg.append("Installed ");
    installedMsg.append(toolchainName);
    let _ = println(installedMsg);

    // Install the VS Code / Cursor extension if an editor is available.
    installEditorExtension(channel: channel, platform: platform);

    // Install Claude Code / Codex plugins if the CLIs are available.
    installAgentPlugins();

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
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(toolchainName);
            tcDir = s
        }
    }

    if not isDirectory(tcDir) {
        var errMsg = String();
        errMsg.append("toolchain not installed: ");
        errMsg.append(toolchainName);
        return .Err(JessupError.NotFound(errMsg))
    }

    var binPath = "";
    match binDir() {
        .Err(e) => return .Err(e),
        .Ok(dir) => binPath = dir
    }

    var mkdirBinCmd = String();
    mkdirBinCmd.append("mkdir -p ");
    mkdirBinCmd.append(binPath);
    let _ = spawn(mkdirBinCmd);

    // Remove existing symlinks and create new ones
    var rmKestrel = String();
    rmKestrel.append("rm -f ");
    rmKestrel.append(binPath);
    rmKestrel.append("/kestrel");
    let _ = spawn(rmKestrel);
    var rmFlock = String();
    rmFlock.append("rm -f ");
    rmFlock.append(binPath);
    rmFlock.append("/flock");
    let _ = spawn(rmFlock);
    var rmLsp = String();
    rmLsp.append("rm -f ");
    rmLsp.append(binPath);
    rmLsp.append("/kestrel-lsp");
    let _ = spawn(rmLsp);
    var rmDoc = String();
    rmDoc.append("rm -f ");
    rmDoc.append(binPath);
    rmDoc.append("/kestrel-doc");
    let _ = spawn(rmDoc);

    var lnKestrel = String();
    lnKestrel.append("ln -s ");
    lnKestrel.append(tcDir);
    lnKestrel.append("/bin/kestrel ");
    lnKestrel.append(binPath);
    lnKestrel.append("/kestrel");
    let _ = spawn(lnKestrel);
    var lnFlock = String();
    lnFlock.append("ln -s ");
    lnFlock.append(tcDir);
    lnFlock.append("/bin/flock ");
    lnFlock.append(binPath);
    lnFlock.append("/flock");
    let _ = spawn(lnFlock);
    var lspBin = String();
    lspBin.append(tcDir);
    lspBin.append("/bin/kestrel-lsp");
    if fileExists(lspBin) {
        var lnLsp = String();
        lnLsp.append("ln -s ");
        lnLsp.append(lspBin);
        lnLsp.append(" ");
        lnLsp.append(binPath);
        lnLsp.append("/kestrel-lsp");
        let _ = spawn(lnLsp);
    }
    var docBin = String();
    docBin.append(tcDir);
    docBin.append("/bin/kestrel-doc");
    if fileExists(docBin) {
        var lnDoc = String();
        lnDoc.append("ln -s ");
        lnDoc.append(docBin);
        lnDoc.append(" ");
        lnDoc.append(binPath);
        lnDoc.append("/kestrel-doc");
        let _ = spawn(lnDoc);
    }

    // Update config with the channel name
    var config = readConfig();
    config.defaultChannel = toolchainName;
    match writeConfig(config: config) {
        .Err(e) => return .Err(e),
        .Ok(_) => {}
    }

    var defaultMsg = String();
    defaultMsg.append("Default toolchain set to ");
    defaultMsg.append(toolchainName);
    let _ = println(defaultMsg);

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
        if name.byteCount > 0 and name.bytes(unchecked: 0) != 46 {
            if name == activeChannel {
                var activeMsg = String();
                activeMsg.append("  ");
                activeMsg.append(name);
                activeMsg.append(" (active)");
                let _ = println(activeMsg);
            } else {
                var nameMsg = String();
                nameMsg.append("  ");
                nameMsg.append(name);
                let _ = println(nameMsg);
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
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(toolchainName);
            tcDir = s
        }
    }

    if not isDirectory(tcDir) {
        var errMsg = String();
        errMsg.append("toolchain not installed: ");
        errMsg.append(toolchainName);
        return .Err(JessupError.NotFound(errMsg))
    }

    // Check if this is the active toolchain
    let config = readConfig();
    if config.defaultChannel == toolchainName {
        let _ = println("Warning: removing the active toolchain. Run 'jessup default <version>' to set a new default.");
        // Remove symlinks
        match binDir() {
            .Ok(bp) => {
                var rmK = String();
                rmK.append("rm -f ");
                rmK.append(bp);
                rmK.append("/kestrel");
                let _ = spawn(rmK);
                var rmF = String();
                rmF.append("rm -f ");
                rmF.append(bp);
                rmF.append("/flock");
                let _ = spawn(rmF);
                var rmLsp = String();
                rmLsp.append("rm -f ");
                rmLsp.append(bp);
                rmLsp.append("/kestrel-lsp");
                let _ = spawn(rmLsp);
                var rmDoc = String();
                rmDoc.append("rm -f ");
                rmDoc.append(bp);
                rmDoc.append("/kestrel-doc");
                let _ = spawn(rmDoc);
            },
            .Err(_) => {}
        }
    }

    var rmTcCmd = String();
    rmTcCmd.append("rm -rf ");
    rmTcCmd.append(tcDir);
    let _ = spawn(rmTcCmd);
    var removedMsg = String();
    removedMsg.append("Removed toolchain ");
    removedMsg.append(toolchainName);
    let _ = println(removedMsg);

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
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(activeChannel);
            tcDir = s
        }
    }

    if isDirectory(tcDir) {
        var activeMsg = String();
        activeMsg.append("Active toolchain: ");
        activeMsg.append(activeChannel);
        let _ = println(activeMsg);
        var locMsg = String();
        locMsg.append("Location: ");
        locMsg.append(tcDir);
        let _ = println(locMsg);

        // Show kestrel version if available
        var kestrelBin = String();
        kestrelBin.append(tcDir);
        kestrelBin.append("/bin/kestrel");
        if fileExists(kestrelBin) {
            var versionCmd = String();
            versionCmd.append(kestrelBin);
            versionCmd.append(" --version");
            let version = captureOutput(versionCmd);
            var verMsg = String();
            verMsg.append("Version: ");
            verMsg.append(version);
            let _ = println(verMsg);
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
        // Update channels (stable-*, preview-*, beta-*, nightly-*)
        if name.starts(with: "stable") or name.starts(with: "preview") or name.starts(with: "beta") or name.starts(with: "nightly") {
            let channel = if name.starts(with: "stable") { "stable" }
                else if name.starts(with: "preview") { "preview" }
                else if name.starts(with: "beta") { "beta" }
                else { "nightly" };
            var updMsg = String();
            updMsg.append("Updating ");
            updMsg.append(channel);
            updMsg.append("...");
            let _ = println(updMsg);

            // Remove old version
            var rmOldCmd = String();
            rmOldCmd.append("rm -rf ");
            rmOldCmd.append(tcDirPath);
            rmOldCmd.append("/");
            rmOldCmd.append(name);
            let _ = spawn(rmOldCmd);

            // Install latest
            match installToolchain(channel: channel) {
                .Ok(newName) => {
                    updated = true;
                    // Re-link if this was the active toolchain
                    let config = readConfig();
                    if config.defaultChannel == name {
                        match setDefault(toolchainName: newName) {
                            .Ok(_) => {},
                            .Err(e) => {
                                let _ = eprintln(e.description());
                            }
                        }
                    }
                },
                .Err(e) => {
                    var failMsg = String();
                    failMsg.append("Failed to update ");
                    failMsg.append(channel);
                    failMsg.append(": ");
                    failMsg.append(e.description());
                    let _ = eprintln(failMsg);
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
    var mkdirSelfCmd = String();
    mkdirSelfCmd.append("mkdir -p ");
    mkdirSelfCmd.append(tmpDir);
    let _ = spawn(mkdirSelfCmd);
    var archivePath = String();
    archivePath.append(tmpDir);
    archivePath.append("/jessup.tar.gz");

    var curlCmd = String();
    curlCmd.append("curl -sL ");
    match getenv("GITHUB_TOKEN") {
        .Some(selfToken) => {
            if selfToken.byteCount > 0 {
                curlCmd.append("-H 'Authorization: Bearer ");
                curlCmd.append(selfToken);
                curlCmd.append("' -H 'Accept: application/octet-stream' ");
            }
        },
        .None => {}
    }
    curlCmd.append("-o ");
    curlCmd.append(archivePath);
    curlCmd.append(" ");
    curlCmd.append(downloadUrl);
    let exitCode = spawn(curlCmd);
    if exitCode != 0 {
        var rmSelfCmd = String();
        rmSelfCmd.append("rm -rf ");
        rmSelfCmd.append(tmpDir);
        let _ = spawn(rmSelfCmd);
        return .Err(JessupError.NetworkError("failed to download jessup update"))
    }

    // Extract and strip top-level directory
    var tarSelfCmd = String();
    tarSelfCmd.append("tar xzf ");
    tarSelfCmd.append(archivePath);
    tarSelfCmd.append(" -C ");
    tarSelfCmd.append(tmpDir);
    tarSelfCmd.append(" --strip-components=1");
    let _ = spawn(tarSelfCmd);
    var chmodSelfCmd = String();
    chmodSelfCmd.append("chmod +x ");
    chmodSelfCmd.append(tmpDir);
    chmodSelfCmd.append("/jessup");
    let _ = spawn(chmodSelfCmd);
    var mvCmd = String();
    mvCmd.append("mv ");
    mvCmd.append(tmpDir);
    mvCmd.append("/jessup ");
    mvCmd.append(bp);
    mvCmd.append("/jessup");
    let _ = spawn(mvCmd);
    var rmFinalCmd = String();
    rmFinalCmd.append("rm -rf ");
    rmFinalCmd.append(tmpDir);
    let _ = spawn(rmFinalCmd);

    let _ = println("jessup has been updated");

    .Ok(())
}

// ============================================================================
// EDITOR EXTENSION
// ============================================================================

/// Attempts to install the Kestrel VS Code extension (.vsix) into any
/// detected editor (VS Code, Cursor, or codium). Silently skips if no
/// supported editor CLI is found or the VSIX isn't available.
public func installEditorExtension(channel channel: String, platform platform: Platform) {
    // Detect which editor CLIs are available.
    var editors = Array[String]();
    if spawn("which code > /dev/null 2>&1") == 0 {
        editors.append("code")
    }
    if spawn("which cursor > /dev/null 2>&1") == 0 {
        editors.append("cursor")
    }
    if spawn("which codium > /dev/null 2>&1") == 0 {
        editors.append("codium")
    }
    if editors.count == 0 {
        return
    }

    // Fetch the VSIX download URL from the same release.
    var vsixUrl = "";
    match fetchVsixRelease(channel: channel, platform: platform) {
        .Err(_) => return,
        .Ok(url) => vsixUrl = url
    }

    // Download the VSIX to a temp file.
    let tmpVsix = "/tmp/jessup-kestrel-extension.vsix";
    var dlCmd = String();
    dlCmd.append("curl -sL ");
    match getenv("GITHUB_TOKEN") {
        .Some(vsixToken) => {
            if vsixToken.byteCount > 0 {
                dlCmd.append("-H 'Authorization: Bearer ");
                dlCmd.append(vsixToken);
                dlCmd.append("' -H 'Accept: application/octet-stream' ");
            }
        },
        .None => {}
    }
    dlCmd.append("-o ");
    dlCmd.append(tmpVsix);
    dlCmd.append(" ");
    dlCmd.append(vsixUrl);
    if spawn(dlCmd) != 0 {
        return
    }

    // Install into each detected editor.
    var i: Int64 = 0;
    while i < editors.count {
        let editor = editors(unchecked: i);
        var installMsg = String();
        installMsg.append("Installing Kestrel extension for ");
        installMsg.append(editor);
        installMsg.append("...");
        let _ = println(installMsg);

        var installCmd = String();
        installCmd.append(editor);
        installCmd.append(" --install-extension ");
        installCmd.append(tmpVsix);
        installCmd.append(" --force 2>/dev/null");
        let _ = spawn(installCmd);
        i = i + 1
    }

    // Clean up.
    var rmVsix = String();
    rmVsix.append("rm -f ");
    rmVsix.append(tmpVsix);
    let _ = spawn(rmVsix);
}

// ============================================================================
// AGENT PLUGINS
// ============================================================================

/// Installs the Kestrel plugin for Claude Code and Codex if either CLI
/// is detected. Silently skips if neither is available.
func installAgentPlugins() {
    if spawn("which claude > /dev/null 2>&1") == 0 {
        let _ = println("Installing Kestrel plugin for Claude Code...");
        let _ = spawn("claude plugin add kestrellang/kestrel-plugin 2>/dev/null");
    }
    if spawn("which codex > /dev/null 2>&1") == 0 {
        let _ = println("Installing Kestrel plugin for Codex...");
        let _ = spawn("codex plugin add kestrellang/kestrel-plugin 2>/dev/null");
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Generates a toolchain directory name from channel and release tag.
/// e.g., "stable" + "v1.0.0"  -> "stable-1.0.0"
/// e.g., "preview" + "v0.16.0" -> "preview-0.16.0"
/// e.g., "beta" + "beta" -> "beta-2026-03-02"
/// e.g., "nightly" + "nightly" -> "nightly-2026-03-02"
func toolchainDirName(channel channel: String, tag tag: String) -> String {
    if channel == "nightly" or channel == "beta" {
        // Rolling channels — the tag is fixed (`nightly`/`beta`), so
        // disambiguate installs by date instead.
        let date = captureOutput("date +%Y-%m-%d");
        let trimmed = trimTrailingNewline(date);
        var s = String();
        s.append(channel);
        s.append("-");
        s.append(trimmed);
        s
    } else if channel == "stable" or channel == "preview" {
        // Named version channels resolve to a concrete tag; name the dir
        // "<channel>-<version>" with any leading 'v' stripped.
        var s = String();
        s.append(channel);
        s.append("-");
        s.append(stripLeadingV(tag: tag));
        s
    } else {
        // Specific version — use as-is
        channel
    }
}

/// Strips a single leading 'v' (byte 118) from a tag, if present.
func stripLeadingV(tag tag: String) -> String {
    if tag.byteCount > 0 and tag.bytes(unchecked: 0) == 118 {
        tag.asSlice().subslice(from: 1, to: tag.byteCount).toOwned()
    } else {
        tag
    }
}

func trimTrailingNewline(s: String) -> String {
    let len = s.byteCount;
    var end = len;
    while end > 0 {
        let b = s.bytes(unchecked: end - 1);
        if b == 10 or b == 13 {
            end = end - 1
        } else {
            return s.asSlice().subslice(from: 0, to: end).toOwned()
        }
    }
    ""
}
