module js

// JSON.parse and JSON.stringify (ECMA-262 §25.5).

/// JSON.stringify's state: the replacer, the property allow-list, the
/// indentation and the objects being serialized (for cycle checks).
struct JsonState {
    var replacer: Value
    var allowList: Optional[Array[Key]]
    var gap: Units
    var indent: Units
    var stack: [Value]
}

/// JSON.parse's cursor over the source text.
struct JsonParser {
    var text: Units
    var pos: Int64
}

extend VM {
    // ------------------------------------------------------------------
    // JSON.stringify
    // ------------------------------------------------------------------

    mutating func jsonStringifyNative(value: Value, replacer: Value, space: Value) -> Result[Value, Value] {
        var st = JsonState(replacer: Value.undefined, allowList: .None, gap: [], indent: [], stack: []);
        if self.isCallable(replacer) {
            st.replacer = replacer;
        } else if try self.isArrayV(replacer) {
            var keys: [Key] = [];
            let len = try self.lengthOf(replacer);
            for i in 0..<len {
                let v = try self.getIndex(replacer, i);
                var item: Optional[Key] = .None;
                if v.isString or v.isNumber {
                    item = .Some(try self.toKey(v));
                } else if v.isObject {
                    match self.kindAt(v.asRef) {
                        .StringBox(_) or .NumberBox(_) => item = .Some(try self.toKey(try self.toStringValue(v))),
                        _ => {}
                    }
                }
                if let .Some(k) = item {
                    if not keys.contains(k) { keys.append(k); }
                }
            }
            st.allowList = .Some(keys);
        }
        var sp = space;
        if sp.isObject {
            match self.kindAt(sp.asRef) {
                .NumberBox(_) => sp = Value.number(try self.toNumber(sp)),
                .StringBox(_) => sp = try self.toStringValue(sp),
                _ => {}
            }
        }
        if sp.isNumber {
            let n = toIntegerOrInfinityOf(sp.asNumber);
            let count = if n < 1.0 { 0 } else if n > 10.0 { 10 } else { n.toInt64() ?? 0 };
            for _ in 0..<count { st.gap.append(32); }
        } else if sp.isString {
            let u = self.units(sp);
            for i in 0..<u.count {
                if i >= 10 { break; }
                st.gap.append(u(i));
            }
        }
        let wrapper = self.newObject();
        self.pushRoot(wrapper);
        let empty = self.atom("");
        self.defineOwn(wrapper.asRef, empty, value, Attr.all);
        let out = try self.jsonProperty(st, empty, wrapper);
        match out {
            .Some(u) => .Ok(self.newString(u)),
            .None => .Ok(Value.undefined)
        }
    }

    /// SerializeJSONProperty: `holder[key]` as JSON, or None to leave it out.
    mutating func jsonProperty(mutating st: JsonState, key: Key, holder: Value) -> Result[Optional[Units], Value] {
        var value = try self.getV(holder, key);
        if value.isObject {
            let toJSON = try self.getV(value, self.atom("toJSON"));
            if self.isCallable(toJSON) {
                value = try self.callValue(toJSON, value, [self.keyValue(key)]);
            }
        }
        if not st.replacer.isUndefined {
            value = try self.callValue(st.replacer, holder, [self.keyValue(key), value]);
        }
        if value.isObject {
            match self.kindAt(value.asRef) {
                .NumberBox(_) => value = Value.number(try self.toNumber(value)),
                .StringBox(_) => value = try self.toStringValue(value),
                .BooleanBox(b) => value = Value.bool(b),
                _ => {}
            }
        }
        if value.isNull { return .Ok(.Some(unitsOf("null"))); }
        if value.isBool { return .Ok(.Some(unitsOf(if value.asBool { "true" } else { "false" }))); }
        if value.isString { return .Ok(.Some(jsonQuote(self.units(value)))); }
        if value.isNumber {
            let x = value.asNumber;
            return .Ok(.Some(unitsOf(if x.isFinite { numberToString(x) } else { "null" })));
        }
        if value.isObject and not self.isCallable(value) {
            let isArr = try self.isArrayV(value);
            let out = if isArr { try self.jsonArray(st, value) } else { try self.jsonObject(st, value) };
            return .Ok(.Some(out));
        }
        .Ok(.None)
    }

    /// Enters `value` for serialization: checks for cycles and indents.
    mutating func jsonEnter(mutating st: JsonState, value: Value) -> Result[Units, Value] {
        for v in st.stack {
            if v.same(value) { return self.throwType("Converting circular structure to JSON"); }
        }
        st.stack.append(value);
        self.pushRoot(value);
        let stepback = st.indent;
        for c in st.gap { st.indent.append(c); }
        .Ok(stepback)
    }

    mutating func jsonLeave(mutating st: JsonState, stepback: Units) {
        let _ = st.stack.pop();
        self.popRoots(1);
        st.indent = stepback;
    }

    /// Joins serialized members with the right brackets and indentation.
    func jsonJoin(st: JsonState, parts: Array[Units], open: UInt16, close: UInt16, stepback: Units) -> Units {
        var out: Units = [open];
        if parts.count == 0 {
            out.append(close);
            return out;
        }
        for i in 0..<parts.count {
            if i > 0 { out.append(44); }
            if st.gap.count > 0 {
                out.append(10);
                for c in st.indent { out.append(c); }
            }
            for c in parts(i) { out.append(c); }
        }
        if st.gap.count > 0 {
            out.append(10);
            for c in stepback { out.append(c); }
        }
        out.append(close);
        out
    }

    mutating func jsonObject(mutating st: JsonState, value: Value) -> Result[Units, Value] {
        let stepback = try self.jsonEnter(st, value);
        var keys: [Key] = [];
        match st.allowList {
            .Some(list) => keys = list,
            .None => {
                let ownKeyList9 = try self.ownKeys(value.asRef);
                for k in ownKeyList9 {
                    if keyIsSymbol(k) { continue; }
                    let desc = try self.getOwnPropertyV(value, k);
                    if let .Some(p) = desc {
                        if p.enumerable ?? false { keys.append(k); }
                    }
                }
            }
        }
        var parts: [Units] = [];
        for k in keys {
            if let .Some(u) = try self.jsonProperty(st, k, value) {
                var member = jsonQuote(unitsOf(self.atomNames(k)));
                member.append(58);
                if st.gap.count > 0 { member.append(32); }
                for c in u { member.append(c); }
                parts.append(member);
            }
        }
        let out = self.jsonJoin(st, parts, 123, 125, stepback);
        self.jsonLeave(st, stepback);
        .Ok(out)
    }

    mutating func jsonArray(mutating st: JsonState, value: Value) -> Result[Units, Value] {
        let stepback = try self.jsonEnter(st, value);
        let len = try self.lengthOf(value);
        var parts: [Units] = [];
        for i in 0..<len {
            let k = self.internIndex(i);
            match try self.jsonProperty(st, k, value) {
                .Some(u) => parts.append(u),
                .None => parts.append(unitsOf("null"))
            }
        }
        let out = self.jsonJoin(st, parts, 91, 93, stepback);
        self.jsonLeave(st, stepback);
        .Ok(out)
    }

    // ------------------------------------------------------------------
    // JSON.parse
    // ------------------------------------------------------------------

    mutating func jsonParseNative(text: Value, reviver: Value) -> Result[Value, Value] {
        let source = try self.toUnits(text);
        var p = JsonParser(text: source, pos: 0);
        let mark = self.roots.count;
        jsonSkipSpace(p);
        let value = try self.jsonValue(p);
        jsonSkipSpace(p);
        if p.pos < p.text.count { return self.jsonError(p); }
        self.truncateRoots(mark);
        if not self.isCallable(reviver) { return .Ok(value); }
        let root = self.newObject();
        self.pushRoot(root);
        let empty = self.atom("");
        self.defineOwn(root.asRef, empty, value, Attr.all);
        let result = try self.jsonRevive(root, empty, reviver);
        self.popRoots(1);
        .Ok(result)
    }

    mutating func jsonError[T](p: JsonParser) -> Result[T, Value] {
        let what: String = if p.pos >= p.text.count { "Unexpected end of JSON input" } else { "Unexpected token in JSON at position \(p.pos)" };
        .Err(self.makeError(.SyntaxError, what))
    }

    /// One JSON value. Objects and arrays it makes stay rooted until
    /// the caller truncates the roots.
    mutating func jsonValue(mutating p: JsonParser) -> Result[Value, Value] {
        if p.pos >= p.text.count { return self.jsonError(p); }
        let c = Int64(from: p.text(p.pos));
        if c == 123 { return self.jsonObjectLiteral(p); }
        if c == 91 { return self.jsonArrayLiteral(p); }
        if c == 34 {
            let u = try self.jsonString(p);
            return .Ok(self.newString(u));
        }
        if c == 45 or (c >= 48 and c <= 57) { return self.jsonNumber(p); }
        if jsonWord(p, "true") { return .Ok(Value.bool(true)); }
        if jsonWord(p, "false") { return .Ok(Value.bool(false)); }
        if jsonWord(p, "null") { return .Ok(Value.nullValue); }
        self.jsonError(p)
    }

    mutating func jsonObjectLiteral(mutating p: JsonParser) -> Result[Value, Value] {
        p.pos = p.pos + 1;
        let o = self.newObject();
        self.pushRoot(o);
        jsonSkipSpace(p);
        if jsonPeek(p) == 125 {
            p.pos = p.pos + 1;
            return .Ok(o);
        }
        while true {
            jsonSkipSpace(p);
            if jsonPeek(p) != 34 { return self.jsonError(p); }
            let name = try self.jsonString(p);
            jsonSkipSpace(p);
            if jsonPeek(p) != 58 { return self.jsonError(p); }
            p.pos = p.pos + 1;
            jsonSkipSpace(p);
            let v = try self.jsonValue(p);
            let k = self.atomOfUnits(name);
            if k == self.atom("__proto__") {
                // A JSON "__proto__" is an ordinary property.
                let _ = self.definePropertyRaw(o.asRef, k, v);
            } else {
                self.defineOwn(o.asRef, k, v, Attr.all);
            }
            jsonSkipSpace(p);
            let d = jsonPeek(p);
            p.pos = p.pos + 1;
            if d == 125 { break; }
            if d != 44 { return self.jsonError(p); }
        }
        .Ok(o)
    }

    /// Defines a data property without going through [[Set]].
    mutating func definePropertyRaw(r: Ref, k: Key, v: Value) -> Bool {
        self.defineOwn(r, k, v, Attr.all);
        true
    }

    mutating func jsonArrayLiteral(mutating p: JsonParser) -> Result[Value, Value] {
        p.pos = p.pos + 1;
        let a = self.newArray([]);
        self.pushRoot(a);
        jsonSkipSpace(p);
        if jsonPeek(p) == 93 {
            p.pos = p.pos + 1;
            return .Ok(a);
        }
        while true {
            jsonSkipSpace(p);
            let v = try self.jsonValue(p);
            self.appendElement(a.asRef, v);
            jsonSkipSpace(p);
            let d = jsonPeek(p);
            p.pos = p.pos + 1;
            if d == 93 { break; }
            if d != 44 { return self.jsonError(p); }
        }
        .Ok(a)
    }

    mutating func jsonString(mutating p: JsonParser) -> Result[Units, Value] {
        p.pos = p.pos + 1;
        var out: Units = [];
        while true {
            if p.pos >= p.text.count { return self.jsonError(p); }
            let c = p.text(p.pos);
            let x = Int64(from: c);
            p.pos = p.pos + 1;
            if x == 34 { break; }
            if x < 32 {
                p.pos = p.pos - 1;
                return self.jsonError(p);
            }
            if x != 92 {
                out.append(c);
                continue;
            }
            if p.pos >= p.text.count { return self.jsonError(p); }
            let e = Int64(from: p.text(p.pos));
            p.pos = p.pos + 1;
            if e == 34 or e == 92 or e == 47 { out.append(UInt16(truncating: e)); }
            else if e == 98 { out.append(8); }
            else if e == 102 { out.append(12); }
            else if e == 110 { out.append(10); }
            else if e == 114 { out.append(13); }
            else if e == 116 { out.append(9); }
            else if e == 117 {
                var v: Int64 = 0;
                for _ in 0..<4 {
                    if p.pos >= p.text.count { return self.jsonError(p); }
                    let h = hexDigitValue(Int64(from: p.text(p.pos)));
                    if h < 0 { return self.jsonError(p); }
                    v = v * 16 + h;
                    p.pos = p.pos + 1;
                }
                out.append(UInt16(truncating: v));
            } else {
                p.pos = p.pos - 1;
                return self.jsonError(p);
            }
        }
        .Ok(out)
    }

    mutating func jsonNumber(mutating p: JsonParser) -> Result[Value, Value] {
        let start = p.pos;
        if jsonPeek(p) == 45 { p.pos = p.pos + 1; }
        if jsonPeek(p) == 48 {
            p.pos = p.pos + 1;
        } else if jsonIsDigit(jsonPeek(p)) {
            while jsonIsDigit(jsonPeek(p)) { p.pos = p.pos + 1; }
        } else {
            return self.jsonError(p);
        }
        if jsonPeek(p) == 46 {
            p.pos = p.pos + 1;
            if not jsonIsDigit(jsonPeek(p)) { return self.jsonError(p); }
            while jsonIsDigit(jsonPeek(p)) { p.pos = p.pos + 1; }
        }
        if jsonPeek(p) == 101 or jsonPeek(p) == 69 {
            p.pos = p.pos + 1;
            if jsonPeek(p) == 43 or jsonPeek(p) == 45 { p.pos = p.pos + 1; }
            if not jsonIsDigit(jsonPeek(p)) { return self.jsonError(p); }
            while jsonIsDigit(jsonPeek(p)) { p.pos = p.pos + 1; }
        }
        .Ok(Value.number(stringToNumber(unitsSlice(p.text, start, p.pos))))
    }

    /// InternalizeJSONProperty: runs the reviver bottom-up.
    mutating func jsonRevive(holder: Value, key: Key, reviver: Value) -> Result[Value, Value] {
        let value = try self.getV(holder, key);
        if value.isObject {
            self.pushRoot(value);
            if try self.isArrayV(value) {
                let len = try self.lengthOf(value);
                for i in 0..<len {
                    try self.jsonReviveMember(value, self.internIndex(i), reviver);
                }
            } else {
                let ownKeyList10 = try self.ownKeys(value.asRef);
                for k in ownKeyList10 {
                    if keyIsSymbol(k) { continue; }
                    let p = try self.getOwnPropertyV(value, k);
                    if let .Some(rec) = p {
                        if rec.enumerable ?? false { try self.jsonReviveMember(value, k, reviver); }
                    }
                }
            }
            self.popRoots(1);
        }
        self.callValue(reviver, holder, [self.keyValue(key), value])
    }

    mutating func jsonReviveMember(value: Value, k: Key, reviver: Value) -> Result[(), Value] {
        let v = try self.jsonRevive(value, k, reviver);
        if v.isUndefined {
            let hp10 = try self.deleteOwn(value.asRef, k);
            let _ = hp10;
        } else {
            let desc = Descriptor(value: .Some(v), writable: .Some(true), getter: .None, setter: .None,
                                  enumerable: .Some(true), configurable: .Some(true));
            let _ = try self.defineProperty(value.asRef, k, desc);
        }
        .Ok(())
    }
}

