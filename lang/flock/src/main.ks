// Flock - Package manager for Kestrel
//
// Usage:
//   flock build    Build the current package
//   flock run      Build and run the current package
//   flock check    Type-check the current package
//   flock init     Create a new flock.toml

module flock.main

import clutch.os.(getArgv, getcwd, fileExists, isDirectory, spawn, captureOutput)
import clutch.command.(Command)
import clutch.arg.(Arg)
import clutch.matches.(ArgMatches)
import clutch.error.(ParseError)
import flock.error.(FlockError)
import flock.manifest.(Manifest, BuildConfig, parseManifest)
import flock.source.(ResolvedPackage, PathSource, joinPath)
import flock.graph.(DepNode, buildGraph, topologicalSort)
import flock.discover.(discoverSources)
import flock.compiler.(invokeCompiler)
import flock.version.(Version)

// ============================================================================
// ENTRY POINT
// ============================================================================

func main() {
    let argv = getArgv();

    // Set up CLI
    var cmd = Command(name: "flock");
    cmd.setAbout(text: "Package manager for Kestrel");
    cmd.setVersion(ver: "0.1.0");

    var buildCmd = Command(name: "build");
    buildCmd.setAbout(text: "Build the current package");
    cmd.addSubcommand(sub: buildCmd);

    var runCmd = Command(name: "run");
    runCmd.setAbout(text: "Build and run the current package");
    cmd.addSubcommand(sub: runCmd);

    var checkCmd = Command(name: "check");
    checkCmd.setAbout(text: "Type-check the current package");
    cmd.addSubcommand(sub: checkCmd);

    var initCmd = Command(name: "init");
    initCmd.setAbout(text: "Create a new flock.toml in the current directory");
    cmd.addSubcommand(sub: initCmd);

    match cmd.parse(tokens: argv) {
        .Ok(matches) => {
            match matches.subcommand {
                .Some(sub) => {
                    if sub.equals("build") {
                        handleBuild()
                    } else if sub.equals("run") {
                        handleRun()
                    } else if sub.equals("check") {
                        handleCheck()
                    } else if sub.equals("init") {
                        handleInit()
                    }
                },
                .None => {
                    // No subcommand — show help
                    let _ = println(cmd.helpText());
                }
            }
        },
        .Err(e) => {
            // ParseError.Message is used for --help output
            let _ = eprintln(e.description());
        }
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

func handleBuild() {
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); },
        .Ok(info) => {
            let _ = println("Building " + info.name + "...");
            match invokeCompiler(mode: "build", sources: info.sources, output: .Some(info.name), linkLibs: info.linkLibs, linkPaths: info.linkPaths, frameworks: info.frameworks) {
                .Ok(_) => { let _ = println("Built " + info.name + " successfully"); },
                .Err(e) => { let _ = eprintln(e.description()); }
            }
        }
    }
}

func handleRun() {
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); },
        .Ok(info) => {
            match invokeCompiler(mode: "run", sources: info.sources, output: .None, linkLibs: info.linkLibs, linkPaths: info.linkPaths, frameworks: info.frameworks) {
                .Ok(_) => {},
                .Err(e) => { let _ = eprintln(e.description()); }
            }
        }
    }
}

func handleCheck() {
    match resolveAndDiscover() {
        .Err(e) => { let _ = eprintln(e.description()); },
        .Ok(info) => {
            let _ = println("Checking " + info.name + "...");
            match invokeCompiler(mode: "check", sources: info.sources, output: .None, linkLibs: Array[String](), linkPaths: Array[String](), frameworks: Array[String]()) {
                .Ok(_) => { let _ = println("Check passed"); },
                .Err(e) => { let _ = eprintln(e.description()); }
            }
        }
    }
}

