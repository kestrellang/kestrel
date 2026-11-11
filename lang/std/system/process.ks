// This process, and starting and waiting on child processes.

module std.system

import std.numeric.(Int32, Int64, UInt8)
import std.memory.(RawPointer, UnsafeSlice)
import std.collections.(Array, Slice)
import std.text.(String, Formatter, Formattable, FormatOptions, _writePadded)
import std.core.(Bool, Equatable)
import std.result.(Optional, Result)
import std.streams.(IoError, Readable, Writable, flushStandardOutput)
import std.ffi.(malloc, free)
import std.time.(Duration)
import std.files.(Path)

// ============================================================================
// THIS PROCESS
// ============================================================================

/// A process ID.
public struct ProcessId: Equatable, Formattable {
    public var value: Int64

    public func isEqual(to other: ProcessId) -> Bool { self.value == other.value }

    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        _writePadded(into: writer, "\(self.value)", options)
    }
}

/// The running process.
public enum Process {
    /// This process's ID.
    public static var id: ProcessId { ProcessId(value: Int64(from: unsafe native_getpid())) }

    /// The parent process's ID.
    public static var parentId: ProcessId { ProcessId(value: Int64(from: unsafe native_getppid())) }

    /// The absolute path of the running executable, with symbolic links
    /// resolved on Linux.
    public static func executablePath() -> Result[Path, IoError] {
        let size: Int64 = 4096;
        let buf = unsafe malloc(size + 1);
        let length = unsafe native_executable_path(buf, size);
        if length < 0 {
            unsafe free(buf);
            return .Err(IoError.last())
        }
        let bytes = buf.cast[UInt8]();
        var path = Array[UInt8](capacity: length);
        var i: Int64 = 0;
        while i < length {
            path.append(unsafe bytes.offset(by: i).read());
            i = i + 1
        }
        unsafe free(buf);
        .Ok(Path(bytes: path))
    }

    /// The raw command-line arguments, `argv[0]` (the program name)
    /// included. Arguments that aren't valid UTF-8 are decoded lossily.
    ///
    /// This is the unparsed list; a parsing library such as clutch builds
    /// on it.
    public static func arguments() -> Array[String] {
        var result = Array[String]();
        let argv = unsafe native_argv();
        if argv.isNull {
            return result
        }
        let count = Int64(from: unsafe native_argc());
        for i in 0..<count {
            let bytes = bytesFromNative(nativePointerAt(argv, i));
            result.append(String(fromUtf8Lossy: bytes.asSlice()))
        }
        result
    }
}

// ============================================================================
// EXIT STATUS AND OUTPUT
// ============================================================================

/// How a child process ended.
public enum ExitStatus: Equatable, Formattable {
    /// It returned from `main` or called `exit` with this code.
    case Exited(ExitCode)
    /// It was ended by a signal: `signal` is its number (9 for `SIGKILL`,
    /// 11 for `SIGSEGV`, 2 for `SIGINT`), so a crash and a kill are told
    /// apart.
    case Terminated(signal: Int32)
    /// It outlived the `Command`'s `timeout` and std killed it. Not an
    /// error: `output()` still returns whatever the child wrote before.
    case TimedOut

    /// True if it exited with code 0.
    public var isSuccess: Bool {
        match self {
            .Exited(code) => code.isSuccess,
            .Terminated(_) => false,
            .TimedOut => false
        }
    }

    /// The exit code, or `None` if it was terminated or timed out.
    public var code: Optional[ExitCode] {
        match self {
            .Exited(code) => .Some(code),
            .Terminated(_) => .None,
            .TimedOut => .None
        }
    }

    public func isEqual(to other: ExitStatus) -> Bool {
        match (self, other) {
            (.Exited(a), .Exited(b)) => a.isEqual(to: b),
            (.Terminated(a), .Terminated(b)) => a == b,
            (.TimedOut, .TimedOut) => true,
            _ => false
        }
    }

    /// Prints `exit code 3`, `terminated by signal 9` or `timed out`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        let text: String = match self {
            .Exited(code) => "exit code \(code.value)",
            .Terminated(signal) => "terminated by signal \(signal)",
            .TimedOut => "timed out"
        };
        _writePadded(into: writer, text, options)
    }
}

/// What a finished child printed, and how it ended.
public struct Output: not Copyable {
    public var status: ExitStatus
    public var standardOutput: Array[UInt8]
    public var standardError: Array[UInt8]

