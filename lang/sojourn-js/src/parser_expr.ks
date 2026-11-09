module js

// Expressions, from comma sequences down to primary expressions.

extend Parser {
    /// `a, b, c`.
    mutating func parseExpression(allowIn: Bool) -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        let first = try self.parseAssignment(allowIn);
        if not self.at(",") { return .Ok(first); }
        var items: [Expr] = [first];
        while try self.eat(",") {
            items.append(try self.parseAssignment(allowIn));
        }
        .Ok(self.expr(.Sequence(items), start))
    }

    /// Like `parseExpression`, but a single expression may still hold
    /// `{ a = 1 }` shorthand defaults, for a `for` head that may turn out
    /// to be a for-in/of target.
    mutating func parseExpressionCover(allowIn: Bool) -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        let first = try self.parseMaybeAssign(allowIn);
        if not self.at(",") { return .Ok(first); }
        if self.coverInit >= 0 { throw syntaxError("invalid shorthand property initializer", at: self.coverInit); }
        var items: [Expr] = [first];
        while try self.eat(",") {
            items.append(try self.parseAssignment(allowIn));
        }
        .Ok(self.expr(.Sequence(items), start))
    }

    /// An AssignmentExpression that must be a complete expression, so any
    /// pattern-only syntax inside it is an error.
    mutating func parseAssignment(allowIn: Bool) -> Result[Expr, SyntaxError] {
        let saved = self.coverInit;
        self.coverInit = -1;
        let e = try self.parseMaybeAssign(allowIn);
        if self.coverInit >= 0 { throw syntaxError("invalid shorthand property initializer", at: self.coverInit); }
        self.coverInit = saved;
        .Ok(e)
    }

    /// An AssignmentExpression that may still become a pattern (an array
    /// element, an object value, a parenthesized item). Pattern-only syntax
    /// inside it is left recorded in `coverInit` for the caller.
    mutating func parseMaybeAssign(allowIn: Bool) -> Result[Expr, SyntaxError] {
        if self.ctx.inGenerator and self.atKeyword("yield") {
            return self.parseYield(allowIn);
        }
        let start = self.tok.start;
        let left = try self.parseConditional(allowIn);
        if self.tok.kind != TokenKind.Punct or not isAssignmentOperator(self.tok.value) {
            return .Ok(left);
        }
        if isBareArrow(left) { throw self.unexpected(); }
        let op = self.tok.value;
        let target = if op == "=" {
            let wasLiteral = isLiteralPattern(left);
            let t = try self.toAssignmentTarget(left);
            // Shorthand defaults inside the literal were patterns after all.
            if wasLiteral { self.coverInit = -1; }
            t
        } else {
            try self.toSimpleTarget(left)
        };
        try self.advance();
        let value = try self.parseAssignment(allowIn);
        .Ok(self.expr(.Assign(op: op, target: target, value: value), start))
    }

    mutating func parseYield(allowIn: Bool) -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        if self.ctx.inParameters { throw earlyError("'yield' in parameters", at: start); }
        if self.tok.escaped { throw self.error("keyword 'yield' can't contain escapes"); }
        if self.ctx.yieldPos < 0 { self.ctx.yieldPos = start; }
        try self.advance();
        // `yield` alone, before a line break or something that can't start
        // an expression.
        if self.tok.newlineBefore or self.tok.kind == TokenKind.End {
            return .Ok(self.expr(.Yield(argument: .None, delegate: false), start));
        }
        let delegate = try self.eat("*");
        if not delegate and not self.startsExpression() {
            return .Ok(self.expr(.Yield(argument: .None, delegate: false), start));
        }
        let argument = try self.parseAssignment(allowIn);
        .Ok(self.expr(.Yield(argument: .Some(argument), delegate: delegate), start))
    }

    /// The current token can begin an expression, so `yield` takes it as
    /// its argument.
    func startsExpression() -> Bool {
        if self.tok.kind == TokenKind.End or self.tok.newlineBefore { return false; }
        if self.tok.kind == TokenKind.Punct { return not cannotStartExpression(self.tok.value); }
        if self.tok.kind == TokenKind.Name and not self.tok.escaped {
            let v = self.tok.value;
            return not (v == "in" or v == "of" or v == "instanceof");
        }
        true
    }

    mutating func parseConditional(allowIn: Bool) -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        let test = try self.parseBinary(1, allowIn);
        if not self.at("?") { return .Ok(test); }
        if isBareArrow(test) { throw self.unexpected(); }
        try self.advance();
        let consequent = try self.parseAssignment(true);
        try self.expect(":");
        let alternate = try self.parseAssignment(allowIn);
        .Ok(self.expr(.Conditional(test: test, consequent: consequent, alternate: alternate), start))
    }

    /// The binary operator at the current token and its precedence, or 0.
    func binaryPrecedence(allowIn: Bool) -> Int64 {
        if self.tok.kind == TokenKind.Name {
            if self.tok.escaped { return 0; }
            if self.tok.value == "instanceof" { return 8; }
            if self.tok.value == "in" and allowIn { return 8; }
            return 0;
        }
        if self.tok.kind != TokenKind.Punct { return 0; }
        match self.tok.value {
            "??" => 1,
            "||" => 2,
            "&&" => 3,
            "|" => 4,
            "^" => 5,
            "&" => 6,
            "==" or "!=" or "===" or "!==" => 7,
            "<" or ">" or "<=" or ">=" => 8,
            "<<" or ">>" or ">>>" => 9,
            "+" or "-" => 10,
            "*" or "/" or "%" => 11,
            "**" => 12,
            _ => 0
        }
    }

    /// Binary operators at precedence `minPrec` and above.
    mutating func parseBinary(minPrec: Int64, allowIn: Bool) -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        var left = if self.tok.kind == TokenKind.PrivateName {
            // `#x in obj`
            let name = self.tok.value;
            try self.advance();
            if not self.atKeyword("in") or not allowIn or minPrec > 8 { throw self.unexpected(); }
            try self.advance();
            let right = try self.parseBinary(9, allowIn);
            self.expr(.PrivateIn(name: name, right: right), start)
        } else {
            try self.parseUnary()
        };
        while true {
            let prec = self.binaryPrecedence(allowIn);
            if prec == 0 or prec < minPrec { break; }
            if isBareArrow(left) { throw self.unexpected(); }
            let op = self.tok.value;
            if op == "**" and not left.parenthesized {
                // `-x ** 2` is ambiguous, so the spec forbids it.
                let isUnary = match left.kind { .Unary(_, _) => true, .Await(_) => true, _ => false };
                if isUnary { throw self.error("unparenthesized unary expression before '**'"); }
            }
            try self.advance();
            let right = if op == "**" { try self.parseBinary(prec, allowIn) } else { try self.parseBinary(prec + 1, allowIn) };
            if isBareArrow(right) and self.binaryPrecedence(allowIn) > 0 { throw self.unexpected(); }
            // `??` can't mix with `&&` or `||` without parentheses.
            if op == "??" and (isUnparenthesizedLogical(left, "&&", "||") or isUnparenthesizedLogical(right, "&&", "||")) {
                throw self.error("'??' mixed with '&&' or '||' needs parentheses");
            }
            if (op == "&&" or op == "||") and (isUnparenthesizedLogical(left, "??", "??") or isUnparenthesizedLogical(right, "??", "??")) {
                throw self.error("'??' mixed with '&&' or '||' needs parentheses");
            }
            let kind = if op == "&&" or op == "||" or op == "??" {
                ExprKind.Logical(op: op, left: left, right: right)
            } else {
                ExprKind.Binary(op: op, left: left, right: right)
            };
            left = self.expr(kind, start);
        }
        .Ok(left)
    }

    mutating func parseUnary() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        if self.tok.kind == TokenKind.Punct {
            let v = self.tok.value;
            if v == "!" or v == "~" or v == "+" or v == "-" {
                try self.advance();
                let argument = try self.parseUnaryOperand();
                return .Ok(self.expr(.Unary(op: v, argument: argument), start));
            }
            if v == "++" or v == "--" {
                try self.advance();
                let argStart = self.tok.start;
                let argument = try self.parseUnary();
                let _ = try self.toSimpleTargetAt(argument, argStart);
                return .Ok(self.expr(.Update(op: v, prefix: true, argument: argument), start));
            }
        }
        if self.tok.kind == TokenKind.Name and not self.tok.escaped {
            let v = self.tok.value;
            if v == "typeof" or v == "void" or v == "delete" {
                try self.advance();
                let argument = try self.parseUnaryOperand();
                if v == "delete" {
                    match argument.kind {
                        .Identifier(_) => if self.ctx.strict {
                            throw earlyError("deleting a variable in strict mode", at: start);
                        },
                        .Member(_, prop, _) => match prop {
                            .Private(_) => throw earlyError("deleting a private field", at: start),
                            _ => {}
                        },
                        .Chain(inner) => match inner.kind {
                            .Member(_, prop, _) => match prop {
                                .Private(_) => throw earlyError("deleting a private field", at: start),
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    }
                }
                return .Ok(self.expr(.Unary(op: v, argument: argument), start));
            }
            if v == "await" and self.awaitIsOperator() {
                if self.ctx.inParameters { throw earlyError("'await' in parameters", at: start); }
                if not self.ctx.inFunction { self.topLevelAwait = true; }
                try self.advance();
                let argument = try self.parseUnaryOperand();
                return .Ok(self.expr(.Await(argument), start));
            }
        }
        self.parsePostfix()
    }

    /// A unary operator's operand, which can't be an unparenthesized arrow:
    /// `typeof () => {}`.
    mutating func parseUnaryOperand() -> Result[Expr, SyntaxError] {
        let e = try self.parseUnary();
        if isBareArrow(e) { throw syntaxError("an arrow function must be parenthesized here", at: e.span.start); }
        .Ok(e)
    }

    /// `await` is an operator in async functions and at a module's top level.
    func awaitIsOperator() -> Bool {
        self.ctx.inAsync or (self.isModule and not self.ctx.inFunction and not self.ctx.inStaticBlock)
    }

    mutating func parsePostfix() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        let e = try self.parseLeftHandSide();
        if (self.at("++") or self.at("--")) and not self.tok.newlineBefore {
            if isBareArrow(e) { throw self.unexpected(); }
            let op = self.tok.value;
            let _ = try self.toSimpleTargetAt(e, start);
            try self.advance();
            return .Ok(self.expr(.Update(op: op, prefix: false, argument: e), start));
        }
        .Ok(e)
    }

    /// Calls and member accesses: `a.b(c)[d]`.
    mutating func parseLeftHandSide() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        if self.atKeyword("new") {
            let e = try self.parseNew();
            return self.parseCallTail(e, start, true);
        }
        if self.atKeyword("super") {
            let e = try self.parseSuper();
            return self.parseCallTail(e, start, true);
        }
        if self.atKeyword("import") {
            let e = try self.parseImportExpression();
            return self.parseCallTail(e, start, true);
        }
        let e = try self.parsePrimary();
        if isBareArrow(e) { return .Ok(e); }
        self.parseCallTail(e, start, true)
    }

    /// `super.x`, `super[x]` or `super(…)`; the tail parses the rest.
    mutating func parseSuper() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        if self.at("(") {
            if not self.ctx.allowSuperCall { throw earlyError("'super()' outside a derived class constructor", at: start); }
        } else if self.at(".") or self.at("[") {
            if not self.ctx.allowSuperProperty { throw earlyError("'super' outside a method", at: start); }
        } else {
            throw self.unexpected();
        }
        .Ok(self.expr(.Super, start))
    }

    /// `import.meta` or `import(…)`.
    mutating func parseImportExpression() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        if try self.eat(".") {
            if not self.tok.isKeyword("meta") { throw self.unexpected(); }
            if not self.isModule { throw self.error("'import.meta' only works in modules"); }
            try self.advance();
            return .Ok(self.expr(.MetaProperty(meta: "import", property: "meta"), start));
        }
        try self.expect("(");
        let source = try self.parseAssignment(true);
        var options: Optional[Expr] = .None;
        if try self.eat(",") {
            if not self.at(")") {
                options = .Some(try self.parseAssignment(true));
                let _ = try self.eat(",");
            }
        }
        try self.expect(")");
        .Ok(self.expr(.Import(source: source, options: options), start))
    }

    /// `new X(args)`, `new X`, `new.target`.
    mutating func parseNew() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        if try self.eat(".") {
            if not self.tok.isKeyword("target") { throw self.unexpected(); }
            if not self.ctx.allowNewTarget { throw earlyError("'new.target' outside a function", at: start); }
            try self.advance();
            return .Ok(self.expr(.MetaProperty(meta: "new", property: "target"), start));
        }
        let calleeStart = self.tok.start;
        let base = if self.atKeyword("new") {
            try self.parseNew()
        } else if self.atKeyword("super") {
            let next = try self.peek();
            if not next.isPunct(".") and not next.isPunct("[") { throw self.error("'new super()' isn't allowed"); }
            try self.parseSuper()
        } else if self.atKeyword("import") {
            // `new import.meta` is fine; `new import(…)` isn't.
            let next = try self.peek();
            if not next.isPunct(".") { throw self.error("'new import(…)' isn't allowed"); }
            try self.parseImportExpression()
        } else {
            try self.parsePrimary()
        };
        if isBareArrow(base) { throw self.unexpected(); }
        let callee = try self.parseCallTail(base, calleeStart, false);
        var arguments: [Argument] = [];
        if self.at("(") { arguments = try self.parseArguments(); }
        .Ok(self.expr(.New(callee: callee, arguments: arguments), start))
    }

    /// Member accesses, calls (when `allowCall`), optional chains and tagged
    /// templates after `e`.
    mutating func parseCallTail(e: Expr, start: Int64, allowCall: Bool) -> Result[Expr, SyntaxError] {
        var current = e;
        var inChain = false;
        while true {
            if self.at(".") {
                try self.advance();
                let prop = try self.parseMemberName(current);
                current = self.expr(.Member(object: current, property: prop, optional: false), start);
            } else if self.at("?.") {
                if not allowCall { throw self.error("optional chain in a 'new' expression"); }
                if let .Super = current.kind { throw self.unexpected(); }
                try self.advance();
                inChain = true;
                if self.at("(") {
                    let args = try self.parseArguments();
                    current = self.expr(.Call(callee: current, arguments: args, optional: true), start);
                } else if self.at("[") {
                    try self.advance();
                    let prop = try self.parseExpression(true);
                    try self.expect("]");
                    current = self.expr(.Member(object: current, property: .Computed(prop), optional: true), start);
                } else if self.tok.kind == TokenKind.Template or self.tok.kind == TokenKind.TemplateHead {
                    throw self.error("tagged template in an optional chain");
                } else {
                    let prop = try self.parseMemberName(current);
                    current = self.expr(.Member(object: current, property: prop, optional: true), start);
                }
            } else if self.at("[") {
                try self.advance();
                let prop = try self.parseExpression(true);
                try self.expect("]");
                current = self.expr(.Member(object: current, property: .Computed(prop), optional: false), start);
            } else if self.at("(") and allowCall {
                let args = try self.parseArguments();
                if let .Identifier(n) = current.kind {
                    if n == "eval" {
                        self.sawDirectEval = true;
                        self.directEvalCalls = self.directEvalCalls + 1;
                    }
                }
                current = self.expr(.Call(callee: current, arguments: args, optional: false), start);
            } else if self.tok.kind == TokenKind.Template or self.tok.kind == TokenKind.TemplateHead {
                if inChain { throw self.error("tagged template in an optional chain"); }
                let quasi = try self.parseTemplateLiteral(true);
                current = self.expr(.TaggedTemplate(tag: current, quasi: quasi), start);
            } else {
                break;
            }
        }
        if inChain {
            current = self.expr(.Chain(current), start);
        }
        .Ok(current)
    }

    /// The name after `.` or `?.`: any IdentifierName, or `#private`.
    mutating func parseMemberName(object: Expr) -> Result[MemberProperty, SyntaxError] {
        if self.tok.kind == TokenKind.PrivateName {
            if let .Super = object.kind { throw self.unexpected(); }
            let name = self.tok.value;
            try self.advance();
            return .Ok(.Private(name));
        }
        .Ok(.Name(try self.parseIdentifierName()))
    }

    /// `(a, ...b)` call arguments.
    mutating func parseArguments() -> Result[Array[Argument], SyntaxError] {
        try self.expect("(");
        var args: [Argument] = [];
        while not self.at(")") {
            if try self.eat("...") {
                args.append(.Spread(try self.parseAssignment(true)));
            } else {
                args.append(.Item(try self.parseAssignment(true)));
            }
            if not (try self.eat(",")) { break; }
        }
        try self.expect(")");
        .Ok(args)
    }

    // ------------------------------------------------------------------
    // Primary expressions
    // ------------------------------------------------------------------

    mutating func parsePrimary() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        let kind = self.tok.kind;
        if kind == TokenKind.Name {
            return self.parseNamePrimary();
        }
        if kind == TokenKind.Number {
            if self.tok.legacyOctal and self.ctx.strict { throw earlyError("octal literal in strict mode", at: start); }
            let n = self.tok.number;
            try self.advance();
            return .Ok(self.expr(.Number(n), start));
        }
        if kind == TokenKind.BigInt {
            let v = self.tok.value;
            try self.advance();
            return .Ok(self.expr(.BigInt(v), start));
        }
        if kind == TokenKind.String {
            if self.tok.legacyOctal and self.ctx.strict { throw earlyError("octal escape in strict mode", at: start); }
            let v = self.tok.value;
            try self.advance();
            return .Ok(self.expr(.StringLit(v), start));
        }
        if kind == TokenKind.Template or kind == TokenKind.TemplateHead {
            let t = try self.parseTemplateLiteral(false);
            return .Ok(self.expr(.Template(t), start));
        }
        if kind == TokenKind.Punct {
            let v = self.tok.value;
            if v == "(" { return self.parseParenthesized(); }
            if v == "[" { return self.parseArrayLiteral(); }
            if v == "{" { return self.parseObjectLiteral(); }
            if v == "/" or v == "/=" {
                self.tok = try self.lexer.rescanRegex(self.tok);
                let pattern = self.tok.value;
                let flags = self.tok.flags;
                try self.advance();
                return .Ok(self.expr(.Regex(pattern: pattern, flags: flags), start));
            }
        }
        throw self.unexpected()
    }

    /// A primary expression starting with a name: keywords like `this`,
    /// `function` and `class`, arrows, and plain identifiers.
    mutating func parseNamePrimary() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        let v = self.tok.value;
        if not self.tok.escaped {
            if v == "this" { try self.advance(); return .Ok(self.expr(.This, start)); }
            if v == "null" { try self.advance(); return .Ok(self.expr(.Null, start)); }
            if v == "true" { try self.advance(); return .Ok(self.expr(.BoolLit(true), start)); }
            if v == "false" { try self.advance(); return .Ok(self.expr(.BoolLit(false), start)); }
            if v == "function" { return self.parseFunctionExpression(start, false); }
            if v == "class" { return self.parseClassExpression(); }
            if v == "async" {
                let next = try self.peek();
                if not next.newlineBefore {
                    if next.isKeyword("function") {
                        try self.advance();
                        return self.parseFunctionExpression(start, true);
                    }
                    if next.kind == TokenKind.Name and not next.isKeyword("in") and not next.isKeyword("instanceof") {
                        // `async x => …`
                        try self.advance();
                        let paramStart = self.tok.start;
                        let saved = self.ctx.inAsync;
                        self.ctx.inAsync = true;
                        let param = self.parseBindingIdentifier();
                        self.ctx.inAsync = saved;
                        let name = try param;
                        if not self.at("=>") or self.tok.newlineBefore { throw self.unexpected(); }
                        let _ = paramStart;
                        return self.parseArrowFunction(start, [Pattern.Identifier(name)], true, false);
                    }
                    if next.isPunct("(") {
                        return self.parseAsyncCallOrArrow();
                    }
                }
            }
        }
        if not self.atIdentifier() {
            if self.tok.escaped and isAlwaysReserved(v) {
                throw self.error("keyword '\(v)' can't contain escapes");
            }
            throw self.unexpected();
        }
        let name = try self.parseIdentifier();
        if self.at("=>") and not self.tok.newlineBefore {
            try self.checkBindingName(name, start);
            return self.parseArrowFunction(start, [Pattern.Identifier(name)], false, false);
        }
        .Ok(self.expr(.Identifier(name), start))
    }

    /// `async(…)`: a call to something named `async`, or an async arrow's
    /// parameters when `=>` follows.
    mutating func parseAsyncCallOrArrow() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        let calleeEnd = self.prevEnd;
        try self.expect("(");
        var items: [Expr] = [];
        var spreads: [Bool] = [];
        var rest: Optional[Pattern] = .None;
        let savedAsync = self.ctx.inAsync;
        let evalsBefore = self.directEvalCalls;
        let savedYield = self.ctx.yieldPos;
        self.ctx.yieldPos = -1;
        while not self.at(")") {
            if try self.eat("...") {
                // `...x` is a spread argument or a rest parameter.
                let e = try self.parseMaybeAssign(true);
                items.append(e);
                spreads.append(true);
            } else {
                items.append(try self.parseMaybeAssign(true));
                spreads.append(false);
            }
            if not (try self.eat(",")) { break; }
        }
        try self.expect(")");
        let _ = savedAsync;
        let paramYield = self.ctx.yieldPos;
        if savedYield >= 0 { self.ctx.yieldPos = savedYield; }
        if self.at("=>") and not self.tok.newlineBefore {
            if paramYield >= 0 { throw earlyError("'yield' in arrow parameters", at: paramYield); }
            var params: [Pattern] = [];
            for i in 0..<items.count {
                if spreads(i) == true {
                    if i != items.count - 1 { throw self.error("rest parameter must be last"); }
                    rest = .Some(try self.toBindingPattern(items(i)));
                } else {
                    params.append(try self.toBindingElement(items(i)));
                }
            }
            for p in params {
                if patternContainsAwait(p) { throw earlyError("'await' in async arrow parameters", at: start); }
            }
            self.coverInit = -1;
            if let .Some(r) = rest {
                if patternContainsAwait(r) { throw earlyError("'await' in async arrow parameters", at: start); }
                params.append(restMarker(r));
            }
            return self.parseArrowFunction(start, params, true, self.directEvalCalls > evalsBefore);
        }
        var args: [Argument] = [];
        for i in 0..<items.count {
            if spreads(i) == true { args.append(.Spread(items(i))); } else { args.append(.Item(items(i))); }
        }
        let callee = Expr(kind: .Identifier("async"), span: Span(start: start, end: calleeEnd), parenthesized: false);
        let call = self.expr(.Call(callee: callee, arguments: args, optional: false), start);
        self.parseCallTail(call, start, true)
    }

    /// `( … )`: a parenthesized expression, or an arrow function's
    /// parameters when `=>` follows.
    mutating func parseParenthesized() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        var items: [Expr] = [];
        var rest: Optional[Pattern] = .None;
        var trailingComma = false;
        let evalsBefore = self.directEvalCalls;
        let savedYield = self.ctx.yieldPos;
        self.ctx.yieldPos = -1;
        while not self.at(")") {
            if self.at("...") {
                try self.advance();
                rest = .Some(try self.parseBindingTarget());
                if self.at("=") { throw self.error("a rest parameter can't have a default"); }
                break;
            }
            items.append(try self.parseMaybeAssign(true));
            if not self.at(",") { break; }
            try self.advance();
            if self.at(")") { trailingComma = true; }
        }
        try self.expect(")");
        let paramYield = self.ctx.yieldPos;
        if savedYield >= 0 { self.ctx.yieldPos = savedYield; }
        if self.at("=>") and not self.tok.newlineBefore {
            if paramYield >= 0 { throw earlyError("'yield' in arrow parameters", at: paramYield); }
            var params: [Pattern] = [];
            for item in items {
                params.append(try self.toBindingElement(item));
            }
            self.coverInit = -1;
            if let .Some(r) = rest { params.append(restMarker(r)); }
            return self.parseArrowFunction(start, params, false, self.directEvalCalls > evalsBefore);
        }
        let hasRest = match rest { .Some(_) => true, .None => false };
        if items.isEmpty or hasRest or trailingComma { throw self.unexpected(); }
        var e = if items.count == 1 {
            items(0)
        } else {
            let first = items(0).span.start;
            Expr(kind: .Sequence(items), span: Span(start: first, end: items(items.count - 1).span.end), parenthesized: false)
        };
        e.parenthesized = true;
        .Ok(e)
    }

    mutating func parseArrayLiteral() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        var elements: [ArrayElement] = [];
        while not self.at("]") {
            if self.at(",") {
                try self.advance();
                elements.append(.Hole);
                continue;
            }
            if try self.eat("...") {
                elements.append(.Spread(try self.parseMaybeAssign(true)));
                if self.at(",") { self.commaAfterSpread.append(start); }
            } else {
                elements.append(.Item(try self.parseMaybeAssign(true)));
            }
            if not self.at("]") { try self.expect(","); }
        }
        try self.advance();
        .Ok(self.expr(.ArrayLit(elements), start))
    }

    mutating func parseObjectLiteral() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        var props: [Property] = [];
        var protoCount = 0;
        var protoPos: Int64 = -1;
        while not self.at("}") {
            if try self.eat("...") {
                props.append(.Spread(try self.parseMaybeAssign(true)));
                if self.at(",") { self.commaAfterSpread.append(start); }
            } else {
                let propStart = self.tok.start;
                let p = try self.parseObjectProperty();
                if isProtoInit(p) {
                    protoCount = protoCount + 1;
                    if protoCount == 2 { protoPos = propStart; }
                }
                props.append(p);
            }
            if not self.at("}") { try self.expect(","); }
        }
        try self.advance();
        // A repeated `__proto__:` is only allowed in a pattern.
        if protoPos >= 0 and self.coverInit < 0 { self.coverInit = protoPos; }
        .Ok(self.expr(.ObjectLit(props), start))
    }

    /// One `key: value`, shorthand, method, getter or setter.
    mutating func parseObjectProperty() -> Result[Property, SyntaxError] {
        let start = self.tok.start;
        var isAsync = false;
        var isGenerator = false;
        var kind = MethodKind.Method;
        // `get`, `set` and `async` are modifiers only when a key follows.
        if self.tok.kind == TokenKind.Name and not self.tok.escaped and (self.tok.value == "get" or self.tok.value == "set" or self.tok.value == "async") {
            let next = try self.peek();
            let keyFollows = not (next.isPunct(",") or next.isPunct(":") or next.isPunct("(") or next.isPunct("}") or next.isPunct("="));
            if keyFollows and not (self.tok.value == "async" and next.newlineBefore) {
                if self.tok.value == "get" { kind = .Get; }
                else if self.tok.value == "set" { kind = .Set; }
                else { isAsync = true; }
                try self.advance();
            }
        }
        if try self.eat("*") {
            if kind != MethodKind.Method { throw self.unexpected(); }
            isGenerator = true;
        }
        let keyToken = self.tok;
        let key = try self.parsePropertyKey(false);
        if self.at("(") {
            let f = try self.parseMethod(kind, isAsync, isGenerator, false);
            return .Ok(.Method(key: key, kind: kind, function: f));
        }
        if kind != MethodKind.Method or isAsync or isGenerator { throw self.unexpected(); }
        if try self.eat(":") {
            let value = try self.parseMaybeAssign(true);
            return .Ok(.Init(key: key, value: value, shorthand: false));
        }
        // Shorthand `a` or `a = default`: the key must be a usable identifier.
        if keyToken.kind != TokenKind.Name { throw self.unexpected(); }
        let name = keyToken.value;
        if self.isReserved(name) {
            if keyToken.escaped and isAlwaysReserved(name) { throw syntaxError("keyword '\(name)' can't contain escapes", at: keyToken.start); }
            throw syntaxError("unexpected reserved word '\(name)'", at: keyToken.start);
        }
        if self.ctx.inFieldInit and name == "arguments" { throw earlyError("'arguments' in a class field initializer", at: start); }
        let ref = Expr(kind: .Identifier(name), span: Span(start: keyToken.start, end: keyToken.end), parenthesized: false);
        if self.at("=") {
            // Only valid if the literal becomes a pattern.
            if self.coverInit < 0 { self.coverInit = self.tok.start; }
            try self.advance();
            let value = try self.parseAssignment(true);
            let assign = self.expr(.Assign(op: "=", target: .Identifier(name), value: value), keyToken.start);
            return .Ok(.Init(key: key, value: assign, shorthand: true));
        }
        .Ok(.Init(key: key, value: ref, shorthand: true))
    }

    /// A property name: an IdentifierName, string, number, `[computed]`,
    /// or (in classes) `#private`.
    mutating func parsePropertyKey(allowPrivate: Bool) -> Result[PropertyKey, SyntaxError] {
        let kind = self.tok.kind;
        if kind == TokenKind.Name {
            return .Ok(.Name(try self.parseIdentifierName()));
        }
        if kind == TokenKind.String {
            if self.tok.legacyOctal and self.ctx.strict { throw earlyError("octal escape in strict mode", at: self.tok.start); }
            let v = self.tok.value;
            try self.advance();
            return .Ok(.StringKey(v));
        }
        if kind == TokenKind.Number {
            if self.tok.legacyOctal and self.ctx.strict { throw earlyError("octal literal in strict mode", at: self.tok.start); }
            let n = self.tok.number;
            try self.advance();
            return .Ok(.NumberKey(n));
        }
        if kind == TokenKind.BigInt {
            let v = self.tok.value;
            try self.advance();
            return .Ok(.BigIntKey(v));
        }
        if kind == TokenKind.PrivateName and allowPrivate {
            let v = self.tok.value;
            try self.advance();
            return .Ok(.Private(v));
        }
        if try self.eat("[") {
            let e = try self.parseAssignment(true);
            try self.expect("]");
            return .Ok(.Computed(e));
        }
        throw self.unexpected()
    }

    /// A template literal at a Template or TemplateHead token. A tagged
    /// template allows invalid escapes (their cooked value is undefined).
    mutating func parseTemplateLiteral(tagged: Bool) -> Result[TemplateLiteral, SyntaxError] {
        var quasis: [TemplateElement] = [];
        var expressions: [Expr] = [];
        while true {
            if self.tok.badEscape and not tagged { throw self.error("invalid escape in template"); }
            let cooked: Optional[String] = if self.tok.badEscape { .None } else { .Some(self.tok.value) };
            quasis.append(TemplateElement(cooked: cooked, raw: self.tok.raw));
            let k = self.tok.kind;
            if k == TokenKind.Template or k == TokenKind.TemplateTail {
                try self.advance();
                break;
            }
            try self.advance();
            expressions.append(try self.parseExpression(true));
            if not self.at("}") { throw self.unexpected(); }
            self.tok = try self.lexer.rescanTemplate(self.tok);
        }
        .Ok(TemplateLiteral(quasis: quasis, expressions: expressions))
    }

    // ------------------------------------------------------------------
    // Functions, arrows and classes
    // ------------------------------------------------------------------

    /// `function` expressions, after `async` when `isAsync`.
    mutating func parseFunctionExpression(start: Int64, isAsync: Bool) -> Result[Expr, SyntaxError] {
        try self.advance();
        let isGenerator = try self.eat("*");
        var name: Optional[String] = .None;
        if not self.at("(") {
            // The name follows the function's own rules for yield/await.
            let savedGen = self.ctx.inGenerator;
            let savedAsync = self.ctx.inAsync;
            self.ctx.inGenerator = isGenerator;
            self.ctx.inAsync = isAsync;
            let n = self.parseBindingIdentifier();
            self.ctx.inGenerator = savedGen;
            self.ctx.inAsync = savedAsync;
            name = .Some(try n);
        }
        let f = try self.parseFunctionRest(name, isAsync, isGenerator, .Method, false);
        .Ok(self.expr(.FunctionExpr(f), start))
    }

    /// A method's parameters and body; `kind` decides getter and setter
    /// arity, and `derived` allows `super()` in a constructor.
    mutating func parseMethod(kind: MethodKind, isAsync: Bool, isGenerator: Bool, derived: Bool) -> Result[Function, SyntaxError] {
        let start = self.tok.start;
        let saved = self.ctx;
        self.ctx.allowSuperProperty = true;
        self.ctx.allowSuperCall = kind == MethodKind.Constructor and derived;
        let f = self.parseFunctionBody(.None, isAsync, isGenerator, true, kind);
        let superProp = saved.allowSuperProperty;
        let superCall = saved.allowSuperCall;
        self.ctx.allowSuperProperty = superProp;
        self.ctx.allowSuperCall = superCall;
        let result = try f;
        if kind == MethodKind.Get and result.params.count != 0 { throw syntaxError("a getter takes no parameters", at: start); }
        if kind == MethodKind.Set {
            if result.params.count != 1 { throw syntaxError("a setter takes exactly one parameter", at: start); }
            if let .Some(_) = restOf(result.params(0)) { throw syntaxError("a setter can't have a rest parameter", at: start); }
        }
        .Ok(result)
    }

    /// Parameters and body of a plain function (not a method).
    mutating func parseFunctionRest(id: Optional[String], isAsync: Bool, isGenerator: Bool, kind: MethodKind, isMethod: Bool) -> Result[Function, SyntaxError] {
        let saved = self.ctx;
        self.ctx.allowSuperProperty = false;
        self.ctx.allowSuperCall = false;
        let f = self.parseFunctionBody(id, isAsync, isGenerator, isMethod, kind);
        self.ctx.allowSuperProperty = saved.allowSuperProperty;
        self.ctx.allowSuperCall = saved.allowSuperCall;
        f
    }

    /// `(params) { body }` with a fresh function context.
    mutating func parseFunctionBody(id: Optional[String], isAsync: Bool, isGenerator: Bool, isMethod: Bool, kind: MethodKind) -> Result[Function, SyntaxError] {
        let outer = self.ctx;
        self.ctx.inGenerator = isGenerator;
        self.ctx.inAsync = isAsync;
        self.ctx.inFunction = true;
        self.ctx.allowReturn = true;
        self.ctx.allowNewTarget = true;
        self.ctx.inStaticBlock = false;
        self.ctx.inFieldInit = false;
        self.ctx.labels = [];
        self.ctx.labelRun = 0;
        self.ctx.loopLabels = [];
        self.ctx.loopDepth = 0;
        self.ctx.switchDepth = 0;
        let result = self.parseParamsAndBody(id, isAsync, isGenerator, isMethod);
        self.ctx = outer;
        result
    }

    mutating func parseParamsAndBody(id: Optional[String], isAsync: Bool, isGenerator: Bool, isMethod: Bool) -> Result[Function, SyntaxError] {
        let outerEval = self.sawDirectEval;
        self.sawDirectEval = false;
        let paramStart = self.tok.start;
        let params = try self.parseFormalParameters();
        self.enterFunctionScope(params);
        let bodyStart = self.tok.start;
        try self.expect("{");
        var body: [Stmt] = [];
        let wasStrict = self.ctx.strict;
        let sawStrict = try self.parseDirectives(body);
        let simple = paramsAreSimple(params);
        if sawStrict and not simple {
            throw earlyError("'use strict' in a function with non-simple parameters", at: bodyStart);
        }
        // Parameters are checked against the function's final strictness
        // and kind.
        let names = boundNames(params);
        try self.checkParameterNames(names, paramStart, not simple or isMethod or self.ctx.strict);
        if self.ctx.strict and not wasStrict {
            if let .Some(n) = id {
                if n == "eval" or n == "arguments" or isStrictReserved(n) {
                    throw earlyError("invalid function name '\(n)' in strict mode", at: paramStart);
                }
            }
        }
        while not self.at("}") {
            if self.tok.kind == TokenKind.End { throw self.unexpected(); }
            body.append(try self.parseStatementListItem());
        }
        try self.advance();
        self.exitScope();
        let hasEval = self.sawDirectEval;
        self.sawDirectEval = outerEval;
        .Ok(Function(id: id, params: params, body: .Block(body), isAsync: isAsync, isGenerator: isGenerator, hasDirectEval: hasEval))
    }

    /// Bound parameter names can't repeat when `unique`, and in strict
    /// code can't be `eval`, `arguments` or strict reserved words.
    func checkParameterNames(names: Array[String], at: Int64, unique: Bool) -> Result[(), SyntaxError] {
        for i in 0..<names.count {
            let n = names(i);
            if self.ctx.strict and (n == "eval" or n == "arguments" or isStrictReserved(n)) {
                throw earlyError("invalid parameter name '\(n)' in strict mode", at: at);
            }
            if unique or self.ctx.strict {
                for j in 0..<i {
                    if names(j) == n { throw earlyError("duplicate parameter '\(n)'", at: at); }
                }
            }
        }
        .Ok(())
    }

    /// `(a, b = 1, ...rest)`. A rest parameter is stored last, wrapped by
    /// `restMarker`.
    mutating func parseFormalParameters() -> Result[Array[Pattern], SyntaxError] {
        try self.expect("(");
        let savedParams = self.ctx.inParameters;
        self.ctx.inParameters = true;
        var params: [Pattern] = [];
        while not self.at(")") {
            if try self.eat("...") {
                let r = try self.parseBindingTarget();
                if self.at("=") { throw self.error("a rest parameter can't have a default"); }
                params.append(restMarker(r));
                if not self.at(")") { throw self.error("rest parameter must be last"); }
                break;
            }
            params.append(try self.parseBindingElement());
            if not self.at(")") { try self.expect(","); }
        }
        try self.advance();
        self.ctx.inParameters = savedParams;
        .Ok(params)
    }

    /// An arrow function body after `=>`. `params` are already patterns.
    /// `paramsEval`: the parameters contain a direct eval.
    mutating func parseArrowFunction(start: Int64, params: Array[Pattern], isAsync: Bool, paramsEval: Bool) -> Result[Expr, SyntaxError] {
        try self.expect("=>");
        let outer = self.ctx;
        self.ctx.inGenerator = false;
        self.ctx.inAsync = isAsync;
        self.ctx.allowReturn = true;
        self.ctx.inParameters = false;
        self.ctx.labels = [];
        self.ctx.labelRun = 0;
        self.ctx.loopLabels = [];
        self.ctx.loopDepth = 0;
        self.ctx.switchDepth = 0;
        let result = self.parseArrowBody(start, params, isAsync, paramsEval);
        self.ctx = outer;
        result
    }

    mutating func parseArrowBody(start: Int64, params: Array[Pattern], isAsync: Bool, paramsEval: Bool) -> Result[Expr, SyntaxError] {
        let outerEval = self.sawDirectEval;
        self.sawDirectEval = paramsEval;
        let names = boundNames(params);
        if self.at("{") {
            let bodyStart = self.tok.start;
            try self.advance();
            var body: [Stmt] = [];
            let sawStrict = try self.parseDirectives(body);
            if sawStrict and not paramsAreSimple(params) {
                throw earlyError("'use strict' in a function with non-simple parameters", at: bodyStart);
            }
            try self.checkParameterNames(names, start, true);
            self.enterFunctionScope(params);
            while not self.at("}") {
                if self.tok.kind == TokenKind.End { throw self.unexpected(); }
                body.append(try self.parseStatementListItem());
            }
            try self.advance();
            self.exitScope();
            let hasEval = self.sawDirectEval;
            self.sawDirectEval = outerEval;
            let f = Function(id: .None, params: params, body: .Block(body), isAsync: isAsync, isGenerator: false, hasDirectEval: hasEval);
            return .Ok(self.expr(.ArrowFunction(f), start));
        }
        try self.checkParameterNames(names, start, true);
        // An expression body declares nothing, but functions inside it
        // still need a scope to hoist into.
        self.enterFunctionScope(params);
        let value = try self.parseAssignment(true);
        self.exitScope();
        let hasEval = self.sawDirectEval;
        self.sawDirectEval = outerEval;
        let f = Function(id: .None, params: params, body: .Expression(value), isAsync: isAsync, isGenerator: false, hasDirectEval: hasEval);
        .Ok(self.expr(.ArrowFunction(f), start))
    }

    mutating func parseClassExpression() -> Result[Expr, SyntaxError] {
        let start = self.tok.start;
        try self.advance();
        let saved = self.ctx.strict;
        self.ctx.strict = true;
        var name: Optional[String] = .None;
        if self.tok.kind == TokenKind.Name and not self.atKeyword("extends") {
            name = .Some(try self.parseBindingIdentifier());
        }
        let c = self.parseClassRest(name);
        self.ctx.strict = saved;
        .Ok(self.expr(.ClassExpr(try c), start))
    }

    /// `extends X { members }`, in strict mode.
    mutating func parseClassRest(id: Optional[String]) -> Result[Class, SyntaxError] {
        var superClass: Optional[Expr] = .None;
        if try self.eatKeyword("extends") {
            superClass = .Some(try self.parseLeftHandSide());
        }
        let derived = match superClass { .Some(_) => true, .None => false };
        try self.expect("{");
        var members: [ClassMember] = [];
        var sawConstructor = false;
        while not self.at("}") {
            if try self.eat(";") { continue; }
            let memberStart = self.tok.start;
            let member = try self.parseClassMember(derived);
            match member {
                .Method(key, kind, isStatic, _) => {
                    if kind == MethodKind.Constructor {
                        if sawConstructor { throw syntaxError("a class can have only one constructor", at: memberStart); }
                        sawConstructor = true;
                    }
                    if isStatic and keyIsName(key, "prototype") {
                        throw syntaxError("a static method can't be named 'prototype'", at: memberStart);
                    }
                },
                _ => {}
            }
            members.append(member);
        }
        try self.advance();
        .Ok(Class(id: id, superClass: superClass, body: members))
    }

    mutating func parseClassMember(derived: Bool) -> Result[ClassMember, SyntaxError] {
        let start = self.tok.start;
        var isStatic = false;
        if self.tok.isKeyword("static") {
            let next = try self.peek();
            if next.isPunct("{") {
                try self.advance();
                return self.parseStaticBlock();
            }
            if not (next.isPunct("(") or next.isPunct("=") or next.isPunct(";") or next.isPunct("}")) {
                isStatic = true;
                try self.advance();
            }
        }
        var isAsync = false;
        var isGenerator = false;
        var kind = MethodKind.Method;
        if self.tok.kind == TokenKind.Name and not self.tok.escaped and (self.tok.value == "get" or self.tok.value == "set" or self.tok.value == "async") {
            let next = try self.peek();
            let keyFollows = not (next.isPunct("(") or next.isPunct("=") or next.isPunct(";") or next.isPunct("}") or next.newlineBefore and not next.isPunct("*") and self.tok.value == "async");
            if keyFollows and not (self.tok.value == "async" and next.newlineBefore) {
                if self.tok.value == "get" { kind = .Get; }
                else if self.tok.value == "set" { kind = .Set; }
                else { isAsync = true; }
                try self.advance();
            }
        }
        if try self.eat("*") {
            if kind != MethodKind.Method { throw self.unexpected(); }
            isGenerator = true;
        }
        let keyStart = self.tok.start;
        let key = try self.parsePropertyKey(true);
        if keyIsPrivate(key, "constructor") { throw syntaxError("'#constructor' isn't allowed", at: keyStart); }
        if self.at("(") {
            let isCtor = not isStatic and keyIsName(key, "constructor");
            if isCtor {
                if kind != MethodKind.Method or isAsync or isGenerator {
                    throw syntaxError("the constructor can't be a getter, setter, generator or async", at: keyStart);
                }
                kind = .Constructor;
            }
            let f = try self.parseMethod(kind, isAsync, isGenerator, derived);
            return .Ok(.Method(key: key, kind: kind, isStatic: isStatic, function: f));
        }
        if kind != MethodKind.Method or isAsync or isGenerator { throw self.unexpected(); }
        // A field.
        if keyIsName(key, "constructor") { throw syntaxError("a field can't be named 'constructor'", at: keyStart); }
        if isStatic and keyIsName(key, "prototype") { throw syntaxError("a static field can't be named 'prototype'", at: keyStart); }
        var value: Optional[Expr] = .None;
        if try self.eat("=") {
            let outer = self.ctx;
            self.ctx.inFieldInit = true;
            self.ctx.allowSuperProperty = true;
            self.ctx.allowSuperCall = false;
            self.ctx.allowNewTarget = true;
            self.ctx.inGenerator = false;
            self.ctx.inAsync = false;
            self.ctx.allowReturn = false;
            let v = self.parseAssignment(true);
            self.ctx = outer;
            value = .Some(try v);
        }
        try self.semicolon();
        let _ = start;
        .Ok(.Field(key: key, isStatic: isStatic, value: value))
    }

    /// `static { … }`, after `static`.
    mutating func parseStaticBlock() -> Result[ClassMember, SyntaxError] {
        let outer = self.ctx;
        self.ctx.inStaticBlock = true;
        self.ctx.inFieldInit = true;
        self.ctx.inAsync = false;
        self.ctx.inGenerator = false;
        self.ctx.allowReturn = false;
        self.ctx.allowSuperProperty = true;
        self.ctx.allowSuperCall = false;
        self.ctx.allowNewTarget = true;
        self.ctx.inFunction = true;
        self.ctx.labels = [];
        self.ctx.labelRun = 0;
        self.ctx.loopLabels = [];
        self.ctx.loopDepth = 0;
        self.ctx.switchDepth = 0;
        // A static block is var-scoped like a function body.
        self.enterScope(.Function);
        let body = self.parseBlockInCurrentScope();
        self.exitScope();
        self.ctx = outer;
        .Ok(.StaticBlock(try body))
    }
}

