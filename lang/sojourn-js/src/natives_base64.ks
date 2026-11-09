module js

// Uint8Array base64 and hex: Uint8Array.fromBase64 / fromHex and
// Uint8Array.prototype.toBase64 / toHex / setFromBase64 / setFromHex.

/// What a decode got through: how much input it consumed, the bytes, and
/// the SyntaxError message it stopped at ("" when none).
struct DecodeResult {
    var read: Int64
    var bytes: [UInt8]
    var error: String
}

/// `lastChunkHandling`.
struct LastChunk {
    static var loose: Int64 { 0 }
    static var strict: Int64 { 1 }
    static var stopBeforePartial: Int64 { 2 }
}

func base64Alphabet(url: Bool) -> Units {
    unitsOf(if url { "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_" } else { "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/" })
}

/// The 6-bit value of a standard base64 character, or -1.
func base64Value(c: Int64) -> Int64 {
    if c >= 65 and c <= 90 { return c - 65; }
    if c >= 97 and c <= 122 { return c - 71; }
    if c >= 48 and c <= 57 { return c + 4; }
    if c == 43 { return 62; }
    if c == 47 { return 63; }
    return -1;
}

func isAsciiWhitespace(c: Int64) -> Bool { c == 9 or c == 10 or c == 12 or c == 13 or c == 32 }

func skipAsciiWhitespace(s: Units, from: Int64) -> Int64 {
    var i = from;
    while i < s.count and isAsciiWhitespace(Int64(from: s(i))) { i = i + 1; }
    i
}

/// DecodeBase64Chunk: 2 to 4 characters' values into 1 to 3 bytes. With
/// `strictBits`, leftover bits of a short chunk must be zero.
func decodeBase64Chunk(chunk: Array[Int64], strictBits: Bool) -> Optional[Array[UInt8]] {
    var n: Int64 = 0;
    for i in 0..<4 { n = (n << 6) | (if i < chunk.count { chunk(i) } else { 0 }); }
    let b = [UInt8(truncating: n >> 16), UInt8(truncating: n >> 8), UInt8(truncating: n)];
    if chunk.count == 2 {
        if strictBits and b(1) != 0 { return .None; }
        return .Some([b(0)]);
    }
    if chunk.count == 3 {
        if strictBits and b(2) != 0 { return .None; }
        return .Some([b(0), b(1)]);
    }
    .Some(b)
}

/// FromBase64, stopping after `maxLength` bytes.
func fromBase64(s: Units, url: Bool, lastChunk: Int64, maxLength: Int64) -> DecodeResult {
    var res = DecodeResult(read: 0, bytes: [], error: "");
    if maxLength == 0 { return res; }
    var chunk: [Int64] = [];
    var index: Int64 = 0;
    let length = s.count;
    while true {
        index = skipAsciiWhitespace(s, index);
        if index == length {
            if chunk.count > 0 {
                if lastChunk == LastChunk.stopBeforePartial { return res; }
                if lastChunk == LastChunk.strict or chunk.count == 1 {
                    res.error = "Incomplete base64 chunk";
                    return res;
                }
                for b in decodeBase64Chunk(chunk, false) ?? [] { res.bytes.append(b); }
            }
            res.read = length;
            return res;
        }
        var c = Int64(from: s(index));
        index = index + 1;
        if c == 61 {
            if chunk.count < 2 {
                res.error = "Unexpected base64 padding";
                return res;
            }
            index = skipAsciiWhitespace(s, index);
            if chunk.count == 2 {
                if index == length {
                    if lastChunk != LastChunk.stopBeforePartial { res.error = "Incomplete base64 padding"; }
                    return res;
                }
                if Int64(from: s(index)) == 61 { index = skipAsciiWhitespace(s, index + 1); }
            }
            if index < length {
                res.error = "Unexpected characters after base64 padding";
                return res;
            }
            match decodeBase64Chunk(chunk, lastChunk == LastChunk.strict) {
                .Some(bs) => { for b in bs { res.bytes.append(b); } },
                .None => {
                    res.error = "Nonzero bits in base64 padding";
                    return res;
                }
            }
            res.read = length;
            return res;
        }
        if url {
            if c == 43 or c == 47 { c = -1; } else if c == 45 { c = 43; } else if c == 95 { c = 47; }
        }
        let v = if c < 0 { -1 } else { base64Value(c) };
        if v < 0 {
            res.error = "Invalid base64 character";
            return res;
        }
        let remaining = maxLength - res.bytes.count;
        if (remaining == 1 and chunk.count == 2) or (remaining == 2 and chunk.count == 3) { return res; }
        chunk.append(v);
        if chunk.count == 4 {
            for b in decodeBase64Chunk(chunk, false) ?? [] { res.bytes.append(b); }
            chunk = [];
            res.read = index;
            if res.bytes.count == maxLength { return res; }
        }
    }
    res
}

