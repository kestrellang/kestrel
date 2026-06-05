// Compiler invocation
//
// The kestrel binary only exposes `build` and `dump` subcommands, so flock
// synthesizes `run` (build to a temp file + exec) and `check` (dump
// diagnostics) on top of those.

module flock.compiler

import flock.error.(FlockError)

// ============================================================================
// COMPILER INVOCATION
// ============================================================================

/// Invokes the kestrel compiler with the given mode, sources, and optional output.
///
/// mode: "build", "run", or "check"
/// sources: all .ks files to compile (in dependency order)
/// output: output binary name (only used with "build"; ignored for run/check)
/// linkLibs: libraries and object files to link (-l flags)
/// linkPaths: library search paths (-L flags)
/// frameworks: macOS frameworks (--framework flags)
/// release: when true, build optimized (passes `-O 2`); ignored for check
public func invokeCompiler(
    mode mode: String,
    sources sources: Array[String],
    output output: Optional[String],
    linkLibs linkLibs: Array[String],
    linkPaths linkPaths: Array[String],
    frameworks frameworks: Array[String],
    release release: Bool
) -> Result[(), FlockError] {
    if mode == "run" {
        return invokeRun(sources: sources, linkLibs: linkLibs, linkPaths: linkPaths, frameworks: frameworks, release: release)
    }
    if mode == "check" {
        return invokeCheck(sources: sources)
    }
    invokeBuild(sources: sources, output: output, linkLibs: linkLibs, linkPaths: linkPaths, frameworks: frameworks, release: release)
}

// ----------------------------------------------------------------------------
// build: `kestrel build ... -o <out>`
// ----------------------------------------------------------------------------

func invokeBuild(
    sources sources: Array[String],
    output output: Optional[String],
    linkLibs linkLibs: Array[String],
    linkPaths linkPaths: Array[String],
    frameworks frameworks: Array[String],
    release release: Bool
) -> Result[(), FlockError] {
    var cmd = String();
    cmd.append(compilerPath());
    cmd.append(" build");

    // `--release` maps to the compiler's `-O 2` (Cranelift speed_and_size).
    // Default (debug) builds omit the flag, leaving the compiler at -O 0.
    if release {
        cmd.append(" -O 2")
    }

    for source in sources {
        cmd.append(" ");
        cmd.append(quoteArg(source))
    }

    match output {
        .Some(out) => { cmd.append(" -o "); cmd.append(quoteArg(out)) },
        .None => {}
    }

    for lib in linkLibs {
        cmd.append(" -l ");
        cmd.append(quoteArg(lib))
    }
    for path in linkPaths {
        cmd.append(" -L ");
        cmd.append(quoteArg(path))
    }
    for framework in frameworks {
        cmd.append(" --framework ");
        cmd.append(quoteArg(framework))
    }

    let exitCode = spawn(cmd);
    if exitCode != 0 {
        return .Err(FlockError.CompilerFailed(exitCode))
    }
    .Ok(())
}

// ----------------------------------------------------------------------------
// run: build to a temp path, exec it, clean up
// ----------------------------------------------------------------------------

func invokeRun(
    sources sources: Array[String],
    linkLibs linkLibs: Array[String],
    linkPaths linkPaths: Array[String],
    frameworks frameworks: Array[String],
    release release: Bool
) -> Result[(), FlockError] {
    // `mktemp -t flock-run` works on both macOS and Linux.
    let tempPath = captureOutput("mktemp -t flock-run");
    if tempPath.byteCount == 0 {
        return .Err(FlockError.IoError("failed to create temp file for run"))
    }

    match invokeBuild(sources: sources, output: .Some(tempPath), linkLibs: linkLibs, linkPaths: linkPaths, frameworks: frameworks, release: release) {
        .Err(e) => {
            cleanupTemp(path: tempPath);
            return .Err(e)
        },
        .Ok(_) => {}
    }

    let exitCode = spawn(quoteArg(tempPath));
    cleanupTemp(path: tempPath);

    if exitCode != 0 {
        return .Err(FlockError.CompilerFailed(exitCode))
    }
    .Ok(())
}

func cleanupTemp(path path: String) {
    var rm = String();
    rm.append("rm -f ");
    rm.append(quoteArg(path));
     spawn(rm);
}

// ----------------------------------------------------------------------------
// check: `kestrel dump diagnostics ...` (exits non-zero if any errors)
// ----------------------------------------------------------------------------

func invokeCheck(sources sources: Array[String]) -> Result[(), FlockError] {
    var cmd = String();
    cmd.append(compilerPath());
    cmd.append(" dump diagnostics");

    for source in sources {
        cmd.append(" ");
        cmd.append(quoteArg(source))
    }

    let exitCode = spawn(cmd);
    if exitCode != 0 {
        return .Err(FlockError.CompilerFailed(exitCode))
    }
    .Ok(())
}

// ----------------------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------------------

func compilerPath() -> String {
    match getenv("KESTREL") {
        .Some(path) => path,
        .None => "kestrel"
    }
}

/// Quotes a shell argument if it contains spaces.
func quoteArg(s: String) -> String {
    if containsSpace(s) {
        var q = String();
        q.append("\"");
        q.append(s);
        q.append("\"");
        q
    } else {
        s
    }
}

func containsSpace(s: String) -> Bool {
    var i: Int64 = 0;
    while i < s.byteCount {
        if s.bytes(unchecked: i) == 32 { // space
            return true
        }
        i = i + 1
    }
    false
}
