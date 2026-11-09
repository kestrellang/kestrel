module js

// Objects: shapes, own properties, and the [[Get]], [[Set]],
// [[Delete]] and [[OwnPropertyKeys]] operations, with the exotic
// behaviour of arrays (indexed elements and `length`) and string
// wrappers.
//
// Invariant: a shape's existing keys, slots and attributes never change
// once made. Adding a key to a shared shape makes or reuses a child;
// adding to an object's own (dictionary) shape appends in place, which
// keeps old slots valid; anything else makes a fresh shape. So an inline
// cache entry (shape, slot) stays correct for as long as it matches.

/// An own property as found: its value (an accessor pair object for
/// accessors) and attributes.
struct PropertyRecord {
    var value: Value
    var attrs: Int64
}

extend VM {
    // ------------------------------------------------------------------
    // Creating objects
    // ------------------------------------------------------------------

    mutating func newObjectKind(proto: Value, kind: ObjectKind) -> Value {
        let o = JSObject(shape: 0, slots: [], proto: proto, elements: [], length: 0, hasShapeIndices: false, lengthWritable: true, kind: kind, extensible: true, isConstructor: false);
        Value.object(self.heap.allocObject(o))
    }

    mutating func newObject() -> Value { self.newObjectKind(self.intrinsics.objectProto, .Ordinary) }

    mutating func newArray(elements: Array[Value]) -> Value {
        let a = self.newObjectKind(self.intrinsics.arrayProto, .ArrayObject);
        self.setDense(a.asRef, elements);
        a
    }

    /// Replaces an array's elements wholesale; `length` follows.
    mutating func setDense(r: Ref, elements: Array[Value]) {
        self.heap.objects(r).length = elements.count;
        self.heap.objects(r).elements = elements;
    }

    /// Appends to a simple array (a literal or a fresh result).
    mutating func appendElement(r: Ref, v: Value) {
        self.heap.objects(r).elements.append(v);
        self.heap.objects(r).length = self.heap.objects(r).elements.count;
    }

    /// An array the natives may treat as its dense `elements`: every
    /// index is there, and `length` is theirs.
    func isSimpleArray(v: Value) -> Bool {
        if not self.isArray(v) { return false; }
        let r = v.asRef;
        self.heap.objects(r).extensible and not self.heap.objects(r).hasShapeIndices and self.heap.objects(r).lengthWritable
            and self.heap.objects(r).elements.count == self.heap.objects(r).length
    }

    func kindOf(v: Value) -> ObjectKind { self.heap.objects(v.asRef).kind }

    /// A copy of the object's kind, so callers can match on it while
    /// changing the heap.
    func kindAt(r: Ref) -> ObjectKind { self.heap.objects(r).kind }

    func isArray(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .ArrayObject => true,
            _ => false
        }
    }

    /// IsArray, which throws for a revoked proxy.
    mutating func isArrayV(v: Value) -> Result[Bool, Value] {
        if self.isProxy(v) {
            let (t, _) = try self.proxyParts(v);
            return self.isArrayV(t);
        }
        .Ok(self.isArray(v))
    }

    func isCallable(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .Closure(_, _, _, _) => true,
            .NativeFunction(_) => true,
            .NativeClosure(_, _) => true,
            .BoundFunction(_, _, _) => true,
            .ProxyObject(t, _) => self.isCallable(t),
            _ => false
        }
    }

    func isConstructor(v: Value) -> Bool {
        v.isObject and self.heap.objects(v.asRef).isConstructor
    }

    func protoOf(r: Ref) -> Value { self.heap.objects(r).proto }

    // ------------------------------------------------------------------
    // Shapes
    // ------------------------------------------------------------------

    func shapeSlot(shape: Int64, key: Key) -> Int64 {
        self.shapes(shape).slotOf(key) ?? -1
    }

    /// Adds data or accessor property `key` to `r`, returning its slot.
    mutating func addSlot(r: Ref, key: Key, value: Value, attrs: Int64) -> Int64 {
        let s = self.heap.objects(r).shape;
        if self.shapes(s).shared {
            if attrs == Attr.all {
                let next = self.shapes(s).transitions(key);
                if let .Some(child) = next {
                    self.heap.objects(r).shape = child;
                    self.heap.objects(r).slots.append(value);
                    return self.heap.objects(r).slots.count - 1;
                }
            }
            var child = self.shapes(s);
            let slot = child.keys.count;
            child.keys.append(key);
            child.attrs.append(attrs);
            let _ = child.slotOf.insert(key, slot);
            child.transitions = [:];
            // Big or unusual objects get a shape of their own.
            child.shared = attrs == Attr.all and slot < 64;
            self.shapes.append(child);
            let id = self.shapes.count - 1;
            if child.shared { let _ = self.shapes(s).transitions.insert(key, id); }
            self.heap.objects(r).shape = id;
        } else {
            let slot = self.shapes(s).keys.count;
            self.shapes(s).keys.append(key);
            self.shapes(s).attrs.append(attrs);
            let _ = self.shapes(s).slotOf.insert(key, slot);
        }
        self.heap.objects(r).slots.append(value);
        self.heap.objects(r).slots.count - 1
    }

    /// Gives `r` a fresh shape of its own with `key` removed, or its
    /// attributes changed when `newAttrs >= 0`.
    mutating func reshape(r: Ref, key: Key, newAttrs: Int64) {
        let old = self.shapes(self.heap.objects(r).shape);
        let oldSlots = self.heap.objects(r).slots;
        var s = Shape(keys: [], attrs: [], slotOf: [:], transitions: [:], shared: false);
        var slots: [Value] = [];
        for i in 0..<old.keys.count {
            let k = old.keys(i);
            var a = old.attrs(i);
            if k == key {
                if newAttrs < 0 { continue; }
                a = newAttrs;
            }
            let _ = s.slotOf.insert(k, s.keys.count);
            s.keys.append(k);
            s.attrs.append(a);
            slots.append(oldSlots(i));
        }
        self.shapes.append(s);
        self.heap.objects(r).shape = self.shapes.count - 1;
        self.heap.objects(r).slots = slots;
    }

    // ------------------------------------------------------------------
    // Own properties
    // ------------------------------------------------------------------

    func isLengthKey(key: Key) -> Bool { key == self.names.length }

    /// The array index `key` names, or -1.
    func indexOfKey(key: Key) -> Int64 {
        if key < 0 { return -1; }
        self.atomIndex(key)
    }

    /// An own data property's value, or undefined. Used by realm setup
    /// to share builtins without running getters.
    func ownValue(r: Ref, key: Key) -> Value {
        match self.ownProperty(r, key) {
            .Some(p) => p.value,
            .None => Value.undefined
        }
    }

    func ownProperty(r: Ref, key: Key) -> Optional[PropertyRecord] {
        let o = self.heap.objects(r);
        match o.kind {
            .ArrayObject => {
                let i = self.indexOfKey(key);
                if i >= 0 {
                    if i < o.elements.count and not o.elements(i).isEmpty {
                        return .Some(PropertyRecord(value: o.elements(i), attrs: Attr.all));
                    }
                    if i >= o.length { return .None; }
                    // A hole may be an element with attributes, an
                    // accessor or a sparse element, which live in the
                    // shape under their keys.
                }
                if self.isLengthKey(key) {
                    return .Some(PropertyRecord(value: Value.int(o.length), attrs: if o.lengthWritable { Attr.writable } else { 0 }));
                }
            },
            .StringBox(s) => {
                let u = self.units(s);
                let i = self.indexOfKey(key);
                if i >= 0 and i < u.count {
                    return .Some(PropertyRecord(value: Value.undefined, attrs: Attr.enumerable));
                }
                if self.isLengthKey(key) {
                    return .Some(PropertyRecord(value: Value.int(u.count), attrs: 0));
                }
            },
            .MappedArguments(env, map) => {
                let i = self.indexOfKey(key);
                if i >= 0 and i < map.count and map(i) >= 0 {
                    let slot = self.shapeSlot(o.shape, key);
                    if slot >= 0 and self.shapes(o.shape).attrs(slot) & Attr.accessor == 0 {
                        return .Some(PropertyRecord(value: self.heap.envs(env).slots(map(i)), attrs: self.shapes(o.shape).attrs(slot)));
                    }
                }
            },
            .ModuleNamespace(_) => {
                // Exports hold import references; the value may be the TDZ marker.
                let slot = self.shapeSlot(o.shape, key);
                if slot < 0 { return .None; }
                return .Some(PropertyRecord(value: self.derefImport(o.slots(slot)), attrs: self.shapes(o.shape).attrs(slot)));
            },
            .TypedArrayObject(d) => {
                // Numeric keys are elements or nothing, never shape slots.
                if let .Some(n) = self.canonicalNumeric(key) {
                    if not self.taValidIndex(d, n) { return .None; }
                    let v = self.taGetPure(d, n.toInt64() ?? 0);
                    return .Some(PropertyRecord(value: v, attrs: Attr.writable | Attr.enumerable | Attr.configurable));
                }
            },
            _ => {}
        }
        let slot = self.shapeSlot(o.shape, key);
        if slot < 0 { return .None; }
        .Some(PropertyRecord(value: o.slots(slot), attrs: self.shapes(o.shape).attrs(slot)))
    }

    /// Like `ownProperty`, but fills in a string wrapper's characters.
    mutating func ownPropertyValue(r: Ref, key: Key) -> Optional[PropertyRecord] {
        // BigInt typed array elements are allocated on read.
        if let .Some((d, n)) = self.taKeyIndex(r, key) {
            if not self.taValidIndex(d, n) { return .None; }
            let v = self.taGet(d, n.toInt64() ?? 0);
            return .Some(PropertyRecord(value: v, attrs: Attr.writable | Attr.enumerable | Attr.configurable));
        }
        if let .StringBox(s) = self.kindAt(r) {
            let i = self.indexOfKey(key);
            let u = self.units(s);
            if i >= 0 and i < u.count {
                let c = self.newString([u(i)]);
                return .Some(PropertyRecord(value: c, attrs: Attr.enumerable));
            }
        }
        self.ownProperty(r, key)
    }

    func hasOwn(r: Ref, key: Key) -> Bool {
        match self.ownProperty(r, key) { .Some(_) => true, .None => false }
    }

    /// Creates or replaces an own data property.
    mutating func defineOwn(r: Ref, key: Key, value: Value, attrs: Int64) {
        if let .ArrayObject = self.kindAt(r) {
            let i = self.indexOfKey(key);
            if i >= 0 {
                let inShape = self.shapeSlot(self.heap.objects(r).shape, key) >= 0;
                if attrs == Attr.all and self.isDenseIndex(r, i) {
                    if inShape { self.reshape(r, key, -1); }
                    self.setElement(r, i, value);
                    return;
                }
                // Everything else lives in the shape; a dense slot stays
                // a hole.
                if i < self.heap.objects(r).elements.count { self.heap.objects(r).elements(i) = Value.empty; }
                if i >= self.heap.objects(r).length { self.heap.objects(r).length = i + 1; }
                self.heap.objects(r).hasShapeIndices = true;
            }
            if self.isLengthKey(key) {
                if value.isNumber {
                    let n = value.asNumber.toInt64() ?? 0;
                    self.setArrayLength(r, n);
                }
                return;
            }
        }
        let o = self.heap.objects(r);
        let slot = self.shapeSlot(o.shape, key);
        if slot < 0 {
            let _ = self.addSlot(r, key, value, attrs);
            return;
        }
        if self.shapes(o.shape).attrs(slot) != attrs { self.reshape(r, key, attrs); }
        let s = self.shapeSlot(self.heap.objects(r).shape, key);
        self.heap.objects(r).slots(s) = value;
    }

    mutating func defineAccessorOwn(r: Ref, key: Key, getter: Value, setter: Value, attrs: Int64) {
        let pair = self.newObjectKind(Value.nullValue, .AccessorPair(getter: getter, setter: setter));
        self.defineOwn(r, key, pair, attrs | Attr.accessor);
    }

    /// Writes an existing own data property, or an array element.
    mutating func writeOwn(r: Ref, key: Key, value: Value) {
        self.writeMappedArgument(r, key, value);
        if let .ArrayObject = self.kindAt(r) {
            let i = self.indexOfKey(key);
            if i >= 0 and self.shapeSlot(self.heap.objects(r).shape, key) < 0 and self.isDenseIndex(r, i) {
                self.setElement(r, i, value);
                return;
            }
            if self.isLengthKey(key) {
                let n = value.asNumber.toInt64() ?? 0;
                self.setArrayLength(r, n);
                return;
            }
        }
        let s = self.shapeSlot(self.heap.objects(r).shape, key);
        if s >= 0 { self.heap.objects(r).slots(s) = value; }
    }

    /// False for an index at or past an array's read-only length.
    func canAddIndex(r: Ref, key: Key) -> Bool {
        if let .ArrayObject = self.kindAt(r) {
            let i = self.indexOfKey(key);
            if i >= 0 and i >= self.heap.objects(r).length and not self.heap.objects(r).lengthWritable { return false; }
        }
        true
    }

    /// Whether index `i` belongs in the dense elements: an array grows
    /// them by at most a million holes at a time.
    func isDenseIndex(r: Ref, i: Int64) -> Bool {
        i < self.heap.objects(r).elements.count + 1000000
    }

    mutating func setElement(r: Ref, i: Int64, value: Value) {
        while self.heap.objects(r).elements.count <= i {
            self.heap.objects(r).elements.append(Value.empty);
        }
        self.heap.objects(r).elements(i) = value;
        if i >= self.heap.objects(r).length { self.heap.objects(r).length = i + 1; }
    }

    mutating func setArrayLength(r: Ref, n: Int64) {
        // Attributed elements past the new end go too.
        var doomed: [Key] = [];
        for k in self.shapes(self.heap.objects(r).shape).keys {
            if self.indexOfKey(k) >= n { doomed.append(k); }
        }
        for k in doomed { self.reshape(r, k, -1); }
        while self.heap.objects(r).elements.count > n {
            let _ = self.heap.objects(r).elements.pop();
        }
        self.heap.objects(r).length = n;
    }

    /// [[Delete]]: false for a non-configurable property.
    mutating func deleteOwn(r: Ref, key: Key) -> Result[Bool, Value] {
        if let .ProxyObject(_, _) = self.kindAt(r) { return self.proxyDelete(Value.object(r), key); }
        .Ok(self.deleteOrdinary(r, key))
    }

    mutating func deleteOrdinary(r: Ref, key: Key) -> Bool {
        let o = self.heap.objects(r);
        match o.kind {
            .ArrayObject => {
                let i = self.indexOfKey(key);
                if i >= 0 and i < o.elements.count and not o.elements(i).isEmpty {
                    self.heap.objects(r).elements(i) = Value.empty;
                    return true;
                }
                if self.isLengthKey(key) { return false; }
            },
            .StringBox(s) => {
                let i = self.indexOfKey(key);
                if (i >= 0 and i < self.units(s).count) or self.isLengthKey(key) { return false; }
            },
            .TypedArrayObject(d) => {
                if let .Some(n) = self.canonicalNumeric(key) { return not self.taValidIndex(d, n); }
            },
            _ => {}
        }
        let slot = self.shapeSlot(o.shape, key);
        if slot < 0 { return true; }
        if self.shapes(o.shape).attrs(slot) & Attr.configurable == 0 { return false; }
        self.unmapArgument(r, key);
        self.reshape(r, key, -1);
        true
    }

    // ------------------------------------------------------------------
    // Mapped arguments
    // ------------------------------------------------------------------

    /// The env slot a mapped `arguments` index aliases, or -1.
    func mappedArgumentSlot(r: Ref, key: Key) -> Int64 {
        let kind = self.kindAt(r);
        match kind {
            .MappedArguments(_, map) => {
                let i = self.indexOfKey(key);
                if i >= 0 and i < map.count { return map(i); }
                return -1;
            },
            _ => return -1
        }
    }

    /// Writes through a mapped index to its parameter.
    mutating func writeMappedArgument(r: Ref, key: Key, value: Value) {
        let slot = self.mappedArgumentSlot(r, key);
        if slot < 0 { return; }
        if let .MappedArguments(env, _) = self.kindAt(r) { self.heap.envs(env).slots(slot) = value; }
    }

    /// Ends the alias for `key`, first saving the parameter's current
    /// value in the property.
    mutating func unmapArgument(r: Ref, key: Key) {
        let slot = self.mappedArgumentSlot(r, key);
        if slot < 0 { return; }
        if let .MappedArguments(env, map) = self.kindAt(r) {
            // Only a data property takes the parameter's value; an accessor
            // keeps its getter/setter pair.
            let shape = self.heap.objects(r).shape;
            let own = self.shapeSlot(shape, key);
            if own >= 0 and self.shapes(shape).attrs(own) & Attr.accessor == 0 {
                self.heap.objects(r).slots(own) = self.heap.envs(env).slots(slot);
            }
            var m = map;
            m(self.indexOfKey(key)) = -1;
            self.heap.objects(r).kind = .MappedArguments(env: env, map: m);
        }
    }

    /// [[OwnPropertyKeys]]: array indices ascending, then strings in
    /// creation order, then symbols.
    mutating func ownKeys(r: Ref) -> Result[Array[Key], Value] {
        if let .ProxyObject(_, _) = self.kindAt(r) { return self.proxyOwnKeys(Value.object(r)); }
        .Ok(self.ordinaryOwnKeys(r))
    }

    mutating func ordinaryOwnKeys(r: Ref) -> Array[Key] {
        let o = self.heap.objects(r);
        var indices: [Int64] = [];
        var out: [Key] = [];
        var extraIndexKeys: [Key] = [];
        match o.kind {
            .ArrayObject => {
                for i in 0..<o.elements.count {
                    if not o.elements(i).isEmpty { indices.append(i); }
                }
            },
            .StringBox(s) => {
                for i in 0..<self.units(s).count { indices.append(i); }
            },
            .TypedArrayObject(d) => {
                for i in 0..<self.taLength(d) { indices.append(i); }
            },
            _ => {}
        }
        let shape = self.shapes(o.shape);
        for k in shape.keys {
            if self.indexOfKey(k) >= 0 { extraIndexKeys.append(k); }
        }
        for k in extraIndexKeys { indices.append(self.indexOfKey(k)); }
        if extraIndexKeys.count > 0 { indices.sort(); }
        for i in indices { out.append(self.internIndex(i)); }
        match o.kind {
            .ArrayObject => out.append(self.names.length),
            .StringBox(_) => out.append(self.names.length),
            _ => {}
        }
        for k in shape.keys {
            if not keyIsSymbol(k) and self.indexOfKey(k) < 0 { out.append(k); }
        }
        for k in shape.keys {
            if keyIsSymbol(k) and not self.isPrivateKey(k) { out.append(k); }
        }
        out
    }

    mutating func internIndex(i: Int64) -> Key { self.atom("\(i)") }

    // ------------------------------------------------------------------
    // [[Get]] and [[Set]]
    // ------------------------------------------------------------------

    /// `obj[key]` with `receiver` as `this` for getters.
    mutating func getFrom(obj: Value, key: Key, receiver: Value) -> Result[Value, Value] {
        var cur = obj;
        while cur.isObject {
            let r = cur.asRef;
            if self.isProxy(cur) { return self.proxyGet(cur, key, receiver); }
            if self.isNamespace(cur) { return self.namespaceGet(r, key); }
            // A typed array's numeric keys never reach its prototype.
            if let .Some((d, n)) = self.taKeyIndex(r, key) {
                if not self.taValidIndex(d, n) { return .Ok(Value.undefined); }
                return .Ok(self.taGet(d, n.toInt64() ?? 0));
            }
            if let .Some(p) = self.ownPropertyValue(r, key) {
                if p.attrs & Attr.accessor != 0 {
                    if let .AccessorPair(getter, _) = self.kindAt(p.value.asRef) {
                        if getter.isUndefined { return .Ok(Value.undefined); }
                        return self.callValue(getter, receiver, []);
                    }
                }
                return .Ok(p.value);
            }
            cur = self.heap.objects(r).proto;
        }
        .Ok(Value.undefined)
    }

    /// `v[key]` for any value, boxing primitives.
    mutating func getV(v: Value, key: Key) -> Result[Value, Value] {
        if v.isObject { return self.getFrom(v, key, v); }
        if v.isString {
            let u = self.units(v);
            if self.isLengthKey(key) { return .Ok(Value.int(u.count)); }
            let i = self.indexOfKey(key);
            if i >= 0 {
                if i < u.count { return .Ok(self.newString([u(i)])); }
                return .Ok(Value.undefined);
            }
            return self.getFrom(self.intrinsics.stringProto, key, v);
        }
        if v.isNumber { return self.getFrom(self.intrinsics.numberProto, key, v); }
        if v.isBool { return self.getFrom(self.intrinsics.booleanProto, key, v); }
        if v.isSymbol { return self.getFrom(self.intrinsics.symbolProto, key, v); }
        if v.isBigInt { return self.getFrom(self.intrinsics.bigintProto, key, v); }
        let what = if v.isNull { "null" } else { "undefined" };
        self.throwType("Cannot read properties of \(what) (reading '\(self.keyText(key))')")
    }

    /// `obj[key] = value`. In strict code a failed assignment throws.
    mutating func setOn(obj: Value, key: Key, value: Value, receiver: Value, strict: Bool) -> Result[(), Value] {
        let ok = try self.ordinarySet(obj, key, value, receiver);
        if not ok and strict {
            return self.throwType("Cannot assign to read only property '\(self.keyText(key))'");
        }
        .Ok(())
    }

    /// OrdinarySet, returning whether it succeeded.
    mutating func ordinarySet(obj: Value, key: Key, value: Value, receiver: Value) -> Result[Bool, Value] {
        // An array's own `length` goes through ArraySetLength, which
        // validates the value and respects non-configurable elements.
        if obj.same(receiver) and self.isLengthKey(key) {
            if let .ArrayObject = self.kindAt(obj.asRef) {
                if not self.heap.objects(obj.asRef).lengthWritable { return .Ok(false); }
                let d = Descriptor(value: .Some(value), writable: .None, getter: .None, setter: .None, enumerable: .None, configurable: .None);
                return self.arraySetLength(obj.asRef, d);
            }
        }
        var cur = obj;
        while cur.isObject {
            let r = cur.asRef;
            if self.isProxy(cur) { return self.proxySet(cur, key, value, receiver); }
            // A module namespace's [[Set]] always fails.
            if self.isNamespace(cur) { return .Ok(false); }
            // TypedArray [[Set]]: write the element when the typed array is
            // the receiver; an invalid index silently does nothing.
            if let .Some((d, n)) = self.taKeyIndex(r, key) {
                if cur.same(receiver) {
                    try self.taSetElement(d, n, value);
                    return .Ok(true);
                }
                if not self.taValidIndex(d, n) { return .Ok(true); }
            }
            if let .Some(p) = self.ownProperty(r, key) {
                if p.attrs & Attr.accessor != 0 {
                    if let .AccessorPair(_, setter) = self.kindAt(p.value.asRef) {
                        if setter.isUndefined { return .Ok(false); }
                        let _ = try self.callValue(setter, receiver, [value]);
                        return .Ok(true);
                    }
                }
                if p.attrs & Attr.writable == 0 { return .Ok(false); }
                if cur.same(receiver) {
                    self.writeOwn(r, key, value);
                    return .Ok(true);
                }
                break;
            }
            cur = self.heap.objects(r).proto;
        }
        if not receiver.isObject { return .Ok(false); }
        if self.isProxy(receiver) { return self.setOnReceiver(receiver, key, value); }
        let rr = receiver.asRef;
        // A namespace receiver: its [[DefineOwnProperty]] (which throws in the TDZ).
        if self.isNamespace(receiver) and not keyIsSymbol(key) {
            let d = Descriptor(value: .Some(value), writable: .None, getter: .None, setter: .None, enumerable: .None, configurable: .None);
            return self.namespaceDefine(rr, key, d);
        }
        // A typed array receiver defines elements through its own [[DefineOwnProperty]].
        if let .Some((d, n)) = self.taKeyIndex(rr, key) {
            if not self.taValidIndex(d, n) { return .Ok(false); }
            try self.taSetElement(d, n, value);
            return .Ok(true);
        }
        if let .Some(p) = self.ownProperty(rr, key) {
            if p.attrs & Attr.accessor != 0 or p.attrs & Attr.writable == 0 { return .Ok(false); }
            self.writeOwn(rr, key, value);
            return .Ok(true);
        }
        if not self.heap.objects(rr).extensible { return .Ok(false); }
        if not self.canAddIndex(rr, key) { return .Ok(false); }
        self.defineOwn(rr, key, value, Attr.all);
        .Ok(true)
    }

    /// `v[key] = value` for any value: primitives have no own properties,
    /// so this only runs setters (or fails in strict code).
    mutating func setV(v: Value, key: Key, value: Value, strict: Bool) -> Result[(), Value] {
        if v.isObject { return self.setOn(v, key, value, v, strict); }
        if v.isNullish {
            let what = if v.isNull { "null" } else { "undefined" };
            return self.throwType("Cannot set properties of \(what) (setting '\(self.keyText(key))')");
        }
        let proto = try self.toObject(v);
        self.setOn(self.heap.objects(proto.asRef).proto, key, value, v, strict)
    }

    /// [[HasProperty]].
    mutating func hasProperty(obj: Value, key: Key) -> Result[Bool, Value] {
        var cur = obj;
        while cur.isObject {
            if self.isProxy(cur) { return self.proxyHas(cur, key); }
            if let .Some((d, n)) = self.taKeyIndex(cur.asRef, key) { return .Ok(self.taValidIndex(d, n)); }
            if self.hasOwn(cur.asRef, key) { return .Ok(true); }
            cur = self.heap.objects(cur.asRef).proto;
        }
        .Ok(false)
    }

    /// OrdinarySetWithOwnDescriptor's last steps, for a receiver that may
    /// be exotic: update its own data property or create one.
    mutating func setOnReceiver(receiver: Value, key: Key, value: Value) -> Result[Bool, Value] {
        let none = Descriptor(value: .Some(value), writable: .None, getter: .None, setter: .None, enumerable: .None, configurable: .None);
        if let .Some(d) = try self.getOwnPropertyV(receiver, key) {
            if isAccessorDescriptor(d) or not (d.writable ?? false) { return .Ok(false); }
            return self.defineProperty(receiver.asRef, key, none);
        }
        let full = Descriptor(value: .Some(value), writable: .Some(true), getter: .None, setter: .None, enumerable: .Some(true), configurable: .Some(true));
        self.defineProperty(receiver.asRef, key, full)
    }

    /// Defines `name`, `length` and the like on a function: not
    /// writable, not enumerable, configurable.
    mutating func defineReadOnly(r: Ref, name: String, value: Value) {
        let k = self.atom(name);
        self.defineOwn(r, k, value, Attr.configurable);
    }

    /// Defines a built-in method or value: writable and configurable,
    /// not enumerable.
    mutating func defineHidden(r: Ref, key: Key, value: Value) {
        self.defineOwn(r, key, value, Attr.writable | Attr.configurable);
    }
}

