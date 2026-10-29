module crypto.kdf

import crypto.digest.(BLAKE2b, BLAKE2b512)
import crypto.error.(CryptoError)
import crypto.key.(SymmetricKey)

/// Argon2id (RFC 9106, version 0x13): the memory-hard password KDF OWASP
/// recommends first. It fills `memory` KiB with BLAKE2b-derived blocks,
/// reading them back data-independently in the first half-pass (resisting
/// side channels) and data-dependently after (resisting GPU trade-offs).
///
/// OWASP's 2023 minimum is `memory: 19456, iterations: 2, parallelism: 1`
/// (19 MiB). `PasswordHash` wraps this for storing passwords.
///
/// # Examples
///
/// ```
/// let key = try Argon2id.deriveKey(password.asByteSlice(), salt: salt, memory: 19456, iterations: 2, parallelism: 1, length: 32);
/// ```
public struct Argon2id {
    /// Throws `.InvalidParameter` unless `parallelism` is 1–16777215 and
    /// `memory` is at least `8 × parallelism` KiB (and below 2^32);
    /// `.InvalidIterationCount` below 1 iteration; `.InvalidLength` if
    /// `length` is below 4 or the salt is shorter than 8 bytes.
    public static func deriveKey(
        password: some Slice[UInt8],
        salt salt: some Slice[UInt8],
        memory memory: Int64,
        iterations iterations: Int64,
        parallelism parallelism: Int64,
        length length: Int64
    ) -> SymmetricKey throws CryptoError {
        if parallelism < 1 or parallelism > 16777215 { throw CryptoError.InvalidParameter; }
        if memory < 8 * parallelism or memory > 4294967295 { throw CryptoError.InvalidParameter; }
        if iterations < 1 { throw CryptoError.InvalidIterationCount; }
        if length < 4 or length > 4294967295 { throw CryptoError.InvalidLength; }
        let passwordBytes = password.asSlice();
        let saltBytes = salt.asSlice();
        if saltBytes.count < 8 { throw CryptoError.InvalidLength; }

        // H0 (RFC 9106 §3.2): every parameter and input, length-prefixed.
        var h0 = BLAKE2b512();
        h0.update(Argon2id.le32(parallelism));
        h0.update(Argon2id.le32(length));
        h0.update(Argon2id.le32(memory));
        h0.update(Argon2id.le32(iterations));
        h0.update(Argon2id.le32(0x13));          // version
        h0.update(Argon2id.le32(2));             // type: Argon2id
        h0.update(Argon2id.le32(passwordBytes.count));
        h0.update(passwordBytes);
        h0.update(Argon2id.le32(saltBytes.count));
        h0.update(saltBytes);
        h0.update(Argon2id.le32(0));             // no secret
        h0.update(Argon2id.le32(0));             // no associated data
        let seed = h0.finalize().bytes;

        // Memory: p lanes × q columns of 1 KiB blocks (128 words each),
        // with q a multiple of 4 so each lane splits into 4 segments.
        let blockCount = 4 * parallelism * (memory / (4 * parallelism));
        var state = Argon2State(
            parallelism,
            blockCount / parallelism,
            blockCount,
            iterations
        );

        // The first two blocks of each lane come straight from H0.
        for lane in 0..<parallelism {
            for column in 0..<2 {
                var input: [UInt8] = [];
                input.append(contentsOf: seed.asSlice());
                input.append(contentsOf: Argon2id.le32(column).asSlice());
                input.append(contentsOf: Argon2id.le32(lane).asSlice());
                state.store(Argon2id.wordsFromBytes(Argon2id.variableHash(1024, input)), lane, column);
            }
        }

        for pass in 0..<iterations {
            for slice in 0..<4 {
                for lane in 0..<parallelism {
                    state.fillSegment(pass, lane, slice);
                }
            }
        }

        // The tag hashes the XOR of every lane's last block.
        var last = state.block(0, state.columns - 1);
        for lane in 1..<parallelism {
            let other = state.block(lane, state.columns - 1);
            for k in 0..<128 { last(k) = last(k) ^ other(k); }
        }
        .Ok(SymmetricKey(bytes: Argon2id.variableHash(length, Argon2id.bytesFromWords(last))))
    }

    // --- internals ---

