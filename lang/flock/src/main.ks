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
import flock.source.(ResolvedPackage, PathSource, joinPath, flockHome)
import flock.graph.(DepNode, buildGraph, topologicalSort)
import flock.discover.(discoverSources, discoverBins, BinTarget)
import flock.compiler.(invokeCompiler)
import flock.version.(Version, parseVersion, VersionConstraint)
import flock.dependency.(DependencySpec)
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
    var buildCmd = Command("build").about("Build the current package");
    buildCmd = buildCmd.argument(flag: "release", about: "Build optimized (passes -O 2 to the compiler)");
    buildCmd = buildCmd.argument("bin", about: "Build only the named binary target");
    cmd = cmd.subcommand(buildCmd);
    var runCmd = Command("run").about("Build and run the current package");
    runCmd = runCmd.argument(flag: "release", about: "Build optimized (passes -O 2 to the compiler)");
    runCmd = runCmd.argument("bin", about: "Run only the named binary target");
    cmd = cmd.subcommand(runCmd);
    cmd = cmd.subcommand(Command("check").about("Type-check the current package"));
    var initCmd = Command("init").about("Create a new flock.toml in the current directory");
    initCmd = initCmd.argument(flag: "lib", about: "Create a library package (src/lib.ks) instead of a binary (src/main.ks)");
    cmd = cmd.subcommand(initCmd);
    cmd = cmd.subcommand(Command("publish").about("Publish a package to the registry"));
    cmd = cmd.subcommand(Command("update").about("Update dependencies (re-resolve and rewrite flock.lock)"));
    var installCmd = Command("install").about("Build and install a package's binaries into ~/.flock/bin");
    installCmd = installCmd.argument(Argument(positional: "package", about: "Registry <org>/<pkg>[@version] or a local path; omit for the current package").optional());
    installCmd = installCmd.argument("bin", about: "Install only the named binary target");
    installCmd = installCmd.argument(flag: "force", about: "Overwrite an existing installed binary");
    installCmd = installCmd.argument(flag: "debug", about: "Build unoptimized (default is optimized)");
    cmd = cmd.subcommand(installCmd);

    match cmd.parse(from: argv) {
        .Ok(matches) => {
            match matches.subcommand {
                .Some(sub) => {
                    // The subcommand's own flags live in submatches[0]; `--release`
                    // turns on optimized codegen for build/run, `--bin` selects a
                    // single binary target.
                    var release = false;
                    var binFlag: Optional[String] = .None;
                    if matches.submatches.count > 0 {
                        let sm = matches.submatches(unchecked: 0);
                        release = sm.hasFlag("release");
                        binFlag = sm.value(of: "bin")
                    }
                    if sub == "build" {
                        handleBuild(release: release, binFlag: binFlag.clone())
                    } else if sub == "run" {
                        handleRun(release: release, binFlag: binFlag.clone())
                    } else if sub == "check" {
                        handleCheck()
                    } else if sub == "init" {
                        var lib = false;
                        if matches.submatches.count > 0 {
                            lib = matches.submatches(unchecked: 0).hasFlag("lib")
                        }
                        handleInit(lib: lib)
                    } else if sub == "publish" {
                        handlePublish()
                    } else if sub == "update" {
                        handleUpdate()
                    } else if sub == "install" {
                        // install defaults to an optimized build; --debug opts out.
                        var force = false;
                        var pkg: Optional[String] = .None;
                        var instRelease = true;
                        if matches.submatches.count > 0 {
                            let sm2 = matches.submatches(unchecked: 0);
                            force = sm2.hasFlag("force");
                            pkg = sm2.value(of: "package");
                            instRelease = not sm2.hasFlag("debug")
                        }
                        handleInstall(target: pkg, binFlag: binFlag.clone(), force: force, release: instRelease)
                    } else {
                        0
                    }
                },
                .None => {
                    // No subcommand — show help
                     println(cmd.helpText());
                    0
                }
            }
        },
        .Err(e) => {
            // ParseError.Message carries --help/--version text (success); every
            // other variant is a real usage error and must exit non-zero.
             eprintln(e.description());
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

func handleBuild(release release: Bool, binFlag binFlag: Optional[String]) -> lang.i32 {
    match resolveAndDiscover() {
        .Err(e) => {  eprintln(e.description()); 1 },
        .Ok(built) => {
            match selectBin(bins: built.bins, binFlag: binFlag, packageName: built.name) {
                .Err(e) => {  eprintln(e.description()); 1 },
                .Ok(bin) => {
                    var sources = built.shared.clone();
                    sources.append(bin.entry.clone());
                    var msg = String(); msg.append("Building "); msg.append(bin.name.clone());
                    if release { msg.append(" (release)") };
                    msg.append("...");
                     println(msg);
                    match invokeCompiler(mode: "build", sources: sources, output: .Some(bin.name.clone()), linkLibs: built.linkLibs, linkPaths: built.linkPaths, frameworks: built.frameworks, release: release) {
                        .Ok(_) => {
                            var doneMsg = String(); doneMsg.append("Built "); doneMsg.append(bin.name); doneMsg.append(" successfully");
                             println(doneMsg);
                            0
                        },
                        .Err(e) => {  eprintln(e.description()); 1 }
                    }
                }
            }
        }
    }
}

func handleRun(release release: Bool, binFlag binFlag: Optional[String]) -> lang.i32 {
    match resolveAndDiscover() {
        .Err(e) => {  eprintln(e.description()); 1 },
        .Ok(built) => {
            match selectBin(bins: built.bins, binFlag: binFlag, packageName: built.name) {
                .Err(e) => {  eprintln(e.description()); 1 },
                .Ok(bin) => {
                    var sources = built.shared.clone();
                    sources.append(bin.entry.clone());
                    match invokeCompiler(mode: "run", sources: sources, output: .None, linkLibs: built.linkLibs, linkPaths: built.linkPaths, frameworks: built.frameworks, release: release) {
                        .Ok(_) => 0,
                        .Err(e) => {  eprintln(e.description()); 1 }
                    }
                }
            }
        }
    }
}

func handleCheck() -> lang.i32 {
    match resolveAndDiscover() {
        .Err(e) => {  eprintln(e.description()); 1 },
        .Ok(built) => {
            // Check the whole package: shared sources plus every bin entry.
            var sources = built.shared.clone();
            var i: Int64 = 0;
            while i < built.bins.count {
                sources.append(built.bins(unchecked: i).entry.clone());
                i = i + 1
            }
            var msg = String(); msg.append("Checking "); msg.append(built.name); msg.append("...");
             println(msg);
            match invokeCompiler(mode: "check", sources: sources, output: .None, linkLibs: Array[String](), linkPaths: Array[String](), frameworks: Array[String](), release: false) {
                .Ok(_) => {  println("Check passed"); 0 },
                .Err(e) => {  eprintln(e.description()); 1 }
            }
        }
    }
}

func handleInit(lib lib: Bool) -> lang.i32 {
    let cwd = getcwd();
    let manifestPath = joinPath(base: cwd, rel: "flock.toml");

    if fileExists( manifestPath) {
         eprintln("flock.toml already exists in this directory");
        return 1
    }

    // Extract directory name as default package name
    let dirName = lastPathComponent(cwd);

    var content = String();
    content.append("[package]\nname = \""); content.append(dirName.clone()); content.append("\"\nversion = \"0.1.0\"\norg = \"\"\ndescription = \"\"\nauthor = \"\"\nlicense = \"\"\nrepository = \"\"\nwebsite = \"\"\ndocumentation = \"\"\n\n[dependencies]\n");

    match writeFileString(manifestPath, content) {
        .Ok(_) => {  println("Created flock.toml"); },
        .Err(e) => {
             eprintln("Failed to create flock.toml");
            return 1
        }
    }

    // Create src/ directory
    let srcDir = joinPath(base: cwd, rel: "src");
    if not isDirectory( srcDir) {
        var mkdirCmd = String(); mkdirCmd.append("mkdir -p "); mkdirCmd.append(srcDir.clone());
         spawn(mkdirCmd);
         println("Created src/");
    }

    // Scaffold an entry source so the package has a target out of the box:
    // a binary (src/main.ks) by default, or a library (src/lib.ks) with --lib.
    let modName = sanitizeModuleName(name: dirName.clone());
    if lib {
        let libPath = joinPath(base: srcDir, rel: "lib.ks");
        if not fileExists(libPath) {
            var c = String();
            c.append("module "); c.append(modName); c.append(".lib\n\n/// Returns a friendly greeting.\npublic func greeting() -> String {\n    \"Hello from "); c.append(dirName); c.append("\"\n}\n");
            match writeFileString(libPath, c) {
                .Ok(_) => {  println("Created src/lib.ks"); },
                .Err(_) => {}
            }
        }
    } else {
        let mainPath = joinPath(base: srcDir, rel: "main.ks");
        if not fileExists(mainPath) {
            var c = String();
            c.append("module "); c.append(modName); c.append(".main\n\n@main\nfunc main() -> lang.i32 {\n    println(\"Hello from "); c.append(dirName); c.append("!\");\n    0\n}\n");
            match writeFileString(mainPath, c) {
                .Ok(_) => {  println("Created src/main.ks"); },
                .Err(_) => {}
            }
        }
    }
    0
}

/// Turns a package/directory name into a valid module-name segment by replacing
/// any non-identifier byte with `_` (e.g. "my-tool" -> "my_tool").
func sanitizeModuleName(name name: String) -> String {
    var result = String();
    var i: Int64 = 0;
    while i < name.byteCount {
        let b = name.bytes(unchecked: i);
        let ok = (b >= 97 and b <= 122) or (b >= 65 and b <= 90) or (b >= 48 and b <= 57) or b == 95;
        if ok {
            result.append(name.asSlice().subslice(from: i, to: i + 1).toOwned())
        } else {
            result.append("_")
        }
        i = i + 1
    }
    if result.byteCount == 0 {
        result.append("pkg")
    }
    result
}

func handlePublish() -> lang.i32 {
    let cwd = getcwd();
    let manifestPath = joinPath(base: cwd, rel: "flock.toml");

    if not fileExists(manifestPath) {
         eprintln("flock.toml not found in current directory");
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
             eprintln("cannot read flock.toml");
            return 1
        },
        .Ok(source) => {
            match parseManifest(source: source) {
                .Err(e) => {
                     eprintln(e.description());
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

    // Read token from ~/.flock/credentials
    var token = "";
    let credPath = joinPath(base: flockHome(), rel: "credentials");
    match readFileString(credPath) {
        .Ok(contents) => token = trimWhitespace(contents),
        .Err(_) => {}
    }

    // Fall back to FLOCK_TOKEN env var
    if token.byteCount == 0 {
        match getenv("FLOCK_TOKEN") {
            .Some(t) => token = t,
            .None => {
                 eprintln("No auth token found.");
                 eprintln("Set FLOCK_TOKEN or save your token to ~/.flock/credentials");
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
         eprintln("failed to create archive");
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
         eprintln("Note: docs not generated (kestrel-doc not available or source has errors)");
    }

    // Upload archive via curl
    var url = String(); url.append(regUrl); url.append("/api/v1/packages/"); url.append(org); url.append("/"); url.append(name); url.append("/"); url.append(version);
    var curlCmd = String(); curlCmd.append("curl -s -X PUT "); curlCmd.append(quoteArg(url)); curlCmd.append(" -H \"Authorization: Bearer "); curlCmd.append(token); curlCmd.append("\" -H \"Content-Type: application/gzip\" --data-binary @"); curlCmd.append(archivePath);
    var pubMsg = String(); pubMsg.append("Publishing "); pubMsg.append(org); pubMsg.append("/"); pubMsg.append(name); pubMsg.append("@"); pubMsg.append(version); pubMsg.append(" to "); pubMsg.append(regUrl); pubMsg.append("...");
     println(pubMsg);

    let output = captureOutput(curlCmd);
     println(output);

    // Upload docs if generated
    if hasDocs {
        var docsPath = String(); docsPath.append(docsDir); docsPath.append("/docs.json");
        var docsUrl = String(); docsUrl.append(regUrl); docsUrl.append("/api/v1/packages/"); docsUrl.append(org); docsUrl.append("/"); docsUrl.append(name); docsUrl.append("/"); docsUrl.append(version); docsUrl.append("/docs");
        var docsCurlCmd = String(); docsCurlCmd.append("curl -s -X PUT "); docsCurlCmd.append(quoteArg(docsUrl)); docsCurlCmd.append(" -H \"Authorization: Bearer "); docsCurlCmd.append(token); docsCurlCmd.append("\" -H \"Content-Type: application/json\" --data-binary @"); docsCurlCmd.append(docsPath);
         println("Uploading documentation...");
        let docsOutput = captureOutput(docsCurlCmd);
         println(docsOutput);
    }

    // Clean up
    var rmCmd = String(); rmCmd.append("rm -f "); rmCmd.append(archivePath);
     spawn(rmCmd);
    var rmDocsCmd = String(); rmDocsCmd.append("rm -rf "); rmDocsCmd.append(docsDir);
     spawn(rmDocsCmd);
    0
}

func handleUpdate() -> lang.i32 {
    let cwd = getcwd();
    let lockPath = joinPath(base: cwd, rel: "flock.lock");

    // Delete existing lock file to force re-resolution
    if fileExists(lockPath) {
        var rmCmd = String(); rmCmd.append("rm "); rmCmd.append(lockPath);
         spawn(rmCmd);
         println("Removed flock.lock");
    }

    // Re-resolve everything
    match resolveAndDiscover() {
        .Err(e) => {  eprintln(e.description()); 1 },
        .Ok(built) => {
            var msg = String(); msg.append("Dependencies updated for "); msg.append(built.name);
             println(msg);
            0
        }
    }
}

func handleInstall(target target: Optional[String], binFlag binFlag: Optional[String], force force: Bool, release release: Bool) -> lang.i32 {
    match resolveInstallRoot(target: target) {
        .Err(e) => {  eprintln(e.description()); 1 },
        .Ok(root) => {
            let pkgName = root.manifest.package.name.clone();
            match collectBuild(root: root) {
                .Err(e) => {  eprintln(e.description()); 1 },
                .Ok(built) => {
                    if built.bins.count == 0 {
                         eprintln(FlockError.NoBinaryTargets(pkgName).description());
                        return 1
                    }

                    // Select targets: --bin narrows to one, otherwise install all.
                    var targets = Array[BinTarget]();
                    match binFlag {
                        .Some(name) => {
                            match selectBin(bins: built.bins, binFlag: .Some(name.clone()), packageName: pkgName.clone()) {
                                .Ok(b) => targets.append(b),
                                .Err(e) => {  eprintln(e.description()); return 1 }
                            }
                        },
                        .None => {
                            var i: Int64 = 0;
                            while i < built.bins.count {
                                targets.append(built.bins(unchecked: i).clone());
                                i = i + 1
                            }
                        }
                    }

                    // Ensure ~/.flock/bin exists.
                    let binDir = joinPath(base: flockHome(), rel: "bin");
                    match mkdirAll(binDir) {
                        .Ok(_) => {},
                        .Err(_) => {  eprintln("failed to create ~/.flock/bin"); return 1 }
                    }

                    // Build + install each target.
                    var i: Int64 = 0;
                    while i < targets.count {
                        let bin = targets(unchecked: i);
                        i = i + 1;
                        match installOne(bin: bin, built: built, binDir: binDir, force: force, release: release) {
                            .Ok(_) => {},
                            .Err(e) => {  eprintln(e.description()); return 1 }
                        }
                    }

                    printPathHintIfNeeded(binDir: binDir);
                    0
                }
            }
        }
    }
}

/// Resolves the package to install: the current directory (no target), a
/// registry package `<org>/<pkg>[@version]`, or a local directory path.
func resolveInstallRoot(target target: Optional[String]) -> Result[ResolvedPackage, FlockError] {
    match target {
        .None => loadLocalPackage(rootDir: getcwd()),
        .Some(t) => {
            if not isRegistryName(name: t) {
                // A bare (non-`org/pkg`) target is treated as a local path.
                return loadLocalPackage(rootDir: t)
            }
            // Split `<org>/<pkg>@<version>` into the name and an optional pin.
            var name = t.clone();
            var verOpt: Optional[String] = .None;
            var i: Int64 = 0;
            while i < t.byteCount {
                if t.bytes(unchecked: i) == 64 { // '@'
                    name = t.asSlice().subslice(from: 0, to: i).toOwned();
                    verOpt = .Some(t.asSlice().subslice(from: i + 1, to: t.byteCount).toOwned());
                    break
                }
                i = i + 1
            }
            // Bare name => latest stable (.Any); `@x.y.z` => exact pin.
            var constraint = VersionConstraint.Any;
            match verOpt {
                .Some(v) => {
                    match parseVersion(s: v) {
                        .Ok(ver) => constraint = VersionConstraint.Exact(ver),
                        .Err(e) => return .Err(e)
                    }
                },
                .None => {}
            }
            let regUrl = resolveRegistryUrl(projectUrl: .None);
            let regSrc = RegistrySource(config: RegistryConfig(url: regUrl));
            regSrc.resolve(name: name, spec: DependencySpec.Registry(constraint), baseDir: getcwd())
        }
    }
}

/// Builds one binary target to a temp file, then atomically moves it into
/// `~/.flock/bin`. Refuses toolchain names and won't clobber without `force`.
func installOne(bin bin: BinTarget, built built: ResolvedBuild, binDir binDir: String, force force: Bool, release release: Bool) -> Result[(), FlockError] {
    if bin.name == "flock" or bin.name == "kestrel" or bin.name == "jessup" {
        var m = String(); m.append("refusing to install a binary named '"); m.append(bin.name.clone()); m.append("' (would shadow the toolchain)");
        return .Err(FlockError.IoError(m))
    }

    let dest = joinPath(base: binDir, rel: bin.name);
    if fileExists(dest) and not force {
        return .Err(FlockError.BinaryExists(bin.name.clone()))
    }

    var tmp = String(); tmp.append(dest); tmp.append(".tmp");
    var sources = built.shared.clone();
    sources.append(bin.entry.clone());
    match invokeCompiler(mode: "build", sources: sources, output: .Some(tmp.clone()), linkLibs: built.linkLibs.clone(), linkPaths: built.linkPaths.clone(), frameworks: built.frameworks.clone(), release: release) {
        .Err(e) => return .Err(e),
        .Ok(_) => {}
    }

    match rename(tmp, dest.clone()) {
        .Ok(_) => {},
        .Err(_) => {
            var m = String(); m.append("failed to move binary into "); m.append(binDir.clone());
            return .Err(FlockError.IoError(m))
        }
    }

    var msg = String(); msg.append("Installed "); msg.append(bin.name.clone()); msg.append(" -> "); msg.append(dest);
     println(msg);
    .Ok(())
}

// ============================================================================
// RESOLVED BUILD
// ============================================================================

/// Everything needed to build a package's binaries: the shared compile set
/// (all dependency + package sources EXCEPT every package's bin entries), the
/// package's own binary targets, the collected link flags, and the resolved
/// dependency nodes (for lock-file generation). `build`, `run`, and `install`
/// all compile `shared + [one bin entry]`.
struct ResolvedBuild: Cloneable {
    var name: String
    var shared: Array[String]
    var bins: Array[BinTarget]
    var linkLibs: Array[String]
    var linkPaths: Array[String]
    var frameworks: Array[String]
    var nodes: Array[DepNode]

    init(name name: String, shared shared: Array[String], bins bins: Array[BinTarget], linkLibs linkLibs: Array[String], linkPaths linkPaths: Array[String], frameworks frameworks: Array[String], nodes nodes: Array[DepNode]) {
        self.name = name;
        self.shared = shared;
        self.bins = bins;
        self.linkLibs = linkLibs;
        self.linkPaths = linkPaths;
        self.frameworks = frameworks;
        self.nodes = nodes;
    }

    func clone() -> ResolvedBuild {
        ResolvedBuild(name: self.name.clone(), shared: self.shared.clone(), bins: self.bins.clone(), linkLibs: self.linkLibs.clone(), linkPaths: self.linkPaths.clone(), frameworks: self.frameworks.clone(), nodes: self.nodes.clone())
    }
}

// ============================================================================
// CORE LOGIC
// ============================================================================

/// Reads and parses `<rootDir>/flock.toml` into a ResolvedPackage.
func loadLocalPackage(rootDir rootDir: String) -> Result[ResolvedPackage, FlockError] {
    let manifestPath = joinPath(base: rootDir, rel: "flock.toml");
    if not fileExists(manifestPath) {
        return .Err(FlockError.ManifestNotFound(manifestPath))
    }
    match readFileString(manifestPath) {
        .Err(e) => {
            var msg = String(); msg.append("cannot read "); msg.append(manifestPath);
            .Err(FlockError.IoError(msg))
        },
        .Ok(source) => {
            match parseManifest(source: source) {
                .Err(e) => .Err(e),
                .Ok(m) => .Ok(ResolvedPackage(name: m.package.name, version: m.package.version, rootDir: rootDir, manifest: m))
            }
        }
    }
}

/// Resolves `root`'s dependency graph, discovers sources (excluding each
/// package's own bin entries so a dependency's `@main` can't leak in), compiles
/// C, and collects link flags. Returns the shared compile set plus `root`'s
/// binary targets. Does NOT write the lock file.
func collectBuild(root root: ResolvedPackage) -> Result[ResolvedBuild, FlockError] {
    let pathSrc = PathSource();
    let regUrl = resolveRegistryUrl(projectUrl: root.manifest.registryUrl);
    let regSrc = RegistrySource(config: RegistryConfig(url: regUrl));

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

        // Discover .ks sources, then exclude this package's own bin entries so
        // a dependency's @main never leaks into the shared compile.
        let srcDir = joinPath(base: node.rootDir, rel: node.sourceDir);
        let sources = discoverSources(rootDir: srcDir);
        var nodeEntries = Array[String]();
        match discoverBins(rootDir: node.rootDir, sourceDir: node.sourceDir, packageName: node.name, bins: node.bins) {
            .Err(e) => return .Err(e),
            .Ok(targets) => {
                var t: Int64 = 0;
                while t < targets.count {
                    nodeEntries.append(targets(unchecked: t).entry.clone());
                    t = t + 1
                }
            }
        }
        var j: Int64 = 0;
        var nodeLibCount: Int64 = 0;
        while j < sources.count {
            let s = sources(unchecked: j);
            if not containsString(arr: nodeEntries, value: s) {
                allSources.append(s);
                nodeLibCount = nodeLibCount + 1
            }
            j = j + 1
        }

        // Cargo strategy: a dependency must expose a library (≥1 non-bin source).
        // A bin-only package can't be depended on. The root is exempt — you
        // build/install ITS binaries.
        if node.rootDir != root.rootDir and nodeLibCount == 0 {
            return .Err(FlockError.NoLibraryTarget(node.name))
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

    // Discover the package's own binary targets.
    var bins = Array[BinTarget]();
    match discoverBins(rootDir: root.rootDir, sourceDir: root.manifest.package.source, packageName: root.manifest.package.name, bins: root.manifest.bins) {
        .Err(e) => return .Err(e),
        .Ok(b) => bins = b
    }

    .Ok(ResolvedBuild(name: root.manifest.package.name, shared: allSources, bins: bins, linkLibs: allLinkLibs, linkPaths: allLinkPaths, frameworks: allFrameworks, nodes: sorted))
}

/// Writes flock.lock from the resolved dependency nodes (skips the root).
func writeLockFile(cwd cwd: String, rootName rootName: String, nodes nodes: Array[DepNode]) {
    var lockEntries = Array[LockEntry]();
    var i: Int64 = 0;
    while i < nodes.count {
        let node = nodes(unchecked: i);
        // Skip the root package itself
        if node.name != rootName {
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
}

/// Reads the current package, resolves + discovers everything, writes the lock
/// file, and returns the build set (shared sources + binary targets).
func resolveAndDiscover() -> Result[ResolvedBuild, FlockError] {
    let cwd = getcwd();
    match loadLocalPackage(rootDir: cwd) {
        .Err(e) => .Err(e),
        .Ok(root) => {
            let rootName = root.name.clone();
            match collectBuild(root: root) {
                .Err(e) => .Err(e),
                .Ok(built) => {
                    writeLockFile(cwd: cwd, rootName: rootName, nodes: built.nodes);
                    .Ok(built)
                }
            }
        }
    }
}

/// Selects which binary target to build. With `--bin`, the named target (or
/// BinNotFound). Otherwise the sole target, else the package-named default
/// (src/main.ks), else AmbiguousBinary listing the candidates.
func selectBin(bins bins: Array[BinTarget], binFlag binFlag: Optional[String], packageName packageName: String) -> Result[BinTarget, FlockError] {
    if bins.count == 0 {
        return .Err(FlockError.NoBinaryTargets(packageName))
    }
    match binFlag {
        .Some(name) => {
            var i: Int64 = 0;
            while i < bins.count {
                if bins(unchecked: i).name == name {
                    return .Ok(bins(unchecked: i).clone())
                }
                i = i + 1
            }
            .Err(FlockError.BinNotFound(name))
        },
        .None => {
            if bins.count == 1 {
                return .Ok(bins(unchecked: 0).clone())
            }
            // Prefer the package-named default (src/main.ks).
            var i: Int64 = 0;
            while i < bins.count {
                if bins(unchecked: i).name == packageName {
                    return .Ok(bins(unchecked: i).clone())
                }
                i = i + 1
            }
            // Ambiguous — list the candidate names.
            var names = String();
            i = 0;
            while i < bins.count {
                if i > 0 { names.append(", ") };
                names.append(bins(unchecked: i).name.clone());
                i = i + 1
            }
            .Err(FlockError.AmbiguousBinary(names))
        }
    }
}

// ============================================================================
// HELPERS
// ============================================================================

/// Prints a hint to add `binDir` to PATH when it isn't already there.
func printPathHintIfNeeded(binDir binDir: String) {
    let path = match getenv("PATH") {
        .Some(p) => p,
        .None => return
    };
    if pathContains(path: path, dir: binDir) {
        return
    }
     println("");
    var m = String(); m.append("note: "); m.append(binDir.clone()); m.append(" is not on your PATH. Add it with:");
     println(m);
     println("    export PATH=\"$HOME/.flock/bin:$PATH\"");
}

/// True if the `:`-separated `path` contains a segment equal to `dir`.
func pathContains(path path: String, dir dir: String) -> Bool {
    var start: Int64 = 0;
    var i: Int64 = 0;
    let len = path.byteCount;
    while i <= len {
        if i == len or path.bytes(unchecked: i) == 58 { // ':'
            if i > start {
                let seg = path.asSlice().subslice(from: start, to: i).toOwned();
                if seg == dir {
                    return true
                }
            }
            start = i + 1
        }
        i = i + 1
    }
    false
}

/// True if `arr` contains a string equal to `value`.
func containsString(arr arr: Array[String], value value: String) -> Bool {
    var i: Int64 = 0;
    while i < arr.count {
        if arr(unchecked: i) == value {
            return true
        }
        i = i + 1
    }
    false
}

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
