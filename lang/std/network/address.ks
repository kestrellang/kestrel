// IP and socket addresses: plain values that parse, print and hash.

module std.network

import std.numeric.(Int64, UInt8, UInt16, UInt32, UInt64)
import std.collections.(Array, Slice)
import std.text.(String, Formatter, Formattable, FormatOptions)
import std.core.(Bool, Equatable, Hashable, Hasher)
import std.result.(Optional)
import std.memory.(RawPointer, UnsafeSlice)

/// An IPv4 or IPv6 address.
///
/// # Examples
///
/// ```
/// let ip = IpAddress.parse("::1").unwrap();
/// assert(ip.isLoopback);
/// ```
public enum IpAddress: Equatable, Hashable, Formattable {
    case V4(Ipv4Address)
    case V6(Ipv6Address)

    /// Parses dotted IPv4 (`127.0.0.1`) or IPv6 (`::1`, `fe80::1%3`).
    public static func parse(text: String) -> Optional[IpAddress] {
        if let .Some(v4) = Ipv4Address.parse(text) {
            return .Some(.V4(v4))
        }
        match Ipv6Address.parse(text) {
            .Some(v6) => .Some(.V6(v6)),
            .None => .None
        }
    }

    /// `127.0.0.0/8` or `::1`.
    public var isLoopback: Bool {
        match self { .V4(a) => a.isLoopback, .V6(a) => a.isLoopback }
    }

    /// `0.0.0.0` or `::`.
    public var isUnspecified: Bool {
        match self { .V4(a) => a.isUnspecified, .V6(a) => a.isUnspecified }
    }

    /// Same family and address.
    public func isEqual(to other: IpAddress) -> Bool {
        match (self, other) {
            (.V4(a), .V4(b)) => a == b,
            (.V6(a), .V6(b)) => a == b,
            _ => false
        }
    }

    /// Hashes the family and address.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        match self {
            .V4(a) => { let tag: UInt8 = 4; tag.hash(into: hasher); a.hash(into: hasher) },
            .V6(a) => { let tag: UInt8 = 6; tag.hash(into: hasher); a.hash(into: hasher) }
        }
    }

    /// The address in its usual text form.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        match self {
            .V4(a) => a.format(into: writer, options),
            .V6(a) => a.format(into: writer, options)
        }
    }
}

/// An IPv4 address, such as `192.168.1.1`.
public struct Ipv4Address: Equatable, Hashable, Formattable {
    /// The address as one big-endian number: `127.0.0.1` is `0x7f000001`.
    public var bits: UInt32

    /// @name From Octets
    /// The address `a.b.c.d`.
    public init(a: UInt8, b: UInt8, c: UInt8, d: UInt8) {
        self.bits = (UInt32(from: a) << 24) | (UInt32(from: b) << 16) | (UInt32(from: c) << 8) | UInt32(from: d)
    }

    /// @name From Bits
    /// The address whose big-endian number is `bits`.
    public init(bits bits: UInt32) {
        self.bits = bits
    }

    /// `127.0.0.1`.
    public static var localhost: Ipv4Address { Ipv4Address(127, 0, 0, 1) }

    /// `0.0.0.0`: every local address, when binding.
    public static var any: Ipv4Address { Ipv4Address(bits: 0) }

    /// The four octets, most significant first.
    public var octets: (UInt8, UInt8, UInt8, UInt8) {
        (self.octet(0), self.octet(1), self.octet(2), self.octet(3))
    }

    /// `127.0.0.0/8`.
    public var isLoopback: Bool { (self.bits >> 24) == 127 }

    /// `0.0.0.0`.
    public var isUnspecified: Bool { self.bits == 0 }

    /// Parses four decimal octets, `a.b.c.d`, with no leading zeros.
    public static func parse(text: String) -> Optional[Ipv4Address] {
        let b = text.asByteSlice();
        var bits: UInt32 = 0;
        var parts = 0;
        var i = 0;
        while i <= b.count {
            let (value, next) = match decimal(b, i, 255) {
                .Some(found) => found,
                .None => return .None
            };
            bits = (bits << 8) | UInt32(exactly: value)!;
            parts = parts + 1;
            if next == b.count {
                return if parts == 4 { .Some(Ipv4Address(bits: bits)) } else { .None }
            }
            if parts == 4 or b(next) != dot() {
                return .None
            }
            i = next + 1
        }
        .None
    }

