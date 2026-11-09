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
        let o = JSObject(shape: 0, slots: [], proto: proto, elements: [], kind: kind, extensible: true, isConstructor: false);
        Value.object(self.heap.allocObject(o))
    }

    mutating func newObject() -> Value { self.newObjectKind(self.intrinsics.objectProto, .Ordinary) }

    mutating func newArray(elements: Array[Value]) -> Value {
        let a = self.newObjectKind(self.intrinsics.arrayProto, .ArrayObject);
        self.heap.objects(a.asRef).elements = elements;
        a
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

    func isCallable(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .Closure(_, _, _, _) => true,
            .NativeFunction(_) => true,
            .BoundFunction(_, _, _) => true,
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
                    return .None;
                }
                if self.isLengthKey(key) {
                    return .Some(PropertyRecord(value: Value.int(o.elements.count), attrs: Attr.writable));
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
            _ => {}
        }
        let slot = self.shapeSlot(o.shape, key);
        if slot < 0 { return .None; }
        .Some(PropertyRecord(value: o.slots(slot), attrs: self.shapes(o.shape).attrs(slot)))
    }

    /// Like `ownProperty`, but fills in a string wrapper's characters.
    mutating func ownPropertyValue(r: Ref, key: Key) -> Optional[PropertyRecord] {
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
                self.setElement(r, i, value);
                return;
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
        if let .ArrayObject = self.kindAt(r) {
            let i = self.indexOfKey(key);
            if i >= 0 {
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

    mutating func setElement(r: Ref, i: Int64, value: Value) {
        while self.heap.objects(r).elements.count <= i {
            self.heap.objects(r).elements.append(Value.empty);
        }
        self.heap.objects(r).elements(i) = value;
    }

    mutating func setArrayLength(r: Ref, n: Int64) {
        while self.heap.objects(r).elements.count > n {
            let _ = self.heap.objects(r).elements.pop();
        }
        while self.heap.objects(r).elements.count < n {
            self.heap.objects(r).elements.append(Value.empty);
        }
    }

    /// [[Delete]]: false for a non-configurable property.
    mutating func deleteOwn(r: Ref, key: Key) -> Bool {
        let o = self.heap.objects(r);
        match o.kind {
            .ArrayObject => {
                let i = self.indexOfKey(key);
                if i >= 0 {
                    if i < o.elements.count { self.heap.objects(r).elements(i) = Value.empty; }
                    return true;
                }
                if self.isLengthKey(key) { return false; }
            },
            .StringBox(s) => {
                let i = self.indexOfKey(key);
                if (i >= 0 and i < self.units(s).count) or self.isLengthKey(key) { return false; }
            },
            _ => {}
        }
        let slot = self.shapeSlot(o.shape, key);
        if slot < 0 { return true; }
        if self.shapes(o.shape).attrs(slot) & Attr.configurable == 0 { return false; }
        self.reshape(r, key, -1);
        true
    }

    /// [[OwnPropertyKeys]]: array indices ascending, then strings in
    /// creation order, then symbols.
    mutating func ownKeys(r: Ref) -> Array[Key] {
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
            _ => {}
        }
        let shape = self.shapes(o.shape);
        for k in shape.keys {
            if self.indexOfKey(k) >= 0 { extraIndexKeys.append(k); }
        }
        var fromShape: [Int64] = [];
        for k in extraIndexKeys { fromShape.append(self.indexOfKey(k)); }
        fromShape.sort();
        for i in indices { out.append(self.internIndex(i)); }
        for i in fromShape { out.append(self.internIndex(i)); }
        match o.kind {
            .ArrayObject => out.append(self.names.length),
            .StringBox(_) => out.append(self.names.length),
            _ => {}
        }
        for k in shape.keys {
            if not keyIsSymbol(k) and self.indexOfKey(k) < 0 { out.append(k); }
        }
        for k in shape.keys {
            if keyIsSymbol(k) { out.append(k); }
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
        var cur = obj;
        while cur.isObject {
            let r = cur.asRef;
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
        let rr = receiver.asRef;
        if let .Some(p) = self.ownProperty(rr, key) {
            if p.attrs & Attr.accessor != 0 or p.attrs & Attr.writable == 0 { return .Ok(false); }
            self.writeOwn(rr, key, value);
            return .Ok(true);
        }
        if not self.heap.objects(rr).extensible { return .Ok(false); }
        if let .ArrayObject = self.kindAt(rr) {
            let i = self.indexOfKey(key);
            if i >= 0 and i > self.heap.objects(rr).elements.count + 10000000 {
                return .Err(self.makeError(.RangeError, "sparse arrays aren't supported yet"));
            }
        }
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

    mutating func hasProperty(obj: Value, key: Key) -> Bool {
        var cur = obj;
        while cur.isObject {
            if self.hasOwn(cur.asRef, key) { return true; }
            cur = self.heap.objects(cur.asRef).proto;
        }
        false
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
