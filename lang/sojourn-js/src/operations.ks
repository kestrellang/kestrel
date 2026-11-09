module js

// The abstract operations: type conversion, equality, comparison and
// the arithmetic and bitwise operators.

extend VM {
    func toBoolean(v: Value) -> Bool {
        if v.isNumber {
            let n = v.asNumber;
            return not (n == 0.0 or n.isNaN);
        }
        if v.isBool { return v.asBool; }
        if v.isString { return self.heap.strings(v.asRef).count > 0; }
        if v.isNullish or v.isEmpty { return false; }
        true
    }

    /// ToPrimitive, with `hint` "default", "number" or "string".
    mutating func toPrimitive(v: Value, hint: String) -> Result[Value, Value] {
        if not v.isObject { return .Ok(v); }
        let exotic = try self.getFrom(v, symbolKey(WellKnown.toPrimitive), v);
        if not exotic.isNullish {
            if not self.isCallable(exotic) { return self.throwType("Symbol.toPrimitive isn't a function"); }
            let h = self.str(hint);
            let r = try self.callValue(exotic, v, [h]);
            if r.isObject { return self.throwType("Cannot convert object to primitive value"); }
            return .Ok(r);
        }
        self.ordinaryToPrimitive(v, hint)
    }

    /// OrdinaryToPrimitive: toString then valueOf for "string", the
    /// other way round otherwise.
    mutating func ordinaryToPrimitive(v: Value, hint: String) -> Result[Value, Value] {
        let first = if hint == "string" { self.names.toString } else { self.names.valueOf };
        let second = if hint == "string" { self.names.valueOf } else { self.names.toString };
        for k in [first, second] {
            let m = try self.getFrom(v, k, v);
            if self.isCallable(m) {
                let r = try self.callValue(m, v, []);
                if not r.isObject { return .Ok(r); }
            }
        }
        self.throwType("Cannot convert object to primitive value")
    }

    mutating func toNumber(v: Value) -> Result[Float64, Value] {
        if v.isNumber { return .Ok(v.asNumber); }
        if v.isUndefined { return .Ok(Float64.nan); }
        if v.isNull { return .Ok(0.0); }
        if v.isBool { return .Ok(if v.asBool { 1.0 } else { 0.0 }); }
        if v.isString { return .Ok(stringToNumber(self.units(v))); }
        if v.isSymbol { return self.throwType("Cannot convert a Symbol value to a number"); }
        let p = try self.toPrimitive(v, "number");
        self.toNumber(p)
    }

    /// ToString, as code units.
    mutating func toUnits(v: Value) -> Result[Units, Value] {
        if v.isString { return .Ok(self.units(v)); }
        if v.isNumber { return .Ok(unitsOf(numberToString(v.asNumber))); }
        if v.isUndefined { return .Ok(unitsOf("undefined")); }
        if v.isNull { return .Ok(unitsOf("null")); }
        if v.isBool { return .Ok(unitsOf(if v.asBool { "true" } else { "false" })); }
        if v.isSymbol { return self.throwType("Cannot convert a Symbol value to a string"); }
        let p = try self.toPrimitive(v, "string");
        self.toUnits(p)
    }

    /// ToString, as a string value.
    mutating func toStringValue(v: Value) -> Result[Value, Value] {
        if v.isString { return .Ok(v); }
        let u = try self.toUnits(v);
        .Ok(self.newString(u))
    }

    /// ToString, as a Kestrel string (for messages and printing).
    mutating func toText(v: Value) -> Result[String, Value] {
        let u = try self.toUnits(v);
        .Ok(stringOf(u))
    }

    mutating func toObject(v: Value) -> Result[Value, Value] {
        if v.isObject { return .Ok(v); }
        if v.isNullish { return self.throwType("Cannot convert undefined or null to object"); }
        if v.isString { return .Ok(self.newObjectKind(self.intrinsics.stringProto, .StringBox(v))); }
        if v.isNumber { return .Ok(self.newObjectKind(self.intrinsics.numberProto, .NumberBox(v.asNumber))); }
        if v.isBool { return .Ok(self.newObjectKind(self.intrinsics.booleanProto, .BooleanBox(v.asBool))); }
        .Ok(self.newObjectKind(self.intrinsics.symbolProto, .SymbolBox(v)))
    }

    /// ToPropertyKey, as a key.
    mutating func toKey(v: Value) -> Result[Key, Value] {
        if v.isString { return .Ok(self.atomOfUnits(self.units(v))); }
        if v.isSymbol { return .Ok(symbolKey(v.asRef)); }
        if v.isNumber {
            let n = v.asNumber;
            if let .Some(i) = n.toInt64() {
                if Float64(from: i) == n and i >= 0 and not (n == 0.0 and n.isSignMinus) { return .Ok(self.internIndex(i)); }
            }
            return .Ok(self.atom(numberToString(n)));
        }
        let p = if v.isObject { try self.toPrimitive(v, "string") } else { v };
        if p.isSymbol or p.isNumber or p.isString { return self.toKey(p); }
        let u = try self.toUnits(p);
        .Ok(self.atomOfUnits(u))
    }

    /// A key as a value: a string or a symbol.
    mutating func keyValue(k: Key) -> Value {
        if keyIsSymbol(k) { return Value.symbol(symbolOfKey(k)); }
        self.atomValues(k)
    }

    mutating func typeOf(v: Value) -> String {
        if v.isNumber { return "number"; }
        if v.isString { return "string"; }
        if v.isBool { return "boolean"; }
        if v.isUndefined { return "undefined"; }
        if v.isNull { return "object"; }
        if v.isSymbol { return "symbol"; }
        if self.isCallable(v) { "function" } else { "object" }
    }

    // ------------------------------------------------------------------
    // Equality
    // ------------------------------------------------------------------

    func strictEquals(a: Value, b: Value) -> Bool {
        if a.isNumber and b.isNumber { return a.asNumber == b.asNumber; }
        if a.isString and b.isString { return unitsEqual(self.units(a), self.units(b)); }
        a.same(b)
    }

    /// SameValue: like `===` but NaN equals NaN and 0 differs from -0.
    func sameValue(a: Value, b: Value) -> Bool {
        if a.isNumber and b.isNumber {
            let x = a.asNumber;
            let y = b.asNumber;
            if x.isNaN and y.isNaN { return true; }
            return x.bitPattern == y.bitPattern or (x == y and x != 0.0);
        }
        self.strictEquals(a, b)
    }

    func sameValueZero(a: Value, b: Value) -> Bool {
        if a.isNumber and b.isNumber and a.asNumber.isNaN and b.asNumber.isNaN { return true; }
        self.strictEquals(a, b)
    }

    mutating func looseEquals(a: Value, b: Value) -> Result[Bool, Value] {
        if a.isNullish and b.isNullish { return .Ok(true); }
        if a.isNumber and b.isNumber { return .Ok(a.asNumber == b.asNumber); }
        if a.isString and b.isString { return .Ok(self.strictEquals(a, b)); }
        if a.isObject and b.isObject { return .Ok(a.same(b)); }
        if a.isBool and b.isBool { return .Ok(a.same(b)); }
        if a.isSymbol and b.isSymbol { return .Ok(a.same(b)); }
        if a.isNullish or b.isNullish { return .Ok(false); }
        if a.isNumber and b.isString { return .Ok(a.asNumber == stringToNumber(self.units(b))); }
        if a.isString and b.isNumber { return .Ok(stringToNumber(self.units(a)) == b.asNumber); }
        if a.isBool { return self.looseEquals(Value.number(if a.asBool { 1.0 } else { 0.0 }), b); }
        if b.isBool { return self.looseEquals(a, Value.number(if b.asBool { 1.0 } else { 0.0 })); }
        if b.isObject and not a.isObject {
            let p = try self.toPrimitive(b, "default");
            return self.looseEquals(a, p);
        }
        if a.isObject and not b.isObject {
            let p = try self.toPrimitive(a, "default");
            return self.looseEquals(p, b);
        }
        .Ok(false)
    }

    // ------------------------------------------------------------------
    // Operators
    // ------------------------------------------------------------------

    /// `a < b`, or `undefined` (as None) when either is NaN.
    mutating func lessThan(a: Value, b: Value, leftFirst: Bool) -> Result[Optional[Bool], Value] {
        var pa = a;
        var pb = b;
        if leftFirst {
            pa = try self.toPrimitive(a, "number");
            pb = try self.toPrimitive(b, "number");
        } else {
            pb = try self.toPrimitive(b, "number");
            pa = try self.toPrimitive(a, "number");
        }
        if pa.isString and pb.isString { return .Ok(.Some(unitsLess(self.units(pa), self.units(pb)))); }
        let x = try self.toNumber(pa);
        let y = try self.toNumber(pb);
        if x.isNaN or y.isNaN { return .Ok(.None); }
        .Ok(.Some(x < y))
    }

    mutating func binary(op: BinOp, a: Value, b: Value) -> Result[Value, Value] {
        match op {
            .Add => {
                if a.isNumber and b.isNumber { return .Ok(Value.number(a.asNumber + b.asNumber)); }
                let pa = try self.toPrimitive(a, "default");
                let pb = try self.toPrimitive(b, "default");
                if pa.isString or pb.isString {
                    var u = try self.toUnits(pa);
                    let v = try self.toUnits(pb);
                    for x in v { u.append(x); }
                    return .Ok(self.newString(u));
                }
                let x = try self.toNumber(pa);
                let y = try self.toNumber(pb);
                return .Ok(Value.number(x + y));
            },
            .StrictEq => return .Ok(Value.bool(self.strictEquals(a, b))),
            .StrictNotEq => return .Ok(Value.bool(not self.strictEquals(a, b))),
            .Eq => return .Ok(Value.bool(try self.looseEquals(a, b))),
            .NotEq => return .Ok(Value.bool(not (try self.looseEquals(a, b)))),
            .Lt => return .Ok(Value.bool((try self.lessThan(a, b, true)) ?? false)),
            .Gt => return .Ok(Value.bool((try self.lessThan(b, a, false)) ?? false)),
            .LtEq => {
                let r = try self.lessThan(b, a, false);
                return .Ok(Value.bool(match r { .Some(x) => not x, .None => false }));
            },
            .GtEq => {
                let r = try self.lessThan(a, b, true);
                return .Ok(Value.bool(match r { .Some(x) => not x, .None => false }));
            },
            .In => {
                if not b.isObject { return self.throwType("Cannot use 'in' operator to search for a key in a non-object"); }
                let k = try self.toKey(a);
                return .Ok(Value.bool(self.hasProperty(b, k)));
            },
            .InstanceOf => return .Ok(Value.bool(try self.instanceOf(a, b))),
            _ => {}
        }
        let x = try self.toNumber(a);
        let y = try self.toNumber(b);
        .Ok(Value.number(arithmetic(op, x, y)))
    }

    mutating func instanceOf(v: Value, target: Value) -> Result[Bool, Value] {
        if not target.isObject { return self.throwType("Right-hand side of 'instanceof' is not an object"); }
        let h = try self.getFrom(target, symbolKey(WellKnown.hasInstance), target);
        if not h.isNullish {
            let r = try self.callValue(h, target, [v]);
            return .Ok(self.toBoolean(r));
        }
        if not self.isCallable(target) { return self.throwType("Right-hand side of 'instanceof' is not callable"); }
        self.ordinaryHasInstance(target, v)
    }

    mutating func ordinaryHasInstance(c: Value, v: Value) -> Result[Bool, Value] {
        if not self.isCallable(c) { return .Ok(false); }
        if let .BoundFunction(target, _, _) = self.kindAt(c.asRef) {
            return self.instanceOf(v, target);
        }
        if not v.isObject { return .Ok(false); }
        let proto = try self.getFrom(c, self.names.prototype, c);
        if not proto.isObject { return self.throwType("Function has non-object prototype in instanceof check"); }
        var cur = self.heap.objects(v.asRef).proto;
        while cur.isObject {
            if cur.same(proto) { return .Ok(true); }
            cur = self.heap.objects(cur.asRef).proto;
        }
        .Ok(false)
    }

    mutating func unary(op: UnOp, v: Value) -> Result[Value, Value] {
        match op {
            .Not => return .Ok(Value.bool(not self.toBoolean(v))),
            .TypeOf => {
                let t = self.typeOf(v);
                return .Ok(self.atomValues(self.atom(t)));
            },
            .ToPropertyKey => {
                if v.isString or v.isSymbol { return .Ok(v); }
                let k = try self.toKey(v);
                return .Ok(self.keyValue(k));
            },
            .ToStr => return self.toStringValue(v),
            .ToObject => return self.toObject(v),
            .RequireObject => {
                if v.isNullish { return self.throwType("Cannot destructure '\(if v.isNull { "null" } else { "undefined" })'"); }
                return .Ok(v);
            },
            _ => {}
        }
        let n = try self.toNumber(v);
        let r = match op {
            .Neg => 0.0 - n,
            .BitNot => Float64(from: (toInt32(n) ^ -1)),
            .Inc => n + 1.0,
            .Dec => n - 1.0,
            _ => n
        };
        // -0 for negating 0.
        if op == UnOp.Neg and n == 0.0 {
            return .Ok(Value.number(if n.isSignMinus { 0.0 } else { -0.0 }));
        }
        .Ok(Value.number(r))
    }
}

