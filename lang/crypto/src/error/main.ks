module crypto.error

/// Errors from cryptographic operations.
public enum CryptoError {
    /// The authentication tag did not match: the ciphertext, AAD, nonce
    /// or key is wrong, or the data was tampered with.
    case AuthenticationFailure

    /// The key is not the size the algorithm requires (ChaCha20-Poly1305
    /// needs exactly 32 bytes).
    case InvalidKeySize

    /// A requested output length is negative or exceeds the algorithm's
    /// limit (HKDF: 255 × digest size).
    case InvalidLength

    /// An iteration count is less than 1.
    case InvalidIterationCount

    /// A cost parameter is out of range: scrypt's N, r or p, or Argon2's
    /// memory or parallelism.
    case InvalidParameter
}