    /// Standard output as text, or `None` if it isn't valid UTF-8.
    public func standardOutputText() -> Optional[String] {
        String(fromUtf8: self.standardOutput.asSlice())
    }

    /// Standard error as text, or `None` if it isn't valid UTF-8.
    public func standardErrorText() -> Optional[String] {
        String(fromUtf8: self.standardError.asSlice())
    }
}

// ============================================================================
// PIPES
// ============================================================================

/// The read end of a pipe from a child: its standard output or error.
/// Closed on drop.
public struct PipeReader: Readable, not Copyable {
    var fd: Int32

    /// Reads what the child has written so far; `0` once it closes its end.
    public mutating func read(mutating into buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_read(self.fd, buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    deinit {
        if self.fd >= 0 {
            unsafe native_close(self.fd);
        }
    }
}

/// The write end of a pipe to a child's standard input. Drop it to send
/// end-of-file.
public struct PipeWriter: Writable, not Copyable {
    var fd: Int32

    /// Writes to the child's standard input.
    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, IoError] {
        let n = unsafe native_write(self.fd, buf.pointer.asRaw(), buf.count);
        if n < 0 {
            return .Err(IoError.last())
        }
        .Ok(n)
    }

    /// Nothing to flush: pipe writes are unbuffered.
    public mutating func flush() -> Result[(), IoError] { .Ok(()) }

    deinit {
        if self.fd >= 0 {
            unsafe native_close(self.fd);
        }
    }
}

// ============================================================================
// COMMAND
// ============================================================================

/// Where a child's standard input, output or error goes.
public enum Stdio {
    /// Share the parent's stream. The default for `run` and `start`.
    case Inherit
    /// Connect a pipe the parent reads or writes through `Child`.
    case Pipe
    /// Connect `/dev/null`.
    case Null
    /// Connect the file at this path: standard input reads it, and
    /// standard output or error writes it, creating it if missing and
    /// truncating it otherwise.
    case File(Path)

    func nativeMode() -> Int32 {
        match self {
            .Inherit => 0,
            .Pipe => 1,
            .Null => 2,
            .File(_) => 3
        }
    }
}

/// A stream's mode for `kestrel_spawn`: its explicit setting, or `fallback`.
func nativeMode(of mode: Optional[Stdio], or fallback: Stdio) -> Int32 {
    match mode {
        .Some(m) => m.nativeMode(),
        .None => fallback.nativeMode()
    }
}

/// A `.File` stream's path as a C string the caller frees; null otherwise.
/// `Command.hasNulByte` has already ruled out the one failure.
func nativePath(of mode: Optional[Stdio]) -> RawPointer {
    match mode {
        .Some(.File(path)) => path.toNativePath().unwrap().raw.asRaw(),
        _ => RawPointer.nullPointer()
    }
}

/// `true` when `mode` is a `.File` whose path holds a NUL byte.
func hasNulByte(in mode: Optional[Stdio]) -> Bool {
    match mode {
        .Some(.File(path)) => path.bytes.contains(0),
        _ => false
    }
}

/// A program to run, with its arguments and environment.
///
/// Arguments are passed as a list straight to the program, with no shell in
/// between, so spaces and quotes in them need no escaping. Use
/// `Command.shell` when you want a shell.
///
/// Builder methods return the command so calls chain.
///
/// # Examples
///
/// ```
/// let status = try Command("tar").arguments(["-czf", out, "-C", dir, "."]).run();
///
/// let result = try Command("git").arguments(["rev-parse", "HEAD"]).output();
/// if result.status.isSuccess {
///     print(result.standardOutputText() ?? "")
/// }
/// ```
public struct Command: not Copyable {
    var program: String
    var argumentList: Array[String]
    var environmentChanges: Array[(String, Optional[String])]
    var clearsEnvironment: Bool
    var workingDirectory: Optional[Path]
    var inputMode: Optional[Stdio]
    var outputMode: Optional[Stdio]
    var errorMode: Optional[Stdio]
    var timeoutMilliseconds: Optional[Int64]

    /// A command running `program`. A name without `/` is looked up on
    /// `PATH`; anything with a `/` is used as a path.
    public init(program: String) {
        self.program = program;
        self.argumentList = Array[String]();
        self.environmentChanges = Array[(String, Optional[String])]();
        self.clearsEnvironment = false;
        self.workingDirectory = .None;
        self.inputMode = .None;
        self.outputMode = .None;
        self.errorMode = .None;
        self.timeoutMilliseconds = .None
    }

