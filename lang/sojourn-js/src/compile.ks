module js

// The bytecode compiler: syntax tree to `Code`.
//
// Each function gets a `FnState` on `self.fns`. Expressions compile into
// a destination register; temporaries are allocated upward and released
// at the end of each statement. Variables resolve at compile time to
// (hops, slot) through the scope stacks of every enclosing function,
// matching the environment chain the interpreter builds; names that
// resolve nowhere are globals.

import std.collections.(Dictionary, Set)

/// How a pattern's names get their values.
@derive(Equatable)
enum BindMode {
    /// `a = b` and friends: the names already exist.
    case Assign
    /// A declaration or parameter initializing its binding.
    case Declare
}

struct Resolved {
    var found: Bool
    var hops: Int64
    var slot: Int64
    var isConst: Bool
    /// Hops to each `with` scope crossed before the binding, innermost
    /// first: the name is looked up on those objects first.
    var withHops: [Int64]
}

/// What kind of function is being compiled.
struct FnFlags {
    var isArrow: Bool
    var isMethod: Bool
    var isClassConstructor: Bool
    var isDerived: Bool
    var hasFields: Bool
}

func plainFlags() -> FnFlags {
    FnFlags(isArrow: false, isMethod: false, isClassConstructor: false, isDerived: false, hasFields: false)
}

func compileError(message: String) -> SyntaxError {
    SyntaxError(message: message, position: 0, early: false)
}

