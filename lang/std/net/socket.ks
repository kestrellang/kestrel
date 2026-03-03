// TCP socket types

module std.net.socket

import std.num.(Int64, Int32, UInt8, UInt16)
import std.result.(Result)
import std.memory.(Slice, Pointer)
import std.collections.(Array)
import std.text.(String)
import std.core.(Bool)
import std.net.libc
import std.io.error.(Error)
import std.io.read.(Read)
import std.io.write.(Write)

// addrinfo struct size and layout (platform-specific)
@platform(.darwin)
func ADDRINFO_SIZE() -> Int64 { 48 }

@platform(.linux)
func ADDRINFO_SIZE() -> Int64 { 48 }

@platform(.darwin)
func AI_ADDR_OFFSET() -> Int64 { 32 }

@platform(.linux)
func AI_ADDR_OFFSET() -> Int64 { 24 }

// Build sockaddr_in (platform-specific layout, 16 bytes)
// macOS has sin_len (1 byte) + sin_family (1 byte)
@platform(.darwin)
func buildSockaddrIn(port: UInt16) -> Array[UInt8] {
    var addr = Array[UInt8]();
    // sin_len = 16, sin_family = AF_INET (2)
    addr.append(16);
    addr.append(2);
    // sin_port in network byte order (big-endian)
    let port64 = Int64(from: port);
    let portHi = port64 / 256;
    let portLo = port64 % 256;
    addr.append(UInt8(from: portHi));
    addr.append(UInt8(from: portLo));
    // sin_addr = INADDR_ANY + zero padding (12 bytes)
    var pad: Int64 = 0;
    while pad < 12 {
        addr.append(0);
        pad = pad + 1
    }
    addr
}

// Linux has sin_family (2 bytes, uint16) — no sin_len
@platform(.linux)
func buildSockaddrIn(port: UInt16) -> Array[UInt8] {
    var addr = Array[UInt8]();
    // sin_family = AF_INET (2) as little-endian uint16
    addr.append(2);
    addr.append(0);
    // sin_port in network byte order (big-endian)
    let port64 = Int64(from: port);
    let portHi = port64 / 256;
    let portLo = port64 % 256;
    addr.append(UInt8(from: portHi));
    addr.append(UInt8(from: portLo));
    // sin_addr = INADDR_ANY + zero padding (12 bytes)
    var pad: Int64 = 0;
    while pad < 12 {
        addr.append(0);
        pad = pad + 1
    }
    addr
}

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

    /// Returns the file descriptor and detaches it from this stream.
    /// After calling this, the TcpStream will NOT close the fd on destruction.
    public mutating func detachFd() -> Int32 {
        let fd = self.fd;
        self.fd = -1;
        fd
    }

    deinit {
        if self.fd >= 0 {
            let _ = libc.close(self.fd);
        }
    }
}

extend TcpStream {
    /// Connects to a remote host and port, returning a TcpStream.
    /// Uses getaddrinfo for DNS resolution.
    public static func connect(host: String, port: UInt16) -> Result[TcpStream, Error] {
        // Build port string for getaddrinfo
        let port64 = Int64(from: port);
        let portStr = port64.format();

        // Set up hints: AF_INET, SOCK_STREAM, IPPROTO_TCP
        let addrinfoSize = ADDRINFO_SIZE();
        var hints = Array[UInt8](capacity: addrinfoSize);
        var hi: Int64 = 0;
        while hi < addrinfoSize {
            hints.append(0);
            hi = hi + 1
        }
        // ai_family = AF_INET (2) at offset 4
        let hintsPtr = hints.asPointer();
        hintsPtr.offset(by: 4).cast[Int32]().write(libc.AF_INET());
        // ai_socktype = SOCK_STREAM (1) at offset 8
        hintsPtr.offset(by: 8).cast[Int32]().write(libc.SOCK_STREAM());
        // ai_protocol = IPPROTO_TCP (6) at offset 12
        hintsPtr.offset(by: 12).cast[Int32]().write(libc.IPPROTO_TCP());

        // Null-terminate host and port strings for C
        var hostBuf = Array[UInt8]();
        var hci: Int64 = 0;
        while hci < host.byteCount {
            hostBuf.append(host.byteAtUnchecked(hci));
            hci = hci + 1
        }
        hostBuf.append(0);

        var portBuf = Array[UInt8]();
        var pci: Int64 = 0;
        while pci < portStr.byteCount {
            portBuf.append(portStr.byteAtUnchecked(pci));
            pci = pci + 1
        }
        portBuf.append(0);

        // Call getaddrinfo
        var resultPtr = Pointer[UInt8].nullPointer();
        let gaiResult = libc.getaddrinfo(
            hostBuf.asPointer(),
            portBuf.asPointer(),
            hints.asPointer(),
            Pointer(to: resultPtr).cast[Pointer[UInt8]]()
        );
        if gaiResult != 0 {
            return .Err(Error(gaiResult))
        }

        // Extract address info from first result
        // ai_family at offset 4, ai_socktype at offset 8, ai_protocol at offset 12
        // ai_addrlen at offset 16, ai_addr offset is platform-specific
        let infoPtr = resultPtr;
        let family = infoPtr.offset(by: 4).cast[Int32]().read();
        let socktype = infoPtr.offset(by: 8).cast[Int32]().read();
        let proto = infoPtr.offset(by: 12).cast[Int32]().read();
        let addrlen = infoPtr.offset(by: 16).cast[Int32]().read();
        let addrPtr = infoPtr.offset(by: AI_ADDR_OFFSET()).cast[Pointer[UInt8]]().read();

        // Create socket
        let fd = libc.socket(family, socktype, proto);
        if fd < 0 {
            libc.freeaddrinfo(resultPtr);
            return .Err(Error.last())
        }

        // Connect
        let connResult = libc.connect(fd, addrPtr, addrlen);
        libc.freeaddrinfo(resultPtr);

        if connResult < 0 {
            let _ = libc.close(fd);
            return .Err(Error.last())
        }

        .Ok(TcpStream(fd))
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

        // Build sockaddr_in (platform-specific layout, 16 bytes)
        var addr = buildSockaddrIn(port);

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
