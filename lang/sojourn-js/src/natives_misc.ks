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
                self.str(toFixedExact(x, digits.toInt64() ?? 0))
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
                let hp37 = try self.hasProperty(a1, self.names.cause);
                if a1.isObject and hp37 {
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
            .JSONStringify => try self.jsonStringifyNative(a0, a1, arg(args, 2)),
            .JSONParse => try self.jsonParseNative(a0, a1),
            .ReflectOwnKeys => {
                if not a0.isObject { return self.throwType("Reflect.ownKeys called on non-object"); }
                var out: [Value] = [];
                let keys = try self.ownKeys(a0.asRef);
                for k in keys { out.append(self.keyValue(k)); }
                self.newArray(out)
            },
            .ReflectGetPrototypeOf => {
                if not a0.isObject { return self.throwType("Reflect.getPrototypeOf called on non-object"); }
                try self.getProto(a0)
            },
            .ReflectSetPrototypeOf => {
                if not a0.isObject { return self.throwType("Reflect.setPrototypeOf called on non-object"); }
                if not a1.isObject and not a1.isNull { return self.throwType("Object prototype may only be an Object or null"); }
                Value.bool(try self.setProtoV(a0, a1))
            },
            .ReflectHas => {
                if not a0.isObject { return self.throwType("Reflect.has called on non-object"); }
                let k = try self.toKey(a1);
                let hp38 = try self.hasProperty(a0, k);
                Value.bool(hp38)
            },
            .ReflectApply => {
                if not arg(args, 2).isObject { return self.throwType("CreateListFromArrayLike called on non-object"); }
                let list = try self.argumentList(arg(args, 2));
                try self.callValue(try self.requireCallable(a0), a1, list)
            },
            .ReflectConstruct => {
                if not self.isConstructor(a0) { return self.throwType("\(self.describe(a0)) is not a constructor"); }
                let nt = if args.count > 2 { arg(args, 2) } else { a0 };
                if not self.isConstructor(nt) { return self.throwType("\(self.describe(nt)) is not a constructor"); }
                if not a1.isObject { return self.throwType("CreateListFromArrayLike called on non-object"); }
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
                let hp39 = try self.deleteOwn(a0.asRef, k);
                Value.bool(hp39)
            },
            .ReflectGetOwnPropertyDescriptor => {
                if not a0.isObject { return self.throwType("Reflect.getOwnPropertyDescriptor called on non-object"); }
                let k = try self.toKey(a1);
                try self.ownDescriptorObject(a0, k)
            },
            .ReflectIsExtensible => {
                if not a0.isObject { return self.throwType("Reflect.isExtensible called on non-object"); }
                Value.bool(try self.isExtensibleV(a0))
            },
            .ReflectPreventExtensions => {
                if not a0.isObject { return self.throwType("Reflect.preventExtensions called on non-object"); }
                Value.bool(try self.preventExtensionsV(a0))
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

}

func jsonQuote(u: Units) -> Units {
    var out: Units = [34];
    var i: Int64 = 0;
    for c in u {
        let x = Int64(from: c);
        if x == 34 { out.append(92); out.append(34); }
        else if x == 92 { out.append(92); out.append(92); }
        else if x == 10 { out.append(92); out.append(110); }
        else if x == 13 { out.append(92); out.append(114); }
        else if x == 9 { out.append(92); out.append(116); }
        else if x == 8 { out.append(92); out.append(98); }
        else if x == 12 { out.append(92); out.append(102); }
        else if x < 32 or isLoneSurrogate(u, i) {
            out.append(92);
            out.append(117);
            let hex = unitsOf("0123456789abcdef");
            out.append(hex((x >> 12) & 15));
            out.append(hex((x >> 8) & 15));
            out.append(hex((x >> 4) & 15));
            out.append(hex(x & 15));
        } else { out.append(c); }
        i = i + 1;
    }
    out.append(34);
    out
}

/// Whether the unit at `i` is a surrogate without its other half.
func isLoneSurrogate(u: Units, i: Int64) -> Bool {
    let x = Int64(from: u(i));
    if x >= 0xD800 and x <= 0xDBFF {
        if i + 1 < u.count {
            let y = Int64(from: u(i + 1));
            if y >= 0xDC00 and y <= 0xDFFF { return false; }
        }
        return true;
    }
    if x >= 0xDC00 and x <= 0xDFFF {
        if i > 0 {
            let w = Int64(from: u(i - 1));
            if w >= 0xD800 and w <= 0xDBFF { return false; }
        }
        return true;
    }
    false
}