    /// A command that runs `script` with `/bin/sh -c`. The shell parses the
    /// script, so quote anything that comes from outside the program.
    public static func shell(script: String) -> Command {
        var command = Command("/bin/sh");
        command.argumentList.append("-c");
        command.argumentList.append(script);
        command
    }

    /// The path `program` resolves to on `PATH`, or `None` if no executable
    /// by that name is found. A name containing `/` is returned as-is when
    /// that file is executable.
    public static func find(program: String) -> Optional[String] {
        if program.contains("/") {
            return if isExecutable(program) { .Some(program) } else { .None }
        }
        let path = Environment.variable("PATH") ?? "/usr/bin:/bin";
        for directory in path.split(by: ":") {
            let dir = if directory.isEmpty { "." } else { directory.toOwned() };
            let candidate = "\(dir)/\(program)";
            if isExecutable(candidate) {
                return .Some(candidate)
            }
        }
        .None
    }

    /// Adds one argument.
    public mutating func argument(arg: String) -> &mutating Command {
        self.argumentList.append(arg);
        self
    }

    /// Adds each of `args`, in order.
    public mutating func arguments(args: Array[String]) -> &mutating Command {
        for arg in args {
            self.argumentList.append(arg.clone())
        }
        self
    }

    /// Sets variable `name` for the child only.
    public mutating func environment(name: String, to value: String) -> &mutating Command {
        self.environmentChanges.append((name, .Some(value)));
        self
    }

    /// Unsets variable `name` for the child only.
    public mutating func removeEnvironment(name: String) -> &mutating Command {
        self.environmentChanges.append((name, .None));
        self
    }

    /// Starts the child with no inherited variables; only those set with
    /// `environment(_:to:)` reach it.
    public mutating func clearEnvironment() -> &mutating Command {
        self.clearsEnvironment = true;
        self.environmentChanges.clear();
        self
    }

    /// Runs the child in `path` instead of the current directory.
    public mutating func directory(path: Path) -> &mutating Command {
        self.workingDirectory = .Some(path);
        self
    }

    /// Sets the child's standard input.
    public mutating func standardInput(stdio: Stdio) -> &mutating Command {
        self.inputMode = .Some(stdio);
        self
    }

    /// Sets the child's standard output.
    public mutating func standardOutput(stdio: Stdio) -> &mutating Command {
        self.outputMode = .Some(stdio);
        self
    }

    /// Sets the child's standard error.
    public mutating func standardError(stdio: Stdio) -> &mutating Command {
        self.errorMode = .Some(stdio);
        self
    }

    /// Kills the child if `run` or `output` is still waiting on it after
    /// `duration`, as in `.timeout(.seconds(5))`. The call still succeeds,
    /// with status `.TimedOut`; `output()` keeps what the child wrote before
    /// it was killed.
    public mutating func timeout(duration: Duration) -> &mutating Command {
        self.timeoutMilliseconds = .Some(duration.timeoutMilliseconds);
        self
    }

    /// Runs the command with inherited stdio and waits for it.
    ///
    /// # Errors
    ///
    /// `NotFound` if the program doesn't exist, or `PermissionDenied` if it
    /// isn't executable. A program that runs and fails, or outlives its
    /// `timeout` (status `.TimedOut`), is still `Ok`; check `isSuccess`.
    public func run() -> Result[ExitStatus, IoError] {
        var child = try self.spawn(Stdio.Inherit, Stdio.Inherit, Stdio.Inherit);
        child.finish(self.timeoutMilliseconds)
    }

    /// Runs the command, capturing standard output and error, and waits for
    /// it. Standard input is `/dev/null` unless set with `standardInput`.
    public func output() -> Result[Output, IoError] {
        self.output(limit: 0)
    }

    /// Like `output()`, but keeps at most `bytes` of each stream; the rest is
    /// read and discarded so the child doesn't block. `0` means no limit.
    public func output(limit bytes: Int64) -> Result[Output, IoError] {
        var child = try self.spawn(Stdio.Null, Stdio.Pipe, Stdio.Pipe);
        child.collectOutput(bytes, self.timeoutMilliseconds)
    }

    /// Starts the command and returns without waiting. Stdio is inherited
    /// unless set; use `Stdio.Pipe` to talk to the child.
    public func start() -> Result[Child, IoError] {
        self.spawn(Stdio.Inherit, Stdio.Inherit, Stdio.Inherit)
    }

