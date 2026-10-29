/// Cryptographic hash functions.
///
/// Pure Kestrel hashes (SHA-1, SHA-2, SHA-3, BLAKE2b, BLAKE3, MD5), HMAC,
/// HKDF, PBKDF2, password hashing and ChaCha20-Poly1305.
///
/// # Examples
///
/// ```
/// import crypto.digest.(Digest, DigestOutput, SHA256)
///
/// let hex = SHA256.hash(data).hexString;
///
/// var hasher = SHA256();
/// hasher.update(part1);
/// hasher.update(part2);
/// let output = hasher.finalize();
/// print(output.hexString);
/// ```
