// Flock - Package manager for Kestrel
//
// Usage:
//   flock build    Build the current package
//   flock run      Build and run the current package
//   flock check    Type-check the current package
//   flock init     Create a new flock.toml

module flock.main

import clutch.os.(getArgv)
import clutch.command.(Command)
import clutch.argument.(Argument)
import clutch.matches.(ArgumentMatches)
import clutch.error.(ParseError)
import flock.error.(FlockError)
import flock.manifest.(Manifest, BuildConfig, parseManifest)
import flock.source.(ResolvedPackage, PathSource, joinPath)
import flock.graph.(DepNode, buildGraph, topologicalSort)
import flock.discover.(discoverSources)
import flock.compiler.(invokeCompiler)
import flock.version.(Version)
import flock.registry.(RegistryConfig, resolveRegistryUrl, isRegistryName)
import flock.registry_source.(RegistrySource)
import flock.lock.(LockFile, LockEntry, parseLockFile, generateLockFile)

// ============================================================================
// ENTRY POINT
// ============================================================================

@main
func main() -> lang.i32 {
    let argv = getArgv();

    var cmd = Command("flock");
    cmd = cmd.about("Package manager for Kestrel");
    cmd = cmd.version("0.1.0");
    cmd = cmd.subcommand(Command("build").about("Build the current package")
        .argument(flag: "release", about: "Build optimized (passes -O 2 to the compiler)"));
    cmd = cmd.subcommand(Command("run").about("Build and run the current package")
        .argument(flag: "release", about: "Build optimized (passes -O 2 to the compiler)"));
    cmd = cmd.subcommand(Command("check").about("Type-check the current package"));
    cmd = cmd.subcommand(Command("init").about("Create a new flock.toml in the current directory"));
    cmd = cmd.subcommand(Command("publish").about("Publish a package to the registry"));
    cmd = cmd.subcommand(Command("update").about("Update dependencies (re-resolve and rewrite flock.lock)"));

    match cmd.parse(from: argv) {
        .Ok(matches) => {
            match matches.subcommand {
                .Some(sub) => {
                    // The subcommand's own flags live in submatches[0]; `--release`
                    // turns on optimized codegen for build/run.
                    var release = false;
                    if matches.submatches.count > 0 {
                        release = matches.submatches(unchecked: 0).hasFlag("release")
                    }
                    if sub == "build" {
                        handleBuild(release: release)
                    } else if sub == "run" {
                        handleRun(release: release)
                    } else if sub == "check" {
                        handleCheck()
                    } else if sub == "init" {
                        handleInit()
                    } else if sub == "publish" {
                        handlePublish()
                    } else if sub == "update" {
                        handleUpdate()
                    } else {
                        0
                    }
                },
                .None => {
                    // No subcommand — show help
                    let _ = println(cmd.helpText());
                    0
                }
            }
        },
        .Err(e) => {
            // ParseError.Message carries --help/--version text (success); every
            // other variant is a real usage error and must exit non-zero.
            let _ = eprintln(e.description());
            match e {
                .Message(_) => 0,
                _ => 1
            }
        }
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

func handleBuild(release release: Bool) -> lang.i32 {
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); 1 },
        .Ok(info) => {
            var msg = String(); msg.append("Building "); msg.append(info.name);
            if release { msg.append(" (release)") };
            msg.append("...");
            let _ = println(msg);
            match invokeCompiler(mode: "build", sources: info.sources, output: .Some(info.name), linkLibs: info.linkLibs, linkPaths: info.linkPaths, frameworks: info.frameworks, release: release) {
                .Ok(_) => {
                    var doneMsg = String(); doneMsg.append("Built "); doneMsg.append(info.name); doneMsg.append(" successfully");
                    let _ = println(doneMsg);
                    0
                },
                .Err(e) => { let _ = eprintln(e.description()); 1 }
            }
        }
    }
}

func handleRun(release release: Bool) -> lang.i32 {
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); 1 },
        .Ok(info) => {
            match invokeCompiler(mode: "run", sources: info.sources, output: .None, linkLibs: info.linkLibs, linkPaths: info.linkPaths, frameworks: info.frameworks, release: release) {
                .Ok(_) => 0,
                .Err(e) => { let _ = eprintln(e.description()); 1 }
            }
        }
    }
}

func handleCheck() -> lang.i32 {
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); 1 },
        .Ok(info) => {
            var msg = String(); msg.append("Checking "); msg.append(info.name); msg.append("...");
            let _ = println(msg);
            match invokeCompiler(mode: "check", sources: info.sources, output: .None, linkLibs: Array[String](), linkPaths: Array[String](), frameworks: Array[String](), release: false) {
                .Ok(_) => { let _ = println("Check passed"); 0 },
                .Err(e) => { let _ = eprintln(e.description()); 1 }
            }
        }
    }
}

