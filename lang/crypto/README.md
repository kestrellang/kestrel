# Crypto

Cryptographic primitives for Kestrel, implemented in pure Kestrel: hash functions, HMAC, key derivation (HKDF, PBKDF2), password hashing, authenticated encryption (ChaCha20-Poly1305), and a secure random source.

## Installation

```toml
[dependencies]
crypto = { path = "../../lang/crypto" }
```

## Conventions

- **The data being processed is positional; everything else is labeled.** `SHA256.hash(data)`, `PBKDF2[SHA256].deriveKey(password, salt: …)`, `ChaCha20Poly1305.seal(message, using: key)`.
- **Keys are passed as `using:`** in operations (`seal`, `open`, `authenticate`); initializers that hold a key take `key:`.
- **Byte inputs are `some Slice[UInt8]`**, so `Array[UInt8]` and `ArraySlice[UInt8]` both work. For a `String`, pass `s.asByteSlice()` (no copy). Values a type stores (`SymmetricKey(bytes:)`, `Salt(bytes:)`, …) take an `Array[UInt8]`.

## Modules

### `crypto.bytes` — typed byte values

Every byte-valued type here (`DigestOutput`, `AuthenticationCode`, `SymmetricKey`, `Nonce`, `Salt`, `Token`) conforms to **ByteValue**. A conforming type declares `.bytes` and `init(bytes:)?` (a type that accepts any bytes declares a plain `init(bytes:)`), and gets:

- `.hexString` — lowercase hex
- `init(hex:)?` — accepts either case; null on bad hex or bytes invalid for the type
- `base64String(style: = .Standard)` — RFC 4648 base64; **Base64Style** is `.Standard` (padded; MIME, JSON, most APIs), `.StandardUnpadded` (PHC strings) or `.URLSafe` (`-_`, unpadded; tokens, URLs, JWT)
- `init(base64:)?` — accepts every style, padded or not; rejects malformed padding and non-canonical trailing bits
- constant-time `==`

Compare the typed values, not `.bytes` or `.hexString` — those comparisons are not timing-safe. `constantTimeEquals(a, b)` is there for raw arrays.

**RandomByteValue** (in `crypto.random`) adds generation to `SymmetricKey` (32 bytes), `Nonce` (12), `Salt` (16) and `Token` (32): `T.random()` makes the type's default length, `T.random(count:)` returns null for a length the type rejects (anything but 12 for a `Nonce`, under 8 for a `Salt`). A conforming type only declares `static var randomCount`.

### `crypto.digest` — hash functions

- **Digest** - protocol: `static var digestSize`, `static var blockSize`, `init()`, `mutating func update[S](bytes: S) where S: Slice[UInt8]`, `func finalize() -> DigestOutput`
- **`H.hash(data) -> DigestOutput`** - one-shot hash, defined once on `Digest`, so generic code can call it too
- Conforming structs; each type's `digestSize` is exactly what it outputs, and every one works with `HMAC`, `HKDF` and `PBKDF2`:

  | Family | Types |
  |---|---|
  | SHA-2 | **SHA224**, **SHA256**, **SHA384**, **SHA512**, **SHA512_256** |
  | SHA-3 | **SHA3_224**, **SHA3_256**, **SHA3_384**, **SHA3_512** |
  | BLAKE | **BLAKE2b512**, **BLAKE2b256**, **BLAKE3** (plain 32-byte hash) |
  | Legacy | **SHA1** (WebSocket handshake, TOTP, git), **MD5** — not collision-resistant; never for signatures |
- **SHAKE128**, **SHAKE256** (FIPS 202) - extendable output, not a `Digest`: `update`, `finalize(outputLength:) -> DigestOutput?` (non-mutating; any length ≥ 1) and `static func hash(data, outputLength:)`. 128/256-bit security once the output is at least 32/64 bytes
- **BLAKE2b** - variable output length (1–64 bytes), not a `Digest`: `init(outputLength:)?` and `static func hash(data, outputLength:) -> DigestOutput?`, both null outside 1–64. Use `BLAKE2b512`/`BLAKE2b256` with HMAC, HKDF and PBKDF2
- **DigestOutput** - `.bytes`, plus `ByteValue` (hex, constant-time `==`); `Hashable`

```kestrel
import crypto.digest.(SHA256)

// One-shot
let hex = SHA256.hash(data).hexString;

// Incremental
var hasher = SHA256();
hasher.update(part1);
hasher.update(part2);
let output = hasher.finalize();
```

### `crypto.key` — key material

- **SymmetricKey** - `init(bytes: Array[UInt8])`, `.bytes`, `.count`, plus `ByteValue`. Deliberately not `Hashable` or formattable, so keys don't land in dictionaries or logs by accident.

### `crypto.mac` — message authentication

- **HMAC[H]** (RFC 2104), generic over any `Digest` - `init(key: SymmetricKey)`, `update(bytes:)`, `finalize() -> AuthenticationCode`, one-shot `static func authenticate(message, using: key)`
- **AuthenticationCode** - `.bytes`, plus `ByteValue`; `Hashable`. Verify a received tag with `AuthenticationCode(hex: received) == tag`

```kestrel
import crypto.mac.(HMAC)
import crypto.digest.(SHA256)

let tag = HMAC[SHA256].authenticate(data, using: key);
if tag == expected { /* verified, constant-time */ }
```

### `crypto.kdf` — key derivation

