module js

// Compiling a module (see module.ks). The module scope is the single
// scope of a synthetic outer function, so its slots are known; `init` and
// `body` are arrows inside it, run as closures over the module's
// environment once the graph links.

import std.collections.(Dictionary, Set)

extend VM {
    /// Compiles module `mi`'s code into its record.
    mutating func compileModule(p: Program, mi: Int64) -> Result[(), SyntaxError] {
        self.fns = [];
        self.compilingModule = mi;
        // Strict, `this` undefined; a script-like top so `arguments` is global.
        self.beginFunction(true, false, true);
        let outer = self.fnIndex();

        var vars: [String] = [];
        collectVarNames(p.body, vars);
        var lexicals: [String] = [];
        var consts: [String] = [];
        collectLexicalNames(p.body, false, lexicals, consts);
        var funs = hoistedFunctions(p.body);
        var funNames: [String] = [];
        for f in funs { funNames.append(f.id ?? ""); }
        // `export default`: a hoisted function, a class, or an expression
        // bound to *default*.
        for s in p.body {
            let d = match s.kind {
                .Export(.Default(x)) => x,
                _ => continue
            };
            match d.kind {
                .FunctionDecl(f) => {
                    funs.append(f);
                    funNames.append(f.id ?? "*default*");
                },
                .ClassDecl(c) => match c.id {
                    .Some(n) => lexicals.append(n),
                    .None => {
                        lexicals.append("*default*");
                        consts.append("*default*");
                    }
                },
                _ => {
                    lexicals.append("*default*");
                    consts.append("*default*");
                }
            }
        }
        var importNames: [String] = [];
        for imp in self.modules(mi).imports { importNames.append(imp.localName); }
        for n in importNames { let _ = self.fns(outer).scopes(0).declare(n); }
        for n in vars { let _ = self.fns(outer).scopes(0).declare(n); }
        for n in funNames { let _ = self.fns(outer).scopes(0).declare(n); }
        for n in lexicals {
            let _ = self.fns(outer).scopes(0).declare(n);
            let _ = self.fns(outer).scopes(0).lexicals.insert(n);
        }
        for n in consts { let _ = self.fns(outer).scopes(0).consts.insert(n); }
        for n in importNames { let _ = self.fns(outer).scopes(0).consts.insert(n); }

        // init: the TDZ for lexical bindings, then the functions.
        self.beginFunction(true, true, false);
        let t = self.reg();
        self.load(t, Value.empty);
        for n in lexicals {
            let r = self.resolve(n);
            let _ = self.emit(.InitVar(hops: r.hops, slot: r.slot, src: t));
        }
        for i in 0..<funs.count {
            let name = funs(i).id ?? "default";
            let code = try self.compileFunction(funs(i), name, plainFlags());
            let _ = self.emit(.MakeClosure(dst: t, code: code, home: -1));
            let r = self.resolve(funNames(i));
            let _ = self.emit(.InitVar(hops: r.hops, slot: r.slot, src: t));
        }
        var arrow = plainFlags();
        arrow.isArrow = true;
        let initCode = self.endFunction("<module init>", 0, arrow);

        // body: the statements, async with top-level await.
        self.beginFunction(true, true, false);
        let bi = self.fnIndex();
        self.fns(bi).isAsync = p.hasTopLevelAwait;
        if p.hasTopLevelAwait { let _ = self.emit(.GeneratorStart); }
        for s in p.body { try self.compileModuleItem(s); }
        let bodyCode = self.endFunction("<module>", 0, arrow);

        let scope = self.fns(outer).scopes(0);
        self.fns = [];
        self.compilingModule = -1;
        self.modules(mi).slots = scope.slots;
        self.modules(mi).envSize = scope.size;
        self.modules(mi).initCode = initCode;
        self.modules(mi).bodyCode = bodyCode;
        .Ok(())
    }

    mutating func compileModuleItem(s: Stmt) -> Result[(), SyntaxError] {
        match s.kind {
            .Import(_) => {},
            .Export(e) => match e {
                .Declaration(d) => try self.compileStmt(d),
                .Default(d) => match d.kind {
                    .FunctionDecl(_) => {},
                    .ClassDecl(c) => {
                        if c.id.isSome {
                            try self.compileStmt(d);
                        } else {
                            let r = self.reg();
                            try self.compileClass(c, r, "default");
                            self.initDefaultExport(r);
                        }
                    },
                    .Expression(x) => {
                        let r = self.reg();
                        try self.compileNamed(x, r, "default");
                        self.initDefaultExport(r);
                    },
                    _ => try self.compileStmt(d)
                },
                _ => {}
            },
            _ => try self.compileStmt(s)
        }
        .Ok(())
    }

    mutating func initDefaultExport(r: Int64) {
        let b = self.resolve("*default*");
        let _ = self.emit(.InitVar(hops: b.hops, slot: b.slot, src: r));
        self.freeRegs(to: r);
    }
}
