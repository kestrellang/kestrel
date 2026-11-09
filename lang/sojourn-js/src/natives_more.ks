module js

// More built-ins: the rest of Math, Number's digit formatting, string
// replacement and the HTML methods, the change-by-copy array methods,
// Object.groupBy and the legacy accessor methods, and the URI functions.

extend VM {
    mutating func setupMore() {
        let g = self.global;
        let math = self.ownValue(g.asRef, self.atom("Math"));
        for (name, n, len) in [
            ("acosh", NativeFn.MathAcosh, 1), ("asinh", NativeFn.MathAsinh, 1), ("atanh", NativeFn.MathAtanh, 1),
            ("cosh", NativeFn.MathCosh, 1), ("sinh", NativeFn.MathSinh, 1), ("tanh", NativeFn.MathTanh, 1),
            ("expm1", NativeFn.MathExpm1, 1), ("log1p", NativeFn.MathLog1p, 1), ("clz32", NativeFn.MathClz32, 1),
            ("imul", NativeFn.MathImul, 2), ("fround", NativeFn.MathFround, 1)
        ] {
            self.method(math, name, n, len);
        }
        let numberProto = self.intrinsics.numberProto;
        self.method(numberProto, "toExponential", .NumberToExponential, 1);
        self.method(numberProto, "toPrecision", .NumberToPrecision, 1);

        let sp = self.intrinsics.stringProto;
        for (name, n, len) in [
            ("replace", NativeFn.StringReplace, 2), ("replaceAll", NativeFn.StringReplaceAll, 2),
            ("isWellFormed", NativeFn.StringIsWellFormed, 0), ("toWellFormed", NativeFn.StringToWellFormed, 0),
            ("anchor", NativeFn.StringAnchor, 1), ("big", NativeFn.StringBig, 0), ("blink", NativeFn.StringBlink, 0),
            ("bold", NativeFn.StringBold, 0), ("fixed", NativeFn.StringFixed, 0), ("fontcolor", NativeFn.StringFontcolor, 1),
            ("fontsize", NativeFn.StringFontsize, 1), ("italics", NativeFn.StringItalics, 0), ("link", NativeFn.StringLink, 1),
            ("small", NativeFn.StringSmall, 0), ("strike", NativeFn.StringStrike, 0), ("sub", NativeFn.StringSub, 0),
            ("sup", NativeFn.StringSup, 0)
        ] {
            self.method(sp, name, n, len);
        }
        // trimLeft and trimRight are the same functions as trimStart and trimEnd.
        let trimStart = self.ownValue(sp.asRef, self.atom("trimStart"));
        let trimEnd = self.ownValue(sp.asRef, self.atom("trimEnd"));
        self.defineHidden(sp.asRef, self.atom("trimLeft"), trimStart);
        self.defineHidden(sp.asRef, self.atom("trimRight"), trimEnd);
        let stringCtor = self.ownValue(g.asRef, self.atom("String"));
        self.method(stringCtor, "raw", .StringRaw, 1);

        let ap = self.intrinsics.arrayProto;
        for (name, n, len) in [
            ("copyWithin", NativeFn.ArrayCopyWithin, 2), ("flatMap", NativeFn.ArrayFlatMap, 1),
            ("toReversed", NativeFn.ArrayToReversed, 0), ("toSorted", NativeFn.ArrayToSorted, 1),
            ("toSpliced", NativeFn.ArrayToSpliced, 2), ("with", NativeFn.ArrayWith, 2)
        ] {
            self.method(ap, name, n, len);
        }

        let op = self.intrinsics.objectProto;
        self.method(op, "__defineGetter__", .ObjectDefineGetter, 2);
        self.method(op, "__defineSetter__", .ObjectDefineSetter, 2);
        self.method(op, "__lookupGetter__", .ObjectLookupGetter, 1);
        self.method(op, "__lookupSetter__", .ObjectLookupSetter, 1);
        self.setupCollections();
        self.setupDate();
        self.setupGenerators();
        self.setupRegExp();
        self.setupPromise();
        self.setupIterator();
        self.setupSetMethods();
        self.setupTypedArrays();
        self.setupProxy();
        self.setupBigInt();
        self.setupProposals();
        self.setupBase64();
        self.setupIteratorProposals();
        let objectCtor = self.ownValue(g.asRef, self.atom("Object"));
        self.method(objectCtor, "groupBy", .ObjectGroupBy, 2);

        for (name, n) in [
            ("encodeURI", NativeFn.EncodeURI), ("encodeURIComponent", NativeFn.EncodeURIComponent),
            ("decodeURI", NativeFn.DecodeURI), ("decodeURIComponent", NativeFn.DecodeURIComponent),
            ("escape", NativeFn.Escape), ("unescape", NativeFn.Unescape)
        ] {
            self.method(g, name, n, 1);
        }
    }

    mutating func moreNative(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            // Math
            .MathAcosh => Value.number((try self.mathArg(args, 0)).acosh()),
            .MathAsinh => {
                let x = try self.mathArg(args, 0);
                // Keep the sign of -0.
                Value.number(if x == 0.0 { x } else { x.asinh() })
            },
            .MathAtanh => Value.number((try self.mathArg(args, 0)).atanh()),
            .MathCosh => Value.number((try self.mathArg(args, 0)).cosh()),
            .MathSinh => Value.number((try self.mathArg(args, 0)).sinh()),
            .MathTanh => Value.number((try self.mathArg(args, 0)).tanh()),
            .MathExpm1 => Value.number((try self.mathArg(args, 0)).expm1()),
            .MathLog1p => Value.number((try self.mathArg(args, 0)).ln1p()),
            .MathClz32 => {
                var x = toUint32(try self.mathArg(args, 0));
                var zeros: Int64 = 32;
                while x > 0 {
                    zeros = zeros - 1;
                    x = x >> 1;
                }
                Value.int(zeros)
            },
            .MathImul => {
                let a = toInt32(try self.mathArg(args, 0));
                let b = toInt32(try self.mathArg(args, 1));
                Value.int(wrapInt32(a * b))
            },
            .MathFround => Value.number(Float64(from: (try self.mathArg(args, 0)).toFloat32())),

            // Number
            .NumberToExponential => {
                let x = try self.thisNumber(this);
                let f = try self.toIntegerOrInfinity(a0);
                if not x.isFinite { return .Ok(.Some(self.str(numberToString(x)))); }
                if f < 0.0 or f > 100.0 { return .Err(self.makeError(.RangeError, "toExponential() argument must be between 0 and 100")); }
                self.str(toExponential(x, if a0.isUndefined { -1 } else { f.toInt64() ?? 0 }))
            },
            .NumberToPrecision => {
                let x = try self.thisNumber(this);
                if a0.isUndefined { return .Ok(.Some(self.str(numberToString(x)))); }
                let p = try self.toIntegerOrInfinity(a0);
                if not x.isFinite { return .Ok(.Some(self.str(numberToString(x)))); }
                if p < 1.0 or p > 100.0 { return .Err(self.makeError(.RangeError, "toPrecision() argument must be between 1 and 100")); }
                self.str(toPrecision(x, p.toInt64() ?? 1))
            },

            // String
            .StringReplace => try self.stringReplace(this, a0, a1, false),
            .StringReplaceAll => try self.stringReplace(this, a0, a1, true),
            .StringIsWellFormed => {
                let s = try self.thisUnits(this, "isWellFormed");
                var ok = true;
                for i in 0..<s.count {
                    if isLoneSurrogate(s, i) {
                        ok = false;
                        break;
                    }
                }
                Value.bool(ok)
            },
            .StringToWellFormed => {
                var s = try self.thisUnits(this, "toWellFormed");
                for i in 0..<s.count {
                    if isLoneSurrogate(s, i) { s(i) = 0xFFFD; }
                }
                self.newString(s)
            },
            .StringAnchor => try self.createHTML(this, "anchor", "a", "name", a0),
            .StringBig => try self.createHTML(this, "big", "big", "", a0),
            .StringBlink => try self.createHTML(this, "blink", "blink", "", a0),
            .StringBold => try self.createHTML(this, "bold", "b", "", a0),
            .StringFixed => try self.createHTML(this, "fixed", "tt", "", a0),
            .StringFontcolor => try self.createHTML(this, "fontcolor", "font", "color", a0),
            .StringFontsize => try self.createHTML(this, "fontsize", "font", "size", a0),
            .StringItalics => try self.createHTML(this, "italics", "i", "", a0),
            .StringLink => try self.createHTML(this, "link", "a", "href", a0),
            .StringSmall => try self.createHTML(this, "small", "small", "", a0),
            .StringStrike => try self.createHTML(this, "strike", "strike", "", a0),
            .StringSub => try self.createHTML(this, "sub", "sub", "", a0),
            .StringSup => try self.createHTML(this, "sup", "sup", "", a0),
            .StringRaw => {
                let cooked = try self.toObject(a0);
                let raw = try self.toObject(try self.getV(cooked, self.names.raw));
                let len = try self.lengthOf(raw);
                var out: Units = [];
                for i in 0..<len {
                    for c in try self.toUnits(try self.getIndex(raw, i)) { out.append(c); }
                    if i + 1 < len and i + 1 < args.count {
                        for c in try self.toUnits(args(i + 1)) { out.append(c); }
                    }
                }
                self.newString(out)
            },

            // Array
            .ArrayCopyWithin => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                var to = relativeIndex(try self.toIntegerOrInfinity(a0), len);
                var from = relativeIndex(try self.toIntegerOrInfinity(a1), len);
                let e = arg(args, 2);
                let end = if e.isUndefined { len } else { relativeIndex(try self.toIntegerOrInfinity(e), len) };
                var count = if end - from < len - to { end - from } else { len - to };
                var step: Int64 = 1;
                if from < to and to < from + count {
                    step = -1;
                    from = from + count - 1;
                    to = to + count - 1;
                }
                while count > 0 {
                    let hp1 = try self.hasIndex(o, from);
                    if hp1 {
                        try self.setIndex(o, to, try self.getIndex(o, from));
                    } else {
                        let k = self.internIndex(to);
                        let hp2 = try self.deleteOwn(o.asRef, k);
                        if not hp2 { return self.throwType("Cannot delete property"); }
                    }
                    from = from + step;
                    to = to + step;
                    count = count - 1;
                }
                o
            },
            .ArrayToReversed => {
                let o = try self.toObject(this);
                let len = try self.checkedLength(o);
                var out: [Value] = [];
                var i = len;
                while i > 0 {
                    i = i - 1;
                    out.append(try self.getIndex(o, i));
                }
                self.newArray(out)
            },
            .ArrayToSorted => {
                if not a0.isUndefined { let _ = try self.requireCallable(a0); }
                let o = try self.toObject(this);
                let len = try self.checkedLength(o);
                var items: [Value] = [];
                for i in 0..<len { items.append(try self.getIndex(o, i)); }
                let holder = self.newArray(items);
                self.pushRoot(holder);
                let sorted = try self.mergeSort(items, a0);
                self.newArray(sorted)
            },
            .ArrayToSpliced => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let start = relativeIndex(try self.toIntegerOrInfinity(a0), len);
                var skip: Int64 = 0;
                if args.count == 1 {
                    skip = len - start;
                } else if args.count > 1 {
                    let dc = try self.toIntegerOrInfinity(a1);
                    let maxDc = Float64(from: len - start);
                    let clamped = if dc < 0.0 { 0.0 } else if dc > maxDc { maxDc } else { dc };
                    skip = clamped.toInt64() ?? 0;
                }
                let inserted = if args.count > 2 { args.count - 2 } else { 0 };
                let newLen = len + inserted - skip;
                if newLen > 9007199254740991 { return self.throwType("Invalid array length"); }
                if newLen > 4294967295 { return .Err(self.makeError(.RangeError, "Invalid array length")); }
                var out: [Value] = [];
                for i in 0..<start { out.append(try self.getIndex(o, i)); }
                for i in 2..<args.count { out.append(args(i)); }
                for i in (start + skip)..<len { out.append(try self.getIndex(o, i)); }
                self.newArray(out)
            },
            .ArrayWith => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let rel = try self.toIntegerOrInfinity(a0);
                let actual = if rel >= 0.0 { rel } else { Float64(from: len) + rel };
                if actual >= Float64(from: len) or actual < 0.0 { return .Err(self.makeError(.RangeError, "Invalid index")); }
                if len > 4294967295 { return .Err(self.makeError(.RangeError, "Invalid array length")); }
                let at = actual.toInt64() ?? 0;
                var out: [Value] = [];
                for i in 0..<len { out.append(if i == at { a1 } else { try self.getIndex(o, i) }); }
                self.newArray(out)
            },

            // Object
            .ObjectDefineGetter or .ObjectDefineSetter => {
                let o = try self.toObject(this);
                if not self.isCallable(a1) { return self.throwType("accessor must be a function"); }
                let key = try self.toKey(a0);
                let isGetter = n == NativeFn.ObjectDefineGetter;
                let d = Descriptor(value: .None, writable: .None,
                                   getter: if isGetter { .Some(a1) } else { .None },
                                   setter: if isGetter { .None } else { .Some(a1) },
                                   enumerable: .Some(true), configurable: .Some(true));
                if not (try self.defineProperty(o.asRef, key, d)) { return self.throwType("Cannot redefine property"); }
                Value.undefined
            },
            .ObjectLookupGetter or .ObjectLookupSetter => {
                var o = try self.toObject(this);
                let key = try self.toKey(a0);
                let isGetter = n == NativeFn.ObjectLookupGetter;
                var found = Value.undefined;
                while o.isObject {
                    let desc = try self.getOwnPropertyV(o, key);
                    if let .Some(p) = desc {
                        if isAccessorDescriptor(p) {
                            found = if isGetter { p.getter ?? Value.undefined } else { p.setter ?? Value.undefined };
                        }
                        break;
                    }
                    o = try self.getProto(o);
                }
                found
            },
            .ObjectGroupBy => {
                let f = try self.requireCallable(a1);
                if a0.isNullish { return self.throwType("groupBy called on null or undefined"); }
                let out = self.newObjectKind(Value.nullValue, .Ordinary);
                self.pushRoot(out);
                let (it, next) = try self.getIterator(a0);
                self.pushRoot(it);
                var k: Int64 = 0;
                while true {
                    let step = try self.iteratorStep(it, next);
                    let v = match step {
                        .Some(x) => x,
                        .None => break
                    };
                    let keyed = match self.callValue(f, Value.undefined, [v, Value.int(k)]) {
                        .Ok(x) => x,
                        .Err(e) => {
                            let _ = self.iteratorClose(it);
                            return .Err(e);
                        }
                    };
                    let key = match self.toKey(keyed) {
                        .Ok(x) => x,
                        .Err(e) => {
                            let _ = self.iteratorClose(it);
                            return .Err(e);
                        }
                    };
                    var group = self.ownValue(out.asRef, key);
                    if group.isUndefined {
                        group = self.newArray([]);
                        self.defineOwn(out.asRef, key, group, Attr.all);
                    }
                    self.appendElement(group.asRef, v);
                    k = k + 1;
                }
                out
            },

            // URI functions
            .EncodeURI => try self.encodeURI(a0, ";/?:@&=+$,#"),
            .EncodeURIComponent => try self.encodeURI(a0, ""),
            .DecodeURI => try self.decodeURI(a0, ";/?:@&=+$,#"),
            .DecodeURIComponent => try self.decodeURI(a0, ""),
            .Escape => {
                let s = try self.toUnits(a0);
                let keep = unitsOf("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789@*_+-./");
                var out: Units = [];
                for c in s {
                    if keep.contains(c) {
                        out.append(c);
                    } else if Int64(from: c) < 256 {
                        for x in percentHex(Int64(from: c)) { out.append(x); }
                    } else {
                        out.append(37);
                        out.append(117);
                        for x in hex4(Int64(from: c)) { out.append(x); }
                    }
                }
                self.newString(out)
            },
            .Unescape => {
                let s = try self.toUnits(a0);
                var out: Units = [];
                var i: Int64 = 0;
                while i < s.count {
                    let c = s(i);
                    if Int64(from: c) == 37 {
                        if i + 6 <= s.count and Int64(from: s(i + 1)) == 117 {
                            let v = hexValue(s, i + 2, 4);
                            if v >= 0 {
                                out.append(UInt16(truncating: v));
                                i = i + 6;
                                continue;
                            }
                        }
                        if i + 3 <= s.count {
                            let v = hexValue(s, i + 1, 2);
                            if v >= 0 {
                                out.append(UInt16(truncating: v));
                                i = i + 3;
                                continue;
                            }
                        }
                    }
                    out.append(c);
                    i = i + 1;
                }
                self.newString(out)
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// LengthOfArrayLike, then the ArrayCreate length check.
    mutating func checkedLength(o: Value) -> Result[Int64, Value] {
        let len = try self.lengthOf(o);
        if len > 4294967295 { return .Err(self.makeError(.RangeError, "Invalid array length")); }
        .Ok(len)
    }

    /// CreateHTML for the String.prototype HTML methods.
    mutating func createHTML(this: Value, name: String, tag: String, attribute: String, value: Value) -> Result[Value, Value] {
        let s = try self.thisUnits(this, name);
        var out = unitsOf("<" + tag);
        if not attribute.isEmpty {
            let v = try self.toUnits(value);
            for c in unitsOf(" " + attribute + "=\"") { out.append(c); }
            for c in v {
                if Int64(from: c) == 34 {
                    for x in unitsOf("&quot;") { out.append(x); }
                } else {
                    out.append(c);
                }
            }
            out.append(34);
        }
        out.append(62);
        for c in s { out.append(c); }
        for c in unitsOf("</" + tag + ">") { out.append(c); }
        .Ok(self.newString(out))
    }

    /// String.prototype.replace and replaceAll with a string pattern.
    mutating func stringReplace(this: Value, search: Value, replace: Value, all: Bool) -> Result[Value, Value] {
        if this.isNullish { return self.throwType("String.prototype.replace called on null or undefined"); }
        if search.isObject {
            if all {
                let isRx = try self.isRegExp(search);
                if isRx {
                    let flags = try self.getV(search, self.atom("flags"));
                    if flags.isNullish { return self.throwType("flags is null or undefined"); }
                    if not (try self.toUnits(flags)).contains(103) {
                        return self.throwType("replaceAll must be called with a global RegExp");
                    }
                }
            }
            let m = try self.getMethod(search, symbolKey(WellKnown.replace));
            if not m.isUndefined { return self.callValue(m, search, [this, replace]); }
        }
        let s = try self.toUnits(this);
        let needle = try self.toUnits(search);
        let functional = self.isCallable(replace);
        let template: Units = if functional { [] } else { try self.toUnits(replace) };
        var positions: [Int64] = [];
        let advance = if needle.count > 0 { needle.count } else { 1 };
        var pos = unitsIndexOf(s, needle, 0);
        while pos >= 0 {
            positions.append(pos);
            if not all { break; }
            pos = if pos + advance > s.count { -1 } else { unitsIndexOf(s, needle, pos + advance) };
        }
        var out: Units = [];
        var end: Int64 = 0;
        for p in positions {
            for i in end..<p { out.append(s(i)); }
            if functional {
                let matched = self.newString(needle);
                let whole = self.newString(s);
                let v = try self.callValue(replace, Value.undefined, [matched, Value.int(p), whole]);
                for c in try self.toUnits(v) { out.append(c); }
            } else {
                for c in substitution(needle, s, p, template) { out.append(c); }
            }
            end = p + needle.count;
        }
        for i in end..<s.count { out.append(s(i)); }
        .Ok(self.newString(out))
    }

    /// Encode: UTF-8 percent-escapes everything but the unreserved marks
    /// and `extra`.
    mutating func encodeURI(v: Value, extra: String) -> Result[Value, Value] {
        let s = try self.toUnits(v);
        let keep = unitsOf("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()" + extra);
        var out: Units = [];
        var i: Int64 = 0;
        while i < s.count {
            let c = Int64(from: s(i));
            if keep.contains(s(i)) {
                out.append(s(i));
                i = i + 1;
                continue;
            }
            var cp = c;
            if c >= 0xDC00 and c <= 0xDFFF { return .Err(self.makeError(.URIError, "URI malformed")); }
            if c >= 0xD800 and c <= 0xDBFF {
                if i + 1 >= s.count { return .Err(self.makeError(.URIError, "URI malformed")); }
                let d = Int64(from: s(i + 1));
                if d < 0xDC00 or d > 0xDFFF { return .Err(self.makeError(.URIError, "URI malformed")); }
                cp = 0x10000 + ((c - 0xD800) << 10) + (d - 0xDC00);
                i = i + 1;
            }
            for b in utf8Bytes(cp) {
                for x in percentHex(b) { out.append(x); }
            }
            i = i + 1;
        }
        .Ok(self.newString(out))
    }

    /// Decode: undoes percent-escapes, except ones for characters in
    /// `reserved`.
    mutating func decodeURI(v: Value, reserved: String) -> Result[Value, Value] {
        let s = try self.toUnits(v);
        let keep = unitsOf(reserved);
        var out: Units = [];
        var i: Int64 = 0;
        while i < s.count {
            if Int64(from: s(i)) != 37 {
                out.append(s(i));
                i = i + 1;
                continue;
            }
            let start = i;
            if i + 3 > s.count { return .Err(self.makeError(.URIError, "URI malformed")); }
            let b = hexValue(s, i + 1, 2);
            if b < 0 { return .Err(self.makeError(.URIError, "URI malformed")); }
            i = i + 3;
            if b < 0x80 {
                let u = UInt16(truncating: b);
                if keep.contains(u) {
                    for j in start..<i { out.append(s(j)); }
                } else {
                    out.append(u);
                }
                continue;
            }
            // A multi-byte UTF-8 sequence: the lead byte says how long.
            let n = if b & 0xE0 == 0xC0 { 2 } else if b & 0xF0 == 0xE0 { 3 } else if b & 0xF8 == 0xF0 { 4 } else { 0 };
            if n == 0 { return .Err(self.makeError(.URIError, "URI malformed")); }
            var cp = b & (if n == 2 { 0x1F } else if n == 3 { 0x0F } else { 0x07 });
            for _ in 1..<n {
                if i + 3 > s.count or Int64(from: s(i)) != 37 { return .Err(self.makeError(.URIError, "URI malformed")); }
                let c = hexValue(s, i + 1, 2);
                if c < 0 or c & 0xC0 != 0x80 { return .Err(self.makeError(.URIError, "URI malformed")); }
                cp = (cp << 6) | (c & 0x3F);
                i = i + 3;
            }
            // Overlong forms, surrogates and out-of-range code points.
            let min = if n == 2 { 0x80 } else if n == 3 { 0x800 } else { 0x10000 };
            if cp < min or cp > 0x10FFFF or (cp >= 0xD800 and cp <= 0xDFFF) {
                return .Err(self.makeError(.URIError, "URI malformed"));
            }
            if cp >= 0x10000 {
                out.append(UInt16(truncating: 0xD800 + ((cp - 0x10000) >> 10)));
                out.append(UInt16(truncating: 0xDC00 + ((cp - 0x10000) & 0x3FF)));
            } else {
                out.append(UInt16(truncating: cp));
            }
        }
        .Ok(self.newString(out))
    }
}

