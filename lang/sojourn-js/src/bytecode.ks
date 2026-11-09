module js

// The bytecode the compiler emits and the interpreter runs.
//
// It's register-based: each function has a fixed set of registers, and
// instructions name the registers they read and write. Variables live in
// environments (`Env`), addressed as (hops, slot): how many environments
// up the chain, and which slot there. Names are only looked up at run
// time for globals.

@derive(Equatable)
enum BinOp {
    case Add
    case Sub
    case Mul
    case Div
    case Mod
    case Exp
    case Shl
    case Shr
    case UShr
    case BitAnd
    case BitOr
    case BitXor
    case Eq
    case NotEq
    case StrictEq
    case StrictNotEq
    case Lt
    case Gt
    case LtEq
    case GtEq
    case In
    case InstanceOf
}

@derive(Equatable)
enum UnOp {
    case Neg
    case Plus
    case Not
    case BitNot
    case TypeOf
    case ToNumeric
    case Inc
    case Dec
    case ToPropertyKey
    case ToStr
    /// `ToObject`, throwing on null and undefined (for destructuring).
    case RequireObject
    /// `ToObject`, boxing primitives (for `with`).
    case ToObject
}

/// Errors the bytecode itself raises.
@derive(Equatable)
enum ErrorKind {
    case TypeError
    case ReferenceError
    case RangeError
    case SyntaxError
    case URIError
}

enum Instr {
    case Load(dst: Int64, value: Value)
    /// An interned string literal.
    case LoadString(dst: Int64, atom: Key)
    /// A BigInt literal, from its decimal digits.
    case LoadBigInt(dst: Int64, atom: Key)
    case Move(dst: Int64, src: Int64)

    // Variables
    /// Reads a slot, throwing a ReferenceError if it's uninitialized.
    case GetVar(dst: Int64, hops: Int64, slot: Int64, name: Key)
    /// Assigns to an initialized `let` or `var`.
    case SetVar(hops: Int64, slot: Int64, src: Int64, name: Key)
    /// Initializes a declaration's slot (no TDZ check).
    case InitVar(hops: Int64, slot: Int64, src: Int64)
    /// Inside `with`: if the object `hops` envs up has `name` (and its
    /// @@unscopables doesn't hide it), reads it into `dst`, puts the object
    /// in `this` (unless -1) and jumps to `target`. A `dst` of -1 only
    /// finds the object.
    case WithGet(dst: Int64, this: Int64, hops: Int64, slot: Int64, name: Key, target: Int64)
    /// The same lookup for an assignment: stores `src` and jumps.
    case WithSet(hops: Int64, slot: Int64, name: Key, src: Int64, strict: Bool, target: Int64)
    case GetGlobal(dst: Int64, name: Key, forTypeof: Bool)
    case SetGlobal(name: Key, src: Int64, strict: Bool)
    /// Puts a fresh eval-vars object (see `CScope.hasEvalVars`) in slot
    /// `slot` of the current env.
    case InitEvalVars(slot: Int64)
    /// A `var` or function a sloppy direct eval declares: a configurable
    /// property, left alone if it exists, on the eval-vars object in
    /// `slot` `hops` envs up, or (hops -1) on the global object.
    case DeclareEvalVar(hops: Int64, slot: Int64, name: Key)
    /// `eval(...)` by name: if `callee` is the realm's eval, runs the
    /// string in the scope `VM.evalContexts(ctx)` describes; otherwise
    /// an ordinary call.
    case DirectEval(dst: Int64, callee: Int64, this: Int64, args: Int64, argc: Int64, ctx: Int64)
    /// Script-level `var`: a global property, left alone if it exists.
    case DeclareGlobalVar(name: Key)
    /// Script-level function: a global property set to `src`.
    /// Script-level (or global eval) function: defines the global, or only
    /// sets its value when the existing property can't be redefined.
    /// `deletable` (eval) makes a new property configurable.
    case DeclareGlobalFunction(name: Key, src: Int64, deletable: Bool)
    /// The checks before a script or global eval declares anything:
    /// CanDeclareGlobalVar / CanDeclareGlobalFunction (TypeError) and a
    /// clash with a global lexical (SyntaxError).
    case CheckGlobalVar(name: Key)
    case CheckGlobalFunction(name: Key)
    /// A script-level let/const/class can't shadow a global var or a
    /// non-configurable global property (SyntaxError).
    case CheckGlobalLexical(name: Key)
    /// Script-level `let`/`const`/`class`: a binding in the shared global
    /// lexical scope, uninitialized until `InitGlobalLexical`.
    case DeclareGlobalLexical(name: Key, isConst: Bool)
    case InitGlobalLexical(name: Key, src: Int64)
    case PushEnv(size: Int64)
    case PopEnv
    /// Replaces the innermost environment with a copy (a `for (let …)`
    /// loop's per-iteration bindings).
    case CopyEnv

    // Operators
    case Binary(op: BinOp, dst: Int64, a: Int64, b: Int64)
    case Unary(op: UnOp, dst: Int64, src: Int64)
    case Jump(target: Int64)
    /// Jumps when the value's truthiness is `when`.
    case JumpIf(cond: Int64, target: Int64, when: Bool)
    case JumpIfNullish(src: Int64, target: Int64)
    case JumpIfUndefined(src: Int64, target: Int64)
    case JumpIfNotUndefined(src: Int64, target: Int64)