func handleInit() -> lang.i32 {
    let cwd = getcwd();
    let manifestPath = joinPath(base: cwd, rel: "flock.toml");

    if fileExists( manifestPath) {
        let _ = eprintln("flock.toml already exists in this directory");
        return 1
    }

    // Extract directory name as default package name
    let dirName = lastPathComponent(cwd);

    var content = String();
    content.append("[package]\nname = \""); content.append(dirName); content.append("\"\nversion = \"0.1.0\"\norg = \"\"\ndescription = \"\"\nauthor = \"\"\nlicense = \"\"\nrepository = \"\"\nwebsite = \"\"\ndocumentation = \"\"\n\n[dependencies]\n");

    match writeFileString(manifestPath, content) {
        .Ok(_) => { let _ = println("Created flock.toml"); },
        .Err(e) => {
            let _ = eprintln("Failed to create flock.toml");
            return 1
        }
    }

    // Create src/ directory
    let srcDir = joinPath(base: cwd, rel: "src");
    if not isDirectory( srcDir) {
        var mkdirCmd = String(); mkdirCmd.append("mkdir -p "); mkdirCmd.append(srcDir);
        let _ = spawn(mkdirCmd);
        let _ = println("Created src/");
    }
    0
}

func handlePublish() -> lang.i32 {
    let cwd = getcwd();
    let manifestPath = joinPath(base: cwd, rel: "flock.toml");

    if not fileExists(manifestPath) {
        let _ = eprintln("flock.toml not found in current directory");
        return 1
    }

    // Parse manifest
    var manifest: Manifest = Manifest(
        package: flock.manifest.PackageInfo(
            name: "",
            version: Version(major: 0, minor: 0, patch: 0),
            description: .None,
            source: "src"
        ),
        dependencies: Array[flock.dependency.Dependency]()
    );
    match readFileString(manifestPath) {
        .Err(_) => {
            let _ = eprintln("cannot read flock.toml");
            return 1
        },
        .Ok(source) => {
            match parseManifest(source: source) {
                .Err(e) => {
                    let _ = eprintln(e.description());
                    return 1
                },
                .Ok(m) => manifest = m
            }
        }
    }

    let name = manifest.package.name;
    let version = manifest.package.version.toString();

    // Resolve org. The package's own [package] org in flock.toml is the default
    // (version-controlled identity); FLOCK_ORG overrides it for forks / CI / one-offs.
    var org = "";
    let manifestOrg = manifest.package.org;
    match manifestOrg {
        .Some(o) => org = o,
        .None => {}
    }
    match getenv("FLOCK_ORG") {
        .Some(o) => org = o,
        .None => {}
    }
    if org.byteCount == 0 {
        let _ = eprintln("No org specified. Add `org = \"myorg\"` under [package] in flock.toml, or set FLOCK_ORG.");
        return 1
    }

    // Read token from ~/.kestrel/credentials
    var token = "";
    match getenv("HOME") {
        .Some(home) => {
            let credPath = joinPath(base: home, rel: ".kestrel/credentials");
            match readFileString(credPath) {
                .Ok(contents) => token = trimWhitespace(contents),
                .Err(_) => {}
            }
        },
        .None => {}
    }

    // Fall back to FLOCK_TOKEN env var
    if token.byteCount == 0 {
        match getenv("FLOCK_TOKEN") {
            .Some(t) => token = t,
            .None => {
                let _ = eprintln("No auth token found.");
                let _ = eprintln("Set FLOCK_TOKEN or save your token to ~/.kestrel/credentials");
                return 1
            }
        }
    }

    // Resolve registry URL
    let regUrl = resolveRegistryUrl(projectUrl: manifest.registryUrl);

    // Create archive
    var archivePath = String(); archivePath.append("/tmp/flock-publish-"); archivePath.append(name); archivePath.append("-"); archivePath.append(version); archivePath.append(".tar.gz");
    var tarCmd = String(); tarCmd.append("tar czf "); tarCmd.append(archivePath); tarCmd.append(" -C "); tarCmd.append(quoteArg(cwd)); tarCmd.append(" .");
    let tarExit = spawn(tarCmd);
    if tarExit != 0 {
        let _ = eprintln("failed to create archive");
        return 1
    }

    // Generate docs (best-effort — publish continues without docs)
    var docsDir = String(); docsDir.append("/tmp/flock-docs-"); docsDir.append(name); docsDir.append("-"); docsDir.append(version);
    let sourceDir = joinPath(base: cwd, rel: manifest.package.source);
    var docCmd = String(); docCmd.append("kestrel-doc --src "); docCmd.append(quoteArg(sourceDir)); docCmd.append(" --out "); docCmd.append(quoteArg(docsDir)); docCmd.append(" --bundle --format json");
    let docExit = spawn(docCmd);
    var hasDocs = false;
    if docExit == 0 {
        var docsPath = String(); docsPath.append(docsDir); docsPath.append("/docs.json");
        if fileExists(docsPath) {
            hasDocs = true
        }
    } else {
        let _ = eprintln("Note: docs not generated (kestrel-doc not available or source has errors)");
    }

    // Upload archive via curl
    var url = String(); url.append(regUrl); url.append("/api/v1/packages/"); url.append(org); url.append("/"); url.append(name); url.append("/"); url.append(version);
    var curlCmd = String(); curlCmd.append("curl -s -X PUT "); curlCmd.append(quoteArg(url)); curlCmd.append(" -H \"Authorization: Bearer "); curlCmd.append(token); curlCmd.append("\" -H \"Content-Type: application/gzip\" --data-binary @"); curlCmd.append(archivePath);
    var pubMsg = String(); pubMsg.append("Publishing "); pubMsg.append(org); pubMsg.append("/"); pubMsg.append(name); pubMsg.append("@"); pubMsg.append(version); pubMsg.append(" to "); pubMsg.append(regUrl); pubMsg.append("...");
    let _ = println(pubMsg);

    let output = captureOutput(curlCmd);
    let _ = println(output);

    // Upload docs if generated
    if hasDocs {
        var docsPath = String(); docsPath.append(docsDir); docsPath.append("/docs.json");
        var docsUrl = String(); docsUrl.append(regUrl); docsUrl.append("/api/v1/packages/"); docsUrl.append(org); docsUrl.append("/"); docsUrl.append(name); docsUrl.append("/"); docsUrl.append(version); docsUrl.append("/docs");
        var docsCurlCmd = String(); docsCurlCmd.append("curl -s -X PUT "); docsCurlCmd.append(quoteArg(docsUrl)); docsCurlCmd.append(" -H \"Authorization: Bearer "); docsCurlCmd.append(token); docsCurlCmd.append("\" -H \"Content-Type: application/json\" --data-binary @"); docsCurlCmd.append(docsPath);
        let _ = println("Uploading documentation...");
        let docsOutput = captureOutput(docsCurlCmd);
        let _ = println(docsOutput);
    }

    // Clean up
    var rmCmd = String(); rmCmd.append("rm -f "); rmCmd.append(archivePath);
    let _ = spawn(rmCmd);
    var rmDocsCmd = String(); rmDocsCmd.append("rm -rf "); rmDocsCmd.append(docsDir);
    let _ = spawn(rmDocsCmd);
    0
}

