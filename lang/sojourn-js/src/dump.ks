module js

// Prints a syntax tree as S-expressions, without spans or parentheses.
// Two programs that differ only in layout, comments, grouping parentheses
// and optional semicolons print the same, which is how the test harness
// checks `pass/` files against their `pass-explicit/` twins.

func dumpProgram(p: Program) -> String {
    var out = "";
    out.append(if p.isModule { "(module" } else { "(script" });
    for s in p.body {
        out.append(" ");
        dumpStmt(s, out);
    }
    out.append(")");
    out
}

func dumpStmts(stmts: Array[Stmt], mutating out: String) {
    out.append("(");
    var first = true;
    for s in stmts {
        if not first { out.append(" "); }
        dumpStmt(s, out);
        first = false;
    }
    out.append(")");
}

func dumpOptExpr(e: Optional[Expr], mutating out: String) {
    match e {
        .Some(x) => dumpExpr(x, out),
        .None => out.append("_")
    }
}

func dumpOptName(n: Optional[String], mutating out: String) {
    match n {
        .Some(x) => dumpString(x, out),
        .None => out.append("_")
    }
}

/// A quoted string with `"`, `\` and control characters escaped.
func dumpString(s: String, mutating out: String) {
    out.append("\"");
    for c in s.chars {
        let v = c.value();
        if v == 34 { out.append("\\\""); }
        else if v == 92 { out.append("\\\\"); }
        else if v == 10 { out.append("\\n"); }
        else if v < 32 { out.append("\\x\(v)"); }
        else { out.append(char: c); }
    }
    out.append("\"");
}

func dumpStmt(s: Stmt, mutating out: String) {
    match s.kind {
        .Expression(e) => { out.append("(expr "); dumpExpr(e, out); out.append(")"); },
        .Directive(raw) => { out.append("(directive "); dumpString(raw, out); out.append(")"); },
        .Var(d) => dumpDeclaration(d, out),
        .FunctionDecl(f) => { out.append("(function-decl "); dumpFunction(f, out); out.append(")"); },
        .ClassDecl(c) => { out.append("(class-decl "); dumpClass(c, out); out.append(")"); },
        .Block(body) => { out.append("(block "); dumpStmts(body, out); out.append(")"); },
        .Empty => out.append("(empty)"),
        .If(test, consequent, alternate) => {
            out.append("(if ");
            dumpExpr(test, out);
            out.append(" ");
            dumpStmt(consequent, out);
            out.append(" ");
            match alternate {
                .Some(a) => dumpStmt(a, out),
                .None => out.append("_")
            }
            out.append(")");
        },
        .For(initializer, test, update, body) => {
            out.append("(for ");
            match initializer {
                .Some(i) => match i {
                    .Declaration(d) => dumpDeclaration(d, out),
                    .Expression(e) => dumpExpr(e, out)
                },
                .None => out.append("_")
            }
            out.append(" ");
            dumpOptExpr(test, out);
            out.append(" ");
            dumpOptExpr(update, out);
            out.append(" ");
            dumpStmt(body, out);
            out.append(")");
        },
        .ForIn(left, right, body) => {
            out.append("(for-in ");
            dumpForTarget(left, out);
            out.append(" ");
            dumpExpr(right, out);
            out.append(" ");
            dumpStmt(body, out);
            out.append(")");
        },
        .ForOf(left, right, body, isAwait) => {
            out.append(if isAwait { "(for-await-of " } else { "(for-of " });
            dumpForTarget(left, out);
            out.append(" ");
            dumpExpr(right, out);
            out.append(" ");
            dumpStmt(body, out);
            out.append(")");
        },
        .While(test, body) => { out.append("(while "); dumpExpr(test, out); out.append(" "); dumpStmt(body, out); out.append(")"); },
        .DoWhile(body, test) => { out.append("(do-while "); dumpStmt(body, out); out.append(" "); dumpExpr(test, out); out.append(")"); },
        .Return(arg) => { out.append("(return "); dumpOptExpr(arg, out); out.append(")"); },
        .Break(label) => { out.append("(break "); dumpOptName(label, out); out.append(")"); },
        .Continue(label) => { out.append("(continue "); dumpOptName(label, out); out.append(")"); },
        .Throw(e) => { out.append("(throw "); dumpExpr(e, out); out.append(")"); },
        .Try(block, handler, finalizer) => {
            out.append("(try ");
            dumpStmts(block, out);
            out.append(" ");
            match handler {
                .Some(h) => {
                    out.append("(catch ");
                    match h.param {
                        .Some(p) => dumpPattern(p, out),
                        .None => out.append("_")
                    }
                    out.append(" ");
                    dumpStmts(h.body, out);
                    out.append(")");
                },
                .None => out.append("_")
            }
            out.append(" ");
            match finalizer {
                .Some(f) => dumpStmts(f, out),
                .None => out.append("_")
            }
            out.append(")");
        },
        .Switch(discriminant, cases) => {
            out.append("(switch ");
            dumpExpr(discriminant, out);
            for c in cases {
                out.append(" (case ");
                dumpOptExpr(c.test, out);
                out.append(" ");
                dumpStmts(c.body, out);
                out.append(")");
            }
            out.append(")");
        },
        .Labeled(label, body) => { out.append("(label "); dumpString(label, out); out.append(" "); dumpStmt(body, out); out.append(")"); },
        .Debugger => out.append("(debugger)"),
        .With(object, body) => { out.append("(with "); dumpExpr(object, out); out.append(" "); dumpStmt(body, out); out.append(")"); },
        .Import(d) => {
            out.append("(import ");
            dumpString(d.source, out);
            for spec in d.specifiers {
                out.append(" ");
                match spec {
                    .Default(local) => { out.append("(default "); dumpString(local, out); out.append(")"); },
                    .Namespace(local) => { out.append("(namespace "); dumpString(local, out); out.append(")"); },
                    .Named(imported, local) => { out.append("(named "); dumpString(imported, out); out.append(" "); dumpString(local, out); out.append(")"); }
                }
            }
            dumpAttributes(d.attributes, out);
            out.append(")");
        },
        .Export(d) => {
            out.append("(export ");
            match d {
                .Declaration(s) => dumpStmt(s, out),
                .Default(s) => { out.append("default "); dumpStmt(s, out); },
                .Named(specifiers, source, attributes) => {
                    out.append("(named");
                    for spec in specifiers {
                        out.append(" ");
                        dumpString(spec.local, out);
                        out.append(" ");
                        dumpString(spec.exported, out);
                    }
                    out.append(") ");
                    dumpOptName(source, out);
                    dumpAttributes(attributes, out);
                },
                .All(exported, source, attributes) => {
                    out.append("(all ");
                    dumpOptName(exported, out);
                    out.append(") ");
                    dumpString(source, out);
                    dumpAttributes(attributes, out);
                }
            }
            out.append(")");
        }
    }
}

