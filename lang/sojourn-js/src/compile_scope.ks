module js

// Compile-time scopes: which names each scope declares and where they
// live, and the declaration scans that find them before code runs
// (hoisting).

import std.collections.(Dictionary, Set)

/// One environment's worth of bindings. Each scope the compiler pushes
/// becomes one `Env` at run time, so (hops, slot) addresses line up.
struct CScope {
    var slots: [String: Int64]
    var consts: Set[String]
    var size: Int64
    /// A function's own top scope, where `var`s go.
    var isFunction: Bool
    /// An arrow function's top scope: `this` and `arguments` come from outside.
    var isArrow: Bool
    /// A `with` statement's scope: slot 0 holds the object, and names
    /// resolved past it are looked up on the object first.
    var isWith: Bool
}

func newCScope(isFunction: Bool, isArrow: Bool) -> CScope {
    CScope(slots: [:], consts: Set(), size: 0, isFunction: isFunction, isArrow: isArrow, isWith: false)
}

extend CScope {
    /// Adds `name` if it's new, returning its slot.
    mutating func declare(name: String) -> Int64 {
        if let .Some(s) = self.slots(name) { return s; }
        let s = self.size;
        let _ = self.slots.insert(name, s);
        self.size = s + 1;
        s
    }
}

/// A break or continue target, or a construct a jump out of must clean up.
struct Ctl {
    var kind: CtlKind
    var labels: [String]
    var breaks: [Int64]
    var continues: [Int64]
    /// How many scopes were open when the construct started.
    var envDepth: Int64
    var finalizer: Optional[Array[Stmt]]
    /// The iterator register of a for-of loop, closed when a jump leaves
    /// it, or -1.
    var iter: Int64
}

@derive(Equatable)
enum CtlKind {
    case Loop
    /// A labelled statement that isn't a loop: `break label` only.
    case Breakable
    /// Takes unlabelled `break`.
    case Switch
    case Finally
    case Catch
}

/// Per-function compiler state.
struct FnState {
    var instrs: [Instr]
    var scopes: [CScope]
    var nextReg: Int64
    var maxReg: Int64
    var ctl: [Ctl]
    var strict: Bool
    var isArrow: Bool
    var isScript: Bool
    /// Labels just before the statement being compiled.
    var pendingLabels: [String]
    /// Jumps out of the current optional chain, to its end.
    var chainJumps: [[Int64]]
    /// Where `arguments` gets created if the body turns out to use it.
    var argumentsAt: Int64
    var usesArguments: Bool
    var isDerivedCtor: Bool
    /// The class whose constructor this is runs field initializers.
    var hasFields: Bool
    /// Scripts only: the register holding the completion value (what
    /// `eval` returns), or -1.
    var completion: Int64
    var isGenerator: Bool
}

/// The names a list of statements declares with `var` (looking into
/// blocks but not functions), plus top-level function declarations.
func collectVarNames(stmts: Array[Stmt], mutating out: Array[String]) {
    for s in stmts { collectVarNamesStmt(s, out); }
}

func collectVarNamesStmt(s: Stmt, mutating out: Array[String]) {
    match s.kind {
        .Var(d) => if d.kind == VariableKind.Var {
            for decl in d.declarations { collectNames(decl.target, out); }
        },
        .Block(body) => collectVarNames(body, out),
        .If(_, c, a) => {
            collectVarNamesStmt(c, out);
            if let .Some(x) = a { collectVarNamesStmt(x, out); }
        },
        .For(setup, _, _, body) => {
            if let .Some(.Declaration(d)) = setup {
                if d.kind == VariableKind.Var {
                    for decl in d.declarations { collectNames(decl.target, out); }
                }
            }
            collectVarNamesStmt(body, out);
        },
        .ForIn(left, _, body) => {
            collectForTargetVars(left, out);
            collectVarNamesStmt(body, out);
        },
        .ForOf(left, _, body, _) => {
            collectForTargetVars(left, out);
            collectVarNamesStmt(body, out);
        },
        .While(_, body) => collectVarNamesStmt(body, out),
        .DoWhile(body, _) => collectVarNamesStmt(body, out),
        .Try(block, handler, finalizer) => {
            collectVarNames(block, out);
            if let .Some(h) = handler { collectVarNames(h.body, out); }
            if let .Some(f) = finalizer { collectVarNames(f, out); }
        },
        .Switch(_, cases) => {
            for c in cases { collectVarNames(c.body, out); }
        },
        .Labeled(_, body) => collectVarNamesStmt(body, out),
        .With(_, body) => collectVarNamesStmt(body, out),
        .Export(e) => match e {
            .Declaration(d) => collectVarNamesStmt(d, out),
            _ => {}
        },
        _ => {}
    }
}