func jsonPeek(p: JsonParser) -> Int64 {
    if p.pos >= p.text.count { return -1; }
    Int64(from: p.text(p.pos))
}

func jsonIsDigit(c: Int64) -> Bool { c >= 48 and c <= 57 }

/// JSON whitespace: tab, newline, carriage return and space only.
func jsonSkipSpace(mutating p: JsonParser) {
    while p.pos < p.text.count {
        let c = Int64(from: p.text(p.pos));
        if c != 9 and c != 10 and c != 13 and c != 32 { return; }
        p.pos = p.pos + 1;
    }
}

/// Consumes `word` if the text continues with it.
func jsonWord(mutating p: JsonParser, word: String) -> Bool {
    let w = unitsOf(word);
    if p.pos + w.count > p.text.count { return false; }
    for i in 0..<w.count {
        if p.text(p.pos + i) != w(i) { return false; }
    }
    p.pos = p.pos + w.count;
    true
}

func hexDigitValue(c: Int64) -> Int64 {
    if c >= 48 and c <= 57 { return c - 48; }
    if c >= 97 and c <= 102 { return c - 87; }
    if c >= 65 and c <= 70 { return c - 55; }
    return -1;
}

/// ToIntegerOrInfinity on a number.
func toIntegerOrInfinityOf(x: Float64) -> Float64 {
    if x.isNaN { return 0.0; }
    if x.isInfinite { return x; }
    let t = x.trunc();
    if t == 0.0 { 0.0 } else { t }
}
