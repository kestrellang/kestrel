// Kestrel System Library
//
// The running process and the machine it runs on: environment variables,
// child processes, exit, the target platform, and machine facts.
//
// # Examples
//
// ```
// let home = Environment.homeDirectory();
// let status = try run("git", ["status", "--short"]);
// if Platform.current.operatingSystem == .Darwin { ... }
// exit(ExitCode.success)
// ```

module std.system
