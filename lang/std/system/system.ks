// Kestrel System Library
//
// The running process and the machine it runs on: environment variables,
// child processes, exit, the target platform, and machine facts.
//
// # Examples
//
// ```
// let home = Environment.homeDirectory();
// let status = try Command("git").arguments(["status", "--short"]).run();
// if Platform.current.operatingSystem == .Darwin { ... }
// exit(ExitCode.success)
// ```

module std.system
