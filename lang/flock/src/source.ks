// Package source abstraction and path-based implementation
//
// PackageSource protocol enables dependency inversion:
// - PathSource resolves local path dependencies (MVP)
// - Future: RegistrySource, GitSource

module flock.source

import flock.error.(FlockError)
import flock.version.(Version)
import flock.dependency.(DependencySpec)
import flock.manifest.(Manifest, PackageInfo, parseManifest)

// ============================================================================
// RESOLVED PACKAGE
// ============================================================================

/// A fully resolved package with its manifest and location.
public struct ResolvedPackage: Cloneable {
    public var name: String
    public var version: Version
    public var rootDir: String
    public var manifest: Manifest

    public init(name name: String, version version: Version, rootDir rootDir: String, manifest manifest: Manifest) {
        self.name = name;
        self.version = version;
        self.rootDir = rootDir;
        self.manifest = manifest;
    }

    public func clone() -> ResolvedPackage {
        ResolvedPackage(name: self.name.clone(), version: self.version.clone(), rootDir: self.rootDir.clone(), manifest: self.manifest.clone())
    }
}

// ============================================================================
// PACKAGE SOURCE PROTOCOL
// ============================================================================

/// Abstraction for resolving package dependencies from different sources.
///
/// Implementations:
/// - PathSource: resolves local path dependencies
/// - (future) RegistrySource: resolves from a package registry
public protocol PackageSource {
    func resolve(name name: String, spec spec: DependencySpec, baseDir baseDir: String) -> Result[ResolvedPackage, FlockError]
}

// ============================================================================
// PATH SOURCE
// ============================================================================

/// Resolves dependencies from local filesystem paths.
public struct PathSource: PackageSource {
    public init() {}

    public func resolve(name name: String, spec spec: DependencySpec, baseDir baseDir: String) -> Result[ResolvedPackage, FlockError] {
        match spec {
            .Path(relPath) => {
                let pkgDir = joinPath(base: baseDir, rel: relPath);
                let manifestPath = joinPath(base: pkgDir, rel: "flock.toml");

                if not fileExists(manifestPath) {
                    return .Err(FlockError.ManifestNotFound(manifestPath))
                }

                match readFileString(manifestPath) {
                    .Err(e) => {
                        var msg = String(); msg.append("cannot read "); msg.append(manifestPath);
                        return .Err(FlockError.IoError(msg))
                    },
                    .Ok(source) => {
                        match parseManifest(source: source) {
                            .Err(e) => return .Err(e),
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
            },
            .Registry(_) => {
                var msg = String(); msg.append(name); msg.append(" (registry dependencies not yet supported)");
                .Err(FlockError.DependencyNotFound(msg))
            }
        }
    }
}

// ============================================================================
// PATH UTILITIES
// ============================================================================

/// Joins a base path with a relative path.
/// Handles trailing slashes and normalizes ".." segments.
public func joinPath(base base: String, rel rel: String) -> String {
    if rel.byteCount == 0 {
        return base
    }

    // If rel is absolute, return it directly
    if rel.starts(with: "/") {
        return rel
    }

    // Strip trailing slash from base
    var cleanBase = base;
    if cleanBase.byteCount > 1 and cleanBase.ends(with: "/") {
        cleanBase = cleanBase.asSlice().subslice(from: 0, to: cleanBase.byteCount - 1).toOwned()
    }

    // Split relative path and process ".." segments
    var parts = splitOnSlash(cleanBase);
    let relParts = splitOnSlash(rel);

    var i: Int64 = 0;
    while i < relParts.count {
        let part = relParts(unchecked: i);
        if part == ".." {
            if parts.count > 0 {
                let _ = parts.pop();
            }
        } else if part == "." {
            // skip current dir
        } else if part.byteCount > 0 {
            parts.append(part)
        }
        i = i + 1
    }

    // Rebuild path
    if parts.count == 0 {
        return "/"
    }

    var result = String();
    i = 0;
    while i < parts.count {
        if i > 0 or base.starts(with: "/") {
            result.append("/")
        }
        result.append(parts(unchecked: i));
        i = i + 1
    }

    result
}

/// Splits a path on "/" characters.
func splitOnSlash(s: String) -> Array[String] {
    var result = Array[String]();
    var start: Int64 = 0;
    var i: Int64 = 0;
    let len = s.byteCount;

    while i < len {
        let byte = s.bytes(unchecked: i);
        if byte == 47 { // '/'
            if i > start {
                result.append(s.asSlice().subslice(from: start, to: i).toOwned())
            }
            start = i + 1
        }
        i = i + 1
    }

    if start < len {
        result.append(s.asSlice().subslice(from: start, to: len).toOwned())
    }

    result
}
