module js

// The syntax tree. It follows ESTree's shapes (https://github.com/estree)
// so it maps onto what JS tools already know, with Kestrel enums in place
// of `type` strings. Expressions and statements carry their source span;
// the smaller pieces don't.

/// A code point range in the source.
struct Span {
    var start: Int64
    var end: Int64
}

// ----------------------------------------------------------------------
// Programs
// ----------------------------------------------------------------------

struct Program {
    var body: Array[Stmt]
    var isModule: Bool
}

// ----------------------------------------------------------------------
// Expressions
// ----------------------------------------------------------------------

struct Expr {
    var kind: ExprKind
    /// The expression's own span; enclosing parentheses aren't included.
    var span: Span
    /// Written in parentheses. The tree doesn't otherwise record them, but
    /// some rules depend on them: `(a) = 1` is fine and `({a}) = 1` isn't,
    /// `-x ** 2` is an error and `(-x) ** 2` isn't.
    var parenthesized: Bool
}

indirect enum ExprKind {
    case Identifier(String)
    case This
    /// `super`, only as the object of a member access or the callee of a
    /// call.
    case Super
    case Null
    case BoolLit(Bool)
    case Number(Float64)
    /// The value in base 10.
    case BigInt(String)
    case StringLit(String)
    case Regex(pattern: String, flags: String)
    case Template(TemplateLiteral)
    case TaggedTemplate(tag: Expr, quasi: TemplateLiteral)
    case ArrayLit(Array[ArrayElement])
    case ObjectLit(Array[Property])
    case FunctionExpr(Function)
    case ArrowFunction(Function)
    case ClassExpr(Class)
    /// `-x`, `!x`, `typeof x`, `void x`, `delete x`, …
    case Unary(op: String, argument: Expr)
    /// `++x`, `x--`.
    case Update(op: String, prefix: Bool, argument: Expr)
    /// Arithmetic, comparison, bitwise, `in`, `instanceof`.
    case Binary(op: String, left: Expr, right: Expr)
    /// `&&`, `||`, `??`.
    case Logical(op: String, left: Expr, right: Expr)
    /// `#x in obj`.
    case PrivateIn(name: String, right: Expr)
    /// `=` takes any pattern; compound operators take an identifier or a
    /// member expression, held as `Pattern.Expression`.
    case Assign(op: String, target: Pattern, value: Expr)
    case Conditional(test: Expr, consequent: Expr, alternate: Expr)
    case Call(callee: Expr, arguments: Array[Argument], optional: Bool)
    case New(callee: Expr, arguments: Array[Argument])
    case Member(object: Expr, property: MemberProperty, optional: Bool)
    /// The whole of an optional chain `a?.b.c`, so `(a?.b).c` differs.
    case Chain(Expr)
    case Sequence(Array[Expr])
    case Yield(argument: Optional[Expr], delegate: Bool)
    case Await(Expr)
    /// `new.target`, `import.meta`.
    case MetaProperty(meta: String, property: String)
    /// `import(source)` and `import(source, options)`.
    case Import(source: Expr, options: Optional[Expr])
}

enum ArrayElement {
    case Hole
    case Item(Expr)
    case Spread(Expr)
}

enum Argument {
    case Item(Expr)
    case Spread(Expr)
}

enum MemberProperty {
    /// `a.name`
    case Name(String)
    /// `a.#name`
    case Private(String)
    /// `a[expr]`
    case Computed(Expr)
}

/// A property name in an object literal, class or object pattern.
enum PropertyKey {
    /// `name` or a keyword used as a name.
    case Name(String)
    case StringKey(String)
    case NumberKey(Float64)
    case BigIntKey(String)
    /// `[expr]`
    case Computed(Expr)
    /// `#name`, only in classes.
    case Private(String)
}

enum Property {
    /// `key: value`, or `key` alone when `shorthand`.
    case Init(key: PropertyKey, value: Expr, shorthand: Bool)
    /// A method, getter or setter.
    case Method(key: PropertyKey, kind: MethodKind, function: Function)
    /// `...expr`
    case Spread(Expr)
}

@derive(Equatable)
enum MethodKind {
    case Method
    case Get
    case Set
    case Constructor
}

struct TemplateLiteral {
    /// The text parts. There is always one more quasi than expression.
    var quasis: Array[TemplateElement]
    var expressions: Array[Expr]
}

struct TemplateElement {
    /// The cooked text, or `None` for an invalid escape in a tagged template.
    var cooked: Optional[String]
    var raw: String
}

// ----------------------------------------------------------------------
// Functions and classes
// ----------------------------------------------------------------------

struct Function {
    var id: Optional[String]
    var params: Array[Pattern]
    var body: FunctionBody
    var isAsync: Bool
    var isGenerator: Bool
    /// The body (not counting nested functions) calls `eval(...)`
    /// directly, so its scopes must be visible at run time.
    var hasDirectEval: Bool
}