/// The numeric binary operators.
func arithmetic(op: BinOp, x: Float64, y: Float64) -> Float64 {
    match op {
        .Sub => x - y,
        .Mul => x * y,
        .Div => x / y,
        .Mod => {
            if y.isInfinite and not x.isInfinite { x } else { fmod(x, y) }
        },
        .Exp => {
            if y.isNaN { Float64.nan }
            else if y == 0.0 { 1.0 }
            else if (x == 1.0 or x == -1.0) and y.isInfinite { Float64.nan }
            else { x.pow(y) }
        },
        .Shl => Float64(from: wrapInt32(toInt32(x) << (toUint32(y) & 31))),
        .Shr => Float64(from: toInt32(x) >> (toUint32(y) & 31)),
        .UShr => Float64(from: toUint32(x) >> (toUint32(y) & 31)),
        .BitAnd => Float64(from: toInt32(x) & toInt32(y)),
        .BitOr => Float64(from: toInt32(x) | toInt32(y)),
        .BitXor => Float64(from: toInt32(x) ^ toInt32(y)),
        _ => Float64.nan
    }
}

/// The low 32 bits of `n`, as a signed value.
func wrapInt32(n: Int64) -> Int64 {
    let m = n & 0xFFFFFFFF;
    if m >= 2147483648 { m - 4294967296 } else { m }
}

/// C's fmod (the sign follows `x`), built on IEEE remainder since Float64
/// has no `%`. Adjusting the remainder by |y| is exact.
func fmod(x: Float64, y: Float64) -> Float64 {
    let r = x.remainder(dividingBy: y);
    if r.isNaN { return r; }
    if r == 0.0 { return (0.0).copysign(from: x); }
    if r.isSignMinus == x.isSignMinus { return r; }
    r + y.abs().copysign(from: x)
}
