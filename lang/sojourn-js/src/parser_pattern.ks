module js

// Binding patterns, and turning expressions into patterns.
//
// `[a, b] = c` and `(a, b) => c` start out looking like an array literal
// and a parenthesized expression; only the `=` or `=>` after them shows
// they were patterns. The parser reads them as expressions (the spec's
// "cover grammar") and converts them here, rejecting expressions that
// have no pattern form.

extend Parser {
    /// A binding target: a name, `[…]` or `{…}`.
    mutating func parseBindingTarget() -> Result[Pattern, SyntaxError] {
        if self.at("[") { return self.parseArrayBindingPattern(); }
        if self.at("{") { return self.parseObjectBindingPattern(); }
        .Ok(.Identifier(try self.parseBindingIdentifier()))
    }

    /// A binding target with an optional `= default`.
    mutating func parseBindingElement() -> Result[Pattern, SyntaxError] {
        let target = try self.parseBindingTarget();
        if try self.eat("=") {
            let value = try self.parseAssignment(true);
            return .Ok(.WithDefault(target: target, value: value));
        }
        .Ok(target)
    }

    mutating func parseArrayBindingPattern() -> Result[Pattern, SyntaxError] {
        try self.advance();
        var elements: [Optional[Pattern]] = [];
        var rest: Optional[Pattern] = .None;
        while not self.at("]") {
            if try self.eat(",") {
                elements.append(.None);
                continue;
            }
            if try self.eat("...") {
                rest = .Some(try self.parseBindingTarget());
                if not self.at("]") { throw self.error("rest element must be last"); }
                break;
            }
            elements.append(.Some(try self.parseBindingElement()));
            if not self.at("]") { try self.expect(","); }
        }
        try self.advance();
        .Ok(.ArrayPattern(elements: elements, rest: rest))
    }

    mutating func parseObjectBindingPattern() -> Result[Pattern, SyntaxError] {
        try self.advance();
        var props: [PatternProperty] = [];
        var rest: Optional[Pattern] = .None;
        while not self.at("}") {
            if try self.eat("...") {
                rest = .Some(.Identifier(try self.parseBindingIdentifier()));
                if not self.at("}") { throw self.error("rest element must be last"); }
                break;
            }
            let keyToken = self.tok;
            let key = try self.parsePropertyKey(false);
            if try self.eat(":") {
                props.append(PatternProperty(key: key, value: try self.parseBindingElement(), shorthand: false));
            } else {
                // Shorthand: the key is also the bound name.
                if keyToken.kind != TokenKind.Name { throw syntaxError("expected ':'", at: self.tok.start); }
                let name = keyToken.value;
                if self.isReserved(name) { throw syntaxError("unexpected reserved word '\(name)'", at: keyToken.start); }
                try self.checkBindingName(name, keyToken.start);
                var value = Pattern.Identifier(name);
                if try self.eat("=") {
                    value = .WithDefault(target: value, value: try self.parseAssignment(true));
                }
                props.append(PatternProperty(key: key, value: value, shorthand: true));
            }
            if not self.at("}") { try self.expect(","); }
        }
        try self.advance();
        .Ok(.ObjectPattern(properties: props, rest: rest))
    }

    // ------------------------------------------------------------------
    // Expressions to patterns
    // ------------------------------------------------------------------

    /// The left side of `=` or of a for-in/of.
    mutating func toAssignmentTarget(e: Expr) -> Result[Pattern, SyntaxError] {
        self.convertTarget(e, false)
    }

    /// An arrow function parameter, which may have a default.
    mutating func toBindingElement(e: Expr) -> Result[Pattern, SyntaxError] {
        self.convertElement(e, true)
    }

    /// An arrow function's rest parameter.
    mutating func toBindingPattern(e: Expr) -> Result[Pattern, SyntaxError] {
        self.convertTarget(e, true)
    }

    /// Converts an element that may be `target = default`.
    mutating func convertElement(e: Expr, binding: Bool) -> Result[Pattern, SyntaxError] {
        if not e.parenthesized {
            if let .Assign(op, target, value) = e.kind {
                if op != "=" { throw syntaxError("invalid destructuring target", at: e.span.start); }
                if binding { try self.validateBinding(target, e.span.start); }
                // `{eval = 1}` was read as a shorthand default without the
                // strict-mode check a plain assignment gets.
                if let .Identifier(name) = target {
                    if self.ctx.strict and (name == "eval" or name == "arguments") {
                        throw earlyError("can't assign to '\(name)' in strict mode", at: e.span.start);
                    }
                }
                return .Ok(.WithDefault(target: target, value: value));
            }
        }
        self.convertTarget(e, binding)
    }

    mutating func convertTarget(e: Expr, binding: Bool) -> Result[Pattern, SyntaxError] {
        let at = e.span.start;
        match e.kind {
            .Identifier(name) => {
                if binding and e.parenthesized { throw syntaxError("a parameter can't be parenthesized", at: at); }
                if binding { try self.checkBindingName(name, at); }
                if self.ctx.strict and (name == "eval" or name == "arguments") {
                    throw earlyError("can't assign to '\(name)' in strict mode", at: at);
                }
                return .Ok(.Identifier(name));
            },
            .Member(_, _, optional) => {
                if binding or optional { throw syntaxError("invalid destructuring target", at: at); }
                return .Ok(.Expression(e));
            },
            .ArrayLit(elements) => {
                if e.parenthesized or self.commaAfterSpread.contains(at) {
                    throw syntaxError("invalid destructuring target", at: at);
                }
                var out: [Optional[Pattern]] = [];
                var rest: Optional[Pattern] = .None;
                for i in 0..<elements.count {
                    match elements(i) {
                        .Hole => out.append(.None),
                        .Item(x) => out.append(.Some(try self.convertElement(x, binding))),
                        .Spread(x) => {
                            if i != elements.count - 1 { throw syntaxError("rest element must be last", at: x.span.start); }
                            rest = .Some(try self.convertRest(x, binding));
                        }
                    }
                }
                return .Ok(.ArrayPattern(elements: out, rest: rest));
            },
            .ObjectLit(props) => {
                if e.parenthesized or self.commaAfterSpread.contains(at) {
                    throw syntaxError("invalid destructuring target", at: at);
                }
                var out: [PatternProperty] = [];
                var rest: Optional[Pattern] = .None;
                for i in 0..<props.count {
                    match props(i) {
                        .Init(key, value, shorthand) => {
                            out.append(PatternProperty(key: key, value: try self.convertElement(value, binding), shorthand: shorthand));
                        },
                        .Method(_, _, _) => throw syntaxError("a method can't be a destructuring target", at: at),
                        .Spread(x) => {
                            if i != props.count - 1 { throw syntaxError("rest element must be last", at: x.span.start); }
                            let r = try self.convertRest(x, binding);
                            // An object rest is a plain name or member, never a pattern.
                            match r {
                                .Identifier(_) => {},
                                .Expression(_) => {},
                                _ => throw syntaxError("invalid rest element", at: x.span.start)
                            }
                            rest = .Some(r);
                        }
                    }
                }
                return .Ok(.ObjectPattern(properties: out, rest: rest));
            },
            _ => throw syntaxError("invalid assignment target", at: at)
        }
    }

    /// A rest element can't have a default.
    mutating func convertRest(e: Expr, binding: Bool) -> Result[Pattern, SyntaxError] {
        if let .Assign(_, _, _) = e.kind {
            if not e.parenthesized { throw syntaxError("a rest element can't have a default", at: e.span.start); }
        }
        self.convertTarget(e, binding)
    }

    /// A pattern built in assignment mode (inside `a = …` defaults) used as
    /// a binding: it can't hold member targets, and names must be bindable.
    func validateBinding(p: Pattern, at: Int64) -> Result[(), SyntaxError] {
        match p {
            .Identifier(name) => try self.checkBindingName(name, at),
            .Expression(_) => throw syntaxError("invalid destructuring target", at: at),
            .ObjectPattern(props, rest) => {
                for prop in props { try self.validateBinding(prop.value, at); }
                if let .Some(r) = rest { try self.validateBinding(r, at); }
            },
            .ArrayPattern(elements, rest) => {
                for el in elements {
                    if let .Some(x) = el { try self.validateBinding(x, at); }
                }
                if let .Some(r) = rest { try self.validateBinding(r, at); }
            },
            .WithDefault(target, _) => try self.validateBinding(target, at),
            .Rest(inner) => try self.validateBinding(inner, at)
        }
        .Ok(())
    }

    /// The operand of `++`, `--` and compound assignment: a name or a
    /// member access.
    mutating func toSimpleTarget(e: Expr) -> Result[Pattern, SyntaxError] {
        self.toSimpleTargetAt(e, e.span.start)
    }

    mutating func toSimpleTargetAt(e: Expr, at: Int64) -> Result[Pattern, SyntaxError] {
        match e.kind {
            .Identifier(name) => {
                if self.ctx.strict and (name == "eval" or name == "arguments") {
                    throw earlyError("can't assign to '\(name)' in strict mode", at: at);
                }
                return .Ok(.Identifier(name));
            },
            .Member(_, _, optional) => {
                if optional { throw syntaxError("invalid assignment target", at: at); }
                return .Ok(.Expression(e));
            },
            _ => throw syntaxError("invalid assignment target", at: at)
        }
    }
}

