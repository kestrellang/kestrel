module js

// Object and Function built-ins, and property descriptors.

/// A property descriptor as passed to `Object.defineProperty`; each
/// field may be absent.
struct Descriptor {
    var value: Optional[Value]
    var writable: Optional[Bool]
    var getter: Optional[Value]
    var setter: Optional[Value]
    var enumerable: Optional[Bool]
    var configurable: Optional[Bool]
}

func isAccessorDescriptor(d: Descriptor) -> Bool {
    match (d.getter, d.setter) {
        (.None, .None) => false,
        _ => true
    }
}

func isDataDescriptor(d: Descriptor) -> Bool {
    match (d.value, d.writable) {
        (.None, .None) => false,
        _ => true
    }
}

extend VM {
    mutating func toDescriptor(v: Value) -> Result[Descriptor, Value] {
        if not v.isObject { return self.throwType("Property description must be an object"); }
        var d = Descriptor(value: .None, writable: .None, getter: .None, setter: .None, enumerable: .None, configurable: .None);
        let hp4 = try self.hasProperty(v, self.names.enumerable);
        if hp4 { d.enumerable = .Some(self.toBoolean(try self.getFrom(v, self.names.enumerable, v))); }
        let hp5 = try self.hasProperty(v, self.names.configurable);
        if hp5 { d.configurable = .Some(self.toBoolean(try self.getFrom(v, self.names.configurable, v))); }
        let hp6 = try self.hasProperty(v, self.names.value);
        if hp6 { d.value = .Some(try self.getFrom(v, self.names.value, v)); }
        let hp7 = try self.hasProperty(v, self.names.writable);
        if hp7 { d.writable = .Some(self.toBoolean(try self.getFrom(v, self.names.writable, v))); }
        let hp8 = try self.hasProperty(v, self.names.getKey);
        if hp8 {
            let g = try self.getFrom(v, self.names.getKey, v);
            if not g.isUndefined and not self.isCallable(g) { return self.throwType("Getter must be a function"); }
            d.getter = .Some(g);
        }
        let hp9 = try self.hasProperty(v, self.names.setKey);
        if hp9 {
            let s = try self.getFrom(v, self.names.setKey, v);
            if not s.isUndefined and not self.isCallable(s) { return self.throwType("Setter must be a function"); }
            d.setter = .Some(s);
        }
        if isAccessorDescriptor(d) and isDataDescriptor(d) {
            return self.throwType("Invalid property descriptor. Cannot both specify accessors and a value or writable attribute");
        }
        .Ok(d)
    }

    /// The descriptor object for an own property, or undefined.
    mutating func fromProperty(r: Ref, key: Key) -> Value {
        let p = match self.ownPropertyValue(r, key) {
            .Some(x) => x,
            .None => return Value.undefined
        };
        let o = self.newObject();
        let d = o.asRef;
        if p.attrs & Attr.accessor != 0 {
            if let .AccessorPair(g, s) = self.kindAt(p.value.asRef) {
                self.defineOwn(d, self.names.getKey, g, Attr.all);
                self.defineOwn(d, self.names.setKey, s, Attr.all);
            }
        } else {
            self.defineOwn(d, self.names.value, p.value, Attr.all);
            self.defineOwn(d, self.names.writable, Value.bool(p.attrs & Attr.writable != 0), Attr.all);
        }
        self.defineOwn(d, self.names.enumerable, Value.bool(p.attrs & Attr.enumerable != 0), Attr.all);
        self.defineOwn(d, self.names.configurable, Value.bool(p.attrs & Attr.configurable != 0), Attr.all);
        o
    }

    /// ArraySetLength: `length` is writable, non-enumerable and
    /// non-configurable; shrinking deletes elements from the end and stops
    /// at a non-configurable one.
    mutating func arraySetLength(r: Ref, d: Descriptor) -> Result[Bool, Value] {
        // The new length is converted (ToUint32, then ToNumber, as the
        // spec has it) before the descriptor is validated.
        var len: Int64 = 0;
        if let .Some(v) = d.value {
            len = toUint32(try self.toNumber(v));
            let n = try self.toNumber(v);
            if Float64(from: len) != n { return .Err(self.makeError(.RangeError, "Invalid array length")); }
        }
        if d.configurable ?? false { return .Ok(false); }
        if d.enumerable ?? false { return .Ok(false); }
        if isAccessorDescriptor(d) { return .Ok(false); }
        let writable = self.heap.objects(r).lengthWritable;
        if d.value.isNone {
            if (d.writable ?? false) and not writable { return .Ok(false); }
            if d.writable == .Some(false) { self.heap.objects(r).lengthWritable = false; }
            return .Ok(true);
        }
        let old = self.heap.objects(r).length;
        if not writable {
            if d.writable ?? false { return .Ok(false); }
            return .Ok(len == old);
        }
        let keepWritable = d.writable != .Some(false);
        if len >= old {
            self.setArrayLength(r, len);
            if not keepWritable { self.heap.objects(r).lengthWritable = false; }
            return .Ok(true);
        }
        // Stop at the highest non-configurable element at or past `len`.
        var floor = len;
        for k in self.shapes(self.heap.objects(r).shape).keys {
            let i = self.indexOfKey(k);
            if i >= floor {
                let slot = self.shapeSlot(self.heap.objects(r).shape, k);
                if self.shapes(self.heap.objects(r).shape).attrs(slot) & Attr.configurable == 0 { floor = i + 1; }
            }
        }
        self.setArrayLength(r, floor);
        if not keepWritable { self.heap.objects(r).lengthWritable = false; }
        .Ok(floor == len)
    }

    /// ValidateAndApplyPropertyDescriptor: defines or changes `key` on
    /// `r`, returning false when the change isn't allowed.
    mutating func defineProperty(r: Ref, key: Key, d: Descriptor) -> Result[Bool, Value] {
        if let .ProxyObject(_, _) = self.kindAt(r) { return self.proxyDefine(Value.object(r), key, d); }
        // TypedArray [[DefineOwnProperty]] for numeric keys.
        if let .Some((ta, n)) = self.taKeyIndex(r, key) {
            if not self.taValidIndex(ta, n) { return .Ok(false); }
            if not (d.configurable ?? true) or not (d.enumerable ?? true) or isAccessorDescriptor(d) or not (d.writable ?? true) {
                return .Ok(false);
            }
            if let .Some(v) = d.value { try self.taSetElement(ta, n, v); }
            return .Ok(true);
        }
        let current = self.ownProperty(r, key);
        let isArrayObject = match self.kindAt(r) { .ArrayObject => true, _ => false };
        if isArrayObject and self.isLengthKey(key) { return self.arraySetLength(r, d); }
        match current {
            .None => {
                if not self.heap.objects(r).extensible { return .Ok(false); }
                if not self.canAddIndex(r, key) { return .Ok(false); }
                var attrs: Int64 = 0;
                if d.enumerable ?? false { attrs = attrs | Attr.enumerable; }
                if d.configurable ?? false { attrs = attrs | Attr.configurable; }
                if isAccessorDescriptor(d) {
                    self.defineAccessorOwn(r, key, d.getter ?? Value.undefined, d.setter ?? Value.undefined, attrs);
                } else {
                    if d.writable ?? false { attrs = attrs | Attr.writable; }
                    self.defineOwn(r, key, d.value ?? Value.undefined, attrs);
                }
                return .Ok(true);
            },
            .Some(p) => {
                let wasAccessor = p.attrs & Attr.accessor != 0;
                let configurable = p.attrs & Attr.configurable != 0;
                if not configurable {
                    if d.configurable ?? false { return .Ok(false); }
                    if let .Some(e) = d.enumerable {
                        if e != (p.attrs & Attr.enumerable != 0) { return .Ok(false); }
                    }
                    if isAccessorDescriptor(d) and not wasAccessor { return .Ok(false); }
                    if isDataDescriptor(d) and wasAccessor { return .Ok(false); }
                    if wasAccessor {
                        if let .AccessorPair(g, s) = self.kindAt(p.value.asRef) {
                            if let .Some(ng) = d.getter { if not ng.same(g) { return .Ok(false); } }
                            if let .Some(ns) = d.setter { if not ns.same(s) { return .Ok(false); } }
                        }
                    } else if p.attrs & Attr.writable == 0 {
                        if d.writable ?? false { return .Ok(false); }
                        if let .Some(v) = d.value { if not self.sameValue(v, p.value) { return .Ok(false); } }
                    }
                }
                var attrs: Int64 = p.attrs & (Attr.enumerable | Attr.configurable);
                if let .Some(e) = d.enumerable { attrs = if e { attrs | Attr.enumerable } else { attrs & (0 - 1 - Attr.enumerable) }; }
                if let .Some(c) = d.configurable { attrs = if c { attrs | Attr.configurable } else { attrs & (0 - 1 - Attr.configurable) }; }
                if isAccessorDescriptor(d) or (wasAccessor and not isDataDescriptor(d)) {
                    var g = Value.undefined;
                    var s = Value.undefined;
                    if wasAccessor {
                        if let .AccessorPair(og, os) = self.kindAt(p.value.asRef) {
                            g = og;
                            s = os;
                        }
                    }
                    if let .Some(ng) = d.getter { g = ng; }
                    if let .Some(ns) = d.setter { s = ns; }
                    self.defineAccessorOwn(r, key, g, s, attrs);
                } else {
                    var writable = if wasAccessor { false } else { p.attrs & Attr.writable != 0 };
                    if let .Some(w) = d.writable { writable = w; }
                    if writable { attrs = attrs | Attr.writable; }
                    let v = d.value ?? (if wasAccessor { Value.undefined } else { p.value });
                    self.defineOwn(r, key, v, attrs);
                }
                .Ok(true)
            }
        }
    }

    mutating func definePropertyOrThrow(o: Value, keyValue: Value, descValue: Value) -> Result[(), Value] {
        if not o.isObject { return self.throwType("Object.defineProperty called on non-object"); }
        let k = try self.toKey(keyValue);
        let d = try self.toDescriptor(descValue);
        if not (try self.defineProperty(o.asRef, k, d)) {
            return self.throwType("Cannot redefine property: \(self.keyText(k))");
        }
        .Ok(())
    }

    /// SetIntegrityLevel: freeze (`frozen`) or seal, returning false when
    /// the object refuses to stop being extensible.
    mutating func setIntegrity(o: Value, frozen: Bool) -> Result[Bool, Value] {
        if not (try self.preventExtensionsV(o)) { return .Ok(false); }
        let keys = try self.ownKeys(o.asRef);
        for k in keys {
            var d = Descriptor(value: .None, writable: .None, getter: .None, setter: .None, enumerable: .None, configurable: .Some(false));
            if frozen {
                let cur = try self.getOwnPropertyV(o, k);
                match cur {
                    .Some(c) => if not isAccessorDescriptor(c) { d.writable = .Some(false); },
                    .None => continue
                }
            }
            if not (try self.defineProperty(o.asRef, k, d)) { return self.throwType("Cannot redefine property: \(self.keyText(k))"); }
        }
        .Ok(true)
    }

    /// TestIntegrityLevel.
    mutating func testIntegrity(o: Value, frozen: Bool) -> Result[Bool, Value] {
        if not o.isObject { return .Ok(true); }
        if try self.isExtensibleV(o) { return .Ok(false); }
        let keys = try self.ownKeys(o.asRef);
        for k in keys {
            let cur = try self.getOwnPropertyV(o, k);
            if let .Some(c) = cur {
                if c.configurable ?? false { return .Ok(false); }
                if frozen and isDataDescriptor(c) and (c.writable ?? false) { return .Ok(false); }
            }
        }
        .Ok(true)
    }

    /// FromPropertyDescriptor of [[GetOwnProperty]]: a descriptor object
    /// or undefined.
    mutating func ownDescriptorObject(o: Value, key: Key) -> Result[Value, Value] {
        let d = try self.getOwnPropertyV(o, key);
        match d {
            .Some(x) => .Ok(self.descriptorObject(x)),
            .None => .Ok(Value.undefined)
        }
    }

    mutating func keysArray(o: Value, which: Int64) -> Result[Value, Value] {
        // which: 0 keys, 1 values, 2 entries
        let obj = try self.toObject(o);
        var out: [Value] = [];
        let ownKeyList3 = try self.ownKeys(obj.asRef);
        for k in ownKeyList3 {
            if keyIsSymbol(k) { continue; }
            let desc = try self.getOwnPropertyV(obj, k);
            if let .Some(p) = desc {
                if not (p.enumerable ?? false) { continue; }
                let kv = self.keyValue(k);
                if which == 0 {
                    out.append(kv);
                } else {
                    let v = try self.getFrom(obj, k, obj);
                    if which == 1 { out.append(v); } else { out.append(self.newArray([kv, v])); }
                }
            }
        }
        .Ok(self.newArray(out))
    }

    mutating func objectNative(n: NativeFn, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .ObjectCtor => {
                if not newTarget.isUndefined and not newTarget.same(self.global) {
                    let ctor = try self.getFrom(self.global, self.atom("Object"), self.global);
                    if not newTarget.same(ctor) {
                        let proto = try self.prototypeFor(newTarget, self.intrinsics.objectProto);
                        return .Ok(.Some(self.newObjectKind(proto, .Ordinary)));
                    }
                }
                if a0.isNullish { self.newObject() } else { try self.toObject(a0) }
            },
            .ObjectKeys => try self.keysArray(a0, 0),
            .ObjectValues => try self.keysArray(a0, 1),
            .ObjectEntries => try self.keysArray(a0, 2),
            .ObjectGetPrototypeOf => {
                let o = try self.toObject(a0);
                try self.getProto(o)
            },
            .ObjectSetPrototypeOf => {
                if a0.isNullish { return self.throwType("Object.setPrototypeOf called on null or undefined"); }
                if not a1.isObject and not a1.isNull { return self.throwType("Object prototype may only be an Object or null"); }
                if a0.isObject {
                    if not (try self.setProtoV(a0, a1)) { return self.throwType("Object.setPrototypeOf failed"); }
                }
                a0
            },
            .ObjectCreate => {
                if not a0.isObject and not a0.isNull { return self.throwType("Object prototype may only be an Object or null"); }
                let o = self.newObjectKind(a0, .Ordinary);
                if not a1.isUndefined { try self.defineProperties(o, a1); }
                o
            },
            .ObjectDefineProperty => {
                try self.definePropertyOrThrow(a0, a1, arg(args, 2));
                a0
            },
            .ObjectDefineProperties => {
                if not a0.isObject { return self.throwType("Object.defineProperties called on non-object"); }
                try self.defineProperties(a0, a1);
                a0
            },
            .ObjectGetOwnPropertyDescriptor => {
                let o = try self.toObject(a0);
                let k = try self.toKey(a1);
                try self.ownDescriptorObject(o, k)
            },
            .ObjectGetOwnPropertyDescriptors => {
                let o = try self.toObject(a0);
                let out = self.newObject();
                let ownKeyList4 = try self.ownKeys(o.asRef);
                for k in ownKeyList4 {
                    let d = try self.ownDescriptorObject(o, k);
                    if not d.isUndefined { self.defineOwn(out.asRef, k, d, Attr.all); }
                }
                out
            },
            .ObjectGetOwnPropertyNames => {
                let o = try self.toObject(a0);
                var out: [Value] = [];
                let ownKeyList5 = try self.ownKeys(o.asRef);
                for k in ownKeyList5 {
                    if not keyIsSymbol(k) { out.append(self.keyValue(k)); }
                }
                self.newArray(out)
            },
            .ObjectGetOwnPropertySymbols => {
                let o = try self.toObject(a0);
                var out: [Value] = [];
                let ownKeyList6 = try self.ownKeys(o.asRef);
                for k in ownKeyList6 {
                    if keyIsSymbol(k) { out.append(self.keyValue(k)); }
                }
                self.newArray(out)
            },
            .ObjectFreeze or .ObjectSeal => {
                if a0.isObject {
                    if not (try self.setIntegrity(a0, n == NativeFn.ObjectFreeze)) { return self.throwType("Cannot prevent extensions"); }
                }
                a0
            },
            .ObjectIsFrozen => Value.bool(try self.testIntegrity(a0, true)),
            .ObjectIsSealed => Value.bool(try self.testIntegrity(a0, false)),
            .ObjectPreventExtensions => {
                if a0.isObject {
                    if not (try self.preventExtensionsV(a0)) { return self.throwType("Cannot prevent extensions"); }
                }
                a0
            },
            .ObjectIsExtensible => Value.bool(if a0.isObject { try self.isExtensibleV(a0) } else { false }),
            .ObjectAssign => {
                let target = try self.toObject(a0);
                for i in 1..<args.count {
                    let src = args(i);
                    if src.isNullish { continue; }
                    let from = try self.toObject(src);
                    let ownKeyList7 = try self.ownKeys(from.asRef);
                    for k in ownKeyList7 {
                        let desc = try self.getOwnPropertyV(from, k);
                        if let .Some(p) = desc {
                            if not (p.enumerable ?? false) { continue; }
                            let v = try self.getFrom(from, k, from);
                            try self.setOn(target, k, v, target, true);
                        }
                    }
                }
                target
            },
            .ObjectIs => Value.bool(self.sameValue(a0, a1)),
            .ObjectFromEntries => try self.objectFromEntries(a0),
            .ObjectHasOwn => {
                let o = try self.toObject(a0);
                let k = try self.toKey(a1);
                let d = try self.getOwnPropertyV(o, k);
                Value.bool(d.isSome)
            },
            .HasOwnProperty => {
                let k = try self.toKey(a0);
                let o = try self.toObject(this);
                let d = try self.getOwnPropertyV(o, k);
                Value.bool(d.isSome)
            },
            .IsPrototypeOf => {
                if not a0.isObject { return .Ok(.Some(Value.bool(false))); }
                let o = try self.toObject(this);
                var cur = try self.getProto(a0);
                var found = false;
                while cur.isObject {
                    if cur.same(o) {
                        found = true;
                        break;
                    }
                    cur = try self.getProto(cur);
                }
                Value.bool(found)
            },
            .PropertyIsEnumerable => {
                let k = try self.toKey(a0);
                let o = try self.toObject(this);
                let d = try self.getOwnPropertyV(o, k);
                match d {
                    .Some(p) => Value.bool(p.enumerable ?? false),
                    .None => Value.bool(false)
                }
            },
            .ObjectToString => try self.objectToString(this),
            .ObjectToLocaleString => {
                let f = try self.getV(this, self.names.toString);
                try self.callValue(f, this, [])
            },
            .ObjectValueOf => try self.toObject(this),
            .ProtoGetter => {
                let o = try self.toObject(this);
                try self.getProto(o)
            },
            .ProtoSetter => {
                if this.isNullish { return self.throwType("Object.prototype.__proto__ called on null or undefined"); }
                if (a0.isObject or a0.isNull) and this.isObject {
                    if not (try self.setProtoV(this, a0)) { return self.throwType("Object.prototype.__proto__ setter failed"); }
                }
                Value.undefined
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    /// [[SetPrototypeOf]]: refuses cycles and non-extensible objects.
    mutating func setPrototype(o: Value, proto: Value) -> Result[Bool, Value] {
        let r = o.asRef;
        if self.heap.objects(r).proto.same(proto) { return .Ok(true); }
        // %Object.prototype% is an immutable prototype exotic object.
        if o.same(self.intrinsics.objectProto) { return .Ok(false); }
        if not self.heap.objects(r).extensible { return .Ok(false); }
        var cur = proto;
        while cur.isObject {
            if cur.same(o) { return .Ok(false); }
            if self.isProxy(cur) { break; }
            cur = self.heap.objects(cur.asRef).proto;
        }
        self.heap.objects(r).proto = proto;
        .Ok(true)
    }

    mutating func defineProperties(o: Value, props: Value) -> Result[(), Value] {
        let p = try self.toObject(props);
        var descs: [(Key, Descriptor)] = [];
        let ownKeyList8 = try self.ownKeys(p.asRef);
        for k in ownKeyList8 {
            let desc = try self.getOwnPropertyV(p, k);
            if let .Some(rec) = desc {
                if not (rec.enumerable ?? false) { continue; }
                let dv = try self.getFrom(p, k, p);
                descs.append((k, try self.toDescriptor(dv)));
            }
        }
        for (k, d) in descs {
            if not (try self.defineProperty(o.asRef, k, d)) {
                return self.throwType("Cannot redefine property: \(self.keyText(k))");
            }
        }
        .Ok(())
    }

    mutating func objectToString(this: Value) -> Result[Value, Value] {
        if this.isUndefined { return .Ok(self.str("[object Undefined]")); }
        if this.isNull { return .Ok(self.str("[object Null]")); }
        let o = try self.toObject(this);
        let isArr = try self.isArrayV(o);
        var tag = if isArr { "Array" } else { match self.kindAt(o.asRef) {
            .ErrorObject => "Error",
            .BooleanBox(_) => "Boolean",
            .NumberBox(_) => "Number",
            .StringBox(_) => "String",
            .ArgumentsObject => "Arguments",
            .DateObject(_) => "Date",
            .RegExpObject(_, _, _) => "RegExp",
            _ => if self.isCallable(o) { "Function" } else { "Object" }
        } };
        let t = try self.getFrom(o, symbolKey(WellKnown.toStringTag), o);
        if t.isString { tag = self.text(t); }
        .Ok(self.str("[object \(tag)]"))
    }

    mutating func functionNative(n: NativeFn, callee: Value, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let r: Value = match n {
            .FunctionCall => {
                let f = try self.requireCallable(this);
                var rest: [Value] = [];
                for i in 1..<args.count { rest.append(args(i)); }
                try self.callValue(f, a0, rest)
            },
            .FunctionApply => {
                let f = try self.requireCallable(this);
                let list = try self.argumentList(arg(args, 1));
                try self.callValue(f, a0, list)
            },
            .FunctionBind => {
                let f = try self.requireCallable(this);
                var bound: [Value] = [];
                for i in 1..<args.count { bound.append(args(i)); }
                let b = self.newObjectKind(self.heap.objects(f.asRef).proto, .BoundFunction(target: f, boundThis: a0, args: bound));
                self.heap.objects(b.asRef).isConstructor = self.isConstructor(f);
                var len = 0.0;
                if self.hasOwn(f.asRef, self.names.length) {
                    let l = try self.getFrom(f, self.names.length, f);
                    if l.isNumber {
                        let t = l.asNumber;
                        if t.isInfinite { len = if t > 0.0 { t } else { 0.0 }; }
                        else if not t.isNaN {
                            let x = t.trunc() - Float64(from: bound.count);
                            len = if x > 0.0 { x } else { 0.0 };
                        }
                    }
                }
                self.defineReadOnly(b.asRef, "length", Value.number(len));
                let nm = try self.getFrom(f, self.names.name, f);
                var u = unitsOf("bound ");
                if nm.isString {
                    for x in self.units(nm) { u.append(x); }
                }
                let s = self.newString(u);
                self.defineReadOnly(b.asRef, "name", s);
                b
            },
            .FunctionToString => {
                if not self.isCallable(this) { return self.throwType("Function.prototype.toString requires that 'this' be a Function"); }
                let nm = try self.getFrom(this, self.names.name, this);
                let name = if nm.isString { self.text(nm) } else { "" };
                self.str("function \(name)() { [native code] }")
            },
            .FunctionHasInstance => Value.bool(try self.ordinaryHasInstance(this, a0)),
            _ => return .Ok(.None)
        };
        let _ = callee;
        .Ok(.Some(r))
    }

    /// CreateListFromArrayLike, for `apply`.
    mutating func argumentList(v: Value) -> Result[Array[Value], Value] {
        if v.isNullish { return .Ok([]); }
        if not v.isObject { return self.throwType("CreateListFromArrayLike called on non-object"); }
        if self.isArray(v) { return .Ok(self.arrayElements(v)); }
        let len = try self.lengthOf(v);
        var out: [Value] = [];
        for i in 0..<len { out.append(try self.getIndex(v, i)); }
        .Ok(out)
    }
}
