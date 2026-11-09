module js

// BigInt values and built-ins. A BigInt lives in the string space (so
// the GC treats it like a string): its sign unit, then each 32-bit limb
// as two 16-bit units, least significant first. The arithmetic is
// bigint.ks.

/// The largest BigInt the engine builds, in bits.
func maxBigIntBits() -> Int64 { 16777216 }

extend VM {
    mutating func newBigInt(a: BigNum) -> Value {
        var u: [UInt16] = [if a.neg { 1 } else { 0 }];
        for x in a.mag {
            let w = UInt64(from: x);
            u.append(UInt16(truncating: w & 0xFFFF));
            u.append(UInt16(truncating: w >> 16));
        }
        Value.bigint(self.heap.allocString(u))
    }

    func bigOf(v: Value) -> BigNum {
        let u = self.heap.strings(v.asRef);
        var mag: [UInt32] = [];
        var i: Int64 = 1;
        while i + 1 < u.count {
            mag.append(UInt32(truncating: UInt64(from: u(i)) | (UInt64(from: u(i + 1)) << 16)));
            i = i + 2;
        }
        BigNum(neg: u.count > 0 and u(0) == 1, mag: mag)
    }

    func bigIsZero(v: Value) -> Bool { self.heap.strings(v.asRef).count <= 1 }

    /// A BigInt result, refusing ones too large to hold.
    mutating func bigResult(a: BigNum) -> Result[Value, Value] {
        if magBitLength(a.mag) > maxBigIntBits() { return .Err(self.makeError(.RangeError, "Maximum BigInt size exceeded")); }
        .Ok(self.newBigInt(a))
    }

    // ------------------------------------------------------------------
    // Conversions
    // ------------------------------------------------------------------

    /// ToNumeric: a number or a BigInt.
    mutating func toNumeric(v: Value) -> Result[Value, Value] {
        if v.isNumber or v.isBigInt { return .Ok(v); }
        let p = try self.toPrimitive(v, "number");
        if p.isBigInt { return .Ok(p); }
        .Ok(Value.number(try self.toNumber(p)))
    }

    /// ToBigInt.
    mutating func toBigInt(v: Value) -> Result[BigNum, Value] {
        let p = try self.toPrimitive(v, "number");
        if p.isBigInt { return .Ok(self.bigOf(p)); }
        if p.isBool { return .Ok(bnFromInt(if p.asBool { 1 } else { 0 })); }
        if p.isString {
            return match stringToBigNum(self.units(p)) {
                .Some(b) => .Ok(b),
                .None => .Err(self.makeError(.SyntaxError, "Cannot convert \(self.text(p)) to a BigInt"))
            };
        }
        if p.isNumber { return self.throwType("Cannot convert \(numberToString(p.asNumber)) to a BigInt"); }
        if p.isSymbol { return self.throwType("Cannot convert a Symbol value to a BigInt"); }
        self.throwType("Cannot convert \(if p.isNull { "null" } else { "undefined" }) to a BigInt")
    }

    /// NumberToBigInt: integral numbers only.
    mutating func numberToBigInt(x: Float64) -> Result[BigNum, Value] {
        if x.isNaN or x.isInfinite or x != x.trunc() {
            return .Err(self.makeError(.RangeError, "The number \(numberToString(x)) cannot be converted to a BigInt because it is not an integer"));
        }
        .Ok(bnFromFloat(x))
    }

    /// thisBigIntValue.
    mutating func thisBigInt(v: Value, name: String) -> Result[Value, Value] {
        if v.isBigInt { return .Ok(v); }
        if v.isObject {
            if let .BigIntBox(b) = self.kindAt(v.asRef) { return .Ok(b); }
        }
        self.throwType("BigInt.prototype.\(name) requires that 'this' be a BigInt")
    }

    // ------------------------------------------------------------------
    // Operators
    // ------------------------------------------------------------------

    /// A binary operator on two BigInts.
    mutating func bigBinary(op: BinOp, a: Value, b: Value) -> Result[Value, Value] {
        let x = self.bigOf(a);
        let y = self.bigOf(b);
        match op {
            .Add => self.bigResult(bnAdd(x, y)),
            .Sub => self.bigResult(bnSub(x, y)),
            .Mul => {
                if magBitLength(x.mag) + magBitLength(y.mag) > maxBigIntBits() + 1 { return .Err(self.makeError(.RangeError, "Maximum BigInt size exceeded")); }
                self.bigResult(bnMul(x, y))
            },
            .Div or .Mod => {
                if bnIsZero(y) { return .Err(self.makeError(.RangeError, "Division by zero")); }
                .Ok(self.newBigInt(if op == BinOp.Div { bnDiv(x, y) } else { bnRem(x, y) }))
            },
            .Exp => {
                if y.neg { return .Err(self.makeError(.RangeError, "Exponent must be non-negative")); }
                if bnIsZero(y) { return .Ok(self.newBigInt(bnFromInt(1))); }
                let xl = magBitLength(x.mag);
                // 0, 1 and -1 stay small whatever the exponent.
                if xl <= 1 {
                    let odd = y.mag(0) % 2 == 1;
                    return .Ok(self.newBigInt(if x.neg and not odd { bnNeg(x) } else { x }));
                }
                if y.mag.count > 1 or (xl - 1) * Int64(from: y.mag(0)) > maxBigIntBits() {
                    return .Err(self.makeError(.RangeError, "Maximum BigInt size exceeded"));
                }
                self.bigResult(bnPow(x, Int64(from: y.mag(0))))
            },
            .Shl or .Shr => {
                let right = op == BinOp.Shr;
                let leftward = y.neg == right;
                // A shift count past any size we hold.
                if y.mag.count > 1 or Int64(from: limb(y.mag, 0)) > maxBigIntBits() {
                    if leftward and not bnIsZero(x) { return .Err(self.makeError(.RangeError, "Maximum BigInt size exceeded")); }
                    if leftward { return .Ok(a); }
                    return .Ok(self.newBigInt(if x.neg { bnFromInt(-1) } else { bnZero() }));
                }
                let n = Int64(bitPattern: limb(y.mag, 0));
                let count = if leftward { n } else { 0 - n };
                self.bigResult(bnShl(x, count))
            },
            .UShr => self.throwType("BigInts have no unsigned right shift, use >> instead"),
            .BitAnd => .Ok(self.newBigInt(bnBitwise(x, y, 0))),
            .BitOr => .Ok(self.newBigInt(bnBitwise(x, y, 1))),
            .BitXor => .Ok(self.newBigInt(bnBitwise(x, y, 2))),
            _ => self.throwType("Invalid BigInt operation")
        }
    }

    /// A numeric binary operator after ToNumeric on both sides.
    mutating func numericBinary(op: BinOp, x: Value, y: Value) -> Result[Value, Value] {
        if x.isBigInt and y.isBigInt { return self.bigBinary(op, x, y); }
        if x.isBigInt or y.isBigInt { return self.throwType("Cannot mix BigInt and other types, use explicit conversions"); }
        if op == BinOp.Add { return .Ok(Value.number(x.asNumber + y.asNumber)); }
        .Ok(Value.number(arithmetic(op, x.asNumber, y.asNumber)))
    }

    /// A unary operator on a BigInt.
    mutating func bigUnary(op: UnOp, v: Value) -> Result[Value, Value] {
        let x = self.bigOf(v);
        match op {
            .Neg => .Ok(self.newBigInt(bnNeg(x))),
            .BitNot => .Ok(self.newBigInt(bnSub(bnNeg(x), bnFromInt(1)))),
            .Inc => self.bigResult(bnAdd(x, bnFromInt(1))),
            .Dec => self.bigResult(bnSub(x, bnFromInt(1))),
            .ToNumeric => .Ok(v),
            _ => self.throwType("Cannot convert a BigInt value to a number")
        }
    }

    /// `a == b` where `a` is a BigInt and `b` a number or string.
    func bigLooseEquals(a: Value, b: Value) -> Bool {
        let x = self.bigOf(a);
        if b.isString {
            return match stringToBigNum(self.units(b)) {
                .Some(y) => bnEq(x, y),
                .None => false
            };
        }
        let n = b.asNumber;
        if n.isNaN or n.isInfinite { return false; }
        bnCmpFloat(x, n) == 0
    }

    /// `a < b` with at least one BigInt; None when a number is NaN or a
    /// string isn't an integer.
    func bigLessThan(a: Value, b: Value) -> Optional[Bool] {
        if a.isBigInt and b.isBigInt { return .Some(bnCmp(self.bigOf(a), self.bigOf(b)) < 0); }
        if a.isBigInt {
            if b.isString {
                return match stringToBigNum(self.units(b)) {
                    .Some(y) => .Some(bnCmp(self.bigOf(a), y) < 0),
                    .None => .None
                };
            }
            let n = b.asNumber;
            if n.isNaN { return .None; }
            if n.isInfinite { return .Some(n > 0.0); }
            return .Some(bnCmpFloat(self.bigOf(a), n) < 0);
        }
        if a.isString {
            return match stringToBigNum(self.units(a)) {
                .Some(x) => .Some(bnCmp(x, self.bigOf(b)) < 0),
                .None => .None
            };
        }
        let n = a.asNumber;
        if n.isNaN { return .None; }
        if n.isInfinite { return .Some(n < 0.0); }
        .Some(bnCmpFloat(self.bigOf(b), n) > 0)
    }

    // ------------------------------------------------------------------
    // Built-ins
    // ------------------------------------------------------------------

    mutating func setupBigInt() {
        let proto = self.newObject();
        self.intrinsics.bigintProto = proto;
        let c = self.constructorFor("BigInt", .BigIntCtor, 1, proto);
        self.method(c, "asIntN", .BigIntAsIntN, 2);
        self.method(c, "asUintN", .BigIntAsUintN, 2);
        self.method(proto, "toString", .BigIntToString, 0);
        self.method(proto, "toLocaleString", .BigIntToLocaleString, 0);
        self.method(proto, "valueOf", .BigIntValueOf, 0);
        let tag = self.str("BigInt");
        self.defineOwn(proto.asRef, symbolKey(WellKnown.toStringTag), tag, Attr.configurable);
    }

    mutating func bigintNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .BigIntCtor => {
                if not newTarget.isUndefined { return self.throwType("BigInt is not a constructor"); }
                let p = try self.toPrimitive(a0, "number");
                let b = if p.isNumber { try self.numberToBigInt(p.asNumber) } else { try self.toBigInt(p) };
                self.newBigInt(b)
            },
            .BigIntAsIntN or .BigIntAsUintN => {
                let bits = try self.toIndex(a0);
                let b = try self.toBigInt(a1);
                let r = if n == NativeFn.BigIntAsIntN { bnAsIntN(bits, b) } else { bnAsUintN(bits, b) };
                try self.bigResult(r)
            },
            .BigIntToString or .BigIntToLocaleString => {
                let v = try self.thisBigInt(this, "toString");
                var radix: Int64 = 10;
                if n == NativeFn.BigIntToString and not a0.isUndefined {
                    let rf = try self.toIntegerOrInfinity(a0);
                    if rf < 2.0 or rf > 36.0 { return .Err(self.makeError(.RangeError, "toString() radix must be between 2 and 36")); }
                    radix = rf.toInt64() ?? 10;
                }
                let s = bnToString(self.bigOf(v), radix);
                self.str(s)
            },
            .BigIntValueOf => try self.thisBigInt(this, "valueOf"),
            _ => return .Ok(.None)
        };
        let _ = a1;
        .Ok(.Some(r))
    }
}

/// StringToBigInt: whitespace-trimmed decimal (signed) or 0x/0o/0b
/// digits; the empty string is 0n.
func stringToBigNum(u: Array[UInt16]) -> Optional[BigNum] {
    let t = unitsTrim(u, true, true);
    return stringToBigNumTrimmed(t);
}

func stringToBigNumTrimmed(u: Array[UInt16]) -> Optional[BigNum] {
    var start: Int64 = 0;
    let end = u.count;
    if start == end { return .Some(bnZero()); }
    if end - start > 2 and u(start) == 48 {
        let p = Int64(from: u(start + 1)) | 32;
        let radix: Int64 = if p == 120 { 16 } else if p == 111 { 8 } else if p == 98 { 2 } else { 0 };
        if radix > 0 {
            return match bnParseDigits(u, start + 2, end, radix) {
                .Some(m) => .Some(bnMake(false, m)),
                .None => .None
            };
        }
    }
    var neg = false;
    if u(start) == 45 or u(start) == 43 {
        neg = u(start) == 45;
        start = start + 1;
    }
    match bnParseDigits(u, start, end, 10) {
        .Some(m) => .Some(bnMake(neg, m)),
        .None => .None
    }
}
