// Registry package source — resolves dependencies from a package registry
//
// Implements the PackageSource protocol for registry dependencies.
// Fetches package metadata via HTTP, downloads archives, and caches locally.

module flock.registry_source

import swoop.swoop.(Swoop)
import quill.value.(Value)
import flock.error.(FlockError)
import flock.version.(Version, VersionConstraint, parseVersion, satisfies)
import flock.dependency.(DependencySpec)
import flock.source.(ResolvedPackage, PackageSource, joinPath)
import flock.manifest.(Manifest, parseManifest)
import flock.cache.(cachePath, isCached, ensureCacheDir, downloadFile, extractArchive)
import flock.registry.(RegistryConfig, splitPackageName)

// ============================================================================
// REGISTRY SOURCE
// ============================================================================

/// Resolves package dependencies from a remote registry.
public struct RegistrySource: PackageSource, Cloneable {
    var config: RegistryConfig

    public init(config config: RegistryConfig) {
        self.config = config;
    }

    public func resolve(name name: String, spec spec: DependencySpec, baseDir baseDir: String) -> Result[ResolvedPackage, FlockError] {
        match spec {
            .Path(_) => {
                .Err(FlockError.DependencyNotFound("\(name) (path dependency sent to registry source)"))
            },
            .Registry(constraint) => {
                self.resolveRegistry(name: name, constraint: constraint)
            }
        }
    }

    // ========================================================================
    // RESOLUTION
    // ========================================================================

    func resolveRegistry(name name: String, constraint constraint: VersionConstraint) -> Result[ResolvedPackage, FlockError] {
        // 1. Split org/pkg
        match splitPackageName(name: name) {
            .None => {
                return .Err(FlockError.DependencyNotFound("\(name) (registry packages must use org/pkg format)"))
            },
            .Some(parts) => {
                let org = parts.0;
                let pkg = parts.1;

                // 2. Fetch available versions from registry
                var versions = Array[Version]();
                match self.fetchVersions(org: org, pkg: pkg) {
                    .Err(e) => return .Err(e),
                    .Ok(v) => versions = v
                }

                // 3. Select best version satisfying constraint
                match selectBestVersion(versions: versions, constraint: constraint) {
                    .None => {
                        return .Err(FlockError.DependencyNotFound("\(name) (no version satisfies constraint)"))
                    },
                    .Some(bestVersion) => {
                        // 4. Check local cache
                        if isCached(org: org, pkg: pkg, version: bestVersion) {
                            match cachePath(org: org, pkg: pkg, version: bestVersion) {
                                .Err(e) => return .Err(e),
                                .Ok(pkgDir) => {
                                    return loadCachedPackage(pkgDir: pkgDir)
                                }
                            }
                        }

                        // 5. Download, cache, and load
                        self.downloadAndCache(org: org, pkg: pkg, version: bestVersion)
                    }
                }
            }
        }
    }

    // ========================================================================
    // REGISTRY API
    // ========================================================================

    /// Fetches the list of available versions for a package from the registry.
    ///
    /// API: GET /api/v1/packages/{org}/{pkg}
    /// Response: { "name": "org/pkg", "versions": ["1.0.0", "1.1.0", ...] }
    func fetchVersions(org org: String, pkg pkg: String) -> Result[Array[Version], FlockError] {
        let url = "\(self.config.url)/api/v1/packages/\(org)/\(pkg)";

        var client = Swoop();
        client = client.header("Accept", "application/json");

        match client.fetch(url) {
            .Err(_) => {
                return .Err(FlockError.RegistryError("failed to fetch package info for \(org)/\(pkg)"))
            },
            .Ok(resp) => {
                if not resp.status.isSuccess() {
                    return .Err(FlockError.RegistryError("\(org)/\(pkg): registry returned status \(resp.status.code)"))
                }

                match resp.json() {
                    .Err(_) => {
                        return .Err(FlockError.RegistryError("invalid JSON response for \(org)/\(pkg)"))
                    },
                    .Ok(json) => {
                        parseVersionList(json: json)
                    }
                }
            }
        }
    }

    /// Fetches version metadata including checksum and download URL.
    ///
    /// API: GET /api/v1/packages/{org}/{pkg}/{version}
    /// Response: { "name": "org/pkg", "version": "1.2.3", "checksum": "sha256:...",
    ///             "archive_url": "/api/v1/packages/{org}/{pkg}/{version}/download" }
    func fetchVersionMeta(org org: String, pkg pkg: String, version version: Version) -> Result[VersionMeta, FlockError] {
        let versionStr = version.toString();
        let url = "\(self.config.url)/api/v1/packages/\(org)/\(pkg)/\(versionStr)";

        var client = Swoop();
        client = client.header("Accept", "application/json");

        match client.fetch(url) {
            .Err(_) => {
                return .Err(FlockError.RegistryError("failed to fetch version info for \(org)/\(pkg)@\(versionStr)"))
            },
            .Ok(resp) => {
                if not resp.status.isSuccess() {
                    return .Err(FlockError.RegistryError("\(org)/\(pkg)@\(versionStr): registry returned status \(resp.status.code)"))
                }

                match resp.json() {
                    .Err(_) => {
                        return .Err(FlockError.RegistryError("invalid JSON response for \(org)/\(pkg)@\(versionStr)"))
                    },
                    .Ok(json) => {
                        parseVersionMeta(json: json)
                    }
                }
            }
        }
    }

