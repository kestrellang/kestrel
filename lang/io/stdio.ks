// Standard I/O streams

module io.stdio

import std.num.(Int64, UInt8)
import std.result.(Result, Optional)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import io.libc
import io.error.(Error)
import io.read.(Read)
import io.write.(Write, writeStr, writeByte, writeLine)

// Stdin - standard input
public struct Stdin: Read {
    public init() {}

    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.read(libc.STDIN(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }
}

// Stdout - standard output
public struct Stdout: Write {
    public init() {}

    public func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.write(libc.STDOUT(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    public func flush() -> Result[(), Error] {
        .Ok(())
    }
}

// Stderr - standard error
public struct Stderr: Write {
    public init() {}

    public func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.write(libc.STDERR(), buf.pointer, buf.count);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    public func flush() -> Result[(), Error] {
        .Ok(())
    }
}

// Get stdin handle
public func stdin() -> Stdin {
    Stdin()
}

// Get stdout handle
public func stdout() -> Stdout {
    Stdout()
}

// Get stderr handle
public func stderr() -> Stderr {
    Stderr()
}

// Print string to stdout (no newline)
public func print(s: String) -> Result[(), Error] {
    var out = stdout();
    writeStr(out, s)
}

// Print string to stdout with newline
public func println(s: String) -> Result[(), Error] {
    var out = stdout();
    writeLine(out, s)
}

// Print empty line
public func printlnEmpty() -> Result[(), Error] {
    var out = stdout();
    writeByte(out, 10)
}

// Print to stderr (no newline)
public func eprint(s: String) -> Result[(), Error] {
    var err = stderr();
    writeStr(err, s)
}

// Print to stderr with newline
public func eprintln(s: String) -> Result[(), Error] {
    var err = stderr();
    writeLine(err, s)
}

// Read line from stdin
public func readLine() -> Result[String, Error] {
    var input = stdin();
    var bytes = Array[UInt8]();

    var done: Bool = false;
    while done == false {
        var buf = Array[UInt8](capacity: 1);
        buf.append(0);
        let slice = Slice(pointer: buf.pointer(), count: 1);
        let n = try input.read(into: slice);
        if n == 0 {
            done = true  // EOF
        } else {
            let b = buf.getUnchecked(0);
            if b == 10 {  // newline
                done = true
            } else {
                bytes.append(b)
            }
        }
    }

    // Strip trailing \r if present (Windows line endings)
    let count = bytes.count();
    if count > 0 {
        let lastByte = bytes.getUnchecked(count - 1);
        if lastByte == 13 {
            let _ = bytes.pop();
        }
    }

    // Build string from bytes (inefficient but works)
    // TODO: Add proper String.fromUtf8Bytes()
    var result = "";
    .Ok(result)
}

// Prompt and read line
public func prompt(message: String) -> Result[String, Error] {
    try print(s: message);
    var out = stdout();
    try out.flush();
    readLine()
}
