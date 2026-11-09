module js

// RegExp objects and the string methods that use them (ECMA-262 §22.2.4
// onward). The matching itself is in regex.ks.

extend VM {
    mutating func setupRegExp() {
        let g = self.global;
        let proto = self.newObject();
        self.intrinsics.regexpProto = proto;
        let ctor = self.constructorFor("RegExp", .RegExpCtor, 2, proto);
        let species = self.nativeFunction(.ReturnThis, "get [Symbol.species]", 0, false);
        self.defineAccessorOwn(ctor.asRef, symbolKey(WellKnown.species), species, Value.undefined, Attr.configurable);
        self.method(proto, "exec", .RegExpExec, 1);
        self.method(proto, "test", .RegExpTest, 1);
        self.method(proto, "toString", .RegExpToString, 0);
        self.method(proto, "compile", .RegExpCompile, 2);
        for (name, n) in [
            ("flags", NativeFn.RegExpFlags), ("source", NativeFn.RegExpSource), ("global", NativeFn.RegExpGlobal),
            ("ignoreCase", NativeFn.RegExpIgnoreCase), ("multiline", NativeFn.RegExpMultiline),
            ("dotAll", NativeFn.RegExpDotAll), ("unicode", NativeFn.RegExpUnicode),
            ("unicodeSets", NativeFn.RegExpUnicodeSets), ("sticky", NativeFn.RegExpSticky),
            ("hasIndices", NativeFn.RegExpHasIndices)
        ] {
            self.getter(proto, name, self.atom(name), n);
        }
        self.symbolMethod(proto, WellKnown.matchSym, "[Symbol.match]", .RegExpSymbolMatch, 1);
        self.symbolMethod(proto, WellKnown.matchAll, "[Symbol.matchAll]", .RegExpSymbolMatchAll, 1);
        self.symbolMethod(proto, WellKnown.replace, "[Symbol.replace]", .RegExpSymbolReplace, 2);
        self.symbolMethod(proto, WellKnown.search, "[Symbol.search]", .RegExpSymbolSearch, 1);
        self.symbolMethod(proto, WellKnown.split, "[Symbol.split]", .RegExpSymbolSplit, 2);

        let it = self.newObjectKind(self.intrinsics.iteratorProto, .Ordinary);
        self.intrinsics.regexpStringIteratorProto = it;
        self.method(it, "next", .RegExpStringIteratorNext, 0);
        let tag = self.str("RegExp String Iterator");
        self.defineOwn(it.asRef, symbolKey(WellKnown.toStringTag), tag, Attr.configurable);

        let sp = self.intrinsics.stringProto;
        self.method(sp, "match", .StringMatch, 1);
        self.method(sp, "matchAll", .StringMatchAll, 1);
        self.method(sp, "search", .StringSearch, 1);
        let _ = g;
    }

    /// The compiled program for a pattern and flags, compiling (and
    /// caching) it on first use.
    mutating func regexProgram(pattern: Units, flags: RxFlags, flagText: Units) -> Result[Int64, Value] {
        var key = stringOf(flagText);
        key.append("/");
        var plain = true;
        for c in pattern {
            if Int64(from: c) >= 0xD800 and Int64(from: c) <= 0xDFFF { plain = false; }
        }
        key.append(stringOf(pattern));
        if plain {
            if let .Some(id) = self.regexCache(key) { return .Ok(id); }
        }
        let program = match compileRegex(pattern, flags) {
            .Ok(p) => p,
            .Err(message) => return .Err(self.makeError(.SyntaxError, "Invalid regular expression: /\(stringOf(pattern))/: \(message)"))
        };
        self.regexPrograms.append(program);
        let id = self.regexPrograms.count - 1;
        if plain { self.regexCache.insert(key, id); }
        .Ok(id)
    }

    /// RegExpInitialize: (re)compiles `o` from a pattern and flags value.
    mutating func regexpInitialize(o: Value, pattern: Value, flags: Value) -> Result[Value, Value] {
        let p = if pattern.isUndefined { self.str("") } else { try self.toStringValue(pattern) };
        self.pushRoot(p);
        let f = if flags.isUndefined { self.str("") } else { try self.toStringValue(flags) };
        self.pushRoot(f);
        let parsed = match parseRxFlags(self.units(f)) {
            .Some(x) => x,
            .None => return .Err(self.makeError(.SyntaxError, "Invalid regular expression flags '\(self.text(f))'"))
        };
        let id = try self.regexProgram(self.units(p), parsed, self.units(f));
        self.heap.objects(o.asRef).kind = .RegExpObject(id: id, source: p, flags: f);
        self.popRoots(2);
        try self.setV(o, self.names.lastIndex, Value.int(0), true);
        .Ok(o)
    }

    /// RegExpAlloc: an uninitialized RegExp with its lastIndex property.
    mutating func regexpAlloc(newTarget: Value) -> Result[Value, Value] {
        let proto = try self.prototypeFor(newTarget, self.intrinsics.regexpProto);
        let o = self.newObjectKind(proto, .Ordinary);
        self.defineOwn(o.asRef, self.names.lastIndex, Value.int(0), Attr.writable);
        .Ok(o)
    }

    /// RegExpCreate: `new RegExp(pattern, flags)` with the intrinsic.
    mutating func regexpCreate(pattern: Value, flags: Value) -> Result[Value, Value] {
        let ctor = self.ownValue(self.intrinsics.regexpProto.asRef, self.names.constructor);
        let o = try self.regexpAlloc(ctor);
        self.pushRoot(o);
        let r = try self.regexpInitialize(o, pattern, flags);
        self.popRoots(1);
        .Ok(r)
    }

    func isRegExpObject(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .RegExpObject(_, _, _) => true,
            _ => false
        }
    }

    /// IsRegExp: an object with a truthy @@match, or a RegExp.
    mutating func isRegExp(v: Value) -> Result[Bool, Value] {
        if not v.isObject { return .Ok(false); }
        let m = try self.getV(v, symbolKey(WellKnown.matchSym));
        if not m.isUndefined { return .Ok(self.toBoolean(m)); }
        .Ok(self.isRegExpObject(v))
    }

    /// GetMethod: undefined for a missing method, else it must be callable.
    mutating func getMethod(v: Value, key: Key) -> Result[Value, Value] {
        let f = try self.getV(v, key);
        if f.isNullish { return .Ok(Value.undefined); }
        if not self.isCallable(f) { return self.throwType("\(self.keyText(key)) is not a function"); }
        .Ok(f)
    }

    mutating func speciesConstructor(o: Value, fallback: Value) -> Result[Value, Value] {
        let c = try self.getV(o, self.names.constructor);
        if c.isUndefined { return .Ok(fallback); }
        if not c.isObject { return self.throwType("object.constructor is not an object"); }
        let s = try self.getV(c, symbolKey(WellKnown.species));
        if s.isNullish { return .Ok(fallback); }
        if self.isConstructor(s) { return .Ok(s); }
        self.throwType("object.constructor[Symbol.species] is not a constructor")
    }

    /// RegExpBuiltinExec.
    mutating func regexpBuiltinExec(r: Value, s: Value) -> Result[Value, Value] {
        let (id, flagsValue) = match self.kindAt(r.asRef) {
            .RegExpObject(i, _, f) => (i, f),
            _ => return self.throwType("RegExp.prototype.exec called on incompatible receiver")
        };
        let flags = parseRxFlags(self.units(flagsValue)) ?? RxFlags(hasIndices: false, global: false, ignoreCase: false,
            multiline: false, dotAll: false, unicode: false, unicodeSets: false, sticky: false);
        let li = try self.getV(r, self.names.lastIndex);
        var lastIndex = toLengthOf(try self.toNumber(li));
        let input = self.units(s);
        let length = input.count;
        let globalOrSticky = flags.global or flags.sticky;
        if not globalOrSticky { lastIndex = 0; }
        let fullUnicode = flags.unicode or flags.unicodeSets;
        var caps: [Int64] = [];
        while true {
            if lastIndex > length {
                if globalOrSticky { try self.setV(r, self.names.lastIndex, Value.int(0), true); }
                return .Ok(Value.nullValue);
            }
            let found = rxMatchAt(self.regexPrograms(id), input, lastIndex);
            if let .Some(c) = found {
                caps = c;
                break;
            }
            if flags.sticky {
                try self.setV(r, self.names.lastIndex, Value.int(0), true);
                return .Ok(Value.nullValue);
            }
            lastIndex = advanceStringIndex(input, lastIndex, fullUnicode);
        }
        if globalOrSticky { try self.setV(r, self.names.lastIndex, Value.int(caps(1)), true); }
        let program = self.regexPrograms(id);
        let mark = self.roots.count;
        var items: [Value] = [];
        for g in 0..<(program.groupCount + 1) {
            let st = caps(g * 2);
            let en = caps(g * 2 + 1);
            let v = if st < 0 or en < 0 { Value.undefined } else { self.newString(unitsSlice(input, st, en)) };
            self.pushRoot(v);
            items.append(v);
        }
        let a = self.newArray(items);
        self.pushRoot(a);
        self.defineOwn(a.asRef, self.atom("index"), Value.int(caps(0)), Attr.all);
        self.defineOwn(a.asRef, self.atom("input"), s, Attr.all);
        var hasNames = false;
        for n in program.names { if not n.isEmpty { hasNames = true; } }
        var groups = Value.undefined;
        if hasNames {
            groups = self.newObjectKind(Value.nullValue, .Ordinary);
            for g in 1..<program.names.count {
                let n = program.names(g);
                if not n.isEmpty { self.defineOwn(groups.asRef, self.atom(n), items(g), Attr.all); }
            }
        }
        self.defineOwn(a.asRef, self.atom("groups"), groups, Attr.all);
        if flags.hasIndices {
            var pairs: [Value] = [];
            for g in 0..<(program.groupCount + 1) {
                let st = caps(g * 2);
                let en = caps(g * 2 + 1);
                let v = if st < 0 or en < 0 { Value.undefined } else { self.newArray([Value.int(st), Value.int(en)]) };
                self.pushRoot(v);
                pairs.append(v);
            }
            let indices = self.newArray(pairs);
            self.pushRoot(indices);
            var indexGroups = Value.undefined;
            if hasNames {
                indexGroups = self.newObjectKind(Value.nullValue, .Ordinary);
                for g in 1..<program.names.count {
                    let n = program.names(g);
                    if not n.isEmpty { self.defineOwn(indexGroups.asRef, self.atom(n), pairs(g), Attr.all); }
                }
            }
            self.defineOwn(indices.asRef, self.atom("groups"), indexGroups, Attr.all);
            self.defineOwn(a.asRef, self.atom("indices"), indices, Attr.all);
        }
        self.truncateRoots(mark);
        .Ok(a)
    }

    /// RegExpExec: a user `exec` if there is one, else the built-in.
    mutating func regexpExec(r: Value, s: Value) -> Result[Value, Value] {
        let exec = try self.getV(r, self.atom("exec"));
        if self.isCallable(exec) {
            let result = try self.callValue(exec, r, [s]);
            if not result.isObject and not result.isNull { return self.throwType("exec result must be an object or null"); }
            return .Ok(result);
        }
        if not self.isRegExpObject(r) { return self.throwType("RegExp exec called on incompatible receiver"); }
        self.regexpBuiltinExec(r, s)
    }

    /// The flags string of a RegExp-like object, via its `flags` getter.
    mutating func flagsOf(r: Value) -> Result[Units, Value] {
        let f = try self.getV(r, self.atom("flags"));
        self.toUnits(f)
    }

    /// After an empty match: lastIndex moves past it.
    mutating func advanceLastIndex(r: Value, s: Units, fullUnicode: Bool) -> Result[(), Value] {
        let li = try self.getV(r, self.names.lastIndex);
        let thisIndex = toLengthOf(try self.toNumber(li));
        self.setV(r, self.names.lastIndex, Value.int(advanceStringIndex(s, thisIndex, fullUnicode)), true)
    }

    /// One of the flag getters: `bit` is the flag's letter.
    mutating func flagGetter(this: Value, letter: Int64, name: String) -> Result[Value, Value] {
        if not this.isObject { return self.throwType("RegExp.prototype.\(name) getter called on non-object"); }
        if let .RegExpObject(_, _, f) = self.kindAt(this.asRef) {
            for c in self.units(f) {
                if Int64(from: c) == letter { return .Ok(Value.bool(true)); }
            }
            return .Ok(Value.bool(false));
        }
        if this.same(self.intrinsics.regexpProto) { return .Ok(Value.undefined); }
        self.throwType("RegExp.prototype.\(name) getter called on incompatible receiver")
    }

    mutating func regexpNative(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .RegExpCtor => {
                let patternIsRegExp = try self.isRegExp(a0);
                var target = newTarget;
                if target.isUndefined {
                    target = callee;
                    if patternIsRegExp and a1.isUndefined {
                        let pc = try self.getV(a0, self.names.constructor);
                        if pc.same(target) { return .Ok(.Some(a0)); }
                    }
                }
                var p = a0;
                var f = a1;
                if let .Some((src, fl)) = self.regexpParts(a0) {
                    p = src;
                    if a1.isUndefined { f = fl; }
                } else if patternIsRegExp {
                    p = try self.getV(a0, self.atom("source"));
                    if a1.isUndefined { f = try self.getV(a0, self.atom("flags")); }
                }
                self.pushRoot(p);
                self.pushRoot(f);
                let o = try self.regexpAlloc(target);
                self.pushRoot(o);
                try self.regexpInitialize(o, p, f)
            },
            .RegExpExec => {
                if not self.isRegExpObject(this) { return self.throwType("RegExp.prototype.exec called on incompatible receiver"); }
                let s = try self.toStringValue(a0);
                try self.regexpBuiltinExec(this, s)
            },
            .RegExpTest => {
                if not this.isObject { return self.throwType("RegExp.prototype.test called on non-object"); }
                let s = try self.toStringValue(a0);
                Value.bool(not (try self.regexpExec(this, s)).isNull)
            },
            .RegExpToString => {
                if not this.isObject { return self.throwType("RegExp.prototype.toString called on non-object"); }
                var out: Units = [47];
                for c in try self.toUnits(try self.getV(this, self.atom("source"))) { out.append(c); }
                out.append(47);
                for c in try self.toUnits(try self.getV(this, self.atom("flags"))) { out.append(c); }
                self.newString(out)
            },
            .RegExpCompile => {
                if not self.isRegExpObject(this) { return self.throwType("RegExp.prototype.compile called on incompatible receiver"); }
                var p = a0;
                var f = a1;
                if let .Some((src, fl)) = self.regexpParts(a0) {
                    if not a1.isUndefined { return self.throwType("Cannot supply flags when constructing one RegExp from another"); }
                    p = src;
                    f = fl;
                }
                try self.regexpInitialize(this, p, f)
            },
            .RegExpFlags => {
                if not this.isObject { return self.throwType("RegExp.prototype.flags getter called on non-object"); }
                var out: Units = [];
                for (letter, name) in [(100, "hasIndices"), (103, "global"), (105, "ignoreCase"), (109, "multiline"),
                                       (115, "dotAll"), (117, "unicode"), (118, "unicodeSets"), (121, "sticky")] {
                    if self.toBoolean(try self.getV(this, self.atom(name))) { out.append(UInt16(truncating: letter)); }
                }
                self.newString(out)
            },
            .RegExpSource => {
                if not this.isObject { return self.throwType("RegExp.prototype.source getter called on non-object"); }
                if let .Some((src, _)) = self.regexpParts(this) {
                    return .Ok(.Some(self.newString(escapePattern(self.units(src)))));
                }
                if this.same(self.intrinsics.regexpProto) { return .Ok(.Some(self.str("(?:)"))); }
                return self.throwType("RegExp.prototype.source getter called on incompatible receiver");
            },
            .RegExpGlobal => try self.flagGetter(this, 103, "global"),
            .RegExpIgnoreCase => try self.flagGetter(this, 105, "ignoreCase"),
            .RegExpMultiline => try self.flagGetter(this, 109, "multiline"),
            .RegExpDotAll => try self.flagGetter(this, 115, "dotAll"),
            .RegExpUnicode => try self.flagGetter(this, 117, "unicode"),
            .RegExpUnicodeSets => try self.flagGetter(this, 118, "unicodeSets"),
            .RegExpSticky => try self.flagGetter(this, 121, "sticky"),
            .RegExpHasIndices => try self.flagGetter(this, 100, "hasIndices"),
            .RegExpSymbolMatch => {
                if not this.isObject { return self.throwType("RegExp.prototype[Symbol.match] called on non-object"); }
                let s = try self.toStringValue(a0);
                self.pushRoot(s);
                let flags = try self.flagsOf(this);
                if not flags.contains(103) { return .Ok(.Some(try self.regexpExec(this, s))); }
                let fullUnicode = flags.contains(117) or flags.contains(118);
                try self.setV(this, self.names.lastIndex, Value.int(0), true);
                let out = self.newArray([]);
                self.pushRoot(out);
                while true {
                    let result = try self.regexpExec(this, s);
                    if result.isNull { break; }
                    let matched = try self.toStringValue(try self.getIndex(result, 0));
                    self.appendElement(out.asRef, matched);
                    if self.units(matched).count == 0 { try self.advanceLastIndex(this, self.units(s), fullUnicode); }
                }
                if self.heap.objects(out.asRef).length == 0 { Value.nullValue } else { out }
            },
            .RegExpSymbolMatchAll => {
                if not this.isObject { return self.throwType("RegExp.prototype[Symbol.matchAll] called on non-object"); }
                let s = try self.toStringValue(a0);
                self.pushRoot(s);
                let regexpCtor = self.ownValue(self.intrinsics.regexpProto.asRef, self.names.constructor);
                let c = try self.speciesConstructor(this, regexpCtor);
                let flags = try self.toStringValue(try self.getV(this, self.atom("flags")));
                self.pushRoot(flags);
                let matcher = try self.construct(c, [this, flags], c);
                self.pushRoot(matcher);
                let li = toLengthOf(try self.toNumber(try self.getV(this, self.names.lastIndex)));
                try self.setV(matcher, self.names.lastIndex, Value.int(li), true);
                let fu = self.units(flags);
                self.newObjectKind(self.intrinsics.regexpStringIteratorProto,
                    .RegExpStringIterator(regexp: matcher, string: s, global: fu.contains(103),
                                          unicode: fu.contains(117) or fu.contains(118), done: false))
            },
            .RegExpStringIteratorNext => {
                if not this.isObject { return self.throwType("next called on incompatible receiver"); }
                let (rx, s, global, unicode, done) = match self.kindAt(this.asRef) {
                    .RegExpStringIterator(a, b, c, d, e) => (a, b, c, d, e),
                    _ => return self.throwType("next called on incompatible receiver")
                };
                if done { return .Ok(.Some(self.iterResult(Value.undefined, true))); }
                let m = try self.regexpExec(rx, s);
                if m.isNull {
                    self.heap.objects(this.asRef).kind = .RegExpStringIterator(regexp: rx, string: s, global: global, unicode: unicode, done: true);
                    return .Ok(.Some(self.iterResult(Value.undefined, true)));
                }
                if global {
                    self.pushRoot(m);
                    let matched = try self.toUnits(try self.getIndex(m, 0));
                    if matched.count == 0 { try self.advanceLastIndex(rx, self.units(s), unicode); }
                } else {
                    self.heap.objects(this.asRef).kind = .RegExpStringIterator(regexp: rx, string: s, global: global, unicode: unicode, done: true);
                }
                self.iterResult(m, false)
            },
            .RegExpSymbolReplace => try self.regexpReplace(this, a0, a1),
            .RegExpSymbolSearch => {
                if not this.isObject { return self.throwType("RegExp.prototype[Symbol.search] called on non-object"); }
                let s = try self.toStringValue(a0);
                self.pushRoot(s);
                let previous = try self.getV(this, self.names.lastIndex);
                if not self.sameValue(previous, Value.int(0)) { try self.setV(this, self.names.lastIndex, Value.int(0), true); }
                let result = try self.regexpExec(this, s);
                self.pushRoot(result);
                let current = try self.getV(this, self.names.lastIndex);
                if not self.sameValue(current, previous) { try self.setV(this, self.names.lastIndex, previous, true); }
                if result.isNull { Value.int(-1) } else { try self.getV(result, self.atom("index")) }
            },
            .RegExpSymbolSplit => try self.regexpSplit(this, a0, a1),
            .StringMatch or .StringSearch => {
                if this.isNullish { return self.throwType("String.prototype method called on null or undefined"); }
                let isMatch = n == NativeFn.StringMatch;
                let sym = if isMatch { WellKnown.matchSym } else { WellKnown.search };
                if not a0.isNullish {
                    let m = try self.getMethod(a0, symbolKey(sym));
                    if not m.isUndefined { return .Ok(.Some(try self.callValue(m, a0, [this]))); }
                }
                let s = try self.toStringValue(this);
                self.pushRoot(s);
                let rx = try self.regexpCreate(a0, Value.undefined);
                self.pushRoot(rx);
                let f = try self.getV(rx, symbolKey(sym));
                try self.callValue(f, rx, [s])
            },
            .StringMatchAll => {
                if this.isNullish { return self.throwType("String.prototype.matchAll called on null or undefined"); }
                if not a0.isNullish {
                    if try self.isRegExp(a0) {
                        let flags = try self.getV(a0, self.atom("flags"));
                        if flags.isNullish { return self.throwType("flags is null or undefined"); }
                        if not (try self.toUnits(flags)).contains(103) {
                            return self.throwType("String.prototype.matchAll called with a non-global RegExp argument");
                        }
                    }
                    let m = try self.getMethod(a0, symbolKey(WellKnown.matchAll));
                    if not m.isUndefined { return .Ok(.Some(try self.callValue(m, a0, [this]))); }
                }
                let s = try self.toStringValue(this);
                self.pushRoot(s);
                let g = self.str("g");
                let rx = try self.regexpCreate(a0, g);
                self.pushRoot(rx);
                let f = try self.getV(rx, symbolKey(WellKnown.matchAll));
                try self.callValue(f, rx, [s])
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// The original source and flags of a RegExp object.
    func regexpParts(v: Value) -> Optional[(Value, Value)] {
        if not v.isObject { return .None; }
        match self.kindAt(v.asRef) {
            .RegExpObject(_, s, f) => .Some((s, f)),
            _ => .None
        }
    }

    /// RegExp.prototype[@@replace].
    mutating func regexpReplace(rx: Value, string: Value, replaceValue: Value) -> Result[Value, Value] {
        if not rx.isObject { return self.throwType("RegExp.prototype[Symbol.replace] called on non-object"); }
        let mark = self.roots.count;
        let sv = try self.toStringValue(string);
        self.pushRoot(sv);
        let s = self.units(sv);
        let functional = self.isCallable(replaceValue);
        var template: Units = [];
        if not functional { template = try self.toUnits(replaceValue); }
        let flags = try self.flagsOf(rx);
        let global = flags.contains(103);
        let fullUnicode = flags.contains(117) or flags.contains(118);
        if global { try self.setV(rx, self.names.lastIndex, Value.int(0), true); }
        var results: [Value] = [];
        while true {
            let result = try self.regexpExec(rx, sv);
            if result.isNull { break; }
            self.pushRoot(result);
            results.append(result);
            if not global { break; }
            let matched = try self.toUnits(try self.getIndex(result, 0));
            if matched.count == 0 { try self.advanceLastIndex(rx, s, fullUnicode); }
        }
        var out: Units = [];
        var nextSource: Int64 = 0;
        for result in results {
            let count = try self.lengthOf(result);
            let nCaptures = if count > 1 { count - 1 } else { 0 };
            let matched = try self.toStringValue(try self.getIndex(result, 0));
            self.pushRoot(matched);
            let matchLength = self.units(matched).count;
            let posValue = try self.toIntegerOrInfinity(try self.getV(result, self.atom("index")));
            let position = if posValue < 0.0 { 0 } else if posValue > Float64(from: s.count) { s.count } else { posValue.toInt64() ?? 0 };
            var captures: [Value] = [];
            for i in 1..<(nCaptures + 1) {
                var c = try self.getIndex(result, i);
                if not c.isUndefined { c = try self.toStringValue(c); }
                self.pushRoot(c);
                captures.append(c);
            }
            var named = try self.getV(result, self.atom("groups"));
            var replacement: Units = [];
            if functional {
                var callArgs: [Value] = [matched];
                for c in captures { callArgs.append(c); }
                callArgs.append(Value.int(position));
                callArgs.append(sv);
                if not named.isUndefined { callArgs.append(named); }
                let v = try self.callValue(replaceValue, Value.undefined, callArgs);
                replacement = try self.toUnits(v);
            } else {
                if not named.isUndefined {
                    named = try self.toObject(named);
                    self.pushRoot(named);
                }
                replacement = try self.getSubstitution(self.units(matched), s, position, captures, named, template);
            }
            if position >= nextSource {
                for i in nextSource..<position { out.append(s(i)); }
                for c in replacement { out.append(c); }
                nextSource = position + matchLength;
            }
        }
        for i in nextSource..<s.count { out.append(s(i)); }
        self.truncateRoots(mark);
        .Ok(self.newString(out))
    }

    /// GetSubstitution with captures and named groups.
    mutating func getSubstitution(matched: Units, s: Units, position: Int64, captures: Array[Value], named: Value, template: Units) -> Result[Units, Value] {
        var out: Units = [];
        let m = captures.count;
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
                i = i + 2;
            } else if d == 38 {
                for x in matched { out.append(x); }
                i = i + 2;
            } else if d == 96 {
                for j in 0..<position { out.append(s(j)); }
                i = i + 2;
            } else if d == 39 {
                let tail = position + matched.count;
                for j in tail..<s.count { out.append(s(j)); }
                i = i + 2;
            } else if d >= 48 and d <= 57 {
                // $n or $nn: the longest valid capture number.
                let one = d - 48;
                let e = if i + 2 < template.count { Int64(from: template(i + 2)) } else { -1 };
                let two = if e >= 48 and e <= 57 { one * 10 + e - 48 } else { -1 };
                if two >= 1 and two <= m {
                    let v = captures(two - 1);
                    if not v.isUndefined { for x in self.units(v) { out.append(x); } }
                    i = i + 3;
                } else if one >= 1 and one <= m {
                    let v = captures(one - 1);
                    if not v.isUndefined { for x in self.units(v) { out.append(x); } }
                    i = i + 2;
                } else {
                    out.append(36);
                    i = i + 1;
                }
            } else if d == 60 and not named.isUndefined {
                var close = i + 2;
                while close < template.count and Int64(from: template(close)) != 62 { close = close + 1; }
                if close >= template.count {
                    out.append(36);
                    i = i + 1;
                } else {
                    let name = unitsSlice(template, i + 2, close);
                    let v = try self.getV(named, self.atomOfUnits(name));
                    if not v.isUndefined { for x in try self.toUnits(v) { out.append(x); } }
                    i = close + 1;
                }
            } else {
                out.append(36);
                i = i + 1;
            }
        }
        .Ok(out)
    }

    /// RegExp.prototype[@@split].
    mutating func regexpSplit(rx: Value, string: Value, limit: Value) -> Result[Value, Value] {
        if not rx.isObject { return self.throwType("RegExp.prototype[Symbol.split] called on non-object"); }
        let mark = self.roots.count;
        let sv = try self.toStringValue(string);
        self.pushRoot(sv);
        let s = self.units(sv);
        let regexpCtor = self.ownValue(self.intrinsics.regexpProto.asRef, self.names.constructor);
        let c = try self.speciesConstructor(rx, regexpCtor);
        var flags = try self.flagsOf(rx);
        let unicodeMatching = flags.contains(117) or flags.contains(118);
        if not flags.contains(121) { flags.append(121); }
        let newFlags = self.newString(flags);
        self.pushRoot(newFlags);
        let splitter = try self.construct(c, [rx, newFlags], c);
        self.pushRoot(splitter);
        let out = self.newArray([]);
        self.pushRoot(out);
        let lim = if limit.isUndefined { 4294967295 } else { toUint32(try self.toNumber(limit)) };
        if lim == 0 {
            self.truncateRoots(mark);
            return .Ok(out);
        }
        let size = s.count;
        if size == 0 {
            let z = try self.regexpExec(splitter, sv);
            if z.isNull { self.appendElement(out.asRef, sv); }
            self.truncateRoots(mark);
            return .Ok(out);
        }
        var p: Int64 = 0;
        var q: Int64 = 0;
        while q < size {
            try self.setV(splitter, self.names.lastIndex, Value.int(q), true);
            let z = try self.regexpExec(splitter, sv);
            if z.isNull {
                q = advanceStringIndex(s, q, unicodeMatching);
                continue;
            }
            self.pushRoot(z);
            let li = toLengthOf(try self.toNumber(try self.getV(splitter, self.names.lastIndex)));
            let e = if li < size { li } else { size };
            if e == p {
                q = advanceStringIndex(s, q, unicodeMatching);
                continue;
            }
            self.appendElement(out.asRef, self.newString(unitsSlice(s, p, q)));
            if self.heap.objects(out.asRef).length == lim {
                self.truncateRoots(mark);
                return .Ok(out);
            }
            p = e;
            let count = try self.lengthOf(z);
            for i in 1..<(if count > 1 { count } else { 1 }) {
                let cap = try self.getIndex(z, i);
                self.appendElement(out.asRef, cap);
                if self.heap.objects(out.asRef).length == lim {
                    self.truncateRoots(mark);
                    return .Ok(out);
                }
            }
            q = p;
        }
        self.appendElement(out.asRef, self.newString(unitsSlice(s, p, size)));
        self.truncateRoots(mark);
        .Ok(out)
    }
}

