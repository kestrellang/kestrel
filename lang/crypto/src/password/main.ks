module crypto.password

import crypto.bytes.(Base64Style)
import crypto.digest.(SHA256)
import crypto.error.(CryptoError)
import crypto.kdf.(Argon2id, PBKDF2, Salt, Scrypt)
import crypto.key.(SymmetricKey)

/// How a password is hashed, and with what cost.
///
/// `recommended` is what `PasswordHash.hash(_:)` uses. Compare a stored
/// hash's `scheme` against it (`needsRehash`) to upgrade users on login.
public enum PasswordScheme: Equatable {
    /// Argon2id (RFC 9106): memory in KiB. OWASP's first choice.
    case Argon2id(memory: Int64, iterations: Int64, parallelism: Int64)
    /// scrypt (RFC 7914): N = 2^logN. OWASP minimum `logN: 17, blockSize: 8, parallelism: 1`.
    case Scrypt(logN: Int64, blockSize: Int64, parallelism: Int64)
    /// PBKDF2-HMAC-SHA256. OWASP minimum 600,000 iterations.
    case PBKDF2(iterations: Int64)
    /// bcrypt, cost 4–31. Only reads 72 bytes of password; OWASP minimum cost 10.
    case Bcrypt(cost: Int64)

    /// OWASP's 2023 Argon2id minimum: 19 MiB, 2 passes, 1 lane.
    public static var recommended: PasswordScheme {
        .Argon2id(memory: 19456, iterations: 2, parallelism: 1)
    }

    /// Schemes are equal when their algorithm and every cost parameter match.
    public func isEqual(to other: PasswordScheme) -> Bool {
        self.identifier == other.identifier and self.parameters == other.parameters
    }

    /// The PHC algorithm id (bcrypt's is `2b`).
    var identifier: String {
        match self {
            .Argon2id(memory: _, iterations: _, parallelism: _) => "argon2id",
            .Scrypt(logN: _, blockSize: _, parallelism: _) => "scrypt",
            .PBKDF2(iterations: _) => "pbkdf2-sha256",
            .Bcrypt(cost: _) => "2b"
        }
    }

    /// The PHC parameter field (bcrypt's is its two-digit cost).
    var parameters: String {
        match self {
            .Argon2id(memory: m, iterations: t, parallelism: p) => "m=\(m),t=\(t),p=\(p)",
            .Scrypt(logN: n, blockSize: r, parallelism: p) => "ln=\(n),r=\(r),p=\(p)",
            .PBKDF2(iterations: i) => "i=\(i)",
            .Bcrypt(cost: c) => "\(c:02)"
        }
    }
}

/// A salted password hash that carries everything needed to verify it:
/// the scheme, its cost parameters, the salt and the derived key.
///
/// Store `encoded` in one column. It's the standard string for each
/// scheme, so hashes move to and from other libraries:
///
/// ```text
/// $argon2id$v=19$m=19456,t=2,p=1$<salt>$<key>      PHC
/// $scrypt$ln=17,r=8,p=1$<salt>$<key>                PHC
/// $pbkdf2-sha256$i=600000,l=32$<salt>$<key>         PHC (Rust password-hash layout)
/// $2b$12$<22-char salt><31-char key>                bcrypt
/// ```
///
/// Parsing also accepts bcrypt's `$2a$` and `$2y$` prefixes.
///
/// # Examples
///
/// ```
/// // Register
/// let stored = PasswordHash.hash(password);
/// saveUser(email, stored.encoded);
///
/// // Log in
/// guard let some stored = PasswordHash(encoded: row.passwordHash) else { return false; }
/// guard stored.verify(attempt) else { return false; }
/// if stored.needsRehash() { saveUser(email, PasswordHash.hash(attempt).encoded); }
/// ```
public struct PasswordHash {
    private var schemeValue: PasswordScheme;
    private var salt: Salt;
    private var derivedKey: SymmetricKey;

    private init(scheme scheme: PasswordScheme, salt salt: Salt, derivedKey derivedKey: SymmetricKey) {
        self.schemeValue = scheme;
        self.salt = salt;
        self.derivedKey = derivedKey;
    }

    /// Hashes `password` with a fresh random salt and `PasswordScheme.recommended`.
    public static func hash(password: String) -> PasswordHash {
        PasswordHash.hash(password, with: PasswordScheme.recommended)
            .expect("the recommended parameters are valid")
    }

