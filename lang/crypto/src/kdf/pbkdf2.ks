module crypto.kdf

import crypto.digest.(Digest)
import crypto.key.(SymmetricKey)
import crypto.error.(CryptoError)
import crypto.mac.(HMAC)

/// PBKDF2: Password-based key derivation function 2 (RFC 8018).
///
/// Iteratively applies HMAC to strengthen a password into a key.
///
/// # Examples
///
/// ```
/// let key = try PBKDF2[SHA256].deriveKey(
///     password.asByteSlice(),
///     salt: salt,
///     iterations: 100000,
///     length: 32
/// );
/// ```
public struct PBKDF2[H] where H: Digest {

    /// Derives a key from a password using iterated HMAC.
    ///
    /// Higher iteration counts are slower but more resistant to brute-force.
    /// OWASP recommends at least 600,000 for SHA-256 as of 2023.
    ///
    /// Throws `.InvalidIterationCount` if `iterations` is less than 1, and
    /// `.InvalidLength` if `length` is negative or exceeds
    /// (2³² − 1) × digest size (RFC 8018 §5.2) — past that the 32-bit
    /// block index would wrap.
    public static func deriveKey(
        password: some Slice[UInt8],
        salt salt: some Slice[UInt8],
        iterations iterations: Int64,
        length length: Int64
    ) -> SymmetricKey throws CryptoError {
        let hashLen = H.digestSize;
        if iterations < 1 { throw CryptoError.InvalidIterationCount; }
        if length < 0 or length > 4294967295 * hashLen { throw CryptoError.InvalidLength; }
        let blocks = (length + hashLen - 1) / hashLen;
        var passwordBytes: [UInt8] = [];
        passwordBytes.append(contentsOf: password.asSlice());
        let passwordKey = SymmetricKey(bytes: passwordBytes);

        var okm: [UInt8] = [];

        for blockIndex in 1..=blocks {
            // U_1 = HMAC(password, salt || INT_32_BE(blockIndex))
            var mac = HMAC[H](key: passwordKey);
            mac.update(salt.asSlice());
            let be = UInt32(from: blockIndex).toBytesBigEndian();
            mac.update(be);
            var u = mac.finalize().bytes;
            var result = u;

            // U_2 .. U_c: each is HMAC(password, U_prev), XOR into result
            for iter in 1..<iterations {
                var iterMac = HMAC[H](key: passwordKey);
                iterMac.update(u);
                u = iterMac.finalize().bytes;

                for j in 0..<hashLen {
                    result(j) = result(j) ^ u(j);
                }
            }

            okm.append(contentsOf: result.asSlice());
        }

        var truncated: [UInt8] = [];
        for i in 0..<length {
            truncated.append(okm(i));
        }
        return .Ok(SymmetricKey(bytes: truncated));
    }
}