/// ToLength on a number.
func toLengthOf(x: Float64) -> Int64 {
    if x.isNaN or x <= 0.0 { return 0; }
    if x >= 9007199254740991.0 { return 9007199254740991; }
    x.trunc().toInt64() ?? 0
}

func advanceStringIndex(s: Units, index: Int64, unicode: Bool) -> Int64 {
    if not unicode or index + 1 >= s.count { return index + 1; }
    let c = Int64(from: s(index));
    let d = Int64(from: s(index + 1));
    if c >= 0xD800 and c <= 0xDBFF and d >= 0xDC00 and d <= 0xDFFF { return index + 2; }
    index + 1
}

/// EscapeRegExpPattern: a source that reads back as the same literal.
func escapePattern(src: Units) -> Units {
    if src.count == 0 { return unitsOf("(?:)"); }
    var out: Units = [];
    var inClass = false;
    var i: Int64 = 0;
    while i < src.count {
        let c = Int64(from: src(i));
        if c == 92 and i + 1 < src.count {
            out.append(src(i));
            let d = Int64(from: src(i + 1));
            if d == 10 { for x in unitsOf("n") { out.append(x); } }
            else if d == 13 { for x in unitsOf("r") { out.append(x); } }
            else if d == 0x2028 { for x in unitsOf("u2028") { out.append(x); } }
            else if d == 0x2029 { for x in unitsOf("u2029") { out.append(x); } }
            else { out.append(src(i + 1)); }
            i = i + 2;
            continue;
        }
        if c == 91 { inClass = true; }
        if c == 93 { inClass = false; }
        if c == 47 and not inClass {
            out.append(92);
            out.append(47);
        } else if c == 10 {
            for x in unitsOf("\\n") { out.append(x); }
        } else if c == 13 {
            for x in unitsOf("\\r") { out.append(x); }
        } else if c == 0x2028 {
            for x in unitsOf("\\u2028") { out.append(x); }
        } else if c == 0x2029 {
            for x in unitsOf("\\u2029") { out.append(x); }
        } else {
            out.append(src(i));
        }
        i = i + 1;
    }
    out
}
