module js

// Set.prototype.union and friends (ECMA-262 §24.2.4, "set methods").
//
// `other` is any set-like object: a `size`, a `has` and a `keys`
// (GetSetRecord). Each method walks whichever side the spec picks, by
// index for `this` because callbacks into `other` may change it.

/// What GetSetRecord reads from a set-like object.
struct SetRecord {
    var obj: Value
    var size: Float64
    var has: Value
    var keys: Value
}

/// The set methods, for `setMethod`.
struct SetOp {
    static var union: Int64 { 0 }
    static var intersection: Int64 { 1 }
    static var difference: Int64 { 2 }
    static var symmetricDifference: Int64 { 3 }
    static var isSubsetOf: Int64 { 4 }
    static var isSupersetOf: Int64 { 5 }
    static var isDisjointFrom: Int64 { 6 }
}

extend VM {
    mutating func setupSetMethods() {
        let setCtor = self.ownValue(self.global.asRef, self.atom("Set"));
        let proto = self.ownValue(setCtor.asRef, self.names.prototype);
        for (name, n) in [
            ("union", NativeFn.SetUnion), ("intersection", NativeFn.SetIntersection),
            ("difference", NativeFn.SetDifference), ("symmetricDifference", NativeFn.SetSymmetricDifference),
            ("isSubsetOf", NativeFn.SetIsSubsetOf), ("isSupersetOf", NativeFn.SetIsSupersetOf),
            ("isDisjointFrom", NativeFn.SetIsDisjointFrom)
        ] {
            self.method(proto, name, n, 1);
        }
    }

    mutating func getSetRecord(obj: Value) -> Result[SetRecord, Value] {
        if not obj.isObject { return self.throwType("\(self.describe(obj)) is not a set-like object"); }
        let rawSize = try self.getV(obj, self.atom("size"));
        let num = try self.toNumber(rawSize);
        if num.isNaN { return self.throwType("The 'size' property must be a number"); }
        let size = toIntegerOrInfinityOf(num);
        if size < 0.0 { return .Err(self.makeError(.RangeError, "The 'size' property must not be negative")); }
        let has = try self.getV(obj, self.atom("has"));
        if not self.isCallable(has) { return self.throwType("The 'has' property must be a function"); }
        let keys = try self.getV(obj, self.atom("keys"));
        if not self.isCallable(keys) { return self.throwType("The 'keys' property must be a function"); }
        .Ok(SetRecord(obj: obj, size: size, has: has, keys: keys))
    }

    /// `other.keys()` and its `next`.
    mutating func setRecordKeys(rec: SetRecord) -> Result[(Value, Value), Value] {
        let it = try self.callValue(rec.keys, rec.obj, []);
        if not it.isObject { return self.throwType("keys() result is not an object"); }
        let next = try self.getV(it, self.names.next);
        .Ok((it, next))
    }

    func collectionId(v: Value) -> Int64 {
        match self.kindAt(v.asRef) {
            .Collection(id, _) => id,
            _ => -1
        }
    }

    /// A new Set holding `this`'s elements.
    mutating func copySet(id: Int64) -> Value {
        let setCtor = self.ownValue(self.global.asRef, self.atom("Set"));
        let proto = self.ownValue(setCtor.asRef, self.names.prototype);
        let out = self.newCollection(proto, CollectionKind.setKind);
        let rid = self.collectionId(out);
        for k in self.heap.maps(id).keys {
            if not k.isEmpty { self.setAdd(rid, k); }
        }
        out
    }

    /// Adds `v` to set `id` unless it's there, -0 as +0.
    mutating func setAdd(id: Int64, v: Value) {
        if self.setHas(id, v) { return; }
        let k = if v.isNumber and v.asNumber == 0.0 { Value.int(0) } else { v };
        self.collectionPut(id, k, k);
    }

    mutating func setHas(id: Int64, v: Value) -> Bool { self.collectionFind(id, self.collectionKey(v)) >= 0 }

    mutating func callHas(rec: SetRecord, v: Value) -> Result[Bool, Value] {
        let r = try self.callValue(rec.has, rec.obj, [v]);
        .Ok(self.toBoolean(r))
    }

    mutating func setMethod(op: Int64, this: Value, other: Value) -> Result[Value, Value] {
        let id = try self.receiver(this, [CollectionKind.setKind], "Set.prototype method");
        let rec = try self.getSetRecord(other);
        let mark = self.roots.count;
        let r = self.setMethodBody(op, id, rec);
        self.truncateRoots(mark);
        r
    }

    mutating func setMethodBody(op: Int64, id: Int64, rec: SetRecord) -> Result[Value, Value] {
        let thisSize = Float64(from: self.heap.maps(id).size);
        if op == SetOp.isSubsetOf {
            if thisSize > rec.size { return .Ok(Value.bool(false)); }
            var i = 0;
            while i < self.heap.maps(id).keys.count {
                let e = self.heap.maps(id).keys(i);
                i = i + 1;
                if e.isEmpty { continue; }
                let inOther = try self.callHas(rec, e);
                if not inOther { return .Ok(Value.bool(false)); }
            }
            return .Ok(Value.bool(true));
        }
        if op == SetOp.isSupersetOf or (op == SetOp.isDisjointFrom and thisSize > rec.size) {
            if op == SetOp.isSupersetOf and thisSize < rec.size { return .Ok(Value.bool(false)); }
            let (it, next) = try self.setRecordKeys(rec);
            self.pushRoot(it);
            while true {
                let step = try self.iteratorStep(it, next);
                let v = match step { .Some(x) => x, .None => break };
                let has = self.setHas(id, v);
                if has == (op == SetOp.isDisjointFrom) {
                    try self.iteratorClose(it);
                    return .Ok(Value.bool(false));
                }
            }
            return .Ok(Value.bool(true));
        }
        if op == SetOp.isDisjointFrom {
            var i = 0;
            while i < self.heap.maps(id).keys.count {
                let e = self.heap.maps(id).keys(i);
                i = i + 1;
                if e.isEmpty { continue; }
                let inOther = try self.callHas(rec, e);
                if inOther { return .Ok(Value.bool(false)); }
            }
            return .Ok(Value.bool(true));
        }

        // The rest build a new set.
        if op == SetOp.intersection {
            let setCtor = self.ownValue(self.global.asRef, self.atom("Set"));
            let proto = self.ownValue(setCtor.asRef, self.names.prototype);
            let out = self.newCollection(proto, CollectionKind.setKind);
            self.pushRoot(out);
            let rid = self.collectionId(out);
            if thisSize <= rec.size {
                var i = 0;
                while i < self.heap.maps(id).keys.count {
                    let e = self.heap.maps(id).keys(i);
                    i = i + 1;
                    if e.isEmpty { continue; }
                    let inOther = try self.callHas(rec, e);
                    if inOther { self.setAdd(rid, e); }
                }
            } else {
                let (it, next) = try self.setRecordKeys(rec);
                self.pushRoot(it);
                while true {
                    let step = try self.iteratorStep(it, next);
                    let v = match step { .Some(x) => x, .None => break };
                    if self.setHas(id, v) { self.setAdd(rid, v); }
                }
            }
            return .Ok(out);
        }
        let out = self.copySet(id);
        self.pushRoot(out);
        let rid = self.collectionId(out);
        if op == SetOp.difference and thisSize <= rec.size {
            var i = 0;
            while i < self.heap.maps(rid).keys.count {
                let e = self.heap.maps(rid).keys(i);
                i = i + 1;
                if e.isEmpty { continue; }
                let inOther = try self.callHas(rec, e);
                if inOther { let _ = self.collectionRemove(rid, e); }
            }
            return .Ok(out);
        }
        let (it, next) = try self.setRecordKeys(rec);
        self.pushRoot(it);
        while true {
            let step = try self.iteratorStep(it, next);
            let v = match step { .Some(x) => x, .None => break };
            if op == SetOp.union {
                self.setAdd(rid, v);
            } else if op == SetOp.difference {
                let _ = self.collectionRemove(rid, v);
            } else if self.setHas(id, v) {
                let _ = self.collectionRemove(rid, v);
            } else {
                self.setAdd(rid, v);
            }
        }
        .Ok(out)
    }

    mutating func setMethodsNative(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let op = match n {
            .SetUnion => SetOp.union,
            .SetIntersection => SetOp.intersection,
            .SetDifference => SetOp.difference,
            .SetSymmetricDifference => SetOp.symmetricDifference,
            .SetIsSubsetOf => SetOp.isSubsetOf,
            .SetIsSupersetOf => SetOp.isSupersetOf,
            .SetIsDisjointFrom => SetOp.isDisjointFrom,
            _ => return .Ok(.None)
        };
        .Ok(.Some(try self.setMethod(op, this, arg(args, 0))))
    }
}