/// An unparenthesized array or object literal, which `=` turns into a
/// destructuring pattern.
func isLiteralPattern(e: Expr) -> Bool {
    if e.parenthesized { return false; }
    match e.kind {
        .ArrayLit(_) => true,
        .ObjectLit(_) => true,
        _ => false
    }
}

/// Wraps a function's rest parameter.
func restMarker(p: Pattern) -> Pattern { .Rest(p) }

/// The pattern inside a rest parameter.
func restOf(p: Pattern) -> Optional[Pattern] {
    match p {
        .Rest(inner) => .Some(inner),
        _ => .None
    }
}

/// Plain names only: no defaults, destructuring or rest.
func paramsAreSimple(params: Array[Pattern]) -> Bool {
    for p in params {
        match p {
            .Identifier(_) => {},
            _ => return false
        }
    }
    true
}

/// Every name the patterns bind, in order.
func boundNames(params: Array[Pattern]) -> Array[String] {
    var out: [String] = [];
    for p in params { collectNames(p, out); }
    out
}

func collectNames(p: Pattern, mutating out: Array[String]) {
    match p {
        .Identifier(name) => out.append(name),
        .Expression(_) => {},
        .ObjectPattern(props, rest) => {
            for prop in props { collectNames(prop.value, out); }
            if let .Some(r) = rest { collectNames(r, out); }
        },
        .ArrayPattern(elements, rest) => {
            for el in elements {
                if let .Some(x) = el { collectNames(x, out); }
            }
            if let .Some(r) = rest { collectNames(r, out); }
        },
        .WithDefault(target, _) => collectNames(target, out),
        .Rest(inner) => collectNames(inner, out)
    }
}

/// An async arrow can't bind `await` or use it in a default.
func patternContainsAwait(p: Pattern) -> Bool {
    var names: [String] = [];
    collectNames(p, names);
    for n in names {
        if n == "await" { return true; }
    }
    false
}