    func octet(index: Int64) -> UInt8 {
        UInt8(truncating: (self.bits >> (24 - index * 8)) & 255)
    }

    /// Same address.
    public func isEqual(to other: Ipv4Address) -> Bool { self.bits == other.bits }

    /// Hashes the address.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.bits.hash(into: hasher)
    }

    /// `a.b.c.d`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        let (a, b, c, d) = self.octets;
        writer.pad("\(a).\(b).\(c).\(d)", options)
    }
}

/// An IPv6 address, such as `2001:db8::1`.
public struct Ipv6Address: Equatable, Hashable, Formattable {
    /// The first 64 bits, big-endian.
    var high: UInt64
    /// The last 64 bits, big-endian.
    var low: UInt64
    /// The interface for a link-local address (`fe80::1%3`), or 0.
    public var scopeId: UInt32

    /// @name From Segments
    /// The address of eight 16-bit segments, most significant first.
    public init(segments segments: (UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16)) {
        let (a, b, c, d, e, f, g, h) = segments;
        self.high = join(a, b, c, d);
        self.low = join(e, f, g, h);
        self.scopeId = 0
    }

    /// @name From Halves
    /// The address whose first and last 64 bits are `high` and `low`.
    public init(high high: UInt64, low low: UInt64, scopeId scopeId: UInt32) {
        self.high = high;
        self.low = low;
        self.scopeId = scopeId
    }

    /// `::1`.
    public static var localhost: Ipv6Address { Ipv6Address(high: 0, low: 1, scopeId: 0) }

    /// `::`: every local address, when binding.
    public static var any: Ipv6Address { Ipv6Address(high: 0, low: 0, scopeId: 0) }

    /// The eight segments, most significant first.
    public var segments: (UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16, UInt16) {
        (self.segment(0), self.segment(1), self.segment(2), self.segment(3),
         self.segment(4), self.segment(5), self.segment(6), self.segment(7))
    }

    /// `::1`.
    public var isLoopback: Bool { self.high == 0 and self.low == 1 }

    /// `::`.
    public var isUnspecified: Bool { self.high == 0 and self.low == 0 }

    /// The segment at `index` (0 to 7).
    public func segment(index: Int64) -> UInt16 {
        let half = if index < 4 { self.high } else { self.low };
        UInt16(truncating: (half >> ((3 - index % 4) * 16)) & 65535)
    }

    /// Parses RFC 4291 text: up to eight hex segments, one `::` for a run
    /// of zeros, an optional trailing dotted IPv4 (`::ffff:1.2.3.4`) and an
    /// optional numeric zone (`%3`).
    public static func parse(text: String) -> Optional[Ipv6Address] {
        let b = text.asByteSlice();
        // The zone, after `%`.
        var end = b.count;
        var scope: UInt32 = 0;
        var k = 0;
        while k < b.count {
            if b(k) == 37 {
                let (value, next) = match decimal(b, k + 1, 4294967295) {
                    .Some(found) => found,
                    .None => return .None
                };
                if next != b.count {
                    return .None
                }
                scope = UInt32(exactly: value)!;
                end = k;
                break
            }
            k = k + 1
        }
        // Segments before and after `::`.
        var head = Array[UInt16]();
        var tail = Array[UInt16]();
        var gap = false;
        var i = 0;
        if end >= 2 and b(0) == colon() and b(1) == colon() {
            gap = true;
            i = 2
        }
        while i < end {
            // A dotted IPv4 tail fills the last two segments.
            if containsDot(b, i, end) {
                let v4 = match Ipv4Address.parse(String(fromUtf8Unchecked: b(i..<end))) {
                    .Some(a) => a,
                    .None => return .None
                };
                let upper = UInt16(truncating: v4.bits >> 16);
                let lower = UInt16(truncating: v4.bits & 65535);
                if gap {
                    tail.append(upper);
                    tail.append(lower)
                } else {
                    head.append(upper);
                    head.append(lower)
                }
                i = end;
                break
            }
            let (value, next) = match hex(b, i, end) {
                .Some(found) => found,
                .None => return .None
            };
            if gap { tail.append(value) } else { head.append(value) }
            i = next;
            if i == end {
                break
            }
            if b(i) != colon() {
                return .None
            }
            if i + 1 < end and b(i + 1) == colon() {
                if gap {
                    return .None
                }
                gap = true;
                i = i + 2
            } else {
                i = i + 1;
                if i == end {
                    return .None
                }
            }
        }
        let total = head.count + tail.count;
        if (gap and total > 7) or (not gap and total != 8) {
            return .None
        }
        var all = Array[UInt16]();
        all.append(contentsOf: head.asSlice());
        var z = 0;
        while z < 8 - total {
            all.append(0);
            z = z + 1
        }
        all.append(contentsOf: tail.asSlice());
        .Some(Ipv6Address(
            high: join(all(0), all(1), all(2), all(3)),
            low: join(all(4), all(5), all(6), all(7)),
            scopeId: scope
        ))
    }

