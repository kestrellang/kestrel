// Known-answer and behavior checks for crypto's core API: SHA-256/512, MD5,
// BLAKE2b, HMAC (RFC 4231), HKDF (RFC 5869), PBKDF2, ChaCha20-Poly1305
// (RFC 8439), ByteValue hex/base64 (RFC 4648), constant-time ==, PHC parsing,
// and the API's defaults. Vectors are the RFCs' own, or Python hashlib/hmac.

module crypto_tests_core

import crypto.digest.(SHA256, SHA512, MD5, BLAKE2b, BLAKE2b512, BLAKE2b256, DigestOutput, Digest)
import crypto.mac.(HMAC, AuthenticationCode)
import crypto.kdf.(HKDF, PBKDF2)
import crypto.key.(SymmetricKey)
import crypto.aead.(ChaCha20Poly1305, Nonce, SealedBox)
import crypto.error.(CryptoError)
import crypto.kdf.(Salt)
import crypto.random.(Token)
import crypto.password.(PasswordHash)
import crypto.bytes.(constantTimeEquals, Base64Style)

func bytes(s: String) -> Array[UInt8] {
    s.toBytes()
}

func hex(b: Array[UInt8]) -> String {
    DigestOutput(bytes: b).hexString
}

func check(name: String, got: String, want: String) {
    if got == want { print("PASS \(name)\n"); } else { print("FAIL \(name)\n  got  \(got)\n  want \(want)\n"); }
}

