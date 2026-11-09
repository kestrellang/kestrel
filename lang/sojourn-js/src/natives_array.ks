module js

// Array built-ins. They work on any array-like `this`, through [[Get]]
// and [[Set]] by index, as the spec has them.

extend VM {
    mutating func arrayNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        if let .Some(r) = try self.arraySpeciesNative(n, this, args) { return .Ok(.Some(r)); }
        let r: Value = match n {
            .ArrayCtor => {
                let proto = try self.prototypeFor(newTarget, self.intrinsics.arrayProto);
                let a = self.newObjectKind(proto, .ArrayObject);
                if args.count == 1 and a0.isNumber {
                    let len = toUint32(a0.asNumber);
                    if Float64(from: len) != a0.asNumber { return .Err(self.makeError(.RangeError, "Invalid array length")); }
                    self.setArrayLength(a.asRef, len);
                } else {
                    self.setDense(a.asRef, args);
                }
                a
            },
            .ArrayIsArray => Value.bool(try self.isArrayV(a0)),
            .ArrayPush => {
                let o = try self.toObject(this);
                if self.isSimpleArray(o) {
                    for a in args { self.appendElement(o.asRef, a); }
                    return .Ok(.Some(Value.int(self.heap.objects(o.asRef).length)));
                }
                var len = try self.lengthOf(o);
                if len + args.count > maxSafeLength() { return self.throwType("Pushing \(args.count) elements on an array-like of length \(len) is disallowed"); }
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
                let hp17 = try self.deleteOwn(o.asRef, k);
                let _ = hp17;
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
                if self.isSimpleArray(o) {
                    let _ = self.heap.objects(o.asRef).elements.remove(at: 0);
                    self.heap.objects(o.asRef).length = len - 1;
                    return .Ok(.Some(if first.isEmpty { Value.undefined } else { first }));
                }
                for i in 1..<len {
                    let hp18 = try self.hasIndex(o, i);
                    if hp18 {
                        try self.setIndex(o, i - 1, try self.getIndex(o, i));
                    } else {
                        let hp19 = try self.deleteOwn(o.asRef, self.internIndex(i - 1));
                        let _ = hp19;
                    }
                }
                let hp20 = try self.deleteOwn(o.asRef, self.internIndex(len - 1));
                let _ = hp20;
                try self.setLength(o, len - 1);
                first
            },
            .ArrayUnshift => {
                let o = try self.toObject(this);
                if self.isSimpleArray(o) {
                    var els = args;
                    for e in self.heap.objects(o.asRef).elements { els.append(e); }
                    self.setDense(o.asRef, els);
                    return .Ok(.Some(Value.int(els.count)));
                }
                let len = try self.lengthOf(o);
                if args.count > 0 and len + args.count > maxSafeLength() { return self.throwType("Invalid array length"); }
                var i = if args.count > 0 { len } else { 0 };
                while i > 0 {
                    i = i - 1;
                    let hp21 = try self.hasIndex(o, i);
                    if hp21 {
                        try self.setIndex(o, i + args.count, try self.getIndex(o, i));
                    } else {
                        let hp22 = try self.deleteOwn(o.asRef, self.internIndex(i + args.count));
                        let _ = hp22;
                    }
                }
                for j in 0..<args.count { try self.setIndex(o, j, args(j)); }
                try self.setLength(o, len + args.count);
                Value.int(len + args.count)
            },
            .ArrayToLocaleString => {
                let o = try self.toObject(this);
                self.pushRoot(o);
                let len = try self.lengthOf(o);
                var out: Units = [];
                let key = self.atom("toLocaleString");
                for k in 0..<len {
                    if k > 0 { out.append(44); }
                    let e = try self.getIndex(o, k);
                    if e.isNullish { continue; }
                    let f = try self.getV(e, key);
                    let s = try self.callValue(f, e, []);
                    for u in try self.toUnits(s) { out.append(u); }
                }
                let _ = self.roots.pop();
                self.newString(out)
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
                if len == 0 { return .Ok(.Some(if n == NativeFn.ArrayIncludes { Value.bool(false) } else { Value.int(-1) })); }
                let from = if args.count > 1 { relativeIndex(try self.toIntegerOrInfinity(a1), len) } else { 0 };
                var found: Int64 = -1;
                for i in from..<len {
                    let hp25 = try self.hasIndex(o, i);
                    if not hp25 { continue; }
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
                if len == 0 { return .Ok(.Some(Value.int(-1))); }
                var i = len - 1;
                if args.count > 1 {
                    let f = try self.toIntegerOrInfinity(a1);
                    i = if f < 0.0 { len + (f.toInt64() ?? (0 - len - 1)) } else if f >= Float64(from: len) { len - 1 } else { f.toInt64() ?? 0 };
                }
                var found: Int64 = -1;
                while i >= 0 {
                    let hp26 = try self.hasIndex(o, i);
                    if hp26 {
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
                if len == 0 { return .Ok(.Some(if n == NativeFn.ArrayIncludes { Value.bool(false) } else { Value.int(-1) })); }
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
                    let hp27 = try self.hasIndex(o, i);
                    if hp27 {
                        let v = try self.getIndex(o, i);
                        let _ = try self.callValue(f, a1, [v, Value.int(i), o]);
                    }
                }
                Value.undefined
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
                    while i >= 0 and i < len {
                        if try self.hasIndex(o, i) { break; }
                        i = i + stepBy;
                    }
                    if i < 0 or i >= len { return self.throwType("Reduce of empty array with no initial value"); }
                    acc = try self.getIndex(o, i);
                    i = i + stepBy;
                }
                while i >= 0 and i < len {
                    let hp29 = try self.hasIndex(o, i);
                    if hp29 {
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
                    let hp30 = try self.hasIndex(o, i);
                    if not hp30 { continue; }
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
                    let hp31 = try self.hasIndex(o, lo);
                    let hasLo = hp31;
                    let hp32 = try self.hasIndex(o, hi);
                    let hasHi = hp32;
                    let vl = if hasLo { try self.getIndex(o, lo) } else { Value.undefined };
                    let vh = if hasHi { try self.getIndex(o, hi) } else { Value.undefined };
                    let hp33 = try self.deleteOwn(o.asRef, self.internIndex(lo));
                    if hasHi { try self.setIndex(o, lo, vh); } else { let _ = hp33; }
                    let hp34 = try self.deleteOwn(o.asRef, self.internIndex(hi));
                    if hasLo { try self.setIndex(o, hi, vl); } else { let _ = hp34; }
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
                    let hp35 = try self.hasIndex(o, i);
                    if hp35 { items.append(try self.getIndex(o, i)); } else { holes = holes + 1; }
                }
                let holder = self.newArray(items);
                self.pushRoot(holder);
                let sorted = self.mergeSort(items, a0);
                self.popRoots(1);
                let s = try sorted;
                for i in 0..<s.count { try self.setIndex(o, i, s(i)); }
                for i in s.count..<len { let _ = try self.deleteOwn(o.asRef, self.internIndex(i)); }
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
