module js

// String built-ins.

extend VM {
    mutating func stringNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .StringCtor => {
                var s = self.str("");
                if args.count > 0 {
                    if a0.isSymbol and newTarget.isUndefined {
                        s = self.str(self.keyText(symbolKey(a0.asRef)));
                    } else {
                        s = try self.toStringValue(a0);
                    }
                }
                if newTarget.isUndefined { return .Ok(.Some(s)); }
                let proto = try self.prototypeFor(newTarget, self.intrinsics.stringProto);
                self.newObjectKind(proto, .StringBox(s))
            },
            .StringFromCharCode => {
                var u: Units = [];
                for a in args { u.append(UInt16(truncating: toUint32(try self.toNumber(a)) & 0xFFFF)); }
                self.newString(u)
            },
            .StringFromCodePoint => {
                var u: Units = [];
                for a in args {
                    let x = try self.toNumber(a);
                    if x.isNaN or x < 0.0 or x > 1114111.0 or x.trunc() != x {
                        return .Err(self.makeError(.RangeError, "Invalid code point \(numberToString(x))"));
                    }
                    let c = x.toInt64() ?? 0;
                    if c >= 0x10000 {
                        let y = c - 0x10000;
                        u.append(UInt16(truncating: 0xD800 + (y >> 10)));
                        u.append(UInt16(truncating: 0xDC00 + (y & 0x3FF)));
                    } else {
                        u.append(UInt16(truncating: c));
                    }
                }
                self.newString(u)
            },
            .StringCharAt => {
                let s = try self.thisUnits(this, "charAt");
                let i = try self.toIntegerOrInfinity(a0);
                if i < 0.0 or i >= Float64(from: s.count) { self.str("") } else { self.newString([s(i.toInt64() ?? 0)]) }
            },
            .StringCharCodeAt => {
                let s = try self.thisUnits(this, "charCodeAt");
                let i = try self.toIntegerOrInfinity(a0);
                if i < 0.0 or i >= Float64(from: s.count) { Value.number(Float64.nan) } else { Value.int(Int64(from: s(i.toInt64() ?? 0))) }
            },
            .StringCodePointAt => {
                let s = try self.thisUnits(this, "codePointAt");
                let f = try self.toIntegerOrInfinity(a0);
                if f < 0.0 or f >= Float64(from: s.count) {
                    Value.undefined
                } else {
                    let i = f.toInt64() ?? 0;
                    let hi = Int64(from: s(i));
                    if hi >= 0xD800 and hi <= 0xDBFF and i + 1 < s.count {
                        let lo = Int64(from: s(i + 1));
                        if lo >= 0xDC00 and lo <= 0xDFFF {
                            return .Ok(.Some(Value.int(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00))));
                        }
                    }
                    Value.int(hi)
                }
            },
            .StringIndexOf => {
                let s = try self.thisUnits(this, "indexOf");
                let needle = try self.toUnits(a0);
                let p = try self.toIntegerOrInfinity(a1);
                let from = relativeIndex(if p < 0.0 { 0.0 } else { p }, s.count);
                Value.int(unitsIndexOf(s, needle, from))
            },
            .StringLastIndexOf => {
                let s = try self.thisUnits(this, "lastIndexOf");
                let needle = try self.toUnits(a0);
                let p = try self.toNumber(a1);
                var from = if p.isNaN { s.count } else { relativeIndex(if p < 0.0 { 0.0 } else { p.trunc() }, s.count) };
                var found: Int64 = -1;
                while from >= 0 {
                    if from + needle.count <= s.count and unitsIndexOf(s, needle, from) == from {
                        found = from;
                        break;
                    }
                    from = from - 1;
                }
                Value.int(found)
            },
            .StringIncludes or .StringStartsWith or .StringEndsWith => {
                let s = try self.thisUnits(this, "includes");
                if a0.isObject and self.heap.objects(a0.asRef).proto.same(self.intrinsics.regexpProto) {
                    return self.throwType("First argument must not be a regular expression");
                }
                let needle = try self.toUnits(a0);
                if n == NativeFn.StringEndsWith {
                    let end = if a1.isUndefined { s.count } else { relativeIndex(clampNonNegative(try self.toIntegerOrInfinity(a1)), s.count) };
                    let start = end - needle.count;
                    Value.bool(start >= 0 and unitsEqual(unitsSlice(s, start, end), needle))
                } else {
                    let from = relativeIndex(clampNonNegative(try self.toIntegerOrInfinity(a1)), s.count);
                    if n == NativeFn.StringStartsWith {
                        Value.bool(from + needle.count <= s.count and unitsEqual(unitsSlice(s, from, from + needle.count), needle))
                    } else {
                        Value.bool(unitsIndexOf(s, needle, from) >= 0)
                    }
                }
            },
            .StringSlice => {
                let s = try self.thisUnits(this, "slice");
                let start = relativeIndex(try self.toIntegerOrInfinity(a0), s.count);
                let end = if a1.isUndefined { s.count } else { relativeIndex(try self.toIntegerOrInfinity(a1), s.count) };
                self.newString(if start < end { unitsSlice(s, start, end) } else { [] })
            },
            .StringSubstring => {
                let s = try self.thisUnits(this, "substring");
                let a = relativeIndex(clampNonNegative(try self.toIntegerOrInfinity(a0)), s.count);
                let b = if a1.isUndefined { s.count } else { relativeIndex(clampNonNegative(try self.toIntegerOrInfinity(a1)), s.count) };
                let lo = if a < b { a } else { b };
                let hi = if a < b { b } else { a };
                self.newString(unitsSlice(s, lo, hi))
            },
            .StringSubstr => {
                let s = try self.thisUnits(this, "substr");
                let start = relativeIndex(try self.toIntegerOrInfinity(a0), s.count);
                let len = if a1.isUndefined { Float64(from: s.count) } else { try self.toIntegerOrInfinity(a1) };
                let maxLen = Float64(from: s.count - start);
                let l = if len < 0.0 { 0.0 } else if len > maxLen { maxLen } else { len };
                self.newString(unitsSlice(s, start, start + (l.toInt64() ?? 0)))
            },
            .StringToUpperCase => {
                let s = try self.thisUnits(this, "toUpperCase");
                self.str(stringOf(s).uppercased())
            },
            .StringToLowerCase => {
                let s = try self.thisUnits(this, "toLowerCase");
                self.str(stringOf(s).lowercased())
            },
            .StringTrim => self.newString(unitsTrim(try self.thisUnits(this, "trim"), true, true)),
            .StringTrimStart => self.newString(unitsTrim(try self.thisUnits(this, "trimStart"), true, false)),
            .StringTrimEnd => self.newString(unitsTrim(try self.thisUnits(this, "trimEnd"), false, true)),
            .StringSplit => {
                if this.isNullish { return self.throwType("String.prototype.split called on null or undefined"); }
                if not a0.isNullish {
                    let m = try self.getMethod(a0, symbolKey(WellKnown.split));
                    if not m.isUndefined { return .Ok(.Some(try self.callValue(m, a0, [this, a1]))); }
                }
                let s = try self.thisUnits(this, "split");
                let limit = if a1.isUndefined { 4294967295 } else { toUint32(try self.toNumber(a1)) };
                if a0.isUndefined {
                    let whole = self.newString(s);
                    return .Ok(.Some(self.newArray(if limit == 0 { [] } else { [whole] })));
                }
                let sep = try self.toUnits(a0);
                var parts: [Value] = [];
                if limit == 0 { return .Ok(.Some(self.newArray([]))); }
                if sep.count == 0 {
                    for u in s {
                        if parts.count >= limit { break; }
                        parts.append(self.newString([u]));
                    }
                    return .Ok(.Some(self.newArray(parts)));
                }
                var start: Int64 = 0;
                while true {
                    let i = unitsIndexOf(s, sep, start);
                    if i < 0 { break; }
                    parts.append(self.newString(unitsSlice(s, start, i)));
                    if parts.count >= limit { return .Ok(.Some(self.newArray(parts))); }
                    start = i + sep.count;
                }
                parts.append(self.newString(unitsSlice(s, start, s.count)));
                self.newArray(parts)
            },
            .StringRepeat => {
                let s = try self.thisUnits(this, "repeat");
                let count = try self.toIntegerOrInfinity(a0);
                if count < 0.0 or count.isInfinite { return .Err(self.makeError(.RangeError, "Invalid count value")); }
                let c = count.toInt64() ?? 0;
                if c * s.count > 100000000 { return .Err(self.makeError(.RangeError, "Invalid string length")); }
                var out: Units = [];
                for _ in 0..<c { for u in s { out.append(u); } }
                self.newString(out)
            },
            .StringPadStart or .StringPadEnd => {
                let s = try self.thisUnits(this, "padStart");
                let maxLen = try self.lengthOfValue(a0);
                if maxLen <= s.count { return .Ok(.Some(self.newString(s))); }
                let filler = if a1.isUndefined { unitsOf(" ") } else { try self.toUnits(a1) };
                if filler.count == 0 { return .Ok(.Some(self.newString(s))); }
                var pad: Units = [];
                var i: Int64 = 0;
                while pad.count < maxLen - s.count {
                    pad.append(filler(i % filler.count));
                    i = i + 1;
                }
                var out: Units = [];
                if n == NativeFn.StringPadStart {
                    for u in pad { out.append(u); }
                    for u in s { out.append(u); }
                } else {
                    for u in s { out.append(u); }
                    for u in pad { out.append(u); }
                }
                self.newString(out)
            },
            .StringConcat => {
                var s = try self.thisUnits(this, "concat");
                for a in args { for u in try self.toUnits(a) { s.append(u); } }
                self.newString(s)
            },
            .StringToString => {
                if this.isString { return .Ok(.Some(this)); }
                if this.isObject {
                    if let .StringBox(s) = self.kindAt(this.asRef) { return .Ok(.Some(s)); }
                }
                return self.throwType("String.prototype.toString requires that 'this' be a String");
            },
            .StringAt => {
                let s = try self.thisUnits(this, "at");
                let rel = try self.toIntegerOrInfinity(a0);
                let k = if rel >= 0.0 { rel } else { Float64(from: s.count) + rel };
                if k < 0.0 or k >= Float64(from: s.count) { Value.undefined } else { self.newString([s(k.toInt64() ?? 0)]) }
            },
            .StringLocaleCompare => {
                let s = try self.thisUnits(this, "localeCompare");
                let t = try self.toUnits(a0);
                Value.int(if unitsLess(s, t) { -1 } else if unitsEqual(s, t) { 0 } else { 1 })
            },
            .StringIterator => {
                let s = try self.thisUnits(this, "[Symbol.iterator]");
                let sv = self.newString(s);
                self.newObjectKind(self.intrinsics.stringIteratorProto, .StringIterator(target: sv, index: 0))
            },
            .StringIteratorNext => {
                if not this.isObject { return self.throwType("next called on incompatible receiver"); }
                let (target, index) = match self.kindAt(this.asRef) {
                    .StringIterator(t, i) => (t, i),
                    _ => return self.throwType("next called on incompatible receiver")
                };
                if target.isUndefined { return .Ok(.Some(self.iterResult(Value.undefined, true))); }
                let s = self.units(target);
                if index >= s.count {
                    self.heap.objects(this.asRef).kind = .StringIterator(target: Value.undefined, index: index);
                    return .Ok(.Some(self.iterResult(Value.undefined, true)));
                }
                var end = index + 1;
                let hi = Int64(from: s(index));
                if hi >= 0xD800 and hi <= 0xDBFF and end < s.count {
                    let lo = Int64(from: s(end));
                    if lo >= 0xDC00 and lo <= 0xDFFF { end = end + 1; }
                }
                self.heap.objects(this.asRef).kind = .StringIterator(target: target, index: end);
                let piece = self.newString(unitsSlice(s, index, end));
                self.iterResult(piece, false)
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// ToLength of a value.
    mutating func lengthOfValue(v: Value) -> Result[Int64, Value] {
        let n = try self.toIntegerOrInfinity(v);
        if n <= 0.0 { return .Ok(0); }
        if n > 100000000.0 { return .Err(self.makeError(.RangeError, "Invalid string length")); }
        .Ok(n.toInt64() ?? 0)
    }
}

func clampNonNegative(x: Float64) -> Float64 { if x < 0.0 { 0.0 } else { x } }
