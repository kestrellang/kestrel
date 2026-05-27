module crypto.digest

/// The output of a cryptographic hash function.
public struct DigestOutput: Equatable, Hashable {
    private var storage: Array[UInt8];

    public init(bytes bytes: Array[UInt8]) {
        self.storage = bytes;
    }

    /// The raw digest bytes.
    public var bytes: Array[UInt8] { self.storage }

    /// The digest as a lowercase hexadecimal string.
    public var hexString: String {
        var result = String();
        for b in self.storage {
            result.append(hexNibble(Int64(from: b) >> 4));
            result.append(hexNibble(Int64(from: b) & 0x0f));
        }
        return result;
    }

    /// Constant-time comparison to prevent timing side-channel attacks.
    public func equals(other: DigestOutput) -> Bool {
        if self.storage.count != other.storage.count {
            return false;
        }
        var diff: UInt8 = 0;
        for i in 0..<self.storage.count {
            diff = diff | (self.storage(i) ^ other.storage(i));
        }
        return diff == 0;
    }

    public func hash[H](mutating into hasher: H) where H: Hasher {
        hasher.write(self.storage.asSlice());
    }
}

/// A cryptographic hash function that maps arbitrary data to a fixed-size digest.
public protocol Digest {
    /// The size of the digest output in bytes.
    static var digestSize: Int64 { get }

    /// Feeds data into the hash function.
    mutating func update[S](bytes: S) where S: Slice[UInt8]

    /// Computes the final digest. The hasher state is unchanged, so
    /// you can continue calling `update` and `finalize` again to get
    /// the digest of a longer message.
    func finalize() -> DigestOutput
}

func hexNibble(n: Int64) -> String {
    match n {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "a",
        11 => "b",
        12 => "c",
        13 => "d",
        14 => "e",
        _ => "f"
    }
}
