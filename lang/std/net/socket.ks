// TCP socket types

module std.net.socket

import std.num.(Int64, Int32, UInt8, UInt16)
import std.result.(Result)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.core.(Bool)
import std.net.libc
import std.io.error.(Error)
import std.io.read.(Read)
import std.io.write.(Write)

public struct TcpStream: Read, Write {
    var fd: Int32

    public init(fd: Int32) {
        self.fd = fd
    }

    public mutating func read(into buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.recv(self.fd, buf.pointer, buf.count, 0);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    public mutating func write(from buf: Slice[UInt8]) -> Result[Int64, Error] {
        let n = libc.send(self.fd, buf.pointer, buf.count, 0);
        if n < 0 {
            return .Err(Error.last())
        }
        .Ok(n)
    }

    public mutating func flush() -> Result[(), Error] {
        .Ok(())
    }

    public func rawFd() -> Int32 {
        self.fd
    }

    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}

public struct TcpListener {
    var fd: Int32

    init(fd: Int32) {
        self.fd = fd
    }

    public static func bind(port: UInt16) -> Result[TcpListener, Error] {
        let fd = libc.socket(libc.AF_INET(), libc.SOCK_STREAM(), libc.IPPROTO_TCP());
        if fd < 0 {
            return .Err(Error.last())
        }

        // Set SO_REUSEADDR
        var optval: Int32 = 1;
        let optPtr = Pointer(to: optval).cast[UInt8]();
        let optResult = libc.setsockopt(fd, libc.SOL_SOCKET(), libc.SO_REUSEADDR(), optPtr, 4);
        if optResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        // Build sockaddr_in (macOS layout, 16 bytes)
        var addr = Array[UInt8]();
        // sin_len = 16, sin_family = 2
        addr.append(16);
        addr.append(2);
        // sin_port in network byte order (big-endian)
        let port64 = Int64(from: port);
        let portHi = port64 / 256;
        let portLo = port64 % 256;
        let hi = UInt8(from: portHi);
        let lo = UInt8(from: portLo);
        addr.append(hi);
        addr.append(lo);
        // sin_addr = INADDR_ANY + zero padding (12 bytes)
        var pad: Int64 = 0;
        while pad < 12 {
            addr.append(0);
            pad = pad + 1
        }

        let bindResult = libc.bind(fd, addr.asPointer(), libc.SOCKADDR_IN_SIZE());
        if bindResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        let listenResult = libc.listen(fd, 128);
        if listenResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        .Ok(TcpListener(fd))
    }

    public func accept() -> Result[TcpStream, Error] {
        let clientFd = libc.accept(self.fd, Pointer[UInt8].nullPointer(), Pointer[Int32].nullPointer());
        if clientFd < 0 {
            return .Err(Error.last())
        }
        .Ok(TcpStream(clientFd))
    }

    public func rawFd() -> Int32 {
        self.fd
    }

    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}
