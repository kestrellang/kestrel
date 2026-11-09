module js

// Iterator proposals: Iterator.concat, Iterator.zip / zipKeyed, and
// Iterator.prototype.chunks / windows / includes / join.
//
// The lazy ones are iterator helpers (natives_iterator.ks) with their own
// `HelperKind`, and keep their state in the shared `IterHelper` fields:
//
// - concat: `fn` is an array [iterable, method, ...], `counter` the next
//   pair, `inner`/`innerNext` the iterator being drained.
// - chunks/windows: `iter`/`next` underlying, `remaining` the size,
//   `counter` -1 once the underlying iterator is done; windows keeps its
//   buffer as an array in `extra` and "allow-partial" as `mode` 1.
// - zip: `fn` the iterators (null once a "longest" one finishes),
//   `inner` their next methods, `innerNext` the padding, `extra` an array
//   [open flags, keys or undefined], `mode` a ZipMode.

struct ZipMode {
    static var shortest: Int64 { 0 }
    static var longest: Int64 { 1 }
    static var strict: Int64 { 2 }
}

extend VM {
    mutating func setupIteratorProposals() {
        let c = self.ownValue(self.global.asRef, self.atom("Iterator"));
        self.method(c, "concat", .IteratorConcat, 0);
        self.method(c, "zip", .IteratorZip, 1);
        self.method(c, "zipKeyed", .IteratorZipKeyed, 1);
        let proto = self.intrinsics.iteratorProto;
        self.method(proto, "chunks", .IteratorChunks, 1);
        self.method(proto, "windows", .IteratorWindows, 1);
        self.method(proto, "includes", .IteratorIncludes, 1);
        self.method(proto, "join", .IteratorJoin, 1);
    }

    mutating func iteratorProposalNative(n: NativeFn, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .IteratorConcat => try self.iteratorConcat(args),
            .IteratorZip => try self.iteratorZip(a0, a1),
            .IteratorZipKeyed => try self.iteratorZipKeyed(a0, a1),
            .IteratorChunks => {
                let size = try self.chunkSizeArg(this, a0);
                let (it, next) = try self.iteratorDirect(this);
                var h = blankHelper(HelperKind.chunks);
                h.iter = it;
                h.next = next;
                h.remaining = size;
                self.newObjectKind(self.intrinsics.iteratorHelperProto, .IteratorHelper(h))
            },
            .IteratorWindows => {
                let size = try self.chunkSizeArg(this, a0);
                var mode: Int64 = 0;
                if not a1.isUndefined {
                    let partial = a1.isString and unitsEqual(self.units(a1), unitsOf("allow-partial"));
                    let full = a1.isString and unitsEqual(self.units(a1), unitsOf("only-full"));
                    if not partial and not full {
                        return self.closeWith(this, self.typeError("undersized must be \"only-full\" or \"allow-partial\""));
                    }
                    if partial { mode = 1; }
                }
                let (it, next) = try self.iteratorDirect(this);
                var h = blankHelper(HelperKind.windows);
                h.iter = it;
                h.next = next;
                h.remaining = size;
                h.mode = mode;
                h.extra = self.newArray([]);
                self.newObjectKind(self.intrinsics.iteratorHelperProto, .IteratorHelper(h))
            },
            .IteratorIncludes => try self.iteratorIncludes(this, a0, a1),
            .IteratorJoin => try self.iteratorJoin(this, a0),
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }

    // ------------------------------------------------------------------
    // Eager methods
    // ------------------------------------------------------------------

    mutating func iteratorIncludes(this: Value, search: Value, skip: Value) -> Result[Value, Value] {
        if not this.isObject { return self.throwType("Iterator.prototype.includes called on a non-object"); }
        var toSkip = 0.0;
        if not skip.isUndefined {
            let x = if skip.isNumber { skip.asNumber } else { Float64.nan };
            let integral = x.isInfinite or (not x.isNaN and x.floor() == x);
            if not integral { return self.closeWith(this, self.typeError("skippedElements must be an integer")); }
            if x < 0.0 { return self.closeWith(this, self.makeError(.RangeError, "skippedElements must not be negative")); }
            if not x.isInfinite and x > 9007199254740991.0 {
                return self.closeWith(this, self.makeError(.RangeError, "skippedElements is too large"));
            }
            toSkip = x;
        }
        let (it, next) = try self.iteratorDirect(this);
        var skipped = 0.0;
        while true {
            let v = match try self.iteratorStep(it, next) {
                .Some(x) => x,
                .None => break
            };
            if skipped < toSkip {
                skipped = skipped + 1.0;
                continue;
            }
            if self.sameValueZero(v, search) {
                try self.iteratorClose(it);
                return .Ok(Value.bool(true));
            }
        }
        .Ok(Value.bool(false))
    }

    mutating func iteratorJoin(this: Value, separator: Value) -> Result[Value, Value] {
        if not this.isObject { return self.throwType("Iterator.prototype.join called on a non-object"); }
        let sep = if separator.isUndefined { unitsOf(",") } else {
            match self.toUnits(separator) {
                .Ok(u) => u,
                .Err(e) => return self.closeWith(this, e)
            }
        };
        let (it, next) = try self.iteratorDirect(this);
        var out: Units = [];
        var first = true;
        while true {
            let v = match try self.iteratorStep(it, next) {
                .Some(x) => x,
                .None => break
            };
            if not first { for c in sep { out.append(c); } }
            first = false;
            if v.isNullish { continue; }
            let u = match self.toUnits(v) {
                .Ok(x) => x,
                .Err(e) => return self.closeWith(it, e)
            };
            for c in u { out.append(c); }
        }
        .Ok(self.newString(out))
    }

    /// chunks/windows's size: an integral Number in 1...2^32-1, checked
    /// without coercion; the receiver is closed when it's wrong.
    mutating func chunkSizeArg(this: Value, v: Value) -> Result[Float64, Value] {
        if not this.isObject { return self.throwType("Iterator.prototype method called on a non-object"); }
        let x = if v.isNumber { v.asNumber } else { Float64.nan };
        if x.isNaN or x.isInfinite or x.floor() != x {
            return self.closeWith(this, self.typeError("\(self.describe(v)) is not an integer"));
        }
        if x < 1.0 or x > 4294967295.0 { return self.closeWith(this, self.makeError(.RangeError, "\(numberToString(x)) is out of range")); }
        .Ok(x)
    }

    // ------------------------------------------------------------------
    // Lazy helpers
    // ------------------------------------------------------------------

    mutating func iteratorConcat(items: Array[Value]) -> Result[Value, Value] {
        var pairs: [Value] = [];
        for item in items {
            if not item.isObject { return self.throwType("Iterator.concat: \(self.describe(item)) is not an object"); }
            let m = try self.getMethod(item, symbolKey(WellKnown.iterator));
            if m.isUndefined { return self.throwType("Iterator.concat: \(self.describe(item)) is not iterable"); }
            self.pushRoot(m);
            pairs.append(item);
            pairs.append(m);
        }
        var h = blankHelper(HelperKind.concat);
        h.fn = self.newArray(pairs);
        .Ok(self.newObjectKind(self.intrinsics.iteratorHelperProto, .IteratorHelper(h)))
    }

    mutating func proposalHelperStep(this: Value, start: IterHelper) -> Result[Optional[Value], Value] {
        if start.kind == HelperKind.concat { return self.concatStep(this, start); }
        if start.kind == HelperKind.chunks { return self.chunksStep(this, start); }
        if start.kind == HelperKind.windows { return self.windowsStep(this, start); }
        self.zipStep(this, start)
    }

    mutating func concatStep(this: Value, start: IterHelper) -> Result[Optional[Value], Value] {
        var h = start;
        while true {
            if not h.inner.isUndefined {
                if let .Some(v) = try self.iteratorStep(h.inner, h.innerNext) { return .Ok(.Some(v)); }
                h.inner = Value.undefined;
                h.innerNext = Value.undefined;
                self.setHelper(this, h);
            }
            let pairs = self.arrayElements(h.fn);
            if h.counter * 2 >= pairs.count { return .Ok(.None); }
            let item = pairs(h.counter * 2);
            let method = pairs(h.counter * 2 + 1);
            h.counter = h.counter + 1;
            self.setHelper(this, h);
            let it = try self.callValue(method, item, []);
            if not it.isObject { return self.throwType("Iterator.concat: the iterator is not an object"); }
            let (inner, next) = try self.iteratorDirect(it);
            h.inner = inner;
            h.innerNext = next;
            self.setHelper(this, h);
        }
        .Ok(.None)
    }

    mutating func chunksStep(this: Value, start: IterHelper) -> Result[Optional[Value], Value] {
        var h = start;
        if h.counter < 0 { return .Ok(.None); }
        var buf: [Value] = [];
        while true {
            match try self.iteratorStep(h.iter, h.next) {
                .Some(v) => {
                    self.pushRoot(v);
                    buf.append(v);
                    if Float64(from: buf.count) == h.remaining { return .Ok(.Some(self.newArray(buf))); }
                },
                .None => {
                    h.counter = -1;
                    self.setHelper(this, h);
                    if buf.isEmpty { return .Ok(.None); }
                    return .Ok(.Some(self.newArray(buf)));
                }
            }
        }
        .Ok(.None)
    }

    mutating func windowsStep(this: Value, start: IterHelper) -> Result[Optional[Value], Value] {
        var h = start;
        if h.counter < 0 { return .Ok(.None); }
        while true {
            match try self.iteratorStep(h.iter, h.next) {
                .Some(v) => {
                    let old = self.arrayElements(h.extra);
                    // A full window slides by one.
                    let from: Int64 = if Float64(from: old.count) == h.remaining { 1 } else { 0 };
                    var buf: [Value] = [];
                    for i in from..<old.count { buf.append(old(i)); }
                    buf.append(v);
                    h.extra = self.newArray(buf);
                    self.setHelper(this, h);
                    if Float64(from: buf.count) == h.remaining { return .Ok(.Some(self.newArray(buf))); }
                },
                .None => {
                    h.counter = -1;
                    self.setHelper(this, h);
                    let buf = self.arrayElements(h.extra);
                    // "allow-partial": one short window when there was never a full one.
                    if h.mode == 1 and buf.count > 0 and Float64(from: buf.count) < h.remaining {
                        return .Ok(.Some(self.newArray(buf)));
                    }
                    return .Ok(.None);
                }
            }
        }
        .Ok(.None)
    }

    /// %IteratorHelperPrototype%.return for the proposal kinds: closes
    /// whatever the helper still has open.
    mutating func proposalHelperReturn(this: Value, start: IterHelper) -> Result[Value, Value] {
        // From suspended-start it's completed at once; from a yield it's
        // running while the closes run, so a re-entrant next/return throws.
        var h = start;
        h.state = if h.state == HelperState.suspendedStart { HelperState.completed } else { HelperState.running };
        self.setHelper(this, h);
        let r = self.closeHelperIterators(h);
        h.state = HelperState.completed;
        self.setHelper(this, h);
        try r;
        .Ok(self.iterResult(Value.undefined, true))
    }

    mutating func closeHelperIterators(h: IterHelper) -> Result[(), Value] {
        if h.kind == HelperKind.concat {
            if h.inner.isUndefined { return .Ok(()); }
            return self.iteratorClose(h.inner);
        }
        if h.kind == HelperKind.zip { return self.closeAllIterators(self.zipOpenIterators(h), .None); }
        if h.counter < 0 { return .Ok(()); }
        self.iteratorClose(h.iter)
    }

    // ------------------------------------------------------------------
    // zip
    // ------------------------------------------------------------------

    /// IteratorCloseAll: closes `iters` last to first. With an error the
    /// error wins; otherwise the first close that fails does.
    mutating func closeAllIterators(iters: Array[Value], error: Optional[Value]) -> Result[(), Value] {
        var err = error;
        var i = iters.count - 1;
        while i >= 0 {
            let r = self.iteratorClose(iters(i));
            if err.isNone {
                if let .Err(e) = r { err = .Some(e); }
            }
            i = i - 1;
        }
        match err {
            .Some(e) => .Err(e),
            .None => .Ok(())
        }
    }

    mutating func failClosingAll[T](iters: Array[Value], e: Value) -> Result[T, Value] {
        let _ = self.closeAllIterators(iters, .Some(e));
        .Err(e)
    }

    /// The mode and padding option of zip's options argument.
    mutating func zipOptions(options: Value) -> Result[(Int64, Value), Value] {
        if options.isUndefined { return .Ok((ZipMode.shortest, Value.undefined)); }
        if not options.isObject { return self.throwType("Iterator.zip options must be an object"); }
        let m = try self.getV(options, self.atom("mode"));
        var mode = ZipMode.shortest;
        if not m.isUndefined {
            let u = if m.isString { self.units(m) } else { unitsOf("") };
            if unitsEqual(u, unitsOf("longest")) {
                mode = ZipMode.longest;
            } else if unitsEqual(u, unitsOf("strict")) {
                mode = ZipMode.strict;
            } else if not unitsEqual(u, unitsOf("shortest")) {
                return self.throwType("mode must be \"shortest\", \"longest\" or \"strict\"");
            }
        }
        if mode != ZipMode.longest { return .Ok((mode, Value.undefined)); }
        let padding = try self.getV(options, self.atom("padding"));
        if not padding.isUndefined and not padding.isObject { return self.throwType("padding must be an object"); }
        .Ok((mode, padding))
    }

    mutating func iteratorZip(iterables: Value, options: Value) -> Result[Value, Value] {
        if not iterables.isObject { return self.throwType("Iterator.zip requires an iterable object"); }
        let (mode, padOpt) = try self.zipOptions(options);
        self.pushRoot(padOpt);
        let (input, inputNext) = try self.getIterator(iterables);
        self.pushRoot(input);
        var iters: [Value] = [];
        var nexts: [Value] = [];
        while true {
            let v = match self.iteratorStep(input, inputNext) {
                .Ok(.Some(x)) => x,
                .Ok(.None) => break,
                .Err(e) => return self.failClosingAll(iters, e)
            };
            let (it, next) = match self.iteratorFlattenable(v, false) {
                .Ok(p) => p,
                .Err(e) => {
                    var all = [input];
                    for x in iters { all.append(x); }
                    return self.failClosingAll(all, e);
                }
            };
            self.pushRoot(it);
            self.pushRoot(next);
            iters.append(it);
            nexts.append(next);
        }
        var padding: [Value] = [];
        if mode == ZipMode.longest {
            if padOpt.isUndefined {
                for _ in 0..<iters.count { padding.append(Value.undefined); }
            } else {
                let (pit, pnext) = match self.getIterator(padOpt) {
                    .Ok(p) => p,
                    .Err(e) => return self.failClosingAll(iters, e)
                };
                self.pushRoot(pit);
                var using = true;
                for _ in 0..<iters.count {
                    if using {
                        match self.iteratorStep(pit, pnext) {
                            .Ok(.Some(x)) => {
                                self.pushRoot(x);
                                padding.append(x);
                            },
                            .Ok(.None) => { using = false; },
                            .Err(e) => return self.failClosingAll(iters, e)
                        }
                    }
                    if not using { padding.append(Value.undefined); }
                }
                if using {
                    if let .Err(e) = self.iteratorClose(pit) { return self.failClosingAll(iters, e); }
                }
            }
        }
        .Ok(self.newZipHelper(iters, nexts, padding, mode, Value.undefined))
    }

    mutating func iteratorZipKeyed(iterables: Value, options: Value) -> Result[Value, Value] {
        if not iterables.isObject { return self.throwType("Iterator.zipKeyed requires an object"); }
        let (mode, padOpt) = try self.zipOptions(options);
        self.pushRoot(padOpt);
        let allKeys = try self.ownKeys(iterables.asRef);
        var keys: [Value] = [];
        var iters: [Value] = [];
        var nexts: [Value] = [];
        for key in allKeys {
            let desc = match self.getOwnPropertyV(iterables, key) {
                .Ok(d) => d,
                .Err(e) => return self.failClosingAll(iters, e)
            };
            let enumerable = match desc {
                .Some(d) => d.enumerable ?? false,
                .None => false
            };
            if not enumerable { continue; }
            let v = match self.getV(iterables, key) {
                .Ok(x) => x,
                .Err(e) => return self.failClosingAll(iters, e)
            };
            if v.isUndefined { continue; }
            let (it, next) = match self.iteratorFlattenable(v, false) {
                .Ok(p) => p,
                .Err(e) => return self.failClosingAll(iters, e)
            };
            self.pushRoot(it);
            self.pushRoot(next);
            keys.append(self.keyValue(key));
            iters.append(it);
            nexts.append(next);
        }
        var padding: [Value] = [];
        if mode == ZipMode.longest {
            for k in keys {
                if padOpt.isUndefined {
                    padding.append(Value.undefined);
                    continue;
                }
                let key = try self.toKey(k);
                let x = match self.getV(padOpt, key) {
                    .Ok(x) => x,
                    .Err(e) => return self.failClosingAll(iters, e)
                };
                self.pushRoot(x);
                padding.append(x);
            }
        }
        let keyList = self.newArray(keys);
        .Ok(self.newZipHelper(iters, nexts, padding, mode, keyList))
    }

    mutating func newZipHelper(iters: Array[Value], nexts: Array[Value], padding: Array[Value], mode: Int64, keys: Value) -> Value {
        var open: [Value] = [];
        for _ in 0..<iters.count { open.append(Value.bool(true)); }
        var h = blankHelper(HelperKind.zip);
        h.fn = self.newArray(iters);
        h.inner = self.newArray(nexts);
        h.innerNext = self.newArray(padding);
        let openList = self.newArray(open);
        h.extra = self.newArray([openList, keys]);
        h.mode = mode;
        self.newObjectKind(self.intrinsics.iteratorHelperProto, .IteratorHelper(h))
    }

    func zipOpenFlags(h: IterHelper) -> Value { self.heap.objects(h.extra.asRef).elements(0) }

    /// The zip's iterators that are still open, in order.
    func zipOpenIterators(h: IterHelper) -> Array[Value] {
        let flags = self.arrayElements(self.zipOpenFlags(h));
        let iters = self.arrayElements(h.fn);
        var out: [Value] = [];
        for i in 0..<iters.count {
            if flags(i).asBool { out.append(iters(i)); }
        }
        out
    }

    mutating func zipClosed(h: IterHelper, i: Int64) {
        let flags = self.zipOpenFlags(h);
        self.heap.objects(flags.asRef).elements(i) = Value.bool(false);
    }

    /// IteratorStep: whether `it` is done, without reading the value.
    mutating func iteratorStepDone(it: Value, next: Value) -> Result[Bool, Value] {
        let r = try self.callValue(next, it, []);
        if not r.isObject { return self.throwType("Iterator result \(self.describe(r)) is not an object"); }
        let done = try self.getFrom(r, self.names.done, r);
        .Ok(self.toBoolean(done))
    }

    mutating func zipStep(this: Value, h: IterHelper) -> Result[Optional[Value], Value] {
        let iters = self.arrayElements(h.fn);
        let nexts = self.arrayElements(h.inner);
        let padding = self.arrayElements(h.innerNext);
        let n = iters.count;
        if n == 0 { return .Ok(.None); }
        var results: [Value] = [];
        for i in 0..<n {
            if iters(i).isNull {
                results.append(padding(i));
                continue;
            }
            let step = match self.iteratorStep(iters(i), nexts(i)) {
                .Ok(s) => s,
                .Err(e) => {
                    self.zipClosed(h, i);
                    return self.failClosingAll(self.zipOpenIterators(h), e);
                }
            };
            if let .Some(v) = step {
                self.pushRoot(v);
                results.append(v);
                continue;
            }
            self.zipClosed(h, i);
            if h.mode == ZipMode.shortest {
                try self.closeAllIterators(self.zipOpenIterators(h), .None);
                return .Ok(.None);
            }
            if h.mode == ZipMode.strict { return self.zipStrictFinish(h, i); }
            if self.zipOpenIterators(h).isEmpty { return .Ok(.None); }
            self.heap.objects(h.fn.asRef).elements(i) = Value.nullValue;
            results.append(padding(i));
        }
        .Ok(.Some(try self.zipResult(h, results)))
    }

    /// "strict": the first iterator ended, so every other one must too.
    mutating func zipStrictFinish(h: IterHelper, i: Int64) -> Result[Optional[Value], Value] {
        if i != 0 {
            return self.failClosingAll(self.zipOpenIterators(h), self.typeError("Iterator.zip: iterators have different lengths"));
        }
        let iters = self.arrayElements(h.fn);
        let nexts = self.arrayElements(h.inner);
        for k in 1..<iters.count {
            let done = match self.iteratorStepDone(iters(k), nexts(k)) {
                .Ok(d) => d,
                .Err(e) => {
                    self.zipClosed(h, k);
                    return self.failClosingAll(self.zipOpenIterators(h), e);
                }
            };
            if not done {
                return self.failClosingAll(self.zipOpenIterators(h), self.typeError("Iterator.zip: iterators have different lengths"));
            }
            self.zipClosed(h, k);
        }
        .Ok(.None)
    }

    /// One zip result: an array, or for zipKeyed a null-prototype object.
    mutating func zipResult(h: IterHelper, results: Array[Value]) -> Result[Value, Value] {
        let keys = self.heap.objects(h.extra.asRef).elements(1);
        if keys.isUndefined { return .Ok(self.newArray(results)); }
        let names = self.arrayElements(keys);
        let o = self.newObjectKind(Value.nullValue, .Ordinary);
        for i in 0..<names.count {
            let key = try self.toKey(names(i));
            self.defineOwn(o.asRef, key, results(i), Attr.all);
        }
        .Ok(o)
    }
}

func blankHelper(kind: Int64) -> IterHelper {
    IterHelper(kind: kind, iter: Value.undefined, next: Value.undefined, fn: Value.undefined, counter: 0, remaining: 0.0,
               inner: Value.undefined, innerNext: Value.undefined, state: HelperState.suspendedStart,
               extra: Value.undefined, mode: 0)
}