    // ========================================================================
    // DOWNLOAD & CACHE
    // ========================================================================

    func downloadAndCache(org org: String, pkg pkg: String, version version: Version) -> Result[ResolvedPackage, FlockError] {
        // 1. Fetch version metadata (contains checksum and download URL)
        var meta = VersionMeta(checksum: "", archiveUrl: "");
        match self.fetchVersionMeta(org: org, pkg: pkg, version: version) {
            .Err(e) => return .Err(e),
            .Ok(m) => meta = m
        }

        // 2. Ensure cache directory exists
        var pkgDir = "";
        match ensureCacheDir(org: org, pkg: pkg, version: version) {
            .Err(e) => return .Err(e),
            .Ok(p) => pkgDir = p
        }

        // 3. Download archive
        let archivePath = "\(pkgDir)/archive.tar.gz";
        let downloadUrl = "\(self.config.url)\(meta.archiveUrl)";
        match downloadFile(url: downloadUrl, outputPath: archivePath) {
            .Err(e) => return .Err(e),
            .Ok(_) => {}
        }

        // 4. Extract archive
        match extractArchive(archivePath: archivePath, targetDir: pkgDir) {
            .Err(e) => return .Err(e),
            .Ok(_) => {}
        }

        // 5. Load the cached package
        loadCachedPackage(pkgDir: pkgDir)
    }

    public func clone() -> RegistrySource {
        RegistrySource(config: self.config.clone())
    }
}

// ============================================================================
// VERSION METADATA
// ============================================================================

/// Metadata about a specific package version from the registry.
struct VersionMeta: Cloneable {
    var checksum: String
    var archiveUrl: String

    init(checksum checksum: String, archiveUrl archiveUrl: String) {
        self.checksum = checksum;
        self.archiveUrl = archiveUrl;
    }

    func clone() -> VersionMeta {
        VersionMeta(checksum: self.checksum.clone(), archiveUrl: self.archiveUrl.clone())
    }
}

// ============================================================================
// JSON PARSING HELPERS
// ============================================================================

/// Parses a JSON response containing a version list.
/// Expected format: { "versions": ["1.0.0", "1.1.0", "2.0.0"] }
func parseVersionList(json json: Value) -> Result[Array[Version], FlockError] {
    match json.value(forKey: "versions") {
        .None => .Err(FlockError.RegistryError("missing 'versions' field in response")),
        .Some(versionsVal) => {
            match versionsVal.asArray() {
                .None => .Err(FlockError.RegistryError("'versions' is not an array")),
                .Some(arr) => {
                    var result = Array[Version]();
                    var i: Int64 = 0;
                    while i < arr.count {
                        match arr(unchecked: i).asString() {
                            .Some(vStr) => {
                                match parseVersion(s: vStr) {
                                    .Ok(v) => result.append(v),
                                    .Err(_) => {}
                                }
                            },
                            .None => {}
                        }
                        i = i + 1
                    }
                    .Ok(result)
                }
            }
        }
    }
}

/// Parses version metadata JSON.
/// Expected format: { "checksum": "sha256:...", "archive_url": "/api/v1/..." }
func parseVersionMeta(json json: Value) -> Result[VersionMeta, FlockError] {
    var checksum = "";
    match json.value(forKey: "checksum") {
        .Some(val) => {
            match val.asString() {
                .Some(s) => checksum = s,
                .None => {}
            }
        },
        .None => {}
    }

    match json.value(forKey: "archive_url") {
        .None => .Err(FlockError.RegistryError("missing 'archive_url' in version metadata")),
        .Some(val) => {
            match val.asString() {
                .None => .Err(FlockError.RegistryError("'archive_url' is not a string")),
                .Some(url) => {
                    .Ok(VersionMeta(checksum: checksum, archiveUrl: url))
                }
            }
        }
    }
}

// ============================================================================
// VERSION SELECTION
// ============================================================================

/// Selects the highest version that satisfies the given constraint.
func selectBestVersion(versions versions: Array[Version], constraint constraint: VersionConstraint) -> Optional[Version] {
    var best: Optional[Version] = .None;
    var i: Int64 = 0;
    while i < versions.count {
        let v = versions(unchecked: i);
        if satisfies(v, constraint) {
            match best {
                .None => best = .Some(v),
                .Some(current) => {
                    if current.lessThan(other: v) {
                        best = .Some(v)
                    }
                }
            }
        }
        i = i + 1
    }
    best
}

// ============================================================================
// CACHE LOADING
// ============================================================================

/// Loads a package from its cached directory.
func loadCachedPackage(pkgDir pkgDir: String) -> Result[ResolvedPackage, FlockError] {
    let manifestPath = joinPath(base: pkgDir, rel: "flock.toml");

    match readFileString(manifestPath) {
        .Err(_) => .Err(FlockError.ManifestNotFound(manifestPath)),
        .Ok(source) => {
            match parseManifest(source: source) {
                .Err(e) => .Err(e),
                .Ok(manifest) => {
                    .Ok(ResolvedPackage(
                        name: manifest.package.name,
                        version: manifest.package.version,
                        rootDir: pkgDir,
                        manifest: manifest
                    ))
                }
            }
        }
    }
}
