// I/O Error types

module io.error

import std.num.(Int32, Int64)
import std.text.(String)
import std.result.(Result)
import io.libc

// I/O Error
public struct Error {
    var code: Int32

    public init(code: Int32) {
        self.code = code
    }

    // Create from current errno
    public static func last() -> Error {
        Error(code: libc.errno())
    }

    public func description() -> String {
        // Convert to Int64 for match (integer literals default to Int64)
        let code64 = Int64(from: self.code);
        match code64 {
            1 => "operation not permitted",
            2 => "no such file or directory",
            4 => "interrupted",
            5 => "i/o error",
            9 => "bad file descriptor",
            11 => "would block",
            12 => "out of memory",
            13 => "permission denied",
            17 => "file exists",
            20 => "not a directory",
            21 => "is a directory",
            22 => "invalid argument",
            28 => "no space left",
            32 => "broken pipe",
            _ => "unknown error"
        }
    }

    public func errno() -> Int32 {
        self.code
    }
}

// Common error constructors
public func notFound() -> Error { Error(code: 2) }
public func permissionDenied() -> Error { Error(code: 13) }
public func alreadyExists() -> Error { Error(code: 17) }
public func invalidInput() -> Error { Error(code: 22) }
public func wouldBlock() -> Error { Error(code: 11) }
public func interrupted() -> Error { Error(code: 4) }
public func brokenPipe() -> Error { Error(code: 32) }