extend VM {
    // ------------------------------------------------------------------
    // Emitting
    // ------------------------------------------------------------------

    func fnIndex() -> Int64 { self.fns.count - 1 }

    mutating func emit(i: Instr) -> Int64 {
        let f = self.fnIndex();
        self.fns(f).instrs.append(i);
        self.fns(f).instrs.count - 1
    }

    func here() -> Int64 { self.fns(self.fnIndex()).instrs.count }

    mutating func reg() -> Int64 {
        let f = self.fnIndex();
        let r = self.fns(f).nextReg;
        self.fns(f).nextReg = r + 1;
        if r + 1 > self.fns(f).maxReg { self.fns(f).maxReg = r + 1; }
        r
    }

    /// `n` consecutive registers, returning the first.
    mutating func regs(n: Int64) -> Int64 {
        let first = self.fns(self.fnIndex()).nextReg;
        for _ in 0..<n { let _ = self.reg(); }
        first
    }

    func regMark() -> Int64 { self.fns(self.fnIndex()).nextReg }

    mutating func freeRegs(to mark: Int64) { self.fns(self.fnIndex()).nextReg = mark; }

    func strict() -> Bool { self.fns(self.fnIndex()).strict }

    func scopeDepth() -> Int64 { self.fns(self.fnIndex()).scopes.count }

    /// Points the jump at `at` to `target`.
    mutating func patch(at: Int64, target: Int64) {
        let f = self.fnIndex();
        let i = self.fns(f).instrs(at);
        let patched = match i {
            .Jump(_) => Instr.Jump(target: target),
            .JumpIf(c, _, w) => Instr.JumpIf(cond: c, target: target, when: w),
            .JumpIfNullish(s, _) => Instr.JumpIfNullish(src: s, target: target),
            .JumpIfUndefined(s, _) => Instr.JumpIfUndefined(src: s, target: target),
            .JumpIfNotUndefined(s, _) => Instr.JumpIfNotUndefined(src: s, target: target),
            .WithGet(d, t, h, n, _) => Instr.WithGet(dst: d, this: t, hops: h, name: n, target: target),
            .WithSet(h, n, s, st, _) => Instr.WithSet(hops: h, name: n, src: s, strict: st, target: target),
            .EnterTry(_, e) => Instr.EnterTry(handler: target, exception: e),
            .ForInNext(d, it, _) => Instr.ForInNext(dst: d, iter: it, done: target),
            .IteratorNext(d, it, _) => Instr.IteratorNext(dst: d, iter: it, done: target),
            _ => i
        };
        self.fns(f).instrs(at) = patched;
    }

    mutating func patchHere(at: Int64) { self.patch(at, self.here()); }

    mutating func loadString(dst: Int64, s: String) {
        let a = self.atom(s);
        let _ = self.emit(.LoadString(dst: dst, atom: a));
    }

    mutating func load(dst: Int64, v: Value) { let _ = self.emit(.Load(dst: dst, value: v)); }

    // ------------------------------------------------------------------
    // Scopes and names
    // ------------------------------------------------------------------

    mutating func pushScope(s: CScope) {
        let f = self.fnIndex();
        self.fns(f).scopes.append(s);
    }

    mutating func popScope() {
        let f = self.fnIndex();
        let _ = self.fns(f).scopes.pop();
    }

    /// Finds `name` in the enclosing scopes of every function being
    /// compiled. `arguments` is declared on first use in the nearest
    /// non-arrow function.
    mutating func resolve(name: String) -> Resolved {
        var hops: Int64 = 0;
        var withHops: [Int64] = [];
        var fi = self.fns.count - 1;
        while fi >= 0 {
            var si = self.fns(fi).scopes.count - 1;
            while si >= 0 {
                let sc = self.fns(fi).scopes(si);
                if let .Some(slot) = sc.slots(name) {
                    return Resolved(found: true, hops: hops, slot: slot, isConst: sc.consts.contains(name), withHops: withHops);
                }
                if name == "arguments" and sc.isFunction and not sc.isArrow and not self.fns(fi).isScript {
                    let slot = self.declareArguments(fi, si);
                    return Resolved(found: true, hops: hops, slot: slot, isConst: false, withHops: withHops);
                }
                if sc.isWith { withHops.append(hops); }
                hops = hops + 1;
                si = si - 1;
            }
            fi = fi - 1;
        }
        Resolved(found: false, hops: 0, slot: 0, isConst: false, withHops: withHops)
    }

    /// Adds an `arguments` binding to function `fi`'s top scope and fills
    /// in the prologue placeholder that creates it.
    mutating func declareArguments(fi: Int64, si: Int64) -> Int64 {
        let slot = self.fns(fi).scopes(si).declare("arguments");
        if not self.fns(fi).usesArguments {
            self.fns(fi).usesArguments = true;
            let at = self.fns(fi).argumentsAt;
            self.fns(fi).instrs(at) = .CreateArguments(dst: 0);
            self.fns(fi).instrs(at + 1) = .InitVar(hops: 0, slot: slot, src: 0);
        }
        slot
    }

    /// Reads variable `name` into `dst`.
    mutating func emitGetName(name: String, dst: Int64, forTypeof: Bool) {
        self.emitGetNameThis(name, dst, -1, forTypeof);
    }

    /// Reads `name` into `dst`. If a `with` object supplies it, that
    /// object goes in `this` (unless -1), for calls.
    mutating func emitGetNameThis(name: String, dst: Int64, this: Int64, forTypeof: Bool) {
        let r = self.resolve(name);
        var jumps: [Int64] = [];
        for h in r.withHops {
            jumps.append(self.emit(.WithGet(dst: dst, this: this, hops: h, name: self.atom(name), target: 0)));
        }
        self.emitGetResolved(name, r, dst, forTypeof);
        for j in jumps { self.patchHere(j); }
    }

    /// Reads `name` from the binding `resolve` found, or the global object.
    mutating func emitGetResolved(name: String, r: Resolved, dst: Int64, forTypeof: Bool) {
        if r.found {
            let _ = self.emit(.GetVar(dst: dst, hops: r.hops, slot: r.slot, name: self.atom(name)));
        } else {
            let _ = self.emit(.GetGlobal(dst: dst, name: self.atom(name), forTypeof: forTypeof));
        }
    }

    /// Gives `name` the value in `src`, trying enclosing `with` objects
    /// first. A `var` initializer inside `with` goes through them too.
    mutating func emitSetName(name: String, src: Int64, mode: BindMode) {
        let r = self.resolve(name);
        let a = self.atom(name);
        var jumps: [Int64] = [];
        for h in r.withHops {
            jumps.append(self.emit(.WithSet(hops: h, name: a, src: src, strict: self.strict(), target: 0)));
        }
        self.emitSetResolved(name, r, src, mode);
        for j in jumps { self.patchHere(j); }
    }

    /// Gives the binding `resolve` found (or the global) the value in `src`.
    mutating func emitSetResolved(name: String, r: Resolved, src: Int64, mode: BindMode) {
        let a = self.atom(name);
        if r.found {
            if mode == BindMode.Declare {
                let _ = self.emit(.InitVar(hops: r.hops, slot: r.slot, src: src));
                return;
            }
            if r.isConst {
                // Reading first throws for the temporal dead zone.
                let t = self.reg();
                let _ = self.emit(.GetVar(dst: t, hops: r.hops, slot: r.slot, name: a));
                let _ = self.emit(.ThrowError(kind: .TypeError, message: "Assignment to constant variable '\(name)'."));
                return;
            }
            let _ = self.emit(.SetVar(hops: r.hops, slot: r.slot, src: src, name: a));
            return;
        }
        if mode == BindMode.Declare and self.globalLexicalNames.contains(name) {
            let _ = self.emit(.InitGlobalLexical(name: a, src: src));
            return;
        }
        let _ = self.emit(.SetGlobal(name: a, src: src, strict: self.strict()));
    }

    /// Opens a scope for a block's lexical declarations, if it has any,
    /// and creates its functions. Returns whether a scope was pushed.
    mutating func enterBlockScope(stmts: Array[Stmt], extra: Array[String]) -> Result[Bool, SyntaxError] {
        var names: [String] = [];
        var consts: [String] = [];
        collectLexicalNames(stmts, true, names, consts);
        if names.isEmpty and extra.isEmpty { return .Ok(false); }
        var sc = newCScope(false, false);
        for n in extra { let _ = sc.declare(n); }
        for n in names { let _ = sc.declare(n); }
        for n in consts { let _ = sc.consts.insert(n); }
        let size = sc.size;
        self.pushScope(sc);
        let _ = self.emit(.PushEnv(size: size));
        try self.initLexicals(names, stmts);
        .Ok(true)
    }

    /// Puts `names` in the temporal dead zone, then creates the block's
    /// function declarations.
    mutating func initLexicals(names: Array[String], stmts: Array[Stmt]) -> Result[(), SyntaxError] {
        let mark = self.regMark();
        let t = self.reg();
        self.load(t, Value.empty);
        for n in names {
            let r = self.resolve(n);
            let _ = self.emit(.InitVar(hops: r.hops, slot: r.slot, src: t));
        }
        for f in hoistedFunctions(stmts) {
            let name = f.id ?? "";
            let code = try self.compileFunction(f, name, plainFlags());
            let _ = self.emit(.MakeClosure(dst: t, code: code, home: -1));
            self.emitSetName(name, t, .Declare);
        }
        self.freeRegs(to: mark);
        .Ok(())
    }

    mutating func exitBlockScope(pushed: Bool) {
        if not pushed { return; }
        self.popScope();
        let _ = self.emit(.PopEnv);
    }

    // ------------------------------------------------------------------
    // Functions and scripts
    // ------------------------------------------------------------------

    mutating func beginFunction(strict: Bool, isArrow: Bool, isScript: Bool) {
        self.fns.append(FnState(
            instrs: [], scopes: [], nextReg: 1, maxReg: 1, ctl: [], strict: strict,
            isArrow: isArrow, isScript: isScript, pendingLabels: [], chainJumps: [],
            argumentsAt: 0, usesArguments: false, isDerivedCtor: false, hasFields: false,
            completion: -1, isGenerator: false, isAsync: false
        ));
        var top = newCScope(true, isArrow);
        if not isArrow { let _ = top.declare("this"); }
        self.pushScope(top);
        // Placeholders for creating `arguments`, filled in if it's used.
        let at = self.emit(.Nop);
        let _ = self.emit(.Nop);
        self.fns(self.fnIndex()).argumentsAt = at;
    }

    /// Finishes the function on top of `self.fns`, returning its code index.
    mutating func endFunction(name: String, length: Int64, flags: FnFlags) -> Int64 {
        let t = self.reg();
        self.load(t, Value.undefined);
        let _ = self.emit(.Return(src: t));
        let f = self.fns.pop() ?? panicFn();
        let code = Code(
            name: name, instrs: f.instrs, registers: f.maxReg, envSize: f.scopes(0).size,
            length: length, isArrow: flags.isArrow, isStrict: f.strict,
            isConstructor: not flags.isArrow and not flags.isMethod and not f.isGenerator and not f.isAsync,
            isClassConstructor: flags.isClassConstructor, isDerived: flags.isDerived,
            isGenerator: f.isGenerator, isAsync: f.isAsync,
            sourceName: self.sourceName
        );
        self.codes.append(code);
        self.codes.count - 1
    }

    /// Compiles a script, returning its code index.
    mutating func compileScript(p: Program) -> Result[Int64, SyntaxError] {
        self.beginFunction(hasUseStrict(p.body), false, true);
        let f = self.fnIndex();
        self.fns(f).strict = hasUseStrict(p.body);
        var vars: [String] = [];
        collectVarNames(p.body, vars);
        var lexicals: [String] = [];
        var consts: [String] = [];
        collectLexicalNames(p.body, false, lexicals, consts);
        for n in vars { let _ = self.emit(.DeclareGlobalVar(name: self.atom(n))); }
        for n in lexicals {
            let _ = self.globalLexicalNames.insert(n);
            let _ = self.emit(.DeclareGlobalLexical(name: self.atom(n), isConst: consts.contains(n)));
        }
        let t = self.reg();
        for fun in hoistedFunctions(p.body) {
            let name = fun.id ?? "";
            let code = try self.compileFunction(fun, name, plainFlags());
            let _ = self.emit(.MakeClosure(dst: t, code: code, home: -1));
            let _ = self.emit(.DeclareGlobalFunction(name: self.atom(name), src: t));
        }
        self.freeRegs(to: 1);
        let completion = self.reg();
        self.load(completion, Value.undefined);
        self.fns(f).completion = completion;
        for s in p.body { try self.compileStmt(s); }
        let _ = self.emit(.Return(src: completion));
        .Ok(self.endFunction("<script>", 0, plainFlags()))
    }

    /// Compiles a function, returning its code index.
    mutating func compileFunction(f: Function, name: String, flags: FnFlags) -> Result[Int64, SyntaxError] {
        let body = match f.body { .Block(b) => b, .Expression(_) => [] };
        let strict = self.strict() or hasUseStrict(body) or flags.isClassConstructor;
        self.beginFunction(strict, flags.isArrow, false);
        let fi = self.fnIndex();
        self.fns(fi).isDerivedCtor = flags.isDerived;
        self.fns(fi).hasFields = flags.hasFields;
        self.fns(fi).isGenerator = f.isGenerator;
        self.fns(fi).isAsync = f.isAsync;

        // Declare everything the function's top scope holds.
        let top = self.fns(fi).scopes.count - 1;
        for n in boundNames(f.params) { let _ = self.fns(fi).scopes(top).declare(n); }
        var vars: [String] = [];
        collectVarNames(body, vars);
        for n in vars { let _ = self.fns(fi).scopes(top).declare(n); }
        var lexicals: [String] = [];
        var consts: [String] = [];
        collectLexicalNames(body, false, lexicals, consts);
        for n in lexicals { let _ = self.fns(fi).scopes(top).declare(n); }
        for n in consts { let _ = self.fns(fi).scopes(top).consts.insert(n); }
        for fun in hoistedFunctions(body) { let _ = self.fns(fi).scopes(top).declare(fun.id ?? ""); }

        // An async function's parameters run inside it, so their errors
        // reject its promise.
        if f.isAsync and not f.isGenerator { let _ = self.emit(.GeneratorStart); }
        // Parameters, then hoisted functions.
        let t = self.reg();
        self.load(t, Value.empty);
        for n in lexicals {
            let r = self.resolve(n);
            let _ = self.emit(.InitVar(hops: r.hops, slot: r.slot, src: t));
        }
        for i in 0..<f.params.count {
            let mark = self.regMark();
            let v = self.reg();
            match f.params(i) {
                .Rest(p) => {
                    let _ = self.emit(.GetRestArgs(dst: v, from: i));
                    try self.bindPattern(p, v, .Declare);
                },
                p => {
                    let _ = self.emit(.GetArg(dst: v, index: i));
                    try self.bindPattern(p, v, .Declare);
                }
            }
            self.freeRegs(to: mark);
        }
        for fun in hoistedFunctions(body) {
            let n = fun.id ?? "";
            let code = try self.compileFunction(fun, n, plainFlags());
            let _ = self.emit(.MakeClosure(dst: t, code: code, home: -1));
            self.emitSetName(n, t, .Declare);
        }
        if flags.isClassConstructor and not flags.isDerived and flags.hasFields {
            self.emitGetName("this", t, false);
            let _ = self.emit(.InitFields(this: t));
        }
        // A generator's body runs on the first next(); parameters and
        // hoisting happen at the call.
        if f.isGenerator { let _ = self.emit(.GeneratorStart); }
        self.freeRegs(to: 1);

        match f.body {
            .Block(stmts) => {
                for s in stmts { try self.compileStmt(s); }
            },
            .Expression(e) => {
                let r = self.reg();
                try self.compileExpr(e, r);
                let _ = self.emit(.Return(src: r));
            }
        }
        .Ok(self.endFunction(name, expectedArgumentCount(f.params), flags))
    }

    // ------------------------------------------------------------------
    // Patterns
    // ------------------------------------------------------------------

    /// Binds pattern `p` to the value in register `src`.
    mutating func bindPattern(p: Pattern, src: Int64, mode: BindMode) -> Result[(), SyntaxError] {
        match p {
            .Identifier(name) => self.emitSetName(name, src, mode),
            .Expression(e) => try self.assignToExpression(e, src),
            .WithDefault(target, value) => {
                let v = self.reg();
                let _ = self.emit(.Move(dst: v, src: src));
                let skip = self.emit(.JumpIfNotUndefined(src: v, target: 0));
                match target {
                    .Identifier(n) => try self.compileNamed(value, v, n),
                    _ => try self.compileExpr(value, v)
                }
                self.patchHere(skip);
                try self.bindPattern(target, v, mode);
            },
            .ObjectPattern(props, rest) => try self.bindObjectPattern(props, rest, src, mode),
            .ArrayPattern(elements, rest) => try self.bindArrayPattern(elements, rest, src, mode),
            .Rest(inner) => try self.bindPattern(inner, src, mode)
        }
        .Ok(())
    }

    mutating func bindObjectPattern(props: Array[PatternProperty], rest: Optional[Pattern], src: Int64, mode: BindMode) -> Result[(), SyntaxError] {
        let checked = self.reg();
        let _ = self.emit(.Unary(op: .RequireObject, dst: checked, src: src));
        // Keys go in consecutive registers so a rest element can skip them.
        let keys = self.regs(props.count);
        for i in 0..<props.count {
            let prop = props(i);
            try self.compileKey(prop.key, keys + i);
            let v = self.reg();
            let _ = self.emit(.GetElem(dst: v, obj: src, key: keys + i));
            try self.bindPattern(prop.value, v, mode);
        }
        if let .Some(r) = rest {
            let obj = self.reg();
            let _ = self.emit(.NewObject(dst: obj));
            let _ = self.emit(.CopyDataProps(dst: obj, src: src, excluded: keys, count: props.count));
            try self.bindPattern(r, obj, mode);
        }
        .Ok(())
    }

    mutating func bindArrayPattern(elements: Array[Optional[Pattern]], rest: Optional[Pattern], src: Int64, mode: BindMode) -> Result[(), SyntaxError] {
        let it = self.regs(2);
        let _ = self.emit(.GetIterator(iter: it, obj: src));
        let done = self.reg();
        self.load(done, Value.bool(false));
        for el in elements {
            let v = self.reg();
            self.load(v, Value.undefined);
            let skip = self.emit(.JumpIf(cond: done, target: 0, when: true));
            let next = self.emit(.IteratorNext(dst: v, iter: it, done: 0));
            let over = self.emit(.Jump(target: 0));
            self.patchHere(next);
            self.load(done, Value.bool(true));
            self.patchHere(skip);
            self.patchHere(over);
            if let .Some(p) = el { try self.bindPattern(p, v, mode); }
        }
        if let .Some(r) = rest {
            let arr = self.reg();
            let _ = self.emit(.NewArray(dst: arr));
            let skip = self.emit(.JumpIf(cond: done, target: 0, when: true));
            let v = self.reg();
            let top = self.here();
            let next = self.emit(.IteratorNext(dst: v, iter: it, done: 0));
            let _ = self.emit(.ArrayAppend(arr: arr, src: v));
            let _ = self.emit(.Jump(target: top));
            self.patchHere(next);
            self.patchHere(skip);
            try self.bindPattern(r, arr, mode);
            return .Ok(());
        }
        let skip = self.emit(.JumpIf(cond: done, target: 0, when: true));
        let _ = self.emit(.IteratorClose(iter: it));
        self.patchHere(skip);
        .Ok(())
    }

    /// `obj.x = src`, `obj[k] = src`.
    mutating func assignToExpression(e: Expr, src: Int64) -> Result[(), SyntaxError] {
        match e.kind {
            .Identifier(n) => self.emitSetName(n, src, .Assign),
            .Member(object, property, _) => {
                if let .Super = object.kind {
                    let key = self.reg();
                    try self.compileMemberKey(property, key);
                    let this = self.reg();
                    self.emitGetName("this", this, false);
                    let _ = self.emit(.SuperSet(key: key, src: src, this: this));
                    return .Ok(());
                }
                let obj = self.reg();
                try self.compileExpr(object, obj);
                match property {
                    .Name(n) => {
                        let ic = self.newIC();
                        let _ = self.emit(.SetProp(obj: obj, key: self.atom(n), src: src, ic: ic, strict: self.strict()));
                    },
                    _ => {
                        let key = self.reg();
                        try self.compileMemberKey(property, key);
                        let _ = self.emit(.SetElem(obj: obj, key: key, src: src, strict: self.strict()));
                    }
                }
            },
            _ => throw compileError("invalid assignment target")
        }
        .Ok(())
    }

    /// A property key into `dst`: a string, or a computed key converted
    /// with ToPropertyKey.
    mutating func compileKey(k: PropertyKey, dst: Int64) -> Result[(), SyntaxError] {
        match k {
            .Name(n) => self.loadString(dst, n),
            .StringKey(s) => self.loadString(dst, s),
            .NumberKey(n) => self.loadString(dst, numberToString(n)),
            .BigIntKey(s) => self.loadString(dst, s),
            .Computed(e) => {
                try self.compileExpr(e, dst);
                let _ = self.emit(.Unary(op: .ToPropertyKey, dst: dst, src: dst));
            },
            .Private(n) => try self.emitPrivateName(n, dst)
        }
        .Ok(())
    }

    /// Loads private name `#n`; naming one no enclosing class declares is
    /// an early error.
    mutating func emitPrivateName(n: String, dst: Int64) -> Result[(), SyntaxError] {
        let r = self.resolve("#" + n);
        if not r.found { throw compileError("Private field '#\(n)' must be declared in an enclosing class"); }
        self.emitGetName("#" + n, dst, false);
        .Ok(())
    }

    mutating func compileMemberKey(p: MemberProperty, dst: Int64) -> Result[(), SyntaxError] {
        match p {
            .Name(n) => self.loadString(dst, n),
            .Computed(e) => try self.compileExpr(e, dst),
            .Private(n) => try self.emitPrivateName(n, dst)
        }
        .Ok(())
    }

    /// The name a key gives an anonymous function, when it's known now.
    func keyName(k: PropertyKey) -> String {
        match k {
            .Name(n) => n,
            .StringKey(s) => s,
            .NumberKey(n) => numberToString(n),
            .BigIntKey(s) => s,
            .Private(n) => "#" + n,
            _ => ""
        }
    }

    mutating func newIC() -> Int64 {
        self.caches.append(InlineCache(shape: -1, slot: 0));
        self.caches.count - 1
    }
}

func panicFn() -> FnState {
    fatalError("compiler function stack underflow")
}
