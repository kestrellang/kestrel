module js

// A recursive-descent parser for ECMAScript 2024 scripts and modules.
//
// Statements are in parser_stmt.ks, expressions in parser_expr.ks, and
// binding patterns and the expression-to-pattern conversion in
// parser_pattern.ks, and declaration scopes in parser_scope.ks. This file
// holds the parser's state and helpers.

import std.collections.(Set)

/// What the code being parsed may do, which changes with each function.
struct Context {
    var strict: Bool
    /// `yield` is an operator.
    var inGenerator: Bool
    /// `await` is an operator.
    var inAsync: Bool
    /// Inside some function (non-arrow), so `return` and `new.target` work.
    var inFunction: Bool
    /// `return` is allowed.
    var allowReturn: Bool
    /// `super.x` is allowed (methods, and arrows inside them).
    var allowSuperProperty: Bool
    /// `super()` is allowed (derived class constructors).
    var allowSuperCall: Bool
    /// `new.target` is allowed.
    var allowNewTarget: Bool
    /// Inside a class static block, where `await` can't be an identifier.
    var inStaticBlock: Bool
    /// Inside parameters, where `yield` and `await` expressions are errors.
    var inParameters: Bool
    /// In a class field initializer, where `arguments` is an error.
    var inFieldInit: Bool
    /// Labels in scope, and whether each labels a loop.
    var labels: Array[String]
    var loopLabels: Array[Bool]
    var loopDepth: Int64
    var switchDepth: Int64
    /// How many labels directly precede the statement being parsed, as in
    /// `a: b: while …`, where both label the loop.
    var labelRun: Int64
    /// Where the first `yield` expression in this function was, or -1.
    /// Arrow parameters read as a parenthesized cover can't contain one.
    var yieldPos: Int64
}

struct Parser {
    var lexer: Lexer
    var tok: Token
    /// Where the previous token ended, for node spans.
    var prevEnd: Int64
    var isModule: Bool
    var ctx: Context
    /// The position of the first `{ a = 1 }` shorthand default seen in an
    /// object literal that hasn't been turned into a pattern yet, or -1.
    /// It's only valid as a destructuring pattern.
    var coverInit: Int64
    /// Start positions of array and object literals with a comma after a
    /// spread, which can't become patterns: `[...a,] = b`.
    var commaAfterSpread: Array[Int64]
    /// Declaration scopes, innermost last (see parser_scope.ks).
    var scopes: Array[Scope]
    /// Names a module exports, each allowed once.
    var exportedNames: Set[String]
    /// Locals named by `export { a }`, checked once the module is parsed.
    var exportedLocals: Array[String]
    var exportedLocalPositions: Array[Int64]
    /// A direct `eval(...)` call was parsed in the current function.
    var sawDirectEval: Bool
    /// Direct eval calls parsed so far: an arrow's parameters contain one
    /// when this grew while they were parsed.
    var directEvalCalls: Int64

    init(source source: String, isModule isModule: Bool) {
        self.lexer = Lexer(text: source);
        self.lexer.htmlComments = not isModule;
        self.tok = Token(kind: .End, start: 0);
        self.prevEnd = 0;
        self.isModule = isModule;
        self.ctx = Context(
            strict: isModule, inGenerator: false, inAsync: false, inFunction: false,
            allowReturn: false, allowSuperProperty: false, allowSuperCall: false,
            allowNewTarget: false, inStaticBlock: false, inParameters: false,
            inFieldInit: false, labels: [], loopLabels: [], loopDepth: 0, switchDepth: 0, labelRun: 0,
            yieldPos: -1
        );
        self.coverInit = -1;
        self.commaAfterSpread = [];
        self.scopes = [newScope(.Function)];
        self.exportedNames = Set();
        self.exportedLocals = [];
        self.exportedLocalPositions = [];
        self.sawDirectEval = false;
        self.directEvalCalls = 0;
    }

    /// Parses a whole script or module.
    mutating func parseProgram() -> Result[Program, SyntaxError] {
        self.lexer.skipHashbang();
        try self.advance();
        var body: [Stmt] = [];
        try self.parseDirectives(body);
        while self.tok.kind != TokenKind.End {
            body.append(try self.parseStatementListItem());
        }
        if self.isModule { try self.checkExportedLocals(); }
        .Ok(Program(body: body, isModule: self.isModule))
    }

