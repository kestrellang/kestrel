module js

// The interpreter loop, calls and construction.
//
// Calls between JS functions don't recurse in Kestrel: `Call` pushes a
// frame and the loop carries on in it. Native code calling back into JS
// (`callValue`) runs a nested loop that returns when its frame does.

/// What a step did.
enum Step {
    case Next
    /// The frame that a nested loop waits on returned this value.
    case Done(Value)
}

extend VM {
    func reg(frame: Int64, r: Int64) -> Value { self.stack(self.frames(frame).base + r) }

    mutating func setReg(frame: Int64, r: Int64, v: Value) {
        let base = self.frames(frame).base;
        self.stack(base + r) = v;
    }

    /// The environment `hops` steps up from `env`.
    func envAt(env: Int64, hops: Int64) -> Int64 {
        var e = env;
        for _ in 0..<hops { e = self.heap.envs(e).parent; }
        e
    }

    // ------------------------------------------------------------------
    // Running
    // ------------------------------------------------------------------

    /// Compiles and runs a script.
    mutating func runScript(source: String, name: String) -> Result[Value, Value] {
        let program = match parseScript(source) {
            .Ok(p) => p,
            .Err(e) => return .Err(self.makeError(.SyntaxError, e.message))
        };
        self.sourceName = name;
        let code = match self.compileScript(program) {
            .Ok(c) => c,
            .Err(e) => {
                self.fns = [];
                return .Err(self.makeError(.SyntaxError, e.message));
            }
        };
        let fnObj = self.makeClosureObject(code, self.globalEnv, Value.undefined);
        self.callValue(fnObj, self.global, [])
    }

    /// `new Function(p1, ..., body)`: parses the pieces as one function
    /// expression in the global scope.
    mutating func functionConstructor(args: Array[Value]) -> Result[Value, Value] {
        self.dynamicFunction(args, "function")
    }

    /// CreateDynamicFunction for `kind`: "function", "function*" or
    /// "async function".
    mutating func dynamicFunction(args: Array[Value], kind: String) -> Result[Value, Value] {
        var params = "";
        for i in 0..<(args.count - 1) {
            if i > 0 { params.append(","); }
            params.append(try self.toText(args(i)));
        }
        let body = if args.count > 0 { try self.toText(args(args.count - 1)) } else { "" };
        let source = "(" + kind + " (" + params + "\n) {\n" + body + "\n})";
        // Anything that closes the function early leaves more than the
        // one expression statement.
        let program = match parseScript(source) {
            .Ok(p) => p,
            .Err(e) => return .Err(self.makeError(.SyntaxError, e.message))
        };
        if program.body.count != 1 { return .Err(self.makeError(.SyntaxError, "Invalid function body")); }
        let f = try self.runScript(source, "Function");
        self.defineReadOnly(f.asRef, "name", self.str("anonymous"));
        .Ok(f)
    }

    /// Runs frames until the one at depth `stop` returns.
    mutating func run(stop: Int64) -> Result[Value, Value] {
        while true {
            let fi = self.frames.count - 1;
            let pc = self.frames(fi).pc;
            self.frames(fi).pc = pc + 1;
            let instr = self.codes(self.frames(fi).code).instrs(pc);
            match self.step(instr, fi, stop) {
                .Ok(s) => if let .Done(v) = s { return .Ok(v); },
                .Err(ex) => {
                    if let .Some(e) = self.unwind(ex, stop) { return .Err(e); }
                }
            }
        }
        .Ok(Value.undefined)
    }

    /// Finds a handler for exception `ex`, popping frames that have none.
    /// Returns the exception if it escapes the frame at depth `stop`.
    mutating func unwind(ex: Value, stop: Int64) -> Optional[Value] {
        // `stop` counts the entry frame, so it is searched (and popped) too.
        while self.frames.count >= stop {
            let fi = self.frames.count - 1;
            let handler = self.frames(fi).handlers.pop();
            if let .Some(h) = handler {
                self.frames(fi).env = h.env;
                self.frames(fi).pc = h.pc;
                self.setReg(fi, h.exception, ex);
                return .None;
            }
            self.popFrame();
        }
        .Some(ex)
    }

    mutating func popFrame() {
        if let .Some(f) = self.frames.pop() {
            while self.stack.count > f.base { let _ = self.stack.pop(); }
            self.depth = self.depth - 1;
        }
    }

    // ------------------------------------------------------------------
    // Calls
    // ------------------------------------------------------------------

    /// Calls `f` from native code, running it to completion.
    mutating func callValue(f: Value, this: Value, args: Array[Value]) -> Result[Value, Value] {
        if not self.isCallable(f) { return self.throwType("\(self.describe(f)) is not a function"); }
        match self.kindAt(f.asRef) {
            .Closure(_, _, _, _) => {
                let stop = self.frames.count;
                try self.pushFrame(f, this, args, Value.undefined, false, -1);
                return self.run(stop + 1);
            },
            .NativeFunction(n) => return self.callNative(n, f, this, args, Value.undefined),
            .NativeClosure(n, _) => return self.callNative(n, f, this, args, Value.undefined),
            .BoundFunction(target, boundThis, boundArgs) => {
                var all = boundArgs;
                for a in args { all.append(a); }
                return self.callValue(target, boundThis, all);
            },
            _ => return self.throwType("not a function")
        }
    }

    /// `new f(...args)` from native code.
    mutating func construct(f: Value, args: Array[Value], newTarget: Value) -> Result[Value, Value] {
        if not self.isConstructor(f) { return self.throwType("\(self.describe(f)) is not a constructor"); }
        match self.kindAt(f.asRef) {
            .Closure(_, _, _, _) => {
                let this = try self.constructThis(f, newTarget);
                let stop = self.frames.count;
                try self.pushFrame(f, this, args, newTarget, true, -1);
                return self.run(stop + 1);
            },
            .NativeFunction(n) => return self.callNative(n, f, Value.undefined, args, newTarget),
            .BoundFunction(target, _, boundArgs) => {
                var all = boundArgs;
                for a in args { all.append(a); }
                let nt = if newTarget.same(f) { target } else { newTarget };
                return self.construct(target, all, nt);
            },
            _ => return self.throwType("not a constructor")
        }
    }

    /// The object `new` makes for a constructor (empty for derived ones,
    /// whose `this` comes from `super()`).
    mutating func constructThis(f: Value, newTarget: Value) -> Result[Value, Value] {
        if let .Closure(code, _, _, _) = self.kindAt(f.asRef) {
            if self.codes(code).isDerived { return .Ok(Value.empty); }
        }
        let proto = try self.prototypeFor(newTarget, self.intrinsics.objectProto);
        .Ok(self.newObjectKind(proto, .Ordinary))
    }

    /// `newTarget.prototype`, or `fallback` if that isn't an object.
    mutating func prototypeFor(newTarget: Value, fallback: Value) -> Result[Value, Value] {
        if not newTarget.isObject { return .Ok(fallback); }
        let p = try self.getFrom(newTarget, self.names.prototype, newTarget);
        .Ok(if p.isObject { p } else { fallback })
    }

    /// Pushes a frame to run closure `f`.
    mutating func pushFrame(f: Value, this: Value, args: Array[Value], newTarget: Value, isConstruct: Bool, returnTo: Int64) -> Result[(), Value] {
        let (code, env) = match self.kindAt(f.asRef) {
            .Closure(c, e, _, _) => (c, e),
            _ => return self.throwType("not a function")
        };
        let c = self.codes(code);
        if c.isClassConstructor and not isConstruct {
            return self.throwType("Class constructor \(c.name) cannot be invoked without 'new'");
        }
        if self.depth > 2000 {
            return .Err(self.makeError(.RangeError, "Maximum call stack size exceeded"));
        }
        var slots: [Value] = [];
        for _ in 0..<c.envSize { slots.append(Value.undefined); }
        if not c.isArrow {
            slots(0) = if c.isStrict or isConstruct { this } else { try self.sloppyThis(this) };
        }
        let e = self.heap.allocEnv(Env(slots: slots, parent: env));
        let base = self.stack.count;
        for _ in 0..<c.registers { self.stack.append(Value.undefined); }
        self.frames.append(Frame(
            code: code, pc: 0, base: base, env: e, funcEnv: e, args: args, callee: f,
            newTarget: newTarget, isConstruct: isConstruct, returnTo: returnTo, handlers: [],
            generator: Value.undefined
        ));
        self.depth = self.depth + 1;
        .Ok(())
    }

    /// Sloppy functions see the global object for a missing `this`, and
    /// primitives boxed.
    mutating func sloppyThis(this: Value) -> Result[Value, Value] {
        if this.isNullish { return .Ok(self.global); }
        self.toObject(this)
    }

    /// Returns from the top frame with `v`.
    mutating func doReturn(v: Value, stop: Int64) -> Result[Step, Value] {
        let fi = self.frames.count - 1;
        let f = self.frames(fi);
        var result = v;
        if f.isConstruct and not v.isObject {
            let derived = match self.kindAt(f.callee.asRef) {
                .Closure(code, _, _, _) => self.codes(code).isDerived,
                _ => false
            };
            if derived and not v.isUndefined {
                return self.throwType("Derived constructors may only return object or undefined");
            }
            result = self.heap.envs(f.funcEnv).slots(0);
            if result.isEmpty {
                return .Err(self.makeError(.ReferenceError, "Must call super constructor in derived class before returning"));
            }
        }
        self.popFrame();
        if self.frames.count < stop or f.returnTo < 0 { return .Ok(.Done(result)); }
        self.stack(f.returnTo) = result;
        .Ok(.Next)
    }

    /// `f(...args)` from the bytecode: closures get a frame, natives run now.
    mutating func callFromBytecode(fi: Int64, dst: Int64, f: Value, this: Value, args: Array[Value]) -> Result[(), Value] {
        if self.heap.wantsCollection() { self.collectGarbage(); }
        if f.isObject {
            if let .Closure(_, _, _, _) = self.kindAt(f.asRef) {
                let returnTo = self.frames(fi).base + dst;
                return self.pushFrame(f, this, args, Value.undefined, false, returnTo);
            }
        }
        let r = try self.callValue(f, this, args);
        self.setReg(fi, dst, r);
        .Ok(())
    }

    mutating func newFromBytecode(fi: Int64, dst: Int64, f: Value, args: Array[Value]) -> Result[(), Value] {
        if self.heap.wantsCollection() { self.collectGarbage(); }
        if not self.isConstructor(f) { return self.throwType("\(self.describe(f)) is not a constructor"); }
        if let .Closure(_, _, _, _) = self.kindAt(f.asRef) {
            let this = try self.constructThis(f, f);
            let returnTo = self.frames(fi).base + dst;
            return self.pushFrame(f, this, args, f, true, returnTo);
        }
        let r = try self.construct(f, args, f);
        self.setReg(fi, dst, r);
        .Ok(())
    }

    /// A short description of a value for error messages.
    /// How an uncaught exception prints: `String(e)` for objects with a
    /// usable toString, else a description.
    mutating func errorText(e: Value) -> String {
        match self.toText(e) {
            .Ok(s) => s,
            .Err(_) => self.describe(e)
        }
    }

    mutating func describe(v: Value) -> String {
        if v.isString { return "\"\(self.text(v))\""; }
        if v.isObject {
            if self.isCallable(v) { return "function"; }
            return "object";
        }
        if v.isSymbol { return "Symbol()"; }
        match self.toText(v) {
            .Ok(s) => s,
            .Err(_) => "value"
        }
    }

    /// The array a value holds, for spread calls.
    func arrayElements(v: Value) -> Array[Value] {
        var out: [Value] = [];
        for e in self.heap.objects(v.asRef).elements {
            out.append(if e.isEmpty { Value.undefined } else { e });
        }
        out
    }

    // ------------------------------------------------------------------
    // Function objects
    // ------------------------------------------------------------------

    mutating func makeClosureObject(code: Int64, env: Int64, home: Value) -> Value {
        let c = self.codes(code);
        let f = self.newObjectKind(self.intrinsics.functionProto, .Closure(code: code, env: env, home: home, fields: Value.undefined));
        let r = f.asRef;
        self.heap.objects(r).isConstructor = c.isConstructor;
        self.defineReadOnly(r, "length", Value.int(c.length));
        let name = self.str(c.name);
        self.defineReadOnly(r, "name", name);
        if c.isAsync and c.isGenerator {
            self.heap.objects(r).proto = self.intrinsics.asyncGeneratorFunctionProto;
            let proto = self.newObjectKind(self.intrinsics.asyncGeneratorProto, .Ordinary);
            self.defineOwn(r, self.names.prototype, proto, Attr.writable);
        } else if c.isAsync {
            self.heap.objects(r).proto = self.intrinsics.asyncFunctionProto;
        } else if c.isGenerator {
            // Generator functions inherit from %GeneratorFunction.prototype%
            // and own a prototype for their generator objects.
            self.heap.objects(r).proto = self.intrinsics.generatorFunctionProto;
            let proto = self.newObjectKind(self.intrinsics.generatorProto, .Ordinary);
            self.defineOwn(r, self.names.prototype, proto, Attr.writable);
        } else if c.isConstructor and not c.isClassConstructor {
            let proto = self.newObject();
            self.defineHidden(proto.asRef, self.names.constructor, f);
            self.defineOwn(r, self.names.prototype, proto, Attr.writable);
        }
        f
    }

    /// GeneratorStart: moves the top frame into a new generator object,
    /// which becomes the call's result.
    mutating func startGenerator(fi: Int64, stop: Int64) -> Result[Step, Value] {
        let code = self.codes(self.frames(fi).code);
        if code.isAsync and not code.isGenerator { return self.startAsync(fi, stop); }
        let callee = self.frames(fi).callee;
        var proto = try self.getV(callee, self.names.prototype);
        if not proto.isObject { proto = if code.isAsync { self.intrinsics.asyncGeneratorProto } else { self.intrinsics.generatorProto }; }
        let placeholder = GenData(frame: self.frames(fi), registers: [], state: GenState.suspendedStart, resumeReg: -1, raw: false, asyncPromise: Value.undefined, isAsyncGenerator: code.isAsync, queue: []);
        let id = self.heap.allocGenerator(placeholder);
        let gen = self.newObjectKind(proto, .Generator(id: id));
        self.frames(fi).generator = gen;
        self.suspendFrame(fi, id);
        self.doReturn(gen, stop)
    }

    /// Saves frame `fi` (the top one) and its registers into generator `id`.
    mutating func suspendFrame(fi: Int64, id: Int64) {
        let f = self.frames(fi);
        var regs: [Value] = [];
        for i in f.base..<self.stack.count { regs.append(self.stack(i)); }
        self.heap.generators(id).frame = f;
        self.heap.generators(id).registers = regs;
    }

    /// next/throw/return on a generator: `mode` 0, 1 or 2.
    mutating func resumeGenerator(gen: Value, value: Value, mode: Int64) -> Result[Value, Value] {
        let id = match self.kindAt(gen.asRef) {
            .Generator(x) => x,
            _ => -1
        };
        let state = self.heap.generators(id).state;
        if state == GenState.executing { return self.throwType("Generator is already running"); }
        if state == GenState.completed or (state == GenState.suspendedStart and mode != 0) {
            self.finishGenerator(id);
            if mode == 1 { return .Err(value); }
            return .Ok(self.iterResult(if mode == 2 { value } else { Value.undefined }, true));
        }
        let result = self.enterGenerator(id, value, mode);
        let yielded = self.heap.generators(id).state == GenState.suspendedYield;
        match result {
            .Err(e) => {
                self.finishGenerator(id);
                .Err(e)
            },
            .Ok(v) => {
                if yielded {
                    if self.heap.generators(id).raw { return .Ok(v); }
                    return .Ok(self.iterResult(v, false));
                }
                self.finishGenerator(id);
                .Ok(self.iterResult(v, true))
            }
        }
    }

    /// Puts generator `id`'s frame back on top of the stack, hands it the
    /// sent value and mode, and runs it until it yields or finishes. The
    /// generator's state then tells which.
    mutating func enterGenerator(id: Int64, value: Value, mode: Int64) -> Result[Value, Value] {
        if self.depth > 2000 { return .Err(self.makeError(.RangeError, "Maximum call stack size exceeded")); }
        var f = self.heap.generators(id).frame;
        f.base = self.stack.count;
        f.returnTo = -1;
        let regs = self.heap.generators(id).registers;
        for v in regs { self.stack.append(v); }
        self.heap.generators(id).registers = [];
        self.frames.append(f);
        self.depth = self.depth + 1;
        if self.heap.generators(id).resumeReg >= 0 {
            let at = f.base + self.heap.generators(id).resumeReg;
            self.stack(at) = value;
            self.stack(at + 1) = Value.int(mode);
        }
        self.heap.generators(id).state = GenState.executing;
        self.run(self.frames.count)
    }

    mutating func finishGenerator(id: Int64) {
        self.heap.generators(id).state = GenState.completed;
        self.heap.generators(id).registers = [];
    }

    /// The home object of the function running in frame `fi`.
    func homeOf(fi: Int64) -> Value {
        let callee = self.frames(fi).callee;
        if let .Closure(_, _, home, _) = self.kindAt(callee.asRef) { return home; }
        Value.undefined
    }

    mutating func setHome(f: Value, home: Value) {
        if let .Closure(code, env, _, fields) = self.kindAt(f.asRef) {
            self.heap.objects(f.asRef).kind = .Closure(code: code, env: env, home: home, fields: fields);
        }
    }

    /// An unmapped `arguments` object.
    mutating func makeArguments(fi: Int64) -> Value {
        let a = self.newObjectKind(self.intrinsics.objectProto, .ArgumentsObject);
        let r = a.asRef;
        let args = self.frames(fi).args;
        for i in 0..<args.count {
            let k = self.internIndex(i);
            self.defineOwn(r, k, args(i), Attr.all);
        }
        self.defineHidden(r, self.names.length, Value.int(args.count));
        let strict = self.codes(self.frames(fi).code).isStrict;
        if strict {
            let t = self.intrinsics.throwTypeError;
            self.defineAccessorOwn(r, self.names.callee, t, t, 0);
        } else {
            self.defineHidden(r, self.names.callee, self.frames(fi).callee);
        }
        let values = self.ownValue(self.intrinsics.arrayProto.asRef, self.atom("values"));
        self.defineHidden(r, symbolKey(WellKnown.iterator), values);
        a
    }
}
