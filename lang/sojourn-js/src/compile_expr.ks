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
            .BigInt(_) => throw compileError("BigInt isn't supported yet"),
            .StringLit(s) => self.loadString(dst, s),
            .Regex(pattern, flags) => {
                let callee = self.reg();
                self.emitGetName("RegExp", callee, false);
                let args = self.reg();
                let _ = self.emit(.NewArray(dst: args));
                let t = self.reg();
                self.loadString(t, pattern);
                let _ = self.emit(.ArrayAppend(arr: args, src: t));
                self.loadString(t, flags);
                let _ = self.emit(.ArrayAppend(arr: args, src: t));
                let _ = self.emit(.New(dst: dst, callee: callee, args: args));
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
            .PrivateIn(_, _) => throw compileError("private names aren't supported yet"),
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
            .Yield(_, _) => throw compileError("generators aren't supported yet"),
            .Await(_) => throw compileError("async functions aren't supported yet"),
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
            try self.compileExpr(callee, f);
            self.load(this, Value.undefined);
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
                // Declared bindings can't be deleted; global properties can.
                let r = self.resolve(n);
                if r.found {
                    self.load(dst, Value.bool(false));
                } else {
                    let global = self.reg();
                    self.emitGetName("globalThis", global, false);
                    let key = self.reg();
                    self.loadString(key, n);
                    let _ = self.emit(.DeleteElem(dst: dst, obj: global, key: key, strict: false));
                }
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

    /// Evaluates the parts of an assignable expression (object and key)
    /// once, for reading and writing it.
    mutating func compileReference(e: Expr) -> Result[Reference, SyntaxError] {
        match e.kind {
            .Identifier(n) => return .Ok(Reference(name: n, obj: -1, key: -1, isSuper: false)),
            .Member(object, property, _) => {
                let key = self.reg();
                if let .Super = object.kind {
                    let this = self.reg();
                    self.emitGetName("this", this, false);
                    try self.compileMemberKey(property, key);
                    return .Ok(Reference(name: "", obj: this, key: key, isSuper: true));
                }
                let obj = self.reg();
                try self.compileExpr(object, obj);
                try self.compileMemberKey(property, key);
                return .Ok(Reference(name: "", obj: obj, key: key, isSuper: false));
            },
            _ => throw compileError("invalid assignment target")
        }
    }

    mutating func readReference(r: Reference, dst: Int64) -> Result[(), SyntaxError] {
        if r.obj < 0 {
            self.emitGetName(r.name, dst, false);
        } else if r.isSuper {
            let _ = self.emit(.SuperGet(dst: dst, key: r.key, this: r.obj));
        } else {
            let _ = self.emit(.GetElem(dst: dst, obj: r.obj, key: r.key));
        }
        .Ok(())
    }

    mutating func writeReference(r: Reference, src: Int64) -> Result[(), SyntaxError] {
        if r.obj < 0 {
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
        var superclass: Int64 = -1;
        if let .Some(s) = c.superClass {
            superclass = self.reg();
            try self.compileExpr(s, superclass);
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
        flags.hasFields = not instanceFields.isEmpty;
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
        if not instanceFields.isEmpty {
            let code = try self.compileFieldInitializer(instanceFields);
            let fi = self.reg();
            let _ = self.emit(.MakeClosure(dst: fi, code: code, home: proto));
            let _ = self.emit(.SetClassFields(ctor: dst, fn: fi));
        }

        if pushed {
            let _ = self.emit(.InitVar(hops: 0, slot: 0, src: dst));
        }

        // Static fields and blocks run now, in order, with `this` the class.
        for m in c.body {
            let mark = self.regMark();
            match m {
                .Field(_, isStatic, _) => if isStatic {
                    let code = try self.compileFieldInitializer([m]);
                    try self.emitRunWithThis(code, dst);
                },
                .StaticBlock(body) => {
                    let fun = Function(id: .None, params: [], body: .Block(body), isAsync: false, isGenerator: false);
                    var mf = plainFlags();
                    mf.isMethod = true;
                    let code = try self.compileFunction(fun, "", mf);
                    try self.emitRunWithThis(code, dst);
                },
                _ => {}
            }
            self.freeRegs(to: mark);
        }
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

    /// A method that defines `fields` on `this`.
    mutating func compileFieldInitializer(fields: Array[ClassMember]) -> Result[Int64, SyntaxError] {
        self.beginFunction(true, false, false);
        let this = self.reg();
        self.emitGetName("this", this, false);
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
}

/// `constructor(...args) { super(...args); }` or `constructor() {}`.
func defaultConstructor(derived: Bool) -> Function {
    if not derived {
        return Function(id: .None, params: [], body: .Block([]), isAsync: false, isGenerator: false);
    }
    let noSpan = Span(start: 0, end: 0);
    let args = Expr(kind: .Identifier("args"), span: noSpan, parenthesized: false);
    let sup = Expr(kind: .Super, span: noSpan, parenthesized: false);
    let call = Expr(kind: .Call(callee: sup, arguments: [.Spread(args)], optional: false), span: noSpan, parenthesized: false);
    let body = [Stmt(kind: .Expression(call), span: noSpan)];
    Function(id: .None, params: [.Rest(.Identifier("args"))], body: .Block(body), isAsync: false, isGenerator: false)
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
