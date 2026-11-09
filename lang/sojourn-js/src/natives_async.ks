module js

// Async iteration (ECMA-262 §27.1.4-27.1.6, §27.4, §27.6): async
// generators, %AsyncIteratorPrototype%, and the wrapper that lets
// `for await` and async `yield*` consume sync iterators.
//
// An async generator is a generator (`GenData.isAsyncGenerator`) whose
// raw yields are awaits, as in async functions, and whose plain yields
// settle the oldest request in `GenData.queue`. `asyncGenResumeNext`
// feeds requests to the body one at a time; while the body runs or
// awaits, its state is `executing` so new requests just queue.

extend VM {
    mutating func setupAsyncIteration() {
        let asyncIterProto = self.newObject();
        self.symbolMethod(asyncIterProto, WellKnown.asyncIterator, "[Symbol.asyncIterator]", .ReturnThis, 0);
        self.intrinsics.asyncIteratorProto = asyncIterProto;

        let fromSync = self.newObjectKind(asyncIterProto, .Ordinary);
        self.method(fromSync, "next", .AsyncFromSyncNext, 1);
        self.method(fromSync, "return", .AsyncFromSyncReturn, 1);
        self.method(fromSync, "throw", .AsyncFromSyncThrow, 1);
        self.intrinsics.asyncFromSyncIteratorProto = fromSync;

        let fnProto = self.newObjectKind(self.intrinsics.functionProto, .Ordinary);
        let genProto = self.newObjectKind(asyncIterProto, .Ordinary);
        self.intrinsics.asyncGeneratorFunctionProto = fnProto;
        self.intrinsics.asyncGeneratorProto = genProto;
        self.defineOwn(fnProto.asRef, self.names.prototype, genProto, Attr.configurable);
        self.defineOwn(genProto.asRef, self.names.constructor, fnProto, Attr.configurable);
        let fnTag = self.str("AsyncGeneratorFunction");
        self.defineOwn(fnProto.asRef, symbolKey(WellKnown.toStringTag), fnTag, Attr.configurable);
        let genTag = self.str("AsyncGenerator");
        self.defineOwn(genProto.asRef, symbolKey(WellKnown.toStringTag), genTag, Attr.configurable);
        self.method(genProto, "next", .AsyncGeneratorNext, 1);
        self.method(genProto, "return", .AsyncGeneratorReturn, 1);
        self.method(genProto, "throw", .AsyncGeneratorThrow, 1);
        self.hiddenConstructor("AsyncGeneratorFunction", .AsyncGeneratorFunctionCtor, fnProto);
    }

    /// GetIterator(obj, async): its @@asyncIterator, or its sync iterator
    /// wrapped by CreateAsyncFromSyncIterator; and the `next` method.
    mutating func getAsyncIterator(obj: Value) -> Result[(Value, Value), Value] {
        let method = try self.getV(obj, symbolKey(WellKnown.asyncIterator));
        if method.isNullish {
            let (it, next) = try self.getIterator(obj);
            let w = self.newObjectKind(self.intrinsics.asyncFromSyncIteratorProto, .AsyncFromSyncIterator(iter: it, next: next));
            let wnext = self.ownValue(self.intrinsics.asyncFromSyncIteratorProto.asRef, self.names.next);
            return .Ok((w, wnext));
        }
        if not self.isCallable(method) { return self.throwType("\(self.describe(obj)) is not async iterable"); }
        let it = try self.callValue(method, obj, []);
        if not it.isObject { return self.throwType("Result of the Symbol.asyncIterator method is not an object"); }
        let next = try self.getV(it, self.names.next);
        .Ok((it, next))
    }

    // ------------------------------------------------------------------
    // Async generators
    // ------------------------------------------------------------------

    /// The generator id of async generator `v`, or -1.
    func asyncGeneratorId(v: Value) -> Int64 {
        if not v.isObject { return -1; }
        if let .Generator(id) = self.kindAt(v.asRef) {
            if self.heap.generators(id).isAsyncGenerator { return id; }
        }
        return -1;
    }

    /// AsyncGeneratorEnqueue: next (`mode` 0), throw (1) or return (2).
    mutating func asyncGeneratorRequest(gen: Value, mode: Int64, value: Value) -> Value {
        let p = self.newPromiseObject(self.intrinsics.promiseProto);
        let (res, rej) = self.resolvingFunctions(p);
        let id = self.asyncGeneratorId(gen);
        if id < 0 {
            let e = self.typeError("AsyncGenerator method called on incompatible receiver");
            self.rejectPromise(p, e);
            return p;
        }
        for v in [Value.int(mode), value, p, res, rej] { self.heap.generators(id).queue.append(v); }
        self.asyncGenResumeNext(gen, id);
        p
    }

    /// Feeds queued requests to the generator until one has to wait: the
    /// body runs, or a return() awaits its value.
    mutating func asyncGenResumeNext(gen: Value, id: Int64) {
        while true {
            var state = self.heap.generators(id).state;
            if state == GenState.executing or state == GenState.awaitingReturn { return; }
            let q = self.heap.generators(id).queue;
            if q.isEmpty { return; }
            let mode = q(0).asNumber.toInt64() ?? 0;
            let value = q(1);
            if mode != 0 and state == GenState.suspendedStart {
                self.finishGenerator(id);
                state = GenState.completed;
            }
            if state == GenState.completed {
                if mode == 2 {
                    self.heap.generators(id).state = GenState.awaitingReturn;
                    match self.promiseResolve(self.intrinsics.promiseCtor, value) {
                        .Ok(pr) => {
                            let mark = self.roots.count;
                            self.pushRoot(pr);
                            let onF = self.nativeClosure(.AsyncGeneratorReturnSettled, 1, [gen, Value.bool(false)]);
                            self.pushRoot(onF);
                            let onR = self.nativeClosure(.AsyncGeneratorReturnSettled, 1, [gen, Value.bool(true)]);
                            let u = Value.undefined;
                            self.performThen(pr, onF, onR, u, u, u);
                            self.truncateRoots(mark);
                            return;
                        },
                        .Err(e) => {
                            self.heap.generators(id).state = GenState.completed;
                            self.asyncGenCompleteStep(id, true, e, true);
                            continue;
                        }
                    }
                }
                // throw() rejects; next() reports done.
                if mode == 1 { self.asyncGenCompleteStep(id, true, value, true); } else { self.asyncGenCompleteStep(id, false, Value.undefined, true); }
                continue;
            }
            self.asyncGenStep(gen, id, value, mode);
            return;
        }
    }

    /// Runs the body with a request's value (or an awaited one) until it
    /// awaits, yields or finishes.
    mutating func asyncGenStep(gen: Value, id: Int64, value: Value, mode: Int64) {
        let result = self.enterGenerator(id, value, mode);
        if self.heap.generators(id).state == GenState.suspendedYield {
            let v = match result { .Ok(x) => x, .Err(_) => Value.undefined };
            if self.heap.generators(id).raw {
                // An await: still running as far as requests are concerned.
                self.heap.generators(id).state = GenState.executing;
                if let .Err(e) = self.awaitValue(gen, v) { self.asyncGenStep(gen, id, e, 1); }
                return;
            }
            self.asyncGenCompleteStep(id, false, v, false);
            self.asyncGenResumeNext(gen, id);
            return;
        }
        self.finishGenerator(id);
        match result {
            .Ok(v) => self.asyncGenCompleteStep(id, false, v, true),
            .Err(e) => self.asyncGenCompleteStep(id, true, e, true)
        }
        self.asyncGenResumeNext(gen, id);
    }

    /// Settles the oldest request: rejects it, or fulfills it with an
    /// iterator result.
    mutating func asyncGenCompleteStep(id: Int64, isThrow: Bool, value: Value, done: Bool) {
        let q = self.heap.generators(id).queue;
        if q.count < 5 { return; }
        let res = q(3);
        let rej = q(4);
        var rest: [Value] = [];
        for i in 5..<q.count { rest.append(q(i)); }
        self.heap.generators(id).queue = rest;
        let mark = self.roots.count;
        self.pushRoot(res);
        self.pushRoot(rej);
        self.pushRoot(value);
        if isThrow {
            let _ = self.callValue(rej, Value.undefined, [value]);
        } else {
            let r = self.iterResult(value, done);
            let _ = self.callValue(res, Value.undefined, [r]);
        }
        self.truncateRoots(mark);
    }

    // ------------------------------------------------------------------
    // %AsyncFromSyncIteratorPrototype%
    // ------------------------------------------------------------------

    /// next/return/throw on a wrapped sync iterator: calls the sync
    /// method and resolves the promise with its result, value awaited.
    mutating func asyncFromSync(n: NativeFn, this: Value, args: Array[Value]) -> Value {
        let p = self.newPromiseObject(self.intrinsics.promiseProto);
        let (res, rej) = self.resolvingFunctions(p);
        let mark = self.roots.count;
        self.pushRoot(p);
        self.pushRoot(res);
        self.pushRoot(rej);
        let (it, next) = match self.kindAt(this.asRef) {
            .AsyncFromSyncIterator(i, nx) => (i, nx),
            _ => (Value.undefined, Value.undefined)
        };
        if let .Err(e) = self.asyncFromSyncStep(n, it, next, args, p, res, rej) {
            let _ = self.callValue(rej, Value.undefined, [e]);
        }
        self.truncateRoots(mark);
        p
    }

    mutating func asyncFromSyncStep(n: NativeFn, it: Value, next: Value, args: Array[Value], p: Value, res: Value, rej: Value) -> Result[(), Value] {
        let a0 = arg(args, 0);
        let pass: [Value] = if args.isEmpty { [] } else { [a0] };
        var result = Value.undefined;
        if n == NativeFn.AsyncFromSyncNext {
            result = try self.callValue(next, it, pass);
        } else if n == NativeFn.AsyncFromSyncReturn {
            let ret = try self.getMethod(it, self.names.returnKey);
            if ret.isUndefined {
                let r = self.iterResult(a0, true);
                let _ = try self.callValue(res, Value.undefined, [r]);
                return .Ok(());
            }
            result = try self.callValue(ret, it, pass);
        } else {
            let thr = try self.getMethod(it, self.atom("throw"));
            if thr.isUndefined {
                // The sync iterator can't take the error: close it and fail.
                try self.iteratorClose(it);
                return self.throwType("The iterator does not provide a 'throw' method");
            }
            result = try self.callValue(thr, it, pass);
        }
        if not result.isObject { return self.throwType("Iterator result \(self.describe(result)) is not an object"); }
        let done = self.toBoolean(try self.getV(result, self.names.done));
        let value = try self.getV(result, self.names.value);
        // A rejected value closes the sync iterator, unless it's done or
        // being returned from anyway.
        let closeOnRejection = not done and n != NativeFn.AsyncFromSyncReturn;
        let wrapped = match self.promiseResolve(self.intrinsics.promiseCtor, value) {
            .Ok(w) => w,
            .Err(e) => {
                if closeOnRejection { let _ = self.iteratorClose(it); }
                return .Err(e);
            }
        };
        self.pushRoot(wrapped);
        let onF = self.nativeClosure(.AsyncFromSyncUnwrap, 1, [Value.bool(done)]);
        self.pushRoot(onF);
        let onR = if closeOnRejection { self.nativeClosure(.AsyncFromSyncCloseRejected, 1, [it]) } else { Value.undefined };
        self.performThen(wrapped, onF, onR, p, res, rej);
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Natives
    // ------------------------------------------------------------------

    mutating func asyncIterationNative(n: NativeFn, callee: Value, this: Value, args: Array[Value]) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let r: Value = match n {
            .AsyncGeneratorNext => self.asyncGeneratorRequest(this, 0, a0),
            .AsyncGeneratorThrow => self.asyncGeneratorRequest(this, 1, a0),
            .AsyncGeneratorReturn => self.asyncGeneratorRequest(this, 2, a0),
            .AsyncGeneratorReturnSettled => {
                let cs = self.captures(callee);
                let gen = cs(0);
                let id = self.asyncGeneratorId(gen);
                if id >= 0 {
                    self.heap.generators(id).state = GenState.completed;
                    self.asyncGenCompleteStep(id, cs(1).asBool, a0, true);
                    self.asyncGenResumeNext(gen, id);
                }
                Value.undefined
            },
            .AsyncFromSyncNext or .AsyncFromSyncReturn or .AsyncFromSyncThrow => {
                if not this.isObject { return self.throwType("not an async-from-sync iterator"); }
                self.asyncFromSync(n, this, args)
            },
            .AsyncFromSyncUnwrap => self.iterResult(a0, self.captures(callee)(0).asBool),
            .AsyncFromSyncCloseRejected => {
                let _ = self.iteratorClose(self.captures(callee)(0));
                return .Err(a0);
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }
}
