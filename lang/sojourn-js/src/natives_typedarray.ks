module js

// ArrayBuffer, the typed arrays and DataView (ECMA-262 §25.1, §23.2,
// §25.3).
//
// A buffer's bytes live in `Heap.buffers`. A typed array is an
// integer-indexed exotic object: numeric keys never reach its shape or
// prototype, but read and write its buffer (`taKeyIndex` is the hook the
// property protocol in object.ks calls). Values are stored little-endian.
// BigInt64Array and BigUint64Array hold BigInts; the rest numbers.

/// Element types, `TypedArrayData.kind`.
struct TAKind {
    static var int8: Int64 { 0 }
    static var uint8: Int64 { 1 }
    static var uint8Clamped: Int64 { 2 }
    static var int16: Int64 { 3 }
    static var uint16: Int64 { 4 }
    static var int32: Int64 { 5 }
    static var uint32: Int64 { 6 }
    static var float32: Int64 { 7 }
    static var float64: Int64 { 8 }
    static var bigInt64: Int64 { 9 }
    static var bigUint64: Int64 { 10 }
    static var float16: Int64 { 11 }
    static var count: Int64 { 12 }
}

func taElementSize(kind: Int64) -> Int64 { [1, 1, 1, 2, 2, 4, 4, 4, 8, 8, 8, 2](kind) }

/// The element type is BigInt (content type BigInt, in the spec's terms).
func taIsBig(kind: Int64) -> Bool { kind == TAKind.bigInt64 or kind == TAKind.bigUint64 }

func taName(kind: Int64) -> String {
    ["Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array",
     "Int32Array", "Uint32Array", "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array", "Float16Array"](kind)
}

/// The largest buffer this engine allocates.
func maxBufferBytes() -> Int64 { 268435456 }

/// ToInt8 and friends: `x` modulo 2^bits, as a signed value when `signed`.
func toIntBits(x: Float64, bits: Int64, signed: Bool) -> Int64 {
    if x.isNaN or x.isInfinite { return 0; }
    let m = Float64(from: oneBit() << bits);
    var r = fmod(x.trunc(), m);
    if r < 0.0 { r = r + m; }
    var i = r.toInt64() ?? 0;
    if signed and i >= (oneBit() << (bits - 1)) { i = i - (oneBit() << bits); }
    i
}

/// ToUint8Clamp: rounds half to even and clamps to 0...255.
func toUint8Clamp(x: Float64) -> Int64 {
    if x.isNaN or x <= 0.0 { return 0; }
    if x >= 255.0 { return 255; }
    let f = x.floor();
    let fi = f.toInt64() ?? 0;
    if x - f > 0.5 { return fi + 1; }
    if x - f < 0.5 { return fi; }
    if fi % 2 == 0 { fi } else { fi + 1 }
}

/// The binary16 bits of `x`, rounded once (ties to even).
func float64ToFloat16(x: Float64) -> UInt64 {
    let bits = x.bitPattern;
    let sign = (bits >> 63) << 15;
    if x.isNaN { return 0x7E00; }
    let a = x.abs();
    if a >= 65520.0 { return sign | 0x7C00; }
    if a < pow2(0 - 14) {
        // Subnormal: a whole number of 2^-24 (1024 carries into the
        // smallest normal, whose bits are the same).
        let q = a * pow2(24);
        let f = q.floor();
        var m = f.toInt64() ?? 0;
        let d = q - f;
        if d > 0.5 or (d == 0.5 and m % 2 == 1) { m = m + 1; }
        return sign | UInt64(bitPattern: m);
    }
    var e = Int64(bitPattern: (bits >> 52) & 0x7FF) - 1023;
    let mant = bits & 0xFFFFFFFFFFFFF;
    var m = mant >> 42;
    let rest = mant & 0x3FFFFFFFFFF;
    let half: UInt64 = 0x20000000000;
    if rest > half or (rest == half and m & 1 == 1) { m = m + 1; }
    if m == 1024 {
        m = 0;
        e = e + 1;
    }
    if e > 15 { return sign | 0x7C00; }
    sign | (UInt64(bitPattern: e + 15) << 10) | m
}

func float16ToFloat64(h: UInt64) -> Float64 {
    let e = Int64(bitPattern: (h >> 10) & 0x1F);
    let m = Float64(from: Int64(bitPattern: h & 0x3FF));
    let v = if e == 0 { m * pow2(0 - 24) } else if e == 31 {
        if m == 0.0 { Float64.infinity } else { Float64.nan }
    } else { (1024.0 + m) * pow2(e - 25) };
    if (h >> 15) & 1 == 1 { v * (0.0 - 1.0) } else { v }
}

