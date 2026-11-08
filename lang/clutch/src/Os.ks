/// Access to the process's command-line arguments.
///
/// Provides `getArgv()`, which returns the raw `argv` tokens (minus the
/// program name) as an `Array[String]`, read through std.system's
/// `Process.arguments()`.

module clutch.os

/// Returns the command-line arguments as an array of strings.
///
/// The program name (`argv[0]`) is skipped; the returned array starts
/// with the first user-supplied argument. Returns an empty array when
/// no arguments were passed.
///
/// # Examples
///
/// ```
/// // $ mycli --verbose input.txt
/// let args = getArgv();
/// args;  // ["--verbose", "input.txt"]
/// ```
public func getArgv() -> Array[String] {
    var result = Array[String]();
    var first = true;
    for arg in Process.arguments() {
        if first {
            first = false;
            continue
        }
        result.append(arg)
    }
    result
}