func dumpAttributes(attributes: Array[ImportAttribute], mutating out: String) {
    for a in attributes {
        out.append(" (with ");
        dumpString(a.key, out);
        out.append(" ");
        dumpString(a.value, out);
        out.append(")");
    }
}

func dumpForTarget(t: ForTarget, mutating out: String) {
    match t {
        .Declaration(d) => dumpDeclaration(d, out),
        .Target(p) => dumpPattern(p, out)
    }
}

func dumpDeclaration(d: VariableDeclaration, mutating out: String) {
    out.append(match d.kind {
        .Var => "(var",
        .Let => "(let",
        .Const => "(const",
        .Using => "(using",
        .AwaitUsing => "(await-using"
    });
    for decl in d.declarations {
        out.append(" (");
        dumpPattern(decl.target, out);
        out.append(" ");
        dumpOptExpr(decl.value, out);
        out.append(")");
    }
    out.append(")");
}

func dumpExpr(e: Expr, mutating out: String) {
    match e.kind {
        .Identifier(name) => { out.append("(id "); dumpString(name, out); out.append(")"); },
        .This => out.append("this"),
        .Super => out.append("super"),
        .Null => out.append("null"),
        .BoolLit(b) => out.append(if b { "true" } else { "false" }),
        .Number(n) => out.append("(num \(n))"),
        .BigInt(v) => out.append("(bigint \(v))"),
        .StringLit(s) => { out.append("(str "); dumpString(s, out); out.append(")"); },
        .Regex(pattern, flags) => { out.append("(regex "); dumpString(pattern, out); out.append(" "); dumpString(flags, out); out.append(")"); },
        .Template(t) => { out.append("(template "); dumpTemplate(t, out); out.append(")"); },
        .TaggedTemplate(tag, quasi) => { out.append("(tagged "); dumpExpr(tag, out); out.append(" "); dumpTemplate(quasi, out); out.append(")"); },
        .ArrayLit(elements) => {
            out.append("(array");
            for el in elements {
                out.append(" ");
                match el {
                    .Hole => out.append("_"),
                    .Item(x) => dumpExpr(x, out),
                    .Spread(x) => { out.append("(spread "); dumpExpr(x, out); out.append(")"); }
                }
            }
            out.append(")");
        },
        .ObjectLit(props) => {
            out.append("(object");
            for p in props {
                out.append(" ");
                match p {
                    .Init(key, value, shorthand) => {
                        out.append(if shorthand { "(shorthand " } else { "(init " });
                        dumpKey(key, out);
                        out.append(" ");
                        dumpExpr(value, out);
                        out.append(")");
                    },
                    .Method(key, kind, function) => {
                        out.append("(");
                        out.append(methodKindName(kind));
                        out.append(" ");
                        dumpKey(key, out);
                        out.append(" ");
                        dumpFunction(function, out);
                        out.append(")");
                    },
                    .Spread(x) => { out.append("(spread "); dumpExpr(x, out); out.append(")"); }
                }
            }
            out.append(")");
        },
        .FunctionExpr(f) => { out.append("(function "); dumpFunction(f, out); out.append(")"); },
        .ArrowFunction(f) => { out.append("(arrow "); dumpFunction(f, out); out.append(")"); },
        .ClassExpr(c) => { out.append("(class "); dumpClass(c, out); out.append(")"); },
        .Unary(op, argument) => { out.append("(\(op) "); dumpExpr(argument, out); out.append(")"); },
        .Update(op, prefix, argument) => {
            out.append(if prefix { "(prefix\(op) " } else { "(postfix\(op) " });
            dumpExpr(argument, out);
            out.append(")");
        },
        .Binary(op, left, right) => { out.append("(\(op) "); dumpExpr(left, out); out.append(" "); dumpExpr(right, out); out.append(")"); },
        .Logical(op, left, right) => { out.append("(\(op) "); dumpExpr(left, out); out.append(" "); dumpExpr(right, out); out.append(")"); },
        .PrivateIn(name, right) => { out.append("(private-in "); dumpString(name, out); out.append(" "); dumpExpr(right, out); out.append(")"); },
        .Assign(op, target, value) => { out.append("(\(op) "); dumpPattern(target, out); out.append(" "); dumpExpr(value, out); out.append(")"); },
        .Conditional(test, consequent, alternate) => {
            out.append("(? ");
            dumpExpr(test, out);
            out.append(" ");
            dumpExpr(consequent, out);
            out.append(" ");
            dumpExpr(alternate, out);
            out.append(")");
        },
        .Call(callee, arguments, optional) => {
            out.append(if optional { "(call?. " } else { "(call " });
            dumpExpr(callee, out);
            dumpArguments(arguments, out);
            out.append(")");
        },
        .New(callee, arguments) => { out.append("(new "); dumpExpr(callee, out); dumpArguments(arguments, out); out.append(")"); },
        .Member(object, property, optional) => {
            out.append(if optional { "(member?. " } else { "(member " });
            dumpExpr(object, out);
            out.append(" ");
            match property {
                .Name(n) => dumpString(n, out),
                .Private(n) => { out.append("#"); dumpString(n, out); },
                .Computed(x) => { out.append("["); dumpExpr(x, out); out.append("]"); }
            }
            out.append(")");
        },
        .Chain(inner) => { out.append("(chain "); dumpExpr(inner, out); out.append(")"); },
        .Sequence(items) => {
            // `(a, b), c` and `a, b, c` are the same sequence.
            out.append("(seq");
            dumpSequenceItems(items, out);
            out.append(")");
        },
        .Yield(argument, delegate) => {
            out.append(if delegate { "(yield* " } else { "(yield " });
            dumpOptExpr(argument, out);
            out.append(")");
        },
        .Await(argument) => { out.append("(await "); dumpExpr(argument, out); out.append(")"); },
        .MetaProperty(meta, property) => out.append("(meta \(meta).\(property))"),
        .Import(source, options) => { out.append("(import-call "); dumpExpr(source, out); out.append(" "); dumpOptExpr(options, out); out.append(")"); }
    }
}

