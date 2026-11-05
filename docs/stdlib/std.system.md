# std.system

## enum `Architecture`

```kestrel
public enum Architecture
```

A CPU architecture. Cases are added as Kestrel gains targets.

_Defined in `lang/std/system/platform.ks`._

### Members

#### case `Arm32`

```kestrel
case Arm32
```

32-bit ARM.

_Defined in `lang/std/system/platform.ks`._

#### case `Arm64`

```kestrel
case Arm64
```

64-bit ARM (`aarch64`, Apple silicon).

_Defined in `lang/std/system/platform.ks`._

#### case `RiscV64`

```kestrel
case RiscV64
```

64-bit RISC-V.

_Defined in `lang/std/system/platform.ks`._

#### case `Wasm32`

```kestrel
case Wasm32
```

32-bit WebAssembly.

_Defined in `lang/std/system/platform.ks`._

#### case `X86`

```kestrel
case X86
```

32-bit x86 (`i686`).

_Defined in `lang/std/system/platform.ks`._

#### case `X86_64`

```kestrel
case X86_64
```

64-bit x86 (`x86_64`, `amd64`).

_Defined in `lang/std/system/platform.ks`._

#### field `name`

```kestrel
public var name: String { get }
```

The name used in target triples and release asset names:
`x86_64`, `arm64`, `x86`, `arm`, `riscv64`, `wasm32`.

_Defined in `lang/std/system/platform.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Architecture) -> Bool
```

_Defined in `lang/std/system/platform.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

_Defined in `lang/std/system/platform.ks`._

## struct `Child`

```kestrel
public struct Child: not Copyable { /* private fields */ }
```

A started child process.

Dropping a `Child` that hasn't been waited on closes its pipes and then
waits for it, so it never lingers as a zombie. Call `kill` first if it
might not exit by itself.

_Defined in `lang/std/system/process.ks`._

### Members

#### function `collectOutput`

```kestrel
mutating func collectOutput(Int64, Optional[Int64]) -> Result[Output, IoError]
```

Drains the piped output streams with `poll`, so a child filling one
pipe can't deadlock against a parent blocked on the other.

_Defined in `lang/std/system/process.ks`._

#### field `error`

```kestrel
var error: Optional[PipeReader]
```

_Defined in `lang/std/system/process.ks`._

#### function `finish`

```kestrel
mutating func finish(Optional[Int64]) -> Result[ExitStatus, IoError]
```

`wait()`, or a timed wait that kills the child when time runs out.

_Defined in `lang/std/system/process.ks`._

#### field `id`

```kestrel
public var id: ProcessId { get }
```

The child's process ID.

_Defined in `lang/std/system/process.ks`._

#### field `input`

```kestrel
var input: Optional[PipeWriter]
```

_Defined in `lang/std/system/process.ks`._

#### function `kill`

```kestrel
public mutating func kill() -> Result[(), IoError]
```

Stops the child at once (`SIGKILL`). It can't be caught or ignored.

_Defined in `lang/std/system/process.ks`._

#### field `output`

```kestrel
var output: Optional[PipeReader]
```

_Defined in `lang/std/system/process.ks`._

#### field `pid`

```kestrel
var pid: Int32
```

_Defined in `lang/std/system/process.ks`._

#### function `reap`

```kestrel
mutating func reap(blocking: Bool) -> Result[Optional[ExitStatus], IoError]
```

One `waitpid`. Caches the status, since a pid can only be reaped once.

_Defined in `lang/std/system/process.ks`._

#### function `signal`

```kestrel
mutating func signal(Int32) -> Result[(), IoError]
```

_Defined in `lang/std/system/process.ks`._

#### field `status`

```kestrel
var status: Optional[ExitStatus]
```

_Defined in `lang/std/system/process.ks`._

#### function `takeStandardError`

```kestrel
public mutating func takeStandardError() -> Optional[PipeReader]
```

Takes the pipe from the child's standard error, if piped.

_Defined in `lang/std/system/process.ks`._

#### function `takeStandardInput`

```kestrel
public mutating func takeStandardInput() -> Optional[PipeWriter]
```

Takes the pipe to the child's standard input, if it was started with
`Stdio.Pipe`. Later calls return `None`.

_Defined in `lang/std/system/process.ks`._

#### function `takeStandardOutput`

```kestrel
public mutating func takeStandardOutput() -> Optional[PipeReader]
```

Takes the pipe from the child's standard output, if piped.

_Defined in `lang/std/system/process.ks`._

#### function `terminate`

```kestrel
public mutating func terminate() -> Result[(), IoError]
```

Asks the child to stop (`SIGTERM`). It may clean up, or ignore it.

_Defined in `lang/std/system/process.ks`._

#### function `tryWait`

```kestrel
public mutating func tryWait() -> Result[Optional[ExitStatus], IoError]
```

The exit status if the child has already exited, else `None`.
Never blocks.

_Defined in `lang/std/system/process.ks`._

#### function `wait`

```kestrel
public mutating func wait() -> Result[ExitStatus, IoError]
```

Waits for the child to exit. Closes its standard input first, so a
child reading to end-of-file can finish.

_Defined in `lang/std/system/process.ks`._

#### function `wait`

```kestrel
public mutating func wait(milliseconds: Int64) -> Result[Optional[ExitStatus], IoError]
```

Waits up to `milliseconds` for the child to exit. `None` if it is
still running when the time is up; the child is left running.

_Defined in `lang/std/system/process.ks`._

#### function `waitForOutput`

```kestrel
public mutating func waitForOutput() -> Result[Output, IoError]
```

Reads all of standard output and error (whichever were piped) and
waits for the child to exit.

_Defined in `lang/std/system/process.ks`._

## struct `Command`

```kestrel
public struct Command: not Copyable { /* private fields */ }
```

A program to run, with its arguments and environment.

Arguments are passed as a list straight to the program, with no shell in
between, so spaces and quotes in them need no escaping. Use
`Command.shell` when you want a shell.

Builder methods return the command so calls chain.

### Examples

```
let status = try Command("tar").arguments(["-czf", out, "-C", dir, "."]).run();

