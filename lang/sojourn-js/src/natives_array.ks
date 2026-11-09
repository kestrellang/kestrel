module js

// Array built-ins. They work on any array-like `this`, through [[Get]]
// and [[Set]] by index, as the spec has them.

extend VM {
    mutating func arrayNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .ArrayCtor => {
                let proto = try self.prototypeFor(newTarget, self.intrinsics.arrayProto);
                let a = self.newObjectKind(proto, .ArrayObject);
                if args.count == 1 and a0.isNumber {
                    let len = toUint32(a0.asNumber);
                    if Float64(from: len) != a0.asNumber { return .Err(self.makeError(.RangeError, "Invalid array length")); }
                    if len > 10000000 { return .Err(self.makeError(.RangeError, "sparse arrays aren't supported yet")); }
                    self.setArrayLength(a.asRef, len);
                } else {
                    self.heap.objects(a.asRef).elements = args;
                }
                a
            },
            .ArrayIsArray => Value.bool(self.isArray(a0)),
            .ArrayOf => self.newArray(args),
            .ArrayFrom => {
                let mapFn = a1;
                if not mapFn.isUndefined { let _ = try self.requireCallable(mapFn); }
                let usesIterator = not (try self.getV(a0, symbolKey(WellKnown.iterator))).isNullish;
                var items: [Value] = [];
                if usesIterator {
                    items = try self.iterateToArray(a0);
                } else {
                    let o = try self.toObject(a0);
                    let len = try self.lengthOf(o);
                    for i in 0..<len { items.append(try self.getIndex(o, i)); }
                }
                let out = self.newArray([]);
                self.pushRoot(out);
                for i in 0..<items.count {
                    var v = items(i);
                    if not mapFn.isUndefined {
                        match self.callValue(mapFn, arg(args, 2), [v, Value.int(i)]) {
                            .Ok(x) => v = x,
                            .Err(e) => {
                                self.popRoots(1);
                                return .Err(e);
                            }
                        }
                    }
                    self.heap.objects(out.asRef).elements.append(v);
                }
                self.popRoots(1);
                out
            },
            .ArrayPush => {
                let o = try self.toObject(this);
                if self.isArray(o) and self.heap.objects(o.asRef).extensible {
                    for a in args { self.heap.objects(o.asRef).elements.append(a); }
                    return .Ok(.Some(Value.int(self.heap.objects(o.asRef).elements.count)));
                }
                var len = try self.lengthOf(o);
                for a in args {
                    try self.setIndex(o, len, a);
                    len = len + 1;
                }
                try self.setLength(o, len);
                Value.int(len)
            },
            .ArrayPop => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                if len == 0 {
                    try self.setLength(o, 0);
                    return .Ok(.Some(Value.undefined));
                }
                let v = try self.getIndex(o, len - 1);
                let k = self.internIndex(len - 1);
                let _ = self.deleteOwn(o.asRef, k);
                try self.setLength(o, len - 1);
                v
            },
            .ArrayShift => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                if len == 0 {
                    try self.setLength(o, 0);
                    return .Ok(.Some(Value.undefined));
                }
                let first = try self.getIndex(o, 0);
                if self.isArray(o) {
                    let _ = self.heap.objects(o.asRef).elements.remove(at: 0);
                    return .Ok(.Some(if first.isEmpty { Value.undefined } else { first }));
                }
                for i in 1..<len {
                    if self.hasIndex(o, i) {
                        try self.setIndex(o, i - 1, try self.getIndex(o, i));
                    } else {
                        let _ = self.deleteOwn(o.asRef, self.internIndex(i - 1));
                    }
                }
                let _ = self.deleteOwn(o.asRef, self.internIndex(len - 1));
                try self.setLength(o, len - 1);
                first
            },
            .ArrayUnshift => {
                let o = try self.toObject(this);
                if self.isArray(o) {
                    var els = args;
                    for e in self.heap.objects(o.asRef).elements { els.append(e); }
                    self.heap.objects(o.asRef).elements = els;
                    return .Ok(.Some(Value.int(els.count)));
                }
                let len = try self.lengthOf(o);
                var i = len;
                while i > 0 {
                    i = i - 1;
                    if self.hasIndex(o, i) {
                        try self.setIndex(o, i + args.count, try self.getIndex(o, i));
                    } else {
                        let _ = self.deleteOwn(o.asRef, self.internIndex(i + args.count));
                    }
                }
                for j in 0..<args.count { try self.setIndex(o, j, args(j)); }
                try self.setLength(o, len + args.count);
                Value.int(len + args.count)
            },
            .ArraySlice => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let start = relativeIndex(try self.toIntegerOrInfinity(a0), len);
                let end = if a1.isUndefined { len } else { relativeIndex(try self.toIntegerOrInfinity(a1), len) };
                var out: [Value] = [];
                var i = start;
                while i < end {
                    out.append(if self.hasIndex(o, i) { try self.getIndex(o, i) } else { Value.empty });
                    i = i + 1;
                }
                self.newArray(out)
            },
            .ArraySplice => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let start = relativeIndex(try self.toIntegerOrInfinity(a0), len);
                var deleteCount: Int64 = 0;
                if args.count == 1 {
                    deleteCount = len - start;
                } else if args.count > 1 {
                    let dc = try self.toIntegerOrInfinity(a1);
                    let maxDc = Float64(from: len - start);
                    let clamped = if dc < 0.0 { 0.0 } else if dc > maxDc { maxDc } else { dc };
                    deleteCount = clamped.toInt64() ?? 0;
                }
                var items: [Value] = [];
                for i in 2..<args.count { items.append(args(i)); }
                var all: [Value] = [];
                for i in 0..<len { all.append(if self.hasIndex(o, i) { try self.getIndex(o, i) } else { Value.empty }); }
                var removed: [Value] = [];
                for i in start..<(start + deleteCount) { removed.append(all(i)); }
                var result: [Value] = [];
                for i in 0..<start { result.append(all(i)); }
                for x in items { result.append(x); }
                for i in (start + deleteCount)..<len { result.append(all(i)); }
                if self.isArray(o) {
                    self.heap.objects(o.asRef).elements = result;
                } else {
                    for i in 0..<result.count { try self.setIndex(o, i, result(i)); }
                    for i in result.count..<len { let _ = self.deleteOwn(o.asRef, self.internIndex(i)); }
                    try self.setLength(o, result.count);
                }
                self.newArray(removed)
            },
            .ArrayConcat => {
                let o = try self.toObject(this);
                var out: [Value] = [];
                var parts: [Value] = [o];
                for a in args { parts.append(a); }
                for p in parts {
                    var spreadable = self.isArray(p);
                    if p.isObject {
                        let s = try self.getFrom(p, symbolKey(WellKnown.isConcatSpreadable), p);
                        if not s.isUndefined { spreadable = self.toBoolean(s); }
                    }
                    if spreadable {
                        let len = try self.lengthOf(p);
                        for i in 0..<len {
                            out.append(if self.hasIndex(p, i) { try self.getIndex(p, i) } else { Value.empty });
                        }
                    } else {
                        out.append(p);
                    }
                }
                self.newArray(out)
            },
            .ArrayJoin => {
                let o = try self.toObject(this);
                let sep = if a0.isUndefined { unitsOf(",") } else { try self.toUnits(a0) };
                try self.join(o, sep)
            },
            .ArrayToString => {
                let o = try self.toObject(this);
                let join = try self.getFrom(o, self.atom("join"), o);
                if self.isCallable(join) { try self.callValue(join, o, []) } else { try self.objectToString(o) }
            },
            .ArrayIndexOf => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let from = if args.count > 1 { relativeIndex(try self.toIntegerOrInfinity(a1), len) } else { 0 };
                var found: Int64 = -1;
                for i in from..<len {
                    if not self.hasIndex(o, i) { continue; }
                    if self.strictEquals(try self.getIndex(o, i), a0) {
                        found = i;
                        break;
                    }
                }
                Value.int(found)
            },
            .ArrayLastIndexOf => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                var i = len - 1;
                if args.count > 1 {
                    let f = try self.toIntegerOrInfinity(a1);
                    i = if f < 0.0 { len + (f.toInt64() ?? (0 - len - 1)) } else if f >= Float64(from: len) { len - 1 } else { f.toInt64() ?? 0 };
                }
                var found: Int64 = -1;
                while i >= 0 {
                    if self.hasIndex(o, i) {
                        if self.strictEquals(try self.getIndex(o, i), a0) {
                            found = i;
                            break;
                        }
                    }
                    i = i - 1;
                }
                Value.int(found)
            },
            .ArrayIncludes => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let from = if args.count > 1 { relativeIndex(try self.toIntegerOrInfinity(a1), len) } else { 0 };
                var found = false;
                for i in from..<len {
                    if self.sameValueZero(try self.getIndex(o, i), a0) {
                        found = true;
                        break;
                    }
                }
                Value.bool(found)
            },
            .ArrayForEach => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let f = try self.requireCallable(a0);
                for i in 0..<len {
                    if self.hasIndex(o, i) {
                        let v = try self.getIndex(o, i);
                        let _ = try self.callValue(f, a1, [v, Value.int(i), o]);
                    }
                }
                Value.undefined
            },
            .ArrayMap or .ArrayFilter => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let f = try self.requireCallable(a0);
                let out = self.newArray([]);
                self.pushRoot(out);
                let isMap = n == NativeFn.ArrayMap;
                if isMap { self.setArrayLength(out.asRef, len); }
                for i in 0..<len {
                    if not self.hasIndex(o, i) { continue; }
                    let step = self.mapStep(o, f, a1, i);
                    match step {
                        .Ok((v, x)) => {
                            if isMap {
                                self.heap.objects(out.asRef).elements(i) = x;
                            } else if self.toBoolean(x) {
                                self.heap.objects(out.asRef).elements.append(v);
                            }
                        },
                        .Err(e) => {
                            self.popRoots(1);
                            return .Err(e);
                        }
                    }
                }
                self.popRoots(1);
                out
            },
            .ArrayReduce or .ArrayReduceRight => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let f = try self.requireCallable(a0);
                let right = n == NativeFn.ArrayReduceRight;
                var i: Int64 = if right { len - 1 } else { 0 };
                let stepBy: Int64 = if right { -1 } else { 1 };
                var acc = Value.empty;
                if args.count > 1 {
                    acc = a1;
                } else {
                    while i >= 0 and i < len and not self.hasIndex(o, i) { i = i + stepBy; }
                    if i < 0 or i >= len { return self.throwType("Reduce of empty array with no initial value"); }
                    acc = try self.getIndex(o, i);
                    i = i + stepBy;
                }
                while i >= 0 and i < len {
                    if self.hasIndex(o, i) {
                        let v = try self.getIndex(o, i);
                        self.pushRoot(acc);
                        let next = self.callValue(f, Value.undefined, [acc, v, Value.int(i), o]);
                        self.popRoots(1);
                        acc = try next;
                    }
                    i = i + stepBy;
                }
                acc
            },
            .ArraySome or .ArrayEvery => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let f = try self.requireCallable(a0);
                let isSome = n == NativeFn.ArraySome;
                var result = not isSome;
                for i in 0..<len {
                    if not self.hasIndex(o, i) { continue; }
                    let v = try self.getIndex(o, i);
                    let t = self.toBoolean(try self.callValue(f, a1, [v, Value.int(i), o]));
                    if isSome and t {
                        result = true;
                        break;
                    }
                    if not isSome and not t {
                        result = false;
                        break;
                    }
                }
                Value.bool(result)
            },
            .ArrayFind or .ArrayFindIndex or .ArrayFindLast or .ArrayFindLastIndex => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let f = try self.requireCallable(a0);
                let last = n == NativeFn.ArrayFindLast or n == NativeFn.ArrayFindLastIndex;
                let wantIndex = n == NativeFn.ArrayFindIndex or n == NativeFn.ArrayFindLastIndex;
                var result = if wantIndex { Value.int(-1) } else { Value.undefined };
                for j in 0..<len {
                    let i = if last { len - 1 - j } else { j };
                    let v = try self.getIndex(o, i);
                    if self.toBoolean(try self.callValue(f, a1, [v, Value.int(i), o])) {
                        result = if wantIndex { Value.int(i) } else { v };
                        break;
                    }
                }
                result
            },
            .ArrayReverse => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                var lo: Int64 = 0;
                var hi = len - 1;
                while lo < hi {
                    let hasLo = self.hasIndex(o, lo);
                    let hasHi = self.hasIndex(o, hi);
                    let vl = if hasLo { try self.getIndex(o, lo) } else { Value.undefined };
                    let vh = if hasHi { try self.getIndex(o, hi) } else { Value.undefined };
                    if hasHi { try self.setIndex(o, lo, vh); } else { let _ = self.deleteOwn(o.asRef, self.internIndex(lo)); }
                    if hasLo { try self.setIndex(o, hi, vl); } else { let _ = self.deleteOwn(o.asRef, self.internIndex(hi)); }
                    lo = lo + 1;
                    hi = hi - 1;
                }
                o
            },
            .ArraySort => {
                if not a0.isUndefined { let _ = try self.requireCallable(a0); }
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                var items: [Value] = [];
                var holes: Int64 = 0;
                for i in 0..<len {
                    if self.hasIndex(o, i) { items.append(try self.getIndex(o, i)); } else { holes = holes + 1; }
                }
                let holder = self.newArray(items);
                self.pushRoot(holder);
                let sorted = self.mergeSort(items, a0);
                self.popRoots(1);
                let s = try sorted;
                for i in 0..<s.count { try self.setIndex(o, i, s(i)); }
                for i in s.count..<len { let _ = self.deleteOwn(o.asRef, self.internIndex(i)); }
                o
            },
            .ArrayFill => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let start = relativeIndex(try self.toIntegerOrInfinity(a1), len);
                let e = arg(args, 2);
                let end = if e.isUndefined { len } else { relativeIndex(try self.toIntegerOrInfinity(e), len) };
                var i = start;
                while i < end {
                    try self.setIndex(o, i, a0);
                    i = i + 1;
                }
                o
            },
            .ArrayKeys => self.arrayIterator(try self.toObject(this), 0),
            .ArrayValues => self.arrayIterator(try self.toObject(this), 1),
            .ArrayEntries => self.arrayIterator(try self.toObject(this), 2),
            .ArrayAt => {
                let o = try self.toObject(this);
                let len = try self.lengthOf(o);
                let rel = try self.toIntegerOrInfinity(a0);
                let k = if rel >= 0.0 { rel } else { Float64(from: len) + rel };
                if k < 0.0 or k >= Float64(from: len) { Value.undefined } else { try self.getIndex(o, k.toInt64() ?? 0) }
            },
            .ArrayFlat => {
                let o = try self.toObject(this);
                let depth = if a0.isUndefined { 1.0 } else { try self.toIntegerOrInfinity(a0) };
                var out: [Value] = [];
                try self.flatten(o, depth, out);
                self.newArray(out)
            },
            .ArrayIteratorNext => {
                if not this.isObject { return self.throwType("next called on incompatible receiver"); }
                let (target, index, kind) = match self.kindAt(this.asRef) {
                    .ArrayIterator(t, i, k) => (t, i, k),
                    _ => return self.throwType("next called on incompatible receiver")
                };
                if target.isUndefined { return .Ok(.Some(self.iterResult(Value.undefined, true))); }
                let len = try self.lengthOf(target);
                if index >= len {
                    self.heap.objects(this.asRef).kind = .ArrayIterator(target: Value.undefined, index: index, kind: kind);
                    return .Ok(.Some(self.iterResult(Value.undefined, true)));
                }
                self.heap.objects(this.asRef).kind = .ArrayIterator(target: target, index: index + 1, kind: kind);
                let v = if kind == 0 { Value.int(index) } else { try self.getIndex(target, index) };
                let out = if kind == 2 { self.newArray([Value.int(index), v]) } else { v };
                self.iterResult(out, false)
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// One map/filter callback call: the element and the callback's result.
    mutating func mapStep(o: Value, f: Value, this: Value, i: Int64) -> Result[(Value, Value), Value] {
        let v = try self.getIndex(o, i);
        let x = try self.callValue(f, this, [v, Value.int(i), o]);
        .Ok((v, x))
    }

    mutating func arrayIterator(o: Value, kind: Int64) -> Value {
        self.newObjectKind(self.intrinsics.arrayIteratorProto, .ArrayIterator(target: o, index: 0, kind: kind))
    }

    mutating func join(o: Value, sep: Units) -> Result[Value, Value] {
        let len = try self.lengthOf(o);
        var out: Units = [];
        for i in 0..<len {
            if i > 0 { for u in sep { out.append(u); } }
            let v = try self.getIndex(o, i);
            if not v.isNullish and not v.isEmpty {
                for u in try self.toUnits(v) { out.append(u); }
            }
        }
        .Ok(self.newString(out))
    }

    mutating func flatten(o: Value, depth: Float64, mutating out: Array[Value]) -> Result[(), Value] {
        let len = try self.lengthOf(o);
        for i in 0..<len {
            if not self.hasIndex(o, i) { continue; }
            let v = try self.getIndex(o, i);
            if depth > 0.0 and self.isArray(v) {
                try self.flatten(v, depth - 1.0, out);
            } else {
                out.append(v);
            }
        }
        .Ok(())
    }

    /// A stable merge sort with the comparator `cmp` (or string order),
    /// undefined values last.
    mutating func mergeSort(items: Array[Value], cmp: Value) -> Result[Array[Value], Value] {
        var defined: [Value] = [];
        var undefinedCount: Int64 = 0;
        for v in items {
            if v.isUndefined { undefinedCount = undefinedCount + 1; } else { defined.append(v); }
        }
        var work = defined;
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
                    if try self.sortLess(work(j), work(i), cmp) {
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
        for _ in 0..<undefinedCount { work.append(Value.undefined); }
        .Ok(work)
    }

    mutating func sortLess(a: Value, b: Value, cmp: Value) -> Result[Bool, Value] {
        if not cmp.isUndefined {
            let r = try self.toNumber(try self.callValue(cmp, Value.undefined, [a, b]));
            return .Ok(r < 0.0);
        }
        let x = try self.toUnits(a);
        let y = try self.toUnits(b);
        .Ok(unitsLess(x, y))
    }
}
