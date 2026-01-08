// Character types

// Byte - single UTF-8 byte
public type Byte = UInt8

// CodePoint - Unicode code point (single scalar value)
public struct CodePoint: Equatable, Comparable, Hashable {
    private var value: UInt32

    public init(value: UInt32) {
        self.value = value;
    }

    public var value: UInt32 { self.value }

    // Character classification
    public func isAscii() -> Bool {
        self.value < 128
    }

    public func isAlphabetic() -> Bool {
        (self.value >= 65 and self.value <= 90) or    // A-Z
        (self.value >= 97 and self.value <= 122)      // a-z
        // Full Unicode alphabetic check would need Unicode tables
    }

    public func isNumeric() -> Bool {
        self.value >= 48 and self.value <= 57  // 0-9
    }

    public func isAlphanumeric() -> Bool {
        self.isAlphabetic() or self.isNumeric()
    }

    public func isWhitespace() -> Bool {
        self.value == 32 or   // space
        self.value == 9 or    // tab
        self.value == 10 or   // newline
        self.value == 13 or   // carriage return
        self.value == 12      // form feed
    }

    public func isControl() -> Bool {
        self.value < 32 or self.value == 127
    }

    public func isUppercase() -> Bool {
        self.value >= 65 and self.value <= 90
    }

    public func isLowercase() -> Bool {
        self.value >= 97 and self.value <= 122
    }

    public func toUppercase() -> CodePoint {
        if self.isLowercase() {
            CodePoint(value: self.value - 32)
        } else {
            self
        }
    }

    public func toLowercase() -> CodePoint {
        if self.isUppercase() {
            CodePoint(value: self.value + 32)
        } else {
            self
        }
    }

    // UTF-8 encoding
    public func utf8Length() -> Int {
        if self.value < 0x80 { 1 }
        else if self.value < 0x800 { 2 }
        else if self.value < 0x10000 { 3 }
        else { 4 }
    }

    public func encodeUtf8(into buffer: mutating [UInt8]) -> Int {
        let len = self.utf8Length()
        match len {
            1 => {
                buffer.append(UInt8(self.value))
            },
            2 => {
                buffer.append(UInt8(0xC0 | ((self.value >> 6) & 0x1F)))
                buffer.append(UInt8(0x80 | (self.value & 0x3F)))
            },
            3 => {
                buffer.append(UInt8(0xE0 | ((self.value >> 12) & 0x0F)))
                buffer.append(UInt8(0x80 | ((self.value >> 6) & 0x3F)))
                buffer.append(UInt8(0x80 | (self.value & 0x3F)))
            },
            4 => {
                buffer.append(UInt8(0xF0 | ((self.value >> 18) & 0x07)))
                buffer.append(UInt8(0x80 | ((self.value >> 12) & 0x3F)))
                buffer.append(UInt8(0x80 | ((self.value >> 6) & 0x3F)))
                buffer.append(UInt8(0x80 | (self.value & 0x3F)))
            }
        }
        len
    }

    // Equatable
    public func equals(other: CodePoint) -> Bool {
        self.value == other.value
    }

    // Comparable
    public func compare(other: CodePoint) -> Ordering {
        self.value.compare(other.value)
    }

    // Hashable
    public func hash[H](into hasher: mutating H) where H: Hasher {
        self.value.hash(into: hasher)
    }
}

// Char - Extended grapheme cluster (user-perceived character)
// May be multiple code points (e.g., "é" or "👨‍👩‍👧")
public struct Char: Equatable, Hashable {
    private var codePoints: Array[CodePoint]

    public init(codePoint: CodePoint) {
        self.codePoints = [codePoint]
    }

    public init(codePoints: Array[CodePoint]) {
        self.codePoints = codePoints
    }

    public var codePoints: Array[CodePoint] { self.codePoints }

    public var codePointCount: Int {
        self.codePoints.count
    }

    public func isAscii() -> Bool {
        self.codePoints.count == 1 and self.codePoints(unchecked: 0).isAscii()
    }

    // Byte length when encoded as UTF-8
    public func utf8Length() -> Int {
        var len = 0
        /* for cp in self.codePoints {
            len += cp.utf8Length()
        } */
        len
    }

    // Equatable
    public func equals(other: Char) -> Bool {
        self.codePoints == other.codePoints
    }

    // Hashable
    public func hash[H](into hasher: mutating H) where H: Hasher {
        /* for cp in self.codePoints {
            cp.hash(into: hasher)
        } */
    }
}

// Decode UTF-8 byte sequence to code point
public func decodeUtf8(bytes: Slice[UInt8], at index: Int) -> Optional[(CodePoint, Int)] {
    if index >= bytes.count {
        return .None
    }

    let first = bytes(unchecked: index)

    if first < 0x80 {
        // Single byte (ASCII)
        return .Some((CodePoint(value: UInt32(first)), 1))
    } else if first < 0xC0 {
        // Continuation byte (invalid as start)
        return .None
    } else if first < 0xE0 {
        // Two bytes
        if index + 1 >= bytes.count { return .None }
        let second = bytes(unchecked: index + 1)
        if (second & 0xC0) != 0x80 { return .None }
        let value = (UInt32(first & 0x1F) << 6) | UInt32(second & 0x3F)
        return .Some((CodePoint(value: value), 2))
    } else if first < 0xF0 {
        // Three bytes
        if index + 2 >= bytes.count { return .None }
        let second = bytes(unchecked: index + 1)
        let third = bytes(unchecked: index + 2)
        if (second & 0xC0) != 0x80 or (third & 0xC0) != 0x80 { return .None }
        let value = (UInt32(first & 0x0F) << 12) |
                    (UInt32(second & 0x3F) << 6) |
                    UInt32(third & 0x3F)
        return .Some((CodePoint(value: value), 3))
    } else if first < 0xF8 {
        // Four bytes
        if index + 3 >= bytes.count { return .None }
        let second = bytes(unchecked: index + 1)
        let third = bytes(unchecked: index + 2)
        let fourth = bytes(unchecked: index + 3)
        if (second & 0xC0) != 0x80 or (third & 0xC0) != 0x80 or (fourth & 0xC0) != 0x80 {
            return .None
        }
        let value = (UInt32(first & 0x07) << 18) |
                    (UInt32(second & 0x3F) << 12) |
                    (UInt32(third & 0x3F) << 6) |
                    UInt32(fourth & 0x3F)
        return .Some((CodePoint(value: value), 4))
    } else {
        return .None
    }
}
