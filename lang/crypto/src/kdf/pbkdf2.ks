module crypto.kdf

import crypto.digest.(Digest)
import crypto.key.(SymmetricKey)
import crypto.mac.(HMAC)

/// PBKDF2: Password-based key derivation function 2 (RFC 8018).
///
/// Iteratively applies HMAC to strengthen a password into a key.
///
/// # Examples
///
/// ```
/// let key = PBKDF2[SHA256].deriveKey(
///     password: passwordBytes,
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
    public static func deriveKey(
        password password: Array[UInt8],
        salt salt: Array[UInt8],
        iterations iterations: Int64,
        length length: Int64
    ) -> SymmetricKey {
        let hashLen = H.digestSize;
        let blocks = (length + hashLen - 1) / hashLen;
        let passwordKey = SymmetricKey(bytes: password);

        var okm = Array[UInt8]();

        for blockIndex in 1..=blocks {
            // U_1 = HMAC(password, salt || INT_32_BE(blockIndex))
            var mac = HMAC[H](key: passwordKey);
            mac.update(salt);
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

        var truncated = Array[UInt8]();
        for i in 0..<length {
            truncated.append(okm(i));
        }
        return SymmetricKey(bytes: truncated);
    }
}