/// GetSubstitution with no captures: `$$`, `$&`, `` $` `` and `$'`.
func substitution(matched: Units, s: Units, position: Int64, template: Units) -> Units {
    var out: Units = [];
    var i: Int64 = 0;
    while i < template.count {
        let c = Int64(from: template(i));
        if c != 36 or i + 1 >= template.count {
            out.append(template(i));
            i = i + 1;
            continue;
        }
        let d = Int64(from: template(i + 1));
        if d == 36 {
            out.append(36);
        } else if d == 38 {
            for x in matched { out.append(x); }
        } else if d == 96 {
            for j in 0..<position { out.append(s(j)); }
        } else if d == 39 {
            let tail = position + matched.count;
            for j in tail..<s.count { out.append(s(j)); }
        } else {
            out.append(template(i));
            i = i + 1;
            continue;
        }
        i = i + 2;
    }
    out
}

func utf8Bytes(cp: Int64) -> Array[Int64] {
    if cp < 0x80 { return [cp]; }
    if cp < 0x800 { return [0xC0 | (cp >> 6), 0x80 | (cp & 0x3F)]; }
    if cp < 0x10000 { return [0xE0 | (cp >> 12), 0x80 | ((cp >> 6) & 0x3F), 0x80 | (cp & 0x3F)]; }
    [0xF0 | (cp >> 18), 0x80 | ((cp >> 12) & 0x3F), 0x80 | ((cp >> 6) & 0x3F), 0x80 | (cp & 0x3F)]
}