func dumpSequenceItems(items: Array[Expr], mutating out: String) {
    for x in items {
        match x.kind {
            .Sequence(inner) => dumpSequenceItems(inner, out),
            _ => { out.append(" "); dumpExpr(x, out); }
        }
    }
}

func dumpArguments(arguments: Array[Argument], mutating out: String) {
    for a in arguments {
        out.append(" ");
        match a {
            .Item(x) => dumpExpr(x, out),
            .Spread(x) => { out.append("(spread "); dumpExpr(x, out); out.append(")"); }
        }
    }
}

func dumpTemplate(t: TemplateLiteral, mutating out: String) {
    for i in 0..<t.quasis.count {
        if i > 0 { out.append(" "); dumpExpr(t.expressions(i - 1), out); out.append(" "); }
        let q = t.quasis(i);
        out.append("(quasi ");
        match q.cooked {
            .Some(c) => dumpString(c, out),
            .None => out.append("_")
        }
        out.append(" ");
        dumpString(q.raw, out);
        out.append(")");
    }
}

/// Names, strings and numbers that name the same property print the
/// same: `a`, `"a"` and `'a'`; `0` and `"0"`.
func dumpKey(key: PropertyKey, mutating out: String) {
    match key {
        .Name(n) => dumpString(n, out),
        .StringKey(s) => dumpString(s, out),
        .NumberKey(n) => dumpString("\(n)", out),
        .BigIntKey(v) => out.append("(bigint \(v))"),
        .Computed(e) => { out.append("["); dumpExpr(e, out); out.append("]"); },
        .Private(n) => { out.append("#"); dumpString(n, out); }
    }
}