let result = try Command("git").arguments(["rev-parse", "HEAD"]).output();
if result.status.isSuccess {
    print(result.standardOutputText() ?? "")
}
```

_Defined in `lang/std/system/process.ks`._

### Members

#### function `argument`

```kestrel
public mutating func argument(String) -> &mutating Command
```

Adds one argument.

_Defined in `lang/std/system/process.ks`._

#### field `argumentList`

```kestrel
var argumentList: Array[String]
```

_Defined in `lang/std/system/process.ks`._

#### function `arguments`

```kestrel
public mutating func arguments(Array[String]) -> &mutating Command
```

Adds each of `args`, in order.

_Defined in `lang/std/system/process.ks`._

#### function `childEnvironment`

```kestrel
func childEnvironment() -> Optional[NativeStringArray]
```

The child's `envp`, or `None` to inherit this process's unchanged.

_Defined in `lang/std/system/process.ks`._

#### function `clearEnvironment`

```kestrel
public mutating func clearEnvironment() -> &mutating Command
```

Starts the child with no inherited variables; only those set with
`environment(_:to:)` reach it.

_Defined in `lang/std/system/process.ks`._

#### field `clearsEnvironment`

```kestrel
var clearsEnvironment: Bool
```

_Defined in `lang/std/system/process.ks`._

#### function `directory`

```kestrel
public mutating func directory(String) -> &mutating Command
```

Runs the child in `path` instead of the current directory.

_Defined in `lang/std/system/process.ks`._

#### function `environment`

```kestrel
public mutating func environment(String, to: String) -> &mutating Command
```

Sets variable `name` for the child only.

_Defined in `lang/std/system/process.ks`._

#### field `environmentChanges`

```kestrel
var environmentChanges: Array[(String, Optional[String])]
```

_Defined in `lang/std/system/process.ks`._

#### field `errorMode`

```kestrel
var errorMode: Optional[Stdio]
```

_Defined in `lang/std/system/process.ks`._

#### function `find`

```kestrel
public static func find(String) -> Optional[String]
```

The path `program` resolves to on `PATH`, or `None` if no executable
by that name is found. A name containing `/` is returned as-is when
that file is executable.

_Defined in `lang/std/system/process.ks`._

#### initializer `init`

```kestrel
public init(String)
```

A command running `program`. A name without `/` is looked up on
`PATH`; anything with a `/` is used as a path.

_Defined in `lang/std/system/process.ks`._

#### field `inputMode`

```kestrel
var inputMode: Optional[Stdio]
```

_Defined in `lang/std/system/process.ks`._

#### function `output`

```kestrel
public func output() -> Result[Output, IoError]
```

Runs the command, capturing standard output and error, and waits for
it. Standard input is `/dev/null` unless set with `standardInput`.

_Defined in `lang/std/system/process.ks`._

#### function `output`

```kestrel
public func output(limit: Int64) -> Result[Output, IoError]
```

Like `output()`, but keeps at most `bytes` of each stream; the rest is
read and discarded so the child doesn't block. `0` means no limit.

_Defined in `lang/std/system/process.ks`._

#### field `outputMode`

```kestrel
var outputMode: Optional[Stdio]
```

_Defined in `lang/std/system/process.ks`._

#### field `program`

```kestrel
var program: String
```

_Defined in `lang/std/system/process.ks`._

#### function `removeEnvironment`

```kestrel
public mutating func removeEnvironment(String) -> &mutating Command
```

Unsets variable `name` for the child only.

_Defined in `lang/std/system/process.ks`._

#### function `run`

```kestrel
public func run() -> Result[ExitStatus, IoError]
```

Runs the command with inherited stdio and waits for it.

##### Errors

`NotFound` if the program doesn't exist, `PermissionDenied` if it
isn't executable, or a timeout error if `timeout` was set and hit.
A program that runs and fails is still `Ok`; check `isSuccess`.

_Defined in `lang/std/system/process.ks`._

#### function `shell`

```kestrel
public static func shell(String) -> Command
```

A command that runs `script` with `/bin/sh -c`. The shell parses the
script, so quote anything that comes from outside the program.

_Defined in `lang/std/system/process.ks`._

#### function `spawn`

```kestrel
func spawn(Stdio, Stdio, Stdio) -> Result[Child, IoError]
```

Spawns with each stream's explicit setting, or the given default.

_Defined in `lang/std/system/process.ks`._

#### function `standardError`

```kestrel
public mutating func standardError(Stdio) -> &mutating Command
```

Sets the child's standard error.

_Defined in `lang/std/system/process.ks`._

#### function `standardInput`

```kestrel
public mutating func standardInput(Stdio) -> &mutating Command
```

Sets the child's standard input.

_Defined in `lang/std/system/process.ks`._

#### function `standardOutput`

```kestrel
public mutating func standardOutput(Stdio) -> &mutating Command
```

Sets the child's standard output.

_Defined in `lang/std/system/process.ks`._

#### function `start`

```kestrel
public func start() -> Result[Child, IoError]
```

Starts the command and returns without waiting. Stdio is inherited
unless set; use `Stdio.Pipe` to talk to the child.

_Defined in `lang/std/system/process.ks`._

#### function `timeout`

```kestrel
public mutating func timeout(milliseconds: Int64) -> &mutating Command
```

Kills the child if `run` or `output` is still waiting on it after
`milliseconds`; the call then fails with `TimedOut`.

_Defined in `lang/std/system/process.ks`._

#### field `timeoutMilliseconds`

```kestrel
var timeoutMilliseconds: Optional[Int64]
```

_Defined in `lang/std/system/process.ks`._

#### field `workingDirectory`

```kestrel
var workingDirectory: Optional[String]
```

_Defined in `lang/std/system/process.ks`._

## enum `Endianness`

```kestrel
public enum Endianness
```

Byte order of multi-byte integers in memory.

_Defined in `lang/std/system/platform.ks`._

### Members

#### case `Big`

```kestrel
case Big
```

_Defined in `lang/std/system/platform.ks`._

#### case `Little`

```kestrel
case Little
```

_Defined in `lang/std/system/platform.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Endianness) -> Bool
```

_Defined in `lang/std/system/platform.ks`._

## enum `Environment`

```kestrel
public enum Environment
```

Environment variables and the process's working, home and temporary
directories.

A caseless enum used as a namespace: every member is `static`.

### Examples

```
if let .Some(home) = Environment.variable("HOME") {
    print(home)
}
let cwd = try Environment.currentDirectory();
```

_Defined in `lang/std/system/environment.ks`._

### Members

#### function `currentDirectory`

```kestrel
public static func currentDirectory() -> Result[String, IoError]
```

The process's current working directory, as an absolute path.

##### Errors

`NotFound` if the directory was deleted; `PermissionDenied` if a
parent can't be read.

_Defined in `lang/std/system/environment.ks`._

#### function `hasVariable`

```kestrel
public static func hasVariable(String) -> Bool
```

True if variable `name` is set, even to the empty string.

_Defined in `lang/std/system/environment.ks`._

#### function `homeDirectory`

```kestrel
public static func homeDirectory() -> Optional[String]
```

The current user's home directory: `$HOME` if set and non-empty,
else the password database entry. `None` if neither has one.

_Defined in `lang/std/system/environment.ks`._

#### function `removeVariable`

```kestrel
public static func removeVariable(String) -> Result[(), IoError]
```

Removes variable `name`. Removing an unset variable succeeds.

##### Safety

Not thread-safe, for the same reason as `setVariable`.

_Defined in `lang/std/system/environment.ks`._

#### function `setCurrentDirectory`

```kestrel
public static func setCurrentDirectory(to: String) -> Result[(), IoError]
```

Changes the process's working directory. Affects every thread.

_Defined in `lang/std/system/environment.ks`._

#### function `setVariable`

```kestrel
public static func setVariable(String, to: String) -> Result[(), IoError]
```

Sets variable `name` to `value`, replacing any old value.

##### Safety

Not thread-safe: C code on another thread that reads the environment
(`getenv`, DNS lookups, locale setup) can crash while this runs. Call
it before starting threads. To give a child process a different
environment, use `Command.environment(_:to:)` instead. This will
become an `unsafe` function once the keyword exists.

##### Errors

`InvalidInput` if `name` is empty or contains `=`.

_Defined in `lang/std/system/environment.ks`._

#### function `temporaryDirectory`

```kestrel
public static func temporaryDirectory() -> String
```

The directory for temporary files: `$TMPDIR` if set and non-empty,
else `/tmp`. Not checked for existence.

_Defined in `lang/std/system/environment.ks`._

#### function `variable`

```kestrel
public static func variable(String) -> Optional[String]
```

The value of variable `name`, or `None` if it is unset or its value
is not valid UTF-8. Use `variableBytes` for values that may not be text.

The value is copied out, so it stays valid after later `setVariable` calls.

_Defined in `lang/std/system/environment.ks`._

#### function `variableBytes`

```kestrel
public static func variableBytes(String) -> Optional[Array[UInt8]]
```

The raw bytes of variable `name`, or `None` if it is unset.

_Defined in `lang/std/system/environment.ks`._

#### function `variables`

```kestrel
public static func variables() -> Array[(String, String)]
```

A snapshot of every variable as `(name, value)` pairs, in the order
the C runtime stores them. Entries that aren't valid UTF-8 are
skipped.

_Defined in `lang/std/system/environment.ks`._

## enum `ExitStatus`

```kestrel
public enum ExitStatus
```

How a child process ended.

_Defined in `lang/std/system/process.ks`._

### Members

#### case `Exited`

```kestrel
case Exited(ExitCode)
```

It returned from `main` or called `exit` with this code.

_Defined in `lang/std/system/process.ks`._

#### case `Terminated`

```kestrel
case Terminated
```

It was ended from outside, by a signal such as `SIGKILL`.

_Defined in `lang/std/system/process.ks`._

#### field `code`

```kestrel
public var code: Optional[ExitCode] { get }
```

The exit code, or `None` if it was terminated.

_Defined in `lang/std/system/process.ks`._

#### field `isSuccess`

```kestrel
public var isSuccess: Bool { get }
```

True if it exited with code 0.

_Defined in `lang/std/system/process.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: ExitStatus) -> Bool
```

_Defined in `lang/std/system/process.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

