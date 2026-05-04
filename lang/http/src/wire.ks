/// Low-level HTTP/1.1 wire protocol helpers.
///
/// Shared utilities used by both server (perch) and client (swoop) for
/// parsing and serializing HTTP/1.1 on the wire. These operate on raw
/// byte buffers and strings — higher-level request/response types live
/// in the framework crates.
///
/// # Examples
///
/// ```
/// var buf = Array[UInt8]();
/// // ... fill buf with "GET / HTTP/1.1\r\n\r\n" bytes ...
/// let end = findHeaderEnd(buf);  // byte offset of first \r in \r\n\r\n
/// ```

module http.wire


/// Scans a byte buffer for the HTTP header terminator `\r\n\r\n`.
///
/// Returns the byte offset of the first `\r` in the `\r\n\r\n`
/// sequence, or `-1` if the terminator is not found. The caller can
/// then split the buffer at `offset` for the header block and
/// `offset + 4` for the body start.
///
/// # Examples
///
/// ```
/// // Given a buffer containing "OK\r\n\r\nBody":
/// findHeaderEnd(buf);  // 2
/// ```
public func findHeaderEnd(buf: Array[UInt8]) -> Int64 {
    let bufLen = buf.count;
    if bufLen < 4 {
        return -1
    }
    var i: Int64 = 0;
    let limit = bufLen - 3;
    while i < limit {
        if buf(unchecked: i) == 13 and buf(unchecked: i + 1) == 10 and buf(unchecked: i + 2) == 13 and buf(unchecked: i + 3) == 10 {
            return i
        }
        i = i + 1
    };
    return -1
}

/// Copies a range of bytes from an array into a new `String`.
///
/// Reads bytes in `[start, end)` and appends each to a fresh string.
/// The caller must ensure the byte range contains valid UTF-8.
///
/// # Examples
///
/// ```
/// let buf: Array[UInt8] = [72, 105];  // "Hi"
/// bytesToString(buf, from: 0, to: 2);  // "Hi"
/// ```
public func bytesToString(buf: Array[UInt8], from start: Int64, to end: Int64) -> String {
    var result = String();
    for i in start..<end {
        result.appendByte(buf(unchecked: i))
    }
    result
}

/// Parses a decimal integer string into an `Int64`.
///
/// Walks the string byte-by-byte, accumulating digits. Non-digit
/// bytes are silently skipped. Returns `0` for an empty string or a
/// string with no digit characters.
///
/// # Examples
///
/// ```
/// parseDecimal("42");    // 42
/// parseDecimal("0");     // 0
/// parseDecimal("");      // 0
/// parseDecimal("12ab");  // 12 (non-digits skipped)
/// ```
public func parseDecimal(s: String) -> Int64 {
    var result: Int64 = 0;
    for i in 0..<s.byteCount {
        let digit = Int64(from: s.bytes(unchecked: i)) - 48;
        if digit >= 0 and digit <= 9 {
            result = result * 10 + digit
        }
    }
    result
}

/// Copies a `String` into a new `Array[UInt8]`.
///
/// The inverse of `bytesToString`. Copies raw UTF-8 bytes without any
/// encoding transformation.
///
/// # Examples
///
/// ```
/// let bytes = stringToBytes("Hi");
/// bytes.count;  // 2
/// bytes(0);     // 72  ('H')
/// ```
public func stringToBytes(s: String) -> Array[UInt8] {
    var buffer = Array[UInt8](capacity: s.byteCount);
    for i in 0..<s.byteCount {
        buffer.append(s.bytes(unchecked: i))
    }
    buffer
}

/// Returns the numeric value of a hex digit byte (0–15), or `-1` if
/// the byte is not a valid hexadecimal character.
///
/// Accepts `0`–`9`, `A`–`F`, and `a`–`f`.
///
/// # Examples
///
/// ```
/// hexDigit(48);   // 0   ('0')
/// hexDigit(65);   // 10  ('A')
/// hexDigit(102);  // 15  ('f')
/// hexDigit(71);   // -1  ('G')
/// ```
public func hexDigit(byte: UInt8) -> Int64 {
    let b = Int64(from: byte);
    if b >= 48 and b <= 57 { return b - 48 }
    if b >= 65 and b <= 70 { return b - 55 }
    if b >= 97 and b <= 102 { return b - 87 }
    return -1
}

/// Returns the ASCII byte for a hex digit value (0–15).
///
/// Values 0–9 produce `'0'`–`'9'`; values 10–15 produce `'A'`–`'F'`.
/// The inverse of `hexDigit`.
///
/// # Examples
///
/// ```
/// hexChar(0);   // 48  ('0')
/// hexChar(10);  // 65  ('A')
/// hexChar(15);  // 70  ('F')
/// ```
public func hexChar(value: Int64) -> UInt8 {
    if value < 10 {
        UInt8(from: value + 48)
    } else {
        UInt8(from: value + 55)
    }
}

/// Decodes an HTTP/1.1 chunked transfer-encoded body.
///
/// Parses the `hex-size\r\ndata\r\n` framing and concatenates the
/// decoded chunks. A zero-length chunk signals the end of the body.
///
/// # Examples
///
/// ```
/// // "5\r\nHello\r\n0\r\n\r\n"
/// let decoded = dechunk(raw);
/// bytesToString(decoded, from: 0, to: decoded.count);  // "Hello"
/// ```
public func dechunk(raw: Array[UInt8]) -> Array[UInt8] {
    var result = Array[UInt8]();
    var pos: Int64 = 0;
    let length = raw.count;

    loop {
        if pos >= length {
            break
        }

        var chunkSize: Int64 = 0;
        while pos < length {
            let b = raw(unchecked: pos);
            if b == 13 {
                pos = pos + 2;
                break
            }
            let digit = hexDigit(b);
            let value = if digit >= 0 { digit } else { 0 };
            chunkSize = chunkSize * 16 + value;
            pos = pos + 1
        }

        if chunkSize == 0 {
            break
        }

        var copied: Int64 = 0;
        while copied < chunkSize and pos < length {
            result.append(raw(unchecked: pos));
            pos = pos + 1;
            copied = copied + 1
        }

        if pos + 1 < length {
            pos = pos + 2
        }
    }

    result
}