func methodKindName(kind: MethodKind) -> String {
    match kind {
        .Method => "method",
        .Get => "get",
        .Set => "set",
        .Constructor => "constructor"
    }
}

func dumpFunction(f: Function, mutating out: String) {
    dumpOptName(f.id, out);
    if f.isAsync { out.append(" async"); }
    if f.isGenerator { out.append(" generator"); }
    out.append(" (");
    var first = true;
    for p in f.params {
        if not first { out.append(" "); }
        dumpPattern(p, out);
        first = false;
    }
    out.append(") ");
    match f.body {
        .Block(body) => dumpStmts(body, out),
        .Expression(e) => dumpExpr(e, out)
    }
}

func dumpClass(c: Class, mutating out: String) {
    dumpOptName(c.id, out);
    out.append(" ");
    dumpOptExpr(c.superClass, out);
    for m in c.body {
        out.append(" ");
        match m {
            .Method(key, kind, isStatic, function) => {
                out.append("(");
                if isStatic { out.append("static "); }
                out.append(methodKindName(kind));
                out.append(" ");
                dumpKey(key, out);
                out.append(" ");
                dumpFunction(function, out);
                out.append(")");
            },
            .Field(key, isStatic, value) => {
                out.append(if isStatic { "(static field " } else { "(field " });
                dumpKey(key, out);
                out.append(" ");
                dumpOptExpr(value, out);
                out.append(")");
            },
            .StaticBlock(body) => { out.append("(static-block "); dumpStmts(body, out); out.append(")"); }
        }
    }
}

func dumpPattern(p: Pattern, mutating out: String) {
    match p {
        .Identifier(name) => { out.append("(id "); dumpString(name, out); out.append(")"); },
        .Expression(e) => dumpExpr(e, out),
        .ObjectPattern(props, rest) => {
            out.append("(object-pattern");
            for prop in props {
                out.append(if prop.shorthand { " (shorthand " } else { " (prop " });
                dumpKey(prop.key, out);
                out.append(" ");
                dumpPattern(prop.value, out);
                out.append(")");
            }
            match rest {
                .Some(r) => { out.append(" (rest "); dumpPattern(r, out); out.append(")"); },
                .None => {}
            }
            out.append(")");
        },
        .ArrayPattern(elements, rest) => {
            out.append("(array-pattern");
            for el in elements {
                out.append(" ");
                match el {
                    .Some(x) => dumpPattern(x, out),
                    .None => out.append("_")
                }
            }
            match rest {
                .Some(r) => { out.append(" (rest "); dumpPattern(r, out); out.append(")"); },
                .None => {}
            }
            out.append(")");
        },
        .WithDefault(target, value) => { out.append("(default "); dumpPattern(target, out); out.append(" "); dumpExpr(value, out); out.append(")"); },
        .Rest(inner) => { out.append("(rest "); dumpPattern(inner, out); out.append(")"); }
    }
}
