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

    var cmd = String();
    cmd.append(compiler);
    cmd.append(" ");
    cmd.append(mode);

    // Pass --std if KESTREL_STD is set
    match getenv("KESTREL_STD") {
        .Some(stdPath) => {
            cmd.append(" --std ");
            cmd.append(quoteArg(stdPath))
        },
        .None => {}
    }

    // Add all source files
    for source in sources {
        cmd.append(" ");
        cmd.append(quoteArg(source))
    }

    // Add output flag for build mode
    match output {
        .Some(out) => {
            if mode.equals("build") {
                cmd.append(" -o ");
                cmd.append(quoteArg(out))
            }
        },
        .None => {}
    }

    // Add link libraries (-l flags)
    for lib in linkLibs {
        cmd.append(" -l ");
        cmd.append(quoteArg(lib))
    }

    // Add library search paths (-L flags)
    for path in linkPaths {
        cmd.append(" -L ");
        cmd.append(quoteArg(path))
    }

    // Add frameworks (--framework flags)
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
        if s.byteAtUnchecked(i) == 32 { // space
            return true
        }
        i = i + 1
    }
    false
}