Prints `exit code 3` or `terminated`.

_Defined in `lang/std/system/process.ks`._

## enum `Machine`

```kestrel
public enum Machine
```

The machine the process is running on.

_Defined in `lang/std/system/machine.ks`._

### Members

#### function `hostName`

```kestrel
public static func hostName() -> Result[String, IoError]
```

The machine's host name.

_Defined in `lang/std/system/machine.ks`._

#### field `pageSize`

```kestrel
public static var pageSize: Int64 { get }
```

The memory page size in bytes.

_Defined in `lang/std/system/machine.ks`._

#### field `processorCount`

```kestrel
public static var processorCount: Int64 { get }
```

Processors this process may run on, at least 1. On Linux this is the
CPU affinity mask, so it respects `taskset` and container CPU sets;
it does not read cgroup CPU quotas.

_Defined in `lang/std/system/machine.ks`._

## enum `OperatingSystem`

```kestrel
public enum OperatingSystem
```

An operating system Kestrel can target. Case names match the
`@platform(.Darwin)` / `@platform(.Linux)` attribute.

_Defined in `lang/std/system/platform.ks`._

### Members

#### case `Darwin`

```kestrel
case Darwin
```

macOS, iOS and the other Apple systems.

_Defined in `lang/std/system/platform.ks`._

