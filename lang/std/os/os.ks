// Kestrel OS Library
//
// Provides filesystem operations, environment variable access,
// and process spawning on POSIX systems.
//
// Example usage:
//
//   // Check if a path exists
//   if os.fileExists("/tmp/myfile") { ... }
//
//   // Create directories
//   try os.mkdirAll("/tmp/foo/bar/baz")
//
//   // Read environment variables
//   let home = os.getenv(name: "HOME")
//
//   // Spawn a process
//   let exitCode = os.spawn(command: "ls -la")

module std.os
