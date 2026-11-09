module js

// Map, Set, WeakMap and WeakSet (ECMA-262 §24).
//
// A collection's entries live in the heap's `maps` table, in insertion
// order with deleted entries left as holes, so iterators can walk them by
// position while the collection changes. Keys are hashed by their
// SameValueZero identity (`collectionKey`). The weak collections hold
// their keys strongly for now: the difference only shows in memory use.

/// Collection kinds, the `kind` of `ObjectKind.Collection`.
struct CollectionKind {
    static var mapKind: Int64 { 0 }
    static var setKind: Int64 { 1 }
    static var weakMap: Int64 { 2 }
    static var weakSet: Int64 { 3 }
}

extend VM {
    mutating func setupCollections() {
        let g = self.global;
        let names = ["Map", "Set", "WeakMap", "WeakSet"];
        let ctors = [NativeFn.MapCtor, NativeFn.SetCtor, NativeFn.WeakMapCtor, NativeFn.WeakSetCtor];
        var protos: [Value] = [];
        for i in 0..<4 {
            let proto = self.newObject();
            let c = self.constructorFor(names(i), ctors(i), 0, proto);
            let tag = self.str(names(i));
            self.defineOwn(proto.asRef, symbolKey(WellKnown.toStringTag), tag, Attr.configurable);
            if i < 2 {
                let species = self.nativeFunction(.ReturnThis, "get [Symbol.species]", 0, false);
                self.defineAccessorOwn(c.asRef, symbolKey(WellKnown.species), species, Value.undefined, Attr.configurable);
            }
            protos.append(proto);
        }
        let mapProto = protos(0);
        let setProto = protos(1);
        for (name, n, len) in [
            ("get", NativeFn.MapGet, 1), ("set", NativeFn.MapSet, 2), ("has", NativeFn.MapHas, 1),
            ("delete", NativeFn.MapDelete, 1), ("clear", NativeFn.MapClear, 0), ("forEach", NativeFn.MapForEach, 1),
            ("keys", NativeFn.MapKeys, 0), ("values", NativeFn.MapValues, 0), ("entries", NativeFn.MapEntries, 0)
        ] {
            self.method(mapProto, name, n, len);
        }
        self.getter(mapProto, "size", self.atom("size"), .MapSize);
        let mapEntries = self.ownValue(mapProto.asRef, self.atom("entries"));
        self.defineHidden(mapProto.asRef, symbolKey(WellKnown.iterator), mapEntries);
        let mapCtor = self.ownValue(g.asRef, self.atom("Map"));
        self.method(mapCtor, "groupBy", .MapGroupBy, 2);

        for (name, n, len) in [
            ("add", NativeFn.SetAdd, 1), ("has", NativeFn.SetHas, 1), ("delete", NativeFn.SetDelete, 1),
            ("clear", NativeFn.SetClear, 0), ("forEach", NativeFn.SetForEach, 1), ("entries", NativeFn.SetEntries, 0),
            ("values", NativeFn.SetValues, 0)
        ] {
            self.method(setProto, name, n, len);
        }
        self.getter(setProto, "size", self.atom("size"), .SetSize);
        // keys and @@iterator are the same function as values.
        let setValues = self.ownValue(setProto.asRef, self.atom("values"));
        self.defineHidden(setProto.asRef, self.atom("keys"), setValues);
        self.defineHidden(setProto.asRef, symbolKey(WellKnown.iterator), setValues);

        for (name, n, len) in [
            ("get", NativeFn.WeakMapGet, 1), ("set", NativeFn.WeakMapSet, 2), ("has", NativeFn.WeakMapHas, 1),
            ("delete", NativeFn.WeakMapDelete, 1)
        ] {
            self.method(protos(2), name, n, len);
        }
        for (name, n, len) in [("add", NativeFn.WeakSetAdd, 1), ("has", NativeFn.WeakSetHas, 1), ("delete", NativeFn.WeakSetDelete, 1)] {
            self.method(protos(3), name, n, len);
        }

        for (i, name) in [(0, "Map Iterator"), (1, "Set Iterator")] {
            let p = self.newObjectKind(self.intrinsics.iteratorProto, .Ordinary);
            self.method(p, "next", .CollectionIteratorNext, 0);
            let tag = self.str(name);
            self.defineOwn(p.asRef, symbolKey(WellKnown.toStringTag), tag, Attr.configurable);
            if i == 0 { self.intrinsics.mapIteratorProto = p; } else { self.intrinsics.setIteratorProto = p; }
        }
    }

    /// The SameValueZero identity of a key: -0 is 0, and strings by
    /// their contents (as atoms, under a tag no value uses).
    mutating func collectionKey(v: Value) -> UInt64 {
        if v.isString {
            let tag: UInt64 = 0xFFF8;
            let a = self.atomOfUnits(self.units(v));
            return (tag << 48) | UInt64(bitPattern: a);
        }
        if v.isBigInt {
            let tag: UInt64 = 0xFFF7;
            let a = self.atom(bnToString(self.bigOf(v), 10));
            return (tag << 48) | UInt64(bitPattern: a);
        }
        if v.isNumber and v.asNumber == 0.0 { return Value.int(0).bits; }
        v.bits
    }

    mutating func collectionFind(id: Int64, h: UInt64) -> Int64 {
        let found = self.heap.maps(id).index(h);
        found ?? -1
    }

    mutating func collectionPut(id: Int64, key: Value, value: Value) {
        let h = self.collectionKey(key);
        let at = self.collectionFind(id, h);
        if at >= 0 {
            self.heap.maps(id).values(at) = value;
            return;
        }
        // -0 is stored as +0.
        let k = if key.isNumber and key.asNumber == 0.0 { Value.int(0) } else { key };
        self.heap.maps(id).keys.append(k);
        self.heap.maps(id).values.append(value);
        let pos = self.heap.maps(id).keys.count - 1;
        self.heap.maps(id).index.insert(h, pos);
        self.heap.maps(id).size = self.heap.maps(id).size + 1;
    }

    mutating func collectionRemove(id: Int64, key: Value) -> Bool {
        let h = self.collectionKey(key);
        let at = self.collectionFind(id, h);
        if at < 0 { return false; }
        self.heap.maps(id).keys(at) = Value.empty;
        self.heap.maps(id).values(at) = Value.empty;
        let _ = self.heap.maps(id).index.remove(h);
        self.heap.maps(id).size = self.heap.maps(id).size - 1;
        true
    }

    /// Whether a value can be a weak collection's key.
    func canBeHeldWeakly(v: Value) -> Bool {
        if v.isObject { return true; }
        if v.isSymbol { return not self.isRegisteredSymbol(v); }
        false
    }

    func isRegisteredSymbol(v: Value) -> Bool {
        for (_, id) in self.symbolRegistry.iter() {
            if id == v.asRef { return true; }
        }
        false
    }

    mutating func collectionNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        // Each prototype's methods check for their own kind of receiver.
        let (op, owner) = collectionOp(n);
        let kinds = [owner];
        let r: Value = match op {
            .MapCtor => try self.makeCollection(newTarget, CollectionKind.mapKind, a0),
            .SetCtor => try self.makeCollection(newTarget, CollectionKind.setKind, a0),
            .WeakMapCtor => try self.makeCollection(newTarget, CollectionKind.weakMap, a0),
            .WeakSetCtor => try self.makeCollection(newTarget, CollectionKind.weakSet, a0),
            .MapGet => {
                let id = try self.receiver(this, kinds, "get");
                let at = self.collectionFind(id, self.collectionKey(a0));
                if at < 0 { Value.undefined } else { self.heap.maps(id).values(at) }
            },
            .MapSet => {
                let id = try self.receiver(this, kinds, "set");
                if self.isWeak(this) and not self.canBeHeldWeakly(a0) { return self.throwType("Invalid value used as weak map key"); }
                self.collectionPut(id, a0, a1);
                this
            },
            .SetAdd => {
                let id = try self.receiver(this, kinds, "add");
                if self.isWeak(this) and not self.canBeHeldWeakly(a0) { return self.throwType("Invalid value used in weak set"); }
                // Sets store -0 as +0, value included.
                let v = if a0.isNumber and a0.asNumber == 0.0 { Value.int(0) } else { a0 };
                self.collectionPut(id, v, v);
                this
            },
            .MapHas => {
                let id = try self.receiver(this, kinds, "has");
                Value.bool(self.collectionFind(id, self.collectionKey(a0)) >= 0)
            },
            .MapDelete => {
                let id = try self.receiver(this, kinds, "delete");
                Value.bool(self.collectionRemove(id, a0))
            },
            .MapClear => {
                let id = try self.receiver(this, kinds, "clear");
                for i in 0..<self.heap.maps(id).keys.count {
                    self.heap.maps(id).keys(i) = Value.empty;
                    self.heap.maps(id).values(i) = Value.empty;
                }
                self.heap.maps(id).index = [:];
                self.heap.maps(id).size = 0;
                Value.undefined
            },
            .MapSize => {
                let id = try self.receiver(this, kinds, "size");
                Value.int(self.heap.maps(id).size)
            },
            .MapForEach => {
                let id = try self.receiver(this, kinds, "forEach");
                let f = try self.requireCallable(a0);
                var i: Int64 = 0;
                // Entries added during the walk are visited too.
                while i < self.heap.maps(id).keys.count {
                    let k = self.heap.maps(id).keys(i);
                    if not k.isEmpty {
                        let v = self.heap.maps(id).values(i);
                        let _ = try self.callValue(f, a1, [v, k, this]);
                    }
                    i = i + 1;
                }
                Value.undefined
            },
            .MapKeys => try self.collectionIterator(this, CollectionKind.mapKind, 0),
            .MapValues => try self.collectionIterator(this, CollectionKind.mapKind, 1),
            .MapEntries => {
                try self.collectionIterator(this, owner, 2)
            },
            .SetValues => try self.collectionIterator(this, CollectionKind.setKind, 1),
            .CollectionIteratorNext => {
                if not this.isObject { return self.throwType("next called on incompatible receiver"); }
                let (target, index, kind) = match self.kindAt(this.asRef) {
                    .CollectionIterator(t, i, k) => (t, i, k),
                    _ => return self.throwType("next called on incompatible receiver")
                };
                if target.isUndefined { return .Ok(.Some(self.iterResult(Value.undefined, true))); }
                let id = match self.kindAt(target.asRef) { .Collection(x, _) => x, _ => -1 };
                var i = index;
                while i < self.heap.maps(id).keys.count and self.heap.maps(id).keys(i).isEmpty { i = i + 1; }
                if i >= self.heap.maps(id).keys.count {
                    self.heap.objects(this.asRef).kind = .CollectionIterator(target: Value.undefined, index: i, kind: kind);
                    return .Ok(.Some(self.iterResult(Value.undefined, true)));
                }
                self.heap.objects(this.asRef).kind = .CollectionIterator(target: target, index: i + 1, kind: kind);
                let k = self.heap.maps(id).keys(i);
                let v = self.heap.maps(id).values(i);
                let out = if kind == 0 { k } else if kind == 1 { v } else { self.newArray([k, v]) };
                self.iterResult(out, false)
            },
            .MapGroupBy => {
                let f = try self.requireCallable(a1);
                if a0.isNullish { return self.throwType("groupBy called on null or undefined"); }
                let mapCtor = self.ownValue(self.global.asRef, self.atom("Map"));
                let out = self.newCollection(self.ownValue(mapCtor.asRef, self.names.prototype), CollectionKind.mapKind);
                self.pushRoot(out);
                let id = try self.receiver(out, [CollectionKind.mapKind], "groupBy");
                let (it, next) = try self.getIterator(a0);
                self.pushRoot(it);
                var k: Int64 = 0;
                while true {
                    let step = try self.iteratorStep(it, next);
                    let v = match step {
                        .Some(x) => x,
                        .None => break
                    };
                    let key = match self.callValue(f, Value.undefined, [v, Value.int(k)]) {
                        .Ok(x) => x,
                        .Err(e) => {
                            let _ = self.iteratorClose(it);
                            return .Err(e);
                        }
                    };
                    let at = self.collectionFind(id, self.collectionKey(key));
                    var group = if at >= 0 { self.heap.maps(id).values(at) } else { Value.undefined };
                    if group.isUndefined {
                        group = self.newArray([]);
                        self.collectionPut(id, key, group);
                    }
                    self.appendElement(group.asRef, v);
                    k = k + 1;
                }
                out
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    mutating func receiver(this: Value, kinds: Array[Int64], name: String) -> Result[Int64, Value] {
        if this.isObject {
            if let .Collection(id, kind) = self.kindAt(this.asRef) {
                if kinds.contains(kind) { return .Ok(id); }
            }
        }
        self.throwType("Method \(name) called on incompatible receiver")
    }

    func isWeak(v: Value) -> Bool {
        match self.kindAt(v.asRef) {
            .Collection(_, k) => k == CollectionKind.weakMap or k == CollectionKind.weakSet,
            _ => false
        }
    }

    /// `new Map(iterable)` and friends.
    mutating func makeCollection(newTarget: Value, kind: Int64, iterable: Value) -> Result[Value, Value] {
        let names = ["Map", "Set", "WeakMap", "WeakSet"];
        if newTarget.isUndefined { return self.throwType("Constructor \(names(kind)) requires 'new'"); }
        let ctor = self.ownValue(self.global.asRef, self.atom(names(kind)));
        let fallback = self.ownValue(ctor.asRef, self.names.prototype);
        let proto = try self.prototypeFor(newTarget, fallback);
        let o = self.newCollection(proto, kind);
        if iterable.isNullish { return .Ok(o); }
        self.pushRoot(o);
        let isMap = kind == CollectionKind.mapKind or kind == CollectionKind.weakMap;
        let adder = try self.getV(o, self.atom(if isMap { "set" } else { "add" }));
        if not self.isCallable(adder) { return self.throwType("'\(if isMap { "set" } else { "add" })' is not a function"); }
        let (it, next) = try self.getIterator(iterable);
        self.pushRoot(it);
        while true {
            let step = try self.iteratorStep(it, next);
            let item = match step {
                .Some(x) => x,
                .None => break
            };
            if not isMap {
                match self.callValue(adder, o, [item]) {
                    .Ok(_) => {},
                    .Err(e) => {
                        let _ = self.iteratorClose(it);
                        return .Err(e);
                    }
                }
                continue;
            }
            if not item.isObject {
                let _ = self.iteratorClose(it);
                return self.throwType("Iterator value \(self.describe(item)) is not an entry object");
            }
            let entry = self.getIndexClosing(item, it);
            let (k, v) = match entry {
                .Ok(pair) => pair,
                .Err(e) => return .Err(e)
            };
            match self.callValue(adder, o, [k, v]) {
                .Ok(_) => {},
                .Err(e) => {
                    let _ = self.iteratorClose(it);
                    return .Err(e);
                }
            }
        }
        .Ok(o)
    }

    mutating func newCollection(proto: Value, kind: Int64) -> Value {
        let id = self.heap.allocMap();
        self.newObjectKind(proto, .Collection(id: id, kind: kind))
    }

    /// An entry's [0] and [1], closing the iterator if reading throws.
    mutating func getIndexClosing(item: Value, it: Value) -> Result[(Value, Value), Value] {
        let k = match self.getIndex(item, 0) {
            .Ok(x) => x,
            .Err(e) => {
                let _ = self.iteratorClose(it);
                return .Err(e);
            }
        };
        let v = match self.getIndex(item, 1) {
            .Ok(x) => x,
            .Err(e) => {
                let _ = self.iteratorClose(it);
                return .Err(e);
            }
        };
        .Ok((k, v))
    }

    mutating func collectionIterator(this: Value, kind: Int64, which: Int64) -> Result[Value, Value] {
        let _ = try self.receiver(this, [kind], if kind == CollectionKind.mapKind { "Map Iterator" } else { "Set Iterator" });
        let proto = if kind == CollectionKind.mapKind { self.intrinsics.mapIteratorProto } else { self.intrinsics.setIteratorProto };
        .Ok(self.newObjectKind(proto, .CollectionIterator(target: this, index: 0, kind: which)))
    }
}

/// The shared implementation behind a collection native, and the kind
/// of collection its prototype holds.
func collectionOp(n: NativeFn) -> (NativeFn, Int64) {
    match n {
        .SetHas => (NativeFn.MapHas, CollectionKind.setKind),
        .SetDelete => (NativeFn.MapDelete, CollectionKind.setKind),
        .SetClear => (NativeFn.MapClear, CollectionKind.setKind),
        .SetForEach => (NativeFn.MapForEach, CollectionKind.setKind),
        .SetEntries => (NativeFn.MapEntries, CollectionKind.setKind),
        .SetSize => (NativeFn.MapSize, CollectionKind.setKind),
        .SetAdd => (NativeFn.SetAdd, CollectionKind.setKind),
        .WeakMapGet => (NativeFn.MapGet, CollectionKind.weakMap),
        .WeakMapSet => (NativeFn.MapSet, CollectionKind.weakMap),
        .WeakMapHas => (NativeFn.MapHas, CollectionKind.weakMap),
        .WeakMapDelete => (NativeFn.MapDelete, CollectionKind.weakMap),
        .WeakSetAdd => (NativeFn.SetAdd, CollectionKind.weakSet),
        .WeakSetHas => (NativeFn.MapHas, CollectionKind.weakSet),
        .WeakSetDelete => (NativeFn.MapDelete, CollectionKind.weakSet),
        _ => (n, CollectionKind.mapKind)
    }
}
