module js

// Declaration scopes, for the early errors about redeclaring names:
// `let a; var a;`, `{ const a = 1; function a(){} }`, `(a) => { let a; }`.
//
// `var` and top-level function declarations belong to the enclosing
// function, but they're checked against every block they hoist through,
// so `{ let a; { var a; } }` is an error too.

import std.collections.(Set)

@derive(Equatable)
enum ScopeKind {
    /// A function body, or the top level of a script or module.
    case Function
    case Block
    /// A catch clause; `params` holds the catch parameter's names.
    case Catch
}

struct Scope {
    var kind: ScopeKind
    /// let, const, class, and function declarations inside blocks.
    var lexical: Set[String]
    /// Plain function declarations among `lexical`, which sloppy code may
    /// declare twice in one block (Annex B).
    var functions: Set[String]
    /// `var` names declared in this scope or hoisted through it.
    var vars: Set[String]
    /// Parameter names (functions) or the catch parameter's names.
    var params: Set[String]
    /// The catch parameter is a plain name, so `var` can redeclare it.
    var simpleCatch: Bool
}

func newScope(kind: ScopeKind) -> Scope {
    Scope(kind: kind, lexical: Set(), functions: Set(), vars: Set(), params: Set(), simpleCatch: false)
}

extend Parser {
    mutating func enterScope(kind: ScopeKind) {
        self.scopes.append(newScope(kind));
    }

    mutating func exitScope() {
        let _ = self.scopes.pop();
    }

    /// At the top level of a module, where function declarations are
    /// lexical rather than var-like.
    func atModuleTop() -> Bool {
        self.isModule and self.scopes.count == 1
    }

    /// Declares let/const/class/import names in the innermost scope.
    mutating func declareLexical(name: String, at: Int64) -> Result[(), SyntaxError] {
        let top = self.scopes.count - 1;
        let s = self.scopes(top);
        if s.lexical.contains(name) or s.vars.contains(name) or s.params.contains(name) {
            throw earlyError("'\(name)' has already been declared", at: at);
        }
        let _ = self.scopes(top).lexical.insert(name);
        .Ok(())
    }

    /// Declares a function declaration's name. At the top of a function or
    /// script it acts like `var`; in a block (or a module's top) it's
    /// lexical.
    mutating func declareFunction(name: String, at: Int64, isPlain: Bool) -> Result[(), SyntaxError] {
        let top = self.scopes.count - 1;
        let s = self.scopes(top);
        if s.kind == ScopeKind.Function and not self.atModuleTop() {
            if s.lexical.contains(name) { throw earlyError("'\(name)' has already been declared", at: at); }
            let _ = self.scopes(top).vars.insert(name);
            return .Ok(());
        }
        if s.lexical.contains(name) {
            // Sloppy code may repeat plain function declarations in a block.
            let bothPlain = isPlain and s.functions.contains(name);
            if self.ctx.strict or not bothPlain or self.atModuleTop() {
                throw earlyError("'\(name)' has already been declared", at: at);
            }
            return .Ok(());
        }
        if s.vars.contains(name) or s.params.contains(name) {
            throw earlyError("'\(name)' has already been declared", at: at);
        }
        let _ = self.scopes(top).lexical.insert(name);
        if isPlain { let _ = self.scopes(top).functions.insert(name); }
        .Ok(())
    }

    /// Declares a `var` name, checking every scope it hoists through.
    /// `forOf` marks `for (var x of …)`, which can't redeclare a catch
    /// parameter even when other `var`s can.
    mutating func declareVar(name: String, at: Int64, forOf: Bool) -> Result[(), SyntaxError] {
        var i = self.scopes.count - 1;
        while i >= 0 {
            let s = self.scopes(i);
            if s.lexical.contains(name) {
                throw earlyError("'\(name)' has already been declared", at: at);
            }
            if s.kind == ScopeKind.Catch and s.params.contains(name) and (forOf or not s.simpleCatch) {
                throw earlyError("'\(name)' has already been declared", at: at);
            }
            let _ = self.scopes(i).vars.insert(name);
            if s.kind == ScopeKind.Function { break; }
            i = i - 1;
        }
        .Ok(())
    }

    /// Declares every name `p` binds, as `kind`.
    mutating func declarePattern(p: Pattern, kind: VariableKind, at: Int64, forOf: Bool) -> Result[(), SyntaxError] {
        var names: [String] = [];
        collectNames(p, names);
        for n in names {
            if kind == VariableKind.Var {
                try self.declareVar(n, at, forOf);
            } else {
                try self.declareLexical(n, at);
            }
        }
        .Ok(())
    }

    /// Declares a declaration's names.
    mutating func declareDeclaration(d: VariableDeclaration, at: Int64, forOf: Bool) -> Result[(), SyntaxError] {
        for decl in d.declarations {
            try self.declarePattern(decl.target, d.kind, at, forOf);
        }
        .Ok(())
    }

    /// Starts a function scope holding `params`' names.
    mutating func enterFunctionScope(params: Array[Pattern]) {
        var s = newScope(.Function);
        for n in boundNames(params) { let _ = s.params.insert(n); }
        self.scopes.append(s);
    }

    /// Declares a catch parameter into the innermost (catch) scope. Its
    /// names can't repeat.
    mutating func declareCatchParameter(p: Pattern, at: Int64) -> Result[(), SyntaxError] {
        let top = self.scopes.count - 1;
        var names: [String] = [];
        collectNames(p, names);
        for n in names {
            if self.scopes(top).params.contains(n) {
                throw earlyError("'\(n)' has already been declared", at: at);
            }
            let _ = self.scopes(top).params.insert(n);
        }
        let simple = match p { .Identifier(_) => true, _ => false };
        self.scopes(top).simpleCatch = simple;
        .Ok(())
    }

    /// Records an exported name; each may be exported once.
    mutating func declareExport(name: String, at: Int64) -> Result[(), SyntaxError] {
        if not self.exportedNames.insert(name) {
            throw earlyError("duplicate export '\(name)'", at: at);
        }
        .Ok(())
    }

    /// `export { a }` with no `from` needs `a` declared at the module's
    /// top level. Checked once the whole module is parsed.
    func checkExportedLocals() -> Result[(), SyntaxError] {
        if self.scopes.count == 0 { return .Ok(()); }
        let top = self.scopes(0);
        for i in 0..<self.exportedLocals.count {
            let name = self.exportedLocals(i);
            if not top.lexical.contains(name) and not top.vars.contains(name) {
                throw earlyError("exported '\(name)' isn't declared", at: self.exportedLocalPositions(i));
            }
        }
        .Ok(())
    }
}

/// A labeled function declaration (`a: function f() {}`, through any
/// number of labels), which can't be the body of an `if` or a loop.
func isLabelledFunction(s: Stmt) -> Bool {
    match s.kind {
        .Labeled(_, body) => match body.kind {
            .FunctionDecl(_) => true,
            _ => isLabelledFunction(body)
        },
        _ => false
    }
}

/// The names a declaration statement binds, for `export <declaration>`.
func declaredNames(s: Stmt) -> Array[String] {
    var out: [String] = [];
    match s.kind {
        .Var(d) => {
            for decl in d.declarations { collectNames(decl.target, out); }
        },
        .FunctionDecl(f) => if let .Some(n) = f.id { out.append(n); },
        .ClassDecl(c) => if let .Some(n) = c.id { out.append(n); },
        _ => {}
    }
    out
}