@main
func main() {
    let abc = bytes("abc");
    check("sha256 abc", SHA256.hash(abc).hexString, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    check("sha256 empty", SHA256.hash(bytes("")).hexString, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    check("sha256 448", SHA256.hash(bytes("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq")).hexString, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    check("sha512 abc", SHA512.hash(abc).hexString, "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f");
    check("md5 abc", MD5.hash(abc).hexString, "900150983cd24fb0d6963f7d28e17f72");
    // RFC 7693 Appendix A: BLAKE2b-512("abc")
    check("blake2b512 abc", BLAKE2b512.hash(abc).hexString, "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923");
    check("blake2b256 abc", BLAKE2b256.hash(abc).hexString, "bddd813c634239723171ef3fee98579b94964e3bb1cb3e427262c8c068d52319");
    check("blake2b512 empty", BLAKE2b512.hash(bytes("")).hexString, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce");
    check("blake2b(outputLength: 64) == blake2b512", "\(BLAKE2b.hash(abc, outputLength: 64).expect("b") == BLAKE2b512.hash(abc))", "true");
    check("blake2b(outputLength: 32) == blake2b256", "\(BLAKE2b.hash(abc, outputLength: 32).expect("b") == BLAKE2b256.hash(abc))", "true");
    check("blake2b(outputLength: 48) count", "\(BLAKE2b.hash(abc, outputLength: 48).expect("b").bytes.count)", "48");
    check("blake2b(outputLength: 65) -> null", "\(BLAKE2b(outputLength: 65).isSome)", "false");
    check("blake2b(outputLength: 0) -> null", "\(BLAKE2b(outputLength: 0).isSome)", "false");
    check("digestSize matches output 512", "\(BLAKE2b512.digestSize == BLAKE2b512.hash(abc).bytes.count)", "true");
    check("hmac-blake2b512 length", "\(HMAC[BLAKE2b512].authenticate(abc, using: SymmetricKey(bytes: bytes("k"))).bytes.count)", "64");
    // 200 bytes over two updates, crossing the 128-byte block, vs one-shot
    var big: [UInt8] = []; for i in 0..<200 { big.append(UInt8(from: i)); }
    var inc = BLAKE2b512();
    inc.update(big.asSlice().prefix(100)); inc.update(big.asSlice().drop(first: 100));
    check("blake2b512 incremental == one-shot", "\(inc.finalize() == BLAKE2b512.hash(big))", "true");
    // incremental == one-shot across block boundary
    var h = SHA256();
    let a = bytes("abcdbcdecdefdefgefghfghighijhijk");
    let b = bytes("ijkljklmklmnlmnomnopnopq");
    h.update(a); h.update(b);
    check("sha256 incremental", h.finalize().hexString, "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1");
    // RFC 4231 case 2
    let tag = HMAC[SHA256].authenticate(bytes("what do ya want for nothing?"), using: SymmetricKey(bytes: bytes("Jefe")));
    check("hmac-sha256 rfc4231#2", tag.hexString, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    // RFC 6070-style PBKDF2-HMAC-SHA256 (c=1, dkLen=32)
    let pk = PBKDF2[SHA256].deriveKey("password".asByteSlice(), salt: bytes("salt"), iterations: 1, length: 32).expect("pbkdf2");
    check("pbkdf2-sha256 c=1", hex(pk.bytes), "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b");
    // RFC 5869 A.1
    var ikm: [UInt8] = []; for _ in 0..<22 { ikm.append(0x0b); }
    var salt: [UInt8] = []; for i in 0..<13 { salt.append(UInt8(from: i)); }
    var info: [UInt8] = []; for i in 0..<10 { info.append(UInt8(from: 0xf0 + i)); }
    let okm = HKDF[SHA256].deriveKey(SymmetricKey(bytes: ikm), length: 42, salt: salt, info: info).expect("hkdf");
    check("hkdf-sha256 rfc5869 A.1", hex(okm.bytes), "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865");
    // AEAD roundtrip + tamper
    var kb: [UInt8] = []; for i in 0..<32 { kb.append(UInt8(from: 0x80 + i)); }
    let key = SymmetricKey(bytes: kb);
    let msg = bytes("Ladies and Gentlemen of the class of '99");
    let sealed = ChaCha20Poly1305.seal(msg, using: key, authenticating: bytes("hdr")).expect("seal");
    let opened = ChaCha20Poly1305.open(sealed, using: key, authenticating: bytes("hdr"));
    match opened { .Ok(p) => check("aead roundtrip", hex(p), hex(msg)), .Err(_) => { print("FAIL aead roundtrip: auth failure"); } }
    match ChaCha20Poly1305.open(sealed, using: key, authenticating: bytes("hdX")) { .Ok(_) => { print("FAIL aead wrong-aad accepted"); }, .Err(_) => { print("PASS aead wrong-aad rejected\n"); } }

    // RFC 8439 2.8.2 AEAD vector (tag)
    let rfcNonce = Nonce(bytes: [0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47]).expect("nonce");
    let rfcAad: [UInt8] = [0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7];
    let rfcMsg = bytes("Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.");
    let rfcBox = ChaCha20Poly1305.seal(rfcMsg, using: key, authenticating: rfcAad, nonce: rfcNonce).expect("seal");
    check("aead rfc8439 2.8.2 tag", hex(rfcBox.tag), "1ae10b594f09e26a7e902ecbd0600691");

    // --- fix 1: malformed SealedBox ---
    let shortTag: [UInt8] = [1, 2, 3, 4, 5];
    match SealedBox(nonce: Nonce.random(), ciphertext: [], tag: shortTag) { .Some(_) => { print("FAIL short tag accepted\n"); }, .None => { print("PASS short tag rejected\n"); } }
    let zeros = Array[UInt8](repeating: 0, count: 28);
    let zbox = SealedBox(combined: zeros).expect("combined");
    match ChaCha20Poly1305.open(zbox, using: key) { .Ok(_) => { print("FAIL zero box opened\n"); }, .Err(.AuthenticationFailure) => { print("PASS zero box -> AuthenticationFailure\n"); }, .Err(_) => { print("FAIL zero box wrong error\n"); } }

    // --- fix 2: key size ---
    let shortKey = SymmetricKey(bytes: Array[UInt8](repeating: 1, count: 16));
    match ChaCha20Poly1305.seal(msg, using: shortKey) { .Err(.InvalidKeySize) => { print("PASS seal 16-byte key -> InvalidKeySize\n"); }, _ => { print("FAIL seal 16-byte key\n"); } }
    match ChaCha20Poly1305.open(sealed, using: shortKey) { .Err(.InvalidKeySize) => { print("PASS open 16-byte key -> InvalidKeySize\n"); }, _ => { print("FAIL open 16-byte key\n"); } }

    // --- fix 3: KDF bounds ---
    let prk = HKDF[SHA256].extract(SymmetricKey(bytes: ikm), salt: salt);
    match HKDF[SHA256].expand(prk, length: 8160, info: info) { .Ok(k) => check("hkdf max length", "\(k.count)", "8160"), .Err(_) => { print("FAIL hkdf max length rejected\n"); } }
    match HKDF[SHA256].expand(prk, length: 8161, info: info) { .Err(.InvalidLength) => { print("PASS hkdf 8161 -> InvalidLength\n"); }, _ => { print("FAIL hkdf 8161\n"); } }
    match HKDF[SHA256].expand(prk, length: -1, info: info) { .Err(.InvalidLength) => { print("PASS hkdf -1 -> InvalidLength\n"); }, _ => { print("FAIL hkdf -1\n"); } }
    match PBKDF2[SHA256].deriveKey(bytes("p"), salt: bytes("s"), iterations: 0, length: 32) { .Err(.InvalidIterationCount) => { print("PASS pbkdf2 0 iters -> InvalidIterationCount\n"); }, _ => { print("FAIL pbkdf2 0 iters\n"); } }
    match PBKDF2[SHA256].deriveKey(bytes("p"), salt: bytes("s"), iterations: 1, length: -1) { .Err(.InvalidLength) => { print("PASS pbkdf2 -1 -> InvalidLength\n"); }, _ => { print("FAIL pbkdf2 -1\n"); } }

    // --- ByteValue ---
    let d = SHA256.hash(abc);
    match DigestOutput(hex: d.hexString) { some back => check("digest hex roundtrip ==", "\(back == d)", "true"), null => { print("FAIL digest fromHex\n"); } }
    match DigestOutput(hex: "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD") { some up => check("uppercase hex ==", "\(up == d)", "true"), null => { print("FAIL uppercase hex\n"); } }
    check("odd hex -> null", "\(DigestOutput(hex: "abc").isSome)", "false");
    check("non-hex -> null", "\(DigestOutput(hex: "zz").isSome)", "false");
    check("digest != other", "\(d == SHA256.hash(bytes("abd")))", "false");
    check("nonce fromHex wrong len -> null", "\(Nonce(hex: "00ff").isSome)", "false");
    check("nonce fromHex 12 bytes", "\(Nonce(hex: "000102030405060708090a0b").isSome)", "true");
    check("salt 7 bytes -> null", "\(Salt(hex: "00010203040506").isSome)", "false");
    check("salt random count", "\(Salt.random().bytes.count)", "16");
    check("salt random(count: 2) -> null", "\(Salt.random(count: 2).isSome)", "false");
    check("salt random(count: 24)", "\(Salt.random(count: 24).expect("s").bytes.count)", "24");
    check("token random(count: 64)", "\(Token.random(count: 64).expect("t").bytes.count)", "64");
    check("nonce random(count: 5) -> null", "\(Nonce.random(count: 5).isSome)", "false");
    check("nonce random()", "\(Nonce.random().bytes.count)", "12");
    check("key random(count: -1) -> null", "\(SymmetricKey.random(count: -1).isSome)", "false");
    check("token random count", "\(Token.random().bytes.count)", "32");
    check("token random distinct", "\(Token.random() == Token.random())", "false");
    check("key random count", "\(SymmetricKey.random().count)", "32");
    check("tag == via fromHex", "\(AuthenticationCode(hex: tag.hexString).expect("t") == tag)", "true");
    check("constantTimeEquals len mismatch", "\(constantTimeEquals([1, 2], [1]))", "false");

    // --- PasswordHash ---
    let ph = PasswordHash.hash("hunter2", with: .PBKDF2(iterations: 1000)).expect("ph");
    check("ph verify right", "\(ph.verify("hunter2"))", "true");
    check("ph verify wrong", "\(ph.verify("hunter3"))", "false");
    match PasswordHash(encoded: ph.encoded) {
        some parsed => {
            check("ph parsed verify", "\(parsed.verify("hunter2"))", "true");
            check("ph parsed scheme", "\(parsed.scheme == .PBKDF2(iterations: 1000))", "true");
            check("ph re-encode stable", "\(parsed.encoded == ph.encoded)", "true");
        },
        null => { print("FAIL ph parse: \(ph.encoded)\n"); }
    }
    print("ENCODED \(ph.encoded)\n");
    check("ph garbage -> null", "\(PasswordHash(encoded: "nope").isSome)", "false");
    check("ph bad scheme -> null", "\(PasswordHash(encoded: "$md5$i=1$AAECAwQFBgc$AA").isSome)", "false");
    check("ph i=0 -> null", "\(PasswordHash(encoded: "$pbkdf2-sha256$i=0$AAECAwQFBgc$AA").isSome)", "false");
    match PasswordHash.hash("x", with: .PBKDF2(iterations: 0)) { .Err(.InvalidIterationCount) => { print("PASS ph 0 iters -> InvalidIterationCount\n"); }, _ => { print("FAIL ph 0 iters\n"); } }

    // --- #1 / #5 ---
    check("generic H.hash", genericHex[SHA256](abc), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    check("hash ArraySlice input", SHA256.hash(abc.asSlice()).hexString, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    check("hash string bytes", SHA256.hash("abc".asByteSlice()).hexString, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    let s1 = ChaCha20Poly1305.seal(msg, using: key).expect("s1");
    let s2 = ChaCha20Poly1305.seal(msg, using: key).expect("s2");
    check("default nonce fresh per call", "\(s1.nonce == s2.nonce)", "false");
    let s3 = ChaCha20Poly1305.seal(msg, using: key, authenticating: rfcAad).expect("s3");
    match ChaCha20Poly1305.open(s3, using: key, authenticating: rfcAad.asSlice()) { .Ok(p) => check("aad default nonce roundtrip", hex(p), hex(msg)), .Err(_) => { print("FAIL aad default nonce\n"); } }
    // --- base64: RFC 4648 §10 ---
    check("b64 empty", Token(bytes: bytes("")).base64String(), "");
    check("b64 f", Token(bytes: bytes("f")).base64String(), "Zg==");
    check("b64 fo", Token(bytes: bytes("fo")).base64String(), "Zm8=");
    check("b64 foo", Token(bytes: bytes("foo")).base64String(), "Zm9v");
    check("b64 foob", Token(bytes: bytes("foob")).base64String(), "Zm9vYg==");
    check("b64 fooba", Token(bytes: bytes("fooba")).base64String(), "Zm9vYmE=");
    check("b64 foobar", Token(bytes: bytes("foobar")).base64String(), "Zm9vYmFy");
    let edge: [UInt8] = [0xfb, 0xff];
    check("b64 standard +/", Token(bytes: edge).base64String(), "+/8=");
    check("b64 unpadded", Token(bytes: edge).base64String(.StandardUnpadded), "+/8");
    check("b64 url -_", Token(bytes: edge).base64String(.URLSafe), "-_8");
    for text in ["Zm9vYmE=", "Zm9vYmE", "-_8", "+/8=", "+/8"] {
        match Token(base64: text) { some t => check("b64 decode \(text)", t.base64String(.StandardUnpadded), Token(bytes: Token(base64: text).expect("x").bytes).base64String(.StandardUnpadded)), null => { print("FAIL b64 decode \(text)\n"); } }
    }
    check("b64 decode foobar", "\(Token(base64: "Zm9vYmFy").expect("x") == Token(bytes: bytes("foobar")))", "true");
    check("b64 decode url == std", "\(Token(base64: "-_8").expect("x") == Token(base64: "+/8=").expect("y"))", "true");
    for bad in ["Z", "Zg=", "Zg===", "Zh==", "Zm9v!", "Zm=9v", "====", "Zm9vY"] {
        check("b64 reject \(bad)", "\(Token(base64: bad).isSome)", "false");
    }
    check("b64 nonce length enforced", "\(Nonce(base64: "AAECAwQFBgcICQ").isSome)", "false");
    let roundTrip = Token.random(count: 31).expect("t");
    check("b64 random roundtrip", "\(Token(base64: roundTrip.base64String(.URLSafe)).expect("r") == roundTrip)", "true");

    // --- PHC interop: hashed by Python hashlib, verified here ---
    match PasswordHash(encoded: "$pbkdf2-sha256$i=1000,l=32$AAECAwQFBgcICQoLDA0ODw$yRTMTwbMbo9G0VfjobWqerzuuxe7BETNTErBbKKumGQ") {
        some py => {
            check("phc python verify", "\(py.verify("correct horse"))", "true");
            check("phc python wrong", "\(py.verify("battery staple"))", "false");
            check("phc python re-encode", py.encoded, "$pbkdf2-sha256$i=1000,l=32$AAECAwQFBgcICQoLDA0ODw$yRTMTwbMbo9G0VfjobWqerzuuxe7BETNTErBbKKumGQ");
        },
        null => { print("FAIL phc python parse\n"); }
    }
    check("phc params any order", "\(PasswordHash(encoded: "$pbkdf2-sha256$l=32,i=1000$AAECAwQFBgcICQoLDA0ODw$yRTMTwbMbo9G0VfjobWqerzuuxe7BETNTErBbKKumGQ").isSome)", "true");
    check("phc l mismatch -> null", "\(PasswordHash(encoded: "$pbkdf2-sha256$i=1000,l=31$AAECAwQFBgcICQoLDA0ODw$yRTMTwbMbo9G0VfjobWqerzuuxe7BETNTErBbKKumGQ").isSome)", "false");
    check("phc unknown param -> null", "\(PasswordHash(encoded: "$pbkdf2-sha256$i=1000,l=32,x=1$AAECAwQFBgcICQoLDA0ODw$yRTMTwbMbo9G0VfjobWqerzuuxe7BETNTErBbKKumGQ").isSome)", "false");
    check("phc no l ok", "\(PasswordHash(encoded: "$pbkdf2-sha256$i=1000$AAECAwQFBgcICQoLDA0ODw$yRTMTwbMbo9G0VfjobWqerzuuxe7BETNTErBbKKumGQ").isSome)", "true");

    // Defaults that needed the compiler fixes.
    let hkIkm = SymmetricKey(bytes: bytes("input key material"));
    let empty: [UInt8] = [];
    let hkDefault = HKDF[SHA256].deriveKey(hkIkm, length: 32).expect("hkdf default");
    let hkExplicit = HKDF[SHA256].deriveKey(hkIkm, length: 32, salt: empty, info: empty).expect("hkdf explicit");
    check("hkdf omitted salt/info == empty", "\(hkDefault == hkExplicit)", "true");
    let hkInfoOnly = HKDF[SHA256].deriveKey(hkIkm, length: 32, info: bytes("ctx")).expect("hkdf info");
    let hkBoth = HKDF[SHA256].deriveKey(hkIkm, length: 32, salt: empty, info: bytes("ctx")).expect("hkdf both");
    check("hkdf skip salt, give info", "\(hkInfoOnly == hkBoth)", "true");
    check("hkdf expand default info", "\(HKDF[SHA256].expand(hkIkm, length: 16).expect("e") == HKDF[SHA256].expand(hkIkm, length: 16, info: empty).expect("f"))", "true");
    let plainBox = ChaCha20Poly1305.seal(bytes("no aad"), using: key).expect("seal no aad");
    match ChaCha20Poly1305.open(plainBox, using: key) { .Ok(p) => check("seal/open omitted aad", hex(p), hex(bytes("no aad"))), .Err(_) => { print("FAIL seal/open omitted aad\n"); } }
    match ChaCha20Poly1305.open(plainBox, using: key, authenticating: bytes("x")) { .Ok(_) => { print("FAIL omitted aad != x\n"); }, .Err(_) => { print("PASS omitted aad != x\n"); } }
    check("token init(hex:) plain init", "\(Token(hex: "00ff").expect("t").bytes.count)", "2");
    check("key init(base64:)", "\(SymmetricKey(base64: "AAEC").expect("k").count)", "3");
    check("random() via static var", "\(Token.random().bytes.count) \(Nonce.random().bytes.count) \(Salt.random().bytes.count)", "32 12 16");
}

func genericHex[H](data: Array[UInt8]) -> String where H: Digest { H.hash(data).hexString }