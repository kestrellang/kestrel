module js

// Promise (ECMA-262 §27.2), the job queue, and async functions.
//
// A promise's state and reactions live in its `ObjectKind.PromiseObject`.
// Settling a promise queues one job per reaction in `VM.jobs`; the host
// drains the queue with `runJobs` once the script finishes. Async functions
// run as generators whose `await`s are raw yields: `asyncStep` resumes the
// body and, when it yields, subscribes to the awaited promise with
// `AsyncResume` closures that resume it again.
//
// Small shared records (a resolving pair's "already resolved" flag, a
// combinator's remaining count) are one-element internal arrays.

/// `ObjectKind.PromiseObject` states.
struct PromiseState {
    static var pending: Int64 { 0 }
    static var fulfilled: Int64 { 1 }
    static var rejected: Int64 { 2 }
}

/// What a `Job` does.
struct JobKind {
    /// Runs a fulfill reaction: a, b, c are the derived promise and its
    /// resolve and reject (undefined for an await), d the handler, e the
    /// value.
    static var fulfill: Int64 { 0 }
    static var reject: Int64 { 1 }
    /// Calls `c` (a thenable's `then`) on `b`, resolving promise `a`.
    static var thenable: Int64 { 2 }
}

/// A queued promise job (`JobKind`).
struct Job {
    var kind: Int64
    var a: Value
    var b: Value
    var c: Value
    var d: Value
    var e: Value
}

/// Promise combinators, for `promiseCombinator`.
struct Combinator {
    static var all: Int64 { 0 }
    static var allSettled: Int64 { 1 }
    static var any: Int64 { 2 }
    static var race: Int64 { 3 }
    /// allKeyed / allSettledKeyed: `all` / `allSettled` over an object's
    /// enumerable own properties, settling with a null-prototype object.
    static var allKeyed: Int64 { 4 }
    static var allSettledKeyed: Int64 { 5 }
}

