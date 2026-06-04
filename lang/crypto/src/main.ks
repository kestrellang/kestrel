/// Cryptographic hash functions.
///
/// Provides pure Kestrel implementations of SHA-256, SHA-512, MD5, and BLAKE2b.
///
/// # Examples
///
/// ```
/// import crypto.digest.(Digest, DigestOutput)
/// import crypto.(SHA256)
///
/// let hex = SHA256.hash(data).hexString;
///
/// var hasher = SHA256();
/// hasher.update(part1);
/// hasher.update(part2);
/// let output = hasher.finalize();
/// print(output.hexString);
/// ```
