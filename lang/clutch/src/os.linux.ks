// OS-level bindings for CLI tools (Linux)
//
// Provides CLI-specific functionality like command-line argument access.
// Uses /proc/self/cmdline to read arguments on Linux.

module clutch.os

// std.os and std.io functions are auto-imported from stdlib

// ============================================================================
// PUBLIC API
// ============================================================================

/// Returns the command-line arguments as an array of strings.
/// Skips argv[0] (the program name).
public func getArgv() -> Array[String] {
    var result = Array[String]();

    // Open /proc/self/cmdline using stdlib io functions
    let path = "/proc/self/cmdline".toCString();
    let fd = open(path.raw, O_RDONLY(), 0);
    path.free();
    if fd < 0 {
        return result
    }

    // Read the entire cmdline (args are null-separated)
    var buf = Array[UInt8](capacity: 4096);
    var i: Int64 = 0;
    while i < 4096 {
        buf.append(0);
        i = i + 1
    }
    let n = read(fd, buf.asPointer(), 4096);
    let _ = close(fd);

    if n <= 0 {
        return result
    }

    // Parse null-separated arguments, skip first (program name)
    var argStart: Int64 = 0;
    var argIndex: Int64 = 0;
    var pos: Int64 = 0;
    while pos < n {
        if buf(pos) == 0 {
            if argIndex > 0 {
                // Build string from argStart to pos
                var s = String();
                var j = argStart;
                while j < pos {
                    s.appendByte(buf(j));
                    j = j + 1
                }
                result.append(s);
            }
            argIndex = argIndex + 1;
            argStart = pos + 1
        }
        pos = pos + 1
    }

    result
}
