module js

// The Iterator global and iterator helpers (ECMA-262 §27.1.3).
//
// Lazy helpers (map, filter, take, drop, flatMap) return an iterator
// helper object whose `IterHelper` state drives `helperNext`; the eager
// ones (reduce, toArray, forEach, some, every, find) run to completion.
// Iterator.from wraps iterators that don't inherit from
// %Iterator.prototype% in a `WrappedIterator`.

/// `IterHelper.kind`.
struct HelperKind {
    static var map: Int64 { 0 }
    static var filter: Int64 { 1 }
    static var take: Int64 { 2 }
    static var drop: Int64 { 3 }
    static var flatMap: Int64 { 4 }
}

/// `IterHelper.state`.
struct HelperState {
    static var suspendedStart: Int64 { 0 }
    static var running: Int64 { 1 }
    static var suspended: Int64 { 2 }
    static var completed: Int64 { 3 }
}

/// A lazy iterator helper's state: the underlying iterator and its
/// `next`, the callback, the index passed to it, take/drop's remaining
/// count, and flatMap's current inner iterator (undefined when none).
struct IterHelper {
    var kind: Int64
    var iter: Value
    var next: Value
    var fn: Value
    var counter: Int64
    var remaining: Float64
    var inner: Value
    var innerNext: Value
    var state: Int64
}

