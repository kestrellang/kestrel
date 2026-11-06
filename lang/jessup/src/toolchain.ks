// Toolchain management
//
// Install, remove, list, and link kestrel toolchains.
// Each toolchain lives in ~/.jessup/toolchains/<name>/ with bin/ and lib/std/.

module jessup.toolchain

import clutch.shell.(runShell, shellOutput, entryNames)

import jessup.error.(JessupError)
import jessup.config.(jessupHome, binDir, toolchainsDir, ensureDirectories, readConfig, writeConfig, JessupConfig)
import jessup.platform.(Platform, detectPlatform)
import jessup.github.(Release, fetchRelease, fetchJessupRelease)

// ============================================================================
// INSTALL
// ============================================================================

/// Installs a toolchain for the given channel.
/// channel can be "stable", "nightly", or a specific version like "1.0.0".
public func installToolchain(channel channel: String) -> String throws JessupError {
    // Ensure directories exist
    match ensureDirectories() {
        .Err(e) => throw e,
        .Ok(_) => {}
    }

    // Detect platform
    var platform = Platform(os: "", arch: "");
    match detectPlatform() {
        .Err(e) => throw e,
        .Ok(p) => platform = p
    }

    var fetchMsg = String();
    fetchMsg.append("Fetching release info for ");
    fetchMsg.append(channel);
    fetchMsg.append("...");
     println(fetchMsg);

    // Fetch release info from GitHub
    var release = Release(tagName: "", assetUrl: "");
    match fetchRelease(channel: channel, platform: platform) {
        .Err(e) => throw e,
        .Ok(r) => release = r
    }

    // Determine toolchain name
    let toolchainName = toolchainDirName(channel: channel, tag: release.tagName);

    // Check if already installed
    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => throw e,
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(toolchainName);
            tcDir = s
        }
    }

    if Path(tcDir).isDirectory() {
        var alreadyMsg = String();
        alreadyMsg.append("Toolchain ");
        alreadyMsg.append(toolchainName);
        alreadyMsg.append(" is already installed");
         println(alreadyMsg);
        return .Ok(toolchainName)
    }

    var dlMsg = String();
    dlMsg.append("Downloading ");
    dlMsg.append(toolchainName);
    dlMsg.append("...");
     println(dlMsg);

    // Create temp directory for download
    var tmpDir = String();
    tmpDir.append("/tmp/jessup-download-");
    tmpDir.append(toolchainName);
    var mkdirCmd = String();
    mkdirCmd.append("mkdir -p ");
    mkdirCmd.append(tmpDir);
     runShell(mkdirCmd);
    var archivePath = String();
    archivePath.append(tmpDir);
    archivePath.append("/toolchain.tar.gz");

    // Download using curl (handles GitHub redirects)
    var curlCmd = String();
    curlCmd.append("curl -sL ");
    match Environment.variable("GITHUB_TOKEN") {
        .Some(token) => {
            if token.bytes.count > 0 {
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
    let exitCode = runShell(curlCmd);
    if exitCode != 0 {
        var rmCmd = String();
        rmCmd.append("rm -rf ");
        rmCmd.append(tmpDir);
         runShell(rmCmd);
        throw JessupError.NetworkError("failed to download toolchain")
    }

    // Create toolchain directory
    var mkdirTcCmd = String();
    mkdirTcCmd.append("mkdir -p ");
    mkdirTcCmd.append(tcDir);
     runShell(mkdirTcCmd);

    // Extract archive (strip the top-level directory from the tarball)
    var tarCmd = String();
    tarCmd.append("tar xzf ");
    tarCmd.append(archivePath);
    tarCmd.append(" -C ");
    tarCmd.append(tcDir);
    tarCmd.append(" --strip-components=1");
    let tarExit = runShell(tarCmd);
    if tarExit != 0 {
        var rmTmpCmd = String();
        rmTmpCmd.append("rm -rf ");
        rmTmpCmd.append(tmpDir);
         runShell(rmTmpCmd);
        var rmTcCmd = String();
        rmTcCmd.append("rm -rf ");
        rmTcCmd.append(tcDir);
         runShell(rmTcCmd);
        throw JessupError.InstallError("failed to extract toolchain archive")
    }

    // Clean up temp files
    var rmCleanCmd = String();
    rmCleanCmd.append("rm -rf ");
    rmCleanCmd.append(tmpDir);
     runShell(rmCleanCmd);

    // Make binaries executable
    var chmodKestrel = String();
    chmodKestrel.append("chmod +x ");
    chmodKestrel.append(tcDir);
    chmodKestrel.append("/bin/kestrel");
     runShell(chmodKestrel);
    var chmodFlock = String();
    chmodFlock.append("chmod +x ");
    chmodFlock.append(tcDir);
    chmodFlock.append("/bin/flock");
     runShell(chmodFlock);
    var lspBin = String();
    lspBin.append(tcDir);
    lspBin.append("/bin/kestrel-lsp");
    if Path(lspBin).exists() {
        var chmodLsp = String();
        chmodLsp.append("chmod +x ");
        chmodLsp.append(lspBin);
         runShell(chmodLsp);
    }
    var docBin = String();
    docBin.append(tcDir);
    docBin.append("/bin/kestrel-doc");
    if Path(docBin).exists() {
        var chmodDoc = String();
        chmodDoc.append("chmod +x ");
        chmodDoc.append(docBin);
         runShell(chmodDoc);
    }

    var installedMsg = String();
    installedMsg.append("Installed ");
    installedMsg.append(toolchainName);
     println(installedMsg);

    // Editor and agent integrations are opt-in. Rather than silently mutating
    // the user's editor or agent config, print the install commands so they
    // can run whichever they want.
    printIntegrationHints();

    .Ok(toolchainName)
}

// ============================================================================
// DEFAULT (SYMLINK MANAGEMENT)
// ============================================================================

/// Sets the default toolchain by updating symlinks in ~/.jessup/bin/.
public func setDefault(toolchainName toolchainName: String) -> () throws JessupError {
    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => throw e,
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(toolchainName);
            tcDir = s
        }
    }

    if not Path(tcDir).isDirectory() {
        var errMsg = String();
        errMsg.append("toolchain not installed: ");
        errMsg.append(toolchainName);
        throw JessupError.NotFound(errMsg)
    }

    var binPath = "";
    match binDir() {
        .Err(e) => throw e,
        .Ok(dir) => binPath = dir
    }

    var mkdirBinCmd = String();
    mkdirBinCmd.append("mkdir -p ");
    mkdirBinCmd.append(binPath);
     runShell(mkdirBinCmd);

    // Remove existing symlinks and create new ones
    var rmKestrel = String();
    rmKestrel.append("rm -f ");
    rmKestrel.append(binPath);
    rmKestrel.append("/kestrel");
     runShell(rmKestrel);
    var rmFlock = String();
    rmFlock.append("rm -f ");
    rmFlock.append(binPath);
    rmFlock.append("/flock");
     runShell(rmFlock);
    var rmLsp = String();
    rmLsp.append("rm -f ");
    rmLsp.append(binPath);
    rmLsp.append("/kestrel-lsp");
     runShell(rmLsp);
    var rmDoc = String();
    rmDoc.append("rm -f ");
    rmDoc.append(binPath);
    rmDoc.append("/kestrel-doc");
     runShell(rmDoc);

    var lnKestrel = String();
    lnKestrel.append("ln -s ");
    lnKestrel.append(tcDir);
    lnKestrel.append("/bin/kestrel ");
    lnKestrel.append(binPath);
    lnKestrel.append("/kestrel");
     runShell(lnKestrel);
    var lnFlock = String();
    lnFlock.append("ln -s ");
    lnFlock.append(tcDir);
    lnFlock.append("/bin/flock ");
    lnFlock.append(binPath);
    lnFlock.append("/flock");
     runShell(lnFlock);
    var lspBin = String();
    lspBin.append(tcDir);
    lspBin.append("/bin/kestrel-lsp");
    if Path(lspBin).exists() {
        var lnLsp = String();
        lnLsp.append("ln -s ");
        lnLsp.append(lspBin);
        lnLsp.append(" ");
        lnLsp.append(binPath);
        lnLsp.append("/kestrel-lsp");
         runShell(lnLsp);
    }
    var docBin = String();
    docBin.append(tcDir);
    docBin.append("/bin/kestrel-doc");
    if Path(docBin).exists() {
        var lnDoc = String();
        lnDoc.append("ln -s ");
        lnDoc.append(docBin);
        lnDoc.append(" ");
        lnDoc.append(binPath);
        lnDoc.append("/kestrel-doc");
         runShell(lnDoc);
    }

    // Update config with the channel name
    var config = readConfig();
    config.defaultChannel = toolchainName;
    match writeConfig(config: config) {
        .Err(e) => throw e,
        .Ok(_) => {}
    }

    var defaultMsg = String();
    defaultMsg.append("Default toolchain set to ");
    defaultMsg.append(toolchainName);
     println(defaultMsg);

    .Ok(())
}