    // ------------------------------------------------------------------
    // Tokens
    // ------------------------------------------------------------------

    mutating func advance() -> Result[(), SyntaxError] {
        self.prevEnd = self.tok.end;
        self.tok = try self.lexer.next();
        .Ok(())
    }

    /// The token after the current one, without consuming anything.
    mutating func peek() -> Result[Token, SyntaxError] {
        let savedPos = self.lexer.pos;
        let next = try self.lexer.next();
        self.lexer.pos = savedPos;
        .Ok(next)
    }

    func at(text: String) -> Bool { self.tok.isPunct(text) }

    func atKeyword(text: String) -> Bool { self.tok.isKeyword(text) }

    /// Consumes punctuator `text` if it's next.
    mutating func eat(text: String) -> Result[Bool, SyntaxError] {
        if self.tok.isPunct(text) {
            try self.advance();
            return .Ok(true);
        }
        .Ok(false)
    }

    /// Consumes keyword `text` if it's next.
    mutating func eatKeyword(text: String) -> Result[Bool, SyntaxError] {
        if self.tok.isKeyword(text) {
            try self.advance();
            return .Ok(true);
        }
        .Ok(false)
    }

    mutating func expect(text: String) -> Result[(), SyntaxError] {
        if not self.tok.isPunct(text) {
            throw self.unexpected();
        }
        self.advance()
    }

    mutating func expectKeyword(text: String) -> Result[(), SyntaxError] {
        if not self.tok.isKeyword(text) {
            throw self.unexpected();
        }
        self.advance()
    }

    func unexpected() -> SyntaxError {
        if self.tok.kind == TokenKind.End {
            return syntaxError("unexpected end of input", at: self.tok.start);
        }
        syntaxError("unexpected token '\(self.lexer.text(self.tok.start, self.tok.end))'", at: self.tok.start)
    }

    func error(message: String) -> SyntaxError {
        syntaxError(message, at: self.tok.start)
    }

    /// Ends a statement: a `;`, or a place automatic semicolon insertion
    /// allows one (before `}`, at the end, or after a line break).
    mutating func semicolon() -> Result[(), SyntaxError] {
        if try self.eat(";") { return .Ok(()); }
        if self.at("}") or self.tok.kind == TokenKind.End or self.tok.newlineBefore {
            return .Ok(());
        }
        throw self.unexpected()
    }

    func span(from start: Int64) -> Span {
        Span(start: start, end: self.prevEnd)
    }

    func expr(kind: ExprKind, start: Int64) -> Expr {
        Expr(kind: kind, span: Span(start: start, end: self.prevEnd), parenthesized: false)
    }

    func stmt(kind: StmtKind, start: Int64) -> Stmt {
        Stmt(kind: kind, span: Span(start: start, end: self.prevEnd))
    }

    // ------------------------------------------------------------------
    // Names
    // ------------------------------------------------------------------

    /// The current token can be an identifier reference or binding here.
    func atIdentifier() -> Bool {
        if self.tok.kind != TokenKind.Name { return false; }
        not self.isReserved(self.tok.value)
    }

    /// `name` is reserved in the current context.
    func isReserved(name: String) -> Bool {
        if isAlwaysReserved(name) { return true; }
        if self.ctx.strict and isStrictReserved(name) { return true; }
        if name == "yield" { return self.ctx.inGenerator or self.ctx.strict; }
        if name == "await" { return self.ctx.inAsync or self.isModule or self.ctx.inStaticBlock; }
        false
    }

    /// Reads an identifier used as a reference or binding.
    mutating func parseIdentifier() -> Result[String, SyntaxError] {
        if self.tok.kind != TokenKind.Name { throw self.unexpected(); }
        let name = self.tok.value;
        if self.isReserved(name) {
            if self.tok.escaped { throw self.error("keyword '\(name)' can't contain escapes"); }
            throw self.unexpected();
        }
        if self.ctx.inFieldInit and name == "arguments" {
            throw earlyError("'arguments' in a class field initializer", at: self.tok.start);
        }
        try self.advance();
        .Ok(name)
    }

    /// Reads an identifier that a declaration binds, which in strict code
    /// can't be `eval` or `arguments`.
    mutating func parseBindingIdentifier() -> Result[String, SyntaxError] {
        let start = self.tok.start;
        let name = try self.parseIdentifier();
        try self.checkBindingName(name, start);
        .Ok(name)
    }