// ----------------------------------------------------------------------
// Private names
// ----------------------------------------------------------------------
//
// A private name is a symbol in `VM.privateSymbols`. Private fields,
// methods and accessors are own properties under it that `ownKeys` hides,
// reached only through the private-aware paths below, which never look
// at the prototype chain and throw where ordinary property access would
// quietly do nothing.

extend VM {
    func isPrivateKey(k: Key) -> Bool { keyIsSymbol(k) and self.privateSymbols.contains(symbolOfKey(k)) }

    /// The own private property `k` of `obj`, or a TypeError naming `verb`.
    mutating func privateProperty(obj: Value, k: Key, verb: String) -> Result[PropertyRecord, Value] {
        if obj.isObject {
            if let .Some(p) = self.ownProperty(obj.asRef, k) { return .Ok(p); }
        }
        self.throwType("Cannot \(verb) private member \(self.keyText(k)) from an object whose class did not declare it")
    }

    /// The accessor pair stored in an accessor property's slot.
    func accessorPair(p: PropertyRecord) -> (Value, Value) {
        match self.kindAt(p.value.asRef) {
            .AccessorPair(g, s) => (g, s),
            _ => (Value.undefined, Value.undefined)
        }
    }

    mutating func privateGet(obj: Value, k: Key) -> Result[Value, Value] {
        let p = try self.privateProperty(obj, k, "read");
        if p.attrs & Attr.accessor == 0 { return .Ok(p.value); }
        let (getter, _) = self.accessorPair(p);
        if getter.isUndefined { return self.throwType("'\(self.keyText(k))' was defined without a getter"); }
        self.callValue(getter, obj, [])
    }

    mutating func privateSet(obj: Value, k: Key, v: Value) -> Result[(), Value] {
        let p = try self.privateProperty(obj, k, "write");
        if p.attrs & Attr.accessor != 0 {
            let (_, setter) = self.accessorPair(p);
            if setter.isUndefined { return self.throwType("'\(self.keyText(k))' was defined without a setter"); }
            let _ = try self.callValue(setter, obj, [v]);
            return .Ok(());
        }
        if p.attrs & Attr.writable == 0 { return self.throwType("Private method \(self.keyText(k)) is not writable"); }
        self.writeOwn(obj.asRef, k, v);
        .Ok(())
    }

    /// PrivateMethodOrAccessorAdd: `kind` 0 a method, 1 a getter, 2 a
    /// setter. A setter joins the getter of the same name.
    mutating func privateMethodAdd(obj: Value, k: Key, f: Value, kind: Int64) -> Result[(), Value] {
        let r = obj.asRef;
        if let .Some(p) = self.ownProperty(r, k) {
            let (getter, setter) = self.accessorPair(p);
            let isPair = p.attrs & Attr.accessor != 0;
            let joins = isPair and ((kind == 2 and setter.isUndefined and not getter.isUndefined) or (kind == 1 and getter.isUndefined and not setter.isUndefined));
            if not joins { return self.throwType("Cannot initialize \(self.keyText(k)) twice on the same object"); }
            if kind == 2 { self.defineAccessorOwn(r, k, getter, f, 0); } else { self.defineAccessorOwn(r, k, f, setter, 0); }
            return .Ok(());
        }
        if kind == 0 {
            self.defineOwn(r, k, f, 0);
        } else if kind == 1 {
            self.defineAccessorOwn(r, k, f, Value.undefined, 0);
        } else {
            self.defineAccessorOwn(r, k, Value.undefined, f, 0);
        }
        .Ok(())
    }
}