extend VM {
    mutating func setupPromise() {
        let proto = self.newObject();
        let c = self.constructorFor("Promise", .PromiseCtor, 1, proto);
        self.intrinsics.promiseCtor = c;
        self.intrinsics.promiseProto = proto;
        let tag = self.str("Promise");
        self.defineOwn(proto.asRef, symbolKey(WellKnown.toStringTag), tag, Attr.configurable);
        let species = self.nativeFunction(.ReturnThis, "get [Symbol.species]", 0, false);
        self.defineAccessorOwn(c.asRef, symbolKey(WellKnown.species), species, Value.undefined, Attr.configurable);
        self.method(proto, "then", .PromiseThen, 2);
        self.method(proto, "catch", .PromiseCatch, 1);
        self.method(proto, "finally", .PromiseFinally, 1);
        for (name, n, len) in [
            ("resolve", NativeFn.PromiseResolve, 1), ("reject", NativeFn.PromiseReject, 1),
            ("all", NativeFn.PromiseAll, 1), ("allSettled", NativeFn.PromiseAllSettled, 1),
            ("any", NativeFn.PromiseAny, 1), ("race", NativeFn.PromiseRace, 1),
            ("withResolvers", NativeFn.PromiseWithResolvers, 0), ("allKeyed", NativeFn.PromiseAllKeyed, 1),
            ("allSettledKeyed", NativeFn.PromiseAllSettledKeyed, 1)
        ] {
            self.method(c, name, n, len);
        }

        // %AsyncFunction.prototype%: async functions inherit from it.
        let asyncProto = self.newObjectKind(self.intrinsics.functionProto, .Ordinary);
        let asyncTag = self.str("AsyncFunction");
        self.defineOwn(asyncProto.asRef, symbolKey(WellKnown.toStringTag), asyncTag, Attr.configurable);
        self.intrinsics.asyncFunctionProto = asyncProto;
        self.hiddenConstructor("AsyncFunction", .AsyncFunctionCtor, asyncProto);
        self.setupAsyncIteration();

        // AggregateError, for Promise.any.
        let errorCtor = self.ownValue(self.global.asRef, self.atom("Error"));
        let aggregate = self.makeErrorType("AggregateError", .AggregateErrorCtor, errorCtor, self.intrinsics.errorProto);
        self.intrinsics.aggregateErrorProto = aggregate;
        let aggregateCtor = self.ownValue(aggregate.asRef, self.names.constructor);
        self.defineReadOnly(aggregateCtor.asRef, "length", Value.int(2));
    }

    // ------------------------------------------------------------------
    // Promise internals
    // ------------------------------------------------------------------

    /// A built-in function closing over `captures`.
    mutating func nativeClosure(n: NativeFn, length: Int64, captures: Array[Value]) -> Value {
        let f = self.newObjectKind(self.intrinsics.functionProto, .NativeClosure(fn: n, captures: captures));
        self.defineReadOnly(f.asRef, "length", Value.int(length));
        let empty = self.str("");
        self.defineReadOnly(f.asRef, "name", empty);
        f
    }

    func captures(f: Value) -> Array[Value] {
        match self.kindAt(f.asRef) {
            .NativeClosure(_, cs) => cs,
            _ => []
        }
    }

    func isPromise(v: Value) -> Bool {
        if not v.isObject { return false; }
        match self.kindAt(v.asRef) {
            .PromiseObject(_, _, _, _, _) => true,
            _ => false
        }
    }

    mutating func newPromiseObject(proto: Value) -> Value {
        self.newObjectKind(proto, .PromiseObject(state: PromiseState.pending, result: Value.undefined, fulfill: [], reject: [], handled: false))
    }

    /// A one-element internal record, for state closures share.
    mutating func newRecord(v: Value) -> Value {
        let r = self.newObjectKind(Value.nullValue, .Ordinary);
        self.setElement(r.asRef, 0, v);
        r
    }

    func recordGet(r: Value) -> Value { self.heap.objects(r.asRef).elements(0) }

    mutating func recordSet(r: Value, v: Value) { self.heap.objects(r.asRef).elements(0) = v; }

    /// CreateResolvingFunctions: a resolve/reject pair sharing one
    /// "already resolved" flag.
    mutating func resolvingFunctions(p: Value) -> (Value, Value) {
        let record = self.newRecord(Value.bool(false));
        let resolve = self.nativeClosure(.PromiseResolveFunction, 1, [p, record]);
        let reject = self.nativeClosure(.PromiseRejectFunction, 1, [p, record]);
        (resolve, reject)
    }

    /// The body of a resolve function: adopts a thenable's state, or
    /// fulfills with anything else.
    mutating func resolvePromise(p: Value, resolution: Value) {
        if resolution.same(p) {
            let e = self.typeError("Chaining cycle detected for promise");
            self.settlePromise(p, PromiseState.rejected, e);
            return;
        }
        if not resolution.isObject {
            self.settlePromise(p, PromiseState.fulfilled, resolution);
            return;
        }
        let then = match self.getV(resolution, self.atom("then")) {
            .Ok(t) => t,
            .Err(e) => {
                self.settlePromise(p, PromiseState.rejected, e);
                return;
            }
        };
        if not self.isCallable(then) {
            self.settlePromise(p, PromiseState.fulfilled, resolution);
            return;
        }
        self.enqueueJob(Job(kind: JobKind.thenable, a: p, b: resolution, c: then, d: Value.undefined, e: Value.undefined));
    }

    mutating func rejectPromise(p: Value, reason: Value) { self.settlePromise(p, PromiseState.rejected, reason); }

    /// Fulfills or rejects a pending promise, queueing its reactions.
    mutating func settlePromise(p: Value, state: Int64, value: Value) {
        let k = self.kindAt(p.asRef);
        let (old, fulfill, reject, handled) = match k {
            .PromiseObject(s, _, fs, rs, h) => (s, fs, rs, h),
            _ => return
        };
        if old != PromiseState.pending { return; }
        self.heap.objects(p.asRef).kind = .PromiseObject(state: state, result: value, fulfill: [], reject: [], handled: handled);
        let reactions = if state == PromiseState.fulfilled { fulfill } else { reject };
        let kind = if state == PromiseState.fulfilled { JobKind.fulfill } else { JobKind.reject };
        var i = 0;
        while i + 3 < reactions.count {
            self.enqueueJob(Job(kind: kind, a: reactions(i), b: reactions(i + 1), c: reactions(i + 2), d: reactions(i + 3), e: value));
            i = i + 4;
        }
    }

    /// PerformPromiseThen: runs `onFulfilled` or `onRejected` once `p`
    /// settles, settling the capability (`cp`, `cres`, `crej`) with the
    /// result. An undefined capability (for await) drops the result.
    mutating func performThen(p: Value, onFulfilled: Value, onRejected: Value, cp: Value, cres: Value, crej: Value) {
        let onF = if self.isCallable(onFulfilled) { onFulfilled } else { Value.undefined };
        let onR = if self.isCallable(onRejected) { onRejected } else { Value.undefined };
        let k = self.kindAt(p.asRef);
        let (state, result, fulfill, reject) = match k {
            .PromiseObject(s, r, fs, rs, _) => (s, r, fs, rs),
            _ => return
        };
        if state == PromiseState.pending {
            var fs = fulfill;
            var rs = reject;
            for v in [cp, cres, crej, onF] { fs.append(v); }
            for v in [cp, cres, crej, onR] { rs.append(v); }
            self.heap.objects(p.asRef).kind = .PromiseObject(state: state, result: result, fulfill: fs, reject: rs, handled: true);
            return;
        }
        let isFulfilled = state == PromiseState.fulfilled;
        let kind = if isFulfilled { JobKind.fulfill } else { JobKind.reject };
        let handler = if isFulfilled { onF } else { onR };
        self.enqueueJob(Job(kind: kind, a: cp, b: cres, c: crej, d: handler, e: result));
        self.heap.objects(p.asRef).kind = .PromiseObject(state: state, result: result, fulfill: [], reject: [], handled: true);
    }

    /// NewPromiseCapability(C): a promise from constructor `c` and the
    /// resolve and reject functions its executor received.
    mutating func newPromiseCapability(c: Value) -> Result[(Value, Value, Value), Value] {
        if c.same(self.intrinsics.promiseCtor) {
            let p = self.newPromiseObject(self.intrinsics.promiseProto);
            let (res, rej) = self.resolvingFunctions(p);
            return .Ok((p, res, rej));
        }
        if not self.isConstructor(c) { return self.throwType("\(self.describe(c)) is not a constructor"); }
        let record = self.newObjectKind(Value.nullValue, .Ordinary);
        self.setElement(record.asRef, 0, Value.undefined);
        self.setElement(record.asRef, 1, Value.undefined);
        let executor = self.nativeClosure(.PromiseCapabilityExecutor, 2, [record]);
        let mark = self.roots.count;
        self.pushRoot(executor);
        let p = self.construct(c, [executor], c);
        self.truncateRoots(mark);
        let promise = try p;
        let res = self.heap.objects(record.asRef).elements(0);
        let rej = self.heap.objects(record.asRef).elements(1);
        if not self.isCallable(res) { return self.throwType("Promise resolve function is not callable"); }
        if not self.isCallable(rej) { return self.throwType("Promise reject function is not callable"); }
        .Ok((promise, res, rej))
    }

    /// PromiseResolve(C, x): `x` itself if it's a promise from `c`, else a
    /// new promise from `c` resolved with it.
    mutating func promiseResolve(c: Value, x: Value) -> Result[Value, Value] {
        if self.isPromise(x) {
            let ctor = try self.getV(x, self.names.constructor);
            if ctor.same(c) { return .Ok(x); }
        }
        let (p, res, _) = try self.newPromiseCapability(c);
        self.pushRoot(p);
        let _ = try self.callValue(res, Value.undefined, [x]);
        .Ok(p)
    }

    mutating func invoke(v: Value, name: String, args: Array[Value]) -> Result[Value, Value] {
        let f = try self.getV(v, self.atom(name));
        self.callValue(f, v, args)
    }

    // ------------------------------------------------------------------
    // Jobs
    // ------------------------------------------------------------------

    mutating func enqueueJob(j: Job) { self.jobs.append(j); }

    /// Runs queued jobs, and the jobs they queue, until none are left.
    /// Errors a job throws are dropped (HostReportErrors).
    mutating func runJobs() {
        var i = 0;
        while i < self.jobs.count {
            let j = self.jobs(i);
            i = i + 1;
            let _ = self.runJob(j);
            // Drop finished jobs now and then so the queue doesn't grow.
            if i >= 4096 {
                var rest: [Job] = [];
                for k in i..<self.jobs.count { rest.append(self.jobs(k)); }
                self.jobs = rest;
                i = 0;
            }
        }
        self.jobs = [];
    }

    mutating func runJob(j: Job) -> Result[(), Value] {
        if j.kind == JobKind.thenable {
            let (res, rej) = self.resolvingFunctions(j.a);
            let mark = self.roots.count;
            self.pushRoot(res);
            self.pushRoot(rej);
            if let .Err(e) = self.callValue(j.c, j.b, [res, rej]) {
                let _ = self.callValue(rej, Value.undefined, [e]);
            }
            self.truncateRoots(mark);
            return .Ok(());
        }
        let isReject = j.kind == JobKind.reject;
        let result: Result[Value, Value] = if not j.d.isUndefined {
            self.callValue(j.d, Value.undefined, [j.e])
        } else if isReject {
            .Err(j.e)
        } else {
            .Ok(j.e)
        };
        // An await has no capability to settle.
        if j.b.isUndefined { return .Ok(()); }
        match result {
            .Ok(v) => { let _ = try self.callValue(j.b, Value.undefined, [v]); },
            .Err(e) => { let _ = try self.callValue(j.c, Value.undefined, [e]); }
        }
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Async functions
    // ------------------------------------------------------------------

    /// GeneratorStart in an async function: moves the frame into a
    /// generator, returns its promise, and runs the body to its first
    /// await.
    mutating func startAsync(fi: Int64, stop: Int64) -> Result[Step, Value] {
        let p = self.newPromiseObject(self.intrinsics.promiseProto);
        let data = GenData(frame: self.frames(fi), registers: [], state: GenState.suspendedStart, resumeReg: -1, raw: false, asyncPromise: p, isAsyncGenerator: false, queue: []);
        let id = self.heap.allocGenerator(data);
        let gen = self.newObjectKind(self.intrinsics.objectProto, .Generator(id: id));
        self.frames(fi).generator = gen;
        self.suspendFrame(fi, id);
        let mark = self.roots.count;
        self.pushRoot(gen);
        self.pushRoot(p);
        let step = self.doReturn(p, stop);
        self.asyncStep(gen, Value.undefined, 0);
        self.truncateRoots(mark);
        step
    }

    /// Resumes an async function with a value (`mode` 0) or a rejection
    /// (`mode` 1), running it to its next await or its end.
    mutating func asyncStep(gen: Value, value: Value, mode: Int64) {
        let id = match self.kindAt(gen.asRef) {
            .Generator(x) => x,
            _ => return
        };
        if self.heap.generators(id).isAsyncGenerator {
            self.asyncGenStep(gen, id, value, mode);
            return;
        }
        let result = self.enterGenerator(id, value, mode);
        let p = self.heap.generators(id).asyncPromise;
        if self.heap.generators(id).state == GenState.suspendedYield {
            let awaited = match result { .Ok(v) => v, .Err(_) => Value.undefined };
            if let .Err(e) = self.awaitValue(gen, awaited) { self.asyncStep(gen, e, 1); }
            return;
        }
        self.finishGenerator(id);
        match result {
            .Ok(v) => self.resolvePromise(p, v),
            .Err(e) => self.rejectPromise(p, e)
        }
    }

    /// Await: resumes `gen` when `v`, as a promise, settles.
    mutating func awaitValue(gen: Value, v: Value) -> Result[(), Value] {
        let promise = try self.promiseResolve(self.intrinsics.promiseCtor, v);
        let mark = self.roots.count;
        self.pushRoot(promise);
        let onF = self.nativeClosure(.AsyncResume, 1, [gen, Value.int(0)]);
        let onR = self.nativeClosure(.AsyncResume, 1, [gen, Value.int(1)]);
        let u = Value.undefined;
        self.performThen(promise, onF, onR, u, u, u);
        self.truncateRoots(mark);
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Combinators
    // ------------------------------------------------------------------

    /// Promise.all, allSettled, any and race (`Combinator`).
    mutating func promiseCombinator(kind: Int64, c: Value, iterable: Value) -> Result[Value, Value] {
        let (p, res, rej) = try self.newPromiseCapability(c);
        let mark = self.roots.count;
        self.pushRoot(p);
        self.pushRoot(res);
        self.pushRoot(rej);
        let r = if kind >= Combinator.allKeyed {
            self.keyedCombinatorBody(kind - Combinator.allKeyed, c, iterable, res, rej)
        } else {
            self.combinatorBody(kind, c, iterable, res, rej)
        };
        if let .Err(e) = r {
            let rejected = self.callValue(rej, Value.undefined, [e]);
            self.truncateRoots(mark);
            let _ = try rejected;
            return .Ok(p);
        }
        self.truncateRoots(mark);
        .Ok(p)
    }

    mutating func combinatorBody(kind: Int64, c: Value, iterable: Value, res: Value, rej: Value) -> Result[(), Value] {
        let resolveFn = try self.getV(c, self.atom("resolve"));
        if not self.isCallable(resolveFn) { return self.throwType("Promise resolve is not a function"); }
        let (it, next) = try self.getIterator(iterable);
        self.pushRoot(it);
        self.pushRoot(next);
        let values = self.newArray([]);
        self.pushRoot(values);
        let remaining = self.newRecord(Value.int(1));
        self.pushRoot(remaining);
        var index = 0;
        while true {
            let step = try self.iteratorStep(it, next);
            if let .Some(value) = step {
                let r = self.combinatorElement(kind, c, resolveFn, value, index, values, Value.undefined, remaining, res, rej);
                if let .Err(e) = r {
                    let _ = self.iteratorClose(it);
                    return .Err(e);
                }
                index = index + 1;
            } else {
                break;
            }
        }
        if kind == Combinator.race { return .Ok(()); }
        if self.countDown(remaining) { try self.finishCombinator(kind, values, Value.undefined, res, rej); }
        .Ok(())
    }

    /// PerformPromiseAllKeyed: `kind` is all or allSettled.
    mutating func keyedCombinatorBody(kind: Int64, c: Value, promises: Value, res: Value, rej: Value) -> Result[(), Value] {
        let resolveFn = try self.getV(c, self.atom("resolve"));
        if not self.isCallable(resolveFn) { return self.throwType("Promise resolve is not a function"); }
        if not promises.isObject { return self.throwType("Promise keyed combinators need an object"); }
        let allKeys = try self.ownKeys(promises.asRef);
        let keys = self.newArray([]);
        self.pushRoot(keys);
        let values = self.newArray([]);
        self.pushRoot(values);
        let remaining = self.newRecord(Value.int(1));
        self.pushRoot(remaining);
        var index = 0;
        for key in allKeys {
            let desc = try self.getOwnPropertyV(promises, key);
            let enumerable = match desc {
                .Some(d) => d.enumerable ?? false,
                .None => false
            };
            if not enumerable { continue; }
            let value = try self.getV(promises, key);
            let kv = self.keyValue(key);
            self.appendElement(keys.asRef, kv);
            try self.combinatorElement(kind, c, resolveFn, value, index, values, keys, remaining, res, rej);
            index = index + 1;
        }
        if self.countDown(remaining) { try self.finishCombinator(kind, values, keys, res, rej); }
        .Ok(())
    }

    /// Subscribes to one element of a combinator's iterable.
    mutating func combinatorElement(kind: Int64, c: Value, resolveFn: Value, value: Value, index: Int64,
                                    values: Value, keys: Value, remaining: Value, res: Value, rej: Value) -> Result[(), Value] {
        if kind != Combinator.race { self.appendElement(values.asRef, Value.undefined); }
        let next = try self.callValue(resolveFn, c, [value]);
        if kind == Combinator.race {
            let _ = try self.invoke(next, "then", [res, rej]);
            return .Ok(());
        }
        let mark = self.roots.count;
        self.pushRoot(next);
        let called = self.newRecord(Value.bool(false));
        let i = Value.int(index);
        var onF = res;
        var onR = rej;
        if kind == Combinator.all {
            onF = self.nativeClosure(.PromiseAllResolveElement, 1, [called, i, values, res, remaining, Value.bool(false), keys]);
        } else if kind == Combinator.allSettled {
            onF = self.nativeClosure(.PromiseAllSettledElement, 1, [called, i, values, res, remaining, Value.bool(false), keys]);
            self.pushRoot(onF);
            onR = self.nativeClosure(.PromiseAllSettledElement, 1, [called, i, values, res, remaining, Value.bool(true), keys]);
        } else {
            onR = self.nativeClosure(.PromiseAnyRejectElement, 1, [called, i, values, rej, remaining, Value.bool(false), Value.undefined]);
        }
        self.pushRoot(onF);
        self.pushRoot(onR);
        let n = self.recordGet(remaining).asNumber;
        self.recordSet(remaining, Value.number(n + 1.0));
        let r = self.invoke(next, "then", [onF, onR]);
        self.truncateRoots(mark);
        let _ = try r;
        .Ok(())
    }

    /// Takes one off a remaining-elements count; true when it hits zero.
    mutating func countDown(remaining: Value) -> Bool {
        let n = self.recordGet(remaining).asNumber - 1.0;
        self.recordSet(remaining, Value.number(n));
        n == 0.0
    }

    /// Settles a combinator's promise once every element has reported.
    /// With `keys`, the result is a null-prototype object of key: value.
    mutating func finishCombinator(kind: Int64, values: Value, keys: Value, res: Value, rej: Value) -> Result[(), Value] {
        if kind == Combinator.any {
            let e = self.aggregateError(values);
            let _ = try self.callValue(rej, Value.undefined, [e]);
            return .Ok(());
        }
        var result = values;
        if not keys.isUndefined {
            result = self.newObjectKind(Value.nullValue, .Ordinary);
            let names = self.arrayElements(keys);
            let items = self.arrayElements(values);
            for i in 0..<names.count {
                let k = try self.toKey(names(i));
                self.defineOwn(result.asRef, k, items(i), Attr.all);
            }
        }
        let _ = try self.callValue(res, Value.undefined, [result]);
        .Ok(())
    }

    mutating func aggregateError(errors: Value) -> Value {
        let e = self.newObjectKind(self.intrinsics.aggregateErrorProto, .ErrorObject);
        let list = self.newArray(self.arrayElements(errors));
        self.defineHidden(e.asRef, self.atom("errors"), list);
        let msg = self.str("All promises were rejected");
        self.defineHidden(e.asRef, self.names.message, msg);
        e
    }

    /// An element function of all, allSettled or any. Captures: the
    /// "already called" record, the index, the values array, the function
    /// to finish with, the remaining count, and for allSettled whether it
    /// reports a rejection.
    mutating func combinatorElementCall(n: NativeFn, callee: Value, x: Value) -> Result[Value, Value] {
        let cs = self.captures(callee);
        if self.recordGet(cs(0)).asBool { return .Ok(Value.undefined); }
        self.recordSet(cs(0), Value.bool(true));
        let index = cs(1).asNumber.toInt64() ?? 0;
        var item = x;
        if n == NativeFn.PromiseAllSettledElement {
            let isReject = cs(5).asBool;
            let o = self.newObject();
            let status = self.str(if isReject { "rejected" } else { "fulfilled" });
            self.defineOwn(o.asRef, self.atom("status"), status, Attr.all);
            self.defineOwn(o.asRef, self.atom(if isReject { "reason" } else { "value" }), x, Attr.all);
            item = o;
        }
        self.setElement(cs(2).asRef, index, item);
        if not self.countDown(cs(4)) { return .Ok(Value.undefined); }
        let kind = if n == NativeFn.PromiseAnyRejectElement { Combinator.any } else { Combinator.all };
        try self.finishCombinator(kind, cs(2), cs(6), cs(3), cs(3));
        .Ok(Value.undefined)
    }

    // ------------------------------------------------------------------
    // Natives
    // ------------------------------------------------------------------

    mutating func promiseNative(n: NativeFn, callee: Value, this: Value, args: Array[Value], newTarget: Value) -> Result[Optional[Value], Value] {
        let a0 = arg(args, 0);
        let a1 = arg(args, 1);
        let r: Value = match n {
            .PromiseCtor => {
                if newTarget.isUndefined { return self.throwType("Promise constructor cannot be invoked without 'new'"); }
                if not self.isCallable(a0) { return self.throwType("Promise resolver \(self.describe(a0)) is not a function"); }
                let proto = try self.prototypeFor(newTarget, self.intrinsics.promiseProto);
                let p = self.newPromiseObject(proto);
                self.pushRoot(p);
                let (res, rej) = self.resolvingFunctions(p);
                self.pushRoot(res);
                self.pushRoot(rej);
                if let .Err(e) = self.callValue(a0, Value.undefined, [res, rej]) {
                    let _ = try self.callValue(rej, Value.undefined, [e]);
                }
                p
            },
            .PromiseResolveFunction or .PromiseRejectFunction => {
                let cs = self.captures(callee);
                if self.recordGet(cs(1)).asBool { return .Ok(.Some(Value.undefined)); }
                self.recordSet(cs(1), Value.bool(true));
                if n == NativeFn.PromiseResolveFunction { self.resolvePromise(cs(0), a0); } else { self.rejectPromise(cs(0), a0); }
                Value.undefined
            },
            .PromiseCapabilityExecutor => {
                let record = self.captures(callee)(0);
                let els = self.heap.objects(record.asRef).elements;
                if not els(0).isUndefined { return self.throwType("Promise executor has already been invoked"); }
                if not els(1).isUndefined { return self.throwType("Promise executor has already been invoked"); }
                self.heap.objects(record.asRef).elements(0) = a0;
                self.heap.objects(record.asRef).elements(1) = a1;
                Value.undefined
            },
            .PromiseThen => {
                if not self.isPromise(this) { return self.throwType("Promise.prototype.then called on a non-promise"); }
                let c = try self.speciesConstructor(this, self.intrinsics.promiseCtor);
                let (p, res, rej) = try self.newPromiseCapability(c);
                self.performThen(this, a0, a1, p, res, rej);
                p
            },
            .PromiseCatch => try self.invoke(this, "then", [Value.undefined, a0]),
            .PromiseFinally => {
                if not this.isObject { return self.throwType("Promise.prototype.finally called on a non-object"); }
                let c = try self.speciesConstructor(this, self.intrinsics.promiseCtor);
                var onF = a0;
                var onR = a0;
                if self.isCallable(a0) {
                    onF = self.nativeClosure(.PromiseThenFinally, 1, [a0, c]);
                    self.pushRoot(onF);
                    onR = self.nativeClosure(.PromiseCatchFinally, 1, [a0, c]);
                    self.pushRoot(onR);
                }
                try self.invoke(this, "then", [onF, onR])
            },
            .PromiseThenFinally or .PromiseCatchFinally => {
                let cs = self.captures(callee);
                let result = try self.callValue(cs(0), Value.undefined, []);
                let p = try self.promiseResolve(cs(1), result);
                self.pushRoot(p);
                let thunk = if n == NativeFn.PromiseThenFinally {
                    self.nativeClosure(.PromiseValueThunk, 0, [a0])
                } else {
                    self.nativeClosure(.PromiseThrower, 0, [a0])
                };
                self.pushRoot(thunk);
                try self.invoke(p, "then", [thunk])
            },
            .PromiseValueThunk => self.captures(callee)(0),
            .PromiseThrower => return .Err(self.captures(callee)(0)),
            .PromiseResolve => {
                if not this.isObject { return self.throwType("Promise.resolve called on a non-object"); }
                try self.promiseResolve(this, a0)
            },
            .PromiseReject => {
                let (p, _, rej) = try self.newPromiseCapability(this);
                self.pushRoot(p);
                let _ = try self.callValue(rej, Value.undefined, [a0]);
                p
            },
            .PromiseWithResolvers => {
                let (p, res, rej) = try self.newPromiseCapability(this);
                let o = self.newObject();
                self.defineOwn(o.asRef, self.atom("promise"), p, Attr.all);
                self.defineOwn(o.asRef, self.atom("resolve"), res, Attr.all);
                self.defineOwn(o.asRef, self.atom("reject"), rej, Attr.all);
                o
            },
            .PromiseAll => try self.promiseCombinator(Combinator.all, this, a0),
            .PromiseAllSettled => try self.promiseCombinator(Combinator.allSettled, this, a0),
            .PromiseAny => try self.promiseCombinator(Combinator.any, this, a0),
            .PromiseAllKeyed => try self.promiseCombinator(Combinator.allKeyed, this, a0),
            .PromiseAllSettledKeyed => try self.promiseCombinator(Combinator.allSettledKeyed, this, a0),
            .PromiseRace => try self.promiseCombinator(Combinator.race, this, a0),
            .PromiseAllResolveElement or .PromiseAllSettledElement or .PromiseAnyRejectElement =>
                try self.combinatorElementCall(n, callee, a0),
            .AsyncResume => {
                let cs = self.captures(callee);
                let mode = cs(1).asNumber.toInt64() ?? 0;
                self.asyncStep(cs(0), a0, mode);
                Value.undefined
            },
            .AggregateErrorCtor => {
                let nt = if newTarget.isUndefined { callee } else { newTarget };
                let proto = try self.prototypeFor(nt, self.intrinsics.aggregateErrorProto);
                let e = self.newObjectKind(proto, .ErrorObject);
                self.pushRoot(e);
                if not a1.isUndefined {
                    let m = try self.toStringValue(a1);
                    self.defineHidden(e.asRef, self.names.message, m);
                }
                let a2 = arg(args, 2);
                let hp11 = try self.hasProperty(a2, self.names.cause);
                if a2.isObject and hp11 {
                    let cause = try self.getFrom(a2, self.names.cause, a2);
                    self.defineHidden(e.asRef, self.names.cause, cause);
                }
                let list = try self.iterateToArray(a0);
                let errors = self.newArray(list);
                self.defineHidden(e.asRef, self.atom("errors"), errors);
                e
            },
            _ => return .Ok(.None)
        };
        .Ok(.Some(r))
    }
}
