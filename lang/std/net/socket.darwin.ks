// Platform-specific socket helpers (macOS)

module std.net.socket

import std.num.(Int64, Int32, UInt8, UInt16)
import std.collections.(Array)

// addrinfo struct is 48 bytes on macOS
// ai_addr is at offset 32 (ai_canonname at 24, ai_addr at 32)
func ADDRINFO_SIZE() -> Int64 { 48 }
func AI_ADDR_OFFSET() -> Int64 { 32 }

// Build sockaddr_in (macOS layout, 16 bytes)
// macOS has sin_len (1 byte) + sin_family (1 byte)
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