    /// Spawns with each stream's explicit setting, or the given default.
    func spawn(defaultInput: Stdio, defaultOutput: Stdio, defaultError: Stdio) -> Result[Child, IoError] {
        // C would cut each of these at a NUL byte and run something else.
        if self.hasNulByte() {
            return .Err(IoError(kind: .InvalidInput))
        }
        var argv = Array[String](capacity: self.argumentList.count + 1);
        argv.append(self.program.clone());
        for arg in self.argumentList {
            argv.append(arg.clone())
        }
        let nativeArgv = NativeStringArray(from: argv);
        let nativeEnvironment = self.childEnvironment();
        let cwd = match self.workingDirectory {
            // `hasNulByte` already ruled out the one failure.
            .Some(dir) => dir.toNativePath().unwrap().raw.asRaw(),
            .None => RawPointer.nullPointer()
        };
        let modes = NativeInt32Cells(3);
        modes.write(nativeMode(of: self.inputMode, or: defaultInput), at: 0);
        modes.write(nativeMode(of: self.outputMode, or: defaultOutput), at: 1);
        modes.write(nativeMode(of: self.errorMode, or: defaultError), at: 2);
        // The defaults are never `.File`, so only explicit settings open one.
        var paths = Array[RawPointer](capacity: 3);
        paths.append(nativePath(of: self.inputMode));
        paths.append(nativePath(of: self.outputMode));
        paths.append(nativePath(of: self.errorMode));
        let pidCell = NativeInt32Cells(1);
        let fds = NativeInt32Cells(3);

        // Anything this process buffered for standard output goes out
        // before the child, which may share fd 1, writes its own.
        let _ = flushStandardOutput();
        let program = self.program.toCString();
        // Borrow, don't bind: binding would move the array into the arm and
        // free the strings before the spawn reads them.
        let envp = match nativeEnvironment.asRef() {
            .Some(env) => env.asNative(),
            .None => RawPointer.nullPointer()
        };
        let status = unsafe native_spawn(program.raw.asRaw(), nativeArgv.asNative(), envp, cwd,
                                  modes.asNative(), paths.asPointer().asRaw(), pidCell.asNative(), fds.asNative());
        program.free();
        if not cwd.isNull {
            unsafe free(cwd);
        }
        for path in paths {
            if not path.isNull {
                unsafe free(path);
            }
        }
        if status != 0 {
            return .Err(IoError(errno: status))
        }
        .Ok(Child(
            pid: pidCell.read(at: 0),
            status: .None,
            input: pipeWriter(fds.read(at: 0)),
            output: pipeReader(fds.read(at: 1)),
            error: pipeReader(fds.read(at: 2))
        ))
    }

    /// The child's `envp`, or `None` to inherit this process's unchanged.
    /// `true` when the program, an argument, the working directory or an
    /// environment change holds a NUL byte.
    func hasNulByte() -> Bool {
        if self.program.containsNulByte { return true }
        for arg in self.argumentList {
            if arg.containsNulByte { return true }
        }
        if let .Some(dir) = self.workingDirectory {
            if dir.bytes.contains(0) { return true }
        }
        if hasNulByte(in: self.inputMode) or hasNulByte(in: self.outputMode) or hasNulByte(in: self.errorMode) {
            return true
        }
        for change in self.environmentChanges {
            if change.0.containsNulByte { return true }
            if let .Some(value) = change.1 {
                if value.containsNulByte { return true }
            }
        }
        false
    }

    func childEnvironment() -> Optional[NativeStringArray] {
        if not self.clearsEnvironment and self.environmentChanges.isEmpty {
            return .None
        }
        var variables = if self.clearsEnvironment {
            Array[(String, String)]()
        } else {
            Environment.variables()
        };
        for change in self.environmentChanges {
            variables.retain(where: { not it.0.isEqual(to: change.0) });
            if let .Some(value) = change.1 {
                variables.append((change.0.clone(), value.clone()))
            }
        }
        var entries = Array[String](capacity: variables.count);
        for pair in variables {
            entries.append("\(pair.0)=\(pair.1)")
        }
        .Some(NativeStringArray(from: entries))
    }
}

/// Runs `program` with `args`, inheriting stdio, and waits for it. Short
/// for `Command(program).arguments(args).run()`.
///
/// # Examples
///
/// ```
/// let status = try run("git", ["fetch", "origin"]);
/// ```
public func run(program: String, args: Array[String]) -> Result[ExitStatus, IoError] {
    Command(program).arguments(args).run()
}

internal func pipeReader(fd: Int32) -> Optional[PipeReader] {
    if fd < 0 { .None } else { .Some(PipeReader(fd: fd)) }
}

internal func pipeWriter(fd: Int32) -> Optional[PipeWriter] {
    if fd < 0 { .None } else { .Some(PipeWriter(fd: fd)) }
}

@extern(.C, mangleName: "access")
internal func native_access(path: RawPointer, mode: Int32) -> Int32

/// True if `path` names a file this process may execute (`X_OK` is 1).
internal func isExecutable(path: String) -> Bool {
    if path.containsNulByte { return false }
    let cpath = path.toCString();
    let result = unsafe native_access(cpath.raw.asRaw(), 1);
    cpath.free();
    result == 0
}

// ============================================================================
// CHILD
// ============================================================================

/// A started child process.
///
/// Dropping a `Child` that hasn't been waited on closes its pipes and then
/// waits for it, so it never lingers as a zombie. Call `kill` first if it
/// might not exit by itself.
public struct Child: not Copyable {
    var pid: Int32
    var status: Optional[ExitStatus]
    var input: Optional[PipeWriter]
    var output: Optional[PipeReader]
    var error: Optional[PipeReader]