// ============================================================================
// LIST
// ============================================================================

/// Lists all installed toolchains. Marks the active one.
public func listToolchains() -> () throws JessupError {
    var tcDirPath = "";
    match toolchainsDir() {
        .Err(e) => throw e,
        .Ok(dir) => tcDirPath = dir
    }

    if not Path(tcDirPath).isDirectory() {
         println("No toolchains installed");
        return .Ok(())
    }

    let entries = entryNames(tcDirPath);
    if entries.count == 0 {
         println("No toolchains installed");
        return .Ok(())
    }

    // Determine the active toolchain from config
    let config = readConfig();
    let activeChannel = config.defaultChannel;

     println("Installed toolchains:");
     println("");

    for name in entries {
        // Skip hidden files
        if name.bytes.count > 0 and unsafe name.bytes(unchecked: 0) != 46 {
            if name == activeChannel {
                var activeMsg = String();
                activeMsg.append("  ");
                activeMsg.append(name);
                activeMsg.append(" (active)");
                 println(activeMsg);
            } else {
                var nameMsg = String();
                nameMsg.append("  ");
                nameMsg.append(name);
                 println(nameMsg);
            }
        }
    }

    .Ok(())
}

// ============================================================================
// REMOVE
// ============================================================================

/// Removes an installed toolchain.
public func removeToolchain(toolchainName toolchainName: String) -> () throws JessupError {
    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => throw e,
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(toolchainName);
            tcDir = s
        }
    }

    if not Path(tcDir).isDirectory() {
        var errMsg = String();
        errMsg.append("toolchain not installed: ");
        errMsg.append(toolchainName);
        throw JessupError.NotFound(errMsg)
    }

    // Check if this is the active toolchain
    let config = readConfig();
    if config.defaultChannel == toolchainName {
         println("Warning: removing the active toolchain. Run 'jessup default <version>' to set a new default.");
        // Remove symlinks
        match binDir() {
            .Ok(bp) => {
                var rmK = String();
                rmK.append("rm -f ");
                rmK.append(bp);
                rmK.append("/kestrel");
                 runShell(rmK);
                var rmF = String();
                rmF.append("rm -f ");
                rmF.append(bp);
                rmF.append("/flock");
                 runShell(rmF);
                var rmLsp = String();
                rmLsp.append("rm -f ");
                rmLsp.append(bp);
                rmLsp.append("/kestrel-lsp");
                 runShell(rmLsp);
                var rmDoc = String();
                rmDoc.append("rm -f ");
                rmDoc.append(bp);
                rmDoc.append("/kestrel-doc");
                 runShell(rmDoc);
            },
            .Err(_) => {}
        }
    }

    var rmTcCmd = String();
    rmTcCmd.append("rm -rf ");
    rmTcCmd.append(tcDir);
     runShell(rmTcCmd);
    var removedMsg = String();
    removedMsg.append("Removed toolchain ");
    removedMsg.append(toolchainName);
     println(removedMsg);

    .Ok(())
}

