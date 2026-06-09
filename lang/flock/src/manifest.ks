// Package manifest parsing (flock.toml)

module flock.manifest

import quill.value.(Value)
import quill.toml.parser.(parseToml)
import flock.error.(FlockError)
import flock.version.(Version, parseVersion)
import flock.dependency.(Dependency, parseDependencies)

// ============================================================================
// PACKAGE INFO
// ============================================================================

/// Metadata from the [package] section of flock.toml.
public struct PackageInfo: Cloneable {
    public var name: String
    public var version: Version
    public var description: Optional[String]
    public var author: Optional[String]
    public var license: Optional[String]
    public var repository: Optional[String]
    public var website: Optional[String]
    public var documentation: Optional[String]
    /// Organization / namespace this package publishes under, forming the
    /// `org/name` scope. Used by `flock publish`; `FLOCK_ORG` overrides it.
    public var org: Optional[String]
    /// Source directory relative to the package root. Defaults to "src".
    public var source: String

    public init(name name: String, version version: Version, description description: Optional[String], source source: String) {
        self.name = name;
        self.version = version;
        self.description = description;
        self.author = .None;
        self.license = .None;
        self.repository = .None;
        self.website = .None;
        self.documentation = .None;
        self.org = .None;
        self.source = source;
    }

    public func clone() -> PackageInfo {
        var info = PackageInfo(name: self.name.clone(), version: self.version.clone(), description: cloneOptionalString(self.description), source: self.source.clone());
        info.author = cloneOptionalString(self.author);
        info.license = cloneOptionalString(self.license);
        info.repository = cloneOptionalString(self.repository);
        info.website = cloneOptionalString(self.website);
        info.documentation = cloneOptionalString(self.documentation);
        info.org = cloneOptionalString(self.org);
        info
    }
}

// ============================================================================
// BUILD CONFIG
// ============================================================================

/// Build configuration from the [build] section of flock.toml.
public struct BuildConfig: Cloneable {
    /// C source files to compile (relative to package root).
    public var cSources: Array[String]
    /// Flags passed to cc when compiling C sources.
    public var cFlags: Array[String]
    /// Shell command whose stdout provides additional C flags.
    public var cFlagsCmd: Optional[String]
    /// Library names to link (become -l flags).
    public var link: Array[String]
    /// Shell command whose stdout provides additional link flags.
    public var linkCmd: Optional[String]
    /// Library search paths (become -L flags).
    public var linkPaths: Array[String]
    /// macOS frameworks (become --framework flags).
    public var frameworks: Array[String]

    public init() {
        self.cSources = Array[String]();
        self.cFlags = Array[String]();
        self.cFlagsCmd = .None;
        self.link = Array[String]();
        self.linkCmd = .None;
        self.linkPaths = Array[String]();
        self.frameworks = Array[String]();
    }

    public func clone() -> BuildConfig {
        var cfg = BuildConfig();
        cfg.cSources = self.cSources.clone();
        cfg.cFlags = self.cFlags.clone();
        cfg.cFlagsCmd = cloneOptionalString(self.cFlagsCmd);
        cfg.link = self.link.clone();
        cfg.linkCmd = cloneOptionalString(self.linkCmd);
        cfg.linkPaths = self.linkPaths.clone();
        cfg.frameworks = self.frameworks.clone();
        cfg
    }
}

// ============================================================================
// MANIFEST
// ============================================================================

/// A parsed flock.toml file.
public struct Manifest: Cloneable {
    public var package: PackageInfo
    public var dependencies: Array[Dependency]
    public var build: BuildConfig
    /// Optional registry URL override from [registry] section.
    public var registryUrl: Optional[String]

    public init(package package: PackageInfo, dependencies dependencies: Array[Dependency]) {
        self.package = package;
        self.dependencies = dependencies;
        self.build = BuildConfig();
        self.registryUrl = .None;
    }

    public init(package package: PackageInfo, dependencies dependencies: Array[Dependency], build build: BuildConfig, registryUrl registryUrl: Optional[String]) {
        self.package = package;
        self.dependencies = dependencies;
        self.build = build;
        self.registryUrl = registryUrl;
    }

    public func clone() -> Manifest {
        var m = Manifest(package: self.package.clone(), dependencies: self.dependencies.clone(), build: self.build.clone(), registryUrl: cloneOptionalString(self.registryUrl));
        m
    }
}

// ============================================================================
// PARSING
// ============================================================================

