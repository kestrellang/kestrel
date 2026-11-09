module js

// The Array.prototype methods that build a new array through
// ArraySpeciesCreate (concat, filter, flat, flatMap, map, slice, splice),
// written step for step from the spec: the result may be any object a
// subclass's @@species makes, so elements go in with
// CreateDataPropertyOrThrow.

/// 2^53 - 1, the largest length an array-like may have.
func maxSafeLength() -> Int64 { 9007199254740991 }

extend VM {
    /// ArrayCreate: a new array of length `len`.
    mutating func arrayCreate(len: Int64) -> Result[Value, Value] {
        if len > 4294967295 { return .Err(self.makeError(.RangeError, "Invalid array length")); }
        let a = self.newArray([]);
        self.setArrayLength(a.asRef, len);
        .Ok(a)
    }

    /// ArraySpeciesCreate.
    mutating func arraySpeciesCreate(o: Value, len: Int64) -> Result[Value, Value] {
        if not (try self.isArrayV(o)) { return self.arrayCreate(len); }
        var c = try self.getV(o, self.names.constructor);
        if c.isObject {
            c = try self.getV(c, symbolKey(WellKnown.species));
            if c.isNull { c = Value.undefined; }
        }
        if c.isUndefined { return self.arrayCreate(len); }
        if not self.isConstructor(c) { return self.throwType("object.constructor[Symbol.species] is not a constructor"); }
        self.construct(c, [Value.int(len)], c)
    }

    /// CreateDataPropertyOrThrow for index `i`.
    mutating func createIndexOrThrow(a: Value, i: Int64, v: Value) -> Result[(), Value] {
        // A fresh dense array just grows.
        if self.isSimpleArray(a) and i == self.heap.objects(a.asRef).length and i < 4294967295 {
            self.appendElement(a.asRef, v);
            return .Ok(());
        }
        let d = Descriptor(value: .Some(v), writable: .Some(true), getter: .None, setter: .None, enumerable: .Some(true), configurable: .Some(true));
        if not (try self.defineProperty(a.asRef, self.internIndex(i), d)) { return self.throwType("Cannot add property \(i), object is not extensible"); }
        .Ok(())
    }

    /// DeletePropertyOrThrow for index `i`.
    mutating func deleteIndexOrThrow(o: Value, i: Int64) -> Result[(), Value] {
        if not (try self.deleteOwn(o.asRef, self.internIndex(i))) { return self.throwType("Cannot delete property '\(i)'"); }
        .Ok(())
    }

    /// Runs one of the methods below with its roots dropped afterwards.
    mutating func arraySpeciesNative(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let mark = self.roots.count;
        let r: Result[Value, Value] = match n {
            .ArrayMap => self.arrayMap(this, arg(args, 0), arg(args, 1)),
            .ArrayFilter => self.arrayFilter(this, arg(args, 0), arg(args, 1)),
            .ArraySlice => self.arraySlice(this, arg(args, 0), arg(args, 1)),
            .ArraySplice => self.arraySplice(this, args),
            .ArrayConcat => self.arrayConcat(this, args),
            .ArrayFlat => self.arrayFlat(this, arg(args, 0)),
            .ArrayFlatMap => self.arrayFlatMap(this, arg(args, 0), arg(args, 1)),
            .ArrayFrom => self.arrayFrom(this, arg(args, 0), arg(args, 1), arg(args, 2)),
            .ArrayOf => self.arrayOf(this, args),
            _ => return .Ok(.None)
        };
        self.truncateRoots(mark);
        .Ok(.Some(try r))
    }

    /// `new C(...args)` when `this` is a constructor, else a new array.
    mutating func constructOrArray(c: Value, args: Array[Value], len: Int64) -> Result[Value, Value] {
        if self.isConstructor(c) { return self.construct(c, args, c); }
        self.arrayCreate(len)
    }

    mutating func arrayOf(this: Value, items: Array[Value]) -> Result[Value, Value] {
        let len = items.count;
        let a = try self.constructOrArray(this, [Value.int(len)], len);
        self.pushRoot(a);
        for k in 0..<len { try self.createIndexOrThrow(a, k, items(k)); }
        try self.setLength(a, len);
        .Ok(a)
    }

    mutating func arrayFrom(this: Value, items: Value, mapFn: Value, thisArg: Value) -> Result[Value, Value] {
        let mapping = not mapFn.isUndefined;
        if mapping { let _ = try self.requireCallable(mapFn); }
        let usingIterator = try self.getMethod(items, symbolKey(WellKnown.iterator));
        if not usingIterator.isUndefined {
            let a = try self.constructOrArray(this, [], 0);
            self.pushRoot(a);
            let it = try self.callValue(usingIterator, items, []);
            if not it.isObject { return self.throwType("Result of the Symbol.iterator method is not an object"); }
            self.pushRoot(it);
            let next = try self.getFrom(it, self.names.next, it);
            var k: Int64 = 0;
            while true {
                let step = try self.iteratorStep(it, next);
                let v = match step {
                    .Some(x) => x,
                    .None => break
                };
                // A failing map or define closes the iterator first.
                let r = self.arrayFromPut(a, k, v, mapFn, thisArg);
                if let .Err(e) = r {
                    let _ = self.iteratorClose(it);
                    return .Err(e);
                }
                k = k + 1;
            }
            try self.setLength(a, k);
            return .Ok(a);
        }
        let src = try self.toObject(items);
        self.pushRoot(src);
        let len = try self.lengthOf(src);
        let a = try self.constructOrArray(this, [Value.int(len)], len);
        self.pushRoot(a);
        for k in 0..<len {
            let v = try self.getIndex(src, k);
            try self.arrayFromPut(a, k, v, mapFn, thisArg);
        }
        try self.setLength(a, len);
        .Ok(a)
    }

    /// One Array.from element: mapped if there's a map function.
    mutating func arrayFromPut(a: Value, k: Int64, v: Value, mapFn: Value, thisArg: Value) -> Result[(), Value] {
        let x = if mapFn.isUndefined { v } else { try self.callValue(mapFn, thisArg, [v, Value.int(k)]) };
        self.createIndexOrThrow(a, k, x)
    }

    /// Object.fromEntries: each entry's key and value go on a new object.
    mutating func objectFromEntries(iterable: Value) -> Result[Value, Value] {
        if iterable.isNullish { return self.throwType("\(self.describe(iterable)) is not iterable"); }
        let mark = self.roots.count;
        let out = self.newObject();
        self.pushRoot(out);
        let r = self.objectFromEntriesInto(out, iterable);
        self.truncateRoots(mark);
        try r;
        .Ok(out)
    }

    mutating func objectFromEntriesInto(out: Value, iterable: Value) -> Result[(), Value] {
        let (it, next) = try self.getIterator(iterable);
        self.pushRoot(it);
        while true {
            let step = try self.iteratorStep(it, next);
            let e = match step {
                .Some(x) => x,
                .None => break
            };
            let r = self.addEntry(out, e);
            if let .Err(err) = r {
                let _ = self.iteratorClose(it);
                return .Err(err);
            }
        }
        .Ok(())
    }

    mutating func addEntry(out: Value, e: Value) -> Result[(), Value] {
        if not e.isObject { return self.throwType("Iterator value \(self.describe(e)) is not an entry object"); }
        let kv = try self.getIndex(e, 0);
        let v = try self.getIndex(e, 1);
        let k = try self.toKey(kv);
        self.createIndexOrThrowKey(out, k, v)
    }

    /// CreateDataPropertyOrThrow for any key.
    mutating func createIndexOrThrowKey(o: Value, k: Key, v: Value) -> Result[(), Value] {
        let d = Descriptor(value: .Some(v), writable: .Some(true), getter: .None, setter: .None, enumerable: .Some(true), configurable: .Some(true));
        if not (try self.defineProperty(o.asRef, k, d)) { return self.throwType("Cannot define property \(self.keyText(k))"); }
        .Ok(())
    }

    mutating func arrayMap(this: Value, f: Value, thisArg: Value) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let len = try self.lengthOf(o);
        let fn = try self.requireCallable(f);
        let a = try self.arraySpeciesCreate(o, len);
        self.pushRoot(a);
        for k in 0..<len {
            if not (try self.hasIndex(o, k)) { continue; }
            let v = try self.getIndex(o, k);
            let mapped = try self.callValue(fn, thisArg, [v, Value.int(k), o]);
            try self.createIndexOrThrow(a, k, mapped);
        }
        .Ok(a)
    }

    mutating func arrayFilter(this: Value, f: Value, thisArg: Value) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let len = try self.lengthOf(o);
        let fn = try self.requireCallable(f);
        let a = try self.arraySpeciesCreate(o, 0);
        self.pushRoot(a);
        var to: Int64 = 0;
        for k in 0..<len {
            if not (try self.hasIndex(o, k)) { continue; }
            let v = try self.getIndex(o, k);
            self.pushRoot(v);
            let selected = try self.callValue(fn, thisArg, [v, Value.int(k), o]);
            if self.toBoolean(selected) {
                try self.createIndexOrThrow(a, to, v);
                to = to + 1;
            }
            let _ = self.roots.pop();
        }
        .Ok(a)
    }

    mutating func arraySlice(this: Value, start: Value, end: Value) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let len = try self.lengthOf(o);
        var k = relativeIndex(try self.toIntegerOrInfinity(start), len);
        let fin = if end.isUndefined { len } else { relativeIndex(try self.toIntegerOrInfinity(end), len) };
        let count = if fin > k { fin - k } else { 0 };
        let a = try self.arraySpeciesCreate(o, count);
        self.pushRoot(a);
        var n: Int64 = 0;
        while k < fin {
            if try self.hasIndex(o, k) {
                let v = try self.getIndex(o, k);
                try self.createIndexOrThrow(a, n, v);
            }
            k = k + 1;
            n = n + 1;
        }
        try self.setLength(a, n);
        .Ok(a)
    }

    mutating func arraySplice(this: Value, args: Array[Value]) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let len = try self.lengthOf(o);
        let start = relativeIndex(try self.toIntegerOrInfinity(arg(args, 0)), len);
        var items: [Value] = [];
        for i in 2..<args.count { items.append(args(i)); }
        let itemCount = items.count;
        var dc: Int64 = 0;
        if args.count == 1 {
            dc = len - start;
        } else if args.count > 1 {
            let d = try self.toIntegerOrInfinity(args(1));
            let most = Float64(from: len - start);
            let clamped = if d < 0.0 { 0.0 } else if d > most { most } else { d };
            dc = clamped.toInt64() ?? 0;
        }
        if len + itemCount - dc > maxSafeLength() { return self.throwType("Invalid array length"); }
        let a = try self.arraySpeciesCreate(o, dc);
        self.pushRoot(a);
        for k in 0..<dc {
            let from = start + k;
            if try self.hasIndex(o, from) {
                let v = try self.getIndex(o, from);
                try self.createIndexOrThrow(a, k, v);
            }
        }
        try self.setLength(a, dc);
        if itemCount < dc {
            var k = start;
            while k < len - dc {
                try self.moveIndex(o, k + dc, k + itemCount);
                k = k + 1;
            }
            k = len;
            while k > len - dc + itemCount {
                try self.deleteIndexOrThrow(o, k - 1);
                k = k - 1;
            }
        } else if itemCount > dc {
            var k = len - dc;
            while k > start {
                try self.moveIndex(o, k + dc - 1, k + itemCount - 1);
                k = k - 1;
            }
        }
        for i in 0..<itemCount { try self.setIndex(o, start + i, items(i)); }
        try self.setLength(o, len - dc + itemCount);
        .Ok(a)
    }

    /// Copies index `from` to `to`, or deletes `to` when `from` is a hole.
    mutating func moveIndex(o: Value, from: Int64, to: Int64) -> Result[(), Value] {
        if try self.hasIndex(o, from) {
            let v = try self.getIndex(o, from);
            return self.setIndex(o, to, v);
        }
        self.deleteIndexOrThrow(o, to)
    }

    /// IsConcatSpreadable.
    mutating func isConcatSpreadable(v: Value) -> Result[Bool, Value] {
        if not v.isObject { return .Ok(false); }
        let s = try self.getFrom(v, symbolKey(WellKnown.isConcatSpreadable), v);
        if not s.isUndefined { return .Ok(self.toBoolean(s)); }
        self.isArrayV(v)
    }

    mutating func arrayConcat(this: Value, args: Array[Value]) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let a = try self.arraySpeciesCreate(o, 0);
        self.pushRoot(a);
        var n: Int64 = 0;
        var parts: [Value] = [o];
        for x in args { parts.append(x); }
        for e in parts {
            if try self.isConcatSpreadable(e) {
                let len = try self.lengthOf(e);
                if n + len > maxSafeLength() { return self.throwType("Invalid array length"); }
                for k in 0..<len {
                    if try self.hasIndex(e, k) {
                        let v = try self.getIndex(e, k);
                        try self.createIndexOrThrow(a, n, v);
                    }
                    n = n + 1;
                }
            } else {
                if n >= maxSafeLength() { return self.throwType("Invalid array length"); }
                try self.createIndexOrThrow(a, n, e);
                n = n + 1;
            }
        }
        try self.setLength(a, n);
        .Ok(a)
    }

    mutating func arrayFlat(this: Value, depthArg: Value) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let len = try self.lengthOf(o);
        var depth = 1.0;
        if not depthArg.isUndefined {
            depth = try self.toIntegerOrInfinity(depthArg);
            if depth < 0.0 { depth = 0.0; }
        }
        let a = try self.arraySpeciesCreate(o, 0);
        self.pushRoot(a);
        let _ = try self.flattenInto(a, o, len, 0, depth, Value.undefined, Value.undefined);
        .Ok(a)
    }

    mutating func arrayFlatMap(this: Value, f: Value, thisArg: Value) -> Result[Value, Value] {
        let o = try self.toObject(this);
        self.pushRoot(o);
        let len = try self.lengthOf(o);
        let fn = try self.requireCallable(f);
        let a = try self.arraySpeciesCreate(o, 0);
        self.pushRoot(a);
        let _ = try self.flattenInto(a, o, len, 0, 1.0, fn, thisArg);
        .Ok(a)
    }

    /// FlattenIntoArray: returns the next target index.
    mutating func flattenInto(target: Value, source: Value, sourceLen: Int64, start: Int64, depth: Float64, mapper: Value, thisArg: Value) -> Result[Int64, Value] {
        var t = start;
        for i in 0..<sourceLen {
            if not (try self.hasIndex(source, i)) { continue; }
            var e = try self.getIndex(source, i);
            if not mapper.isUndefined { e = try self.callValue(mapper, thisArg, [e, Value.int(i), source]); }
            let flatten = if depth > 0.0 { try self.isArrayV(e) } else { false };
            if flatten {
                self.pushRoot(e);
                let elen = try self.lengthOf(e);
                t = try self.flattenInto(target, e, elen, t, depth - 1.0, Value.undefined, Value.undefined);
                let _ = self.roots.pop();
            } else {
                if t >= maxSafeLength() { return self.throwType("Invalid array length"); }
                try self.createIndexOrThrow(target, t, e);
                t = t + 1;
            }
        }
        .Ok(t)
    }
}
