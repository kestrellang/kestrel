// Local package cache management
//
// Packages are cached at ~/.flock/packages/{org}/{pkg}/{version}/
// Each version directory is a self-contained package root with flock.toml + src/.

module flock.cache

import flock.error.(FlockError)
import flock.version.(Version)
import flock.source.(joinPath, flockHome)

// ============================================================================
// CACHE PATHS
// ============================================================================

/// Returns the cache root directory: ~/.kestrel/packages/
public func cacheRoot() -> String throws FlockError {
    match getenv("HOME") {
        .Some(home) => .Ok(joinPath(base: home, rel: ".kestrel/packages")),
        .None => throw FlockError.CacheError("HOME environment variable not set")
    }
}

/// Returns the cache path for a specific package version.
/// e.g., ~/.kestrel/packages/kestrel/swoop/1.0.0/
public func cachePath(org org: String, pkg pkg: String, version version: Version) -> String throws FlockError {
    match cacheRoot() {
        .Err(e) => throw e,
        .Ok(root) => {
            let orgDir = joinPath(base: root, rel: org);
            let pkgDir = joinPath(base: orgDir, rel: pkg);

            let versionDir = joinPath(base: pkgDir, rel: version.toString());
            .Ok(versionDir)
        }
    }
}

// ============================================================================
// CACHE OPERATIONS
// ============================================================================

/// Checks if a package version is already cached (flock.toml exists in cache dir).
public func isCached(org org: String, pkg pkg: String, version version: Version) -> Bool {
    match cachePath(org: org, pkg: pkg, version: version) {
        .Ok(path) => {
            let manifest = joinPath(base: path, rel: "flock.toml");
            fileExists(manifest)
        },
        .Err(_) => false
    }
}

/// Ensures the cache directory exists for a given org/pkg/version.
/// Creates all intermediate directories if needed.
/// Returns the full cache path on success.
public func ensureCacheDir(org org: String, pkg pkg: String, version version: Version) -> String throws FlockError {
    match cachePath(org: org, pkg: pkg, version: version) {
        .Err(e) => throw e,
        .Ok(path) => {
            match mkdirAll(path) {
                .Ok(_) => .Ok(path),
                .Err(_) => {
                    var msg = String(); msg.append("cannot create cache directory: "); msg.append(path);
                    .Err(FlockError.CacheError(msg))
                }
            }
        }
    }
}

// ============================================================================
// ARCHIVE OPERATIONS
// ============================================================================

/// Downloads a file from a URL to a local path using curl.
public func downloadFile(url url: String, outputPath outputPath: String) -> () throws FlockError {
    var cmd = String(); cmd.append("curl -sL -o "); cmd.append(outputPath); cmd.append(" "); cmd.append(url);
    let exitCode = spawn(cmd).unwrap(or: -1);
    if exitCode != 0 {
        var msg = String(); msg.append("download failed: "); msg.append(url);
        throw FlockError.RegistryError(msg)
    }
    .Ok(())
}

/// Extracts a .tar.gz archive into the target directory.
public func extractArchive(archivePath archivePath: String, targetDir targetDir: String) -> () throws FlockError {
    var cmd = String(); cmd.append("tar xzf "); cmd.append(archivePath); cmd.append(" -C "); cmd.append(targetDir);
    let exitCode = spawn(cmd).unwrap(or: -1);
    if exitCode != 0 {
        var msg = String(); msg.append("failed to extract archive: "); msg.append(archivePath);
        throw FlockError.CacheError(msg)
    }
    .Ok(())
}