// ============================================================================
// SHOW
// ============================================================================

/// Shows the active toolchain and its path.
public func showActive() -> () throws JessupError {
    let config = readConfig();
    let activeChannel = config.defaultChannel;

    var tcDir = "";
    match toolchainsDir() {
        .Err(e) => throw e,
        .Ok(dir) => {
            var s = String();
            s.append(dir);
            s.append("/");
            s.append(activeChannel);
            tcDir = s
        }
    }

    if Path(tcDir).isDirectory() {
        var activeMsg = String();
        activeMsg.append("Active toolchain: ");
        activeMsg.append(activeChannel);
         println(activeMsg);
        var locMsg = String();
        locMsg.append("Location: ");
        locMsg.append(tcDir);
         println(locMsg);

        // Show kestrel version if available
        var kestrelBin = String();
        kestrelBin.append(tcDir);
        kestrelBin.append("/bin/kestrel");
        if Path(kestrelBin).exists() {
            var versionCmd = String();
            versionCmd.append(kestrelBin);
            versionCmd.append(" --version");
            let version = shellOutput(versionCmd);
            var verMsg = String();
            verMsg.append("Version: ");
            verMsg.append(version);
             println(verMsg);
        }
    } else {
         println("No active toolchain. Run 'jessup install stable' to get started.");
    };

    .Ok(())
}

// ============================================================================
// UPDATE
// ============================================================================

/// Updates all installed channel toolchains (stable, nightly) to their latest versions.
public func updateToolchains() -> () throws JessupError {
    var tcDirPath = "";
    match toolchainsDir() {
        .Err(e) => throw e,
        .Ok(dir) => tcDirPath = dir
    }

    if not Path(tcDirPath).isDirectory() {
         println("No toolchains installed");
        return .Ok(())
    }

    let entries = entryNames(tcDirPath);
    var updated = false;

    for name in entries {
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
             println(updMsg);

            // Remove old version
            var rmOldCmd = String();
            rmOldCmd.append("rm -rf ");
            rmOldCmd.append(tcDirPath);
            rmOldCmd.append("/");
            rmOldCmd.append(name);
             runShell(rmOldCmd);

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
                                 eprintln(e.description());
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
                     eprintln(failMsg);
                }
            }
        }
    }

    if not updated {
         println("No updatable channels found (install stable or nightly first)");
    }

    .Ok(())
}