func isAssignmentOperator(op: String) -> Bool {
    match op {
        "=" or "+=" or "-=" or "*=" or "/=" or "%=" or "**=" or "<<=" or ">>=" or ">>>=" or "&=" or "|=" or "^=" or "&&=" or "||=" or "??=" => true,
        _ => false
    }
}

/// Punctuators that can't start an expression.
func cannotStartExpression(op: String) -> Bool {
    match op {
        "==" or "!=" or "===" or "!==" or "<" or ">" or "<=" or ">=" or "<<" or ">>" or ">>>" or "*" or "%" or "**" or "&" or "|" or "^" or "&&" or "||" or "??" or "." or "?." or "?" or ":" or ")" or "]" or "}" or "," or ";" or "=>" or "=" or "+=" or "-=" or "*=" or "%=" or "**=" or "<<=" or ">>=" or ">>>=" or "&=" or "|=" or "^=" or "&&=" or "||=" or "??=" => true,
        _ => false
    }
}

/// An arrow function not wrapped in parentheses, which can't be an
/// operand: `x => {} + 1` is an error.
func isBareArrow(e: Expr) -> Bool {
    if e.parenthesized { return false; }
    match e.kind {
        .ArrowFunction(_) => true,
        _ => false
    }
}

func isUnparenthesizedLogical(e: Expr, a: String, b: String) -> Bool {
    if e.parenthesized { return false; }
    match e.kind {
        .Logical(op, _, _) => op == a or op == b,
        _ => false
    }
}

/// `__proto__: value` (not shorthand, not computed).
func isProtoInit(p: Property) -> Bool {
    match p {
        .Init(key, _, shorthand) => {
            if shorthand { return false; }
            match key {
                .Name(n) => n == "__proto__",
                .StringKey(s) => s == "__proto__",
                _ => false
            }
        },
        _ => false
    }
}

/// The key is the name or string `name`.
func keyIsName(key: PropertyKey, name: String) -> Bool {
    match key {
        .Name(n) => n == name,
        .StringKey(s) => s == name,
        _ => false
    }
}

func keyIsPrivate(key: PropertyKey, name: String) -> Bool {
    match key {
        .Private(n) => n == name,
        _ => false
    }
}