    /// The child's process ID.
    public var id: ProcessId { ProcessId(value: Int64(from: self.pid)) }

    /// Takes the pipe to the child's standard input, if it was started with
    /// `Stdio.Pipe`. Later calls return `None`.
    public mutating func takeStandardInput() -> Optional[PipeWriter] { self.input.take() }

    /// Takes the pipe from the child's standard output, if piped.
    public mutating func takeStandardOutput() -> Optional[PipeReader] { self.output.take() }

    /// Takes the pipe from the child's standard error, if piped.
    public mutating func takeStandardError() -> Optional[PipeReader] { self.error.take() }

    /// Waits for the child to exit. Closes its standard input first, so a
    /// child reading to end-of-file can finish.
    public mutating func wait() -> Result[ExitStatus, IoError] {
        self.input = .None;
        match try self.reap(blocking: true) {
            .Some(status) => .Ok(status),
            .None => .Err(IoError(kind: .Interrupted))
        }
    }

    /// The exit status if the child has already exited, else `None`.
    /// Never blocks.
    public mutating func tryWait() -> Result[Optional[ExitStatus], IoError] {
        self.reap(blocking: false)
    }

    /// Waits up to `duration` for the child to exit, as in
    /// `child.wait(for: .seconds(1))`. `None` if it is still running when
    /// the time is up; the child is left running.
    public mutating func wait(for duration: Duration) -> Result[Optional[ExitStatus], IoError] {
        self.waitMilliseconds(duration.timeoutMilliseconds)
    }

    /// `wait(for:)` in whole milliseconds, for std's own deadlines.
    mutating func waitMilliseconds(milliseconds: Int64) -> Result[Optional[ExitStatus], IoError] {
        self.input = .None;
        let deadline = unsafe native_monotonic_ms() + milliseconds;
        var pause: Int32 = 1;
        while true {
            if let .Some(status) = try self.reap(blocking: false) {
                return .Ok(.Some(status))
            }
            if unsafe native_monotonic_ms() >= deadline {
                return .Ok(.None)
            }
            unsafe native_sleep_ms(pause);
            if pause < 20 {
                pause = pause * 2
            }
        }
        .Ok(.None)
    }

    /// Reads all of standard output and error (whichever were piped) and
    /// waits for the child to exit.
    public mutating func waitForOutput() -> Result[Output, IoError] {
        self.collectOutput(0, .None)
    }

    /// Asks the child to stop (`SIGTERM`). It may clean up, or ignore it.
    public mutating func terminate() -> Result[(), IoError] { self.signal(15) }

    /// Stops the child at once (`SIGKILL`). It can't be caught or ignored.
    public mutating func kill() -> Result[(), IoError] { self.signal(9) }

    mutating func signal(number: Int32) -> Result[(), IoError] {
        if self.status.isSome {
            return .Ok(())
        }
        if unsafe native_kill(self.pid, number) != 0 {
            return .Err(IoError.last())
        }
        .Ok(())
    }

