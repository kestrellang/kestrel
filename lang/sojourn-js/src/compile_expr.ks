module js

// Compiling expressions, each into a destination register.

extend VM {
    /// Compiles `e`, naming it `name` if it's an anonymous function or class.
    mutating func compileNamed(e: Expr, dst: Int64, name: String) -> Result[(), SyntaxError] {
        if not isAnonymousFunction(e) { return self.compileExpr(e, dst); }
        match e.kind {
            .FunctionExpr(f) => try self.compileFunctionExpr(f, dst, name, false),
            .ArrowFunction(f) => try self.compileFunctionExpr(f, dst, name, true),
            .ClassExpr(c) => try self.compileClass(c, dst, name),
            _ => try self.compileExpr(e, dst)
        }
        .Ok(())
    }

    mutating func compileExpr(e: Expr, dst: Int64) -> Result[(), SyntaxError] {
        match e.kind {
            .Identifier(n) => self.emitGetName(n, dst, false),
            .This => self.emitGetName("this", dst, false),
            .Super => throw compileError("'super' must be a member access or a call"),
            .Null => self.load(dst, Value.nullValue),
            .BoolLit(b) => self.load(dst, Value.bool(b)),
            .Number(n) => self.load(dst, Value.number(n)),
            .BigInt(digits) => {
                let a = self.atom(digits);
                let _ = self.emit(.LoadBigInt(dst: dst, atom: a));
            },
            .StringLit(s) => self.loadString(dst, s),
            .Regex(pattern, flags) => {
                // An invalid literal is an early error.
                let p = unitsOf(pattern);
                let f = match parseRxFlags(unitsOf(flags)) {
                    .Some(x) => x,
                    .None => throw compileError("Invalid regular expression flags")
                };
                if let .Err(message) = compileRegex(p, f) {
                    throw compileError("Invalid regular expression: /\(pattern)/: \(message)");
                }
                let _ = self.emit(.NewRegExp(dst: dst, pattern: self.atom(pattern), flags: self.atom(flags)));
            },
            .Template(t) => try self.compileTemplate(t, dst),
            .TaggedTemplate(tag, quasi) => try self.compileTaggedTemplate(tag, quasi, dst),
            .ArrayLit(elements) => {
                let _ = self.emit(.NewArray(dst: dst));
                let t = self.reg();
                for el in elements {
                    match el {
                        .Hole => {
                            self.load(t, Value.empty);
                            let _ = self.emit(.ArrayAppend(arr: dst, src: t));
                        },
                        .Item(x) => {
                            try self.compileExpr(x, t);
                            let _ = self.emit(.ArrayAppend(arr: dst, src: t));
                        },
                        .Spread(x) => {
                            try self.compileExpr(x, t);
                            let _ = self.emit(.ArraySpread(arr: dst, src: t));
                        }
                    }
                }
            },
            .ObjectLit(props) => try self.compileObjectLiteral(props, dst),
            .FunctionExpr(f) => try self.compileFunctionExpr(f, dst, f.id ?? "", false),
            .ArrowFunction(f) => try self.compileFunctionExpr(f, dst, "", true),
            .ClassExpr(c) => try self.compileClass(c, dst, c.id ?? ""),
            .Unary(op, argument) => try self.compileUnary(op, argument, dst),
            .Update(op, prefix, argument) => try self.compileUpdate(op, prefix, argument, dst),
            .Binary(op, left, right) => {
                try self.compileExpr(left, dst);
                let r = self.reg();
                try self.compileExpr(right, r);
                let _ = self.emit(.Binary(op: binOp(op), dst: dst, a: dst, b: r));
            },
            .Logical(op, left, right) => {
                try self.compileExpr(left, dst);
                if op == "??" {
                    let toRight = self.emit(.JumpIfNullish(src: dst, target: 0));
                    let over = self.emit(.Jump(target: 0));
                    self.patchHere(toRight);
                    try self.compileExpr(right, dst);
                    self.patchHere(over);
                } else {
                    let over = self.emit(.JumpIf(cond: dst, target: 0, when: op == "||"));
                    try self.compileExpr(right, dst);
                    self.patchHere(over);
                }
            },
            .PrivateIn(name, right) => {
                let key = self.reg();
                try self.emitPrivateName(name, key);
                let obj = self.reg();
                try self.compileExpr(right, obj);
                let _ = self.emit(.Binary(op: .In, dst: dst, a: key, b: obj));
            },
            .Assign(op, target, value) => try self.compileAssign(op, target, value, dst),
            .Conditional(test, consequent, alternate) => {
                try self.compileExpr(test, dst);
                let toElse = self.emit(.JumpIf(cond: dst, target: 0, when: false));
                try self.compileExpr(consequent, dst);
                let over = self.emit(.Jump(target: 0));
                self.patchHere(toElse);
                try self.compileExpr(alternate, dst);
                self.patchHere(over);
            },
            .Call(callee, arguments, optional) => try self.compileCall(callee, arguments, optional, dst),
            .New(callee, arguments) => {
                let c = self.reg();
                try self.compileExpr(callee, c);
                let args = self.reg();
                try self.compileArgumentArray(arguments, args);
                let _ = self.emit(.New(dst: dst, callee: c, args: args));
            },
            .Member(object, property, optional) => try self.compileMember(object, property, optional, dst),
            .Chain(inner) => {
                let f = self.fnIndex();
                self.fns(f).chainJumps.append([]);
                try self.compileExpr(inner, dst);
                let jumps = self.fns(f).chainJumps.pop() ?? [];
                let over = self.emit(.Jump(target: 0));
                for j in jumps { self.patchHere(j); }
                self.load(dst, Value.undefined);
                self.patchHere(over);
            },
            .Sequence(items) => {
                for x in items { try self.compileExpr(x, dst); }
            },
            .Yield(argument, delegate) => try self.compileYield(argument, delegate, dst),
            .Await(argument) => try self.compileAwait(argument, dst),
            .MetaProperty(meta, _) => {
                if meta != "new" { throw compileError("import.meta isn't supported yet"); }
                let _ = self.emit(.NewTarget(dst: dst));
            },
            .Import(_, _) => throw compileError("dynamic import isn't supported yet")
        }
        .Ok(())
    }

    /// Records a jump to the end of the innermost optional chain, taken
    /// when `src` is null or undefined.
    mutating func emitChainCheck(src: Int64) {
        let f = self.fnIndex();
        let j = self.emit(.JumpIfNullish(src: src, target: 0));
        let top = self.fns(f).chainJumps.count - 1;
        if top >= 0 { self.fns(f).chainJumps(top).append(j); }
    }

    mutating func compileMember(object: Expr, property: MemberProperty, optional: Bool, dst: Int64) -> Result[(), SyntaxError] {
        if let .Super = object.kind {
            let key = self.reg();
            try self.compileMemberKey(property, key);
            let this = self.reg();
            self.emitGetName("this", this, false);
            let _ = self.emit(.SuperGet(dst: dst, key: key, this: this));
            return .Ok(());
        }
        let obj = self.reg();
        try self.compileExpr(object, obj);
        if optional { self.emitChainCheck(obj); }
        match property {
            .Name(n) => {
                let ic = self.newIC();
                let _ = self.emit(.GetProp(dst: dst, obj: obj, key: self.atom(n), ic: ic));
            },
            _ => {
                let key = self.reg();
                try self.compileMemberKey(property, key);
                let _ = self.emit(.GetElem(dst: dst, obj: obj, key: key));
            }
        }
        .Ok(())
    }

    mutating func compileCall(callee: Expr, arguments: Array[Argument], optional: Bool, dst: Int64) -> Result[(), SyntaxError] {
        if let .Super = callee.kind {
            let args = self.reg();
            try self.compileArgumentArray(arguments, args);
            let r = self.resolve("this");
            let _ = self.emit(.SuperCall(dst: dst, args: args, hops: r.hops, slot: r.slot));
            if self.fns(self.fnIndex()).hasFields {
                let _ = self.emit(.InitFields(this: dst));
            }
            return .Ok(());
        }
        let f = self.reg();
        let this = self.reg();
        // A method call passes the object as `this`.
        var isMethod = false;
        if let .Member(object, property, opt) = callee.kind {
            isMethod = true;
            if let .Super = object.kind {
                self.emitGetName("this", this, false);
                let key = self.reg();
                try self.compileMemberKey(property, key);
                let _ = self.emit(.SuperGet(dst: f, key: key, this: this));
            } else {
                try self.compileExpr(object, this);
                if opt { self.emitChainCheck(this); }
                match property {
                    .Name(n) => {
                        let ic = self.newIC();
                        let _ = self.emit(.GetProp(dst: f, obj: this, key: self.atom(n), ic: ic));
                    },
                    _ => {
                        let key = self.reg();
                        try self.compileMemberKey(property, key);
                        let _ = self.emit(.GetElem(dst: f, obj: this, key: key));
                    }
                }
            }
        }
        if not isMethod {
            self.load(this, Value.undefined);
            // A name found on a `with` object calls with that object as `this`.
            if let .Identifier(n) = callee.kind {
                self.emitGetNameThis(n, f, this, false);
            } else {
                try self.compileExpr(callee, f);
            }
        }
        if optional { self.emitChainCheck(f); }
        if hasSpread(arguments) {
            let args = self.reg();
            try self.compileArgumentArray(arguments, args);
            let _ = self.emit(.CallSpread(dst: dst, callee: f, this: this, args: args));
            return .Ok(());
        }
        let args = self.regs(arguments.count);
        for i in 0..<arguments.count {
            if let .Item(x) = arguments(i) { try self.compileExpr(x, args + i); }
        }
        if isDirectEvalCallee(callee) and not optional and not isMethod {
            let ctx = self.snapshotEvalContext();
            let _ = self.emit(.DirectEval(dst: dst, callee: f, this: this, args: args, argc: arguments.count, ctx: ctx));
            return .Ok(());
        }
        let _ = self.emit(.Call(dst: dst, callee: f, this: this, args: args, argc: arguments.count));
        .Ok(())
    }

    /// Arguments as an array, for spreads, `new` and `super`.
    mutating func compileArgumentArray(arguments: Array[Argument], dst: Int64) -> Result[(), SyntaxError] {
        let _ = self.emit(.NewArray(dst: dst));
        let t = self.reg();
        for a in arguments {
            match a {
                .Item(x) => {
                    try self.compileExpr(x, t);
                    let _ = self.emit(.ArrayAppend(arr: dst, src: t));
                },
                .Spread(x) => {
                    try self.compileExpr(x, t);
                    let _ = self.emit(.ArraySpread(arr: dst, src: t));
                }
            }
        }
        .Ok(())
    }

    mutating func compileUnary(op: String, argument: Expr, dst: Int64) -> Result[(), SyntaxError] {
        if op == "typeof" {
            if let .Identifier(n) = argument.kind {
                self.emitGetName(n, dst, true);
            } else {
                try self.compileExpr(argument, dst);
            }
            let _ = self.emit(.Unary(op: .TypeOf, dst: dst, src: dst));
            return .Ok(());
        }
        if op == "delete" { return self.compileDelete(argument, dst); }
        try self.compileExpr(argument, dst);
        if op == "void" {
            self.load(dst, Value.undefined);
            return .Ok(());
        }
        let u = match op {
            "-" => UnOp.Neg,
            "+" => UnOp.Plus,
            "!" => UnOp.Not,
            _ => UnOp.BitNot
        };
        let _ = self.emit(.Unary(op: u, dst: dst, src: dst));
        .Ok(())
    }

    mutating func compileDelete(argument: Expr, dst: Int64) -> Result[(), SyntaxError] {
        match argument.kind {
            .Member(object, property, optional) => {
                if let .Super = object.kind {
                    let _ = self.emit(.ThrowError(kind: .ReferenceError, message: "can't delete a super property"));
                    return .Ok(());
                }
                let obj = self.reg();
                try self.compileExpr(object, obj);
                if optional { self.emitChainCheck(obj); }
                let key = self.reg();
                try self.compileMemberKey(property, key);
                let _ = self.emit(.DeleteElem(dst: dst, obj: obj, key: key, strict: self.strict()));
            },
            .Chain(inner) => {
                let f = self.fnIndex();
                self.fns(f).chainJumps.append([]);
                try self.compileDelete(inner, dst);
                let jumps = self.fns(f).chainJumps.pop() ?? [];
                let over = self.emit(.Jump(target: 0));
                for j in jumps { self.patchHere(j); }
                self.load(dst, Value.bool(true));
                self.patchHere(over);
            },
            .Identifier(n) => {
                // Declared bindings can't be deleted; global properties
                // and `with` object properties can.
                let r = self.resolve(n);
                let key = self.reg();
                self.loadString(key, n);
                let obj = self.reg();
                var jumps: [Int64] = [];
                for h in r.withHops {
                    jumps.append(self.emit(.WithGet(dst: -1, this: obj, hops: h.hops, slot: h.slot, name: self.atom(n), target: 0)));
                }
                if r.found {
                    self.load(dst, Value.bool(false));
                } else {
                    let global = self.reg();
                    self.emitGetName("globalThis", global, false);
                    let _ = self.emit(.DeleteElem(dst: dst, obj: global, key: key, strict: false));
                }
                if jumps.isEmpty { return .Ok(()); }
                let over = self.emit(.Jump(target: 0));
                for j in jumps { self.patchHere(j); }
                let _ = self.emit(.DeleteElem(dst: dst, obj: obj, key: key, strict: false));
                self.patchHere(over);
            },
            _ => {
                try self.compileExpr(argument, dst);
                self.load(dst, Value.bool(true));
            }
        }
        .Ok(())
    }

    /// `x++`, `--x`, `a.b++`.
    mutating func compileUpdate(op: String, prefix: Bool, argument: Expr, dst: Int64) -> Result[(), SyntaxError] {
        let u = if op == "++" { UnOp.Inc } else { UnOp.Dec };
        let newValue = self.reg();
        let ref = try self.compileReference(argument);
        try self.readReference(ref, dst);
        let _ = self.emit(.Unary(op: .ToNumeric, dst: dst, src: dst));
        let _ = self.emit(.Unary(op: u, dst: newValue, src: dst));
        try self.writeReference(ref, newValue);
        if prefix { let _ = self.emit(.Move(dst: dst, src: newValue)); }
        .Ok(())
    }

    mutating func compileAssign(op: String, target: Pattern, value: Expr, dst: Int64) -> Result[(), SyntaxError] {
        if op == "=" {
            match target {
                .Identifier(n) => {
                    try self.compileNamed(value, dst, n);
                    self.emitSetName(n, dst, .Assign);
                },
                .Expression(e) => {
                    let ref = try self.compileReference(e);
                    try self.compileExpr(value, dst);
                    try self.writeReference(ref, dst);
                },
                _ => {
                    try self.compileExpr(value, dst);
                    try self.bindPattern(target, dst, .Assign);
                }
            }
            return .Ok(());
        }
        let e = match target {
            .Expression(x) => x,
            .Identifier(n) => Expr(kind: .Identifier(n), span: Span(start: 0, end: 0), parenthesized: false),
            _ => throw compileError("invalid compound assignment target")
        };
        let ref = try self.compileReference(e);
        try self.readReference(ref, dst);
        if op == "&&=" or op == "||=" or op == "??=" {
            let skip = if op == "??=" {
                let toAssign = self.emit(.JumpIfNullish(src: dst, target: 0));
                let over = self.emit(.Jump(target: 0));
                self.patchHere(toAssign);
                over
            } else {
                self.emit(.JumpIf(cond: dst, target: 0, when: op == "||="))
            };
            let name = match e.kind { .Identifier(n) => n, _ => "" };
            try self.compileNamed(value, dst, name);
            try self.writeReference(ref, dst);
            self.patchHere(skip);
            return .Ok(());
        }
        let r = self.reg();
        try self.compileExpr(value, r);
        let _ = self.emit(.Binary(op: compoundOp(op), dst: dst, a: dst, b: r));
        try self.writeReference(ref, dst);
        .Ok(())
    }

    /// `yield x` and `yield* iterable`.
    mutating func compileYield(argument: Optional[Expr], delegate: Bool, dst: Int64) -> Result[(), SyntaxError] {
        let mark = self.regMark();
        let v = self.reg();
        match argument {
            .Some(e) => try self.compileExpr(e, v),
            .None => self.load(v, Value.undefined)
        }
        let isAsync = self.isAsyncGenerator();
        if delegate {
            if isAsync { try self.compileAsyncYieldStar(v, dst); } else { try self.compileYieldStar(v, dst); }
            self.freeRegs(to: mark);
            return .Ok(());
        }
        let sent = self.regs(2);
        // An async generator awaits what it yields.
        if isAsync { self.emitAwait(v, v); }
        let _ = self.emit(.Yield(dst: sent, src: v, raw: false));
        if isAsync { try self.emitAsyncGenResume(sent); } else { try self.emitResumeModes(sent); }
        let _ = self.emit(.Move(dst: dst, src: sent));
        self.freeRegs(to: mark);
        .Ok(())
    }

    /// `await x`: a raw yield of `x` that `asyncStep` turns into waiting on
    /// a promise; it resumes with the value, or throws the rejection.
    mutating func compileAwait(argument: Expr, dst: Int64) -> Result[(), SyntaxError] {
        let mark = self.regMark();
        let v = self.reg();
        try self.compileExpr(argument, v);
        self.emitAwait(v, dst);
        self.freeRegs(to: mark);
        .Ok(())
    }

    func isAsyncGenerator() -> Bool {
        let f = self.fns(self.fnIndex());
        f.isAsync and f.isGenerator
    }

    /// Awaits the value in `src` into `dst`: a raw yield `asyncStep`
    /// turns into waiting on a promise, resumed with the value or the
    /// rejection to throw.
    mutating func emitAwait(src: Int64, dst: Int64) {
        let mark = self.regMark();
        let sent = self.regs(2);
        let _ = self.emit(.Yield(dst: sent, src: src, raw: true));
        let t = self.reg();
        let c = self.reg();
        self.load(c, Value.int(1));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: sent + 1, b: c));
        let notThrow = self.emit(.JumpIf(cond: t, target: 0, when: false));
        let _ = self.emit(.Throw(src: sent));
        self.patchHere(notThrow);
        let _ = self.emit(.Move(dst: dst, src: sent));
        self.freeRegs(to: mark);
    }

    /// After a yield in an async generator: throw() rethrows; return()
    /// awaits its value and returns it.
    mutating func emitAsyncGenResume(sent: Int64) -> Result[(), SyntaxError] {
        let t = self.reg();
        let c = self.reg();
        self.load(c, Value.int(1));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: sent + 1, b: c));
        let notThrow = self.emit(.JumpIf(cond: t, target: 0, when: false));
        let _ = self.emit(.Throw(src: sent));
        self.patchHere(notThrow);
        self.load(c, Value.int(2));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: sent + 1, b: c));
        let normal = self.emit(.JumpIf(cond: t, target: 0, when: false));
        self.emitAwait(sent, sent);
        try self.emitCleanup(-1);
        let _ = self.emit(.Return(src: sent));
        self.patchHere(normal);
        .Ok(())
    }

    /// `yield*` in an async generator: forwards next/throw/return to an
    /// async iterator, awaiting each result, and yields the values.
    mutating func compileAsyncYieldStar(src: Int64, dst: Int64) -> Result[(), SyntaxError] {
        let iter = self.regs(2);
        let _ = self.emit(.GetAsyncIterator(iter: iter, obj: src));
        let recv = self.regs(2);
        self.load(recv, Value.undefined);
        self.load(recv + 1, Value.int(0));
        let res = self.reg();
        let t = self.reg();
        let c = self.reg();
        let m = self.reg();
        let argReg = self.reg();
        let done = self.reg();
        let top = self.here();
        self.load(c, Value.int(1));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: recv + 1, b: c));
        let toThrow = self.emit(.JumpIf(cond: t, target: 0, when: true));
        self.load(c, Value.int(2));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: recv + 1, b: c));
        let toReturn = self.emit(.JumpIf(cond: t, target: 0, when: true));
        // next(v)
        let _ = self.emit(.Move(dst: argReg, src: recv));
        let _ = self.emit(.Call(dst: res, callee: iter + 1, this: iter, args: argReg, argc: 1));
        let afterNext = self.emit(.Jump(target: 0));
        // throw(v): forward it, or close the iterator and fail.
        self.patchHere(toThrow);
        let _ = self.emit(.GetProp(dst: m, obj: iter, key: self.atom("throw"), ic: self.newIC()));
        let noThrow = self.emit(.JumpIfNullish(src: m, target: 0));
        let _ = self.emit(.Move(dst: argReg, src: recv));
        let _ = self.emit(.Call(dst: res, callee: m, this: iter, args: argReg, argc: 1));
        let afterThrow = self.emit(.Jump(target: 0));
        self.patchHere(noThrow);
        let _ = self.emit(.GetProp(dst: m, obj: iter, key: self.names.returnKey, ic: self.newIC()));
        let noClose = self.emit(.JumpIfNullish(src: m, target: 0));
        let _ = self.emit(.Call(dst: res, callee: m, this: iter, args: res, argc: 0));
        self.emitAwait(res, res);
        let _ = self.emit(.RequireObjectResult(src: res));
        self.patchHere(noClose);
        let _ = self.emit(.ThrowError(kind: .TypeError, message: "The iterator does not provide a 'throw' method"));
        // return(v): forward it; a done result returns from here.
        self.patchHere(toReturn);
        let _ = self.emit(.GetProp(dst: m, obj: iter, key: self.names.returnKey, ic: self.newIC()));
        let noReturn = self.emit(.JumpIfNullish(src: m, target: 0));
        let _ = self.emit(.Move(dst: argReg, src: recv));
        let _ = self.emit(.Call(dst: res, callee: m, this: iter, args: argReg, argc: 1));
        self.emitAwait(res, res);
        let _ = self.emit(.RequireObjectResult(src: res));
        let _ = self.emit(.GetProp(dst: done, obj: res, key: self.names.done, ic: self.newIC()));
        let returnNotDone = self.emit(.JumpIf(cond: done, target: 0, when: false));
        let _ = self.emit(.GetProp(dst: done, obj: res, key: self.names.value, ic: self.newIC()));
        self.emitAwait(done, done);
        try self.emitCleanup(-1);
        let _ = self.emit(.Return(src: done));
        self.patchHere(noReturn);
        self.emitAwait(recv, recv);
        try self.emitCleanup(-1);
        let _ = self.emit(.Return(src: recv));
        // A next/throw result: done ends the yield*, else yield its value.
        self.patchHere(afterNext);
        self.patchHere(afterThrow);
        self.emitAwait(res, res);
        let _ = self.emit(.RequireObjectResult(src: res));
        let _ = self.emit(.GetProp(dst: done, obj: res, key: self.names.done, ic: self.newIC()));
        let finished = self.emit(.JumpIf(cond: done, target: 0, when: true));
        self.patchHere(returnNotDone);
        let _ = self.emit(.GetProp(dst: m, obj: res, key: self.names.value, ic: self.newIC()));
        let _ = self.emit(.Yield(dst: recv, src: m, raw: false));
        // A return() request's value is awaited before it's forwarded.
        self.load(c, Value.int(2));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: recv + 1, b: c));
        let notReturn = self.emit(.JumpIf(cond: t, target: 0, when: false));
        self.emitAwait(recv, recv);
        self.patchHere(notReturn);
        let _ = self.emit(.Jump(target: top));
        self.patchHere(finished);
        let _ = self.emit(.GetProp(dst: dst, obj: res, key: self.names.value, ic: self.newIC()));
        .Ok(())
    }

    /// After a yield: rethrows for throw(), returns (running finally
    /// blocks) for return(), and falls through for next().
    mutating func emitResumeModes(sent: Int64) -> Result[(), SyntaxError] {
        let t = self.reg();
        let c = self.reg();
        self.load(c, Value.int(1));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: sent + 1, b: c));
        let notThrow = self.emit(.JumpIf(cond: t, target: 0, when: false));
        let _ = self.emit(.Throw(src: sent));
        self.patchHere(notThrow);
        self.load(c, Value.int(2));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: sent + 1, b: c));
        let normal = self.emit(.JumpIf(cond: t, target: 0, when: false));
        try self.emitCleanup(-1);
        let _ = self.emit(.Return(src: sent));
        self.patchHere(normal);
        .Ok(())
    }

    /// `yield*`: forwards next/throw/return to the inner iterator and
    /// yields its results as they are (ECMA-262 §15.5.5).
    mutating func compileYieldStar(src: Int64, dst: Int64) -> Result[(), SyntaxError] {
        let iter = self.regs(2);
        let _ = self.emit(.GetIterator(iter: iter, obj: src));
        let recv = self.regs(2);
        self.load(recv, Value.undefined);
        self.load(recv + 1, Value.int(0));
        let res = self.reg();
        let t = self.reg();
        let c = self.reg();
        let m = self.reg();
        let argReg = self.reg();
        let done = self.reg();
        let top = self.here();
        self.load(c, Value.int(1));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: recv + 1, b: c));
        let toThrow = self.emit(.JumpIf(cond: t, target: 0, when: true));
        self.load(c, Value.int(2));
        let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: recv + 1, b: c));
        let toReturn = self.emit(.JumpIf(cond: t, target: 0, when: true));
        // next(v)
        let _ = self.emit(.Move(dst: argReg, src: recv));
        let _ = self.emit(.Call(dst: res, callee: iter + 1, this: iter, args: argReg, argc: 1));
        let afterNext = self.emit(.Jump(target: 0));
        // throw(v): forward it, or close the iterator and fail.
        self.patchHere(toThrow);
        let _ = self.emit(.GetProp(dst: m, obj: iter, key: self.atom("throw"), ic: self.newIC()));
        let noThrow = self.emit(.JumpIfNullish(src: m, target: 0));
        let _ = self.emit(.Move(dst: argReg, src: recv));
        let _ = self.emit(.Call(dst: res, callee: m, this: iter, args: argReg, argc: 1));
        let afterThrow = self.emit(.Jump(target: 0));
        self.patchHere(noThrow);
        let _ = self.emit(.IteratorClose(iter: iter));
        let _ = self.emit(.ThrowError(kind: .TypeError, message: "The iterator does not provide a 'throw' method"));
        // return(v): forward it; a done result returns from here.
        self.patchHere(toReturn);
        let _ = self.emit(.GetProp(dst: m, obj: iter, key: self.atom("return"), ic: self.newIC()));
        let noReturn = self.emit(.JumpIfNullish(src: m, target: 0));
        let _ = self.emit(.Move(dst: argReg, src: recv));
        let _ = self.emit(.Call(dst: res, callee: m, this: iter, args: argReg, argc: 1));
        let _ = self.emit(.RequireObjectResult(src: res));
        let _ = self.emit(.GetProp(dst: done, obj: res, key: self.names.done, ic: self.newIC()));
        let returnNotDone = self.emit(.JumpIf(cond: done, target: 0, when: false));
        let _ = self.emit(.GetProp(dst: done, obj: res, key: self.names.value, ic: self.newIC()));
        try self.emitCleanup(-1);
        let _ = self.emit(.Return(src: done));
        self.patchHere(noReturn);
        try self.emitCleanup(-1);
        let _ = self.emit(.Return(src: recv));
        // A next/throw result: done ends the yield*, else yield it as is.
        self.patchHere(afterNext);
        self.patchHere(afterThrow);
        let _ = self.emit(.RequireObjectResult(src: res));
        let _ = self.emit(.GetProp(dst: done, obj: res, key: self.names.done, ic: self.newIC()));
        let finished = self.emit(.JumpIf(cond: done, target: 0, when: true));
        self.patchHere(returnNotDone);
        let _ = self.emit(.Yield(dst: recv, src: res, raw: true));
        let _ = self.emit(.Jump(target: top));
        self.patchHere(finished);
        let _ = self.emit(.GetProp(dst: dst, obj: res, key: self.names.value, ic: self.newIC()));
        .Ok(())
    }

    /// Evaluates the parts of an assignable expression (object and key)
    /// once, for reading and writing it.
    mutating func compileReference(e: Expr) -> Result[Reference, SyntaxError] {
        match e.kind {
            .Identifier(n) => return .Ok(self.compileNameReference(n)),
            .Member(object, property, _) => {
                let key = self.reg();
                let computed = match property { .Computed(_) => true, _ => false };
                if let .Super = object.kind {
                    let this = self.reg();
                    self.emitGetName("this", this, false);
                    try self.compileMemberKey(property, key);
                    return .Ok(Reference(name: "", obj: this, key: key, isSuper: true, dyn: -1, computed: false));
                }
                let obj = self.reg();
                try self.compileExpr(object, obj);
                try self.compileMemberKey(property, key);
                return .Ok(Reference(name: "", obj: obj, key: key, isSuper: false, dyn: -1, computed: computed));
            },
            _ => throw compileError("invalid assignment target")
        }
    }

    /// A name, resolved once: if a `with` (or eval-vars) object holds it
    /// now, the read and the write both use that object, even if the read
    /// removes the property.
    mutating func compileNameReference(n: String) -> Reference {
        let r = self.resolve(n);
        if r.withHops.isEmpty { return Reference(name: n, obj: -1, key: -1, isSuper: false, dyn: -1, computed: false); }
        let dyn = self.reg();
        self.load(dyn, Value.undefined);
        var jumps: [Int64] = [];
        for h in r.withHops {
            jumps.append(self.emit(.WithGet(dst: -1, this: dyn, hops: h.hops, slot: h.slot, name: self.atom(n), target: 0)));
        }
        for j in jumps { self.patchHere(j); }
        let key = self.reg();
        self.loadString(key, n);
        Reference(name: n, obj: -1, key: key, isSuper: false, dyn: dyn, computed: false)
    }

    mutating func readReference(r: Reference, dst: Int64) -> Result[(), SyntaxError] {
        if r.dyn >= 0 {
            let isStatic = self.emit(.JumpIfUndefined(src: r.dyn, target: 0));
            let _ = self.emit(.GetElem(dst: dst, obj: r.dyn, key: r.key));
            let over = self.emit(.Jump(target: 0));
            self.patchHere(isStatic);
            self.emitGetResolved(r.name, self.resolve(r.name), dst, false);
            self.patchHere(over);
        } else if r.obj < 0 {
            self.emitGetName(r.name, dst, false);
        } else if r.isSuper {
            let _ = self.emit(.SuperGet(dst: dst, key: r.key, this: r.obj));
        } else {
            if r.computed {
                // The base's null check comes before the key's conversion.
                let _ = self.emit(.Unary(op: .RequireObject, dst: dst, src: r.obj));
                let _ = self.emit(.Unary(op: .ToPropertyKey, dst: r.key, src: r.key));
            }
            let _ = self.emit(.GetElem(dst: dst, obj: r.obj, key: r.key));
        }
        .Ok(())
    }

    mutating func writeReference(r: Reference, src: Int64) -> Result[(), SyntaxError] {
        if r.dyn >= 0 {
            let isStatic = self.emit(.JumpIfUndefined(src: r.dyn, target: 0));
            let _ = self.emit(.SetBinding(obj: r.dyn, key: r.key, src: src, strict: self.strict()));
            let over = self.emit(.Jump(target: 0));
            self.patchHere(isStatic);
            self.emitSetResolved(r.name, self.resolve(r.name), src, .Assign);
            self.patchHere(over);
        } else if r.obj < 0 {
            self.emitSetName(r.name, src, .Assign);
        } else if r.isSuper {
            let _ = self.emit(.SuperSet(key: r.key, src: src, this: r.obj));
        } else {
            let _ = self.emit(.SetElem(obj: r.obj, key: r.key, src: src, strict: self.strict()));
        }
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Literals
    // ------------------------------------------------------------------

    mutating func compileTemplate(t: TemplateLiteral, dst: Int64) -> Result[(), SyntaxError] {
        self.loadString(dst, t.quasis(0).cooked ?? "");
        let r = self.reg();
        for i in 0..<t.expressions.count {
            try self.compileExpr(t.expressions(i), r);
            let _ = self.emit(.Unary(op: .ToStr, dst: r, src: r));
            let _ = self.emit(.Binary(op: .Add, dst: dst, a: dst, b: r));
            self.loadString(r, t.quasis(i + 1).cooked ?? "");
            let _ = self.emit(.Binary(op: .Add, dst: dst, a: dst, b: r));
        }
        .Ok(())
    }

    mutating func compileTaggedTemplate(tag: Expr, quasi: TemplateLiteral, dst: Int64) -> Result[(), SyntaxError] {
        var args: [Argument] = [];
        for x in quasi.expressions { args.append(.Item(x)); }
        // The strings array goes first; build it, then call like f(strings, ...subs).
        let f = self.reg();
        let this = self.reg();
        if let .Member(object, property, _) = tag.kind {
            try self.compileExpr(object, this);
            let key = self.reg();
            try self.compileMemberKey(property, key);
            let _ = self.emit(.GetElem(dst: f, obj: this, key: key));
        } else {
            try self.compileExpr(tag, f);
            self.load(this, Value.undefined);
        }
        let argv = self.regs(quasi.expressions.count + 1);
        let strings = argv;
        let raw = self.reg();
        let t = self.reg();
        let _ = self.emit(.NewArray(dst: strings));
        let _ = self.emit(.NewArray(dst: raw));
        for q in quasi.quasis {
            match q.cooked {
                .Some(c) => self.loadString(t, c),
                .None => self.load(t, Value.undefined)
            }
            let _ = self.emit(.ArrayAppend(arr: strings, src: t));
            self.loadString(t, q.raw);
            let _ = self.emit(.ArrayAppend(arr: raw, src: t));
        }
        let ic = self.newIC();
        let _ = self.emit(.SetProp(obj: strings, key: self.atom("raw"), src: raw, ic: ic, strict: true));
        for i in 0..<quasi.expressions.count {
            try self.compileExpr(quasi.expressions(i), argv + 1 + i);
        }
        let _ = self.emit(.Call(dst: dst, callee: f, this: this, args: argv, argc: quasi.expressions.count + 1));
        .Ok(())
    }

    mutating func compileObjectLiteral(props: Array[Property], dst: Int64) -> Result[(), SyntaxError] {
        let _ = self.emit(.NewObject(dst: dst));
        for p in props {
            let mark = self.regMark();
            match p {
                .Init(key, value, shorthand) => {
                    let name = self.keyName(key);
                    let computed = match key { .Computed(_) => true, _ => false };
                    if name == "__proto__" and not computed and not shorthand {
                        let v = self.reg();
                        try self.compileExpr(value, v);
                        let _ = self.emit(.SetProto(obj: dst, proto: v));
                    } else {
                        let k = self.reg();
                        try self.compileKey(key, k);
                        let v = self.reg();
                        try self.compileNamed(value, v, name);
                        let _ = self.emit(.DefineProp(obj: dst, key: k, src: v));
                    }
                },
                .Method(key, kind, function) => {
                    let k = self.reg();
                    try self.compileKey(key, k);
                    let fnReg = self.reg();
                    try self.compileMethod(function, kind, self.keyName(key), dst, fnReg);
                    if kind == MethodKind.Get or kind == MethodKind.Set {
                        let _ = self.emit(.DefineAccessor(obj: dst, key: k, fn: fnReg, isGetter: kind == MethodKind.Get, enumerable: true));
                    } else {
                        let _ = self.emit(.DefineMethod(obj: dst, key: k, fn: fnReg, enumerable: true));
                    }
                },
                .Spread(x) => {
                    let s = self.reg();
                    try self.compileExpr(x, s);
                    let _ = self.emit(.CopyDataProps(dst: dst, src: s, excluded: 0, count: 0));
                }
            }
            self.freeRegs(to: mark);
        }
        .Ok(())
    }

    /// A method's closure into `dst`, with `home` as its home object.
    mutating func compileMethod(f: Function, kind: MethodKind, name: String, home: Int64, dst: Int64) -> Result[(), SyntaxError] {
        var flags = plainFlags();
        flags.isMethod = true;
        let prefix = if kind == MethodKind.Get { "get " } else if kind == MethodKind.Set { "set " } else { "" };
        let code = try self.compileFunction(f, "\(prefix)\(name)", flags);
        let _ = self.emit(.MakeClosure(dst: dst, code: code, home: home));
        .Ok(())
    }

    mutating func compileFunctionExpr(f: Function, dst: Int64, name: String, isArrow: Bool) -> Result[(), SyntaxError] {
        var flags = plainFlags();
        flags.isArrow = isArrow;
        // A named function expression can refer to itself by name.
        if let .Some(own) = f.id {
            if not isArrow {
                var sc = newCScope(false, false);
                let _ = sc.declare(own);
                sc.isFunctionName = true;
                self.pushScope(sc);
                let _ = self.emit(.PushEnv(size: 1));
                let code = try self.compileFunction(f, name, flags);
                let _ = self.emit(.MakeClosure(dst: dst, code: code, home: -1));
                let _ = self.emit(.InitVar(hops: 0, slot: 0, src: dst));
                self.exitBlockScope(true);
                return .Ok(());
            }
        }
        let code = try self.compileFunction(f, name, flags);
        let _ = self.emit(.MakeClosure(dst: dst, code: code, home: -1));
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Classes
    // ------------------------------------------------------------------

    mutating func compileClass(c: Class, dst: Int64, name: String) -> Result[(), SyntaxError] {
        let f = self.fnIndex();
        let savedStrict = self.fns(f).strict;
        self.fns(f).strict = true;
        // The class scope holds the class's own name, its private names
        // (fresh symbols each evaluation), and the instance private methods
        // the field initializer installs on each instance.
        var pushed = false;
        if let .Some(own) = c.id {
            var sc = newCScope(false, false);
            let _ = sc.declare(own);
            let _ = sc.consts.insert(own);
            self.pushScope(sc);
            let _ = self.emit(.PushEnv(size: 1));
            let t = self.reg();
            self.load(t, Value.empty);
            let _ = self.emit(.InitVar(hops: 0, slot: 0, src: t));
            pushed = true;
        }
        // The heritage sees the class's name but not its private names.
        var superclass: Int64 = -1;
        if let .Some(s) = c.superClass {
            superclass = self.reg();
            try self.compileExpr(s, superclass);
        }
        // An inner scope holds the private names (fresh symbols each
        // evaluation) and the instance private methods the field
        // initializer installs on each instance.
        let privates = try classPrivateNames(c);
        let privateNames = privates.names;
        let methodNames = privates.methodNames;
        let methodKinds = privates.methodKinds;
        let pushedPrivate = not privateNames.isEmpty;
        if pushedPrivate {
            var sc = newCScope(false, false);
            for n in privateNames { let _ = sc.declare(n); }
            for i in 0..<methodNames.count { let _ = sc.declare(privateMethodSlot(methodNames(i), methodKinds(i))); }
            let size = sc.size;
            self.pushScope(sc);
            let _ = self.emit(.PushEnv(size: size));
            let t = self.reg();
            for n in privateNames {
                let _ = self.emit(.NewPrivateName(dst: t, name: self.atom(n)));
                self.emitSetName(n, t, .Declare);
            }
        }
        let derived = superclass >= 0;

        // The constructor, written or default.
        var instanceFields: [ClassMember] = [];
        var ctorFn: Optional[Function] = .None;
        for m in c.body {
            match m {
                .Method(_, kind, _, function) => if kind == MethodKind.Constructor { ctorFn = .Some(function); },
                .Field(_, isStatic, _) => if not isStatic { instanceFields.append(m); },
                _ => {}
            }
        }
        let ctorFunction = ctorFn ?? defaultConstructor(derived);
        var flags = plainFlags();
        flags.isClassConstructor = true;
        flags.isDerived = derived;
        flags.hasFields = not instanceFields.isEmpty or not methodNames.isEmpty;
        let ctorCode = try self.compileFunction(ctorFunction, name, flags);
        let _ = self.emit(.MakeClosure(dst: dst, code: ctorCode, home: -1));
        let proto = self.reg();
        let _ = self.emit(.MakeClass(ctor: dst, proto: proto, superclass: superclass));

        // Methods and accessors.
        for m in c.body {
            let mark = self.regMark();
            match m {
                .Method(key, kind, isStatic, function) => if kind != MethodKind.Constructor {
                    let home = if isStatic { dst } else { proto };
                    if let .Private(n) = key {
                        try self.compilePrivateMethod(n, kind, isStatic, function, home, dst);
                        self.freeRegs(to: mark);
                        continue;
                    }
                    let k = self.reg();
                    try self.compileKey(key, k);
                    let fnReg = self.reg();
                    try self.compileMethod(function, kind, self.keyName(key), home, fnReg);
                    if kind == MethodKind.Get or kind == MethodKind.Set {
                        let _ = self.emit(.DefineAccessor(obj: home, key: k, fn: fnReg, isGetter: kind == MethodKind.Get, enumerable: false));
                    } else {
                        let _ = self.emit(.DefineMethod(obj: home, key: k, fn: fnReg, enumerable: false));
                    }
                },
                _ => {}
            }
            self.freeRegs(to: mark);
        }

        // Instance fields run in an initializer the constructor calls.
        if not instanceFields.isEmpty or not methodNames.isEmpty {
            let code = try self.compileFieldInitializer(instanceFields, methodNames, methodKinds);
            let fi = self.reg();
            let _ = self.emit(.MakeClosure(dst: fi, code: code, home: proto));
            let _ = self.emit(.SetClassFields(ctor: dst, fn: fi));
        }

        if let .Some(own) = c.id { self.emitSetName(own, dst, .Declare); }

        // Static fields and blocks run now, in order, with `this` the class.
        for m in c.body {
            let mark = self.regMark();
            match m {
                .Field(_, isStatic, _) => if isStatic {
                    let code = try self.compileFieldInitializer([m], [], []);
                    try self.emitRunWithThis(code, dst);
                },
                .StaticBlock(body) => {
                    let fun = Function(id: .None, params: [], body: .Block(body), isAsync: false, isGenerator: false, hasDirectEval: false);
                    var mf = plainFlags();
                    mf.isMethod = true;
                    let code = try self.compileFunction(fun, "", mf);
                    try self.emitRunWithThis(code, dst);
                },
                _ => {}
            }
            self.freeRegs(to: mark);
        }
        self.exitBlockScope(pushedPrivate);
        self.exitBlockScope(pushed);
        self.fns(f).strict = savedStrict;
        .Ok(())
    }

    /// Calls `code` as a method of the object in `this`.
    mutating func emitRunWithThis(code: Int64, this: Int64) -> Result[(), SyntaxError] {
        let fnReg = self.reg();
        let _ = self.emit(.MakeClosure(dst: fnReg, code: code, home: this));
        let out = self.reg();
        let _ = self.emit(.Call(dst: out, callee: fnReg, this: this, args: out, argc: 0));
        .Ok(())
    }

    /// A private method or accessor `#n`: a static one goes on the class
    /// now; an instance one waits in its class-scope slot for the field
    /// initializer.
    mutating func compilePrivateMethod(n: String, kind: MethodKind, isStatic: Bool, function: Function, home: Int64, ctor: Int64) -> Result[(), SyntaxError] {
        let fnReg = self.reg();
        try self.compileMethod(function, kind, "#" + n, home, fnReg);
        let pk = privateMethodKind(kind);
        if not isStatic {
            self.emitSetName(privateMethodSlot("#" + n, pk), fnReg, .Declare);
            return .Ok(());
        }
        let k = self.reg();
        self.emitGetName("#" + n, k, false);
        let _ = self.emit(.PrivateMethod(obj: ctor, key: k, fn: fnReg, kind: pk));
        .Ok(())
    }

    /// A method that installs the class's instance private methods
    /// (`methodNames`/`methodKinds`), then defines `fields`, on `this`.
    mutating func compileFieldInitializer(fields: Array[ClassMember], methodNames: Array[String], methodKinds: Array[Int64]) -> Result[Int64, SyntaxError] {
        self.beginFunction(true, false, false);
        self.fns(self.fnIndex()).isMethod = true;
        self.fns(self.fnIndex()).isFieldInit = true;
        let this = self.reg();
        self.emitGetName("this", this, false);
        for i in 0..<methodNames.count {
            let mark = self.regMark();
            let k = self.reg();
            self.emitGetName(methodNames(i), k, false);
            let f = self.reg();
            self.emitGetName(privateMethodSlot(methodNames(i), methodKinds(i)), f, false);
            let _ = self.emit(.PrivateMethod(obj: this, key: k, fn: f, kind: methodKinds(i)));
            self.freeRegs(to: mark);
        }
        for m in fields {
            if let .Field(key, _, value) = m {
                let mark = self.regMark();
                let k = self.reg();
                try self.compileKey(key, k);
                let v = self.reg();
                match value {
                    .Some(x) => try self.compileNamed(x, v, self.keyName(key)),
                    .None => self.load(v, Value.undefined)
                }
                let _ = self.emit(.DefineProp(obj: this, key: k, src: v));
                self.freeRegs(to: mark);
            }
        }
        var flags = plainFlags();
        flags.isMethod = true;
        .Ok(self.endFunction("", 0, flags))
    }
}

/// An assignable place whose parts have been evaluated: a name, or an
/// object and key in registers.
struct Reference {
    var name: String
    var obj: Int64
    var key: Int64
    var isSuper: Bool
    /// For a name inside `with`: the register holding the object that
    /// supplies it, or undefined when it's an ordinary binding; else -1.
    var dyn: Int64
    /// A computed member key, converted by the first read so the write
    /// reuses the converted key.
    var computed: Bool
}

/// `constructor(...args) { super(...args); }` or `constructor() {}`.
func defaultConstructor(derived: Bool) -> Function {
    if not derived {
        return Function(id: .None, params: [], body: .Block([]), isAsync: false, isGenerator: false, hasDirectEval: false);
    }
    let noSpan = Span(start: 0, end: 0);
    let args = Expr(kind: .Identifier("args"), span: noSpan, parenthesized: false);
    let sup = Expr(kind: .Super, span: noSpan, parenthesized: false);
    let call = Expr(kind: .Call(callee: sup, arguments: [.Spread(args)], optional: false), span: noSpan, parenthesized: false);
    let body = [Stmt(kind: .Expression(call), span: noSpan)];
    Function(id: .None, params: [.Rest(.Identifier("args"))], body: .Block(body), isAsync: false, isGenerator: false, hasDirectEval: false)
}

func hasSpread(args: Array[Argument]) -> Bool {
    for a in args {
        if let .Spread(_) = a { return true; }
    }
    false
}

func binOp(op: String) -> BinOp {
    match op {
        "+" => BinOp.Add,
        "-" => BinOp.Sub,
        "*" => BinOp.Mul,
        "/" => BinOp.Div,
        "%" => BinOp.Mod,
        "**" => BinOp.Exp,
        "<<" => BinOp.Shl,
        ">>" => BinOp.Shr,
        ">>>" => BinOp.UShr,
        "&" => BinOp.BitAnd,
        "|" => BinOp.BitOr,
        "^" => BinOp.BitXor,
        "==" => BinOp.Eq,
        "!=" => BinOp.NotEq,
        "===" => BinOp.StrictEq,
        "!==" => BinOp.StrictNotEq,
        "<" => BinOp.Lt,
        ">" => BinOp.Gt,
        "<=" => BinOp.LtEq,
        ">=" => BinOp.GtEq,
        "in" => BinOp.In,
        _ => BinOp.InstanceOf
    }
}

/// Maps a compound assignment operator ("+=") to its binary operator.
func compoundOp(op: String) -> BinOp {
    match op {
        "+=" => BinOp.Add,
        "-=" => BinOp.Sub,
        "*=" => BinOp.Mul,
        "/=" => BinOp.Div,
        "%=" => BinOp.Mod,
        "**=" => BinOp.Exp,
        "<<=" => BinOp.Shl,
        ">>=" => BinOp.Shr,
        ">>>=" => BinOp.UShr,
        "&=" => BinOp.BitAnd,
        "|=" => BinOp.BitOr,
        _ => BinOp.BitXor
    }
}

/// `PrivateMethod`'s kind for a method: 0 method, 1 getter, 2 setter.
func privateMethodKind(kind: MethodKind) -> Int64 {
    match kind {
        .Get => 1,
        .Set => 2,
        _ => 0
    }
}

/// The class-scope binding holding instance private method `name`
/// (with its `#`) of `kind`, until the field initializer installs it.
func privateMethodSlot(name: String, kind: Int64) -> String {
    let suffix = if kind == 1 { " get" } else if kind == 2 { " set" } else { " fn" };
    name + suffix
}

/// A class's private names (with their `#`), and its instance private
/// methods with their `privateMethodKind`s.
struct ClassPrivates {
    var names: [String]
    var methodNames: [String]
    var methodKinds: [Int64]
}

/// Collects a class's private names. Declaring one twice is an early
/// error, except a getter and a setter that are both static or both not.
func classPrivateNames(c: Class) -> Result[ClassPrivates, SyntaxError] {
    var out = ClassPrivates(names: [], methodNames: [], methodKinds: []);
    // How each name was declared: 1 getter, 2 setter, 3 anything else;
    // plus 4 when static.
    var seen: [String: Int64] = [:];
    for m in c.body {
        var name = "";
        var how: Int64 = 3;
        match m {
            .Method(key, kind, isStatic, _) => if let .Private(n) = key {
                name = "#" + n;
                let k = privateMethodKind(kind);
                how = (if k == 0 { 3 } else { k }) + (if isStatic { 4 } else { 0 });
                if not isStatic {
                    out.methodNames.append(name);
                    out.methodKinds.append(k);
                }
            },
            .Field(key, isStatic, _) => if let .Private(n) = key {
                name = "#" + n;
                how = 3 + (if isStatic { 4 } else { 0 });
            },
            _ => {}
        }
        if name.isEmpty { continue; }
        if let .Some(prev) = seen(name) {
            // Only a getter/setter pair with the same staticness may share.
            let pair = (prev & 3) + (how & 3) == 3 and (prev & 4) == (how & 4);
            if not pair { throw compileError("Identifier '\(name)' has already been declared"); }
            let _ = seen.insert(name, 3 | (how & 4));
            continue;
        }
        let _ = seen.insert(name, how);
        out.names.append(name);
    }
    .Ok(out)
}