/// `%XY` for a byte, uppercase.
func percentHex(b: Int64) -> Units {
    let hex = unitsOf("0123456789ABCDEF");
    [37, hex((b >> 4) & 15), hex(b & 15)]
}

func hex4(v: Int64) -> Units {
    let hex = unitsOf("0123456789ABCDEF");
    [hex((v >> 12) & 15), hex((v >> 8) & 15), hex((v >> 4) & 15), hex(v & 15)]
}

/// The value of `count` hex digits at `at`, or -1.
func hexValue(s: Units, at: Int64, count: Int64) -> Int64 {
    if at + count > s.count { return -1; }
    var v: Int64 = 0;
    for i in 0..<count {
        let h = hexDigitValue(Int64(from: s(at + i)));
        if h < 0 { return -1; }
        v = v * 16 + h;
    }
    v
}

// ----------------------------------------------------------------------
// Exact decimal digits, for toFixed, toExponential and toPrecision
// ----------------------------------------------------------------------

/// The exact decimal digits of a positive finite double, without
/// leading or trailing zeros, and the point position: x = 0.d × 10^n.
/// A double is m × 2^e, so this is m × 2^e or m × 5^-e / 10^-e in
/// base-10^9 limbs.
func exactDigits(x: Float64) -> (Array[Int64], Int64) {
    let bits = x.bitPattern;
    let expField: UInt64 = (bits >> 52) & 0x7FF;
    let mantMask: UInt64 = 0xFFFFFFFFFFFFF;
    let hidden: UInt64 = 0x10000000000000;
    let biased = Int64(bitPattern: expField);
    var m = Int64(bitPattern: bits & mantMask);
    var e: Int64 = -1074;
    if biased != 0 {
        m = Int64(bitPattern: (bits & mantMask) | hidden);
        e = biased - 1075;
    }
    // Little-endian base 10^9 limbs.
    var limbs: [Int64] = [m % 1000000000, m / 1000000000];
    if e >= 0 {
        var k = e;
        while k > 0 {
            let s = if k > 29 { 29 } else { k };
            bigMul(limbs, 1 << s);
            k = k - s;
        }
    } else {
        var k = 0 - e;
        while k > 0 {
            let s = if k > 13 { 13 } else { k };
            var p: Int64 = 1;
            for _ in 0..<s { p = p * 5; }
            bigMul(limbs, p);
            k = k - s;
        }
    }
    // Limbs to digits, most significant first.
    var digits: [Int64] = [];
    var top = limbs.count - 1;
    while top > 0 and limbs(top) == 0 { top = top - 1; }
    var first = true;
    var i = top;
    while i >= 0 {
        var chunk: [Int64] = [];
        var v = limbs(i);
        for _ in 0..<9 {
            chunk.append(v % 10);
            v = v / 10;
        }
        var j: Int64 = 8;
        while j >= 0 {
            if not (first and chunk(j) == 0 and digits.count == 0) { digits.append(chunk(j)); }
            j = j - 1;
        }
        first = false;
        i = i - 1;
    }
    let total = digits.count;
    let point = if e >= 0 { total } else { total + e };
    while digits.count > 1 and digits(digits.count - 1) == 0 { let _ = digits.pop(); }
    (digits, point)
}

