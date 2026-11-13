/// Shell commands and directory listings for command-line tools, on
/// std.system and std.files.
///
/// Tools such as flock and jessup build commands as shell strings (`tar`,
/// `curl`, `cc`), so these wrap `Command.shell` with the two shapes they
/// need: an exit code, or the trimmed standard output.

module clutch.shell

/// Runs `script` with `/bin/sh -c`, sharing flock's stdio. Returns the exit
/// code, or -1 if the shell didn't start or was killed by a signal.
public func runShell(script: String) -> Int32 {
    match Command.shell(script).run() {
        .Ok(status) => match status.code {
            .Some(code) => Int32(from: code.value),
            .None => -1
        },
        .Err(_) => -1
    }
}

/// Runs `script` with `/bin/sh -c` and returns its standard output with
/// trailing whitespace trimmed; `""` if it didn't run. Standard error
/// passes through to flock's.
public func shellOutput(script: String) -> String {
    var command = Command.shell(script);
    command.standardOutput(.Pipe);
    let output = match command.start() {
        .Ok(child) => {
            var child = child;
            match child.waitForOutput() {
                .Ok(o) => o,
                .Err(_) => return ""
            }
        },
        .Err(_) => return ""
    };
    var text = output.standardOutputText().unwrap(or: "");
    text.trimEnd();
    text
}

/// The names in directory `path`, or none if it can't be read.
public func entryNames(path: String) -> Array[String] {
    var names = Array[String]();
    match Directory.entries(Path(path)) {
        .Ok(entries) => {
            for entry in entries {
                if let .Ok(e) = entry {
                    names.append(e.name)
                }
            }
        },
        .Err(_) => ()
    }
    names
}
