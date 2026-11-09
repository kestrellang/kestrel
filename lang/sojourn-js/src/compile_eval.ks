module js

// Direct eval: `eval(src)` by name runs `src` in the caller's scope.
//
// The call site snapshots the compiler's function stack (scopes only)
// into `VM.evalContexts`. At run time the eval code compiles on top of
// that snapshot as an arrow function closing over the caller's current
// env, so its names resolve to the same (hops, slot) addresses the
// caller's code uses. A sloppy eval's `var`s go to the caller's var
// scope: the global object, or the eval-vars object a sloppy function
// containing a direct eval keeps (`CScope.hasEvalVars`).

import std.collections.(Dictionary, Set)

/// `eval(...)` written as a plain name: a direct eval candidate.
func isDirectEvalCallee(callee: Expr) -> Bool {
    if let .Identifier(n) = callee.kind { return n == "eval"; }
    false
}

extend VM {
    /// Sets up function `fi` (whose body contains a direct eval) so eval
    /// code can see and add to its scope: `arguments` exists, and a
    /// sloppy function gets an eval-vars object.
    mutating func prepareDirectEval(fi: Int64, si: Int64) {
        let _ = self.resolve("arguments");
        if self.fns(fi).strict { return; }
        let slot = self.fns(fi).scopes(si).declare("%evalvars");
        self.fns(fi).scopes(si).hasEvalVars = true;
        self.fns(fi).scopes(si).evalSlot = slot;
        let _ = self.emit(.InitEvalVars(slot: slot));
    }

    /// Records the scopes visible at this call site, returning the
    /// DirectEval context index.
    mutating func snapshotEvalContext() -> Int64 {
        var snap = self.fns;
        for i in 0..<snap.count {
            snap(i).instrs = [];
            snap(i).ctl = [];
            snap(i).chainJumps = [];
            snap(i).pendingLabels = [];
        }
        self.evalContexts.append(EvalContext(fns: snap));
        self.evalContexts.count - 1
    }

    /// What eval code may contain, from the nearest non-arrow function
    /// around the call site.
    func evalSyntax(ctx: Int64) -> EvalSyntax {
        let fns = self.evalContexts(ctx).fns;
        var syntax = EvalSyntax(strict: fns(fns.count - 1).strict, inFunction: false, allowNewTarget: false,
            allowSuperProperty: false, allowSuperCall: false, inFieldInit: false);
        var i = fns.count - 1;
        while i >= 0 {
            let f = fns(i);
            if not f.isArrow {
                if f.isScript { return syntax; }
                syntax.inFunction = true;
                syntax.allowNewTarget = true;
                syntax.allowSuperProperty = f.isMethod;
                syntax.allowSuperCall = f.isDerivedCtor;
                syntax.inFieldInit = f.isFieldInit;
                return syntax;
            }
            i = i - 1;
        }
        syntax
    }

    /// Compiles eval code for call site `ctx`, returning its code index.
    /// Leaves `self.fns` empty.
    mutating func compileEval(p: Program, ctx: Int64) -> Result[Int64, SyntaxError] {
        self.fns = self.evalContexts(ctx).fns;
        let strict = self.strict() or hasUseStrict(p.body);
        self.beginFunction(strict, true, true);
        let fi = self.fnIndex();
        self.fns(fi).isEval = true;

        let funs = hoistedFunctions(p.body);
        var vars: [String] = [];
        collectVarNames(p.body, vars);
        for fun in funs { vars.append(fun.id ?? ""); }
        var lexicals: [String] = [];
        var consts: [String] = [];
        collectLexicalNames(p.body, false, lexicals, consts);
        for n in lexicals {
            let _ = self.fns(fi).scopes(0).declare(n);
            let _ = self.fns(fi).scopes(0).lexicals.insert(n);
        }
        for n in consts { let _ = self.fns(fi).scopes(0).consts.insert(n); }
        // Strict eval code keeps its vars; sloppy code gives them to the caller.
        if strict {
            for n in vars { let _ = self.fns(fi).scopes(0).declare(n); }
        } else {
            // Every check comes before any declaration.
            var funNames: [String] = [];
            for fun in funs { funNames.append(fun.id ?? ""); }
            for n in vars { try self.declareEvalVar(n, funNames.contains(n), true); }
            for n in vars { try self.declareEvalVar(n, funNames.contains(n), false); }
        }
        let globalVars = not strict and self.evalVarsAreGlobal();

        let t = self.reg();
        self.load(t, Value.empty);
        for n in lexicals {
            let r = self.resolve(n);
            let _ = self.emit(.InitVar(hops: r.hops, slot: r.slot, src: t));
        }
        for fun in funs {
            let name = fun.id ?? "";
            let code = try self.compileFunction(fun, name, plainFlags());
            let _ = self.emit(.MakeClosure(dst: t, code: code, home: -1));
            if globalVars {
                let _ = self.emit(.DeclareGlobalFunction(name: self.atom(name), src: t, deletable: true));
            } else {
                self.emitSetName(name, t, if strict { BindMode.Declare } else { BindMode.Assign });
            }
        }
        self.freeRegs(to: 1);
        let completion = self.reg();
        self.load(completion, Value.undefined);
        self.fns(fi).completion = completion;
        for s in p.body { try self.compileStmt(s); }
        let _ = self.emit(.Return(src: completion));
        var flags = plainFlags();
        flags.isArrow = true;
        let code = self.endFunction("eval", 0, flags);
        self.fns = [];
        .Ok(code)
    }

    /// Declares sloppy eval code's `var name` in the caller's var scope:
    /// the nearest scope that's a function top or the script, looking
    /// through other sloppy eval code. A lexical declaration in between
    /// conflicts with it.
    /// With `checkOnly`, only emits the checks a global declaration needs.
    mutating func declareEvalVar(name: String, isFunction: Bool, checkOnly: Bool) -> Result[(), SyntaxError] {
        var hops: Int64 = 0;
        var fi = self.fns.count - 1;
        while fi >= 0 {
            let f = self.fns(fi);
            var si = f.scopes.count - 1;
            while si >= 0 {
                let sc = f.scopes(si);
                // Parameter expressions can't see the body's declarations.
                let hidden = sc.isVarScope and f.inParams;
                if sc.lexicals.contains(name) and not hidden {
                    throw compileError("Identifier '\(name)' has already been declared");
                }
                if sc.isVarScope and not (f.isEval and not f.strict) {
                    if name == "arguments" and f.inParams and not f.isArrow {
                        throw compileError("eval in a parameter list can't declare 'arguments'");
                    }
                    if f.inParams and f.paramNames.contains(name) {
                        throw compileError("Identifier '\(name)' has already been declared");
                    }
                    try self.declareEvalVarIn(name, isFunction, checkOnly, f.isScript, sc, hops);
                    return .Ok(());
                }
                hops = hops + 1;
                si = si - 1;
            }
            fi = fi - 1;
        }
        .Ok(())
    }

    /// Emits the declaration of eval var `name` in var scope `sc`, `hops`
    /// envs out (the global object for a script).
    mutating func declareEvalVarIn(name: String, isFunction: Bool, checkOnly: Bool, isScript: Bool, sc: CScope, hops: Int64) -> Result[(), SyntaxError] {
        let a = self.atom(name);
        if isScript {
            if checkOnly {
                let _ = self.emit(if isFunction { Instr.CheckGlobalFunction(name: a) } else { Instr.CheckGlobalVar(name: a) });
            } else if not isFunction {
                let _ = self.emit(.DeclareEvalVar(hops: -1, slot: 0, name: a));
            }
            return .Ok(());
        }
        if checkOnly { return .Ok(()); }
        // An existing parameter or var is the binding already.
        if let .Some(_) = sc.slots(name) { return .Ok(()); }
        if sc.hasEvalVars {
            let _ = self.emit(.DeclareEvalVar(hops: hops, slot: sc.evalSlot, name: a));
        }
        .Ok(())
    }

    /// Whether sloppy eval code being compiled declares its vars on the
    /// global object: its nearest non-eval function is the script.
    func evalVarsAreGlobal() -> Bool {
        var fi = self.fns.count - 1;
        while fi >= 0 {
            let f = self.fns(fi);
            if not (f.isEval and not f.strict) { return f.isScript; }
            fi = fi - 1;
        }
        true
    }

    // ------------------------------------------------------------------
    // Run time
    // ------------------------------------------------------------------

    /// DirectEval with the realm's eval: runs `argv[0]` in frame `fi`'s
    /// scope, its completion value landing in register `dst`.
    mutating func directEval(fi: Int64, dst: Int64, argv: Array[Value], ctx: Int64) -> Result[(), Value] {
        let src = if argv.count > 0 { argv(0) } else { Value.undefined };
        if not src.isString {
            self.setReg(fi, dst, src);
            return .Ok(());
        }
        let program = match parseEvalSource(self.text(src), self.evalSyntax(ctx)) {
            .Ok(p) => p,
            .Err(e) => return .Err(self.makeError(.SyntaxError, e.message))
        };
        let code = match self.compileEval(program, ctx) {
            .Ok(c) => c,
            .Err(e) => {
                self.fns = [];
                return .Err(self.makeError(.SyntaxError, e.message));
            }
        };
        let fnObj = self.makeClosureObject(code, self.frames(fi).env, self.homeOf(fi));
        self.captureContext(fi, fnObj);
        let returnTo = self.frames(fi).base + dst;
        self.pushFrame(fnObj, Value.undefined, [], self.frames(fi).newTarget, false, returnTo)
    }

    /// `eval(src)` called any other way: runs in the global scope, as a
    /// direct eval at script level would.
    mutating func indirectEval(src: Value) -> Result[Value, Value] {
        if not src.isString { return .Ok(src); }
        let syntax = EvalSyntax(strict: false, inFunction: false, allowNewTarget: false,
            allowSuperProperty: false, allowSuperCall: false, inFieldInit: false);
        let program = match parseEvalSource(self.text(src), syntax) {
            .Ok(p) => p,
            .Err(e) => return .Err(self.makeError(.SyntaxError, e.message))
        };
        let ctx = self.globalEvalContext();
        let code = match self.compileEval(program, ctx) {
            .Ok(c) => c,
            .Err(e) => {
                self.fns = [];
                return .Err(self.makeError(.SyntaxError, e.message));
            }
        };
        // The script-level env the context describes: just `this`.
        let env = self.heap.allocEnv(Env(slots: [self.global], parent: self.globalEnv));
        let fnObj = self.makeClosureObject(code, env, Value.undefined);
        self.callValue(fnObj, Value.undefined, [])
    }

    /// The eval context of plain script-level code, made on first use.
    mutating func globalEvalContext() -> Int64 {
        if self.globalEvalCtx >= 0 { return self.globalEvalCtx; }
        self.fns = [];
        self.beginFunction(false, false, true);
        self.globalEvalCtx = self.snapshotEvalContext();
        self.fns = [];
        self.globalEvalCtx
    }

    /// DeclareEvalVar: a configurable var on the eval-vars object or the
    /// global object, unless it's there already.
    mutating func declareEvalVarAt(fi: Int64, hops: Int64, slot: Int64, name: Key) -> Result[(), Value] {
        let target = if hops < 0 { self.global } else { self.heap.envs(self.envAt(self.frames(fi).env, hops)).slots(slot) };
        if not target.isObject { return .Ok(()); }
        if self.hasOwn(target.asRef, name) { return .Ok(()); }
        self.defineOwn(target.asRef, name, Value.undefined, Attr.writable | Attr.enumerable | Attr.configurable);
        .Ok(())
    }
}