/// limbs × k in place, for k < 2^31.
func bigMul(mutating limbs: Array[Int64], k: Int64) {
    var carry: Int64 = 0;
    for i in 0..<limbs.count {
        let v = limbs(i) * k + carry;
        limbs(i) = v % 1000000000;
        carry = v / 1000000000;
    }
    while carry > 0 {
        limbs.append(carry % 1000000000);
        carry = carry / 1000000000;
    }
}

/// The first `count` digits of 0.d × 10^n rounded half up, padded with
/// zeros, and the new point position.
func roundDigits(d: Array[Int64], n: Int64, count: Int64) -> (Array[Int64], Int64) {
    var out: [Int64] = [];
    for i in 0..<count { out.append(if i < d.count { d(i) } else { 0 }); }
    if count >= d.count or d(count) < 5 { return (out, n); }
    var i = count - 1;
    while i >= 0 {
        if out(i) < 9 {
            out(i) = out(i) + 1;
            return (out, n);
        }
        out(i) = 0;
        i = i - 1;
    }
    // All nines: 99.9 → 100.
    var carried: [Int64] = [1];
    for j in 0..<(count - 1) { carried.append(out(j)); }
    (carried, n + 1)
}

func digitsText(d: Array[Int64], from: Int64, to: Int64) -> String {
    var s = "";
    for i in from..<to { s.append(char: Char(unchecked: UInt32(truncating: 48 + d(i)))); }
    s
}