- **HKDF[H]** (RFC 5869) - `static func extract(ikm, salt: = []) -> SymmetricKey`, `static func expand(prk, length:, info: = []) -> SymmetricKey throws CryptoError`, combined `static func deriveKey(ikm, length:, salt: = [], info: = []) -> SymmetricKey throws CryptoError`. `salt` and `info` default to empty (an empty salt means a digest-length run of zeros, per the RFC). Throws `.InvalidLength` for a negative length or one over 255 × digest size
- **Salt** - at least 8 bytes; `init(bytes:)?`, plus `ByteValue`; `Hashable`
- **Argon2id** (RFC 9106) - `static func deriveKey(password, salt:memory:iterations:parallelism:length:) -> SymmetricKey throws CryptoError`; memory in KiB
- **Scrypt** (RFC 7914) - `static func deriveKey(password, salt:logN:blockSize:parallelism:length:) -> SymmetricKey throws CryptoError`
- **PBKDF2[H]** (RFC 8018) - `static func deriveKey(password, salt:iterations:length:) -> SymmetricKey throws CryptoError`. Throws `.InvalidIterationCount` below 1 iteration and `.InvalidLength` for a negative length

```kestrel
let key = try PBKDF2[SHA256].deriveKey(
    password.asByteSlice(),
    salt: salt,
    iterations: 100000,
    length: 32
);
```

### `crypto.aead` — authenticated encryption

- **ChaCha20Poly1305** (RFC 8439) - `seal(message, using: key, authenticating: aad = [], nonce: = Nonce.random()) throws CryptoError` and `open(box, using: key, authenticating: aad = []) throws CryptoError`. `aad` is authenticated but not encrypted, and `open` must be given the same bytes. Both throw `.InvalidKeySize` unless the key is 32 bytes (`ChaCha20Poly1305.keySize`)
- **Nonce** - 12 bytes; `init(bytes:)?` validates length, plus `ByteValue`
- **SealedBox** - read-only `nonce`/`ciphertext`/`tag`; `init(nonce:ciphertext:tag:)?` (null unless the tag is 16 bytes), `.combined` serialization and `init(combined:)?`

```kestrel
let sealed = try ChaCha20Poly1305.seal(plaintext, using: key);
let plaintext = try ChaCha20Poly1305.open(sealed, using: key);
```

### `crypto.password` — password hashing

- **PasswordHash** - a salted hash carrying its scheme and costs. `static func hash(password)` uses `PasswordScheme.recommended` (Argon2id, 19 MiB, 2 passes — about 0.13 s at `-O1`); `hash(password, with: scheme) throws CryptoError` picks another. `verify(password) -> Bool` (constant-time compare), `.scheme`, `needsRehash(for: = .recommended)`, and `.encoded` / `init(encoded:)?` for storage
- **PasswordScheme** - `.Argon2id(memory:iterations:parallelism:)`, `.Scrypt(logN:blockSize:parallelism:)`, `.PBKDF2(iterations:)` (HMAC-SHA256), `.Bcrypt(cost:)`. bcrypt reads only 72 bytes of password, so hashing a longer one throws `.InvalidLength` (and one containing NUL throws `.InvalidParameter`); verifying existing bcrypt hashes follows the usual truncation
- `.encoded` is each scheme's standard string, so hashes move between libraries:

  | Scheme | Format | Checked against |
  |---|---|---|
  | Argon2id | `$argon2id$v=19$m=…,t=…,p=…$<salt>$<key>` | OpenSSL |
  | scrypt | `$scrypt$ln=…,r=…,p=…$<salt>$<key>` | Python `hashlib` |
  | PBKDF2 | `$pbkdf2-sha256$i=…,l=…$<salt>$<key>` (Rust `password-hash` layout) | Python `hashlib` |
  | bcrypt | `$2b$NN$<salt><key>`; parses `$2a$`/`$2y$` too | `htpasswd` |

  PHC fields are unpadded base64. passlib's `$pbkdf2-sha256$` variant is not supported.

```kestrel
import crypto.password.(PasswordHash)

let stored = PasswordHash.hash(password);
save(stored.encoded);

guard let some stored = PasswordHash(encoded: row) else { return false; }
guard stored.verify(attempt) else { return false; }
if stored.needsRehash() { save(PasswordHash.hash(attempt).encoded); }
```

### `crypto.error` — errors

- **CryptoError** - `.AuthenticationFailure`, `.InvalidKeySize`, `.InvalidLength`, `.InvalidIterationCount`, `.InvalidParameter`

### `crypto.random` — secure randomness

- **SecureRandom** - `RandomNumberGenerator` backed by the OS (`arc4random_buf`); `nextUInt64()`
- **Token** - random token for sessions, API keys, reset links; `init(bytes:)`, plus `ByteValue`; `Hashable`
- **randomBytes(count: Int64) -> Array[UInt8]**

## Testing

`lang/crypto/tests/run.sh [path/to/kestrel] [opt-level]` builds and runs three check programs against the in-repo compiler (`-O 1` by default) and fails if any check fails:

- `core.ks`: the core API (SHA-2, MD5, BLAKE2b, HMAC, HKDF, PBKDF2, ChaCha20-Poly1305, hex/base64, defaults)
- `digests.ks`: every digest across lengths that straddle block boundaries
- `passwords.ks`: Argon2id, scrypt, bcrypt and `PasswordHash` encoding

Expected values come from the RFCs, Python `hashlib`/`hmac`, OpenSSL, `htpasswd` and the BLAKE3 C reference.