func handleUpdate() -> lang.i32 {
    let cwd = getcwd();
    let lockPath = joinPath(base: cwd, rel: "flock.lock");

    // Delete existing lock file to force re-resolution
    if fileExists(lockPath) {
        var rmCmd = String(); rmCmd.append("rm "); rmCmd.append(lockPath);
        let _ = spawn(rmCmd);
        let _ = println("Removed flock.lock");
    }

    // Re-resolve everything
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); 1 },
        .Ok(info) => {
            var msg = String(); msg.append("Dependencies updated for "); msg.append(info.name);
            let _ = println(msg);
            0
        }
    }
}

// ============================================================================
// BUILD INFO
// ============================================================================

/// Collected information for a build/run/check operation.
struct BuildInfo: Cloneable {
    var name: String
    var sources: Array[String]
    var linkLibs: Array[String]
    var linkPaths: Array[String]
    var frameworks: Array[String]

    init(name name: String, sources sources: Array[String], linkLibs linkLibs: Array[String], linkPaths linkPaths: Array[String], frameworks frameworks: Array[String]) {
        self.name = name;
        self.sources = sources;
        self.linkLibs = linkLibs;
        self.linkPaths = linkPaths;
        self.frameworks = frameworks;
    }

    func clone() -> BuildInfo {
        BuildInfo(name: self.name.clone(), sources: self.sources.clone(), linkLibs: self.linkLibs.clone(), linkPaths: self.linkPaths.clone(), frameworks: self.frameworks.clone())
    }
}

// ============================================================================
// CORE LOGIC
// ============================================================================

