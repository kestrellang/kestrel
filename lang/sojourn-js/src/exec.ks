module js

// One bytecode instruction at a time.

extend VM {
    mutating func step(instr: Instr, fi: Int64, stop: Int64) -> Result[Step, Value] {
        match instr {
            .Load(dst, value) => self.setReg(fi, dst, value),
            .LoadString(dst, atom) => self.setReg(fi, dst, self.atomValues(atom)),
            .Move(dst, src) => self.setReg(fi, dst, self.reg(fi, src)),
            .Nop => {},

            // Variables
            .GetVar(dst, hops, slot, name) => {
                let e = self.envAt(self.frames(fi).env, hops);
                let v = self.heap.envs(e).slots(slot);
                if v.isEmpty { return .Err(self.tdzError(name)); }
                self.setReg(fi, dst, v);
            },
            .SetVar(hops, slot, src, name) => {
                let e = self.envAt(self.frames(fi).env, hops);
                if self.heap.envs(e).slots(slot).isEmpty { return .Err(self.tdzError(name)); }
                self.heap.envs(e).slots(slot) = self.reg(fi, src);
            },
            .WithGet(dst, this, hops, name, target) => {
                let obj = self.heap.envs(self.envAt(self.frames(fi).env, hops)).slots(0);
                let has = try self.withHas(obj, name);
                if has {
                    if dst >= 0 {
                        let v = try self.getV(obj, name);
                        self.setReg(fi, dst, v);
                    }
                    if this >= 0 { self.setReg(fi, this, obj); }
                    self.frames(fi).pc = target;
                }
            },
            .WithSet(hops, name, src, strict, target) => {
                let obj = self.heap.envs(self.envAt(self.frames(fi).env, hops)).slots(0);
                let has = try self.withHas(obj, name);
                if has {
                    try self.setV(obj, name, self.reg(fi, src), strict);
                    self.frames(fi).pc = target;
                }
            },
            .InitVar(hops, slot, src) => {
                let e = self.envAt(self.frames(fi).env, hops);
                self.heap.envs(e).slots(slot) = self.reg(fi, src);
            },
            .GetGlobal(dst, name, forTypeof) => {
                let v = try self.getGlobal(name, forTypeof);
                self.setReg(fi, dst, v);
            },
            .SetGlobal(name, src, strict) => try self.setGlobal(name, self.reg(fi, src), strict),
            .DeclareGlobalVar(name) => {
                if not self.hasOwn(self.global.asRef, name) {
                    self.defineOwn(self.global.asRef, name, Value.undefined, Attr.writable | Attr.enumerable);
                }
            },
            .DeclareGlobalFunction(name, src) => {
                self.defineOwn(self.global.asRef, name, self.reg(fi, src), Attr.writable | Attr.enumerable);
            },
            .DeclareGlobalLexical(name, isConst) => {
                if let .Some(_) = self.globalLexicals(name) {
                    return .Err(self.makeError(.SyntaxError, "Identifier '\(self.keyText(name))' has already been declared"));
                }
                let slot = self.heap.envs(self.globalEnv).slots.count;
                self.heap.envs(self.globalEnv).slots.append(Value.empty);
                let _ = self.globalLexicals.insert(name, slot);
                if isConst { let _ = self.globalConsts.insert(name); }
            },
            .InitGlobalLexical(name, src) => {
                let slot = self.globalLexicals(name) ?? 0;
                self.heap.envs(self.globalEnv).slots(slot) = self.reg(fi, src);
            },
            .PushEnv(size) => {
                var slots: [Value] = [];
                for _ in 0..<size { slots.append(Value.undefined); }
                let e = self.heap.allocEnv(Env(slots: slots, parent: self.frames(fi).env));
                self.frames(fi).env = e;
            },
            .PopEnv => {
                let e = self.frames(fi).env;
                self.frames(fi).env = self.heap.envs(e).parent;
            },
            .CopyEnv => {
                let e = self.frames(fi).env;
                let copy = self.heap.envs(e);
                self.frames(fi).env = self.heap.allocEnv(copy);
            },

            // Operators
            .Binary(op, dst, a, b) => {
                let x = self.reg(fi, a);
                let y = self.reg(fi, b);
                if x.isNumber and y.isNumber {
                    let fast = match op {
                        .Add => Value.number(x.asNumber + y.asNumber),
                        .Sub => Value.number(x.asNumber - y.asNumber),
                        .Lt => Value.bool(x.asNumber < y.asNumber),
                        .StrictEq => Value.bool(x.asNumber == y.asNumber),
                        _ => try self.binary(op, x, y)
                    };
                    self.setReg(fi, dst, fast);
                } else {
                    let r = try self.binary(op, x, y);
                    self.setReg(fi, dst, r);
                }
            },
            .Unary(op, dst, src) => {
                let r = try self.unary(op, self.reg(fi, src));
                self.setReg(fi, dst, r);
            },
            .Jump(target) => {
                if target < self.frames(fi).pc and self.heap.wantsCollection() { self.collectGarbage(); }
                self.frames(fi).pc = target;
            },
            .JumpIf(cond, target, when) => {
                if self.toBoolean(self.reg(fi, cond)) == when { self.frames(fi).pc = target; }
            },
            .JumpIfNullish(src, target) => {
                if self.reg(fi, src).isNullish { self.frames(fi).pc = target; }
            },
            .JumpIfUndefined(src, target) => {
                if self.reg(fi, src).isUndefined { self.frames(fi).pc = target; }
            },
            .JumpIfNotUndefined(src, target) => {
                if not self.reg(fi, src).isUndefined { self.frames(fi).pc = target; }
            },

            // Properties
            .GetProp(dst, obj, key, ic) => {
                let o = self.reg(fi, obj);
                if o.isObject {
                    let c = self.caches(ic);
                    let r = o.asRef;
                    if self.heap.objects(r).shape == c.shape {
                        self.setReg(fi, dst, self.heap.objects(r).slots(c.slot));
                        return .Ok(.Next);
                    }
                    let shape = self.heap.objects(r).shape;
                    let slot = self.shapeSlot(shape, key);
                    if slot >= 0 and self.shapes(shape).attrs(slot) & Attr.accessor == 0 and not hasExoticKeys(self.heap.objects(r).kind) {
                        self.caches(ic) = InlineCache(shape: shape, slot: slot);
                        self.setReg(fi, dst, self.heap.objects(r).slots(slot));
                        return .Ok(.Next);
                    }
                }
                let v = try self.getV(o, key);
                self.setReg(fi, dst, v);
            },
            .SetProp(obj, key, src, ic, strict) => {
                let o = self.reg(fi, obj);
                let v = self.reg(fi, src);
                if o.isObject {
                    let c = self.caches(ic);
                    let r = o.asRef;
                    if self.heap.objects(r).shape == c.shape {
                        self.heap.objects(r).slots(c.slot) = v;
                        return .Ok(.Next);
                    }
                    let shape = self.heap.objects(r).shape;
                    let slot = self.shapeSlot(shape, key);
                    if slot >= 0 and self.shapes(shape).attrs(slot) & (Attr.accessor | Attr.writable) == Attr.writable and not hasExoticKeys(self.heap.objects(r).kind) {
                        self.caches(ic) = InlineCache(shape: shape, slot: slot);
                        self.heap.objects(r).slots(slot) = v;
                        return .Ok(.Next);
                    }
                }
                try self.setV(o, key, v, strict);
            },
            .GetElem(dst, obj, key) => {
                let o = self.reg(fi, obj);
                let k = self.reg(fi, key);
                // Fast path: array[index].
                if o.isObject and k.isNumber {
                    if let .ArrayObject = self.kindAt(o.asRef) {
                        let n = k.asNumber;
                        if let .Some(i) = n.toInt64() {
                            if i >= 0 and Float64(from: i) == n and i < self.heap.objects(o.asRef).elements.count {
                                let e = self.heap.objects(o.asRef).elements(i);
                                if not e.isEmpty {
                                    self.setReg(fi, dst, e);
                                    return .Ok(.Next);
                                }
                            }
                        }
                    }
                }
                if o.isNullish {
                    let what = if o.isNull { "null" } else { "undefined" };
                    return self.throwType("Cannot read properties of \(what)");
                }
                let pk = try self.toKey(k);
                let v = if self.isPrivateKey(pk) { try self.privateGet(o, pk) } else { try self.getV(o, pk) };
                self.setReg(fi, dst, v);
            },
            .SetElem(obj, key, src, strict) => {
                let o = self.reg(fi, obj);
                let k = self.reg(fi, key);
                let v = self.reg(fi, src);
                if o.isNullish {
                    let what = if o.isNull { "null" } else { "undefined" };
                    return self.throwType("Cannot set properties of \(what)");
                }
                let pk = try self.toKey(k);
                if self.isPrivateKey(pk) {
                    try self.privateSet(o, pk, v);
                } else {
                    try self.setV(o, pk, v, strict);
                }
            },
            .DeleteElem(dst, obj, key, strict) => {
                let o = try self.toObject(self.reg(fi, obj));
                let pk = try self.toKey(self.reg(fi, key));
                let ok = self.deleteOwn(o.asRef, pk);
                if not ok and strict { return self.throwType("Cannot delete property '\(self.keyText(pk))'"); }
                self.setReg(fi, dst, Value.bool(ok));
            },
            .SuperGet(dst, key, this) => {
                let home = self.homeOf(fi);
                if not home.isObject { return self.throwType("'super' outside a method"); }
                let proto = self.heap.objects(home.asRef).proto;
                let pk = try self.toKey(self.reg(fi, key));
                if not proto.isObject { return self.throwType("super prototype is null"); }
                let v = try self.getFrom(proto, pk, self.reg(fi, this));
                self.setReg(fi, dst, v);
            },
            .SuperSet(key, src, this) => {
                let home = self.homeOf(fi);
                if not home.isObject { return self.throwType("'super' outside a method"); }
                let proto = self.heap.objects(home.asRef).proto;
                let pk = try self.toKey(self.reg(fi, key));
                try self.setOn(proto, pk, self.reg(fi, src), self.reg(fi, this), true);
            },
            .NewObject(dst) => {
                let o = self.newObject();
                self.setReg(fi, dst, o);
            },
            .NewRegExp(dst, pattern, flags) => {
                let p = self.atomValues(pattern);
                let f = self.atomValues(flags);
                let rx = try self.regexpCreate(p, f);
                self.setReg(fi, dst, rx);
            },
            .NewArray(dst) => {
                let a = self.newArray([]);
                self.setReg(fi, dst, a);
            },
            .ArrayAppend(arr, src) => {
                let r = self.reg(fi, arr).asRef;
                self.appendElement(r, self.reg(fi, src));
            },
            .ArraySpread(arr, src) => {
                let items = try self.iterateToArray(self.reg(fi, src));
                let r = self.reg(fi, arr).asRef;
                for x in items { self.appendElement(r, x); }
            },
            .DefineProp(obj, key, src) => {
                let pk = try self.toKey(self.reg(fi, key));
                let o = self.reg(fi, obj);
                if self.isPrivateKey(pk) {
                    if self.hasOwn(o.asRef, pk) { return self.throwType("Cannot initialize \(self.keyText(pk)) twice on the same object"); }
                    self.defineOwn(o.asRef, pk, self.reg(fi, src), Attr.writable);
                } else {
                    self.defineOwn(o.asRef, pk, self.reg(fi, src), Attr.all);
                }
            },
            .NewPrivateName(dst, name) => {
                let d = self.atomValues(name);
                let s = self.newSymbol(d);
                let _ = self.privateSymbols.insert(s.asRef);
                self.setReg(fi, dst, s);
            },
            .PrivateMethod(obj, key, fn, kind) => try self.privateMethodAdd(self.reg(fi, obj), symbolKey(self.reg(fi, key).asRef), self.reg(fi, fn), kind),
            .DefineMethod(obj, key, fn, enumerable) => {
                let k = self.reg(fi, key);
                let pk = try self.toKey(k);
                let f = self.reg(fi, fn);
                self.nameFunctionIfAnonymous(f, pk, "");
                let attrs = Attr.writable | Attr.configurable | (if enumerable { Attr.enumerable } else { 0 });
                self.defineOwn(self.reg(fi, obj).asRef, pk, f, attrs);
            },
            .DefineAccessor(obj, key, fn, isGetter, enumerable) => {
                let pk = try self.toKey(self.reg(fi, key));
                let f = self.reg(fi, fn);
                self.nameFunctionIfAnonymous(f, pk, if isGetter { "get " } else { "set " });
                let r = self.reg(fi, obj).asRef;
                var getter = Value.undefined;
                var setter = Value.undefined;
                if let .Some(p) = self.ownProperty(r, pk) {
                    if p.attrs & Attr.accessor != 0 {
                        if let .AccessorPair(g, s) = self.kindAt(p.value.asRef) {
                            getter = g;
                            setter = s;
                        }
                    }
                }
                if isGetter { getter = f; } else { setter = f; }
                let attrs = Attr.configurable | (if enumerable { Attr.enumerable } else { 0 });
                self.defineAccessorOwn(r, pk, getter, setter, attrs);
            },
            .CopyDataProps(dst, src, excluded, count) => {
                var skip: [Key] = [];
                for i in 0..<count { skip.append(try self.toKey(self.reg(fi, excluded + i))); }
                try self.copyDataProperties(self.reg(fi, dst), self.reg(fi, src), skip);
            },
            .SetProto(obj, proto) => {
                let p = self.reg(fi, proto);
                if p.isObject or p.isNull {
                    let r = self.reg(fi, obj).asRef;
                    self.heap.objects(r).proto = p;
                }
            },

            // Functions
            .MakeClosure(dst, code, home) => {
                var h = if home >= 0 { self.reg(fi, home) } else { Value.undefined };
                if home < 0 and self.codes(code).isArrow { h = self.homeOf(fi); }
                let f = self.makeClosureObject(code, self.frames(fi).env, h);
                self.setReg(fi, dst, f);
            },
            .Call(dst, callee, this, args, argc) => {
                var argv: [Value] = [];
                for i in 0..<argc { argv.append(self.reg(fi, args + i)); }
                let f = self.reg(fi, callee);
                if not self.isCallable(f) { return self.throwType("\(self.calleeName(fi, callee)) is not a function"); }
                try self.callFromBytecode(fi, dst, f, self.reg(fi, this), argv);
            },
            .CallSpread(dst, callee, this, args) => {
                let argv = self.arrayElements(self.reg(fi, args));
                let f = self.reg(fi, callee);
                if not self.isCallable(f) { return self.throwType("\(self.calleeName(fi, callee)) is not a function"); }
                try self.callFromBytecode(fi, dst, f, self.reg(fi, this), argv);
            },
            .New(dst, callee, args) => {
                let argv = self.arrayElements(self.reg(fi, args));
                try self.newFromBytecode(fi, dst, self.reg(fi, callee), argv);
            },
            .SuperCall(dst, args, hops, slot) => {
                let argv = self.arrayElements(self.reg(fi, args));
                let callee = self.frames(fi).callee;
                let parent = self.heap.objects(callee.asRef).proto;
                if not self.isConstructor(parent) { return self.throwType("Super constructor is not a constructor"); }
                let newTarget = self.frames(fi).newTarget;
                let result = try self.construct(parent, argv, newTarget);
                let e = self.envAt(self.frames(fi).env, hops);
                if not self.heap.envs(e).slots(slot).isEmpty {
                    return .Err(self.makeError(.ReferenceError, "Super constructor may only be called once"));
                }
                self.heap.envs(e).slots(slot) = result;
                self.setReg(fi, dst, result);
            },
            .Return(src) => return self.doReturn(self.reg(fi, src), stop),
            .Throw(src) => return .Err(self.reg(fi, src)),
            .ThrowError(kind, message) => return .Err(self.makeError(kind, message)),
            .GetArg(dst, index) => {
                let args = self.frames(fi).args;
                self.setReg(fi, dst, if index < args.count { args(index) } else { Value.undefined });
            },
            .GetRestArgs(dst, from) => {
                let args = self.frames(fi).args;
                var rest: [Value] = [];
                var i = from;
                while i < args.count {
                    rest.append(args(i));
                    i = i + 1;
                }
                let a = self.newArray(rest);
                self.setReg(fi, dst, a);
            },
            .CreateArguments(dst) => {
                let a = self.makeArguments(fi);
                self.setReg(fi, dst, a);
            },
            .NewTarget(dst) => self.setReg(fi, dst, self.frames(fi).newTarget),
            .MakeClass(ctor, proto, superclass) => {
                let p = try self.makeClass(self.reg(fi, ctor), if superclass >= 0 { self.reg(fi, superclass) } else { Value.empty });
                self.setReg(fi, proto, p);
            },
            .SetClassFields(ctor, fn) => {
                let f = self.reg(fi, ctor);
                if let .Closure(code, env, home, _) = self.kindAt(f.asRef) {
                    self.heap.objects(f.asRef).kind = .Closure(code: code, env: env, home: home, fields: self.reg(fi, fn));
                }
            },
            .InitFields(this) => {
                let callee = self.frames(fi).callee;
                if let .Closure(_, _, _, fields) = self.kindAt(callee.asRef) {
                    if not fields.isUndefined { let _ = try self.callValue(fields, self.reg(fi, this), []); }
                }
            },

            // Exceptions
            .EnterTry(handler, exception) => {
                let env = self.frames(fi).env;
                self.frames(fi).handlers.append(Handler(pc: handler, exception: exception, env: env));
            },
            .LeaveTry => { let _ = self.frames(fi).handlers.pop(); },

            // Generators
            .GeneratorStart => return self.startGenerator(fi, stop),
            .Yield(dst, src, raw) => {
                let v = self.reg(fi, src);
                let gen = self.frames(fi).generator;
                let id = match self.kindAt(gen.asRef) { .Generator(x) => x, _ => -1 };
                self.suspendFrame(fi, id);
                self.heap.generators(id).state = GenState.suspendedYield;
                self.heap.generators(id).resumeReg = dst;
                self.heap.generators(id).raw = raw;
                self.popFrame();
                return .Ok(.Done(v));
            },
            .RequireObjectResult(src) => {
                let v = self.reg(fi, src);
                if not v.isObject { return self.throwType("Iterator result \(self.describe(v)) is not an object"); }
            },

            // Iteration
            .ForInPrepare(dst, obj) => {
                let it = try self.forInIterator(self.reg(fi, obj));
                self.setReg(fi, dst, it);
            },
            .ForInNext(dst, iter, done) => {
                let it = self.reg(fi, iter);
                match self.forInNext(it) {
                    .Some(k) => self.setReg(fi, dst, k),
                    .None => self.frames(fi).pc = done
                }
            },
            .GetIterator(iter, obj) => {
                let (it, next) = try self.getIterator(self.reg(fi, obj));
                self.setReg(fi, iter, it);
                self.setReg(fi, iter + 1, next);
            },
            .GetAsyncIterator(iter, obj) => {
                let (it, next) = try self.getAsyncIterator(self.reg(fi, obj));
                self.setReg(fi, iter, it);
                self.setReg(fi, iter + 1, next);
            },
            .IteratorNext(dst, iter, done) => {
                match try self.iteratorStep(self.reg(fi, iter), self.reg(fi, iter + 1)) {
                    .Some(v) => self.setReg(fi, dst, v),
                    .None => self.frames(fi).pc = done
                }
            },
            .IteratorClose(iter) => try self.iteratorClose(self.reg(fi, iter))
        }
        .Ok(.Next)
    }

    mutating func tdzError(name: Key) -> Value {
        let n = self.keyText(name);
        if n == "this" {
            return self.makeError(.ReferenceError, "Must call super constructor in derived class before accessing 'this'");
        }
        self.makeError(.ReferenceError, "Cannot access '\(n)' before initialization")
    }

    /// A rough name for the callee in register `r`, for "is not a function".
    mutating func calleeName(fi: Int64, r: Int64) -> String {
        self.describe(self.reg(fi, r))
    }

    mutating func getGlobal(name: Key, forTypeof: Bool) -> Result[Value, Value] {
        if let .Some(slot) = self.globalLexicals(name) {
            let v = self.heap.envs(self.globalEnv).slots(slot);
            if v.isEmpty { return .Err(self.tdzError(name)); }
            return .Ok(v);
        }
        if self.hasProperty(self.global, name) { return self.getFrom(self.global, name, self.global); }
        if forTypeof { return .Ok(Value.undefined); }
        .Err(self.makeError(.ReferenceError, "\(self.keyText(name)) is not defined"))
    }

    /// HasBinding for a `with` object: `name` is on `obj` and its
    /// @@unscopables doesn't list it.
    mutating func withHas(obj: Value, name: Key) -> Result[Bool, Value] {
        if not self.hasProperty(obj, name) { return .Ok(false); }
        let u = try self.getV(obj, symbolKey(WellKnown.unscopables));
        if not u.isObject { return .Ok(true); }
        let blocked = try self.getV(u, name);
        .Ok(not self.toBoolean(blocked))
    }

    mutating func setGlobal(name: Key, v: Value, strict: Bool) -> Result[(), Value] {
        if let .Some(slot) = self.globalLexicals(name) {
            if self.heap.envs(self.globalEnv).slots(slot).isEmpty { return .Err(self.tdzError(name)); }
            if self.globalConsts.contains(name) { return self.throwType("Assignment to constant variable '\(self.keyText(name))'."); }
            self.heap.envs(self.globalEnv).slots(slot) = v;
            return .Ok(());
        }
        if strict and not self.hasProperty(self.global, name) {
            return .Err(self.makeError(.ReferenceError, "\(self.keyText(name)) is not defined"));
        }
        self.setOn(self.global, name, v, self.global, strict)
    }

    /// Gives an anonymous function the name of the key it's stored under.
    mutating func nameFunctionIfAnonymous(f: Value, key: Key, prefix: String) {
        if not f.isObject { return; }
        if let .Some(p) = self.ownProperty(f.asRef, self.names.name) {
            if p.value.isString and self.units(p.value).count > 0 { return; }
        }
        let base = if keyIsSymbol(key) {
            let d = self.symbols(symbolOfKey(key));
            if d.isString { "[" + self.text(d) + "]" } else { "" }
        } else {
            self.atomNames(key)
        };
        let n = self.str("\(prefix)\(base)");
        self.defineReadOnly(f.asRef, "name", n);
    }

    /// Sets up a class: links the constructor and a new prototype to the
    /// parent class. `parent` is empty when there's no `extends`.
    mutating func makeClass(ctor: Value, parent: Value) -> Result[Value, Value] {
        var protoParent = self.intrinsics.objectProto;
        var ctorParent = self.intrinsics.functionProto;
        if not parent.isEmpty {
            if parent.isNull {
                protoParent = Value.nullValue;
            } else {
                if not self.isConstructor(parent) { return self.throwType("Class extends value is not a constructor or null"); }
                let pp = try self.getFrom(parent, self.names.prototype, parent);
                if not pp.isObject and not pp.isNull { return self.throwType("Class extends value does not have valid prototype property"); }
                protoParent = pp;
                ctorParent = parent;
            }
        }
        let proto = self.newObjectKind(protoParent, .Ordinary);
        let r = ctor.asRef;
        self.heap.objects(r).proto = ctorParent;
        self.defineOwn(r, self.names.prototype, proto, 0);
        self.defineHidden(proto.asRef, self.names.constructor, ctor);
        self.setHome(ctor, proto);
        .Ok(proto)
    }

    mutating func copyDataProperties(target: Value, src: Value, excluded: Array[Key]) -> Result[(), Value] {
        if src.isNullish { return .Ok(()); }
        let from = try self.toObject(src);
        for k in self.ownKeys(from.asRef) {
            if excluded.contains(k) { continue; }
            if let .Some(p) = self.ownProperty(from.asRef, k) {
                if p.attrs & Attr.enumerable == 0 { continue; }
                let v = try self.getFrom(from, k, from);
                self.defineOwn(target.asRef, k, v, Attr.all);
            }
        }
        .Ok(())
    }
}

/// Objects whose own keys aren't all in their shape, which inline caches
/// skip.
func hasExoticKeys(kind: ObjectKind) -> Bool {
    match kind {
        .ArrayObject => true,
        .StringBox(_) => true,
        _ => false
    }
}
