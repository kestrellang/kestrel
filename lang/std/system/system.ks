// Kestrel System Library
//
// The running process and the machine it runs on: environment variables,
// child processes, exit, the target platform, and machine facts.
//
// `std.system` replaces `std.os`'s process, environment and platform parts.
// `std.os` stays until callers have moved over.
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