/// Reads the manifest, resolves all dependencies, discovers sources,
/// and returns them in build order.
func resolveAndDiscover() -> Result[BuildInfo, FlockError] {
    let cwd = getcwd();
    let manifestPath = joinPath(base: cwd, rel: "flock.toml");

    if not fileExists( manifestPath) {
        return .Err(FlockError.ManifestNotFound(manifestPath))
    }

    // Read and parse manifest
    var manifest: Manifest = Manifest(
        package: flock.manifest.PackageInfo(
            name: "",
            version: Version(major: 0, minor: 0, patch: 0),
            description: .None,
            source: "src"
        ),
        dependencies: Array[flock.dependency.Dependency]()
    );

    match readFileString(manifestPath) {
        .Err(e) => {
            var msg = String(); msg.append("cannot read "); msg.append(manifestPath);
            return .Err(FlockError.IoError(msg))
        },
        .Ok(source) => {
            match parseManifest(source: source) {
                .Err(e) => return .Err(e),
                .Ok(m) => manifest = m
            }
        }
    }

    // Create root package
    let root = ResolvedPackage(
        name: manifest.package.name,
        version: manifest.package.version,
        rootDir: cwd,
        manifest: manifest
    );

    // Build dependency graph with both path and registry sources
    let pathSrc = PathSource();
    let regUrl = resolveRegistryUrl(projectUrl: manifest.registryUrl);
    let regConfig = RegistryConfig(url: regUrl);
    let regSrc = RegistrySource(config: regConfig);

    var nodes = Array[DepNode]();
    match buildGraph(root: root, pathSource: pathSrc, registrySource: regSrc) {
        .Err(e) => return .Err(e),
        .Ok(n) => nodes = n
    }

    // Topological sort
    var sorted = Array[DepNode]();
    match topologicalSort(nodes: nodes) {
        .Err(e) => return .Err(e),
        .Ok(s) => sorted = s
    }

    // Discover sources, compile C, and collect link flags in dependency order
    var allSources = Array[String]();
    var allLinkLibs = Array[String]();
    var allLinkPaths = Array[String]();
    var allFrameworks = Array[String]();

    var i: Int64 = 0;
    while i < sorted.count {
        let node = sorted(unchecked: i);
        let build = node.build;

        // Discover .ks sources
        let srcDir = joinPath(base: node.rootDir, rel: node.sourceDir);
        let sources = discoverSources(rootDir: srcDir);
        var j: Int64 = 0;
        while j < sources.count {
            allSources.append(sources(unchecked: j));
            j = j + 1
        }

        // Resolve dynamic C flags if c-flags-cmd is set
        var cFlags = Array[String]();
        j = 0;
        while j < build.cFlags.count {
            cFlags.append(build.cFlags(unchecked: j));
            j = j + 1
        }
        match build.cFlagsCmd {
            .Some(cmd) => {
                let output = captureOutput( cmd);
                let extra = splitWhitespace(output);
                j = 0;
                while j < extra.count {
                    cFlags.append(extra(unchecked: j));
                    j = j + 1
                }
            },
            .None => {}
        }

        // Compile C sources
        j = 0;
        while j < build.cSources.count {
            let cSource = build.cSources(unchecked: j);
            let cPath = joinPath(base: node.rootDir, rel: cSource);
            var oPath = String(); oPath.append(cPath); oPath.append(".o");

            // Build cc command: cc -c <cFlags> <source> -o <output>
            var ccCmd = String();
            ccCmd.append("cc -c");
            var k: Int64 = 0;
            while k < cFlags.count {
                ccCmd.append(" "); ccCmd.append(cFlags(unchecked: k));
                k = k + 1
            }
            ccCmd.append(" "); ccCmd.append(quoteArg(cPath)); ccCmd.append(" -o "); ccCmd.append(quoteArg(oPath));

            let exitCode = spawn( ccCmd);
            if exitCode != 0 {
                return .Err(FlockError.CompilerFailed(exitCode))
            }

            // Add the object file as a link library (: prefix for literal path)
            var libPath = String(); libPath.append(":"); libPath.append(oPath);
            allLinkLibs.append(libPath);
            j = j + 1
        }

        // Resolve dynamic link flags if link-cmd is set
        match build.linkCmd {
            .Some(cmd) => {
                let output = captureOutput( cmd);
                let flags = splitWhitespace(output);
                j = 0;
                while j < flags.count {
                    let flag = flags(unchecked: j);
                    // Parse -l, -L, and -framework flags from command output
                    if flag.starts(with: "-l") {
                        allLinkLibs.append(flag.asSlice().subslice(from: 2, to: flag.byteCount).toOwned())
                    } else if flag.starts(with: "-L") {
                        allLinkPaths.append(flag.asSlice().subslice(from: 2, to: flag.byteCount).toOwned())
                    } else if flag.starts(with: "-framework") {
                        // -framework is usually followed by the name as next arg
                        // but sometimes it's -framework<Name>
                    }
                    j = j + 1;
                    // Handle "-framework Name" as two separate tokens
                    if flag == "-framework" and j < flags.count {
                        allFrameworks.append(flags(unchecked: j));
                        j = j + 1
                    }
                }
            },
            .None => {}
        }

        // Collect static link flags
        j = 0;
        while j < build.link.count {
            allLinkLibs.append(build.link(unchecked: j));
            j = j + 1
        }
        j = 0;
        while j < build.linkPaths.count {
            allLinkPaths.append(build.linkPaths(unchecked: j));
            j = j + 1
        }
        j = 0;
        while j < build.frameworks.count {
            allFrameworks.append(build.frameworks(unchecked: j));
            j = j + 1
        }

        i = i + 1
    }

    // Generate lock file from resolved dependencies
    var lockEntries = Array[LockEntry]();
    i = 0;
    while i < sorted.count {
        let node = sorted(unchecked: i);
        // Skip the root package itself
        if node.name != manifest.package.name {
            let isRegistry = isRegistryDep(name: node.name);
            let src = if isRegistry { "registry" } else { "path" };
            var entryPath: Optional[String] = .None;
            if not isRegistry {
                entryPath = .Some(node.rootDir)
            }
            let entry = LockEntry(
                name: node.name,
                version: Version(major: 0, minor: 0, patch: 0),
                source: src,
                checksum: .None,
                path: entryPath
            );
            lockEntries.append(entry)
        }
        i = i + 1
    }

    let lockContent = generateLockFile(entries: lockEntries);
    let lockPath = joinPath(base: cwd, rel: "flock.lock");
    match writeFileString(lockPath, lockContent) {
        .Ok(_) => {},
        .Err(_) => {}
    }

    .Ok(BuildInfo(name: manifest.package.name, sources: allSources, linkLibs: allLinkLibs, linkPaths: allLinkPaths, frameworks: allFrameworks))
}