#### case `Linux`

```kestrel
case Linux
```

Linux, with any C library.

_Defined in `lang/std/system/platform.ks`._

#### field `name`

```kestrel
public var name: String { get }
```

The lowercase name used in target triples: `darwin`, `linux`.

_Defined in `lang/std/system/platform.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: OperatingSystem) -> Bool
```

_Defined in `lang/std/system/platform.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

_Defined in `lang/std/system/platform.ks`._

## struct `Output`

```kestrel
public struct Output: not Copyable { /* private fields */ }
```

What a finished child printed, and how it ended.

_Defined in `lang/std/system/process.ks`._

### Members

#### field `standardError`

```kestrel
public var standardError: Array[UInt8]
```

_Defined in `lang/std/system/process.ks`._

#### function `standardErrorText`

```kestrel
public func standardErrorText() -> Optional[String]
```

Standard error as text, or `None` if it isn't valid UTF-8.

_Defined in `lang/std/system/process.ks`._

#### field `standardOutput`

```kestrel
public var standardOutput: Array[UInt8]
```

_Defined in `lang/std/system/process.ks`._

#### function `standardOutputText`

```kestrel
public func standardOutputText() -> Optional[String]
```

Standard output as text, or `None` if it isn't valid UTF-8.

_Defined in `lang/std/system/process.ks`._

#### field `status`

```kestrel
public var status: ExitStatus
```

_Defined in `lang/std/system/process.ks`._

## struct `PipeReader`

```kestrel
public struct PipeReader: not Copyable { /* private fields */ }
```

The read end of a pipe from a child: its standard output or error.
Closed on drop.

_Defined in `lang/std/system/process.ks`._

### Members

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/system/process.ks`._