enum FunctionBody {
    case Block(Array[Stmt])
    /// An arrow function's concise body: `x => x + 1`.
    case Expression(Expr)
}

struct Class {
    var id: Optional[String]
    var superClass: Optional[Expr]
    var body: Array[ClassMember]
}

enum ClassMember {
    case Method(key: PropertyKey, kind: MethodKind, isStatic: Bool, function: Function)
    case Field(key: PropertyKey, isStatic: Bool, value: Optional[Expr])
    case StaticBlock(Array[Stmt])
}

// ----------------------------------------------------------------------
// Patterns (binding and assignment targets)
// ----------------------------------------------------------------------

indirect enum Pattern {
    case Identifier(String)
    /// An assignment target that isn't a binding: `a.b`, `a[0]`.
    case Expression(Expr)
    case ObjectPattern(properties: Array[PatternProperty], rest: Optional[Pattern])
    /// `None` elements are holes: `[, a]`.
    case ArrayPattern(elements: Array[Optional[Pattern]], rest: Optional[Pattern])
    /// `target = default`
    case WithDefault(target: Pattern, value: Expr)
    /// A function's rest parameter, `...args`. (Rest elements in array and
    /// object patterns use their `rest` field.)
    case Rest(Pattern)
}

struct PatternProperty {
    var key: PropertyKey
    var value: Pattern
    var shorthand: Bool
}

// ----------------------------------------------------------------------
// Statements
// ----------------------------------------------------------------------

struct Stmt {
    var kind: StmtKind
    var span: Span
}

indirect enum StmtKind {
    case Expression(Expr)
    /// A directive in a prologue, `"use strict";`. The text between the
    /// quotes, undecoded.
    case Directive(String)
    case Var(VariableDeclaration)
    case FunctionDecl(Function)
    case ClassDecl(Class)
    case Block(Array[Stmt])
    case Empty
    case If(test: Expr, consequent: Stmt, alternate: Optional[Stmt])
    case For(initializer: Optional[ForInit], test: Optional[Expr], update: Optional[Expr], body: Stmt)
    case ForIn(left: ForTarget, right: Expr, body: Stmt)
    case ForOf(left: ForTarget, right: Expr, body: Stmt, isAwait: Bool)
    case While(test: Expr, body: Stmt)
    case DoWhile(body: Stmt, test: Expr)
    case Return(Optional[Expr])
    case Break(Optional[String])
    case Continue(Optional[String])
    case Throw(Expr)
    case Try(block: Array[Stmt], handler: Optional[CatchClause], finalizer: Optional[Array[Stmt]])
    case Switch(discriminant: Expr, cases: Array[SwitchCase])
    case Labeled(label: String, body: Stmt)
    case Debugger
    case With(object: Expr, body: Stmt)
    case Import(ImportDeclaration)
    case Export(ExportDeclaration)
}

@derive(Equatable)
enum VariableKind {
    case Var
    case Let
    case Const
    case Using
    case AwaitUsing
}

struct VariableDeclaration {
    var kind: VariableKind
    var declarations: Array[VariableDeclarator]
}

struct VariableDeclarator {
    var target: Pattern
    var value: Optional[Expr]
}

enum ForInit {
    case Declaration(VariableDeclaration)
    case Expression(Expr)
}

/// The left side of `for (… in …)` and `for (… of …)`.
enum ForTarget {
    case Declaration(VariableDeclaration)
    case Target(Pattern)
}

struct CatchClause {
    var param: Optional[Pattern]
    var body: Array[Stmt]
}

struct SwitchCase {
    /// `None` for `default:`.
    var test: Optional[Expr]
    var body: Array[Stmt]
}

// ----------------------------------------------------------------------
// Modules
// ----------------------------------------------------------------------

struct ImportDeclaration {
    var specifiers: Array[ImportSpecifier]
    var source: String
    var attributes: Array[ImportAttribute]
}

enum ImportSpecifier {
    /// `import a from "m"`
    case Default(local: String)
    /// `import * as ns from "m"`
    case Namespace(local: String)
    /// `import { imported as local } from "m"`
    case Named(imported: String, local: String)
}

struct ImportAttribute {
    var key: String
    var value: String
}

enum ExportDeclaration {
    /// `export var …`, `export function …`, `export class …`.
    case Declaration(Stmt)
    /// `export default expr;`, `export default function …`.
    case Default(Stmt)
    /// `export { a as b }`, optionally `from "m"`.
    case Named(specifiers: Array[ExportSpecifier], source: Optional[String], attributes: Array[ImportAttribute])
    /// `export * from "m"` or `export * as ns from "m"`.
    case All(exported: Optional[String], source: String, attributes: Array[ImportAttribute])
}

struct ExportSpecifier {
    var local: String
    var exported: String
}
