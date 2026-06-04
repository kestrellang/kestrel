module crypto.kdf

import crypto.digest.(Digest)
import crypto.key.(SymmetricKey)
import crypto.mac.(HMAC, AuthenticationCode)

/// HKDF: HMAC-based key derivation function (RFC 5869).
///
/// All methods are static — HKDF has no accumulated state.
///
/// # Examples
///
/// ```
/// // Combined extract-and-expand (the common case)
/// let key = HKDF[SHA256].deriveKey(
///     ikm: inputKeyMaterial,
///     salt: salt,
///     info: context,
///     length: 32
/// );
///
/// // Two-phase (for protocols like TLS 1.3)
/// let prk = HKDF[SHA256].extract(salt: salt, ikm: inputKeyMaterial);
/// let key = HKDF[SHA256].expand(prk: prk, info: context, length: 32);
/// ```
public struct HKDF[H] where H: Digest {

    /// Extract: derives a fixed-length pseudorandom key from input key material.
    ///
    /// If salt is empty, a string of zeros equal to the hash digest size is used.
    public static func extract(salt salt: Array[UInt8], ikm ikm: SymmetricKey) -> AuthenticationCode {
        var effectiveSalt = salt;
        if effectiveSalt.count == 0 {
            effectiveSalt = Array[UInt8](repeating: 0, count: H.digestSize);
        }
        let saltKey = SymmetricKey(bytes: effectiveSalt);
        var mac = HMAC[H](key: saltKey);
        mac.update(ikm.bytes);
        return mac.finalize();
    }

    /// Expand: derives output key material of the requested length from a PRK.
    ///
    /// Length must not exceed 255 * digest size.
    public static func expand(prk prk: AuthenticationCode, info info: Array[UInt8], length length: Int64) -> SymmetricKey {
        let hashLen = H.digestSize;
        let n = (length + hashLen - 1) / hashLen;

        var okm = Array[UInt8]();
        var prev = Array[UInt8]();
        let prkKey = SymmetricKey(bytes: prk.bytes);

        for i in 1..=n {
            var mac = HMAC[H](key: prkKey);
            mac.update(prev);
            mac.update(info);
            let counter: UInt8 = UInt8(from: i);
            let counterBuf: Array[UInt8] = [counter];
            mac.update(counterBuf);
            let block = mac.finalize();
            prev = block.bytes;
            okm.append(contentsOf: prev.asSlice());
        }

        var result = Array[UInt8]();
        for i in 0..<length {
            result.append(okm(i));
        }
        return SymmetricKey(bytes: result);
    }

    /// Combined extract-and-expand in one call.
    public static func deriveKey(
        ikm ikm: SymmetricKey,
        salt salt: Array[UInt8],
        info info: Array[UInt8],
        length length: Int64
    ) -> SymmetricKey {
        let prk = HKDF[H].extract(salt: salt, ikm: ikm);
        return HKDF[H].expand(prk: prk, info: info, length: length);
    }
}
