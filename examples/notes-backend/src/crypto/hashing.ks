module notes.crypto

import std.collections.(DefaultHasher)

// Hashes a password with a salt using DefaultHasher.
public func hashPassword(password: String, salt: String) -> String {
    var hasher = DefaultHasher();
    salt.hash(into: hasher);
    password.hash(into: hasher);
    hasher.finish().formatted()
}

// Generates a simple salt from the email (deterministic but unique per user).
public func generateSalt(email: String) -> String {
    var hasher = DefaultHasher();
    email.hash(into: hasher);
    "salt-" + hasher.finish().formatted()
}
