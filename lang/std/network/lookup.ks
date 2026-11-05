// Host name resolution.

module std.network

import std.numeric.(Int32, Int64, UInt16)
import std.collections.(Array)
import std.text.(String)
import std.result.(Result)
import std.streams.(IoError, IoErrorKind)
import std.ffi.(malloc, free)

/// Resolves `host` (a name or an address literal) to the addresses to
/// try for `port`, IPv6 and IPv4 interleaved in the system's preferred
/// order.
///
/// # Errors
///
/// `HostNotFound` when the name doesn't resolve, `Other` for any other
/// resolver failure.
///
/// # Examples
///
/// ```
/// for address in try lookup("example.com", 443) {
///     println(address);
/// }
/// ```
public func lookup(host: String, port: UInt16) -> Result[Array[SocketAddress], IoError] {
    let capacity: Int64 = 64;
    let cells = malloc(capacity * nativeAddressSize());
    let kindCell = malloc(4);
    let name = host.toCString();
    let n = native_lookup(name.raw.asRaw(), Int64(from: port), cells, capacity, kindCell);
    let error = IoError.last();
    let kind = kindCell.cast[Int32]().read();
    name.free();
    free(kindCell);
    if n < 0 {
        free(cells);
        // -1: the resolver reported a system error, held in errno.
        return .Err(if kind < 0 { error } else { IoError(kind: lookupErrorKind(kind)) })
    }
    var addresses = Array[SocketAddress]();
    var i: Int64 = 0;
    while i < n {
        addresses.append(SocketAddress.readNative(from: cells.cast[UInt8]().offset(by: i * nativeAddressSize()).asRaw()));
        i = i + 1
    }
    free(cells);
    .Ok(addresses)
}

/// The kind for the shim's resolver error number.
func lookupErrorKind(code: Int32) -> IoErrorKind {
    match Int64(from: code) { 24 => .HostNotFound, 11 => .OutOfMemory, _ => .Other }
}
