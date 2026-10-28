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
    public var description: String?
    public var author: String?
    public var license: String?
    public var repository: String?
    public var website: String?
    public var documentation: String?
    /// Organization / namespace this package publishes under, forming the
    /// `org/name` scope. Used by `flock publish`; `FLOCK_ORG` overrides it.
    public var org: String?
    /// Source directory relative to the package root. Defaults to "src".
    public var source: String

    public init(name name: String, version version: Version, description description: String?, source source: String) {
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
        var info = PackageInfo(name: self.name.clone(), version: self.version.clone(), description: self.description.clone(), source: self.source.clone());
        info.author = self.author.clone();
        info.license = self.license.clone();
        info.repository = self.repository.clone();
        info.website = self.website.clone();
        info.documentation = self.documentation.clone();
        info.org = self.org.clone();
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
    public var cFlagsCmd: String?
    /// Library names to link (become -l flags).
    public var link: Array[String]
    /// Shell command whose stdout provides additional link flags.
    public var linkCmd: String?
    /// Library search paths (become -L flags).
    public var linkPaths: Array[String]
    /// macOS frameworks (become --framework flags).
    public var frameworks: Array[String]

    public init() {
        self.cSources = [];
        self.cFlags = [];
        self.cFlagsCmd = .None;
        self.link = [];
        self.linkCmd = .None;
        self.linkPaths = [];
        self.frameworks = [];
    }

    public func clone() -> BuildConfig {
        var cfg = BuildConfig();
        cfg.cSources = self.cSources.clone();
        cfg.cFlags = self.cFlags.clone();
        cfg.cFlagsCmd = self.cFlagsCmd.clone();
        cfg.link = self.link.clone();
        cfg.linkCmd = self.linkCmd.clone();
        cfg.linkPaths = self.linkPaths.clone();
        cfg.frameworks = self.frameworks.clone();
        cfg
    }
}

// ============================================================================
// BINARY TARGET DECLARATION
// ============================================================================

/// A `[[bin]]` entry from flock.toml. Overrides or adds a binary target on top
/// of the convention-discovered ones (src/main.ks, src/bin/*.ks). `path` is
/// relative to the package root.
public struct BinDecl: Cloneable {
    public var name: String
    public var path: String

    public init(name name: String, path path: String) {
        self.name = name;
        self.path = path;
    }

    public func clone() -> BinDecl {
        BinDecl(name: self.name.clone(), path: self.path.clone())
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
    public var registryUrl: String?
    /// `[[bin]]` declarations — binary targets that override or add to the
    /// convention-discovered ones. Empty for typical lib/single-bin packages.
    public var bins: Array[BinDecl]

    public init(package package: PackageInfo, dependencies dependencies: Array[Dependency]) {
        self.package = package;
        self.dependencies = dependencies;
        self.build = BuildConfig();
        self.registryUrl = .None;
        self.bins = [];
    }

    public init(package package: PackageInfo, dependencies dependencies: Array[Dependency], build build: BuildConfig, registryUrl registryUrl: String?) {
        self.package = package;
        self.dependencies = dependencies;
        self.build = build;
        self.registryUrl = registryUrl;
        self.bins = [];
    }

    public func clone() -> Manifest {
        var m = Manifest(package: self.package.clone(), dependencies: self.dependencies.clone(), build: self.build.clone(), registryUrl: self.registryUrl.clone());
        m.bins = self.bins.clone();
        m
    }
}

// ============================================================================
// PARSING
// ============================================================================

/// Parses a flock.toml source string into a Manifest.
public func parseManifest(source source: String) -> Manifest throws FlockError {
    let root = match parseToml(source) {
        .Ok(v) => v,
        .Err(e) => throw FlockError.ManifestParse(e.description())
    };

    guard let some pkg = root.value(for: "package") else {
        throw FlockError.ManifestParse("missing [package] section")
    }

    let name = try requireString(pkg, "name", path: "package.name");
    let versionText = try requireString(pkg, "version", path: "package.version");
    let version = try parseVersion(s: versionText);

    var packageInfo = PackageInfo(
        name: name,
        version: version,
        description: parseOptionalString(pkg, "description"),
        source: parseOptionalString(pkg, "source").unwrap(or: "src")
    );
    packageInfo.author = parseOptionalString(pkg, "author");
    packageInfo.license = parseOptionalString(pkg, "license");
    packageInfo.repository = parseOptionalString(pkg, "repository");
    packageInfo.website = parseOptionalString(pkg, "website");
    packageInfo.documentation = parseOptionalString(pkg, "documentation");
    packageInfo.org = parseOptionalString(pkg, "org");

    // [dependencies] — a manifest with none is still valid
    var deps: [Dependency] = [];
    if let some depsVal = root.value(for: "dependencies") {
        deps = try parseDependencies(depsValue: depsVal)
    }

    // [build]
    var buildCfg = BuildConfig();
    if let some buildVal = root.value(for: "build") {
        buildCfg.cSources = parseStringArray(buildVal, "c-sources");
        buildCfg.cFlags = parseStringArray(buildVal, "c-flags");
        buildCfg.cFlagsCmd = parseOptionalString(buildVal, "c-flags-cmd");
        buildCfg.link = parseStringArray(buildVal, "link");
        buildCfg.linkCmd = parseOptionalString(buildVal, "link-cmd");
        buildCfg.linkPaths = parseStringArray(buildVal, "link-paths");
        buildCfg.frameworks = parseStringArray(buildVal, "frameworks");
    }

    // [registry]
    var registryUrl: String? = .None;
    if let some regVal = root.value(for: "registry") {
        registryUrl = parseOptionalString(regVal, "url")
    }

    var manifest = Manifest(package: packageInfo, dependencies: deps, build: buildCfg, registryUrl: registryUrl);
    manifest.bins = parseBinDecls(root);
    .Ok(manifest)
}

// ============================================================================
// HELPERS
// ============================================================================

/// Reads a required string field, keeping "missing" and "wrong type" distinct
/// so the diagnostic names the actual problem.
func requireString(parent: Value, key: String, path path: String) -> String throws FlockError {
    guard let some raw = parent.value(for: key) else {
        throw FlockError.ManifestParse("missing " + path)
    }

    guard let some text = raw.asString() else {
        throw FlockError.ManifestParse(path + " must be a string")
    }

    .Ok(text)
}

/// Parses `[[bin]]` array-of-tables from the manifest root. Each table needs a
/// `name` and a `path`; entries missing either field are skipped (binary-target
/// validation happens later, in discovery).
func parseBinDecls(root: Value) -> [BinDecl] {
    var result: [BinDecl] = [];

    guard let some binVal = root.value(for: "bin") else { return result }
    guard let some entries = binVal.asArray() else { return result }

    for entry in entries {
        guard let some name = parseOptionalString(entry, "name") else { continue }
        guard let some path = parseOptionalString(entry, "path") else { continue }
        result.append(BinDecl(name: name, path: path))
    }

    result
}

/// Parses a string array field from a TOML value. Non-string entries are skipped.
func parseStringArray(parent: Value, key: String) -> [String] {
    var result: [String] = [];

    guard let some val = parent.value(for: key) else { return result }
    guard let some entries = val.asArray() else { return result }

    for entry in entries {
        if let some text = entry.asString() {
            result.append(text)
        }
    }

    result
}

/// Parses an optional string field from a TOML value.
func parseOptionalString(parent: Value, key: String) -> String? {
    guard let some val = parent.value(for: key) else { return .None }
    val.asString()
}
