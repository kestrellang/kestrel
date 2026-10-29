module crypto.kdf

import crypto.digest.(Digest)
import crypto.key.(SymmetricKey)
import crypto.error.(CryptoError)
import crypto.mac.(HMAC)

/// HKDF: HMAC-based key derivation function (RFC 5869).
///
/// All methods are static — HKDF has no accumulated state.
///
/// # Examples
///
/// ```
/// // Combined extract-and-expand (the common case); salt and info
/// // default to empty
/// let key = try HKDF[SHA256].deriveKey(inputKeyMaterial, length: 32, salt: salt, info: context);
///
/// // Two-phase (for protocols like TLS 1.3)
/// let prk = HKDF[SHA256].extract(inputKeyMaterial, salt: salt);
/// let key = try HKDF[SHA256].expand(prk, length: 32, info: context);
/// ```
public struct HKDF[H] where H: Digest {

    /// Extract: derives a fixed-length pseudorandom key from input key material.
    ///
    /// If salt is empty (the default), a string of zeros equal to the hash
    /// digest size is used.
    /// The result is key material, so it's a `SymmetricKey`, not a MAC tag.
    public static func extract(ikm: SymmetricKey, salt salt: some Slice[UInt8] = Array[UInt8]()) -> SymmetricKey {
        var effectiveSalt: [UInt8] = [];
        effectiveSalt.append(contentsOf: salt.asSlice());
        if effectiveSalt.count == 0 {
            effectiveSalt = Array[UInt8](repeating: 0, count: H.digestSize);
        }
        let saltKey = SymmetricKey(bytes: effectiveSalt);
        var mac = HMAC[H](key: saltKey);
        mac.update(ikm.bytes);
        return SymmetricKey(bytes: mac.finalize().bytes);
    }

    /// Expand: derives output key material of the requested length from a PRK.
    ///
    /// Throws `.InvalidLength` if `length` is negative or exceeds
    /// 255 × digest size (RFC 5869 §2.3) — past that the one-byte block
    /// counter would wrap and repeat output.
    public static func expand(
        prk: SymmetricKey,
        length length: Int64,
        info info: some Slice[UInt8] = Array[UInt8]()
    ) -> SymmetricKey throws CryptoError {
        let hashLen = H.digestSize;
        if length < 0 or length > 255 * hashLen { throw CryptoError.InvalidLength; }
        let n = (length + hashLen - 1) / hashLen;

        var okm: [UInt8] = [];
        var prev: [UInt8] = [];

        for i in 1..=n {
            var mac = HMAC[H](key: prk);
            mac.update(prev);
            mac.update(info.asSlice());
            let counter: UInt8 = UInt8(from: i);
            let counterBuf: Array[UInt8] = [counter];
            mac.update(counterBuf);
            let block = mac.finalize();
            prev = block.bytes;
            okm.append(contentsOf: prev.asSlice());
        }

        var result: [UInt8] = [];
        for i in 0..<length {
            result.append(okm(i));
        }
        return .Ok(SymmetricKey(bytes: result));
    }

    /// Combined extract-and-expand in one call. Throws `.InvalidLength`
    /// under the same rule as `expand`.
    public static func deriveKey(
        ikm: SymmetricKey,
        length length: Int64,
        salt salt: some Slice[UInt8] = Array[UInt8](),
        info info: some Slice[UInt8] = Array[UInt8]()
    ) -> SymmetricKey throws CryptoError {
        let prk = HKDF[H].extract(ikm, salt: salt);
        return HKDF[H].expand(prk, length: length, info: info);
    }
}
