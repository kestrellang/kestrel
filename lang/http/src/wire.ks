// Shared HTTP wire protocol helpers
//
// Low-level utilities used by both server (perch) and client (swoop)
// for parsing and serializing HTTP/1.1 on the wire.

module http.wire


/// Scans a byte buffer for \r\n\r\n (bytes 13, 10, 13, 10).
/// Returns the byte offset of the first \r, or -1 if not found.
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

/// Converts a range of bytes in an array to a String.
public func bytesToString(buf: Array[UInt8], from start: Int64, to end: Int64) -> String {
    var result = String();
    var i = start;
    while i < end {
        result.appendByte(buf(unchecked: i));
        i = i + 1
    }
    result
}

/// Parses a decimal integer string into an Int64. Returns 0 for invalid input.
public func parseDecimal(s: String) -> Int64 {
    var result: Int64 = 0;
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let byte = s.bytes(unchecked: i);
        let digit = Int64(from: byte) - 48;
        if digit >= 0 and digit <= 9 {
            result = result * 10 + digit
        }
        i = i + 1
    }
    result
}