extend VM {
    /// CanDeclareGlobalVar / CanDeclareGlobalFunction, plus the clash with
    /// a global lexical binding.
    mutating func checkGlobalDecl(name: Key, isFunction: Bool) -> Result[(), Value] {
        if let .Some(_) = self.globalLexicals(name) {
            return .Err(self.makeError(.SyntaxError, "Identifier '\(self.keyText(name))' has already been declared"));
        }
        let g = self.global.asRef;
        let fail = self.makeError(.TypeError, "Cannot declare global '\(self.keyText(name))'");
        let existing = match self.ownProperty(g, name) {
            .Some(p) => p,
            .None => return if self.heap.objects(g).extensible { .Ok(()) } else { .Err(fail) }
        };
        if not isFunction { return .Ok(()); }
        if existing.attrs & Attr.configurable != 0 { return .Ok(()); }
        let plainData = existing.attrs & Attr.accessor == 0 and existing.attrs & Attr.writable != 0 and existing.attrs & Attr.enumerable != 0;
        if plainData { .Ok(()) } else { .Err(fail) }
    }

    /// A script-level lexical can't shadow a global var or a
    /// non-configurable global property.
    mutating func checkGlobalLexical(name: Key) -> Result[(), Value] {
        let clash = self.makeError(.SyntaxError, "Identifier '\(self.keyText(name))' has already been declared");
        if let .Some(_) = self.globalLexicals(name) { return .Err(clash); }
        if self.globalVarNames.contains(name) { return .Err(clash); }
        if let .Some(p) = self.ownProperty(self.global.asRef, name) {
            if p.attrs & Attr.configurable == 0 { return .Err(clash); }
        }
        .Ok(())
    }

    /// CreateGlobalFunctionBinding: a fresh data property when the old one
    /// is configurable (or missing), otherwise just the new value.
    /// Eval's (deletable) declarations don't count as global var names: a
    /// later script can declare a lexical over them.
    mutating func declareGlobalFunction(name: Key, v: Value, deletable: Bool) {
        if not deletable { let _ = self.globalVarNames.insert(name); }
        let g = self.global.asRef;
        let fresh = Attr.writable | Attr.enumerable | (if deletable { Attr.configurable } else { 0 });
        let attrs = match self.ownProperty(g, name) {
            .Some(p) => if p.attrs & Attr.configurable != 0 { fresh } else { p.attrs },
            .None => fresh
        };
        self.defineOwn(g, name, v, attrs);
    }
}
