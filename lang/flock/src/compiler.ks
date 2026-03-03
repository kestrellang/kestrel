// Compiler invocation
//
// Builds a kestrel command and spawns it via system().

module flock.compiler

import flock.error.(FlockError)

// ============================================================================
// COMPILER INVOCATION
// ============================================================================

/// Invokes the kestrel compiler with the given mode, sources, and optional output.
///
/// mode: "build", "run", or "check"
/// sources: all .ks files to compile (in dependency order)
/// output: output binary name (only used with "build")
/// linkLibs: libraries and object files to link (-l flags)
/// linkPaths: library search paths (-L flags)
/// frameworks: macOS frameworks (--framework flags)
public func invokeCompiler(
    mode mode: String,
    sources sources: Array[String],
    output output: Optional[String],
    linkLibs linkLibs: Array[String],
    linkPaths linkPaths: Array[String],
    frameworks frameworks: Array[String]
) -> Result[(), FlockError] {
    // Use KESTREL env var if set, otherwise fall back to "kestrel" in PATH
    var compiler = "kestrel";
    match getenv("KESTREL") {
        .Some(path) => compiler = path,
        .None => {}
    }

    var cmd = compiler + " " + mode;

    // Pass --std if KESTREL_STD is set
    match getenv("KESTREL_STD") {
        .Some(stdPath) => {
            cmd = cmd + " --std " + quoteArg(stdPath)
        },
        .None => {}
    }

    // Add all source files
    var i: Int64 = 0;
    while i < sources.count {
        cmd = cmd + " " + quoteArg(sources(unchecked: i));
        i = i + 1
    }

    // Add output flag for build mode
    match output {
        .Some(out) => {
            if mode.equals("build") {
                cmd = cmd + " -o " + quoteArg(out)
            }
        },
        .None => {}
    }

    // Add link libraries (-l flags)
    i = 0;
    while i < linkLibs.count {
        cmd = cmd + " -l " + quoteArg(linkLibs(unchecked: i));
        i = i + 1
    }

    // Add library search paths (-L flags)
    i = 0;
    while i < linkPaths.count {
        cmd = cmd + " -L " + quoteArg(linkPaths(unchecked: i));
        i = i + 1
    }

    // Add frameworks (--framework flags)
    i = 0;
    while i < frameworks.count {
        cmd = cmd + " --framework " + quoteArg(frameworks(unchecked: i));
        i = i + 1
    }

    let exitCode = spawn(cmd);
    if exitCode != 0 {
        return .Err(FlockError.CompilerFailed(exitCode))
    }

    .Ok(())
}

/// Quotes a shell argument if it contains spaces.
func quoteArg(s: String) -> String {
    if containsSpace(s) {
        "\"" + s + "\""
    } else {
        s
    }
}

func containsSpace(s: String) -> Bool {
    var i: Int64 = 0;
    while i < s.byteCount {
        if s.byteAtUnchecked(i) == UInt8(intLiteral: 32) { // space
            return true
        }
        i = i + 1
    }
    false
}