### Implements `Readable`

#### function `read`

```kestrel
public mutating func read(into: mutating Slice[UInt8]) -> Result[Int64, IoError]
```

_Defined in `lang/std/system/process.ks`._

## struct `PipeWriter`

```kestrel
public struct PipeWriter: not Copyable { /* private fields */ }
```

The write end of a pipe to a child's standard input. Drop it to send
end-of-file.

_Defined in `lang/std/system/process.ks`._

### Members

#### field `fd`

```kestrel
var fd: Int32
```

_Defined in `lang/std/system/process.ks`._

### Implements `Writable`

#### function `flush`

```kestrel
public mutating func flush() -> Result[(), IoError]
```

No-op: pipe writes are unbuffered.

_Defined in `lang/std/system/process.ks`._

#### function `write`

```kestrel
public mutating func write(from: UnsafeSlice[UInt8]) -> Result[Int64, IoError]
```

_Defined in `lang/std/system/process.ks`._

## struct `Platform`

```kestrel
public struct Platform { /* private fields */ }
```

The target this program was compiled for: OS, CPU, pointer width and
byte order.

`Platform.current` describes the build target, not a guess made at run
time, so an x86_64 binary under Rosetta reports `X86_64`.

### Examples

```
let p = Platform.current;
let asset = "tool-\(p.operatingSystem)-\(p.architecture).tar.gz";
```

_Defined in `lang/std/system/platform.ks`._

### Members

#### field `architecture`

```kestrel
public var architecture: Architecture
```

_Defined in `lang/std/system/platform.ks`._