extend VM {
    mutating func setupTypedArrays() {
        let g = self.global;
        let functionCtor = self.ownValue(g.asRef, self.atom("Function"));

        // ArrayBuffer
        let abProto = self.newObject();
        let ab = self.constructorFor("ArrayBuffer", .ArrayBufferCtor, 1, abProto);
        self.intrinsics.arrayBufferProto = abProto;
        self.method(ab, "isView", .ArrayBufferIsView, 1);
        self.speciesGetter(ab);
        self.getter(abProto, "byteLength", self.atom("byteLength"), .ArrayBufferByteLength);
        self.method(abProto, "slice", .ArrayBufferSlice, 2);
        let abTag = self.str("ArrayBuffer");
        self.defineOwn(abProto.asRef, symbolKey(WellKnown.toStringTag), abTag, Attr.configurable);

        // %TypedArray% and its prototype
        let taProto = self.newObject();
        let ta = self.nativeFunction(.TypedArrayCtor, "TypedArray", 0, true);
        self.heap.objects(ta.asRef).proto = functionCtor;
        self.defineOwn(ta.asRef, self.names.prototype, taProto, 0);
        self.defineHidden(taProto.asRef, self.names.constructor, ta);
        self.intrinsics.typedArrayProto = taProto;
        self.intrinsics.typedArrayCtor = ta;
        self.speciesGetter(ta);
        self.method(ta, "from", .TypedArrayFrom, 1);
        self.method(ta, "of", .TypedArrayOf, 0);
        for (name, n) in [("buffer", NativeFn.TABuffer), ("byteLength", NativeFn.TAByteLength),
                          ("byteOffset", NativeFn.TAByteOffset), ("length", NativeFn.TALength)] {
            self.getter(taProto, name, self.atom(name), n);
        }
        let tagGetter = self.nativeFunction(.TAToStringTag, "get [Symbol.toStringTag]", 0, false);
        self.defineAccessorOwn(taProto.asRef, symbolKey(WellKnown.toStringTag), tagGetter, Value.undefined, Attr.configurable);
        for (name, n, len) in [
            ("at", NativeFn.TAAt, 1), ("copyWithin", NativeFn.TACopyWithin, 2), ("entries", NativeFn.TAEntries, 0),
            ("every", NativeFn.TAEvery, 1), ("fill", NativeFn.TAFill, 1), ("filter", NativeFn.TAFilter, 1),
            ("find", NativeFn.TAFind, 1), ("findIndex", NativeFn.TAFindIndex, 1), ("findLast", NativeFn.TAFindLast, 1),
            ("findLastIndex", NativeFn.TAFindLastIndex, 1), ("forEach", NativeFn.TAForEach, 1),
            ("includes", NativeFn.TAIncludes, 1), ("indexOf", NativeFn.TAIndexOf, 1), ("join", NativeFn.TAJoin, 1),
            ("keys", NativeFn.TAKeys, 0), ("lastIndexOf", NativeFn.TALastIndexOf, 1), ("map", NativeFn.TAMap, 1),
            ("reduce", NativeFn.TAReduce, 1), ("reduceRight", NativeFn.TAReduceRight, 1),
            ("reverse", NativeFn.TAReverse, 0), ("set", NativeFn.TASet, 1), ("slice", NativeFn.TASlice, 2),
            ("some", NativeFn.TASome, 1), ("sort", NativeFn.TASort, 1), ("subarray", NativeFn.TASubarray, 2),
            ("toLocaleString", NativeFn.TAToLocaleString, 0), ("toReversed", NativeFn.TAToReversed, 0),
            ("toSorted", NativeFn.TAToSorted, 1), ("values", NativeFn.TAValues, 0), ("with", NativeFn.TAWith, 2)
        ] {
            self.method(taProto, name, n, len);
        }
        // toString is Array.prototype.toString itself; @@iterator is values.
        let arrayToString = self.ownValue(self.intrinsics.arrayProto.asRef, self.names.toString);
        self.defineHidden(taProto.asRef, self.names.toString, arrayToString);
        let values = self.ownValue(taProto.asRef, self.atom("values"));
        self.defineHidden(taProto.asRef, symbolKey(WellKnown.iterator), values);

        // The concrete constructors
        for kind in 0..<TAKind.count {
            let proto = self.newObjectKind(taProto, .Ordinary);
            let c = self.constructorFor(taName(kind), .TypedArrayKindCtor, 3, proto);
            self.heap.objects(c.asRef).kind = .NativeClosure(fn: .TypedArrayKindCtor, captures: [Value.int(kind)]);
            self.heap.objects(c.asRef).proto = ta;
            let size = Value.int(taElementSize(kind));
            self.defineOwn(c.asRef, self.atom("BYTES_PER_ELEMENT"), size, 0);
            self.defineOwn(proto.asRef, self.atom("BYTES_PER_ELEMENT"), size, 0);
            self.typedArrayProtos.append(proto);
            self.typedArrayCtors.append(c);
        }

        // DataView
        let dvProto = self.newObject();
        let _ = self.constructorFor("DataView", .DataViewCtor, 1, dvProto);
        self.intrinsics.dataViewProto = dvProto;
        for (name, n) in [("buffer", NativeFn.DVBuffer), ("byteLength", NativeFn.DVByteLength), ("byteOffset", NativeFn.DVByteOffset)] {
            self.getter(dvProto, name, self.atom(name), n);
        }
        let dvTag = self.str("DataView");
        self.defineOwn(dvProto.asRef, symbolKey(WellKnown.toStringTag), dvTag, Attr.configurable);
        let kindNames = ["Int8", "Uint8", "", "Int16", "Uint16", "Int32", "Uint32", "Float32", "Float64", "BigInt64", "BigUint64", "Float16"];
        for kind in 0..<TAKind.count {
            if kind == TAKind.uint8Clamped { continue; }
            let getName = "get" + kindNames(kind);
            let getF = self.nativeFunction(.DVGet, getName, 1, false);
            self.heap.objects(getF.asRef).kind = .NativeClosure(fn: .DVGet, captures: [Value.int(kind)]);
            self.defineHidden(dvProto.asRef, self.atom(getName), getF);
            let setName = "set" + kindNames(kind);
            let setF = self.nativeFunction(.DVSet, setName, 2, false);
            self.heap.objects(setF.asRef).kind = .NativeClosure(fn: .DVSet, captures: [Value.int(kind)]);
            self.defineHidden(dvProto.asRef, self.atom(setName), setF);
        }

        // The test262 host hook for detaching.
        let host = self.ownValue(g.asRef, self.atom("$262"));
        if host.isObject { self.method(host, "detachArrayBuffer", .DetachArrayBuffer, 1); }
    }

    mutating func speciesGetter(c: Value) {
        let species = self.nativeFunction(.ReturnThis, "get [Symbol.species]", 0, false);
        self.defineAccessorOwn(c.asRef, symbolKey(WellKnown.species), species, Value.undefined, Attr.configurable);
    }

    // ------------------------------------------------------------------
    // Buffers
    // ------------------------------------------------------------------

    func bufferId(v: Value) -> Int64 {
        if not v.isObject { return -1; }
        match self.kindAt(v.asRef) {
            .ArrayBufferObject(id) => id,
            _ => -1
        }
    }

    func isDetached(buffer: Value) -> Bool {
        let id = self.bufferId(buffer);
        id < 0 or self.heap.buffers(id).detached
    }

    func bufferByteLength(buffer: Value) -> Int64 {
        let id = self.bufferId(buffer);
        if id < 0 { return 0; }
        self.heap.buffers(id).bytes.count
    }

    /// AllocateArrayBuffer: `n` zero bytes, with prototype `proto`.
    mutating func allocateArrayBuffer(proto: Value, n: Int64) -> Result[Value, Value] {
        if n > maxBufferBytes() { return .Err(self.makeError(.RangeError, "Array buffer allocation failed")); }
        var bytes: [UInt8] = [];
        for _ in 0..<n { bytes.append(0); }
        let id = self.heap.allocBuffer(BufferData(bytes: bytes, detached: false));
        .Ok(self.newObjectKind(proto, .ArrayBufferObject(id)))
    }

    /// ToIndex: a non-negative integer below 2^53.
    mutating func toIndex(v: Value) -> Result[Int64, Value] {
        if v.isUndefined { return .Ok(0); }
        let i = try self.toIntegerOrInfinity(v);
        if i < 0.0 or i > 9007199254740991.0 { return .Err(self.makeError(.RangeError, "Invalid index")); }
        .Ok(i.toInt64() ?? 0)
    }

    /// The `size` bytes at `at` of buffer `id`, as an integer.
    func rawBits(id: Int64, at: Int64, size: Int64, little: Bool) -> UInt64 {
        var bits: UInt64 = 0;
        for i in 0..<size {
            let b = UInt64(from: self.heap.buffers(id).bytes(at + i));
            let shift = if little { 8 * i } else { 8 * (size - 1 - i) };
            bits = bits | (b << shift);
        }
        bits
    }

    /// Reads one `kind` value at byte `at` of buffer `id`.
    mutating func rawGet(id: Int64, at: Int64, kind: Int64, little: Bool) -> Value {
        if taIsBig(kind) {
            let bits = self.rawBits(id, at, 8, little);
            let b = if kind == TAKind.bigInt64 { bnFromInt(Int64(bitPattern: bits)) } else { bnFromU64(bits, false) };
            return self.newBigInt(b);
        }
        self.rawNumber(id, at, kind, little)
    }

    /// Reads one number-kind value: no allocation, so property lookups can.
    func rawNumber(id: Int64, at: Int64, kind: Int64, little: Bool) -> Value {
        if taIsBig(kind) { return Value.undefined; }
        let size = taElementSize(kind);
        let bits = self.rawBits(id, at, size, little);
        if kind == TAKind.float32 { return Value.number(Float64(from: Float32(bitPattern: UInt32(truncating: bits)))); }
        if kind == TAKind.float64 { return Value.number(Float64(bitPattern: bits)); }
        if kind == TAKind.float16 { return Value.number(float16ToFloat64(bits)); }
        var n = Int64(bitPattern: bits);
        let signed = kind == TAKind.int8 or kind == TAKind.int16 or kind == TAKind.int32;
        let width = 8 * size;
        if signed and n >= (oneBit() << (width - 1)) { n = n - (oneBit() << width); }
        Value.int(n)
    }

    /// Writes `v` (a number, or a BigInt for the BigInt kinds) as one
    /// `kind` value at byte `at` of buffer `id`.
    mutating func rawSet(id: Int64, at: Int64, kind: Int64, v: Value, little: Bool) {
        let size = taElementSize(kind);
        let x = if v.isNumber { v.asNumber } else { 0.0 };
        let bits: UInt64 = if taIsBig(kind) {
            bigLow64(self.bigOf(v))
        } else if kind == TAKind.float32 {
            UInt64(from: x.toFloat32().bitPattern)
        } else if kind == TAKind.float64 {
            x.bitPattern
        } else if kind == TAKind.float16 {
            float64ToFloat16(x)
        } else if kind == TAKind.uint8Clamped {
            UInt64(bitPattern: toUint8Clamp(x))
        } else {
            UInt64(bitPattern: toIntBits(x, 8 * size, false))
        };
        for i in 0..<size {
            let shift = if little { 8 * i } else { 8 * (size - 1 - i) };
            self.heap.buffers(id).bytes(at + i) = UInt8(truncating: bits >> shift);
        }
    }

    // ------------------------------------------------------------------
    // Typed arrays
    // ------------------------------------------------------------------

    func taData(v: Value) -> Optional[TypedArrayData] {
        if not v.isObject { return .None; }
        match self.kindAt(v.asRef) {
            .TypedArrayObject(d) => .Some(d),
            _ => .None
        }
    }

    /// The element count, 0 once the buffer is detached.
    func taLength(d: TypedArrayData) -> Int64 {
        if self.isDetached(d.buffer) { return 0; }
        d.length
    }

    mutating func taGet(d: TypedArrayData, i: Int64) -> Value {
        let id = self.bufferId(d.buffer);
        self.rawGet(id, d.offset + i * taElementSize(d.kind), d.kind, true)
    }

    /// An element without allocating: undefined for the BigInt kinds,
    /// whose values `ownPropertyValue` fills in.
    func taGetPure(d: TypedArrayData, i: Int64) -> Value {
        let id = self.bufferId(d.buffer);
        self.rawNumber(id, d.offset + i * taElementSize(d.kind), d.kind, true)
    }

    /// Writes element `i`; `v` is already the kind's type (`taConvert`).
    mutating func taPut(d: TypedArrayData, i: Int64, v: Value) {
        let id = self.bufferId(d.buffer);
        self.rawSet(id, d.offset + i * taElementSize(d.kind), d.kind, v, true);
    }

    /// ToBigInt or ToNumber, by the element type.
    mutating func taConvert(kind: Int64, v: Value) -> Result[Value, Value] {
        if taIsBig(kind) { return .Ok(self.newBigInt(try self.toBigInt(v))); }
        .Ok(Value.number(try self.toNumber(v)))
    }

    /// IsValidIntegerIndex.
    func taValidIndex(d: TypedArrayData, n: Float64) -> Bool {
        if self.isDetached(d.buffer) { return false; }
        if n != n.trunc() or (n == 0.0 and n.isSignMinus) { return false; }
        n >= 0.0 and n < Float64(from: d.length)
    }

    /// CanonicalNumericIndexString for a key: its number, if it's the
    /// canonical form of one ("-0" included).
    func canonicalNumeric(k: Key) -> Optional[Float64] {
        if keyIsSymbol(k) { return .None; }
        let i = self.indexOfKey(k);
        if i >= 0 { return .Some(Float64(from: i)); }
        let s = self.atomNames(k);
        if s.isEmpty { return .None; }
        let c = s.bytes(0);
        // Digits, '-', 'I'nfinity, 'N'aN: anything else can't be numeric.
        if not ((c >= 48 and c <= 57) or c == 45 or c == 73 or c == 78) { return .None; }
        if s == "-0" { return .Some(Float64(bitPattern: 0x8000000000000000)); }
        let n = stringToNumber(unitsOf(s));
        if numberToString(n) == s { return .Some(n); }
        .None
    }

    /// The numeric index `key` names on typed array `r`, or None when `r`
    /// isn't one or the key is an ordinary property name.
    func taKeyIndex(r: Ref, key: Key) -> Optional[(TypedArrayData, Float64)] {
        let d = match self.kindAt(r) {
            .TypedArrayObject(x) => x,
            _ => return .None
        };
        match self.canonicalNumeric(key) {
            .Some(n) => .Some((d, n)),
            .None => .None
        }
    }

    /// TypedArraySetElement: converts first, then writes if the index is
    /// (still) valid.
    mutating func taSetElement(d: TypedArrayData, n: Float64, v: Value) -> Result[(), Value] {
        let x = try self.taConvert(d.kind, v);
        if self.taValidIndex(d, n) { self.taPut(d, n.toInt64() ?? 0, x); }
        .Ok(())
    }

    /// ValidateTypedArray: `v` must be a typed array on a live buffer.
    mutating func validateTA(v: Value, name: String) -> Result[TypedArrayData, Value] {
        let d = match self.taData(v) {
            .Some(x) => x,
            .None => return self.throwType("%TypedArray%.prototype.\(name) called on incompatible receiver")
        };
        if self.isDetached(d.buffer) { return self.throwType("Cannot perform %TypedArray%.prototype.\(name) on a detached ArrayBuffer"); }
        .Ok(d)
    }

    /// A new typed array of `kind` over a fresh buffer of `length`.
    mutating func newTypedArray(kind: Int64, length: Int64, proto: Value) -> Result[Value, Value] {
        let size = taElementSize(kind);
        if length > maxBufferBytes() / size { return .Err(self.makeError(.RangeError, "Invalid typed array length: \(length)")); }
        let buffer = try self.allocateArrayBuffer(self.intrinsics.arrayBufferProto, length * size);
        self.pushRoot(buffer);
        .Ok(self.newObjectKind(proto, .TypedArrayObject(TypedArrayData(kind: kind, buffer: buffer, offset: 0, length: length))))
    }

    /// TypedArrayCreateFromConstructor: `new c(...args)`, checked to be a
    /// live typed array at least `minLength` long (when given).
    mutating func taCreateFrom(c: Value, args: Array[Value], minLength: Int64) -> Result[Value, Value] {
        let o = try self.construct(c, args, c);
        let d = try self.validateTA(o, "constructor");
        if minLength >= 0 and d.length < minLength { return self.throwType("Derived TypedArray constructor created an array which was too small"); }
        .Ok(o)
    }

    /// TypedArraySpeciesCreate.
    mutating func taSpeciesCreate(exemplar: Value, d: TypedArrayData, args: Array[Value], minLength: Int64) -> Result[Value, Value] {
        let fallback = self.typedArrayCtors(d.kind);
        let c = try self.speciesConstructor(exemplar, fallback);
        self.taCreateFrom(c, args, minLength)
    }

    /// The typed array constructor for `kind`: every argument form.
    mutating func constructTypedArray(kind: Int64, args: Array[Value], newTarget: Value) -> Result[Value, Value] {
        if newTarget.isUndefined { return self.throwType("Constructor \(taName(kind)) requires 'new'"); }
        let fallback = self.typedArrayProtos(kind);
        let a0 = arg(args, 0);
        // A length argument is converted before the prototype is read.
        if not a0.isObject {
            let n = try self.toIndex(a0);
            let p = try self.prototypeFor(newTarget, fallback);
            return self.newTypedArray(kind, n, p);
        }
        let proto = try self.prototypeFor(newTarget, fallback);
        self.pushRoot(proto);
        let size = taElementSize(kind);
        if let .Some(src) = self.taData(a0) {
            if self.isDetached(src.buffer) { return self.throwType("Cannot construct from a detached ArrayBuffer"); }
            let o = try self.newTypedArray(kind, src.length, proto);
            let d = self.taData(o) ?? src;
            if taIsBig(kind) != taIsBig(src.kind) { return self.throwType("Content type of the source doesn't match \(taName(kind))"); }
            for i in 0..<src.length {
                let v = self.taGet(src, i);
                self.taPut(d, i, v);
            }
            return .Ok(o);
        }
        if self.bufferId(a0) >= 0 {
            let offset = try self.toIndex(arg(args, 1));
            if offset % size != 0 { return .Err(self.makeError(.RangeError, "start offset of \(taName(kind)) should be a multiple of \(size)")); }
            let a2 = arg(args, 2);
            let newLength = if a2.isUndefined { 0 } else { try self.toIndex(a2) };
            if self.isDetached(a0) { return self.throwType("Cannot construct on a detached ArrayBuffer"); }
            let bufLen = self.bufferByteLength(a0);
            var byteLength: Int64 = 0;
            if a2.isUndefined {
                if bufLen % size != 0 { return .Err(self.makeError(.RangeError, "byte length of \(taName(kind)) should be a multiple of \(size)")); }
                byteLength = bufLen - offset;
                if byteLength < 0 { return .Err(self.makeError(.RangeError, "Start offset \(offset) is outside the bounds of the buffer")); }
            } else {
                byteLength = newLength * size;
                if offset + byteLength > bufLen { return .Err(self.makeError(.RangeError, "Invalid typed array length: \(newLength)")); }
            }
            let d = TypedArrayData(kind: kind, buffer: a0, offset: offset, length: byteLength / size);
            return .Ok(self.newObjectKind(proto, .TypedArrayObject(d)));
        }
        // An iterable or array-like object.
        let usingIterator = try self.getMethod(a0, symbolKey(WellKnown.iterator));
        var values: [Value] = [];
        var len: Int64 = 0;
        if not usingIterator.isUndefined {
            values = try self.iterateWith(a0, usingIterator);
            len = values.count;
        } else {
            len = try self.lengthOf(a0);
        }
        let o = try self.newTypedArray(kind, len, proto);
        self.pushRoot(o);
        let d = self.taData(o) ?? TypedArrayData(kind: kind, buffer: Value.undefined, offset: 0, length: 0);
        for i in 0..<len {
            let v = if usingIterator.isUndefined { try self.getIndex(a0, i) } else { values(i) };
            try self.taSetElement(d, Float64(from: i), v);
        }
        .Ok(o)
    }

    /// The values an iterator from `method` produces.
    mutating func iterateWith(obj: Value, method: Value) -> Result[Array[Value], Value] {
        let it = try self.callValue(method, obj, []);
        if not it.isObject { return self.throwType("Result of the Symbol.iterator method is not an object"); }
        self.pushRoot(it);
        let next = try self.getV(it, self.names.next);
        var out: [Value] = [];
        while true {
            let step = try self.iteratorStep(it, next);
            let v = match step { .Some(x) => x, .None => break };
            self.pushRoot(v);
            out.append(v);
        }
        .Ok(out)
    }

    /// The typed array's elements.
    mutating func taValues(d: TypedArrayData) -> Array[Value] {
        var out: [Value] = [];
        for i in 0..<self.taLength(d) { out.append(self.taGet(d, i)); }
        out
    }

    /// A relative start/end argument resolved against `len`.
    mutating func relativeArg(v: Value, len: Int64, dflt: Int64) -> Result[Int64, Value] {
        if v.isUndefined { return .Ok(dflt); }
        let rel = try self.toIntegerOrInfinity(v);
        .Ok(relativeIndex(rel, len))
    }

    /// The default typed array sort: numeric, -0 before +0, NaN last.
    func taSortNumbers(xs: Array[Value]) -> Array[Value] {
        var work = xs;
        var width: Int64 = 1;
        while width < work.count {
            var next: [Value] = [];
            var lo: Int64 = 0;
            while lo < work.count {
                let mid = if lo + width < work.count { lo + width } else { work.count };
                let hi = if lo + 2 * width < work.count { lo + 2 * width } else { work.count };
                var i = lo;
                var j = mid;
                while i < mid and j < hi {
                    if self.taSortLess(work(j), work(i)) {
                        next.append(work(j));
                        j = j + 1;
                    } else {
                        next.append(work(i));
                        i = i + 1;
                    }
                }
                while i < mid { next.append(work(i)); i = i + 1; }
                while j < hi { next.append(work(j)); j = j + 1; }
                lo = hi;
            }
            work = next;
            width = width * 2;
        }
        work
    }

    /// The default order: numbers as numberSortLess, BigInts by value.
    func taSortLess(a: Value, b: Value) -> Bool {
        if a.isBigInt and b.isBigInt { return bnCmp(self.bigOf(a), self.bigOf(b)) < 0; }
        numberSortLess(a.asNumber, b.asNumber)
    }

    /// The sorted elements of `d`, by `cmp` or numerically.
    mutating func taSorted(d: TypedArrayData, cmp: Value) -> Result[Array[Value], Value] {
        let nums = self.taValues(d);
        if cmp.isUndefined { return .Ok(self.taSortNumbers(nums)); }
        // The comparator may collect garbage: keep BigInt elements alive.
        for x in nums { self.pushRoot(x); }
        self.mergeSort(nums, cmp)
    }

    // ------------------------------------------------------------------
    // DataView
    // ------------------------------------------------------------------

    mutating func constructDataView(args: Array[Value], newTarget: Value) -> Result[Value, Value] {
        if newTarget.isUndefined { return self.throwType("Constructor DataView requires 'new'"); }
        let buffer = arg(args, 0);
        if self.bufferId(buffer) < 0 { return self.throwType("First argument to DataView constructor must be an ArrayBuffer"); }
        let offset = try self.toIndex(arg(args, 1));
        if self.isDetached(buffer) { return self.throwType("Cannot construct a DataView on a detached ArrayBuffer"); }
        let bufLen = self.bufferByteLength(buffer);
        if offset > bufLen { return .Err(self.makeError(.RangeError, "Start offset \(offset) is outside the bounds of the buffer")); }
        let a2 = arg(args, 2);
        var viewLength = bufLen - offset;
        if not a2.isUndefined {
            viewLength = try self.toIndex(a2);
            if offset + viewLength > bufLen { return .Err(self.makeError(.RangeError, "Invalid DataView length \(viewLength)")); }
        }
        let proto = try self.prototypeFor(newTarget, self.intrinsics.dataViewProto);
        if self.isDetached(buffer) { return self.throwType("Cannot construct a DataView on a detached ArrayBuffer"); }
        .Ok(self.newObjectKind(proto, .DataViewObject(buffer: buffer, offset: offset, length: viewLength)))
    }

    func dataView(v: Value) -> Optional[(Value, Int64, Int64)] {
        if not v.isObject { return .None; }
        match self.kindAt(v.asRef) {
            .DataViewObject(b, o, l) => .Some((b, o, l)),
            _ => .None
        }
    }

    /// DataView get/set of element `kind`; `value` is None for a get.
    mutating func dataViewAccess(this: Value, kind: Int64, args: Array[Value], isSet: Bool) -> Result[Value, Value] {
        let (buffer, offset, length) = match self.dataView(this) {
            .Some(x) => x,
            .None => return self.throwType("DataView method called on incompatible receiver")
        };
        let index = try self.toIndex(arg(args, 0));
        let x = if isSet { try self.taConvert(kind, arg(args, 1)) } else { Value.undefined };
        let little = self.toBoolean(arg(args, if isSet { 2 } else { 1 }));
        if self.isDetached(buffer) { return self.throwType("Cannot perform DataView access on a detached ArrayBuffer"); }
        if index + taElementSize(kind) > length { return .Err(self.makeError(.RangeError, "Offset is outside the bounds of the DataView")); }
        let id = self.bufferId(buffer);
        if isSet {
            self.rawSet(id, offset + index, kind, x, little);
            return .Ok(Value.undefined);
        }
        .Ok(self.rawGet(id, offset + index, kind, little))
    }

    // ------------------------------------------------------------------
    // Natives
    // ------------------------------------------------------------------

    mutating func typedArrayNative(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .ArrayBufferCtor => {
                if newTarget.isUndefined { return self.throwType("Constructor ArrayBuffer requires 'new'"); }
                let len = try self.toIndex(a0);
                let proto = try self.prototypeFor(newTarget, self.intrinsics.arrayBufferProto);
                try self.allocateArrayBuffer(proto, len)
            },
            .ArrayBufferIsView => {
                let isView = self.taData(a0).isSome or self.dataView(a0).isSome;
                Value.bool(isView)
            },
            .ArrayBufferByteLength => {
                if self.bufferId(this) < 0 { return self.throwType("ArrayBuffer.prototype.byteLength called on incompatible receiver"); }
                Value.int(self.bufferByteLength(this))
            },
            .ArrayBufferSlice => {
                let id = self.bufferId(this);
                if id < 0 { return self.throwType("ArrayBuffer.prototype.slice called on incompatible receiver"); }
                if self.isDetached(this) { return self.throwType("Cannot slice a detached ArrayBuffer"); }
                let len = self.bufferByteLength(this);
                let first = try self.relativeArg(a0, len, 0);
                let last = try self.relativeArg(a1, len, len);
                let newLen = if last > first { last - first } else { 0 };
                let abCtor = self.ownValue(self.global.asRef, self.atom("ArrayBuffer"));
                let c = try self.speciesConstructor(this, abCtor);
                let out = try self.construct(c, [Value.int(newLen)], c);
                let oid = self.bufferId(out);
                if oid < 0 { return self.throwType("ArrayBuffer subclass returned this from species constructor"); }
                if self.isDetached(out) { return self.throwType("Species constructor returned a detached ArrayBuffer"); }
                if out.same(this) { return self.throwType("ArrayBuffer subclass returned this from species constructor"); }
                if self.bufferByteLength(out) < newLen { return self.throwType("Species constructor returned a too-small ArrayBuffer"); }
                if self.isDetached(this) { return self.throwType("Cannot slice a detached ArrayBuffer"); }
                for i in 0..<newLen { self.heap.buffers(oid).bytes(i) = self.heap.buffers(id).bytes(first + i); }
                out
            },
            .DetachArrayBuffer => {
                let id = self.bufferId(a0);
                if id < 0 { return self.throwType("not an ArrayBuffer"); }
                self.heap.buffers(id) = BufferData(bytes: [], detached: true);
                Value.nullValue
            },
            .TypedArrayCtor => return self.throwType("Abstract class TypedArray not directly constructable"),
            .TypedArrayKindCtor => {
                let kind = self.captures(callee)(0).asNumber.toInt64() ?? 0;
                let mark = self.roots.count;
                let o = self.constructTypedArray(kind, args, newTarget);
                self.truncateRoots(mark);
                try o
            },
            .TypedArrayFrom or .TypedArrayOf => {
                let mark = self.roots.count;
                let o = self.typedArrayFromOf(n, this, args);
                self.truncateRoots(mark);
                try o
            },
            .TABuffer or .TAByteLength or .TAByteOffset or .TALength => {
                let d = match self.taData(this) {
                    .Some(x) => x,
                    .None => return self.throwType("%TypedArray%.prototype getter called on incompatible receiver")
                };
                let detached = self.isDetached(d.buffer);
                match n {
                    .TABuffer => d.buffer,
                    .TAByteLength => Value.int(if detached { 0 } else { d.length * taElementSize(d.kind) }),
                    .TAByteOffset => Value.int(if detached { 0 } else { d.offset }),
                    _ => Value.int(if detached { 0 } else { d.length })
                }
            },
            .TAToStringTag => match self.taData(this) {
                .Some(d) => self.str(taName(d.kind)),
                .None => Value.undefined
            },
            .DataViewCtor => try self.constructDataView(args, newTarget),
            .DVBuffer or .DVByteLength or .DVByteOffset => {
                let (buffer, offset, length) = match self.dataView(this) {
                    .Some(x) => x,
                    .None => return self.throwType("DataView getter called on incompatible receiver")
                };
                if n == NativeFn.DVBuffer { return .Ok(.Some(buffer)); }
                // A detached view is out of bounds: both read as 0.
                if self.isDetached(buffer) { return .Ok(.Some(Value.int(0))); }
                Value.int(if n == NativeFn.DVByteLength { length } else { offset })
            },
            .DVGet or .DVSet => {
                let kind = self.captures(callee)(0).asNumber.toInt64() ?? 0;
                try self.dataViewAccess(this, kind, args, n == NativeFn.DVSet)
            },
            _ => {
                let mark = self.roots.count;
                let o = self.typedArrayMethod(n, this, args);
                self.truncateRoots(mark);
                return o;
            }
        };
        .Ok(.Some(r))
    }

    /// %TypedArray%.from and .of.
    mutating func typedArrayFromOf(n: NativeFn, c: Value, args: Array[Value]) -> Result[Value, Value] {
        if not self.isConstructor(c) { return self.throwType("\(self.describe(c)) is not a constructor"); }
        if n == NativeFn.TypedArrayOf {
            let o = try self.taCreateFrom(c, [Value.int(args.count)], args.count);
            self.pushRoot(o);
            for i in 0..<args.count { try self.setIndex(o, i, args(i)); }
            return .Ok(o);
        }
        let source = arg(args, 0);
        let mapfn = arg(args, 1);
        let thisArg = arg(args, 2);
        if not mapfn.isUndefined and not self.isCallable(mapfn) { return self.throwType("\(self.describe(mapfn)) is not a function"); }
        let usingIterator = try self.getMethod(source, symbolKey(WellKnown.iterator));
        var values: [Value] = [];
        var src = source;
        var len: Int64 = 0;
        if not usingIterator.isUndefined {
            values = try self.iterateWith(source, usingIterator);
            len = values.count;
        } else {
            src = try self.toObject(source);
            self.pushRoot(src);
            len = try self.lengthOf(src);
        }
        let o = try self.taCreateFrom(c, [Value.int(len)], len);
        self.pushRoot(o);
        for i in 0..<len {
            let v = if usingIterator.isUndefined { try self.getIndex(src, i) } else { values(i) };
            let mapped = if mapfn.isUndefined { v } else { try self.callValue(mapfn, thisArg, [v, Value.int(i)]) };
            try self.setIndex(o, i, mapped);
        }
        .Ok(o)
    }

    /// The %TypedArray%.prototype methods.
    mutating func typedArrayMethod(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        // sort and toSorted check the comparator before the receiver.
        if n == NativeFn.TASort or n == NativeFn.TAToSorted {
            if not a0.isUndefined and not self.isCallable(a0) { return self.throwType("The comparison function must be either a function or undefined"); }
        }
        let name = match n {
            .TAAt => "at", .TACopyWithin => "copyWithin", .TAEntries => "entries", .TAEvery => "every",
            .TAFill => "fill", .TAFilter => "filter", .TAFind => "find", .TAFindIndex => "findIndex",
            .TAFindLast => "findLast", .TAFindLastIndex => "findLastIndex", .TAForEach => "forEach",
            .TAIncludes => "includes", .TAIndexOf => "indexOf", .TAJoin => "join", .TAKeys => "keys",
            .TALastIndexOf => "lastIndexOf", .TAMap => "map", .TAReduce => "reduce", .TAReduceRight => "reduceRight",
            .TAReverse => "reverse", .TASet => "set", .TASlice => "slice", .TASome => "some", .TASort => "sort",
            .TASubarray => "subarray", .TAToLocaleString => "toLocaleString", .TAToReversed => "toReversed",
            .TAToSorted => "toSorted", .TAValues => "values", .TAWith => "with",
            _ => return .Ok(.None)
        };
        // set and subarray work on detached arrays far enough to throw
        // their own errors.
        if n == NativeFn.TASet { return .Ok(.Some(try self.taSetMethod(this, a0, a1))); }
        if n == NativeFn.TASubarray { return .Ok(.Some(try self.taSubarray(this, a0, a1))); }
        let d = try self.validateTA(this, name);
        let len = d.length;
        let r: Value = match n {
            .TAAt => {
                let rel = try self.toIntegerOrInfinity(a0);
                let k = if rel >= 0.0 { rel } else { Float64(from: len) + rel };
                if k < 0.0 or k >= Float64(from: self.taLength(d)) { Value.undefined } else { self.taGet(d, k.toInt64() ?? 0) }
            },
            .TAEntries => self.newObjectKind(self.intrinsics.arrayIteratorProto, .ArrayIterator(target: this, index: 0, kind: 2)),
            .TAKeys => self.newObjectKind(self.intrinsics.arrayIteratorProto, .ArrayIterator(target: this, index: 0, kind: 0)),
            .TAValues => self.newObjectKind(self.intrinsics.arrayIteratorProto, .ArrayIterator(target: this, index: 0, kind: 1)),
            .TAEvery or .TASome or .TAFind or .TAFindIndex or .TAFindLast or .TAFindLastIndex or .TAForEach => {
                let f = try self.requireCallable(a0);
                let backwards = n == NativeFn.TAFindLast or n == NativeFn.TAFindLastIndex;
                for j in 0..<len {
                    let i = if backwards { len - 1 - j } else { j };
                    let v = try self.getIndex(this, i);
                    let res = try self.callValue(f, a1, [v, Value.int(i), this]);
                    let t = self.toBoolean(res);
                    if n == NativeFn.TAEvery and not t { return .Ok(.Some(Value.bool(false))); }
                    if n == NativeFn.TASome and t { return .Ok(.Some(Value.bool(true))); }
                    if (n == NativeFn.TAFind or n == NativeFn.TAFindLast) and t { return .Ok(.Some(v)); }
                    if (n == NativeFn.TAFindIndex or n == NativeFn.TAFindLastIndex) and t { return .Ok(.Some(Value.int(i))); }
                }
                match n {
                    .TAEvery => Value.bool(true),
                    .TASome => Value.bool(false),
                    .TAFindIndex or .TAFindLastIndex => Value.int(0 - 1),
                    _ => Value.undefined
                }
            },
            .TAFill => {
                let x = try self.taConvert(d.kind, a0);
                let k = try self.relativeArg(a1, len, 0);
                let end = try self.relativeArg(arg(args, 2), len, len);
                if self.isDetached(d.buffer) { return self.throwType("Cannot fill a detached ArrayBuffer"); }
                var i = k;
                while i < end and i < self.taLength(d) {
                    self.taPut(d, i, x);
                    i = i + 1;
                }
                this
            },
            .TAFilter => {
                let f = try self.requireCallable(a0);
                var kept: [Value] = [];
                for i in 0..<len {
                    let v = try self.getIndex(this, i);
                    let res = try self.callValue(f, a1, [v, Value.int(i), this]);
                    if self.toBoolean(res) { kept.append(v); }
                }
                let o = try self.taSpeciesCreate(this, d, [Value.int(kept.count)], kept.count);
                self.pushRoot(o);
                for i in 0..<kept.count { try self.setIndex(o, i, kept(i)); }
                o
            },
            .TAMap => {
                let f = try self.requireCallable(a0);
                let o = try self.taSpeciesCreate(this, d, [Value.int(len)], len);
                self.pushRoot(o);
                for i in 0..<len {
                    let v = try self.getIndex(this, i);
                    let mapped = try self.callValue(f, a1, [v, Value.int(i), this]);
                    try self.setIndex(o, i, mapped);
                }
                o
            },
            .TAReduce or .TAReduceRight => {
                let f = try self.requireCallable(a0);
                let right = n == NativeFn.TAReduceRight;
                var j: Int64 = 0;
                var acc = a1;
                if args.count < 2 {
                    if len == 0 { return self.throwType("Reduce of empty array with no initial value"); }
                    acc = try self.getIndex(this, if right { len - 1 } else { 0 });
                    j = 1;
                }
                while j < len {
                    let i = if right { len - 1 - j } else { j };
                    let v = try self.getIndex(this, i);
                    self.pushRoot(acc);
                    acc = try self.callValue(f, Value.undefined, [acc, v, Value.int(i), this]);
                    j = j + 1;
                }
                acc
            },
            .TAIncludes or .TAIndexOf or .TALastIndexOf => {
                let notFound = if n == NativeFn.TAIncludes { Value.bool(false) } else { Value.int(0 - 1) };
                if len == 0 { return .Ok(.Some(notFound)); }
                let last = n == NativeFn.TALastIndexOf;
                var from: Float64 = if last { Float64(from: len - 1) } else { 0.0 };
                if args.count > 1 { from = try self.toIntegerOrInfinity(a1); }
                if from == Float64.infinity and not last { return .Ok(.Some(notFound)); }
                var k: Float64 = from;
                if from < 0.0 { k = Float64(from: len) + from; }
                if last {
                    if k >= Float64(from: len) { k = Float64(from: len - 1); }
                    var i = k.toInt64() ?? (0 - 1);
                    while i >= 0 {
                        if i < self.taLength(d) and self.strictEquals(self.taGet(d, i), a0) { return .Ok(.Some(Value.int(i))); }
                        i = i - 1;
                    }
                    return .Ok(.Some(notFound));
                }
                if k < 0.0 { k = 0.0; }
                var i = k.toInt64() ?? len;
                while i < len {
                    let v = if i < self.taLength(d) { self.taGet(d, i) } else { Value.undefined };
                    let found = if n == NativeFn.TAIncludes { self.sameValueZero(v, a0) } else { i < self.taLength(d) and self.strictEquals(v, a0) };
                    if found { return .Ok(.Some(if n == NativeFn.TAIncludes { Value.bool(true) } else { Value.int(i) })); }
                    i = i + 1;
                }
                notFound
            },
            .TAJoin or .TAToLocaleString => {
                let sep = if n == NativeFn.TAJoin and not a0.isUndefined { try self.toUnits(a0) } else { unitsOf(",") };
                var out: Units = [];
                for i in 0..<len {
                    if i > 0 { for u in sep { out.append(u); } }
                    let v = try self.getIndex(this, i);
                    if v.isUndefined { continue; }
                    let s = if n == NativeFn.TAJoin { v } else { try self.invoke(v, "toLocaleString", []) };
                    for u in try self.toUnits(s) { out.append(u); }
                }
                self.newString(out)
            },
            .TAReverse => {
                var lo: Int64 = 0;
                var hi = len - 1;
                while lo < hi {
                    let a = self.taGet(d, lo);
                    let b = self.taGet(d, hi);
                    self.taPut(d, lo, b);
                    self.taPut(d, hi, a);
                    lo = lo + 1;
                    hi = hi - 1;
                }
                this
            },
            .TACopyWithin => {
                let to = try self.relativeArg(a0, len, 0);
                let from = try self.relativeArg(a1, len, 0);
                let fin = try self.relativeArg(arg(args, 2), len, len);
                var count = fin - from;
                if len - to < count { count = len - to; }
                if count > 0 {
                    if self.isDetached(d.buffer) { return self.throwType("Cannot copyWithin on a detached ArrayBuffer"); }
                    let size = taElementSize(d.kind);
                    let id = self.bufferId(d.buffer);
                    var bytes: [UInt8] = [];
                    let src = d.offset + from * size;
                    for i in 0..<(count * size) { bytes.append(self.heap.buffers(id).bytes(src + i)); }
                    let dst = d.offset + to * size;
                    for i in 0..<bytes.count { self.heap.buffers(id).bytes(dst + i) = bytes(i); }
                }
                this
            },
            .TASlice => {
                let k = try self.relativeArg(a0, len, 0);
                let fin = try self.relativeArg(a1, len, len);
                let count = if fin > k { fin - k } else { 0 };
                let o = try self.taSpeciesCreate(this, d, [Value.int(count)], count);
                self.pushRoot(o);
                if count > 0 {
                    if self.isDetached(d.buffer) { return self.throwType("Cannot slice a detached ArrayBuffer"); }
                    let od = self.taData(o) ?? d;
                    let end = if fin < self.taLength(d) { fin } else { self.taLength(d) };
                    if od.kind == d.kind {
                        // Same type: copy byte by byte, front to back, NaN
                        // payloads and all. The species result may share the
                        // buffer, so the overlap behaviour is the spec's.
                        let size = taElementSize(d.kind);
                        let sid = self.bufferId(d.buffer);
                        let tid = self.bufferId(od.buffer);
                        var i = k * size;
                        var j = od.offset;
                        while i < end * size {
                            self.heap.buffers(tid).bytes(j) = self.heap.buffers(sid).bytes(d.offset + i);
                            i = i + 1;
                            j = j + 1;
                        }
                    } else {
                        var i = k;
                        var j: Int64 = 0;
                        while i < end {
                            try self.setIndex(o, j, self.taGet(d, i));
                            i = i + 1;
                            j = j + 1;
                        }
                    }
                }
                o
            },
            .TASort => {
                let sorted = try self.taSorted(d, a0);
                let n2 = if sorted.count < self.taLength(d) { sorted.count } else { self.taLength(d) };
                for i in 0..<n2 { self.taPut(d, i, sorted(i)); }
                this
            },
            .TAToSorted => {
                let p = self.typedArrayProtos(d.kind);
                let o = try self.newTypedArray(d.kind, len, p);
                self.pushRoot(o);
                let od = self.taData(o) ?? d;
                let sorted = try self.taSorted(d, a0);
                for i in 0..<sorted.count { self.taPut(od, i, sorted(i)); }
                o
            },
            .TAToReversed => {
                let p = self.typedArrayProtos(d.kind);
                let o = try self.newTypedArray(d.kind, len, p);
                let od = self.taData(o) ?? d;
                for i in 0..<len {
                    let v = self.taGet(d, len - 1 - i);
                    self.taPut(od, i, v);
                }
                o
            },
            .TAWith => {
                let rel = try self.toIntegerOrInfinity(a0);
                let actual = if rel >= 0.0 { rel } else { Float64(from: len) + rel };
                let x = try self.taConvert(d.kind, a1);
                if not self.taValidIndex(d, actual) { return .Err(self.makeError(.RangeError, "Invalid typed array index")); }
                let p = self.typedArrayProtos(d.kind);
                let o = try self.newTypedArray(d.kind, len, p);
                let od = self.taData(o) ?? d;
                let at = actual.toInt64() ?? 0;
                for i in 0..<len {
                    let v = if i == at { x } else { self.taGet(d, i) };
                    self.taPut(od, i, v);
                }
                o
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// %TypedArray%.prototype.set(source, offset).
    mutating func taSetMethod(this: Value, source: Value, offsetArg: Value) -> Result[Value, Value] {
        let d = match self.taData(this) {
            .Some(x) => x,
            .None => return self.throwType("%TypedArray%.prototype.set called on incompatible receiver")
        };
        let offset = try self.toIntegerOrInfinity(offsetArg);
        if offset < 0.0 { return .Err(self.makeError(.RangeError, "offset is out of bounds")); }
        if self.isDetached(d.buffer) { return self.throwType("Cannot set on a detached ArrayBuffer"); }
        let targetLen = d.length;
        if let .Some(src) = self.taData(source) {
            if self.isDetached(src.buffer) { return self.throwType("Cannot set from a detached ArrayBuffer"); }
            if Float64(from: src.length) + offset > Float64(from: targetLen) { return .Err(self.makeError(.RangeError, "offset is out of bounds")); }
            // Read everything first: the two may share a buffer.
            if taIsBig(d.kind) != taIsBig(src.kind) { return self.throwType("Content type of the source doesn't match \(taName(d.kind))"); }
            let nums = self.taValues(src);
            let at = offset.toInt64() ?? 0;
            for i in 0..<nums.count { self.taPut(d, at + i, nums(i)); }
            return .Ok(Value.undefined);
        }
        let src = try self.toObject(source);
        self.pushRoot(src);
        let srcLen = try self.lengthOf(src);
        if offset == Float64.infinity or Float64(from: srcLen) + offset > Float64(from: targetLen) {
            return .Err(self.makeError(.RangeError, "offset is out of bounds"));
        }
        let at = offset.toInt64() ?? 0;
        for i in 0..<srcLen {
            let v = try self.getIndex(src, i);
            try self.taSetElement(d, Float64(from: at + i), v);
        }
        .Ok(Value.undefined)
    }

    /// %TypedArray%.prototype.subarray(begin, end).
    mutating func taSubarray(this: Value, begin: Value, end: Value) -> Result[Value, Value] {
        let d = match self.taData(this) {
            .Some(x) => x,
            .None => return self.throwType("%TypedArray%.prototype.subarray called on incompatible receiver")
        };
        let srcLen = self.taLength(d);
        let b = try self.relativeArg(begin, srcLen, 0);
        let e = try self.relativeArg(end, srcLen, srcLen);
        let newLen = if e > b { e - b } else { 0 };
        let byteOffset = d.offset + b * taElementSize(d.kind);
        self.taSpeciesCreate(this, d, [d.buffer, Value.int(byteOffset), Value.int(newLen)], -1)
    }
}

/// The default typed array order: numbers ascending, -0 before +0, NaN
/// after everything.
func numberSortLess(a: Float64, b: Float64) -> Bool {
    if a.isNaN { return false; }
    if b.isNaN { return true; }
    if a < b { return true; }
    if a > b { return false; }
    a == 0.0 and b == 0.0 and a.isSignMinus and not b.isSignMinus
}

/// 1 as an Int64, for building bit masks with `<<`.
func oneBit() -> Int64 { 1 }

/// The low 64 bits of `a` in two's complement (ToBigInt64/ToBigUint64's
/// bit pattern).
func bigLow64(a: BigNum) -> UInt64 {
    let u = bnAsUintN(64, a);
    (limb(u.mag, 1) << 32) | limb(u.mag, 0)
}
