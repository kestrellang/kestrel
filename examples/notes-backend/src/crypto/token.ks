module notes.crypto

import std.collections.(DefaultHasher)

// Generates a token string from userId and salt.
// For now this uses DefaultHasher but ideally should use a CSPRNG.
public func generateToken(userId: Int64, salt: String) -> String {
    var hasher = DefaultHasher();
    userId.formatted().hash(into: hasher);
    salt.hash(into: hasher);
    "tok-" + hasher.finish().formatted()
}