// ============================================================================
// SELF UPDATE
// ============================================================================

/// Updates jessup itself to the latest version.
public func selfUpdate() -> () throws JessupError {
    var platform = Platform(os: "", arch: "");
    match detectPlatform() {
        .Err(e) => throw e,
        .Ok(p) => platform = p
    }

     println("Checking for jessup updates...");

    var downloadUrl = "";
    match fetchJessupRelease(platform: platform) {
        .Err(e) => throw e,
        .Ok(url) => downloadUrl = url
    }

    var bp = "";
    match binDir() {
        .Err(e) => throw e,
        .Ok(dir) => bp = dir
    }

    let tmpDir = "/tmp/jessup-self-update";
    var mkdirSelfCmd = String();
    mkdirSelfCmd.append("mkdir -p ");
    mkdirSelfCmd.append(tmpDir);
     runShell(mkdirSelfCmd);
    var archivePath = String();
    archivePath.append(tmpDir);
    archivePath.append("/jessup.tar.gz");

    var curlCmd = String();
    curlCmd.append("curl -sL ");
    match Environment.variable("GITHUB_TOKEN") {
        .Some(selfToken) => {
            if selfToken.bytes.count > 0 {
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
    let exitCode = runShell(curlCmd);
    if exitCode != 0 {
        var rmSelfCmd = String();
        rmSelfCmd.append("rm -rf ");
        rmSelfCmd.append(tmpDir);
         runShell(rmSelfCmd);
        throw JessupError.NetworkError("failed to download jessup update")
    }

    // Extract and strip top-level directory
    var tarSelfCmd = String();
    tarSelfCmd.append("tar xzf ");
    tarSelfCmd.append(archivePath);
    tarSelfCmd.append(" -C ");
    tarSelfCmd.append(tmpDir);
    tarSelfCmd.append(" --strip-components=1");
     runShell(tarSelfCmd);
    var chmodSelfCmd = String();
    chmodSelfCmd.append("chmod +x ");
    chmodSelfCmd.append(tmpDir);
    chmodSelfCmd.append("/jessup");
     runShell(chmodSelfCmd);
    var mvCmd = String();
    mvCmd.append("mv ");
    mvCmd.append(tmpDir);
    mvCmd.append("/jessup ");
    mvCmd.append(bp);
    mvCmd.append("/jessup");
     runShell(mvCmd);
    var rmFinalCmd = String();
    rmFinalCmd.append("rm -rf ");
    rmFinalCmd.append(tmpDir);
     runShell(rmFinalCmd);

     println("jessup has been updated");

    .Ok(())
}

// ============================================================================
// INTEGRATION HINTS
// ============================================================================

/// Prints the commands for the optional editor extension and AI-agent plugins.
/// We deliberately do NOT install these automatically: a toolchain install
/// shouldn't silently mutate the user's editor or agent config. The user runs
/// whichever lines they want.
func printIntegrationHints() {
     println("");
     println("Optional integrations (run the ones you want):");
     println("");
     println("  Editor extension (VS Code / Cursor): download the .vsix for your");
     println("  platform from https://github.com/kestrellang/kestrel-vscode/releases/latest");
     println("  then run: code --install-extension <file>.vsix");
     println("");
     println("  Claude Code plugin (Kestrel guidance + kestrel-lsp):");
     println("    claude plugin marketplace add kestrellang/kestrel-plugin");
     println("    claude plugin install kestrel-plugin@kestrel");
     println("");
     println("  Codex plugin:");
     println("    codex plugin marketplace add kestrellang/kestrel-plugin");
     println("    codex plugin add kestrel-plugin@kestrel");
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
        let date = shellOutput("date +%Y-%m-%d");
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
    if tag.bytes.count > 0 and unsafe tag.bytes(unchecked: 0) == 118 {
        tag.asSlice().subslice(from: 1, to: tag.bytes.count).toOwned()
    } else {
        tag
    }
}

func trimTrailingNewline(s: String) -> String {
    let len = s.bytes.count;
    var end = len;
    while end > 0 {
        let b = unsafe s.bytes(unchecked: end - 1);
        if b == 10 or b == 13 {
            end = end - 1
        } else {
            return s.asSlice().subslice(from: 0, to: end).toOwned()
        }
    }
    ""
}