    /// Hashes `password` with a fresh random salt and the given scheme.
    ///
    /// Throws the underlying KDF's error for out-of-range costs
    /// (`.InvalidParameter`, `.InvalidIterationCount`). For bcrypt, also
    /// throws `.InvalidLength` for a password over 72 bytes and
    /// `.InvalidParameter` for one containing a NUL byte, rather than
    /// silently hashing only part of it.
    public static func hash(password: String, with scheme: PasswordScheme) -> PasswordHash throws CryptoError {
        if let .Bcrypt(cost: _) = scheme {
            let bytes = password.toBytes();
            if bytes.count > 72 { throw CryptoError.InvalidLength; }
            for b in bytes {
                if b == 0 { throw CryptoError.InvalidParameter; }
            }
        }
        let salt = Salt.random();
        let derivedKey = try PasswordHash.derive(password, salt, scheme, PasswordHash.keyLength(scheme));
        .Ok(PasswordHash(scheme: scheme, salt: salt, derivedKey: derivedKey))
    }

    /// Parses any format `encoded` produces (plus bcrypt's `$2a$`/`$2y$`).
    /// Returns null for anything else, including unknown or missing
    /// parameters and a PBKDF2 `l` that doesn't match the key.
    public init(encoded encoded: String)? {
        guard let some parsed = PasswordHash.parse(encoded) else { return null; }
        self.schemeValue = parsed.schemeValue;
        self.salt = parsed.salt;
        self.derivedKey = parsed.derivedKey;
    }

    /// The storable string for this hash's scheme (see the type docs).
    public var encoded: String {
        let scheme = self.schemeValue;
        if let .Bcrypt(cost: _) = scheme {
            return "$2b$\(scheme.parameters)$\(Bcrypt.encode(self.salt.bytes))\(Bcrypt.encode(self.derivedKey.bytes))";
        }
        let salt = self.salt.base64String(.StandardUnpadded);
        let key = self.derivedKey.base64String(.StandardUnpadded);
        if let .Argon2id(memory: _, iterations: _, parallelism: _) = scheme {
            return "$argon2id$v=19$\(scheme.parameters)$\(salt)$\(key)";
        }
        if let .PBKDF2(iterations: _) = scheme {
            return "$pbkdf2-sha256$\(scheme.parameters),l=\(self.derivedKey.count)$\(salt)$\(key)";
        }
        "$\(scheme.identifier)$\(scheme.parameters)$\(salt)$\(key)"
    }

    /// The scheme and cost parameters this hash was made with.
    public var scheme: PasswordScheme { self.schemeValue }

    /// Whether `password` matches. The final comparison is constant-time.
    public func verify(password: String) -> Bool {
        guard let .Ok(candidate) = PasswordHash.derive(password, self.salt, self.schemeValue, self.derivedKey.count) else {
            return false;
        }
        candidate == self.derivedKey
    }

    /// Whether this hash was made with anything other than `scheme` —
    /// a weaker cost, or another algorithm. Check after a successful
    /// `verify` and store a fresh `hash` of the password if so.
    public func needsRehash(for scheme: PasswordScheme = PasswordScheme.recommended) -> Bool {
        not (self.schemeValue == scheme)
    }

    // --- internals ---

    static func keyLength(scheme: PasswordScheme) -> Int64 {
        if let .Bcrypt(cost: _) = scheme { return 23; }
        32
    }

    /// Runs the scheme's KDF.
    static func derive(password: String, salt: Salt, scheme: PasswordScheme, length: Int64) -> SymmetricKey throws CryptoError {
        let input = password.asByteSlice();
        match scheme {
            .Argon2id(memory: m, iterations: t, parallelism: p) =>
                Argon2id.deriveKey(input, salt: salt.bytes, memory: m, iterations: t, parallelism: p, length: length),
            .Scrypt(logN: n, blockSize: r, parallelism: p) =>
                Scrypt.deriveKey(input, salt: salt.bytes, logN: n, blockSize: r, parallelism: p, length: length),
            .PBKDF2(iterations: i) =>
                PBKDF2[SHA256].deriveKey(input, salt: salt.bytes, iterations: i, length: length),
            .Bcrypt(cost: c) => PasswordHash.bcrypt(password, salt, c)
        }
    }

    static func bcrypt(password: String, salt: Salt, cost: Int64) -> SymmetricKey throws CryptoError {
        if cost < 4 or cost > 31 { throw CryptoError.InvalidParameter; }
        if salt.bytes.count != 16 { throw CryptoError.InvalidParameter; }
        .Ok(SymmetricKey(bytes: Bcrypt.digest(password.toBytes(), salt.bytes, cost)))
    }

