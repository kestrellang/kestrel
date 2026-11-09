module js

// The iterator protocol, and for-in enumeration.

extend VM {
    /// `obj[Symbol.iterator]()` and its `next` method.
    mutating func getIterator(obj: Value) -> Result[(Value, Value), Value] {
        let method = try self.getV(obj, symbolKey(WellKnown.iterator));
        if not self.isCallable(method) { return self.throwType("\(self.describe(obj)) is not iterable"); }
        let it = try self.callValue(method, obj, []);
        if not it.isObject { return self.throwType("Result of the Symbol.iterator method is not an object"); }
        let next = try self.getFrom(it, self.names.next, it);
        .Ok((it, next))
    }

    /// The next value, or None when the iterator is done.
    mutating func iteratorStep(it: Value, next: Value) -> Result[Optional[Value], Value] {
        let r = try self.callValue(next, it, []);
        if not r.isObject { return self.throwType("Iterator result \(self.describe(r)) is not an object"); }
        let done = try self.getFrom(r, self.names.done, r);
        if self.toBoolean(done) { return .Ok(.None); }
        let v = try self.getFrom(r, self.names.value, r);
        .Ok(.Some(v))
    }

    mutating func iteratorClose(it: Value) -> Result[(), Value] {
        let ret = try self.getV(it, self.names.returnKey);
        if ret.isNullish { return .Ok(()); }
        let r = try self.callValue(ret, it, []);
        if not r.isObject { return self.throwType("Iterator result is not an object"); }
        .Ok(())
    }

    /// Everything `v` iterates, as an array of values.
    mutating func iterateToArray(v: Value) -> Result[Array[Value], Value] {
        // Plain arrays with the built-in iterator take a shortcut.
        if self.isArray(v) and self.hasDefaultArrayIterator(v) { return .Ok(self.arrayElements(v)); }
        let (it, next) = try self.getIterator(v);
        self.pushRoot(it);
        var out: [Value] = [];
        let holder = self.newArray([]);
        self.pushRoot(holder);
        while true {
            match self.iteratorStep(it, next) {
                .Ok(.Some(x)) => {
                    out.append(x);
                    self.appendElement(holder.asRef, x);
                },
                .Ok(.None) => break,
                .Err(e) => {
                    self.popRoots(2);
                    return .Err(e);
                }
            }
        }
        self.popRoots(2);
        .Ok(out)
    }

    mutating func hasDefaultArrayIterator(v: Value) -> Bool {
        let k = symbolKey(WellKnown.iterator);
        if self.hasOwn(v.asRef, k) { return false; }
        let proto = self.heap.objects(v.asRef).proto;
        if not proto.same(self.intrinsics.arrayProto) { return false; }
        match self.ownProperty(proto.asRef, k) {
            .Some(p) => {
                if not p.value.isObject { return false; }
                match self.kindAt(p.value.asRef) {
                    .NativeFunction(n) => n == NativeFn.ArrayValues,
                    _ => false
                }
            },
            .None => false
        }
    }

    // ------------------------------------------------------------------
    // for-in
    // ------------------------------------------------------------------

    /// The enumerable string keys of `obj` and its prototypes, nearest
    /// first, each once.
    mutating func forInIterator(obj: Value) -> Result[Value, Value] {
        var keys: [Value] = [];
        if not obj.isNullish {
            let o = try self.toObject(obj);
            var seen: [Key] = [];
            var cur = o;
            while cur.isObject {
                for k in self.ownKeys(cur.asRef) {
                    if keyIsSymbol(k) or seen.contains(k) { continue; }
                    seen.append(k);
                    if let .Some(p) = self.ownProperty(cur.asRef, k) {
                        if p.attrs & Attr.enumerable != 0 { keys.append(self.atomValues(k)); }
                    }
                }
                cur = self.heap.objects(cur.asRef).proto;
            }
            return .Ok(self.newObjectKind(Value.nullValue, .KeyIterator(object: o, keys: keys, index: 0)));
        }
        .Ok(self.newObjectKind(Value.nullValue, .KeyIterator(object: Value.undefined, keys: keys, index: 0)))
    }

    /// The next key still present, or None.
    mutating func forInNext(it: Value) -> Optional[Value] {
        while true {
            let (obj, keys, index) = match self.kindAt(it.asRef) {
                .KeyIterator(o, k, i) => (o, k, i),
                _ => return .None
            };
            if index >= keys.count { return .None; }
            self.heap.objects(it.asRef).kind = .KeyIterator(object: obj, keys: keys, index: index + 1);
            let k = keys(index);
            let key = self.atomOfUnits(self.units(k));
            // Keys deleted during the loop are skipped.
            if self.hasProperty(obj, key) { return .Some(k); }
        }
        .None
    }

    /// `{ value, done }`.
    mutating func iterResult(value: Value, done: Bool) -> Value {
        let o = self.newObject();
        self.defineOwn(o.asRef, self.names.value, value, Attr.all);
        self.defineOwn(o.asRef, self.names.done, Value.bool(done), Attr.all);
        o
    }
}