extend VM {
    mutating func setupIterator() {
        let proto = self.intrinsics.iteratorProto;
        let c = self.nativeFunction(.IteratorCtor, "Iterator", 0, true);
        self.defineOwn(c.asRef, self.names.prototype, proto, 0);
        self.defineHidden(self.global.asRef, self.atom("Iterator"), c);
        self.method(c, "from", .IteratorFrom, 1);
        // `constructor` and @@toStringTag are accessors whose setters
        // ignore %Iterator.prototype% itself (web compatibility).
        let ctorGet = self.nativeFunction(.IteratorCtorGet, "get constructor", 0, false);
        let ctorSet = self.nativeFunction(.IteratorCtorSet, "set constructor", 1, false);
        self.defineAccessorOwn(proto.asRef, self.names.constructor, ctorGet, ctorSet, Attr.configurable);
        let tagGet = self.nativeFunction(.IteratorTagGet, "get [Symbol.toStringTag]", 0, false);
        let tagSet = self.nativeFunction(.IteratorTagSet, "set [Symbol.toStringTag]", 1, false);
        self.defineAccessorOwn(proto.asRef, symbolKey(WellKnown.toStringTag), tagGet, tagSet, Attr.configurable);
        for (name, n, len) in [
            ("map", NativeFn.IteratorMap, 1), ("filter", NativeFn.IteratorFilter, 1),
            ("take", NativeFn.IteratorTake, 1), ("drop", NativeFn.IteratorDrop, 1),
            ("flatMap", NativeFn.IteratorFlatMap, 1), ("reduce", NativeFn.IteratorReduce, 1),
            ("toArray", NativeFn.IteratorToArray, 0), ("forEach", NativeFn.IteratorForEach, 1),
            ("some", NativeFn.IteratorSome, 1), ("every", NativeFn.IteratorEvery, 1),
            ("find", NativeFn.IteratorFind, 1)
        ] {
            self.method(proto, name, n, len);
        }

        let helperProto = self.newObjectKind(proto, .Ordinary);
        self.method(helperProto, "next", .IteratorHelperNext, 0);
        self.method(helperProto, "return", .IteratorHelperReturn, 0);
        let tag = self.str("Iterator Helper");
        self.defineOwn(helperProto.asRef, symbolKey(WellKnown.toStringTag), tag, Attr.configurable);
        self.intrinsics.iteratorHelperProto = helperProto;

        let wrapProto = self.newObjectKind(proto, .Ordinary);
        self.method(wrapProto, "next", .WrappedIteratorNext, 0);
        self.method(wrapProto, "return", .WrappedIteratorReturn, 0);
        self.intrinsics.wrapForValidIteratorProto = wrapProto;
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    /// GetIteratorDirect: `obj` and its `next`, read once.
    mutating func iteratorDirect(obj: Value) -> Result[(Value, Value), Value] {
        let next = try self.getV(obj, self.names.next);
        .Ok((obj, next))
    }

    /// Closes `it` because of error `e`, then fails with `e`.
    mutating func closeWith[T](it: Value, e: Value) -> Result[T, Value] {
        let _ = self.iteratorClose(it);
        .Err(e)
    }

    /// The `this` and callback checks every helper starts with: `this`
    /// must be an object, and `fn` callable (else it's closed).
    mutating func helperReceiver(this: Value, fn: Value, name: String) -> Result[(), Value] {
        if not this.isObject { return self.throwType("Iterator.prototype.\(name) called on a non-object"); }
        if not self.isCallable(fn) {
            let e = self.typeError("\(self.describe(fn)) is not a function");
            return self.closeWith(this, e);
        }
        .Ok(())
    }

    /// take/drop's limit: a non-negative integer or +Infinity.
    mutating func helperLimit(this: Value, limit: Value) -> Result[Float64, Value] {
        if not this.isObject { return self.throwType("Iterator.prototype method called on a non-object"); }
        let n = match self.toNumber(limit) {
            .Ok(x) => x,
            .Err(e) => return self.closeWith(this, e)
        };
        if n.isNaN { return self.closeWith(this, self.makeError(.RangeError, "\(numberToString(n)) must be a number")); }
        let i = toIntegerOrInfinityOf(n);
        // Above 2^53 - 1 only +Infinity is allowed.
        let tooBig = i > 9007199254740991.0 and i != Float64.infinity;
        if i < 0.0 or tooBig { return self.closeWith(this, self.makeError(.RangeError, "\(numberToString(i)) is out of range")); }
        .Ok(i)
    }

    mutating func newHelper(kind: Int64, this: Value, fn: Value, remaining: Float64) -> Result[Value, Value] {
        let (it, next) = try self.iteratorDirect(this);
        let h = IterHelper(kind: kind, iter: it, next: next, fn: fn, counter: 0, remaining: remaining,
                           inner: Value.undefined, innerNext: Value.undefined, state: HelperState.suspendedStart);
        .Ok(self.newObjectKind(self.intrinsics.iteratorHelperProto, .IteratorHelper(h)))
    }

    func helperOf(v: Value) -> Optional[IterHelper] {
        if not v.isObject { return .None; }
        match self.kindAt(v.asRef) {
            .IteratorHelper(h) => .Some(h),
            _ => .None
        }
    }

    mutating func setHelper(v: Value, h: IterHelper) { self.heap.objects(v.asRef).kind = .IteratorHelper(h); }

    /// %IteratorHelperPrototype%.next.
    mutating func helperNext(this: Value) -> Result[Value, Value] {
        var h = match self.helperOf(this) {
            .Some(x) => x,
            .None => return self.throwType("Iterator Helper next called on incompatible receiver")
        };
        if h.state == HelperState.running { return self.throwType("Iterator Helper is already running"); }
        if h.state == HelperState.completed { return .Ok(self.iterResult(Value.undefined, true)); }
        h.state = HelperState.running;
        self.setHelper(this, h);
        let r = self.helperStep(this, h);
        // The step saved its progress; settle the state around it.
        var after = self.helperOf(this) ?? h;
        match r {
            .Ok(.Some(v)) => {
                after.state = HelperState.suspended;
                self.setHelper(this, after);
                .Ok(self.iterResult(v, false))
            },
            .Ok(.None) => {
                after.state = HelperState.completed;
                self.setHelper(this, after);
                .Ok(self.iterResult(Value.undefined, true))
            },
            .Err(e) => {
                after.state = HelperState.completed;
                self.setHelper(this, after);
                .Err(e)
            }
        }
    }

    /// The next value of a helper, or None when it's done. Errors from
    /// the callback (or a flatMap inner iterator) close the underlying
    /// iterator; errors from it don't.
    mutating func helperStep(this: Value, start: IterHelper) -> Result[Optional[Value], Value] {
        var h = start;
        if h.kind == HelperKind.take {
            if h.remaining == 0.0 {
                try self.iteratorClose(h.iter);
                return .Ok(.None);
            }
            if h.remaining != Float64.infinity { h.remaining = h.remaining - 1.0; }
            self.setHelper(this, h);
            return self.iteratorStep(h.iter, h.next);
        }
        if h.kind == HelperKind.drop {
            while h.remaining > 0.0 {
                if h.remaining != Float64.infinity { h.remaining = h.remaining - 1.0; }
                self.setHelper(this, h);
                let skipped = try self.iteratorStep(h.iter, h.next);
                if skipped.isNone { return .Ok(.None); }
            }
            return self.iteratorStep(h.iter, h.next);
        }
        if h.kind == HelperKind.flatMap { return self.flatMapStep(this, h); }
        while true {
            let step = try self.iteratorStep(h.iter, h.next);
            let v = match step {
                .Some(x) => x,
                .None => return .Ok(.None)
            };
            let i = Value.int(h.counter);
            h.counter = h.counter + 1;
            self.setHelper(this, h);
            let r = match self.callValue(h.fn, Value.undefined, [v, i]) {
                .Ok(x) => x,
                .Err(e) => return self.closeWith(h.iter, e)
            };
            if h.kind == HelperKind.map { return .Ok(.Some(r)); }
            if self.toBoolean(r) { return .Ok(.Some(v)); }
        }
        .Ok(.None)
    }

    mutating func flatMapStep(this: Value, start: IterHelper) -> Result[Optional[Value], Value] {
        var h = start;
        while true {
            if not h.inner.isUndefined {
                let step = match self.iteratorStep(h.inner, h.innerNext) {
                    .Ok(s) => s,
                    .Err(e) => return self.closeWith(h.iter, e)
                };
                if let .Some(v) = step { return .Ok(.Some(v)); }
                h.inner = Value.undefined;
                h.innerNext = Value.undefined;
                self.setHelper(this, h);
            }
            let outer = try self.iteratorStep(h.iter, h.next);
            let v = match outer {
                .Some(x) => x,
                .None => return .Ok(.None)
            };
            let i = Value.int(h.counter);
            h.counter = h.counter + 1;
            self.setHelper(this, h);
            let mapped = match self.callValue(h.fn, Value.undefined, [v, i]) {
                .Ok(x) => x,
                .Err(e) => return self.closeWith(h.iter, e)
            };
            let (inner, innerNext) = match self.iteratorFlattenable(mapped, false) {
                .Ok(p) => p,
                .Err(e) => return self.closeWith(h.iter, e)
            };
            h.inner = inner;
            h.innerNext = innerNext;
            self.setHelper(this, h);
        }
        .Ok(.None)
    }

    /// %IteratorHelperPrototype%.return.
    mutating func helperReturn(this: Value) -> Result[Value, Value] {
        var h = match self.helperOf(this) {
            .Some(x) => x,
            .None => return self.throwType("Iterator Helper return called on incompatible receiver")
        };
        if h.state == HelperState.running { return self.throwType("Iterator Helper is already running"); }
        if h.state == HelperState.completed { return .Ok(self.iterResult(Value.undefined, true)); }
        let inner = h.inner;
        h.state = HelperState.completed;
        h.inner = Value.undefined;
        self.setHelper(this, h);
        if not inner.isUndefined {
            if let .Err(e) = self.iteratorClose(inner) { return self.closeWith(h.iter, e); }
        }
        try self.iteratorClose(h.iter);
        .Ok(self.iterResult(Value.undefined, true))
    }

    /// GetIteratorFlattenable: `obj`'s iterator, or `obj` itself when it
    /// has no @@iterator. Strings count only when `iterateStrings`.
    mutating func iteratorFlattenable(obj: Value, iterateStrings: Bool) -> Result[(Value, Value), Value] {
        if not obj.isObject and not (iterateStrings and obj.isString) {
            return self.throwType("\(self.describe(obj)) is not an iterator or iterable");
        }
        let method = try self.getMethod(obj, symbolKey(WellKnown.iterator));
        var it = obj;
        if not method.isUndefined { it = try self.callValue(method, obj, []); }
        if not it.isObject { return self.throwType("\(self.describe(it)) is not an object"); }
        self.iteratorDirect(it)
    }

    /// SetterThatIgnoresPrototypeProperties for %Iterator.prototype%'s
    /// `constructor` and @@toStringTag.
    mutating func iteratorProtoSetter(this: Value, key: Key, v: Value) -> Result[Value, Value] {
        if not this.isObject { return self.throwType("Iterator.prototype setter called on a non-object"); }
        if this.same(self.intrinsics.iteratorProto) { return self.throwType("Cannot assign to this property of Iterator.prototype"); }
        if self.hasOwn(this.asRef, key) {
            try self.setV(this, key, v, true);
        } else {
            if not self.heap.objects(this.asRef).extensible { return self.throwType("Cannot define property on a non-extensible object"); }
            self.defineOwn(this.asRef, key, v, Attr.all);
        }
        .Ok(Value.undefined)
    }

    // ------------------------------------------------------------------
    // Natives
    // ------------------------------------------------------------------

    mutating func iteratorNative(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let r: Value = match n {
            .IteratorCtor => {
                if newTarget.isUndefined or newTarget.same(callee) { return self.throwType("Iterator is an abstract class"); }
                let proto = try self.prototypeFor(newTarget, self.intrinsics.iteratorProto);
                self.newObjectKind(proto, .Ordinary)
            },
            .IteratorFrom => {
                let (it, next) = try self.iteratorFlattenable(a0, true);
                let iterCtor = self.ownValue(self.global.asRef, self.atom("Iterator"));
                let inherits = try self.ordinaryHasInstance(iterCtor, it);
                if inherits { return .Ok(.Some(it)); }
                self.newObjectKind(self.intrinsics.wrapForValidIteratorProto, .WrappedIterator(iter: it, next: next))
            },
            .WrappedIteratorNext or .WrappedIteratorReturn => {
                let (it, next) = match self.kindAt(this.asRef) {
                    .WrappedIterator(i, nx) => (i, nx),
                    _ => return self.throwType("not a wrapped iterator")
                };
                if n == NativeFn.WrappedIteratorNext { return .Ok(.Some(try self.callValue(next, it, []))); }
                let ret = try self.getMethod(it, self.names.returnKey);
                if ret.isUndefined { return .Ok(.Some(self.iterResult(Value.undefined, true))); }
                try self.callValue(ret, it, [])
            },
            .IteratorCtorGet => self.ownValue(self.global.asRef, self.atom("Iterator")),
            .IteratorCtorSet => try self.iteratorProtoSetter(this, self.names.constructor, a0),
            .IteratorTagGet => self.str("Iterator"),
            .IteratorTagSet => try self.iteratorProtoSetter(this, symbolKey(WellKnown.toStringTag), a0),
            .IteratorMap or .IteratorFilter or .IteratorFlatMap => {
                let name = if n == NativeFn.IteratorMap { "map" } else if n == NativeFn.IteratorFilter { "filter" } else { "flatMap" };
                try self.helperReceiver(this, a0, name);
                let kind = if n == NativeFn.IteratorMap { HelperKind.map } else if n == NativeFn.IteratorFilter { HelperKind.filter } else { HelperKind.flatMap };
                try self.newHelper(kind, this, a0, 0.0)
            },
            .IteratorTake or .IteratorDrop => {
                let limit = try self.helperLimit(this, a0);
                let kind = if n == NativeFn.IteratorTake { HelperKind.take } else { HelperKind.drop };
                try self.newHelper(kind, this, Value.undefined, limit)
            },
            .IteratorHelperNext => try self.helperNext(this),
            .IteratorHelperReturn => try self.helperReturn(this),
            .IteratorReduce => {
                try self.helperReceiver(this, a0, "reduce");
                let (it, next) = try self.iteratorDirect(this);
                var counter: Int64 = 0;
                var acc = arg(args, 1);
                if args.count < 2 {
                    let first = try self.iteratorStep(it, next);
                    acc = match first {
                        .Some(v) => v,
                        .None => return self.throwType("Reduce of empty iterator with no initial value")
                    };
                    counter = 1;
                }
                while true {
                    let step = try self.iteratorStep(it, next);
                    let v = match step { .Some(x) => x, .None => break };
                    self.pushRoot(acc);
                    acc = match self.callValue(a0, Value.undefined, [acc, v, Value.int(counter)]) {
                        .Ok(x) => x,
                        .Err(e) => return self.closeWith(it, e)
                    };
                    counter = counter + 1;
                }
                acc
            },
            .IteratorToArray => {
                if not this.isObject { return self.throwType("Iterator.prototype.toArray called on a non-object"); }
                let (it, next) = try self.iteratorDirect(this);
                let out = self.newArray([]);
                self.pushRoot(out);
                while true {
                    let step = try self.iteratorStep(it, next);
                    let v = match step { .Some(x) => x, .None => break };
                    self.appendElement(out.asRef, v);
                }
                out
            },
            .IteratorForEach or .IteratorSome or .IteratorEvery or .IteratorFind => {
                let name = match n { .IteratorForEach => "forEach", .IteratorSome => "some", .IteratorEvery => "every", _ => "find" };
                try self.helperReceiver(this, a0, name);
                let (it, next) = try self.iteratorDirect(this);
                var counter: Int64 = 0;
                while true {
                    let step = try self.iteratorStep(it, next);
                    let v = match step { .Some(x) => x, .None => break };
                    let r = match self.callValue(a0, Value.undefined, [v, Value.int(counter)]) {
                        .Ok(x) => x,
                        .Err(e) => return self.closeWith(it, e)
                    };
                    counter = counter + 1;
                    let truthy = self.toBoolean(r);
                    if n == NativeFn.IteratorSome and truthy {
                        try self.iteratorClose(it);
                        return .Ok(.Some(Value.bool(true)));
                    }
                    if n == NativeFn.IteratorEvery and not truthy {
                        try self.iteratorClose(it);
                        return .Ok(.Some(Value.bool(false)));
                    }
                    if n == NativeFn.IteratorFind and truthy {
                        try self.iteratorClose(it);
                        return .Ok(.Some(v));
                    }
                }
                match n {
                    .IteratorSome => Value.bool(false),
                    .IteratorEvery => Value.bool(true),
                    _ => Value.undefined
                }
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }
}