/// Number.prototype.toFixed for |x| < 1e21, rounding the exact value.
func toFixedExact(x: Float64, f: Int64) -> String {
    let sign = if x < 0.0 { "-" } else { "" };
    let a = x.abs();
    var digits: [Int64] = [];
    if a > 0.0 {
        let (d, n) = exactDigits(a);
        // Digits of a × 10^f, rounded to an integer.
        let count = n + f;
        if count > 0 {
            let (r, n2) = roundDigits(d, n, count);
            digits = r;
            if n2 > n { digits.append(0); digits = trimToCount(digits, count + 1); }
        } else if count == 0 and d(0) >= 5 {
            digits = [1];
        }
    }
    // Pad to at least f + 1 digits, then place the point.
    while digits.count < f + 1 { digits.insert(0, at: 0); }
    let intLen = digits.count - f;
    var s = sign + digitsText(digits, 0, intLen);
    if f > 0 { s = s + "." + digitsText(digits, intLen, digits.count); }
    s
}

func trimToCount(d: Array[Int64], count: Int64) -> Array[Int64] {
    var out: [Int64] = [];
    for i in 0..<count { out.append(d(i)); }
    out
}

/// Number.prototype.toExponential; `f` < 0 means as many digits as it
/// takes.
func toExponential(x: Float64, f: Int64) -> String {
    let sign = if x < 0.0 { "-" } else { "" };
    let a = x.abs();
    var d: [Int64] = [];
    var e: Int64 = 0;
    if a == 0.0 {
        let count = if f < 0 { 1 } else { f + 1 };
        for _ in 0..<count { d.append(0); }
    } else if f < 0 {
        let (s, n) = shortestDigits(a);
        for c in s.chars { d.append(Int64(from: c.value()) - 48); }
        e = n - 1;
    } else {
        let (ds, n) = exactDigits(a);
        let (r, n2) = roundDigits(ds, n, f + 1);
        d = r;
        e = n2 - 1;
    }
    var s = sign + digitsText(d, 0, 1);
    if d.count > 1 { s = s + "." + digitsText(d, 1, d.count); }
    s + "e" + (if e >= 0 { "+" } else { "-" }) + "\(e.abs())"
}

/// Number.prototype.toPrecision with `p` significant digits.
func toPrecision(x: Float64, p: Int64) -> String {
    let sign = if x < 0.0 { "-" } else { "" };
    let a = x.abs();
    var d: [Int64] = [];
    var e: Int64 = 0;
    if a == 0.0 {
        for _ in 0..<p { d.append(0); }
    } else {
        let (ds, n) = exactDigits(a);
        let (r, n2) = roundDigits(ds, n, p);
        d = r;
        e = n2 - 1;
    }
    if e < -6 or e >= p {
        var s = sign + digitsText(d, 0, 1);
        if p > 1 { s = s + "." + digitsText(d, 1, d.count); }
        return s + "e" + (if e >= 0 { "+" } else { "-" }) + "\(e.abs())";
    }
    if e >= 0 {
        var s = sign + digitsText(d, 0, e + 1);
        if p > e + 1 { s = s + "." + digitsText(d, e + 1, d.count); }
        return s;
    }
    sign + "0." + "0".repeated(0 - e - 1) + digitsText(d, 0, d.count)
}