/// FromHex, stopping after `maxLength` bytes.
func fromHex(s: Units, maxLength: Int64) -> DecodeResult {
    var res = DecodeResult(read: 0, bytes: [], error: "");
    if s.count % 2 != 0 {
        res.error = "Hex string has an odd length";
        return res;
    }
    while res.read < s.count and res.bytes.count < maxLength {
        let hi = hexValue(Int64(from: s(res.read)));
        let lo = hexValue(Int64(from: s(res.read + 1)));
        if hi < 0 or lo < 0 {
            res.error = "Invalid hex character";
            return res;
        }
        res.read = res.read + 2;
        res.bytes.append(UInt8(truncating: hi * 16 + lo));
    }
    res
}

extend VM {
    mutating func setupBase64() {
        let c = self.typedArrayCtors(TAKind.uint8);
        self.method(c, "fromBase64", .Uint8FromBase64, 1);
        self.method(c, "fromHex", .Uint8FromHex, 1);
        let proto = self.typedArrayProtos(TAKind.uint8);
        self.method(proto, "toBase64", .Uint8ToBase64, 0);
        self.method(proto, "toHex", .Uint8ToHex, 0);
        self.method(proto, "setFromBase64", .Uint8SetFromBase64, 1);
        self.method(proto, "setFromHex", .Uint8SetFromHex, 1);
    }

    mutating func base64Native(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .Uint8FromBase64 => {
                if not a0.isString { return self.throwType("Uint8Array.fromBase64 requires a string"); }
                let (url, last) = try self.base64Options(a1, true);
                let res = fromBase64(self.units(a0), url, last, 9007199254740991);
                if not res.error.isEmpty { return .Err(self.makeError(.SyntaxError, res.error)); }
                try self.uint8ArrayOf(res.bytes)
            },
            .Uint8FromHex => {
                if not a0.isString { return self.throwType("Uint8Array.fromHex requires a string"); }
                let res = fromHex(self.units(a0), 9007199254740991);
                if not res.error.isEmpty { return .Err(self.makeError(.SyntaxError, res.error)); }
                try self.uint8ArrayOf(res.bytes)
            },
            .Uint8ToBase64 => {
                let d = try self.validateUint8(this);
                let (url, _) = try self.base64Options(a0, false);
                let opts = try self.optionsObject(a0);
                let omit = self.toBoolean(try self.getV(opts, self.atom("omitPadding")));
                let bytes = try self.uint8Bytes(d);
                self.newString(base64Encode(bytes, url, omit))
            },
            .Uint8ToHex => {
                let d = try self.validateUint8(this);
                let bytes = try self.uint8Bytes(d);
                let hex = unitsOf("0123456789abcdef");
                var out: Units = [];
                for b in bytes {
                    let v = Int64(from: b);
                    out.append(hex(v >> 4));
                    out.append(hex(v & 15));
                }
                self.newString(out)
            },
            .Uint8SetFromBase64 or .Uint8SetFromHex => {
                let d = try self.validateUint8(this);
                if not a0.isString { return self.throwType("Uint8Array.prototype.setFrom requires a string"); }
                var url = false;
                var last = LastChunk.loose;
                if n == NativeFn.Uint8SetFromBase64 {
                    let (u, l) = try self.base64Options(a1, true);
                    url = u;
                    last = l;
                }
                if self.isDetached(d.buffer) { return self.throwType("Cannot write into a detached ArrayBuffer"); }
                let s = self.units(a0);
                let res = if n == NativeFn.Uint8SetFromBase64 { fromBase64(s, url, last, d.length) } else { fromHex(s, d.length) };
                let id = self.bufferId(d.buffer);
                for i in 0..<res.bytes.count { self.heap.buffers(id).bytes(d.offset + i) = res.bytes(i); }
                if not res.error.isEmpty { return .Err(self.makeError(.SyntaxError, res.error)); }
                let o = self.newObject();
                self.defineOwn(o.asRef, self.atom("read"), Value.int(res.read), Attr.all);
                self.defineOwn(o.asRef, self.atom("written"), Value.int(res.bytes.count), Attr.all);
                o
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// GetOptionsObject: undefined as an empty object, else an object.
    mutating func optionsObject(v: Value) -> Result[Value, Value] {
        if v.isUndefined { return .Ok(self.newObjectKind(Value.nullValue, .Ordinary)); }
        if not v.isObject { return self.throwType("options must be an object"); }
        .Ok(v)
    }

    /// The `alphabet` option (whether it's base64url), and when
    /// `withLastChunk` the `lastChunkHandling` one.
    mutating func base64Options(options: Value, withLastChunk: Bool) -> Result[(Bool, Int64), Value] {
        let opts = try self.optionsObject(options);
        let a = try self.getV(opts, self.atom("alphabet"));
        var url = false;
        if not a.isUndefined {
            let u = if a.isString { self.units(a) } else { unitsOf("") };
            url = unitsEqual(u, unitsOf("base64url"));
            if not url and not unitsEqual(u, unitsOf("base64")) { return self.throwType("alphabet must be \"base64\" or \"base64url\""); }
        }
        if not withLastChunk { return .Ok((url, LastChunk.loose)); }
        let l = try self.getV(opts, self.atom("lastChunkHandling"));
        var last = LastChunk.loose;
        if not l.isUndefined {
            let u = if l.isString { self.units(l) } else { unitsOf("") };
            if unitsEqual(u, unitsOf("strict")) {
                last = LastChunk.strict;
            } else if unitsEqual(u, unitsOf("stop-before-partial")) {
                last = LastChunk.stopBeforePartial;
            } else if not unitsEqual(u, unitsOf("loose")) {
                return self.throwType("lastChunkHandling must be \"loose\", \"strict\" or \"stop-before-partial\"");
            }
        }
        .Ok((url, last))
    }

    /// ValidateUint8Array.
    mutating func validateUint8(v: Value) -> Result[TypedArrayData, Value] {
        if let .Some(d) = self.taData(v) {
            if d.kind == TAKind.uint8 { return .Ok(d); }
        }
        self.throwType("\(self.describe(v)) is not a Uint8Array")
    }

    /// GetUint8ArrayBytes: the elements, or a TypeError once detached.
    mutating func uint8Bytes(d: TypedArrayData) -> Result[Array[UInt8], Value] {
        if self.isDetached(d.buffer) { return self.throwType("Cannot read from a detached ArrayBuffer"); }
        let id = self.bufferId(d.buffer);
        var out: [UInt8] = [];
        for i in 0..<d.length { out.append(self.heap.buffers(id).bytes(d.offset + i)); }
        .Ok(out)
    }

    mutating func uint8ArrayOf(bytes: Array[UInt8]) -> Result[Value, Value] {
        let proto = self.typedArrayProtos(TAKind.uint8);
        let ta = try self.newTypedArray(TAKind.uint8, bytes.count, proto);
        if let .Some(d) = self.taData(ta) {
            let id = self.bufferId(d.buffer);
            for i in 0..<bytes.count { self.heap.buffers(id).bytes(i) = bytes(i); }
        }
        .Ok(ta)
    }
}

func base64Encode(bytes: Array[UInt8], url: Bool, omitPadding: Bool) -> Units {
    let alpha = base64Alphabet(url);
    var out: Units = [];
    var i: Int64 = 0;
    while i < bytes.count {
        let left = bytes.count - i;
        let b0 = Int64(from: bytes(i));
        let b1 = if left > 1 { Int64(from: bytes(i + 1)) } else { 0 };
        let b2 = if left > 2 { Int64(from: bytes(i + 2)) } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.append(alpha((n >> 18) & 63));
        out.append(alpha((n >> 12) & 63));
        if left > 1 { out.append(alpha((n >> 6) & 63)); } else if not omitPadding { out.append(61); }
        if left > 2 { out.append(alpha(n & 63)); } else if not omitPadding { out.append(61); }
        i = i + 3;
    }
    out
}
