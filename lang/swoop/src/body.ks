// Request body types

module swoop.body


// ============================================================================
// BODY ENUM
// ============================================================================

/// The body of an HTTP request.
public enum Body: Cloneable {
    /// A plain text body. Does not set Content-Type automatically.
    case Text(String)

    /// A raw binary body. Does not set Content-Type automatically.
    case Bytes(Array[UInt8])

    /// A URL-encoded form body. Sets Content-Type to application/x-www-form-urlencoded.
    case Form(Array[(String, String)])
}

// ============================================================================
// BODY HELPERS
// ============================================================================

extend Body {
    public func clone() -> Body {
        match self {
            .Text(s) => Body.Text(s.clone()),
            .Bytes(b) => Body.Bytes(b.clone()),
            .Form(pairs) => Body.Form(pairs.clone())
        }
    }

    /// Returns the bytes for this body.
    public func toBytes() -> Array[UInt8] {
        match self {
            .Text(s) => stringToBytes(s),
            .Bytes(b) => b,
            .Form(pairs) => stringToBytes(encodeFormData(pairs))
        }
    }

    /// Returns the byte count of this body.
    public func byteCount() -> Int64 {
        match self {
            .Text(s) => s.byteCount,
            .Bytes(b) => b.count,
            .Form(pairs) => encodeFormData(pairs).byteCount
        }
    }
}

// ============================================================================
// FORM ENCODING
// ============================================================================

/// URL-encodes form data as key=value&key=value pairs.
func encodeFormData(pairs: Array[(String, String)]) -> String {
    var result = String();
    var i: Int64 = 0;
    while i < pairs.count {
        if i > 0 {
            result.append("&")
        }
        let pair = pairs(unchecked: i);
        result.append(percentEncode(pair.0));
        result.append("=");
        result.append(percentEncode(pair.1));
        i = i + 1
    }
    result
}

/// Percent-encodes a string for use in URL form data.
func percentEncode(s: String) -> String {
    var result = String();
    var i: Int64 = 0;
    let len = s.byteCount;
    while i < len {
        let byte = s.byteAtUnchecked(i);
        let b = Int64(from: byte);
        // Unreserved characters: A-Z, a-z, 0-9, -, _, ., ~
        if (b >= 65 and b <= 90) or (b >= 97 and b <= 122) or (b >= 48 and b <= 57)
            or b == 45 or b == 95 or b == 46 or b == 126 {
            result.appendByte(byte)
        } else if b == 32 {
            result.append("+")
        } else {
            result.append("%");
            result.appendByte(hexChar(b / 16));
            result.appendByte(hexChar(b % 16))
        }
        i = i + 1
    }
    result
}

/// Returns the ASCII byte for a hex digit (0-15).
func hexChar(value: Int64) -> UInt8 {
    if value < 10 {
        UInt8(from: value + 48) // '0' + value
    } else {
        UInt8(from: value + 55) // 'A' + (value - 10)
    }
}

/// Converts a String to an Array[UInt8].
func stringToBytes(s: String) -> Array[UInt8] {
    var buf = Array[UInt8](capacity: s.byteCount);
    var i: Int64 = 0;
    while i < s.byteCount {
        buf.append(s.byteAtUnchecked(i));
        i = i + 1
    }
    buf
}