    /// H' (RFC 9106 §3.3): BLAKE2b stretched to any length by chaining
    /// 64-byte hashes and keeping the first 32 bytes of each.
    static func variableHash(length: Int64, input: Array[UInt8]) -> Array[UInt8] {
        var prefixed: [UInt8] = [];
        prefixed.append(contentsOf: Argon2id.le32(length).asSlice());
        prefixed.append(contentsOf: input.asSlice());
        if length <= 64 {
            return BLAKE2b.hash(prefixed, outputLength: length).expect("1...64").bytes;
        }

        let r = (length + 31) / 32 - 2;
        var v = BLAKE2b512.hash(prefixed).bytes;
        var result: [UInt8] = [];
        result.append(contentsOf: v.asSlice().prefix(32));
        for _ in 1..<r {
            v = BLAKE2b512.hash(v).bytes;
            result.append(contentsOf: v.asSlice().prefix(32));
        }
        let tail = BLAKE2b.hash(v, outputLength: length - 32 * r).expect("33...64");
        result.append(contentsOf: tail.bytes.asSlice());
        result
    }

    static func le32(value: Int64) -> Array[UInt8] {
        UInt32(from: value).toBytesLittleEndian()
    }

    static func wordsFromBytes(bytes: Array[UInt8]) -> Array[UInt64] {
        var words = Array[UInt64](repeating: 0, count: bytes.count / 8);
        for i in 0..<words.count {
            var w: UInt64 = 0;
            for j in 0..<8 { w = w | (UInt64(from: bytes(8 * i + j)) << (8 * j)); }
            words(i) = w;
        }
        words
    }

    static func bytesFromWords(words: Array[UInt64]) -> Array[UInt8] {
        var bytes: [UInt8] = [];
        for w in words { bytes.append(contentsOf: w.toBytesLittleEndian().asSlice()); }
        bytes
    }
}

/// Argon2's memory matrix and the block-filling passes over it.
struct Argon2State {
    var memory: Array[UInt64];
    var lanes: Int64;
    var columns: Int64;
    var segment: Int64;
    var blockCount: Int64;
    var iterations: Int64;

    init(lanes: Int64, columns: Int64, blockCount: Int64, iterations: Int64) {
        self.memory = Array[UInt64](repeating: 0, count: blockCount * 128);
        self.lanes = lanes;
        self.columns = columns;
        self.segment = columns / 4;
        self.blockCount = blockCount;
        self.iterations = iterations;
    }

    func block(lane: Int64, column: Int64) -> Array[UInt64] {
        var result = Array[UInt64](repeating: 0, count: 128);
        let base = (lane * self.columns + column) * 128;
        for k in 0..<128 { result(k) = self.memory(base + k); }
        result
    }

    mutating func store(block: Array[UInt64], lane: Int64, column: Int64) {
        let base = (lane * self.columns + column) * 128;
        for k in 0..<128 { self.memory(base + k) = block(k); }
    }

    /// Fills one segment (RFC 9106 §3.4): each block mixes the previous
    /// block with a reference block chosen by `indexAlpha`.
    mutating func fillSegment(pass: Int64, lane: Int64, slice: Int64) {
        // Argon2id: Argon2i (data-independent) addressing for the first two
        // slices of the first pass, Argon2d (data-dependent) after.
        let independent = pass == 0 and slice < 2;
        var addresses = Array[UInt64](repeating: 0, count: 128);
        var addressInput = Array[UInt64](repeating: 0, count: 128);
        if independent {
            addressInput(0) = UInt64(from: pass);
            addressInput(1) = UInt64(from: lane);
            addressInput(2) = UInt64(from: slice);
            addressInput(3) = UInt64(from: self.blockCount);
            addressInput(4) = UInt64(from: self.iterations);
            let argon2idType: UInt64 = 2;
            addressInput(5) = argon2idType;
        }

        // The first two blocks of each lane already exist.
        var start: Int64 = 0;
        if pass == 0 and slice == 0 {
            start = 2;
            if independent { Argon2State.nextAddresses(addressInput, addresses); }
        }

        for i in start..<self.segment {
            if independent and i % 128 == 0 {
                Argon2State.nextAddresses(addressInput, addresses);
            }
            let column = slice * self.segment + i;
            let previous = if column == 0 { self.columns - 1 } else { column - 1 };

            let pseudoRandom = if independent {
                addresses(i % 128)
            } else {
                self.memory((lane * self.columns + previous) * 128)
            };
            var referenceLane = Int64(from: (pseudoRandom >> 32) % UInt64(from: self.lanes));
            if pass == 0 and slice == 0 { referenceLane = lane; }
            let referenceColumn = self.indexAlpha(pass, slice, i, pseudoRandom & 0xffffffff, referenceLane == lane);

            let mixed = Argon2State.compress(self.block(lane, previous), self.block(referenceLane, referenceColumn));
            if pass == 0 {
                self.store(mixed, lane, column);
            } else {
                // Version 0x13 XORs into the existing block on later passes.
                var existing = self.block(lane, column);
                for k in 0..<128 { existing(k) = existing(k) ^ mixed(k); }
                self.store(existing, lane, column);
            }
        }
    }