// ============================================================================
// HELPERS
// ============================================================================

/// Splits a string on whitespace into individual tokens.
func splitWhitespace(s: String) -> Array[String] {
    var result = Array[String]();
    var start: Int64 = -1;
    var i: Int64 = 0;
    let len = s.byteCount;

    while i < len {
        let b = s.bytes(unchecked: i);
        let isSpace = b == 32 or b == 9 or b == 10 or b == 13;
        if isSpace {
            if start >= 0 {
                result.append(s.asSlice().subslice(from: start, to: i).toOwned());
                start = -1
            }
        } else {
            if start < 0 {
                start = i
            }
        }
        i = i + 1
    }

    if start >= 0 {
        result.append(s.asSlice().subslice(from: start, to: len).toOwned())
    }

    result
}

/// Quotes a shell argument if it contains spaces.
func quoteArg(s: String) -> String {
    var i: Int64 = 0;
    while i < s.byteCount {
        if s.bytes(unchecked: i) == 32 {
            var q = String(); q.append("\""); q.append(s); q.append("\"");
            return q
        }
        i = i + 1
    }
    s
}

/// Checks if a dependency name looks like a registry dep (contains a slash).
func isRegistryDep(name name: String) -> Bool {
    isRegistryName(name: name)
}

/// Extracts the last component of a path.
func lastPathComponent(path: String) -> String {
    let len = path.byteCount;
    // Skip trailing slash
    var end = len;
    if end > 0 and path.bytes(unchecked: end - 1) == 47 {
        end = end - 1
    }

    // Find last slash
    var i = end - 1;
    while i >= 0 {
        if path.bytes(unchecked: i) == 47 { // '/'
            return path.asSlice().subslice(from: i + 1, to: end).toOwned()
        }
        i = i - 1
    }

    path.asSlice().subslice(from: 0, to: end).toOwned()
}

/// Trims leading and trailing whitespace (spaces, tabs, newlines) from a string.
func trimWhitespace(s: String) -> String {
    let len = s.byteCount;
    var start: Int64 = 0;
    while start < len {
        let b = s.bytes(unchecked: start);
        if b == 32 or b == 9 or b == 10 or b == 13 {
            start = start + 1
        } else {
            break
        }
    }
    var end = len;
    while end > start {
        let b = s.bytes(unchecked: end - 1);
        if b == 32 or b == 9 or b == 10 or b == 13 {
            end = end - 1
        } else {
            break
        }
    }
    s.asSlice().subslice(from: start, to: end).toOwned()
}