/// Parses a flock.toml source string into a Manifest.
public func parseManifest(source source: String) -> Result[Manifest, FlockError] {
    // Parse TOML
    let tomlResult = parseToml(source);
    var root: Value = Value.Null;
    match tomlResult {
        .Ok(v) => root = v,
        .Err(e) => return .Err(FlockError.ManifestParse(e.description()))
    }

    // Extract [package] section
    let pkgValue = root.value(forKey: "package");
    match pkgValue {
        .None => return .Err(FlockError.ManifestParse("missing [package] section")),
        .Some(pkg) => {
            // Extract name
            let nameOpt = pkg.value(forKey: "name");
            var name: String = "";
            match nameOpt {
                .Some(nameVal) => {
                    match nameVal.asString() {
                        .Some(s) => name = s,
                        .None => return .Err(FlockError.ManifestParse("package.name must be a string"))
                    }
                },
                .None => return .Err(FlockError.ManifestParse("missing package.name"))
            }

            // Extract version
            let versionOpt = pkg.value(forKey: "version");
            var version: Version = Version(major: 0, minor: 0, patch: 0);
            match versionOpt {
                .Some(verVal) => {
                    match verVal.asString() {
                        .Some(verStr) => {
                            match parseVersion(s: verStr) {
                                .Ok(v) => version = v,
                                .Err(e) => return .Err(e)
                            }
                        },
                        .None => return .Err(FlockError.ManifestParse("package.version must be a string"))
                    }
                },
                .None => return .Err(FlockError.ManifestParse("missing package.version"))
            }

            // Extract description (optional)
            var description: Optional[String] = .None;
            match pkg.value(forKey: "description") {
                .Some(descVal) => {
                    match descVal.asString() {
                        .Some(s) => description = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            // Extract source directory (optional, defaults to "src")
            var sourceDir = "src";
            match pkg.value(forKey: "source") {
                .Some(srcVal) => {
                    match srcVal.asString() {
                        .Some(s) => sourceDir = s,
                        .None => {}
                    }
                },
                .None => {}
            }

            // Extract author (optional)
            var author: Optional[String] = .None;
            match pkg.value(forKey: "author") {
                .Some(val) => {
                    match val.asString() {
                        .Some(s) => author = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            // Extract license (optional)
            var license: Optional[String] = .None;
            match pkg.value(forKey: "license") {
                .Some(val) => {
                    match val.asString() {
                        .Some(s) => license = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            // Extract repository (optional)
            var repository: Optional[String] = .None;
            match pkg.value(forKey: "repository") {
                .Some(val) => {
                    match val.asString() {
                        .Some(s) => repository = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            var packageInfo = PackageInfo(
                name: name,
                version: version,
                description: description,
                source: sourceDir
            );
            // Extract website (optional)
            var website: Optional[String] = .None;
            match pkg.value(forKey: "website") {
                .Some(val) => {
                    match val.asString() {
                        .Some(s) => website = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            // Extract documentation (optional)
            var docs: Optional[String] = .None;
            match pkg.value(forKey: "documentation") {
                .Some(val) => {
                    match val.asString() {
                        .Some(s) => docs = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            // Extract org (optional) — the publish namespace
            var org: Optional[String] = .None;
            match pkg.value(forKey: "org") {
                .Some(val) => {
                    match val.asString() {
                        .Some(s) => org = .Some(s),
                        .None => {}
                    }
                },
                .None => {}
            }

            packageInfo.author = author;
            packageInfo.license = license;
            packageInfo.repository = repository;
            packageInfo.website = website;
            packageInfo.documentation = docs;
            packageInfo.org = org;

            // Extract [dependencies] section
            var deps = Array[Dependency]();
            match root.value(forKey: "dependencies") {
                .Some(depsVal) => {
                    match parseDependencies(depsValue: depsVal) {
                        .Ok(d) => deps = d,
                        .Err(e) => return .Err(e)
                    }
                },
                .None => {} // No dependencies is fine
            }

            // Extract [build] section
            var buildCfg = BuildConfig();
            match root.value(forKey: "build") {
                .Some(buildVal) => {
                    buildCfg.cSources = parseStringArray(buildVal, "c-sources");
                    buildCfg.cFlags = parseStringArray(buildVal, "c-flags");
                    buildCfg.cFlagsCmd = parseOptionalString(buildVal, "c-flags-cmd");
                    buildCfg.link = parseStringArray(buildVal, "link");
                    buildCfg.linkCmd = parseOptionalString(buildVal, "link-cmd");
                    buildCfg.linkPaths = parseStringArray(buildVal, "link-paths");
                    buildCfg.frameworks = parseStringArray(buildVal, "frameworks");
                },
                .None => {}
            }

            // Extract [registry] section (optional)
            var registryUrl: Optional[String] = .None;
            match root.value(forKey: "registry") {
                .Some(regVal) => {
                    registryUrl = parseOptionalString(regVal, "url")
                },
                .None => {}
            }

            .Ok(Manifest(package: packageInfo, dependencies: deps, build: buildCfg, registryUrl: registryUrl))
        }
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Parses a string array field from a TOML value.
func parseStringArray(parent: Value, key: String) -> Array[String] {
    var result = Array[String]();
    match parent.value(forKey: key) {
        .Some(val) => {
            match val.asArray() {
                .Some(arr) => {
                    var i: Int64 = 0;
                    while i < arr.count {
                        match arr(unchecked: i).asString() {
                            .Some(s) => result.append(s),
                            .None => {}
                        }
                        i = i + 1
                    }
                },
                .None => {}
            }
        },
        .None => {}
    }
    result
}

/// Parses an optional string field from a TOML value.
func parseOptionalString(parent: Value, key: String) -> Optional[String] {
    match parent.value(forKey: key) {
        .Some(val) => val.asString(),
        .None => .None
    }
}

/// Clones an Optional[String].
func cloneOptionalString(opt: Optional[String]) -> Optional[String] {
    match opt {
        .Some(s) => .Some(s.clone()),
        .None => .None
    }
}
