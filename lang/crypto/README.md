# Crypto

Cryptographic primitives for Kestrel, implemented in pure Kestrel: hash functions, HMAC, key derivation (HKDF, PBKDF2), authenticated encryption (ChaCha20-Poly1305), and a secure random source.

## Installation

```toml
[dependencies]
crypto = { path = "../../lang/crypto" }
```

## Modules

### `crypto.digest` — hash functions

- **Digest** - protocol: `static var digestSize`, `static var blockSize`, `init()`, `mutating func update[S](bytes: S) where S: Slice[UInt8]`, `func finalize() -> DigestOutput`
- **SHA256**, **SHA512**, **MD5**, **BLAKE2b** - conforming structs, each with a one-shot `static func hash(bytes:) -> DigestOutput`. `BLAKE2b` also has `init(outputLength: Int64)` for variable-length digests.
- **DigestOutput** - `.bytes`, `.hexString`; equality (`==` / `equals`) is constant-time

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

- **SymmetricKey** - `init(bytes: Array[UInt8])`, `.bytes`, `.count`. Deliberately no `hexString`/`Hashable` so keys aren't casually printed.

### `crypto.mac` — message authentication

- **HMAC[H]** (RFC 2104), generic over any `Digest` - `init(key: SymmetricKey)`, `update(bytes:)`, `finalize() -> AuthenticationCode`, one-shot `static func authenticate(key:message:)`
- **AuthenticationCode** - `.bytes`, `.hexString`; constant-time equality

```kestrel
import crypto.mac.(HMAC)
import crypto.digest.(SHA256)

let tag = HMAC[SHA256].authenticate(key: key, message: data);
if tag == expected { /* verified, constant-time */ }
```

### `crypto.kdf` — key derivation

- **HKDF[H]** (RFC 5869) - `static func extract(salt:ikm:) -> AuthenticationCode`, `static func expand(prk:info:length:) -> SymmetricKey`, combined `static func deriveKey(ikm:salt:info:length:) -> SymmetricKey`
- **PBKDF2[H]** (RFC 8018) - `static func deriveKey(password:salt:iterations:length:) -> SymmetricKey`

```kestrel
let key = PBKDF2[SHA256].deriveKey(
    password: passwordBytes,
    salt: salt,
    iterations: 100000,
    length: 32
);
```

### `crypto.aead` — authenticated encryption

- **ChaCha20Poly1305** (RFC 8439) - `static func seal(message, using: key)` (overloads take an explicit `nonce:` and/or `authenticating:` AAD) and `static func open(box, using: key) throws CryptoError`
- **Nonce** - 12 bytes; `init()` is random, `init(from: Array[UInt8])?` validates length
- **SealedBox** - `nonce`/`ciphertext`/`tag`, plus `.combined` serialization and `init(combined:)?`
- **CryptoError** - `.AuthenticationFailure`

```kestrel
let sealed = ChaCha20Poly1305.seal(plaintext, using: key);
let plaintext = try ChaCha20Poly1305.open(sealed, using: key);
```

### `crypto.random` — secure randomness

- **SecureRandom** - `RandomNumberGenerator` backed by the OS (`arc4random_buf`); `nextUInt64()`
- **randomBytes(count: Int64) -> Array[UInt8]**