    func checkBindingName(name: String, at: Int64) -> Result[(), SyntaxError] {
        if self.ctx.strict and (name == "eval" or name == "arguments") {
            throw earlyError("can't bind '\(name)' in strict mode", at: at);
        }
        .Ok(())
    }

    /// Reads any IdentifierName, keywords included: property names after
    /// `.`, and keys in object literals and classes.
    mutating func parseIdentifierName() -> Result[String, SyntaxError] {
        if self.tok.kind != TokenKind.Name { throw self.unexpected(); }
        let name = self.tok.value;
        try self.advance();
        .Ok(name)
    }

    // ------------------------------------------------------------------
    // Directives and function bodies
    // ------------------------------------------------------------------

    /// Reads a directive prologue (`"use strict";` and friends) into
    /// `body`. Returns whether it contained `"use strict"`.
    mutating func parseDirectives(mutating body: Array[Stmt]) -> Result[Bool, SyntaxError] {
        var sawStrict = false;
        var sawOctal = false;
        while self.tok.kind == TokenKind.String {
            // A directive is a string alone in its statement.
            let next = try self.peek();
            let ends = next.isPunct(";") or next.isPunct("}") or next.kind == TokenKind.End or next.newlineBefore;
            if not ends or self.continuesExpression(next) { break; }
            let start = self.tok.start;
            let raw = self.lexer.text(self.tok.start + 1, self.tok.end - 1);
            let value = self.tok.value;
            if self.tok.legacyOctal { sawOctal = true; }
            if raw == "use strict" {
                sawStrict = true;
                self.ctx.strict = true;
            }
            try self.advance();
            try self.semicolon();
            body.append(self.stmt(.Directive(raw: raw, value: value), start));
        }
        if sawOctal and self.ctx.strict {
            throw earlyError("octal escape in strict mode", at: self.tok.start);
        }
        .Ok(sawStrict)
    }

    /// A string followed on a new line by `next` still continues as one
    /// expression when `next` is an operator: `"a"\n + b`.
    func continuesExpression(next: Token) -> Bool {
        if not next.newlineBefore or next.kind != TokenKind.Punct { return false; }
        let v = next.value;
        not (v == ";" or v == "}" or v == "{" or v == "!" or v == "~" or v == "++" or v == "--")
    }
}

func isAlwaysReserved(name: String) -> Bool {
    match name {
        "break" or "case" or "catch" or "class" or "const" or "continue" or "debugger" or "default" or "delete" or "do" or "else" or "enum" or "export" or "extends" or "false" or "finally" or "for" or "function" or "if" or "import" or "in" or "instanceof" or "new" or "null" or "return" or "super" or "switch" or "this" or "throw" or "true" or "try" or "typeof" or "var" or "void" or "while" or "with" => true,
        _ => false
    }
}

func isStrictReserved(name: String) -> Bool {
    match name {
        "implements" or "interface" or "let" or "package" or "private" or "protected" or "public" or "static" or "yield" => true,
        _ => false
    }
}

/// Parses `source` as a script.
func parseScript(source: String) -> Result[Program, SyntaxError] {
    var p = Parser(source: source, isModule: false);
    p.parseProgram()
}

/// What the code around a direct eval allows inside it.
struct EvalSyntax {
    var strict: Bool
    var inFunction: Bool
    var allowNewTarget: Bool
    var allowSuperProperty: Bool
    var allowSuperCall: Bool
    var inFieldInit: Bool
}

/// Parses eval code: a script, with what its surroundings allow.
func parseEvalSource(source: String, syntax: EvalSyntax) -> Result[Program, SyntaxError] {
    var p = Parser(source: source, isModule: false);
    p.ctx.strict = syntax.strict;
    p.ctx.inFunction = syntax.inFunction;
    p.ctx.allowNewTarget = syntax.allowNewTarget;
    p.ctx.allowSuperProperty = syntax.allowSuperProperty;
    p.ctx.allowSuperCall = syntax.allowSuperCall;
    p.ctx.inFieldInit = syntax.inFieldInit;
    p.parseProgram()
}

/// Parses `source` as a module.
func parseModule(source: String) -> Result[Program, SyntaxError] {
    var p = Parser(source: source, isModule: true);
    p.parseProgram()
}