    static func parse(encoded: String) -> PasswordHash? {
        let parts = encoded.split("$").collect();
        if parts.count < 4 or parts(0) != "" { return null; }
        let id = parts(1).toOwned();
        if id == "2a" or id == "2b" or id == "2y" {
            return PasswordHash.parseBcrypt(parts(2).toOwned(), parts(3).toOwned(), parts.count);
        }

        // PHC: $id[$v=19]$params$salt$key
        var field: Int64 = 2;
        if id == "argon2id" {
            if parts.count != 6 or parts(2) != "v=19" { return null; }
            field = 3;
        } else if parts.count != 5 {
            return null;
        }
        guard let some params = PHCParameters.parse(parts(field).toOwned()) else { return null; }
        guard let some salt = Salt(base64: parts(field + 1).toOwned()) else { return null; }
        guard let some derivedKey = SymmetricKey(base64: parts(field + 2).toOwned()) else { return null; }
        if derivedKey.count == 0 { return null; }

        var scheme = PasswordScheme.recommended;
        if id == "argon2id" {
            guard params.onlyUses(["m", "t", "p"]) else { return null; }
            guard let some m = params.value("m") else { return null; }
            guard let some t = params.value("t") else { return null; }
            guard let some p = params.value("p") else { return null; }
            scheme = .Argon2id(memory: m, iterations: t, parallelism: p);
        } else if id == "scrypt" {
            guard params.onlyUses(["ln", "r", "p"]) else { return null; }
            guard let some n = params.value("ln") else { return null; }
            guard let some r = params.value("r") else { return null; }
            guard let some p = params.value("p") else { return null; }
            scheme = .Scrypt(logN: n, blockSize: r, parallelism: p);
        } else if id == "pbkdf2-sha256" {
            guard params.onlyUses(["i", "l"]) else { return null; }
            guard let some i = params.value("i") else { return null; }
            if let some l = params.value("l") {
                if l != derivedKey.count { return null; }
            }
            if i < 1 { return null; }
            scheme = .PBKDF2(iterations: i);
        } else {
            return null;
        }
        PasswordHash(scheme: scheme, salt: salt, derivedKey: derivedKey)
    }

    /// `$2b$NN$` + 22 salt characters + 31 key characters.
    static func parseBcrypt(costText: String, rest: String, partCount: Int64) -> PasswordHash? {
        if partCount != 4 or costText.toBytes().count != 2 or rest.toBytes().count != 53 { return null; }
        guard let some cost = Int64(parsing: costText) else { return null; }
        if cost < 4 or cost > 31 { return null; }
        let chars = rest.toBytes();
        var saltChars: [UInt8] = [];
        saltChars.append(contentsOf: chars.asSlice().prefix(22));
        var keyChars: [UInt8] = [];
        keyChars.append(contentsOf: chars.asSlice().drop(first: 22));
        guard let some saltBytes = Bcrypt.decode(String(fromUtf8Unchecked: saltChars), 16) else { return null; }
        guard let some keyBytes = Bcrypt.decode(String(fromUtf8Unchecked: keyChars), 23) else { return null; }
        guard let some salt = Salt(bytes: saltBytes) else { return null; }
        PasswordHash(scheme: .Bcrypt(cost: cost), salt: salt, derivedKey: SymmetricKey(bytes: keyBytes))
    }
}

/// A PHC parameter field, `name=value,name=value`, with integer values.
struct PHCParameters {
    var names: Array[String];
    var values: Array[Int64];

    init(names: Array[String], values: Array[Int64]) {
        self.names = names;
        self.values = values;
    }

    /// Null on a malformed pair, a non-integer value or a repeated name.
    static func parse(text: String) -> PHCParameters? {
        var names: [String] = [];
        var values: [Int64] = [];
        for pair in text.split(",") {
            let fields = pair.toOwned().split("=").collect();
            if fields.count != 2 { return null; }
            let name = fields(0).toOwned();
            guard let some value = Int64(parsing: fields(1).toOwned()) else { return null; }
            for existing in names {
                if existing == name { return null; }
            }
            names.append(name);
            values.append(value);
        }
        PHCParameters(names, values)
    }

    func value(name: String) -> Int64? {
        for i in 0..<self.names.count {
            if self.names(i) == name { return self.values(i); }
        }
        null
    }

    /// Whether every name is one of `allowed`.
    func onlyUses(allowed: Array[String]) -> Bool {
        for name in self.names {
            var known = false;
            for candidate in allowed {
                if candidate == name { known = true; }
            }
            if not known { return false; }
        }
        true
    }
}