func handleInit() {
    let cwd = getcwd();
    let manifestPath = joinPath(base: cwd, rel: "flock.toml");

    if fileExists(path: manifestPath) {
        let _ = eprintln("flock.toml already exists in this directory");
        return
    }

    // Extract directory name as default package name
    let dirName = lastPathComponent(cwd);

    let content = "[package]\nname = \"" + dirName + "\"\nversion = \"0.1.0\"\ndescription = \"\"\n\n[dependencies]\n";

    match writeFileString(manifestPath, content) {
        .Ok(_) => { let _ = println("Created flock.toml"); },
        .Err(e) => {
            let _ = eprintln("Failed to create flock.toml");
            return
        }
    }

    // Create src/ directory
    let srcDir = joinPath(base: cwd, rel: "src");
    if not isDirectory(path: srcDir) {
        let _ = spawn(command: "mkdir -p " + srcDir);
        let _ = println("Created src/");
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

    if not fileExists(path: manifestPath) {
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
        .Err(e) => return .Err(FlockError.IoError("cannot read " + manifestPath)),
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

    // Build dependency graph
    let source = PathSource();
    var nodes = Array[DepNode]();
    match buildGraph(root: root, source: source) {
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
                let output = captureOutput(command: cmd);
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
            let oPath = cPath + ".o";

            // Build cc command: cc -c <cFlags> <source> -o <output>
            var ccCmd = "cc -c";
            var k: Int64 = 0;
            while k < cFlags.count {
                ccCmd = ccCmd + " " + cFlags(unchecked: k);
                k = k + 1
            }
            ccCmd = ccCmd + " " + quoteArg(cPath) + " -o " + quoteArg(oPath);

            let exitCode = spawn(command: ccCmd);
            if exitCode != 0 {
                return .Err(FlockError.CompilerFailed(exitCode))
            }

            // Add the object file as a link library (: prefix for literal path)
            allLinkLibs.append(":" + oPath);
            j = j + 1
        }

        // Resolve dynamic link flags if link-cmd is set
        match build.linkCmd {
            .Some(cmd) => {
                let output = captureOutput(command: cmd);
                let flags = splitWhitespace(output);
                j = 0;
                while j < flags.count {
                    let flag = flags(unchecked: j);
                    // Parse -l, -L, and -framework flags from command output
                    if flag.starts(with: "-l") {
                        allLinkLibs.append(flag.substringBytes(from: 2, to: flag.byteCount))
                    } else if flag.starts(with: "-L") {
                        allLinkPaths.append(flag.substringBytes(from: 2, to: flag.byteCount))
                    } else if flag.starts(with: "-framework") {
                        // -framework is usually followed by the name as next arg
                        // but sometimes it's -framework<Name>
                    }
                    j = j + 1;
                    // Handle "-framework Name" as two separate tokens
                    if flag.equals("-framework") and j < flags.count {
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
        let b = s.byteAtUnchecked(i);
        let isSpace = b == UInt8(intLiteral: 32) or b == UInt8(intLiteral: 9) or b == UInt8(intLiteral: 10) or b == UInt8(intLiteral: 13);
        if isSpace {
            if start >= 0 {
                result.append(s.substringBytes(from: start, to: i));
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
        result.append(s.substringBytes(from: start, to: len))
    }

    result
}

/// Quotes a shell argument if it contains spaces.
func quoteArg(s: String) -> String {
    var i: Int64 = 0;
    while i < s.byteCount {
        if s.byteAtUnchecked(i) == UInt8(intLiteral: 32) {
            return "\"" + s + "\""
        }
        i = i + 1
    }
    s
}

/// Extracts the last component of a path.
func lastPathComponent(path: String) -> String {
    let len = path.byteCount;
    // Skip trailing slash
    var end = len;
    if end > 0 and path.byteAtUnchecked(end - 1) == UInt8(intLiteral: 47) {
        end = end - 1
    }

    // Find last slash
    var i = end - 1;
    while i >= 0 {
        if path.byteAtUnchecked(i) == UInt8(intLiteral: 47) { // '/'
            return path.substringBytes(from: i + 1, to: end)
        }
        i = i - 1
    }

    path.substringBytes(from: 0, to: end)
}