    /// Same address and zone.
    public func isEqual(to other: Ipv6Address) -> Bool {
        self.high == other.high and self.low == other.low and self.scopeId == other.scopeId
    }

    /// Hashes the address and zone.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.high.hash(into: hasher);
        self.low.hash(into: hasher);
        self.scopeId.hash(into: hasher)
    }

    /// RFC 5952 text: lowercase hex, the longest run of two or more zero
    /// segments as `::`, and `%zone` when set.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        // The longest zero run (first wins on a tie).
        var bestStart = -1;
        var bestLength = 0;
        var i = 0;
        while i < 8 {
            var j = i;
            while j < 8 and self.segment(j) == 0 {
                j = j + 1
            }
            if j - i > bestLength and j - i >= 2 {
                bestStart = i;
                bestLength = j - i
            }
            i = if j > i { j } else { i + 1 }
        }
        var text = "";
        i = 0;
        while i < 8 {
            if i == bestStart {
                text.append("::");
                i = i + bestLength;
                continue
            }
            if i > 0 and i != bestStart + bestLength {
                text.append(":")
            }
            text.append(hexText(self.segment(i)));
            i = i + 1
        }
        if self.scopeId != 0 {
            text.append("%\(self.scopeId)")
        }
        writer.pad(text, options)
    }
}

/// An IP address and port, such as `127.0.0.1:8080` or `[::1]:8080`.
public struct SocketAddress: Equatable, Hashable, Formattable {
    /// The IP address.
    public var ip: IpAddress
    /// The port.
    public var port: UInt16

    /// @name From IP
    /// `ip` at `port`.
    public init(ip: IpAddress, port port: UInt16) {
        self.ip = ip;
        self.port = port
    }

    /// Parses `1.2.3.4:80` or `[::1]:80`.
    public static func parse(text: String) -> Optional[SocketAddress] {
        let b = text.asByteSlice();
        // The port follows the last colon.
        var colonAt = b.count - 1;
        while colonAt >= 0 and b(colonAt) != colon() {
            colonAt = colonAt - 1
        }
        if colonAt <= 0 {
            return .None
        }
        let (port, next) = match decimal(b, colonAt + 1, 65535) {
            .Some(found) => found,
            .None => return .None
        };
        if next != b.count {
            return .None
        }
        let host = if b(0) == 91 {
            // `[v6]`
            if b(colonAt - 1) != 93 {
                return .None
            }
            match Ipv6Address.parse(String(fromUtf8Unchecked: b(1..<(colonAt - 1)))) {
                .Some(a) => IpAddress.V6(a),
                .None => return .None
            }
        } else {
            match Ipv4Address.parse(String(fromUtf8Unchecked: b(0..<colonAt))) {
                .Some(a) => IpAddress.V4(a),
                .None => return .None
            }
        };
        .Some(SocketAddress(host, port: UInt16(exactly: port)!))
    }

    /// Same address and port.
    public func isEqual(to other: SocketAddress) -> Bool {
        self.ip == other.ip and self.port == other.port
    }

    /// Hashes the address and port.
    public func hash[H](mutating into hasher: H) where H: Hasher {
        self.ip.hash(into: hasher);
        self.port.hash(into: hasher)
    }

    /// `a.b.c.d:port` or `[v6]:port`.
    public func format(mutating into writer: some Formatter, options: FormatOptions = FormatOptions.default()) {
        let text = match self.ip {
            .V4(a) => "\(a):\(self.port)",
            .V6(a) => "[\(a)]:\(self.port)"
        };
        writer.pad(text, options)
    }

