module js

// Statements and declarations.

extend Parser {
    /// A statement, or a declaration where one is allowed (in blocks,
    /// function bodies and at the top level).
    mutating func parseStatementListItem() -> Result[Stmt, SyntaxError] {
        if self.atKeyword("function") {
            return self.parseFunctionDeclaration(self.tok.start, false);
        }
        if self.atKeyword("class") {
            return self.parseClassDeclaration();
        }
        if self.atKeyword("const") {
            return self.parseVariableStatement(.Const);
        }
        if try self.atLetDeclaration() {
            return self.parseVariableStatement(.Let);
        }
        if try self.atAsyncFunction() {
            let start = self.tok.start;
            try self.advance();
            return self.parseFunctionDeclaration(start, true);
        }
        if self.atKeyword("import") {
            let next = try self.peek();
            if not next.isPunct("(") and not next.isPunct(".") {
                if not self.isModule { throw self.error("'import' declarations only work in modules"); }
                if not self.atModuleTop() { throw self.error("'import' must be at the top level of a module"); }
                return self.parseImportDeclaration();
            }
        }
        if self.atKeyword("export") {
            if not self.isModule { throw self.error("'export' only works in modules"); }
            if not self.atModuleTop() { throw self.error("'export' must be at the top level of a module"); }
            return self.parseExportDeclaration();
        }
        self.parseStatement()
    }

    /// `let` starts a declaration when an identifier, `[` or `{` follows.
    /// Otherwise it's an identifier (outside strict mode).
    mutating func atLetDeclaration() -> Result[Bool, SyntaxError] {
        if not self.tok.isName("let") { return .Ok(false); }
        let next = try self.peek();
        if next.isPunct("[") or next.isPunct("{") { return .Ok(true); }
        if next.kind == TokenKind.Name {
            // `let` then a line break then a keyword like `in` isn't a
            // declaration; a name is (ASI doesn't split `let \n x`).
            if isAlwaysReserved(next.value) and not next.escaped {
                return .Ok(false);
            }
            return .Ok(true);
        }
        .Ok(false)
    }

    /// `async function` with no line break between them.
    mutating func atAsyncFunction() -> Result[Bool, SyntaxError] {
        if not self.atKeyword("async") { return .Ok(false); }
        let next = try self.peek();
        .Ok(next.isKeyword("function") and not next.newlineBefore)
    }

    /// A statement, where a declaration isn't allowed (the body of an `if`,
    /// a loop, a label).
    mutating func parseStatement() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        if self.tok.kind == TokenKind.Punct {
            if self.at("{") {
                let body = try self.parseBlock();
                return .Ok(self.stmt(.Block(body), start));
            }
            if self.at(";") {
                try self.advance();
                return .Ok(self.stmt(.Empty, start));
            }
        }
        if self.tok.kind == TokenKind.Name and not self.tok.escaped {
            let v = self.tok.value;
            if v == "var" { return self.parseVariableStatement(.Var); }
            if v == "if" { return self.parseIf(); }
            if v == "for" { return self.parseFor(); }
            if v == "while" { return self.parseWhile(); }
            if v == "do" { return self.parseDoWhile(); }
            if v == "return" { return self.parseReturn(); }
            if v == "break" or v == "continue" { return self.parseBreakContinue(v == "break"); }
            if v == "throw" { return self.parseThrow(); }
            if v == "try" { return self.parseTry(); }
            if v == "switch" { return self.parseSwitch(); }
            if v == "with" { return self.parseWith(); }
            if v == "debugger" {
                try self.advance();
                try self.semicolon();
                return .Ok(self.stmt(.Debugger, start));
            }
            if v == "function" { throw self.error("a function declaration can't be the body of a statement"); }
            if v == "class" { throw self.error("a class declaration can't be the body of a statement"); }
            if v == "const" { throw self.error("a lexical declaration can't be the body of a statement"); }
            if v == "let" {
                let next = try self.peek();
                if next.isPunct("[") { throw self.error("a lexical declaration can't be the body of a statement"); }
            }
            if v == "async" {
                if try self.atAsyncFunction() { throw self.error("an async function can't be the body of a statement"); }
            }
        }
        // A label: `name:`.
        if self.atIdentifier() or self.tok.isName("yield") or self.tok.isName("await") {
            let next = try self.peek();
            if next.isPunct(":") and self.atIdentifier() {
                return self.parseLabeled();
            }
        }
        self.parseExpressionStatement()
    }

    mutating func parseExpressionStatement() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        let e = try self.parseExpression(true);
        try self.semicolon();
        .Ok(self.stmt(.Expression(e), start))
    }

    /// `{ … }` in a new block scope, returning the statements.
    mutating func parseBlock() -> Result[Array[Stmt], SyntaxError] {
        self.enterScope(.Block);
        let body = try self.parseBlockInCurrentScope();
        self.exitScope();
        .Ok(body)
    }

    /// `{ … }`, declaring into the innermost scope.
    mutating func parseBlockInCurrentScope() -> Result[Array[Stmt], SyntaxError] {
        try self.expect("{");
        var body: [Stmt] = [];
        while not self.at("}") {
            if self.tok.kind == TokenKind.End { throw self.unexpected(); }
            body.append(try self.parseStatementListItem());
        }
        try self.advance();
        .Ok(body)
    }

    // ------------------------------------------------------------------
    // Variables
    // ------------------------------------------------------------------

    mutating func parseVariableStatement(kind: VariableKind) -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        let decl = try self.parseDeclarators(kind, true);
        try self.declareDeclaration(decl, start, false);
        try self.semicolon();
        .Ok(self.stmt(.Var(decl), start))
    }

    /// The `a = 1, b` after `var`/`let`/`const`. `inFor` is false at the
    /// head of a `for`, where `in` is an operator only inside parentheses
    /// and initializers may be missing (checked by the caller).
    mutating func parseDeclarators(kind: VariableKind, allowIn: Bool) -> Result[VariableDeclaration, SyntaxError] {
        var decls: [VariableDeclarator] = [];
        while true {
            let start = self.tok.start;
            if kind != VariableKind.Var and self.tok.isName("let") {
                throw earlyError("'let' can't be a lexically bound name", at: start);
            }
            let target = try self.parseBindingTarget();
            var value: Optional[Expr] = .None;
            if try self.eat("=") {
                value = .Some(try self.parseAssignment(allowIn));
            } else if allowIn {
                // Outside a `for` head, `const` and destructuring need a value.
                let isPattern = match target { .Identifier(_) => false, _ => true };
                if kind == VariableKind.Const { throw self.error("missing initializer in const declaration"); }
                if isPattern { throw self.error("missing initializer in destructuring declaration"); }
            }
            decls.append(VariableDeclarator(target: target, value: value));
            if not (try self.eat(",")) { break; }
        }
        .Ok(VariableDeclaration(kind: kind, declarations: decls))
    }

    // ------------------------------------------------------------------
    // Control flow
    // ------------------------------------------------------------------

    mutating func parseIf() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        try self.expect("(");
        let test = try self.parseExpression(true);
        try self.expect(")");
        let consequent = try self.parseIfBody();
        var alternate: Optional[Stmt] = .None;
        if try self.eatKeyword("else") {
            alternate = .Some(try self.parseIfBody());
        }
        .Ok(self.stmt(.If(test: test, consequent: consequent, alternate: alternate), start))
    }

    /// The body of an `if`, which in sloppy mode may be a function
    /// declaration (Annex B).
    mutating func parseIfBody() -> Result[Stmt, SyntaxError] {
        if self.atKeyword("function") and not self.ctx.strict {
            let next = try self.peek();
            if next.isPunct("*") { throw self.error("a generator can't be the body of an if"); }
            self.enterScope(.Block);
            let f = try self.parseFunctionDeclaration(self.tok.start, false);
            self.exitScope();
            return .Ok(f);
        }
        self.parseSubstatement()
    }

    /// The body of an `if`, a loop or a `with`, which can't be a labelled
    /// function declaration.
    mutating func parseSubstatement() -> Result[Stmt, SyntaxError] {
        let s = try self.parseStatement();
        if isLabelledFunction(s) { throw syntaxError("a labelled function can't be the body of a statement", at: s.span.start); }
        .Ok(s)
    }

    /// Parses a loop body with the loop counted for `break`/`continue`.
    mutating func parseLoopBody() -> Result[Stmt, SyntaxError] {
        self.ctx.loopDepth = self.ctx.loopDepth + 1;
        let body = self.parseSubstatement();
        self.ctx.loopDepth = self.ctx.loopDepth - 1;
        body
    }

    mutating func parseWhile() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        try self.expect("(");
        let test = try self.parseExpression(true);
        try self.expect(")");
        let body = try self.parseLoopBody();
        .Ok(self.stmt(.While(test: test, body: body), start))
    }

    mutating func parseDoWhile() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        let body = try self.parseLoopBody();
        try self.expectKeyword("while");
        try self.expect("(");
        let test = try self.parseExpression(true);
        try self.expect(")");
        // A `;` after `do … while (…)` is optional even without a line break.
        let _ = try self.eat(";");
        .Ok(self.stmt(.DoWhile(body: body, test: test), start))
    }

    /// A `for` loop, in a scope of its own for `let`/`const` heads.
    mutating func parseFor() -> Result[Stmt, SyntaxError] {
        self.enterScope(.Block);
        let result = try self.parseForInScope();
        self.exitScope();
        .Ok(result)
    }

    mutating func parseForInScope() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        var isAwait = false;
        if self.atKeyword("await") {
            if not self.ctx.inAsync and not (self.isModule and not self.ctx.inFunction) {
                throw self.unexpected();
            }
            isAwait = true;
            if not self.ctx.inFunction { self.topLevelAwait = true; }
            try self.advance();
        }
        try self.expect("(");

        // `for (;` — no initializer.
        if self.at(";") {
            if isAwait { throw self.unexpected(); }
            return self.parseForRest(start, .None);
        }

        // A declaration: `var`, `let`, `const`.
        let initStart = self.tok.start;
        var declKind: Optional[VariableKind] = .None;
        if self.atKeyword("var") {
            declKind = .Some(.Var);
        } else if self.atKeyword("const") {
            declKind = .Some(.Const);
        } else if self.tok.isName("let") {
            let next = try self.peek();
            if next.isPunct("[") or next.isPunct("{") or (next.kind == TokenKind.Name and not (next.isKeyword("in") or next.isKeyword("of"))) {
                declKind = .Some(.Let);
            }
        }
        if let .Some(kind) = declKind {
            try self.advance();
            let decl = try self.parseDeclarators(kind, false);
            if self.atKeyword("of") or self.atKeyword("in") {
                let isOf = self.atKeyword("of");
                if decl.declarations.count != 1 {
                    throw self.error("only one variable allowed in a for-\(if isOf { "of" } else { "in" }) loop");
                }
                let d = decl.declarations(0);
                if let .Some(_) = d.value {
                    // Annex B allows `for (var a = 1 in b)` in sloppy mode only.
                    let simpleVar = match d.target { .Identifier(_) => kind == VariableKind.Var, _ => false };
                    if isOf or self.ctx.strict or not simpleVar {
                        throw self.error("a for-in or for-of variable can't have an initializer");
                    }
                }
                try self.declareDeclaration(decl, initStart, isOf);
                return self.parseForInOf(start, .Declaration(decl), isOf, isAwait);
            }
            // A plain `for`: const and patterns need initializers.
            for d in decl.declarations {
                let hasValue = match d.value { .Some(_) => true, .None => false };
                let isPattern = match d.target { .Identifier(_) => false, _ => true };
                if not hasValue and (kind == VariableKind.Const or isPattern) {
                    throw self.error("missing initializer in declaration");
                }
            }
            if isAwait { throw self.unexpected(); }
            try self.declareDeclaration(decl, initStart, false);
            return self.parseForRest(start, .Some(.Declaration(decl)));
        }

        // An expression, which may turn out to be a for-in/of target.
        let startsWithLet = self.tok.isName("let");
        let startsWithAsync = self.tok.isKeyword("async");
        let savedCover = self.coverInit;
        self.coverInit = -1;
        let head = try self.parseExpressionCover(false);
        if self.atKeyword("of") or self.atKeyword("in") {
            let isOf = self.atKeyword("of");
            if isOf and startsWithLet { throw self.error("'for (let of …)' is ambiguous"); }
            if isOf and startsWithAsync and not isAwait and not head.parenthesized {
                if let .Identifier(_) = head.kind { throw self.error("'for (async of …)' is ambiguous"); }
            }
            let target = try self.toAssignmentTarget(head);
            self.coverInit = savedCover;
            return self.parseForInOf(start, .Target(target), isOf, isAwait);
        }
        if self.coverInit >= 0 { throw syntaxError("invalid shorthand property initializer", at: self.coverInit); }
        self.coverInit = savedCover;
        if isAwait { throw self.unexpected(); }
        self.parseForRest(start, .Some(.Expression(head)))
    }

    /// The `; test; update) body` of a plain `for`.
    mutating func parseForRest(start: Int64, setup: Optional[ForInit]) -> Result[Stmt, SyntaxError] {
        try self.expect(";");
        var test: Optional[Expr] = .None;
        if not self.at(";") { test = .Some(try self.parseExpression(true)); }
        try self.expect(";");
        var update: Optional[Expr] = .None;
        if not self.at(")") { update = .Some(try self.parseExpression(true)); }
        try self.expect(")");
        let body = try self.parseLoopBody();
        .Ok(self.stmt(.For(initializer: setup, test: test, update: update, body: body), start))
    }

    mutating func parseForInOf(start: Int64, left: ForTarget, isOf: Bool, isAwait: Bool) -> Result[Stmt, SyntaxError] {
        try self.advance();
        if isAwait and not isOf { throw self.error("'for await' needs 'of'"); }
        let right = if isOf { try self.parseAssignment(true) } else { try self.parseExpression(true) };
        try self.expect(")");
        let body = try self.parseLoopBody();
        if isOf {
            return .Ok(self.stmt(.ForOf(left: left, right: right, body: body, isAwait: isAwait), start));
        }
        .Ok(self.stmt(.ForIn(left: left, right: right, body: body), start))
    }

    mutating func parseReturn() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        if not self.ctx.allowReturn { throw self.error("'return' outside a function"); }
        try self.advance();
        var argument: Optional[Expr] = .None;
        // `return` is a restricted production: a line break ends it.
        if not self.at(";") and not self.at("}") and self.tok.kind != TokenKind.End and not self.tok.newlineBefore {
            argument = .Some(try self.parseExpression(true));
        }
        try self.semicolon();
        .Ok(self.stmt(.Return(argument), start))
    }

    mutating func parseBreakContinue(isBreak: Bool) -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        var label: Optional[String] = .None;
        if self.tok.kind == TokenKind.Name and not self.tok.newlineBefore and not self.atKeyword("in") {
            let labelStart = self.tok.start;
            let name = try self.parseIdentifier();
            // The label must be in scope, and `continue` needs a loop label.
            var found = false;
            for i in 0..<self.ctx.labels.count {
                if self.ctx.labels(i) == name and (isBreak or self.ctx.loopLabels(i)) { found = true; }
            }
            if not found { throw earlyError("undefined label '\(name)'", at: labelStart); }
            label = .Some(name);
        } else if isBreak {
            if self.ctx.loopDepth == 0 and self.ctx.switchDepth == 0 {
                throw earlyError("'break' outside a loop or switch", at: start);
            }
        } else if self.ctx.loopDepth == 0 {
            throw earlyError("'continue' outside a loop", at: start);
        }
        try self.semicolon();
        if isBreak { return .Ok(self.stmt(.Break(label), start)); }
        .Ok(self.stmt(.Continue(label), start))
    }

    mutating func parseThrow() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        if self.tok.newlineBefore { throw self.error("line break after 'throw'"); }
        let argument = try self.parseExpression(true);
        try self.semicolon();
        .Ok(self.stmt(.Throw(argument), start))
    }

    mutating func parseTry() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        let block = try self.parseBlock();
        var handler: Optional[CatchClause] = .None;
        if try self.eatKeyword("catch") {
            var param: Optional[Pattern] = .None;
            self.enterScope(.Catch);
            if try self.eat("(") {
                let paramStart = self.tok.start;
                let p = try self.parseBindingTarget();
                try self.declareCatchParameter(p, paramStart);
                param = .Some(p);
                try self.expect(")");
            }
            let body = try self.parseBlockInCurrentScope();
            self.exitScope();
            handler = .Some(CatchClause(param: param, body: body));
        }
        var finalizer: Optional[Array[Stmt]] = .None;
        if try self.eatKeyword("finally") {
            finalizer = .Some(try self.parseBlock());
        }
        let hasHandler = match handler { .Some(_) => true, .None => false };
        let hasFinalizer = match finalizer { .Some(_) => true, .None => false };
        if not hasHandler and not hasFinalizer { throw self.error("'try' needs 'catch' or 'finally'"); }
        .Ok(self.stmt(.Try(block: block, handler: handler, finalizer: finalizer), start))
    }

    mutating func parseSwitch() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        try self.expect("(");
        let discriminant = try self.parseExpression(true);
        try self.expect(")");
        try self.expect("{");
        self.ctx.switchDepth = self.ctx.switchDepth + 1;
        self.enterScope(.Block);
        var cases: [SwitchCase] = [];
        var sawDefault = false;
        while not self.at("}") {
            var test: Optional[Expr] = .None;
            if try self.eatKeyword("case") {
                test = .Some(try self.parseExpression(true));
            } else if self.atKeyword("default") {
                if sawDefault { throw self.error("more than one 'default' in a switch"); }
                sawDefault = true;
                try self.advance();
            } else {
                throw self.unexpected();
            }
            try self.expect(":");
            var body: [Stmt] = [];
            while not self.at("}") and not self.atKeyword("case") and not self.atKeyword("default") {
                if self.tok.kind == TokenKind.End { throw self.unexpected(); }
                body.append(try self.parseStatementListItem());
            }
            cases.append(SwitchCase(test: test, body: body));
        }
        try self.advance();
        self.exitScope();
        self.ctx.switchDepth = self.ctx.switchDepth - 1;
        .Ok(self.stmt(.Switch(discriminant: discriminant, cases: cases), start))
    }

    mutating func parseWith() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        if self.ctx.strict { throw earlyError("'with' in strict mode", at: start); }
        try self.advance();
        try self.expect("(");
        let object = try self.parseExpression(true);
        try self.expect(")");
        let body = try self.parseSubstatement();
        .Ok(self.stmt(.With(object: object, body: body), start))
    }

    mutating func parseLabeled() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        let label = try self.parseIdentifier();
        try self.expect(":");
        for l in self.ctx.labels {
            if l == label { throw earlyError("label '\(label)' already declared", at: start); }
        }
        // Whether the label names a loop decides if `continue label` works.
        // Labels chained right before it (`a: b: while`) name it too.
        let isLoop = self.atKeyword("for") or self.atKeyword("while") or self.atKeyword("do");
        self.ctx.labels.append(label);
        self.ctx.loopLabels.append(false);
        if isLoop {
            let n = self.ctx.labels.count;
            for i in (n - 1 - self.ctx.labelRun)..<n { self.ctx.loopLabels(i) = true; }
        }
        let savedRun = self.ctx.labelRun;
        var nextIsLabel = false;
        if self.atIdentifier() {
            nextIsLabel = (try self.peek()).isPunct(":");
        }
        self.ctx.labelRun = if nextIsLabel { savedRun + 1 } else { 0 };
        let body = if self.atKeyword("function") {
            if self.ctx.strict { throw self.error("a labeled function declaration in strict mode"); }
            let next = try self.peek();
            if next.isPunct("*") { throw self.error("a labeled generator"); }
            self.parseFunctionDeclaration(self.tok.start, false)
        } else {
            self.parseStatement()
        };
        self.ctx.labelRun = savedRun;
        let _ = self.ctx.labels.pop();
        let _ = self.ctx.loopLabels.pop();
        .Ok(self.stmt(.Labeled(label: label, body: try body), start))
    }

    // ------------------------------------------------------------------
    // Functions and classes
    // ------------------------------------------------------------------

    /// `function name(…) { … }` at `function`, after `async` if `isAsync`.
    mutating func parseFunctionDeclaration(start: Int64, isAsync: Bool) -> Result[Stmt, SyntaxError] {
        try self.advance();
        let isGenerator = try self.eat("*");
        // The name follows the enclosing context's rules for yield/await.
        let nameStart = self.tok.start;
        let name = try self.parseBindingIdentifier();
        try self.declareFunction(name, nameStart, not isAsync and not isGenerator);
        let f = try self.parseFunctionRest(.Some(name), isAsync, isGenerator, .Method, false);
        .Ok(self.stmt(.FunctionDecl(f), start))
    }

    mutating func parseClassDeclaration() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        let saved = self.ctx.strict;
        self.ctx.strict = true;
        let nameStart = self.tok.start;
        let name = try self.parseBindingIdentifier();
        try self.declareLexical(name, nameStart);
        let c = try self.parseClassRest(.Some(name));
        self.ctx.strict = saved;
        .Ok(self.stmt(.ClassDecl(c), start))
    }

    // ------------------------------------------------------------------
    // Modules
    // ------------------------------------------------------------------

    mutating func parseImportDeclaration() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        var specifiers: [ImportSpecifier] = [];
        if self.tok.kind == TokenKind.String {
            // `import "m";`
            let source = self.tok.value;
            try self.advance();
            let attributes = try self.parseImportAttributes();
            try self.semicolon();
            return .Ok(self.stmt(.Import(ImportDeclaration(specifiers: [], source: source, attributes: attributes)), start));
        }
        if self.tok.kind == TokenKind.Name and not self.at("{") {
            specifiers.append(.Default(local: try self.parseBindingIdentifier()));
            if not (try self.eat(",")) {
                return self.finishImport(start, specifiers);
            }
        }
        if try self.eat("*") {
            if not self.tok.isKeyword("as") { throw self.unexpected(); }
            try self.advance();
            specifiers.append(.Namespace(local: try self.parseBindingIdentifier()));
        } else if try self.eat("{") {
            while not self.at("}") {
                let imported = try self.parseModuleExportName();
                if self.tok.isKeyword("as") {
                    try self.advance();
                    specifiers.append(.Named(imported: imported.name, local: try self.parseBindingIdentifier()));
                } else {
                    // Without `as`, the name must be a valid binding.
                    if imported.isString or self.isReserved(imported.name) { throw self.unexpected(); }
                    try self.checkBindingName(imported.name, self.tok.start);
                    specifiers.append(.Named(imported: imported.name, local: imported.name));
                }
                if not (try self.eat(",")) { break; }
            }
            try self.expect("}");
        } else {
            throw self.unexpected();
        }
        self.finishImport(start, specifiers)
    }

    mutating func finishImport(start: Int64, specifiers: Array[ImportSpecifier]) -> Result[Stmt, SyntaxError] {
        if not self.tok.isKeyword("from") { throw self.unexpected(); }
        try self.advance();
        if self.tok.kind != TokenKind.String { throw self.unexpected(); }
        let source = self.tok.value;
        try self.advance();
        let attributes = try self.parseImportAttributes();
        try self.semicolon();
        for sp in specifiers {
            let local = match sp { .Default(l) => l, .Namespace(l) => l, .Named(_, l) => l };
            try self.declareLexical(local, start);
        }
        .Ok(self.stmt(.Import(ImportDeclaration(specifiers: specifiers, source: source, attributes: attributes)), start))
    }

    /// `with { type: "json" }` after a module specifier.
    mutating func parseImportAttributes() -> Result[Array[ImportAttribute], SyntaxError] {
        var out: [ImportAttribute] = [];
        if not self.atKeyword("with") { return .Ok(out); }
        try self.advance();
        try self.expect("{");
        while not self.at("}") {
            let key = if self.tok.kind == TokenKind.String { self.tok.value } else if self.tok.kind == TokenKind.Name { self.tok.value } else { throw self.unexpected() };
            try self.advance();
            try self.expect(":");
            if self.tok.kind != TokenKind.String { throw self.unexpected(); }
            out.append(ImportAttribute(key: key, value: self.tok.value));
            try self.advance();
            if not (try self.eat(",")) { break; }
        }
        try self.expect("}");
        .Ok(out)
    }

    /// An IdentifierName or a string, in import and export lists.
    mutating func parseModuleExportName() -> Result[ExportName, SyntaxError] {
        if self.tok.kind == TokenKind.String {
            let name = self.tok.value;
            let u = unitsOf(name);
            for i in 0..<u.count {
                if isLoneSurrogate(u, i) { throw self.error("A module export name can't contain a lone surrogate"); }
            }
            try self.advance();
            return .Ok(ExportName(name: name, isString: true));
        }
        .Ok(ExportName(name: try self.parseIdentifierName(), isString: false))
    }

    mutating func parseExportDeclaration() -> Result[Stmt, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        if try self.eat("*") {
            var exported: Optional[String] = .None;
            if self.tok.isKeyword("as") {
                try self.advance();
                let nameStart = self.tok.start;
                let name = (try self.parseModuleExportName()).name;
                try self.declareExport(name, nameStart);
                exported = .Some(name);
            }
            if not self.tok.isKeyword("from") { throw self.unexpected(); }
            try self.advance();
            if self.tok.kind != TokenKind.String { throw self.unexpected(); }
            let source = self.tok.value;
            try self.advance();
            let attributes = try self.parseImportAttributes();
            try self.semicolon();
            return .Ok(self.stmt(.Export(.All(exported: exported, source: source, attributes: attributes)), start));
        }
        if self.atKeyword("default") {
            try self.declareExport("default", self.tok.start);
            try self.advance();
            let declStart = self.tok.start;
            let asyncFunction = try self.atAsyncFunction();
            if self.atKeyword("function") or asyncFunction {
                let isAsync = self.atKeyword("async");
                if isAsync { try self.advance(); }
                try self.advance();
                let isGenerator = try self.eat("*");
                var name: Optional[String] = .None;
                if not self.at("(") {
                    let nameStart = self.tok.start;
                    let n = try self.parseBindingIdentifier();
                    try self.declareFunction(n, nameStart, not isAsync and not isGenerator);
                    name = .Some(n);
                }
                let f = try self.parseFunctionRest(name, isAsync, isGenerator, .Method, false);
                return .Ok(self.stmt(.Export(.Default(self.stmt(.FunctionDecl(f), declStart))), start));
            }
            if self.atKeyword("class") {
                try self.advance();
                let saved = self.ctx.strict;
                self.ctx.strict = true;
                var name: Optional[String] = .None;
                if self.tok.kind == TokenKind.Name and not self.atKeyword("extends") {
                    let nameStart = self.tok.start;
                    let n = try self.parseBindingIdentifier();
                    try self.declareLexical(n, nameStart);
                    name = .Some(n);
                }
                let c = try self.parseClassRest(name);
                self.ctx.strict = saved;
                return .Ok(self.stmt(.Export(.Default(self.stmt(.ClassDecl(c), declStart))), start));
            }
            let e = try self.parseAssignment(true);
            try self.semicolon();
            return .Ok(self.stmt(.Export(.Default(self.stmt(.Expression(e), declStart))), start));
        }
        if try self.eat("{") {
            var specifiers: [ExportSpecifier] = [];
            var localPositions: [Int64] = [];
            var localsAreStrings = false;
            var firstBadLocal: Int64 = -1;
            while not self.at("}") {
                let localStart = self.tok.start;
                let local = try self.parseModuleExportName();
                if local.isString { localsAreStrings = true; }
                if not local.isString and isAlwaysReserved(local.name) and firstBadLocal < 0 { firstBadLocal = localStart; }
                var exported = local.name;
                var exportedStart = localStart;
                if self.tok.isKeyword("as") {
                    try self.advance();
                    exportedStart = self.tok.start;
                    exported = (try self.parseModuleExportName()).name;
                }
                try self.declareExport(exported, exportedStart);
                localPositions.append(localStart);
                specifiers.append(ExportSpecifier(local: local.name, exported: exported));
                if not (try self.eat(",")) { break; }
            }
            try self.expect("}");
            var source: Optional[String] = .None;
            var attributes: [ImportAttribute] = [];
            if self.tok.isKeyword("from") {
                try self.advance();
                if self.tok.kind != TokenKind.String { throw self.unexpected(); }
                source = .Some(self.tok.value);
                try self.advance();
                attributes = try self.parseImportAttributes();
            } else if localsAreStrings or firstBadLocal >= 0 {
                // Without `from`, each local must be a reference.
                throw syntaxError("exporting a name that isn't a local binding", at: if firstBadLocal >= 0 { firstBadLocal } else { start });
            } else {
                for i in 0..<specifiers.count {
                    self.exportedLocals.append(specifiers(i).local);
                    self.exportedLocalPositions.append(localPositions(i));
                }
            }
            try self.semicolon();
            return .Ok(self.stmt(.Export(.Named(specifiers: specifiers, source: source, attributes: attributes)), start));
        }
        // `export var|let|const|function|class|async function`.
        let letDeclaration = try self.atLetDeclaration();
        let asyncFunction = try self.atAsyncFunction();
        let isDeclaration = self.atKeyword("var") or self.atKeyword("const") or self.atKeyword("function")
            or self.atKeyword("class") or letDeclaration or asyncFunction;
        if not isDeclaration { throw self.unexpected(); }
        let decl = try self.parseStatementListItem();
        for name in declaredNames(decl) { try self.declareExport(name, start); }
        .Ok(self.stmt(.Export(.Declaration(decl)), start))
    }
}

struct ExportName {
    var name: String
    var isString: Bool
}
