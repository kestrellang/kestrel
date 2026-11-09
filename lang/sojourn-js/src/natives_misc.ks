module js

// Number, Boolean, Symbol, Error, RegExp, Math, JSON and Reflect.

extend VM {
    /// The number a Number method's `this` holds.
    mutating func thisNumber(this: Value) -> Result[Float64, Value] {
        if this.isNumber { return .Ok(this.asNumber); }
        if this.isObject {
            if let .NumberBox(n) = self.kindAt(this.asRef) { return .Ok(n); }
        }
        self.throwType("Number.prototype method called on incompatible receiver")
    }

    mutating func thisBool(this: Value) -> Result[Bool, Value] {
        if this.isBool { return .Ok(this.asBool); }
        if this.isObject {
            if let .BooleanBox(b) = self.kindAt(this.asRef) { return .Ok(b); }
        }
        self.throwType("Boolean.prototype method called on incompatible receiver")
    }

    mutating func thisSymbol(this: Value) -> Result[Value, Value] {
        if this.isSymbol { return .Ok(this); }
        if this.isObject {
            if let .SymbolBox(s) = self.kindAt(this.asRef) { return .Ok(s); }
        }
        self.throwType("Symbol.prototype method called on incompatible receiver")
    }

    mutating func mathArg(args: Array[Value], i: Int64) -> Result[Float64, Value] {
        self.toNumber(arg(args, i))
    }

    mutating func miscNative(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .NumberCtor => {
                let v = if args.count == 0 { 0.0 } else { try self.toNumber(a0) };
                if newTarget.isUndefined { return .Ok(.Some(Value.number(v))); }
                let proto = try self.prototypeFor(newTarget, self.intrinsics.numberProto);
                self.newObjectKind(proto, .NumberBox(v))
            },
            .NumberIsNaN => Value.bool(a0.isNumber and a0.asNumber.isNaN),
            .NumberIsFinite => Value.bool(a0.isNumber and a0.asNumber.isFinite),
            .NumberIsInteger => Value.bool(a0.isNumber and a0.asNumber.isFinite and a0.asNumber.trunc() == a0.asNumber),
            .NumberIsSafeInteger => Value.bool(a0.isNumber and a0.asNumber.isFinite and a0.asNumber.trunc() == a0.asNumber
                and a0.asNumber.abs() <= 9007199254740991.0),
            .NumberToString => {
                let x = try self.thisNumber(this);
                let radix = if a0.isUndefined { 10.0 } else { try self.toIntegerOrInfinity(a0) };
                if radix < 2.0 or radix > 36.0 { return .Err(self.makeError(.RangeError, "toString() radix must be between 2 and 36")); }
                let rd = radix.toInt64() ?? 10;
                self.str(if rd == 10 { numberToString(x) } else { numberToRadixString(x, rd) })
            },
            .NumberValueOf => Value.number(try self.thisNumber(this)),
            .NumberToFixed => {
                let x = try self.thisNumber(this);
                let digits = try self.toIntegerOrInfinity(a0);
                if digits < 0.0 or digits > 100.0 { return .Err(self.makeError(.RangeError, "toFixed() digits argument must be between 0 and 100")); }
                if not x.isFinite or x.abs() >= 1000000000000000000000.0 { return .Ok(.Some(self.str(numberToString(x)))); }
                self.str(toFixed(x, digits.toInt64() ?? 0))
            },
            .BooleanCtor => {
                let b = self.toBoolean(a0);
                if newTarget.isUndefined { return .Ok(.Some(Value.bool(b))); }
                let proto = try self.prototypeFor(newTarget, self.intrinsics.booleanProto);
                self.newObjectKind(proto, .BooleanBox(b))
            },
            .BooleanToString => self.str(if try self.thisBool(this) { "true" } else { "false" }),
            .BooleanValueOf => Value.bool(try self.thisBool(this)),
            .SymbolCtor => {
                if not newTarget.isUndefined { return self.throwType("Symbol is not a constructor"); }
                let d = if a0.isUndefined { Value.undefined } else { try self.toStringValue(a0) };
                self.newSymbol(d)
            },
            .SymbolFor => {
                let key = try self.toText(a0);
                if let .Some(id) = self.symbolRegistry(key) { return .Ok(.Some(Value.symbol(id))); }
                let d = self.str(key);
                let s = self.newSymbol(d);
                let _ = self.symbolRegistry.insert(key, s.asRef);
                s
            },
            .SymbolKeyFor => {
                if not a0.isSymbol { return self.throwType("\(self.describe(a0)) is not a symbol"); }
                var found = Value.undefined;
                for (k, id) in self.symbolRegistry.iter() {
                    if id == a0.asRef { found = self.str(k); }
                }
                found
            },
            .SymbolToString => {
                let s = try self.thisSymbol(this);
                self.str(self.keyText(symbolKey(s.asRef)))
            },
            .SymbolValueOf => try self.thisSymbol(this),
            .SymbolDescription => {
                let s = try self.thisSymbol(this);
                self.symbols(s.asRef)
            },
            .ErrorCtor or .TypeErrorCtor or .RangeErrorCtor or .ReferenceErrorCtor or .SyntaxErrorCtor or .EvalErrorCtor or .URIErrorCtor => {
                let fallback = match n {
                    .TypeErrorCtor => self.intrinsics.typeErrorProto,
                    .RangeErrorCtor => self.intrinsics.rangeErrorProto,
                    .ReferenceErrorCtor => self.intrinsics.referenceErrorProto,
                    .SyntaxErrorCtor => self.intrinsics.syntaxErrorProto,
                    .EvalErrorCtor => self.intrinsics.evalErrorProto,
                    .URIErrorCtor => self.intrinsics.uriErrorProto,
                    _ => self.intrinsics.errorProto
                };
                let nt = if newTarget.isUndefined { callee } else { newTarget };
                let proto = try self.prototypeFor(nt, fallback);
                let e = self.newObjectKind(proto, .ErrorObject);
                if not a0.isUndefined {
                    let m = try self.toStringValue(a0);
                    self.defineHidden(e.asRef, self.names.message, m);
                }
                if a1.isObject and self.hasProperty(a1, self.names.cause) {
                    let c = try self.getFrom(a1, self.names.cause, a1);
                    self.defineHidden(e.asRef, self.names.cause, c);
                }
                e
            },
            .ErrorToString => {
                if not this.isObject { return self.throwType("Error.prototype.toString called on non-object"); }
                let nv = try self.getFrom(this, self.names.name, this);
                let mv = try self.getFrom(this, self.names.message, this);
                let name = if nv.isUndefined { unitsOf("Error") } else { try self.toUnits(nv) };
                let msg = if mv.isUndefined { [] } else { try self.toUnits(mv) };
                if name.count == 0 { return .Ok(.Some(self.newString(msg))); }
                if msg.count == 0 { return .Ok(.Some(self.newString(name))); }
                var out = name;
                for u in unitsOf(": ") { out.append(u); }
                for u in msg { out.append(u); }
                self.newString(out)
            },
            .RegExpCtor => {
                let proto = try self.prototypeFor(if newTarget.isUndefined { callee } else { newTarget }, self.intrinsics.regexpProto);
                let o = self.newObjectKind(proto, .Ordinary);
                let src = if a0.isUndefined { self.str("(?:)") } else { try self.toStringValue(a0) };
                let flags = if a1.isUndefined { self.str("") } else { try self.toStringValue(a1) };
                self.defineOwn(o.asRef, self.atom("source"), src, Attr.configurable);
                self.defineOwn(o.asRef, self.atom("flags"), flags, Attr.configurable);
                self.defineOwn(o.asRef, self.names.lastIndex, Value.int(0), Attr.writable);
                o
            },
            .RegExpExec => return self.throwType("regular expressions aren't supported yet"),
            .RegExpToString => {
                let s = try self.toText(try self.getV(this, self.atom("source")));
                let f = try self.toText(try self.getV(this, self.atom("flags")));
                self.str("/\(s)/\(f)")
            },
            .MathAbs => Value.number((try self.mathArg(args, 0)).abs()),
            .MathFloor => Value.number((try self.mathArg(args, 0)).floor()),
            .MathCeil => Value.number((try self.mathArg(args, 0)).ceil()),
            .MathRound => {
                let x = try self.mathArg(args, 0);
                if not x.isFinite or x == 0.0 { return .Ok(.Some(Value.number(x))); }
                if x > 0.0 and x < 0.5 { return .Ok(.Some(Value.number(0.0))); }
                if x < 0.0 and x >= -0.5 { return .Ok(.Some(Value.number(-0.0))); }
                Value.number((x + 0.5).floor())
            },
            .MathTrunc => Value.number((try self.mathArg(args, 0)).trunc()),
            .MathSign => {
                let x = try self.mathArg(args, 0);
                Value.number(if x.isNaN or x == 0.0 { x } else if x > 0.0 { 1.0 } else { -1.0 })
            },
            .MathSqrt => Value.number((try self.mathArg(args, 0)).sqrt()),
            .MathCbrt => Value.number((try self.mathArg(args, 0)).cbrt()),
            .MathPow => {
                let x = try self.mathArg(args, 0);
                let y = try self.mathArg(args, 1);
                Value.number(arithmetic(.Exp, x, y))
            },
            .MathMin or .MathMax => {
                let isMax = n == NativeFn.MathMax;
                var best = if isMax { 0.0 - Float64.infinity } else { Float64.infinity };
                var nan = false;
                for a in args {
                    let x = try self.toNumber(a);
                    if x.isNaN { nan = true; continue; }
                    if isMax {
                        if x > best or (x == 0.0 and best == 0.0 and not x.isSignMinus) { best = x; }
                    } else {
                        if x < best or (x == 0.0 and best == 0.0 and x.isSignMinus) { best = x; }
                    }
                }
                Value.number(if nan { Float64.nan } else { best })
            },
            .MathRandom => {
                // xorshift64*
                var x = self.randomState;
                x = x ^ (x >> 12);
                x = x ^ (x << 25);
                x = x ^ (x >> 27);
                self.randomState = x;
                Value.number(Float64(from: Int64(bitPattern: (x >> 11))) / 9007199254740992.0)
            },
            .MathSin => Value.number((try self.mathArg(args, 0)).sin()),
            .MathCos => Value.number((try self.mathArg(args, 0)).cos()),
            .MathTan => Value.number((try self.mathArg(args, 0)).tan()),
            .MathAsin => Value.number((try self.mathArg(args, 0)).asin()),
            .MathAcos => Value.number((try self.mathArg(args, 0)).acos()),
            .MathAtan => Value.number((try self.mathArg(args, 0)).atan()),
            .MathAtan2 => {
                let y = try self.mathArg(args, 0);
                let x = try self.mathArg(args, 1);
                Value.number(y.atan2(x))
            },
            .MathExp => Value.number((try self.mathArg(args, 0)).exp()),
            .MathLog => Value.number((try self.mathArg(args, 0)).ln()),
            .MathLog2 => Value.number((try self.mathArg(args, 0)).log2()),
            .MathLog10 => Value.number((try self.mathArg(args, 0)).log10()),
            .MathHypot => {
                var sum = 0.0;
                var inf = false;
                var nan = false;
                for a in args {
                    let x = try self.toNumber(a);
                    if x.isInfinite { inf = true; }
                    if x.isNaN { nan = true; }
                    sum = sum + x * x;
                }
                Value.number(if inf { Float64.infinity } else if nan { Float64.nan } else { sum.sqrt() })
            },
            .JSONStringify => {
                let out = try self.jsonStringify(a0);
                match out {
                    .Some(u) => self.newString(u),
                    .None => Value.undefined
                }
            },
            .ReflectOwnKeys => {
                if not a0.isObject { return self.throwType("Reflect.ownKeys called on non-object"); }
                var out: [Value] = [];
                for k in self.ownKeys(a0.asRef) { out.append(self.keyValue(k)); }
                self.newArray(out)
            },
            .ReflectGetPrototypeOf => {
                if not a0.isObject { return self.throwType("Reflect.getPrototypeOf called on non-object"); }
                self.heap.objects(a0.asRef).proto
            },
            .ReflectSetPrototypeOf => {
                if not a0.isObject { return self.throwType("Reflect.setPrototypeOf called on non-object"); }
                if not a1.isObject and not a1.isNull { return self.throwType("Object prototype may only be an Object or null"); }
                Value.bool(try self.setPrototype(a0, a1))
            },
            .ReflectHas => {
                if not a0.isObject { return self.throwType("Reflect.has called on non-object"); }
                let k = try self.toKey(a1);
                Value.bool(self.hasProperty(a0, k))
            },
            .ReflectApply => {
                let list = try self.argumentList(arg(args, 2));
                try self.callValue(try self.requireCallable(a0), a1, list)
            },
            .ReflectConstruct => {
                if not self.isConstructor(a0) { return self.throwType("\(self.describe(a0)) is not a constructor"); }
                let nt = if args.count > 2 { arg(args, 2) } else { a0 };
                if not self.isConstructor(nt) { return self.throwType("\(self.describe(nt)) is not a constructor"); }
                let list = try self.argumentList(a1);
                try self.construct(a0, list, nt)
            },
            .ReflectGet => {
                if not a0.isObject { return self.throwType("Reflect.get called on non-object"); }
                let k = try self.toKey(a1);
                let receiver = if args.count > 2 { arg(args, 2) } else { a0 };
                try self.getFrom(a0, k, receiver)
            },
            .ReflectSet => {
                if not a0.isObject { return self.throwType("Reflect.set called on non-object"); }
                let k = try self.toKey(a1);
                let receiver = if args.count > 3 { arg(args, 3) } else { a0 };
                Value.bool(try self.ordinarySet(a0, k, arg(args, 2), receiver))
            },
            .ReflectDefineProperty => {
                if not a0.isObject { return self.throwType("Reflect.defineProperty called on non-object"); }
                let k = try self.toKey(a1);
                let d = try self.toDescriptor(arg(args, 2));
                Value.bool(try self.defineProperty(a0.asRef, k, d))
            },
            .ReflectDeleteProperty => {
                if not a0.isObject { return self.throwType("Reflect.deleteProperty called on non-object"); }
                let k = try self.toKey(a1);
                Value.bool(self.deleteOwn(a0.asRef, k))
            },
            .ReflectGetOwnPropertyDescriptor => {
                if not a0.isObject { return self.throwType("Reflect.getOwnPropertyDescriptor called on non-object"); }
                let k = try self.toKey(a1);
                self.fromProperty(a0.asRef, k)
            },
            .ReflectPreventExtensions => {
                if not a0.isObject { return self.throwType("Reflect.preventExtensions called on non-object"); }
                self.heap.objects(a0.asRef).extensible = false;
                Value.bool(true)
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    // ------------------------------------------------------------------
    // JSON.stringify (no replacer or indentation yet)
    // ------------------------------------------------------------------

    mutating func jsonStringify(v: Value) -> Result[Optional[Units], Value] {
        var value = v;
        if value.isObject {
            let toJSON = try self.getFrom(value, self.atom("toJSON"), value);
            if self.isCallable(toJSON) {
                let k = self.str("");
                value = try self.callValue(toJSON, value, [k]);
            }
        }
        if value.isObject {
            match self.kindAt(value.asRef) {
                .NumberBox(_) => value = Value.number(try self.toNumber(value)),
                .StringBox(s) => value = s,
                .BooleanBox(b) => value = Value.bool(b),
                _ => {}
            }
        }
        if value.isNull { return .Ok(.Some(unitsOf("null"))); }
        if value.isBool { return .Ok(.Some(unitsOf(if value.asBool { "true" } else { "false" }))); }
        if value.isNumber {
            let x = value.asNumber;
            return .Ok(.Some(unitsOf(if x.isFinite { numberToString(x) } else { "null" })));
        }
        if value.isString { return .Ok(.Some(jsonQuote(self.units(value)))); }
        if value.isUndefined or value.isSymbol or self.isCallable(value) { return .Ok(.None); }
        var out: Units = [];
        if self.isArray(value) {
            out.append(91);
            let len = try self.lengthOf(value);
            for i in 0..<len {
                if i > 0 { out.append(44); }
                let item = try self.getIndex(value, i);
                match try self.jsonStringify(item) {
                    .Some(u) => for x in u { out.append(x); },
                    .None => for x in unitsOf("null") { out.append(x); }
                }
            }
            out.append(93);
            return .Ok(.Some(out));
        }
        out.append(123);
        var first = true;
        for k in self.ownKeys(value.asRef) {
            if keyIsSymbol(k) { continue; }
            if let .Some(p) = self.ownProperty(value.asRef, k) {
                if p.attrs & Attr.enumerable == 0 { continue; }
                let item = try self.getFrom(value, k, value);
                if let .Some(u) = try self.jsonStringify(item) {
                    if not first { out.append(44); }
                    first = false;
                    for x in jsonQuote(unitsOf(self.atomNames(k))) { out.append(x); }
                    out.append(58);
                    for x in u { out.append(x); }
                }
            }
        }
        out.append(125);
        .Ok(.Some(out))
    }
}

func jsonQuote(u: Units) -> Units {
    var out: Units = [34];
    for c in u {
        let x = Int64(from: c);
        if x == 34 { out.append(92); out.append(34); }
        else if x == 92 { out.append(92); out.append(92); }
        else if x == 10 { out.append(92); out.append(110); }
        else if x == 13 { out.append(92); out.append(114); }
        else if x == 9 { out.append(92); out.append(116); }
        else if x == 8 { out.append(92); out.append(98); }
        else if x == 12 { out.append(92); out.append(102); }
        else if x < 32 {
            for y in unitsOf("\\u00") { out.append(y); }
            let hex = unitsOf("0123456789abcdef");
            out.append(hex(x >> 4));
            out.append(hex(x & 15));
        } else { out.append(c); }
    }
    out.append(34);
    out
}

/// Number.prototype.toFixed for |x| < 1e21.
func toFixed(x: Float64, digits: Int64) -> String {
    let negative = x < 0.0;
    let a = if negative { 0.0 - x } else { x };
    let scale = (10.0).powi(digits);
    let scaled = (a * scale).round();
    // The integer of `scaled`, as digits.
    var ds = "";
    var v = scaled;
    if v < 1.0 { ds = "0"; }
    var rev: [Char] = [];
    while v >= 1.0 {
        let q = (v / 10.0).floor();
        let d = (v - q * 10.0).toInt64() ?? 0;
        rev.append(Char(unchecked: UInt32(truncating: 48 + d)));
        v = q;
    }
    var i = rev.count - 1;
    while i >= 0 {
        ds.append(char: rev(i));
        i = i - 1;
    }
    var chars: [Char] = [];
    for c in ds.chars { chars.append(c); }
    while chars.count <= digits { chars.insert('0', at: 0); }
    var out = if negative and scaled != 0.0 { "-" } else { "" };
    let intLen = chars.count - digits;
    for j in 0..<intLen { out.append(char: chars(j)); }
    if digits > 0 {
        out.append(char: '.');
        for j in intLen..<chars.count { out.append(char: chars(j)); }
    }
    out
}