    /// Writes the address into a 40-byte `kestrel_address` for the shim.
    func writeNative(into cells: RawPointer) {
        let words = cells.cast[Int64]();
        let bytes = cells.cast[UInt8]().offset(by: 24);
        unsafe words.offset(by: 1).write(Int64(from: self.port));
        match self.ip {
            .V4(a) => {
                unsafe words.write(4);
                unsafe words.offset(by: 2).write(0);
                var i = 0;
                while i < 4 {
                    unsafe bytes.offset(by: i).write(a.octet(i));
                    i = i + 1
                }
            },
            .V6(a) => {
                unsafe words.write(6);
                unsafe words.offset(by: 2).write(Int64(from: a.scopeId));
                var i = 0;
                while i < 8 {
                    let s = a.segment(i);
                    unsafe bytes.offset(by: i * 2).write(UInt8(truncating: s >> 8));
                    unsafe bytes.offset(by: i * 2 + 1).write(UInt8(truncating: s & 255));
                    i = i + 1
                }
            }
        }
    }

    /// Reads a `kestrel_address` the shim wrote.
    static func readNative(from cells: RawPointer) -> SocketAddress {
        let words = cells.cast[Int64]();
        let bytes = cells.cast[UInt8]().offset(by: 24);
        let port = UInt16(exactly: unsafe words.offset(by: 1).read())!;
        if unsafe words.read() == 4 {
            let ip = Ipv4Address(unsafe bytes.read(), unsafe bytes.offset(by: 1).read(), unsafe bytes.offset(by: 2).read(), unsafe bytes.offset(by: 3).read());
            return SocketAddress(IpAddress.V4(ip), port: port)
        }
        var high: UInt64 = 0;
        var low: UInt64 = 0;
        var i = 0;
        while i < 16 {
            let byte = UInt64(from: unsafe bytes.offset(by: i).read());
            if i < 8 { high = (high << 8) | byte } else { low = (low << 8) | byte }
            i = i + 1
        }
        let scope = UInt32(exactly: unsafe words.offset(by: 2).read())!;
        SocketAddress(IpAddress.V6(Ipv6Address(high: high, low: low, scopeId: scope)), port: port)
    }
}

/// The size of a `kestrel_address`.
internal func nativeAddressSize() -> Int64 { 40 }

internal func dot() -> UInt8 { 46 }
internal func colon() -> UInt8 { 58 }

/// Four 16-bit segments as one 64-bit half.
func join(a: UInt16, b: UInt16, c: UInt16, d: UInt16) -> UInt64 {
    (UInt64(from: a) << 48) | (UInt64(from: b) << 32) | (UInt64(from: c) << 16) | UInt64(from: d)
}

/// A decimal number at `start` with no leading zero and at most `max`, and
/// the index after it.
func decimal(b: Slice[UInt8], start: Int64, max: Int64) -> Optional[(Int64, Int64)] {
    var i = start;
    var value = 0;
    while i < b.count and b(i) >= 48 and b(i) <= 57 {
        if i > start and value == 0 {
            return .None
        }
        value = value * 10 + Int64(from: b(i) - 48);
        if value > max {
            return .None
        }
        i = i + 1
    }
    if i == start { .None } else { .Some((value, i)) }
}

/// One to four hex digits at `start`, before `end`, and the index after.
func hex(b: Slice[UInt8], start: Int64, end: Int64) -> Optional[(UInt16, Int64)] {
    var i = start;
    var value: UInt16 = 0;
    while i < end and i - start < 4 {
        let c = b(i);
        let digit = if c >= 48 and c <= 57 { c - 48 }
            else if c >= 97 and c <= 102 { c - 87 }
            else if c >= 65 and c <= 70 { c - 55 }
            else { break };
        value = (value << 4) | UInt16(from: digit);
        i = i + 1
    }
    if i == start or (i < end and b(i) != colon()) { .None } else { .Some((value, i)) }
}

/// Whether a `.` comes before the next `:` in `b(start..<end)`.
func containsDot(b: Slice[UInt8], start: Int64, end: Int64) -> Bool {
    var i = start;
    while i < end and b(i) != colon() {
        if b(i) == dot() {
            return true
        }
        i = i + 1
    }
    false
}

/// `value` in lowercase hex without leading zeros.
func hexText(value: UInt16) -> String {
    let digits = "0123456789abcdef".asByteSlice();
    var out = Array[UInt8]();
    var shift: Int64 = 12;
    var started = false;
    while true {
        let d = (value >> shift) & 15;
        if d != 0 or started or shift == 0 {
            out.append(digits(Int64(from: d)));
            started = true
        }
        if shift == 0 {
            break
        }
        shift = shift - 4
    }
    String(fromUtf8Unchecked: out.asSlice())
}