#### field `current`

```kestrel
public static var current: Platform { get }
```

The platform this program was compiled for.

_Defined in `lang/std/system/platform.ks`._

#### field `endianness`

```kestrel
public var endianness: Endianness
```

_Defined in `lang/std/system/platform.ks`._

#### field `operatingSystem`

```kestrel
public var operatingSystem: OperatingSystem
```

_Defined in `lang/std/system/platform.ks`._

#### field `pointerWidth`

```kestrel
public var pointerWidth: Int64
```

Bits in a pointer: 32 or 64.

_Defined in `lang/std/system/platform.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: Platform) -> Bool
```

_Defined in `lang/std/system/platform.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

Prints `<architecture>-<os>`, for example `arm64-darwin`.

_Defined in `lang/std/system/platform.ks`._

## enum `Process`

```kestrel
public enum Process
```

The running process.

_Defined in `lang/std/system/process.ks`._

### Members

#### function `arguments`

```kestrel
public static func arguments() -> Array[String]
```

The raw command-line arguments, `argv[0]` (the program name)
included. Arguments that aren't valid UTF-8 are decoded lossily.

This is the unparsed list; a parsing library such as clutch builds
on it.

_Defined in `lang/std/system/process.ks`._

#### function `executablePath`

```kestrel
public static func executablePath() -> Result[String, IoError]
```

The absolute path of the running executable, with symbolic links
resolved on Linux.

_Defined in `lang/std/system/process.ks`._

#### field `id`

```kestrel
public static var id: ProcessId { get }
```

This process's ID.

_Defined in `lang/std/system/process.ks`._

#### field `parentId`

```kestrel
public static var parentId: ProcessId { get }
```

The parent process's ID.

_Defined in `lang/std/system/process.ks`._

## struct `ProcessId`

```kestrel
public struct ProcessId { /* private fields */ }
```

A process ID.

_Defined in `lang/std/system/process.ks`._

### Members

#### field `value`

```kestrel
public var value: Int64
```

_Defined in `lang/std/system/process.ks`._

### Implements `Equatable`

#### function `isEqual`

```kestrel
public func isEqual(to: ProcessId) -> Bool
```

_Defined in `lang/std/system/process.ks`._

### Implements `Formattable`

#### function `format`

```kestrel
public func format(into: mutating some Formatter, FormatOptions)
```

_Defined in `lang/std/system/process.ks`._

## enum `Stdio`

```kestrel
public enum Stdio
```

Where a child's standard input, output or error goes.

_Defined in `lang/std/system/process.ks`._

### Members

#### case `Inherit`

```kestrel
case Inherit
```

Share the parent's stream. The default for `run` and `start`.

_Defined in `lang/std/system/process.ks`._

#### case `Null`

```kestrel
case Null
```

Connect `/dev/null`.

_Defined in `lang/std/system/process.ks`._

#### case `Pipe`

```kestrel
case Pipe
```

Connect a pipe the parent reads or writes through `Child`.

_Defined in `lang/std/system/process.ks`._

#### function `nativeMode`

```kestrel
func nativeMode() -> Int32
```

_Defined in `lang/std/system/process.ks`._

## function `abort`

```kestrel
public func abort() -> !
```

Ends the process immediately with `SIGABRT`, without running `atexit`
handlers or flushing anything. For states where continuing, or even
cleaning up, could do harm. Use `exit` for ordinary failures.

_Defined in `lang/std/system/exit.ks`._

## function `exit`

```kestrel
public func exit(ExitCode) -> !
```

Ends the process with `code`, the same type `@main` returns.

Runs `atexit` handlers and flushes C stdio buffers. Kestrel values still
in scope are **not** deinitialized, so flush any buffered writers first.

### Examples

```
if not configFound {
    eprintln("no config file");
    exit(2)
}
```

_Defined in `lang/std/system/exit.ks`._

## function `run`

```kestrel
public func run(String, Array[String]) -> Result[ExitStatus, IoError]
```

Runs `program` with `args`, inheriting stdio, and waits for it. Short
for `Command(program).arguments(args).run()`.

### Examples

```
let status = try run("git", ["fetch", "origin"]);
```

_Defined in `lang/std/system/process.ks`._

