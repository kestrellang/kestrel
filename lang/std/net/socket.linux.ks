// Platform-specific socket helpers (Linux x86_64)

module std.net.socket

import std.num.(Int64, Int32, UInt8, UInt16)
import std.collections.(Array)

// addrinfo struct is 48 bytes on Linux x86_64
// ai_addr is at offset 24 (ai_addr at 24, ai_canonname at 32)
func ADDRINFO_SIZE() -> Int64 { 48 }
func AI_ADDR_OFFSET() -> Int64 { 24 }

// Build sockaddr_in (Linux layout, 16 bytes)
// Linux has sin_family (2 bytes, uint16) — no sin_len
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