    /// Maps the low 32 pseudo-random bits onto the blocks this one may
    /// reference, biased toward recent ones (RFC 9106 §3.4.1.2).
    func indexAlpha(pass: Int64, slice: Int64, index: Int64, j1: UInt64, sameLane: Bool) -> Int64 {
        // How many blocks are available to reference.
        var area: Int64 = 0;
        if pass == 0 {
            if sameLane {
                area = slice * self.segment + index - 1;
            } else {
                area = slice * self.segment + (if index == 0 { -1 } else { 0 });
            }
        } else {
            if sameLane {
                area = self.columns - self.segment + index - 1;
            } else {
                area = self.columns - self.segment + (if index == 0 { -1 } else { 0 });
            }
        }

        let x = (j1 * j1) >> 32;
        let y = (UInt64(from: area) * x) >> 32;
        let relative = area - 1 - Int64(from: y);

        // Where that window starts: the segment after this one, from the
        // second pass on.
        let windowStart: Int64 = if pass == 0 or slice == 3 { 0 } else { (slice + 1) * self.segment };
        (windowStart + relative) % self.columns
    }

    /// Advances the address counter and regenerates 128 reference
    /// addresses: G(0, G(0, input)).
    static func nextAddresses(mutating input: Array[UInt64], mutating addresses: Array[UInt64]) {
        input(6) = input(6) + 1;
        let zero = Array[UInt64](repeating: 0, count: 128);
        addresses = Argon2State.compress(zero, Argon2State.compress(zero, input));
    }

    /// The compression function G (RFC 9106 §3.5): R = X ⊕ Y, run the
    /// BLAKE2b-based permutation over R's rows then columns, ⊕ R again.
    static func compress(x: Array[UInt64], y: Array[UInt64]) -> Array[UInt64] {
        var r = Array[UInt64](repeating: 0, count: 128);
        for k in 0..<128 { r(k) = x(k) ^ y(k); }
        var z = r;

        // Rows: 8 runs over 16 consecutive words.
        for row in 0..<8 {
            let b = 16 * row;
            Argon2State.permute(z, b, b + 1, b + 2, b + 3, b + 4, b + 5, b + 6, b + 7,
                                b + 8, b + 9, b + 10, b + 11, b + 12, b + 13, b + 14, b + 15);
        }
        // Columns: word pairs (2c, 2c+1) taken from every row.
        for column in 0..<8 {
            let b = 2 * column;
            Argon2State.permute(z, b, b + 1, b + 16, b + 17, b + 32, b + 33, b + 48, b + 49,
                                b + 64, b + 65, b + 80, b + 81, b + 96, b + 97, b + 112, b + 113);
        }

        for k in 0..<128 { z(k) = z(k) ^ r(k); }
        z
    }

    /// BLAKE2b's round over 16 words, with GB in place of G.
    static func permute(
        mutating v: Array[UInt64],
        i0: Int64, i1: Int64, i2: Int64, i3: Int64, i4: Int64, i5: Int64, i6: Int64, i7: Int64,
        i8: Int64, i9: Int64, i10: Int64, i11: Int64, i12: Int64, i13: Int64, i14: Int64, i15: Int64
    ) {
        Argon2State.gb(v, i0, i4, i8, i12);
        Argon2State.gb(v, i1, i5, i9, i13);
        Argon2State.gb(v, i2, i6, i10, i14);
        Argon2State.gb(v, i3, i7, i11, i15);
        Argon2State.gb(v, i0, i5, i10, i15);
        Argon2State.gb(v, i1, i6, i11, i12);
        Argon2State.gb(v, i2, i7, i8, i13);
        Argon2State.gb(v, i3, i4, i9, i14);
    }

    /// BLAKE2b's G with an added multiplication of the low 32-bit halves
    /// (RFC 9106 §3.6), which makes the work hard to shortcut in hardware.
    static func gb(mutating v: Array[UInt64], a: Int64, b: Int64, c: Int64, d: Int64) {
        v(a) = v(a) + v(b) + 2 * (v(a) & 0xffffffff) * (v(b) & 0xffffffff);
        v(d) = (v(d) ^ v(a)).rotateRight(by: 32);
        v(c) = v(c) + v(d) + 2 * (v(c) & 0xffffffff) * (v(d) & 0xffffffff);
        v(b) = (v(b) ^ v(c)).rotateRight(by: 24);
        v(a) = v(a) + v(b) + 2 * (v(a) & 0xffffffff) * (v(b) & 0xffffffff);
        v(d) = (v(d) ^ v(a)).rotateRight(by: 16);
        v(c) = v(c) + v(d) + 2 * (v(c) & 0xffffffff) * (v(d) & 0xffffffff);
        v(b) = (v(b) ^ v(c)).rotateRight(by: 63);
    }
}
