module js

// Small built-ins from recent proposals: Promise.try, Error.isError,
// Math.sumPrecise and RegExp.escape. (Map/WeakMap upsert lives with the
// collections.)

extend VM {
    mutating func setupProposals() {
        let g = self.global;
        self.method(self.intrinsics.promiseCtor, "try", .PromiseTry, 1);
        let errorCtor = self.ownValue(g.asRef, self.atom("Error"));
        self.method(errorCtor, "isError", .ErrorIsError, 1);
        let math = self.ownValue(g.asRef, self.atom("Math"));
        self.method(math, "sumPrecise", .MathSumPrecise, 1);
        self.method(math, "f16round", .MathF16Round, 1);
        let regexpCtor = self.ownValue(g.asRef, self.atom("RegExp"));
        self.method(regexpCtor, "escape", .RegExpEscape, 1);
    }

    mutating func proposalNative(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let r: Value = match n {
            .PromiseTry => try self.promiseTry(this, args),
            .ErrorIsError => Value.bool(self.isErrorObject(a0)),
            .MathSumPrecise => try self.sumPrecise(a0),
            .MathF16Round => Value.number(float16ToFloat64(float64ToFloat16(try self.toNumber(a0)))),
            .RegExpEscape => try self.regexpEscape(a0),
            .SelfHostOp => try self.selfHostOp(args),
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// Promise.try(f, ...args): f's result as a promise from `c` (itself
    /// when it already is one), or a promise rejected with what f threw.
    mutating func promiseTry(c: Value, args: Array[Value]) -> Result[Value, Value] {
        if not c.isObject { return self.throwType("Promise.try called on a non-object"); }
        var rest: [Value] = [];
        if args.count > 1 { for i in 1..<args.count { rest.append(args(i)); } }
        let e = match self.callValue(arg(args, 0), Value.undefined, rest) {
            .Ok(v) => return self.promiseResolve(c, v),
            .Err(x) => x
        };
        self.pushRoot(e);
        let (p, _, rej) = try self.newPromiseCapability(c);
        self.pushRoot(p);
        let _ = try self.callValue(rej, Value.undefined, [e]);
        .Ok(p)
    }

    /// Whether `v` has an [[ErrorData]] slot (proxies don't).
    func isErrorObject(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .ErrorObject => true,
            _ => false
        }
    }

    /// Math.sumPrecise: the exact sum of an iterable of numbers, rounded
    /// once. Finite values add up exactly as integers in units of 2^-1074.
    mutating func sumPrecise(items: Value) -> Result[Value, Value] {
        if not items.isObject { return self.throwType("Math.sumPrecise requires an iterable object"); }
        let (it, next) = try self.getIterator(items);
        self.pushRoot(it);
        var sum = bnZero();
        var posInf = false;
        var negInf = false;
        var nan = false;
        var allNegZero = true;
        while true {
            let v = match try self.iteratorStep(it, next) {
                .Some(x) => x,
                .None => break
            };
            if not v.isNumber {
                let e = self.typeError("Math.sumPrecise: \(self.describe(v)) is not a number");
                return self.closeWith(it, e);
            }
            let x = v.asNumber;
            if x.isNaN { nan = true; continue; }
            if x == Float64.infinity { posInf = true; allNegZero = false; continue; }
            if x == 0.0 - Float64.infinity { negInf = true; allNegZero = false; continue; }
            if x == 0.0 {
                if x.bitPattern >> 63 == 0 { allNegZero = false; }
                continue;
            }
            allNegZero = false;
            sum = bnAdd(sum, scaledFloat(x));
        }
        self.popRoots(1);
        if nan or (posInf and negInf) { return .Ok(Value.number(Float64.nan)); }
        if posInf { return .Ok(Value.number(Float64.infinity)); }
        if negInf { return .Ok(Value.number(0.0 - Float64.infinity)); }
        if allNegZero { return .Ok(Value.number(Float64(bitPattern: 0x8000000000000000))); }
        .Ok(Value.number(scaledToFloat(sum)))
    }

    /// RegExp.escape(s): `s` with everything that could mean something
    /// in a pattern escaped.
    mutating func regexpEscape(s: Value) -> Result[Value, Value] {
        if not s.isString { return self.throwType("RegExp.escape requires a string"); }
        let u = self.units(s);
        var out: Units = [];
        var i: Int64 = 0;
        while i < u.count {
            var c = Int64(from: u(i));
            var width: Int64 = 1;
            if c >= 0xD800 and c <= 0xDBFF and i + 1 < u.count {
                let lo = Int64(from: u(i + 1));
                if lo >= 0xDC00 and lo <= 0xDFFF {
                    c = 0x10000 + ((c - 0xD800) << 10) + (lo - 0xDC00);
                    width = 2;
                }
            }
            let enc = if i == 0 and (isDecimalDigit(c) or isAsciiLetter(c)) { hexEscape(c) } else { encodeForRegExpEscape(c) };
            for x in enc { out.append(x); }
            i = i + width;
        }
        .Ok(self.newString(out))
    }
}

func isAsciiLetter(c: Int64) -> Bool { (c >= 65 and c <= 90) or (c >= 97 and c <= 122) }

/// EncodeForRegExpEscape for one code point.
func encodeForRegExpEscape(c: Int64) -> Units {
    if c < 128 and unitsIndexOf(unitsOf("^$\\.*+?()[]{}|/"), [UInt16(truncating: c)], 0) >= 0 {
        return [92, UInt16(truncating: c)];
    }
    let controls = unitsOf("tnvfr");
    if c >= 9 and c <= 13 { return [92, controls(c - 9)]; }
    let punct = c < 128 and unitsIndexOf(unitsOf(",-=<>#&!%:;@~'`\""), [UInt16(truncating: c)], 0) >= 0;
    if punct or isWhitespace(c) or isLineTerminator(c) or (c >= 0xD800 and c <= 0xDFFF) { return hexEscape(c); }
    if c < 0x10000 { return [UInt16(truncating: c)]; }
    let x = c - 0x10000;
    [UInt16(truncating: 0xD800 + (x >> 10)), UInt16(truncating: 0xDC00 + (x & 0x3FF))]
}

/// `\xhh` for a code point up to 0xFF, else `\uhhhh` per code unit.
func hexEscape(c: Int64) -> Units {
    let hex = unitsOf("0123456789abcdef");
    if c <= 0xFF { return [92, 120, hex((c >> 4) & 15), hex(c & 15)]; }
    var out: Units = [];
    var cus: [Int64] = [c];
    if c >= 0x10000 {
        let x = c - 0x10000;
        cus = [0xD800 + (x >> 10), 0xDC00 + (x & 0x3FF)];
    }
    for v in cus {
        for d in [92, 117, hex((v >> 12) & 15), hex((v >> 8) & 15), hex((v >> 4) & 15), hex(v & 15)] { out.append(d); }
    }
    out
}

/// A finite nonzero double as an exact integer count of 2^-1074.
func scaledFloat(x: Float64) -> BigNum {
    let bits = x.bitPattern;
    let neg = bits >> 63 == 1;
    let expField = Int64(bitPattern: (bits >> 52) & 0x7FF);
    var mant = bits & 0xFFFFFFFFFFFFF;
    var shift: Int64 = 0;
    if expField != 0 {
        mant = mant | 0x10000000000000;
        shift = expField - 1;
    }
    bnShl(bnFromU64(mant, neg), shift)
}

/// The double nearest `a` × 2^-1074 (ties to even), ±Infinity past the
/// largest finite one.
func scaledToFloat(a: BigNum) -> Float64 {
    let len = magBitLength(a.mag);
    if len == 0 { return 0.0; }
    // Keep 53 significant bits; below 2^-1021 every bit fits.
    let s = if len > 53 { len - 53 } else { 0 };
    var q = magShr(a.mag, s);
    if s > 0 {
        let half = magShl(oneMag(), s - 1);
        let rem = magSub(a.mag, magShl(q, s));
        let c = magCmp(rem, half);
        if c > 0 or (c == 0 and (limb(q, 0) & 1) == 1) { q = magAdd(q, oneMag()); }
    }
    let e = s - 1074;
    if e + magBitLength(q) > 1024 { return if a.neg { 0.0 - Float64.infinity } else { Float64.infinity }; }
    let x = scalePow2(u64ToFloat((limb(q, 1) << 32) | limb(q, 0)), e);
    if a.neg { 0.0 - x } else { x }
}

/// x × 2^e, in steps that stay exact while the result is representable.
func scalePow2(x: Float64, e: Int64) -> Float64 {
    var r = x;
    var k = e;
    while k > 1000 {
        r = r * pow2(1000);
        k = k - 1000;
    }
    while k < 0 - 1000 {
        r = r * pow2(0 - 1000);
        k = k + 1000;
    }
    r * pow2(k)
}

/// 2^k for a normal exponent k.
func pow2(k: Int64) -> Float64 { Float64(bitPattern: UInt64(bitPattern: k + 1023) << 52) }

// ----------------------------------------------------------------------
// Self-hosted built-ins
// ----------------------------------------------------------------------
//
// Array.fromAsync is an async function, so it's written in JS: the source
// below evaluates to a maker that takes `op`, a native giving the code
// spec operations user code can't intercept (see `selfHostOp`).

func fromAsyncSource() -> String {
    "\"use strict\";\n" +
    "(function (op) {\n" +
    "  async function closeAsync(iter) {\n" +
    "    try {\n" +
    "      var ret = iter.return;\n" +
    "      if (ret !== undefined && ret !== null) await op(6, ret, iter, []);\n" +
    "    } catch (_) {}\n" +
    "  }\n" +
    "  return async function fromAsync(asyncItems, mapfn = undefined, thisArg = undefined) {\n" +
    "    var C = this;\n" +
    "    var mapping = mapfn !== undefined;\n" +
    "    if (mapping && !op(1, mapfn)) throw op(10, \"Array.fromAsync: the mapper is not a function\");\n" +
    "    var rec = op(5, asyncItems);\n" +
    "    var A, v, k = 0;\n" +
    "    if (rec !== undefined) {\n" +
    "      var iter = rec[0], next = rec[1];\n" +
    "      A = op(0, C) ? op(2, C) : op(7, 0);\n" +
    "      while (true) {\n" +
    "        if (k >= 9007199254740991) {\n" +
    "          var err = op(10, \"Array.fromAsync: too many elements\");\n" +
    "          await closeAsync(iter);\n" +
    "          throw err;\n" +
    "        }\n" +
    "        var r = await op(6, next, iter, []);\n" +
    "        if (!op(11, r)) throw op(10, \"Array.fromAsync: the iterator result is not an object\");\n" +
    "        if (r.done) {\n" +
    "          op(8, A, k);\n" +
    "          return A;\n" +
    "        }\n" +
    "        v = r.value;\n" +
    "        if (mapping) {\n" +
    "          try {\n" +
    "            v = await op(6, mapfn, thisArg, [v, k]);\n" +
    "          } catch (e) {\n" +
    "            await closeAsync(iter);\n" +
    "            throw e;\n" +
    "          }\n" +
    "        }\n" +
    "        try {\n" +
    "          op(3, A, k, v);\n" +
    "        } catch (e) {\n" +
    "          await closeAsync(iter);\n" +
    "          throw e;\n" +
    "        }\n" +
    "        k++;\n" +
    "      }\n" +
    "    }\n" +
    "    var arrayLike = op(9, asyncItems);\n" +
    "    var len = op(4, arrayLike);\n" +
    "    A = op(0, C) ? op(2, C, len) : op(7, len);\n" +
    "    while (k < len) {\n" +
    "      v = await arrayLike[k];\n" +
    "      if (mapping) v = await op(6, mapfn, thisArg, [v, k]);\n" +
    "      op(3, A, k, v);\n" +
    "      k++;\n" +
    "    }\n" +
    "    op(8, A, len);\n" +
    "    return A;\n" +
    "  };\n" +
    "})\n"
}

extend VM {
    /// Compiles the self-hosted built-ins and installs them.
    mutating func setupSelfHosted() {
        let maker = match self.runScript(fromAsyncSource(), "builtin") {
            .Ok(v) => v,
            .Err(_) => return
        };
        self.pushRoot(maker);
        let op = self.nativeFunction(.SelfHostOp, "", 0, false);
        self.pushRoot(op);
        let f = match self.callValue(maker, Value.undefined, [op]) {
            .Ok(v) => v,
            .Err(_) => return
        };
        self.popRoots(2);
        // Built-ins inherit from Function.prototype, async or not.
        self.heap.objects(f.asRef).proto = self.intrinsics.functionProto;
        let arrayCtor = self.ownValue(self.global.asRef, self.atom("Array"));
        self.defineHidden(arrayCtor.asRef, self.atom("fromAsync"), f);
    }

    /// The spec operations self-hosted code calls, by number.
    mutating func selfHostOp(args: Array[Value]) -> Result[Value, Value] {
        let a1 = arg(args, 1);
        let a2 = arg(args, 2);
        let which = arg(args, 0).asNumber.toInt64() ?? -1;
        match which {
            0 => .Ok(Value.bool(self.isConstructor(a1))),
            1 => .Ok(Value.bool(self.isCallable(a1))),
            2 => if args.count > 2 { self.construct(a1, [a2], a1) } else { self.construct(a1, [], a1) },
            3 => {
                let k = try self.toKey(a2);
                try self.createIndexOrThrowKey(a1, k, arg(args, 3));
                .Ok(Value.undefined)
            },
            4 => .Ok(Value.number(Float64(from: try self.lengthOf(a1)))),
            5 => self.fromAsyncIterator(a1),
            6 => self.callValue(a1, a2, self.arrayElements(arg(args, 3))),
            7 => self.arrayCreate(a1.asNumber.toInt64() ?? 0),
            8 => {
                try self.setV(a1, self.names.length, a2, true);
                .Ok(Value.undefined)
            },
            9 => self.toObject(a1),
            10 => .Ok(self.typeError(try self.toText(a1))),
            _ => .Ok(Value.bool(a1.isObject))
        }
    }

    /// Array.fromAsync's iterator: [iterator, next] from @@asyncIterator,
    /// else from @@iterator wrapped by CreateAsyncFromSyncIterator, else
    /// undefined (an array-like).
    mutating func fromAsyncIterator(items: Value) -> Result[Value, Value] {
        let am = try self.getMethod(items, symbolKey(WellKnown.asyncIterator));
        if not am.isUndefined {
            let it = try self.callValue(am, items, []);
            if not it.isObject { return self.throwType("Result of the Symbol.asyncIterator method is not an object"); }
            let next = try self.getV(it, self.names.next);
            return .Ok(self.newArray([it, next]));
        }
        let sm = try self.getMethod(items, symbolKey(WellKnown.iterator));
        if sm.isUndefined { return .Ok(Value.undefined); }
        let it = try self.callValue(sm, items, []);
        if not it.isObject { return self.throwType("Result of the Symbol.iterator method is not an object"); }
        let next = try self.getV(it, self.names.next);
        let w = self.newObjectKind(self.intrinsics.asyncFromSyncIteratorProto, .AsyncFromSyncIterator(iter: it, next: next));
        let wnext = self.ownValue(self.intrinsics.asyncFromSyncIteratorProto.asRef, self.names.next);
        .Ok(self.newArray([w, wnext]))
    }
}
