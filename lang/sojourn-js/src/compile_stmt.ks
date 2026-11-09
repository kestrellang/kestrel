module js

// Compiling statements: declarations, control flow, and the cleanup a
// jump out of a block needs (popping environments, running `finally`,
// closing iterators).

extend VM {
    mutating func compileStmt(s: Stmt) -> Result[(), SyntaxError] {
        let mark = self.regMark();
        let f = self.fnIndex();
        let labels = self.fns(f).pendingLabels;
        self.fns(f).pendingLabels = [];
        match s.kind {
            .Expression(e) => {
                // Script code keeps the last expression's value for eval.
                let completion = self.fns(self.fnIndex()).completion;
                let r = if completion >= 0 { completion } else { self.reg() };
                try self.compileExpr(e, r);
            },
            .Directive(_) => {},
            .Var(d) => try self.compileDeclaration(d),
            .FunctionDecl(_) => {},
            .ClassDecl(c) => {
                let r = self.reg();
                let name = c.id ?? "";
                try self.compileClass(c, r, name);
                self.emitSetName(name, r, .Declare);
            },
            .Block(body) => {
                let pushed = try self.enterBlockScope(body, []);
                for x in body { try self.compileStmt(x); }
                self.exitBlockScope(pushed);
            },
            .Empty => {},
            .Debugger => {},
            .If(test, consequent, alternate) => {
                let c = self.reg();
                try self.compileExpr(test, c);
                let skip = self.emit(.JumpIf(cond: c, target: 0, when: false));
                try self.compileSubstatement(consequent);
                if let .Some(alt) = alternate {
                    let over = self.emit(.Jump(target: 0));
                    self.patchHere(skip);
                    try self.compileSubstatement(alt);
                    self.patchHere(over);
                } else {
                    self.patchHere(skip);
                }
            },
            .While(test, body) => {
                let top = self.here();
                let c = self.reg();
                try self.compileExpr(test, c);
                let exit = self.emit(.JumpIf(cond: c, target: 0, when: false));
                self.pushCtl(.Loop, labels, -1);
                try self.compileSubstatement(body);
                self.patchContinues(top);
                let _ = self.emit(.Jump(target: top));
                self.patchHere(exit);
                self.popCtl();
            },
            .DoWhile(body, test) => {
                let top = self.here();
                self.pushCtl(.Loop, labels, -1);
                try self.compileSubstatement(body);
                self.patchContinues(self.here());
                let c = self.reg();
                try self.compileExpr(test, c);
                let _ = self.emit(.JumpIf(cond: c, target: top, when: true));
                self.popCtl();
            },
            .For(setup, test, update, body) => try self.compileFor(setup, test, update, body, labels),
            .ForIn(left, right, body) => try self.compileForInOf(left, right, body, false, labels),
            .ForOf(left, right, body, isAwait) => {
                if isAwait { throw compileError("for await isn't supported yet"); }
                try self.compileForInOf(left, right, body, true, labels);
            },
            .Return(value) => {
                let r = self.reg();
                if let .Some(v) = value { try self.compileExpr(v, r); } else { self.load(r, Value.undefined); }
                try self.emitCleanup(-1);
                let _ = self.emit(.Return(src: r));
            },
            .Break(label) => try self.emitJumpOut(true, label),
            .Continue(label) => try self.emitJumpOut(false, label),
            .Throw(value) => {
                let r = self.reg();
                try self.compileExpr(value, r);
                let _ = self.emit(.Throw(src: r));
            },
            .Try(block, handler, finalizer) => try self.compileTry(block, handler, finalizer),
            .Switch(discriminant, cases) => try self.compileSwitch(discriminant, cases, labels),
            .Labeled(label, body) => {
                var all = labels;
                all.append(label);
                if isLoop(body) {
                    self.fns(f).pendingLabels = all;
                    try self.compileStmt(body);
                } else {
                    self.pushCtl(.Breakable, all, -1);
                    try self.compileStmt(body);
                    self.popCtl();
                }
            },
            .With(_, _) => throw compileError("'with' isn't supported yet"),
            .Import(_) => throw compileError("modules aren't supported yet"),
            .Export(_) => throw compileError("modules aren't supported yet")
        }
        self.freeRegs(to: mark);
        .Ok(())
    }

    /// The body of an `if` or loop. A function declaration there (sloppy
    /// Annex B) acts as if it were in its own block.
    mutating func compileSubstatement(s: Stmt) -> Result[(), SyntaxError] {
        if let .FunctionDecl(_) = s.kind {
            let body = [s];
            let pushed = try self.enterBlockScope(body, []);
            self.exitBlockScope(pushed);
            return .Ok(());
        }
        self.compileStmt(s)
    }

    mutating func compileDeclaration(d: VariableDeclaration) -> Result[(), SyntaxError] {
        if d.kind == VariableKind.Using or d.kind == VariableKind.AwaitUsing {
            throw compileError("'using' isn't supported yet");
        }
        for decl in d.declarations {
            let mark = self.regMark();
            let r = self.reg();
            if let .Some(value) = decl.value {
                match decl.target {
                    .Identifier(n) => try self.compileNamed(value, r, n),
                    _ => try self.compileExpr(value, r)
                }
                try self.bindPattern(decl.target, r, .Declare);
            } else if d.kind != VariableKind.Var {
                self.load(r, Value.undefined);
                try self.bindPattern(decl.target, r, .Declare);
            }
            self.freeRegs(to: mark);
        }
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Loops
    // ------------------------------------------------------------------

    mutating func compileFor(setup: Optional[ForInit], test: Optional[Expr], update: Optional[Expr], body: Stmt, labels: Array[String]) -> Result[(), SyntaxError] {
        var pushed = false;
        var perIteration = false;
        match setup {
            .Some(.Declaration(d)) => {
                if d.kind != VariableKind.Var {
                    let stmt = Stmt(kind: .Var(d), span: Span(start: 0, end: 0));
                    pushed = try self.enterBlockScope([stmt], []);
                    perIteration = d.kind == VariableKind.Let;
                }
                try self.compileDeclaration(d);
            },
            .Some(.Expression(e)) => {
                let r = self.reg();
                try self.compileExpr(e, r);
            },
            .None => {}
        }
        if perIteration { let _ = self.emit(.CopyEnv); }
        let top = self.here();
        var exit: Int64 = -1;
        if let .Some(t) = test {
            let c = self.reg();
            try self.compileExpr(t, c);
            exit = self.emit(.JumpIf(cond: c, target: 0, when: false));
        }
        self.pushCtl(.Loop, labels, -1);
        try self.compileSubstatement(body);
        self.patchContinues(self.here());
        if perIteration { let _ = self.emit(.CopyEnv); }
        if let .Some(u) = update {
            let r = self.reg();
            try self.compileExpr(u, r);
        }
        let _ = self.emit(.Jump(target: top));
        if exit >= 0 { self.patchHere(exit); }
        self.popCtl();
        self.exitBlockScope(pushed);
        .Ok(())
    }

    mutating func compileForInOf(left: ForTarget, right: Expr, body: Stmt, isOf: Bool, labels: Array[String]) -> Result[(), SyntaxError] {
        let obj = self.reg();
        try self.compileExpr(right, obj);
        let it = self.regs(2);
        if isOf {
            let _ = self.emit(.GetIterator(iter: it, obj: obj));
        } else {
            let _ = self.emit(.ForInPrepare(dst: it, obj: obj));
        }
        let top = self.here();
        let v = self.reg();
        let next = if isOf {
            self.emit(.IteratorNext(dst: v, iter: it, done: 0))
        } else {
            self.emit(.ForInNext(dst: v, iter: it, done: 0))
        };
        self.pushCtl(.Loop, labels, if isOf { it } else { -1 });
        var pushed = false;
        match left {
            .Declaration(d) => {
                let target = d.declarations(0).target;
                if d.kind != VariableKind.Var {
                    let stmt = Stmt(kind: .Var(d), span: Span(start: 0, end: 0));
                    pushed = try self.enterBlockScope([stmt], []);
                }
                try self.bindPattern(target, v, .Declare);
            },
            .Target(p) => try self.bindPattern(p, v, .Assign)
        }
        try self.compileSubstatement(body);
        self.exitBlockScope(pushed);
        self.patchContinues(self.here());
        let _ = self.emit(.Jump(target: top));
        self.patchHere(next);
        self.popCtl();
        .Ok(())
    }

    // ------------------------------------------------------------------
    // Jumps out
    // ------------------------------------------------------------------

    mutating func pushCtl(kind: CtlKind, labels: Array[String], iter: Int64) {
        let f = self.fnIndex();
        let depth = self.scopeDepth();
        self.fns(f).ctl.append(Ctl(kind: kind, labels: labels, breaks: [], continues: [], envDepth: depth, finalizer: .None, iter: iter));
    }

    /// Pops the innermost construct, pointing its breaks here.
    mutating func popCtl() {
        let f = self.fnIndex();
        let popped = self.fns(f).ctl.pop();
        if let .Some(c) = popped {
            for b in c.breaks { self.patchHere(b); }
        }
    }

    mutating func patchContinues(target: Int64) {
        let f = self.fnIndex();
        let top = self.fns(f).ctl.count - 1;
        let conts = self.fns(f).ctl(top).continues;
        for c in conts { self.patch(c, target); }
    }

    /// `break` or `continue`, with an optional label.
    mutating func emitJumpOut(isBreak: Bool, label: Optional[String]) -> Result[(), SyntaxError] {
        let f = self.fnIndex();
        var target: Int64 = -1;
        var i = self.fns(f).ctl.count - 1;
        while i >= 0 {
            let c = self.fns(f).ctl(i);
            let matches = match label {
                .Some(l) => c.labels.contains(l) and (isBreak or c.kind == CtlKind.Loop),
                .None => c.kind == CtlKind.Loop or (isBreak and c.kind == CtlKind.Switch)
            };
            if matches {
                target = i;
                break;
            }
            i = i - 1;
        }
        if target < 0 { throw compileError("no target for break or continue"); }
        try self.emitCleanup(target);
        let c = self.fns(f).ctl(target);
        if isBreak and c.iter >= 0 { let _ = self.emit(.IteratorClose(iter: c.iter)); }
        var depth = self.scopeDepth();
        while depth > c.envDepth {
            let _ = self.emit(.PopEnv);
            depth = depth - 1;
        }
        let j = self.emit(.Jump(target: 0));
        if isBreak {
            self.fns(f).ctl(target).breaks.append(j);
        } else {
            self.fns(f).ctl(target).continues.append(j);
        }
        .Ok(())
    }

    /// Leaves every construct above `target` (-1 for all of them): pops
    /// environments, leaves try blocks, runs finally blocks and closes
    /// for-of iterators.
    mutating func emitCleanup(target: Int64) -> Result[(), SyntaxError] {
        let f = self.fnIndex();
        var depth = self.scopeDepth();
        var i = self.fns(f).ctl.count - 1;
        while i > target {
            let c = self.fns(f).ctl(i);
            while depth > c.envDepth {
                let _ = self.emit(.PopEnv);
                depth = depth - 1;
            }
            match c.kind {
                .Finally => {
                    let _ = self.emit(.LeaveTry);
                    if let .Some(body) = c.finalizer { try self.compileFinallyInline(i, body); }
                },
                .Catch => { let _ = self.emit(.LeaveTry); },
                .Loop => if c.iter >= 0 { let _ = self.emit(.IteratorClose(iter: c.iter)); },
                _ => {}
            }
            i = i - 1;
        }
        .Ok(())
    }

    /// Compiles a finally block on a jump's way out, as if it ran where
    /// its try statement is: only the constructs and scopes outside it.
    mutating func compileFinallyInline(at: Int64, body: Array[Stmt]) -> Result[(), SyntaxError] {
        let f = self.fnIndex();
        let savedCtl = self.fns(f).ctl;
        let savedScopes = self.fns(f).scopes;
        let depth = savedCtl(at).envDepth;
        var ctl: [Ctl] = [];
        for j in 0..<at { ctl.append(savedCtl(j)); }
        var scopes: [CScope] = [];
        for j in 0..<depth { scopes.append(savedScopes(j)); }
        self.fns(f).ctl = ctl;
        self.fns(f).scopes = scopes;
        let block = Stmt(kind: .Block(body), span: Span(start: 0, end: 0));
        let result = self.compileStmt(block);
        // Scopes may have gained an `arguments` binding; keep the top one.
        let top = self.fns(f).scopes(0);
        self.fns(f).ctl = savedCtl;
        self.fns(f).scopes = savedScopes;
        self.fns(f).scopes(0) = top;
        result
    }

    // ------------------------------------------------------------------
    // try and switch
    // ------------------------------------------------------------------

    mutating func compileTry(block: Array[Stmt], handler: Optional[CatchClause], finalizer: Optional[Array[Stmt]]) -> Result[(), SyntaxError] {
        let f = self.fnIndex();
        let finallyException = self.reg();
        var finallyTry: Int64 = -1;
        if let .Some(body) = finalizer {
            self.pushCtl(.Finally, [], -1);
            let top = self.fns(f).ctl.count - 1;
            self.fns(f).ctl(top).finalizer = .Some(body);
            finallyTry = self.emit(.EnterTry(handler: 0, exception: finallyException));
        }
        if let .Some(h) = handler {
            let exception = self.reg();
            self.pushCtl(.Catch, [], -1);
            let enter = self.emit(.EnterTry(handler: 0, exception: exception));
            try self.compileStmt(Stmt(kind: .Block(block), span: Span(start: 0, end: 0)));
            let _ = self.emit(.LeaveTry);
            let _ = self.fns(f).ctl.pop();
            let over = self.emit(.Jump(target: 0));
            self.patchHere(enter);
            var names: [String] = [];
            if let .Some(p) = h.param { collectNames(p, names); }
            let pushed = try self.enterBlockScope(h.body, names);
            if let .Some(p) = h.param { try self.bindPattern(p, exception, .Declare); }
            for s in h.body { try self.compileStmt(s); }
            self.exitBlockScope(pushed);
            self.patchHere(over);
        } else {
            try self.compileStmt(Stmt(kind: .Block(block), span: Span(start: 0, end: 0)));
        }
        if let .Some(body) = finalizer {
            let _ = self.emit(.LeaveTry);
            let _ = self.fns(f).ctl.pop();
            try self.compileStmt(Stmt(kind: .Block(body), span: Span(start: 0, end: 0)));
            let over = self.emit(.Jump(target: 0));
            self.patchHere(finallyTry);
            try self.compileStmt(Stmt(kind: .Block(body), span: Span(start: 0, end: 0)));
            let _ = self.emit(.Throw(src: finallyException));
            self.patchHere(over);
        }
        .Ok(())
    }

    mutating func compileSwitch(discriminant: Expr, cases: Array[SwitchCase], labels: Array[String]) -> Result[(), SyntaxError] {
        let d = self.reg();
        try self.compileExpr(discriminant, d);
        var all: [Stmt] = [];
        for c in cases {
            for s in c.body { all.append(s); }
        }
        let pushed = try self.enterBlockScope(all, []);
        self.pushCtl(.Switch, labels, -1);
        var jumps: [Int64] = [];
        for c in cases {
            if let .Some(test) = c.test {
                let mark = self.regMark();
                let t = self.reg();
                try self.compileExpr(test, t);
                let _ = self.emit(.Binary(op: .StrictEq, dst: t, a: d, b: t));
                jumps.append(self.emit(.JumpIf(cond: t, target: 0, when: true)));
                self.freeRegs(to: mark);
            } else {
                jumps.append(-1);
            }
        }
        let toDefault = self.emit(.Jump(target: 0));
        var sawDefault = false;
        for i in 0..<cases.count {
            if jumps(i) >= 0 {
                self.patchHere(jumps(i));
            } else {
                self.patchHere(toDefault);
                sawDefault = true;
            }
            for s in cases(i).body { try self.compileStmt(s); }
        }
        if not sawDefault { self.patchHere(toDefault); }
        self.popCtl();
        self.exitBlockScope(pushed);
        .Ok(())
    }
}

func isLoop(s: Stmt) -> Bool {
    match s.kind {
        .For(_, _, _, _) => true,
        .ForIn(_, _, _) => true,
        .ForOf(_, _, _, _) => true,
        .While(_, _) => true,
        .DoWhile(_, _) => true,
        .Labeled(_, body) => isLoop(body),
        _ => false
    }
}