    // Properties
    /// `obj.key`, with inline cache `ic`.
    case GetProp(dst: Int64, obj: Int64, key: Key, ic: Int64)
    case SetProp(obj: Int64, key: Key, src: Int64, ic: Int64, strict: Bool)
    /// `obj[key]`, where `key` is a register.
    case GetElem(dst: Int64, obj: Int64, key: Int64)
    case SetElem(obj: Int64, key: Int64, src: Int64, strict: Bool)
    case DeleteElem(dst: Int64, obj: Int64, key: Int64, strict: Bool)
    /// `super[key]` with `this` as the receiver.
    case SuperGet(dst: Int64, key: Int64, this: Int64)
    case SuperSet(key: Int64, src: Int64, this: Int64)
    case NewObject(dst: Int64)
    case NewArray(dst: Int64)
    /// A regex literal: a fresh RegExp each evaluation.
    case NewRegExp(dst: Int64, pattern: Key, flags: Key)
    /// Appends `src` to an array literal; an empty value makes a hole.
    case ArrayAppend(arr: Int64, src: Int64)
    /// Appends everything `src` iterates.
    case ArraySpread(arr: Int64, src: Int64)
    /// Defines an own enumerable data property (object literals).
    case DefineProp(obj: Int64, key: Int64, src: Int64)
    /// A class evaluation's fresh private name `#x`: a symbol `GetElem`,
    /// `SetElem`, `DefineProp` and `in` treat as private (`privateGet`).
    case NewPrivateName(dst: Int64, name: Key)
    /// Adds a private method (`kind` 0), getter (1) or setter (2) to `obj`.
    case PrivateMethod(obj: Int64, key: Int64, fn: Int64, kind: Int64)
    /// Defines a method (non-enumerable when `enumerable` is false).
    case DefineMethod(obj: Int64, key: Int64, fn: Int64, enumerable: Bool)
    case DefineAccessor(obj: Int64, key: Int64, fn: Int64, isGetter: Bool, enumerable: Bool)
    /// Copies `src`'s own enumerable properties into `dst`, skipping the
    /// keys in registers `excluded ..< excluded + count`.
    case CopyDataProps(dst: Int64, src: Int64, excluded: Int64, count: Int64)
    case SetProto(obj: Int64, proto: Int64)

    // Functions
    /// Makes a closure of `code` over the current environment. `home` is
    /// the register holding the method's home object, or -1.
    case MakeClosure(dst: Int64, code: Int64, home: Int64)
    /// `callee(args ..< args + argc)` with `this`.
    case Call(dst: Int64, callee: Int64, this: Int64, args: Int64, argc: Int64)
    /// A call whose arguments are the array in `args`.
    case CallSpread(dst: Int64, callee: Int64, this: Int64, args: Int64)
    case New(dst: Int64, callee: Int64, args: Int64)
    /// `super(...)` with the arguments array in `args`. Binds `this` in
    /// the slot (hops, slot).
    case SuperCall(dst: Int64, args: Int64, hops: Int64, slot: Int64)
    case Return(src: Int64)
    case Throw(src: Int64)
    case ThrowError(kind: ErrorKind, message: String)
    case GetArg(dst: Int64, index: Int64)
    /// The arguments from `from` on, as an array.
    case GetRestArgs(dst: Int64, from: Int64)
    case CreateArguments(dst: Int64)
    case NewTarget(dst: Int64)
    /// Sets up a class: `ctor` holds the constructor closure and
    /// `superclass` the parent (or -1). Writes the prototype to `proto`.
    case MakeClass(ctor: Int64, proto: Int64, superclass: Int64)
    /// Stores the field initializer `fn` on class constructor `ctor`.
    case SetClassFields(ctor: Int64, fn: Int64)
    /// Runs a class's field initializers on `this` (an internal method
    /// stored on the constructor).
    case InitFields(this: Int64)

    // Exceptions
    /// Starts a try block: a throw jumps to `handler` with the exception
    /// in `exception`.
    case EnterTry(handler: Int64, exception: Int64)
    case LeaveTry

    // Iteration
    case ForInPrepare(dst: Int64, obj: Int64)
    /// Next for-in key into `dst`, or jumps to `done`.
    case ForInNext(dst: Int64, iter: Int64, done: Int64)
    /// `obj[Symbol.iterator]()`, into `iter`; the `next` method goes in
    /// `iter + 1`.
    case GetIterator(iter: Int64, obj: Int64)
    /// `obj[Symbol.asyncIterator]()`, or its sync iterator wrapped
    /// (CreateAsyncFromSyncIterator), and `next`, into `iter` and `iter+1`.
    case GetAsyncIterator(iter: Int64, obj: Int64)
    /// Calls `next`; jumps to `done` when finished, otherwise writes the value.
    case IteratorNext(dst: Int64, iter: Int64, done: Int64)
    /// Calls the iterator's `return`, if any (leaving a loop early).
    case IteratorClose(iter: Int64)

    // Generators
    /// After the prologue: suspends the new frame into a generator object
    /// and returns that to the caller.
    case GeneratorStart
    /// Suspends with `src`. On resume, `dst` gets the sent value and
    /// `dst + 1` the resume mode (0 next, 1 throw, 2 return). A `raw`
    /// yield (from `yield*`) hands `src` out as the iterator result itself.
    case Yield(dst: Int64, src: Int64, raw: Bool)
    /// Throws a TypeError unless `src` is an object (iterator results).
    case RequireObjectResult(src: Int64)
    case Nop
}

/// A compiled function, script or class field initializer.
struct Code {
    var name: String
    var instrs: [Instr]
    var registers: Int64
    /// Slots in the function's own environment.
    var envSize: Int64
    /// The `length` property: parameters before the first default or rest.
    var length: Int64
    var isArrow: Bool
    var isStrict: Bool
    /// Methods and arrows can't be constructed.
    var isConstructor: Bool
    var isClassConstructor: Bool
    var isDerived: Bool
    var isGenerator: Bool
    /// An async function: runs as a generator whose awaits are yields
    /// (see `startAsync`).
    var isAsync: Bool
    /// Where the function line came from, for error messages.
    var sourceName: String
}