    /// One `waitpid`. Caches the status, since a pid can only be reaped once.
    mutating func reap(blocking blocking: Bool) -> Result[Optional[ExitStatus], IoError] {
        if let .Some(status) = self.status {
            return .Ok(.Some(status))
        }
        let cells = NativeInt32Cells(2);
        let r = unsafe native_wait(self.pid, if blocking { 0 } else { 1 }, cells.asNative(),
                            cells.pointer.offset(by: 1).asRaw());
        if r < 0 {
            return .Err(IoError.last())
        }
        if r == 0 {
            return .Ok(.None)
        }
        let status: ExitStatus = if cells.read(at: 0) == 1 {
            .Exited(ExitCode(UInt8(from: cells.read(at: 1))))
        } else {
            .Terminated(signal: cells.read(at: 1))
        };
        self.status = .Some(status);
        .Ok(.Some(status))
    }

    /// `wait()`, or a timed wait that kills the child when time runs out
    /// and reports `.TimedOut`.
    mutating func finish(timeoutMilliseconds: Optional[Int64]) -> Result[ExitStatus, IoError] {
        guard let .Some(limit) = timeoutMilliseconds else {
            return self.wait()
        }
        if let .Some(status) = try self.waitMilliseconds(limit) {
            return .Ok(status)
        }
        self.killForTimeout();
        .Ok(.TimedOut)
    }

    /// Kills and reaps a child that outlived its timeout.
    mutating func killForTimeout() {
        let _ = self.kill();
        let _ = self.wait();
    }

    /// Drains the piped output streams with `poll`, so a child filling one
    /// pipe can't deadlock against a parent blocked on the other.
    mutating func collectOutput(limit: Int64, timeoutMilliseconds: Optional[Int64]) -> Result[Output, IoError] {
        self.input = .None;
        // The pipes stay open until this returns; a closed stream's fd is
        // set to -1 so poll skips it.
        // A stream that wasn't piped stands in as a closed pipe (fd -1).
        var out = match self.output.take() {
            .Some(pipe) => pipe,
            .None => PipeReader(fd: -1)
        };
        var err = match self.error.take() {
            .Some(pipe) => pipe,
            .None => PipeReader(fd: -1)
        };
        var outFd = out.fd;
        var errFd = err.fd;
        var outBytes = Array[UInt8]();
        var errBytes = Array[UInt8]();
        var scratch = Array[UInt8](repeating: 0, count: 65536);
        let ready = NativeInt32Cells(2);
        let deadline = match timeoutMilliseconds {
            .Some(ms) => unsafe native_monotonic_ms() + ms,
            .None => -1
        };

        while outFd >= 0 or errFd >= 0 {
            var wait: Int32 = -1;
            if deadline >= 0 {
                let left = deadline - unsafe native_monotonic_ms();
                if left <= 0 {
                    // Out of time: keep what was read so far.
                    self.killForTimeout();
                    return .Ok(Output(status: .TimedOut, standardOutput: outBytes, standardError: errBytes))
                }
                wait = Int32(from: left)
            }
            if unsafe native_poll_two(outFd, errFd, wait, ready.asNative()) < 0 {
                return .Err(IoError.last())
            }
            if ready.read(at: 0) != 0 {
                let open = try drainOnce(out, scratch, outBytes, limit);
                if not open {
                    outFd = -1
                }
            }
            if ready.read(at: 1) != 0 {
                let open = try drainOnce(err, scratch, errBytes, limit);
                if not open {
                    errFd = -1
                }
            }
        }

        let status = match deadline {
            -1 => try self.wait(),
            _ => try self.finish(.Some(deadline - unsafe native_monotonic_ms()))
        };
        .Ok(Output(status: status, standardOutput: outBytes, standardError: errBytes))
    }

    deinit {
        self.input = .None;
        self.output = .None;
        self.error = .None;
        if self.status.isNone {
            let _ = self.reap(blocking: true);
        }
    }
}

/// One `read` from `pipe` into `into`, keeping at most `limit` bytes in
/// total (`0` = no limit). `false` at end-of-file.
internal func drainOnce[R](mutating pipe: R, mutating scratch: Array[UInt8], mutating into: Array[UInt8], limit: Int64) -> Result[Bool, IoError]
    where R: Readable, R: not Copyable, R: not Static {
    let n = match pipe.read(into: scratch.asMutableSlice()) {
        .Ok(n) => n,
        .Err(error) => {
            // A signal interrupted the read: poll again.
            if error.kind == .Interrupted {
                return .Ok(true)
            }
            return .Err(error)
        }
    };
    if n == 0 {
        return .Ok(false)
    }
    var keep = n;
    if limit > 0 {
        let room = limit - into.count;
        keep = if room < 0 { 0 } else if room < n { room } else { n }
    }
    if keep > 0 {
        into.append(contentsOf: scratch(0..<keep))
    }
    .Ok(true)
}
