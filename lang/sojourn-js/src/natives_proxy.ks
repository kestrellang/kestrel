module js

// Proxy: the exotic object whose internal methods call a handler's
// traps, and the invariant checks that keep a proxy consistent with its
// target. Ordinary objects take the existing paths in object.ks; each
// internal method here checks for a proxy first.

extend VM {
    mutating func setupProxy() {
        let g = self.global;
        let p = self.nativeFunction(.ProxyCtor, "Proxy", 2, true);
        self.defineHidden(g.asRef, self.atom("Proxy"), p);
        self.method(p, "revocable", .ProxyRevocable, 2);
    }

    func isProxy(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) { .ProxyObject(_, _) => true, _ => false }
    }

    /// A live proxy's target and handler; a revoked one throws.
    mutating func proxyParts(p: Value) -> Result[(Value, Value), Value] {
        let (t, h) = match self.kindAt(p.asRef) {
            .ProxyObject(t, h) => (t, h),
            _ => return self.throwType("not a proxy")
        };
        if h.isNull { return self.throwType("Cannot perform operation on a revoked proxy"); }
        .Ok((t, h))
    }

    /// The handler's `name` trap, or undefined.
    mutating func proxyTrap(h: Value, name: String) -> Result[Value, Value] {
        let k = self.atom(name);
        self.getMethod(h, k)
    }

    mutating func proxyCreate(target: Value, handler: Value) -> Result[Value, Value] {
        if not target.isObject or not handler.isObject {
            return self.throwType("Cannot create proxy with a non-object as target or handler");
        }
        let p = self.newObjectKind(Value.nullValue, .ProxyObject(target: target, handler: handler));
        self.heap.objects(p.asRef).isConstructor = self.isConstructor(target);
        .Ok(p)
    }

    mutating func proxyNative(n: NativeFn, callee: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .ProxyCtor => {
                if newTarget.isUndefined { return self.throwType("Constructor Proxy requires 'new'"); }
                try self.proxyCreate(a0, a1)
            },
            .ProxyRevocable => {
                let p = try self.proxyCreate(a0, a1);
                self.pushRoot(p);
                let revoke = self.nativeFunction(.ProxyRevoke, "", 0, false);
                self.heap.objects(revoke.asRef).kind = .NativeClosure(fn: .ProxyRevoke, captures: [p]);
                self.pushRoot(revoke);
                let o = self.newObject();
                self.defineOwn(o.asRef, self.atom("proxy"), p, Attr.all);
                self.defineOwn(o.asRef, self.atom("revoke"), revoke, Attr.all);
                self.popRoots(2);
                o
            },
            .ProxyRevoke => {
                // The first call revokes; later calls do nothing.
                let p = self.captures(callee)(0);
                if p.isObject {
                    if let .ProxyObject(t, _) = self.kindAt(p.asRef) {
                        self.heap.objects(p.asRef).kind = .ProxyObject(target: t, handler: Value.nullValue);
                    }
                    self.heap.objects(callee.asRef).kind = .NativeClosure(fn: .ProxyRevoke, captures: [Value.nullValue]);
                }
                Value.undefined
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    // ------------------------------------------------------------------
    // Descriptors
    // ------------------------------------------------------------------

    /// An own property as a complete descriptor.
    func recordDescriptor(p: PropertyRecord) -> Descriptor {
        let e = .Some(p.attrs & Attr.enumerable != 0);
        let c = .Some(p.attrs & Attr.configurable != 0);
        if p.attrs & Attr.accessor != 0 {
            if let .AccessorPair(g, s) = self.kindAt(p.value.asRef) {
                return Descriptor(value: .None, writable: .None, getter: .Some(g), setter: .Some(s), enumerable: e, configurable: c);
            }
        }
        Descriptor(value: .Some(p.value), writable: .Some(p.attrs & Attr.writable != 0), getter: .None, setter: .None, enumerable: e, configurable: c)
    }

    /// FromPropertyDescriptor: an object with just the fields `d` has.
    mutating func descriptorObject(d: Descriptor) -> Value {
        let o = self.newObject();
        let r = o.asRef;
        if let .Some(v) = d.value { self.defineOwn(r, self.names.value, v, Attr.all); }
        if let .Some(w) = d.writable { self.defineOwn(r, self.names.writable, Value.bool(w), Attr.all); }
        if let .Some(g) = d.getter { self.defineOwn(r, self.names.getKey, g, Attr.all); }
        if let .Some(s) = d.setter { self.defineOwn(r, self.names.setKey, s, Attr.all); }
        if let .Some(e) = d.enumerable { self.defineOwn(r, self.names.enumerable, Value.bool(e), Attr.all); }
        if let .Some(c) = d.configurable { self.defineOwn(r, self.names.configurable, Value.bool(c), Attr.all); }
        o
    }

    /// IsCompatiblePropertyDescriptor: ValidateAndApplyPropertyDescriptor
    /// without an object to apply to.
    func compatibleDescriptor(extensible: Bool, d: Descriptor, current: Optional[Descriptor]) -> Bool {
        let cur = match current {
            .Some(c) => c,
            .None => return extensible
        };
        if cur.configurable ?? false { return true; }
        if d.configurable ?? false { return false; }
        if let .Some(e) = d.enumerable { if e != (cur.enumerable ?? false) { return false; } }
        let generic = not isAccessorDescriptor(d) and not isDataDescriptor(d);
        if generic { return true; }
        if isAccessorDescriptor(d) != isAccessorDescriptor(cur) { return false; }
        if isAccessorDescriptor(cur) {
            if let .Some(g) = d.getter { if not g.same(cur.getter ?? Value.undefined) { return false; } }
            if let .Some(s) = d.setter { if not s.same(cur.setter ?? Value.undefined) { return false; } }
            return true;
        }
        if not (cur.writable ?? false) {
            if d.writable ?? false { return false; }
            if let .Some(v) = d.value { if not self.sameValue(v, cur.value ?? Value.undefined) { return false; } }
        }
        true
    }

    // ------------------------------------------------------------------
    // The internal methods, for any object
    // ------------------------------------------------------------------

    /// [[GetPrototypeOf]].
    mutating func getProto(o: Value) -> Result[Value, Value] {
        if not self.isProxy(o) { return .Ok(self.heap.objects(o.asRef).proto); }
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "getPrototypeOf");
        if trap.isUndefined { return self.getProto(t); }
        let p = try self.callValue(trap, h, [t]);
        if not p.isObject and not p.isNull { return self.throwType("'getPrototypeOf' on proxy: trap returned neither object nor null"); }
        let targetExt = try self.isExtensibleV(t);
        if targetExt { return .Ok(p); }
        let tp = try self.getProto(t);
        if not p.same(tp) { return self.throwType("'getPrototypeOf' on proxy: proxy target is non-extensible but the trap did not return its actual prototype"); }
        .Ok(p)
    }

    /// [[SetPrototypeOf]].
    mutating func setProtoV(o: Value, proto: Value) -> Result[Bool, Value] {
        if not self.isProxy(o) { return self.setPrototype(o, proto); }
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "setPrototypeOf");
        if trap.isUndefined { return self.setProtoV(t, proto); }
        let ok = self.toBoolean(try self.callValue(trap, h, [t, proto]));
        if not ok { return .Ok(false); }
        let targetExt = try self.isExtensibleV(t);
        if targetExt { return .Ok(true); }
        let tp = try self.getProto(t);
        if not proto.same(tp) { return self.throwType("'setPrototypeOf' on proxy: trap returned truish for setting a new prototype on the non-extensible proxy target"); }
        .Ok(true)
    }

    /// [[IsExtensible]].
    mutating func isExtensibleV(o: Value) -> Result[Bool, Value] {
        if not self.isProxy(o) { return .Ok(self.heap.objects(o.asRef).extensible); }
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "isExtensible");
        if trap.isUndefined { return self.isExtensibleV(t); }
        let b = self.toBoolean(try self.callValue(trap, h, [t]));
        let targetExt = try self.isExtensibleV(t);
        if b != targetExt { return self.throwType("'isExtensible' on proxy: trap result does not reflect extensibility of proxy target"); }
        .Ok(b)
    }

    /// [[PreventExtensions]].
    mutating func preventExtensionsV(o: Value) -> Result[Bool, Value] {
        if not self.isProxy(o) {
            self.heap.objects(o.asRef).extensible = false;
            return .Ok(true);
        }
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "preventExtensions");
        if trap.isUndefined { return self.preventExtensionsV(t); }
        let b = self.toBoolean(try self.callValue(trap, h, [t]));
        let targetExt = try self.isExtensibleV(t);
        if b and targetExt { return self.throwType("'preventExtensions' on proxy: trap returned truish but the proxy target is extensible"); }
        .Ok(b)
    }

    /// [[GetOwnProperty]], as a complete descriptor.
    mutating func getOwnPropertyV(o: Value, key: Key) -> Result[Optional[Descriptor], Value] {
        if not self.isProxy(o) {
            if self.isNamespace(o) { let _ = try self.namespaceGet(o.asRef, key); }
            return match self.ownPropertyValue(o.asRef, key) {
                .Some(p) => .Ok(.Some(self.recordDescriptor(p))),
                .None => .Ok(.None)
            };
        }
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "getOwnPropertyDescriptor");
        if trap.isUndefined { return self.getOwnPropertyV(t, key); }
        let kv = self.keyValue(key);
        let res = try self.callValue(trap, h, [t, kv]);
        if not res.isObject and not res.isUndefined { return self.throwType("'getOwnPropertyDescriptor' on proxy: trap returned neither object nor undefined"); }
        let targetDesc = try self.getOwnPropertyV(t, key);
        if res.isUndefined {
            let td = match targetDesc {
                .Some(x) => x,
                .None => return .Ok(.None)
            };
            if not (td.configurable ?? false) { return self.throwType("'getOwnPropertyDescriptor' on proxy: trap returned undefined for a non-configurable property"); }
            let targetExt = try self.isExtensibleV(t);
            if not targetExt { return self.throwType("'getOwnPropertyDescriptor' on proxy: trap returned undefined for a property of a non-extensible target"); }
            return .Ok(.None);
        }
        let ext = try self.isExtensibleV(t);
        var d = try self.toDescriptor(res);
        // CompletePropertyDescriptor.
        if not isAccessorDescriptor(d) {
            if d.value.isNone { d.value = .Some(Value.undefined); }
            if d.writable.isNone { d.writable = .Some(false); }
        } else {
            if d.getter.isNone { d.getter = .Some(Value.undefined); }
            if d.setter.isNone { d.setter = .Some(Value.undefined); }
        }
        if d.enumerable.isNone { d.enumerable = .Some(false); }
        if d.configurable.isNone { d.configurable = .Some(false); }
        if not self.compatibleDescriptor(ext, d, targetDesc) { return self.throwType("'getOwnPropertyDescriptor' on proxy: trap returned a descriptor incompatible with the target's property"); }
        if not (d.configurable ?? false) {
            let td = match targetDesc {
                .Some(x) => x,
                .None => return self.throwType("'getOwnPropertyDescriptor' on proxy: trap reported non-configurability for a property that doesn't exist on the target")
            };
            if td.configurable ?? false { return self.throwType("'getOwnPropertyDescriptor' on proxy: trap reported non-configurability for a configurable property"); }
            if d.writable == .Some(false) and td.writable == .Some(true) {
                return self.throwType("'getOwnPropertyDescriptor' on proxy: trap reported non-writable for a writable property");
            }
        }
        .Ok(.Some(d))
    }

    /// [[DefineOwnProperty]] on a proxy.
    mutating func proxyDefine(o: Value, key: Key, d: Descriptor) -> Result[Bool, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "defineProperty");
        if trap.isUndefined { return self.defineProperty(t.asRef, key, d); }
        let kv = self.keyValue(key);
        let mark = self.roots.count;
        self.pushRoot(kv);
        let descObj = self.descriptorObject(d);
        self.pushRoot(descObj);
        let res = self.callValue(trap, h, [t, kv, descObj]);
        self.truncateRoots(mark);
        if not self.toBoolean(try res) { return .Ok(false); }
        let targetDesc = try self.getOwnPropertyV(t, key);
        let ext = try self.isExtensibleV(t);
        let settingConfigFalse = d.configurable == .Some(false);
        match targetDesc {
            .None => {
                if not ext { return self.throwType("'defineProperty' on proxy: trap returned truish for adding a property to a non-extensible target"); }
                if settingConfigFalse { return self.throwType("'defineProperty' on proxy: trap returned truish for defining a non-configurable property which doesn't exist on the target"); }
            },
            .Some(td) => {
                if not self.compatibleDescriptor(ext, d, targetDesc) { return self.throwType("'defineProperty' on proxy: trap returned truish for adding a property incompatible with the target's property"); }
                if settingConfigFalse and (td.configurable ?? false) { return self.throwType("'defineProperty' on proxy: trap returned truish for defining a non-configurable property which is configurable on the target"); }
                if isDataDescriptor(td) and not (td.configurable ?? false) and (td.writable ?? false) and d.writable == .Some(false) {
                    return self.throwType("'defineProperty' on proxy: trap returned truish for defining a non-writable property which is writable on the target");
                }
            }
        }
        .Ok(true)
    }

    /// [[HasProperty]] on a proxy.
    mutating func proxyHas(o: Value, key: Key) -> Result[Bool, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "has");
        if trap.isUndefined { return self.hasProperty(t, key); }
        let kv = self.keyValue(key);
        let b = self.toBoolean(try self.callValue(trap, h, [t, kv]));
        if not b {
            let targetDesc = try self.getOwnPropertyV(t, key);
            if let .Some(td) = targetDesc {
                if not (td.configurable ?? false) { return self.throwType("'has' on proxy: trap returned falsish for a non-configurable property"); }
                let targetExt = try self.isExtensibleV(t);
                if not targetExt { return self.throwType("'has' on proxy: trap returned falsish for a property of a non-extensible target"); }
            }
        }
        .Ok(b)
    }

    /// [[Get]] on a proxy.
    mutating func proxyGet(o: Value, key: Key, receiver: Value) -> Result[Value, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "get");
        if trap.isUndefined { return self.getFrom(t, key, receiver); }
        let kv = self.keyValue(key);
        let v = try self.callValue(trap, h, [t, kv, receiver]);
        let targetDesc = try self.getOwnPropertyV(t, key);
        if let .Some(td) = targetDesc {
            if not (td.configurable ?? false) {
                if isDataDescriptor(td) and not (td.writable ?? false) and not self.sameValue(v, td.value ?? Value.undefined) {
                    return self.throwType("'get' on proxy: property '\(self.keyText(key))' is a read-only and non-configurable data property on the target but the trap did not return its actual value");
                }
                if isAccessorDescriptor(td) and (td.getter ?? Value.undefined).isUndefined and not v.isUndefined {
                    return self.throwType("'get' on proxy: property '\(self.keyText(key))' is a non-configurable accessor without a getter on the target but the trap did not return undefined");
                }
            }
        }
        .Ok(v)
    }

    /// [[Set]] on a proxy.
    mutating func proxySet(o: Value, key: Key, value: Value, receiver: Value) -> Result[Bool, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "set");
        if trap.isUndefined { return self.ordinarySet(t, key, value, receiver); }
        let kv = self.keyValue(key);
        let b = self.toBoolean(try self.callValue(trap, h, [t, kv, value, receiver]));
        if not b { return .Ok(false); }
        let targetDesc = try self.getOwnPropertyV(t, key);
        if let .Some(td) = targetDesc {
            if not (td.configurable ?? false) {
                if isDataDescriptor(td) and not (td.writable ?? false) and not self.sameValue(value, td.value ?? Value.undefined) {
                    return self.throwType("'set' on proxy: trap returned truish for property '\(self.keyText(key))' which exists in the proxy target as a non-configurable and non-writable data property with a different value");
                }
                if isAccessorDescriptor(td) and (td.setter ?? Value.undefined).isUndefined {
                    return self.throwType("'set' on proxy: trap returned truish for property '\(self.keyText(key))' which exists in the proxy target as a non-configurable and non-writable accessor property without a setter");
                }
            }
        }
        .Ok(true)
    }

    /// [[Delete]] on a proxy.
    mutating func proxyDelete(o: Value, key: Key) -> Result[Bool, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "deleteProperty");
        if trap.isUndefined { return self.deleteOwn(t.asRef, key); }
        let kv = self.keyValue(key);
        let b = self.toBoolean(try self.callValue(trap, h, [t, kv]));
        if not b { return .Ok(false); }
        let td = match try self.getOwnPropertyV(t, key) {
            .Some(x) => x,
            .None => return .Ok(true)
        };
        if not (td.configurable ?? false) { return self.throwType("'deleteProperty' on proxy: trap returned truish for property '\(self.keyText(key))' which is non-configurable in the proxy target"); }
        let targetExt = try self.isExtensibleV(t);
        if not targetExt { return self.throwType("'deleteProperty' on proxy: trap returned truish for property '\(self.keyText(key))' but the proxy target is non-extensible"); }
        .Ok(true)
    }

    /// [[OwnPropertyKeys]] on a proxy.
    mutating func proxyOwnKeys(o: Value) -> Result[Array[Key], Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "ownKeys");
        if trap.isUndefined { return self.ownKeys(t.asRef); }
        let arr = try self.callValue(trap, h, [t]);
        if not arr.isObject { return self.throwType("CreateListFromArrayLike called on non-object"); }
        self.pushRoot(arr);
        let len = try self.lengthOf(arr);
        var keys: [Key] = [];
        var seen: Set[Key] = Set();
        for i in 0..<len {
            let v = try self.getIndex(arr, i);
            if not v.isString and not v.isSymbol { return self.throwType("\(self.describe(v)) is not a valid property name"); }
            let k = try self.toKey(v);
            if seen.contains(k) { return self.throwType("'ownKeys' on proxy: trap returned duplicate entries"); }
            let _ = seen.insert(k);
            keys.append(k);
        }
        let _ = self.roots.pop();
        let ext = try self.isExtensibleV(t);
        let targetKeys = try self.ownKeys(t.asRef);
        var configurable: [Key] = [];
        var nonconfigurable: [Key] = [];
        for k in targetKeys {
            let d = try self.getOwnPropertyV(t, k);
            if let .Some(x) = d {
                if not (x.configurable ?? false) { nonconfigurable.append(k); continue; }
            }
            configurable.append(k);
        }
        if ext and nonconfigurable.isEmpty { return .Ok(keys); }
        var unchecked = seen;
        for k in nonconfigurable {
            if not unchecked.contains(k) { return self.throwType("'ownKeys' on proxy: trap result did not include '\(self.keyText(k))'"); }
            let _ = unchecked.remove(k);
        }
        if ext { return .Ok(keys); }
        for k in configurable {
            if not unchecked.contains(k) { return self.throwType("'ownKeys' on proxy: trap result did not include '\(self.keyText(k))'"); }
            let _ = unchecked.remove(k);
        }
        if not unchecked.isEmpty { return self.throwType("'ownKeys' on proxy: trap returned extra keys but proxy target is non-extensible"); }
        .Ok(keys)
    }

    /// [[Call]] on a proxy.
    mutating func proxyCall(o: Value, this: Value, args: Array[Value]) -> Result[Value, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "apply");
        if trap.isUndefined { return self.callValue(t, this, args); }
        let a = self.newArray(args);
        self.pushRoot(a);
        let r = self.callValue(trap, h, [t, this, a]);
        let _ = self.roots.pop();
        r
    }

    /// [[Construct]] on a proxy.
    mutating func proxyConstruct(o: Value, args: Array[Value], newTarget: Value) -> Result[Value, Value] {
        let (t, h) = try self.proxyParts(o);
        let trap = try self.proxyTrap(h, "construct");
        if trap.isUndefined { return self.construct(t, args, newTarget); }
        let a = self.newArray(args);
        self.pushRoot(a);
        let r = self.callValue(trap, h, [t, a, newTarget]);
        let _ = self.roots.pop();
        let v = try r;
        if not v.isObject { return self.throwType("'construct' on proxy: trap returned non-object"); }
        .Ok(v)
    }
}