func collectForTargetVars(t: ForTarget, mutating out: Array[String]) {
    if let .Declaration(d) = t {
        if d.kind == VariableKind.Var {
            for decl in d.declarations { collectNames(decl.target, out); }
        }
    }
}

/// A scope's lexical declarations: let, const, class, and (in blocks)
/// function declarations. `consts` gets the const and class names.
func collectLexicalNames(stmts: Array[Stmt], includeFunctions: Bool, mutating out: Array[String], mutating consts: Array[String]) {
    for s in stmts {
        collectLexicalStmt(s, includeFunctions, out, consts);
    }
}

func collectLexicalStmt(s: Stmt, includeFunctions: Bool, mutating out: Array[String], mutating consts: Array[String]) {
    match s.kind {
        .Var(d) => if d.kind != VariableKind.Var {
            var names: [String] = [];
            for decl in d.declarations { collectNames(decl.target, names); }
            for n in names {
                out.append(n);
                if d.kind == VariableKind.Const { consts.append(n); }
            }
        },
        .ClassDecl(c) => if let .Some(n) = c.id { out.append(n); },
        .FunctionDecl(f) => if includeFunctions {
            if let .Some(n) = f.id { out.append(n); }
        },
        .Labeled(_, body) => collectLexicalStmt(body, includeFunctions, out, consts),
        .Export(e) => match e {
            .Declaration(d) => collectLexicalStmt(d, includeFunctions, out, consts),
            _ => {}
        },
        _ => {}
    }
}

/// The function declarations directly in a list of statements (through
/// labels), which are created when the scope starts.
func hoistedFunctions(stmts: Array[Stmt]) -> Array[Function] {
    var out: [Function] = [];
    for s in stmts { hoistedFunctionStmt(s, out); }
    out
}

func hoistedFunctionStmt(s: Stmt, mutating out: Array[Function]) {
    match s.kind {
        .FunctionDecl(f) => out.append(f),
        .Labeled(_, body) => hoistedFunctionStmt(body, out),
        .Export(e) => match e {
            .Declaration(d) => hoistedFunctionStmt(d, out),
            _ => {}
        },
        _ => {}
    }
}

/// The `length` of a function: parameters before the first default or rest.
func expectedArgumentCount(params: Array[Pattern]) -> Int64 {
    var n: Int64 = 0;
    for p in params {
        match p {
            .WithDefault(_, _) => return n,
            .Rest(_) => return n,
            _ => n = n + 1
        }
    }
    n
}

/// A body that starts with a "use strict" directive.
func hasUseStrict(stmts: Array[Stmt]) -> Bool {
    for s in stmts {
        match s.kind {
            .Directive(d) => if d == "use strict" { return true; },
            _ => return false
        }
    }
    false
}

/// An anonymous function or class, which takes its name from what it's
/// assigned to: `let f = function () {}` has name "f".
func isAnonymousFunction(e: Expr) -> Bool {
    match e.kind {
        .FunctionExpr(f) => match f.id { .None => true, _ => false },
        .ArrowFunction(_) => true,
        .ClassExpr(c) => match c.id { .None => true, _ => false },
        _ => false
    }
}
