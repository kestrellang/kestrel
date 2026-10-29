module crypto.bytes

/// A value made of bytes: digests, tags, nonces, salts, tokens, keys.
///
/// Conforming types get hex and base64 encoding (`hexString`,
/// `base64String`), decoding (`init(hex:)?`, `init(base64:)?`) and a
/// constant-time `isEqual(to:)` from one shared
/// implementation, so declaring `: ByteValue, Equatable` makes `==`
/// timing-safe. Comparing `.bytes` or `.hexString` directly is NOT
/// timing-safe — compare the typed values.
public protocol ByteValue {
    /// The raw bytes.
    var bytes: Array[UInt8] { get }

    /// Builds a value from raw bytes, or null if they are invalid for
    /// this type (e.g. a nonce of the wrong length). A type that accepts
    /// any bytes can declare a plain `init(bytes:)`.
    init(bytes bytes: Array[UInt8])?
}

extend ByteValue {
    /// The bytes as a lowercase hexadecimal string.
    public var hexString: String { encodeHex(self.bytes) }

    /// Decodes a hexadecimal string (either case). Returns null if the
    /// string isn't valid hex or the bytes are invalid for this type.
    public init(hex hex: String)? {
        guard let some raw = decodeHex(hex) else { return null; }
        guard let some value = Self(bytes: raw) else { return null; }
        self = value;
    }

    /// The bytes as base64 (RFC 4648). Defaults to the standard, padded
    /// form; pass `.URLSafe` for tokens and URLs, `.StandardUnpadded` for
    /// PHC strings.
    public func base64String(style: Base64Style = .Standard) -> String {
        encodeBase64(self.bytes, style)
    }

    /// Decodes base64 in any `Base64Style`, padded or not. Returns null on
    /// invalid base64 or bytes invalid for this type.
    public init(base64 text: String)? {
        guard let some raw = decodeBase64(text) else { return null; }
        guard let some value = Self(bytes: raw) else { return null; }
        self = value;
    }

    /// Constant-time equality, so `==` doesn't leak through timing how
    /// many leading bytes matched.
    public func isEqual(to other: Self) -> Bool {
        constantTimeEquals(self.bytes, other.bytes)
    }
}

/// Which base64 variant `base64String` produces (RFC 4648).
public enum Base64Style {
    /// `A–Z a–z 0–9 + /`, padded with `=`: MIME, JSON, most APIs (§4).
    case Standard
    /// The standard alphabet without padding: PHC password strings.
    case StandardUnpadded
    /// `A–Z a–z 0–9 - _`, unpadded: tokens, URLs, JWT (§5).
    case URLSafe
}

/// Compares two byte arrays in time that depends only on their lengths,
/// not their contents. Prefer `==` on a `ByteValue` type.
public func constantTimeEquals(a: Array[UInt8], b: Array[UInt8]) -> Bool {
    if a.count != b.count {
        return false;
    }
    var diff: UInt8 = 0;
    for i in 0..<a.count {
        diff = diff | (a(i) ^ b(i));
    }
    return diff == 0;
}

func encodeHex(bytes: Array[UInt8]) -> String {
    let digits = "0123456789abcdef".toBytes();
    var ascii: [UInt8] = [];
    for b in bytes {
        ascii.append(digits(Int64(from: b >> 4)));
        ascii.append(digits(Int64(from: b & 0x0f)));
    }
    String(fromUtf8Unchecked: ascii)
}

func decodeHex(hex: String) -> Array[UInt8]? {
    let digits = hex.toBytes();
    if digits.count % 2 != 0 {
        return null;
    }
    var result: [UInt8] = [];
    for i in 0..<(digits.count / 2) {
        guard let some high = hexValue(digits(2 * i)) else { return null; }
        guard let some low = hexValue(digits(2 * i + 1)) else { return null; }
        result.append((high << 4) | low);
    }
    return result;
}

/// The value of one ASCII hex digit, or null if `c` isn't one.
func hexValue(c: UInt8) -> UInt8? {
    if c >= 0x30 and c <= 0x39 { return c - 0x30; }        // 0-9
    if c >= 0x61 and c <= 0x66 { return c - 0x61 + 10; }   // a-f
    if c >= 0x41 and c <= 0x46 { return c - 0x41 + 10; }   // A-F
    return null;
}

func encodeBase64(bytes: Array[UInt8], style: Base64Style) -> String {
    let alphabet = match style {
        .URLSafe => "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_".toBytes(),
        _ => "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/".toBytes()
    };
    let padded = match style { .Standard => true, _ => false };

    var ascii: [UInt8] = [];
    var i: Int64 = 0;
    // Each 3 input bytes become 4 output characters of 6 bits each.
    while i + 3 <= bytes.count {
        let group = (Int64(from: bytes(i)) << 16) | (Int64(from: bytes(i + 1)) << 8) | Int64(from: bytes(i + 2));
        ascii.append(alphabet((group >> 18) & 63));
        ascii.append(alphabet((group >> 12) & 63));
        ascii.append(alphabet((group >> 6) & 63));
        ascii.append(alphabet(group & 63));
        i = i + 3;
    }

    // A 1- or 2-byte tail becomes 2 or 3 characters, plus `=` to fill 4.
    let tail = bytes.count - i;
    if tail > 0 {
        var group = Int64(from: bytes(i)) << 16;
        if tail == 2 { group = group | (Int64(from: bytes(i + 1)) << 8); }
        ascii.append(alphabet((group >> 18) & 63));
        ascii.append(alphabet((group >> 12) & 63));
        if tail == 2 { ascii.append(alphabet((group >> 6) & 63)); }
        if padded {
            for _ in tail..<3 { ascii.append(0x3d); }   // '='
        }
    }
    String(fromUtf8Unchecked: ascii)
}

/// Decodes any `Base64Style`. Rejects characters outside both alphabets,
/// misplaced or excess `=`, impossible lengths, and non-zero leftover bits
/// (so each byte string has exactly one accepted encoding per style).
func decodeBase64(text: String) -> Array[UInt8]? {
    let chars = text.toBytes();
    var end = chars.count;
    var padding: Int64 = 0;
    while end > 0 and chars(end - 1) == 0x3d {
        end = end - 1;
        padding = padding + 1;
    }
    if padding > 2 { return null; }
    if padding > 0 and chars.count % 4 != 0 { return null; }
    if end % 4 == 1 { return null; }

    var result: [UInt8] = [];
    var buffer: Int64 = 0;
    var bits: Int64 = 0;
    for i in 0..<end {
        guard let some value = base64Value(chars(i)) else { return null; }
        buffer = (buffer << 6) | value;
        bits = bits + 6;
        if bits >= 8 {
            bits = bits - 8;
            result.append(UInt8(from: (buffer >> bits) & 0xff));
            buffer = buffer & ((1 << bits) - 1);
        }
    }
    if buffer != 0 { return null; }
    return result;
}

/// The 6-bit value of one base64 character from either alphabet, or null.
func base64Value(c: UInt8) -> Int64? {
    if c >= 0x41 and c <= 0x5a { return Int64(from: c - 0x41); }        // A-Z
    if c >= 0x61 and c <= 0x7a { return Int64(from: c - 0x61) + 26; }   // a-z
    if c >= 0x30 and c <= 0x39 { return Int64(from: c - 0x30) + 52; }   // 0-9
    if c == 0x2b or c == 0x2d { return 62; }                             // + or -
    if c == 0x2f or c == 0x5f { return 63; }                             // / or _
    return null;
}
